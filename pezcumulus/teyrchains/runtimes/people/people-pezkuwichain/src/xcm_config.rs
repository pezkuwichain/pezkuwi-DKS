// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{
	AccountId, AllPalletsWithSystem, Balances, PezkuwiXcm, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeOrigin, TeyrchainInfo, TeyrchainSystem, WeightToFee, XcmpQueue,
};
use crate::{TransactionByteFee, CENTS};
use pezframe_support::{
	parameter_types,
	traits::{
		tokens::imbalance::ResolveTo, ConstU32, Contains, Disabled, Equals, Everything, Nothing,
	},
};
use pezframe_system::EnsureRoot;
use pezkuwi_teyrchain_primitives::primitives::Sibling;
use pezpallet_collator_selection::StakingPotAccountId;
use pezpallet_xcm::XcmPassthrough;
use testnet_teyrchains_constants::pezkuwichain::locations::AssetHubLocation;
use teyrchains_common::xcm_config::{
	AllSiblingSystemTeyrchains, ConcreteAssetFromSystem, ParentRelayOrSiblingTeyrchains,
	RelayOrOtherSystemTeyrchains,
};
use xcm::latest::{prelude::*, PEZKUWICHAIN_GENESIS_HASH};
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowHrmpNotificationsFromRelayChain,
	AllowKnownQueryResponses, AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom,
	DenyRecursively, DenyReserveTransferToRelayChain, DenyThenTry, DescribeAllTerminal,
	DescribeFamily, DescribeTerminus, EnsureXcmOrigin, FrameTransactionalProcessor,
	FungibleAdapter, HashedDescription, IsConcrete, OriginToPluralityVoice, ParentAsSuperuser,
	ParentIsPreset, RelayChainAsNative, SendXcmFeeToAccount, SiblingTeyrchainAsNative,
	SiblingTeyrchainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation, TakeWeightCredit, TrailingSetTopicAsId, UsingComponents,
	WeightInfoBounds, WithComputedOrigin, WithUniqueTopic, XcmFeeManagerFromComponents,
};
use xcm_executor::XcmExecutor;

parameter_types! {
	pub const RootLocation: Location = Location::here();
	pub const RelayLocation: Location = Location::parent();
	pub const RelayNetwork: Option<NetworkId> = Some(NetworkId::ByGenesis(PEZKUWICHAIN_GENESIS_HASH));
	pub RelayChainOrigin: RuntimeOrigin = pezcumulus_pezpallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(RelayNetwork::get().unwrap()), Teyrchain(TeyrchainInfo::teyrchain_id().into())].into();
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
	pub const GovernanceLocation: Location = Location::parent();
	pub const FellowshipLocation: Location = Location::parent();
	/// The asset ID for the asset that we use to pay for message delivery fees. Just HEZ.
	pub FeeAssetId: AssetId = AssetId(RelayLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
	/// Delivery fees for messages this chain forwards.
	///
	/// They used to go to an address derived from the treasury's pallet id, which is right on a
	/// chain that has a treasury and meaningless on one that does not: the money arrived at an
	/// address with no pallet behind it and stayed there. This chain has no treasury, so the
	/// fee goes where its other operating income goes -- the collators who did the delivering.
	pub StakingPot: AccountId =
		<StakingPotAccountId<Runtime> as pezframe_support::traits::TypedGet>::get();
}

pub type PriceForParentDelivery = pezkuwi_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	TransactionByteFee,
	TeyrchainSystem,
>;

pub type PriceForSiblingTeyrchainDelivery = pezkuwi_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	TransactionByteFee,
	XcmpQueue,
>;

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling teyrchain origins convert to AccountId via the `ParaId::into`.
	SiblingTeyrchainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
	// Here/local root location to `AccountId`.
	HashedDescription<AccountId, DescribeTerminus>,
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

/// Means for transacting the native currency on this chain.
pub type FungibleTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RelayLocation>,
	// Do a simple punn to convert an `AccountId32` `Location` into a native chain
	// `AccountId`:
	LocationToAccountId,
	// Our chain's `AccountId` type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports of `Balances`.
	(),
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with XCM's `Transact`. There is an `OriginKind` that can
/// bias the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain that they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Teyrchains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingTeyrchainAsNative<pezcumulus_pezpallet_xcm::Origin, RuntimeOrigin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// XCM origins can be represented natively under the XCM pezpallet's `Xcm` origin.
	XcmPassthrough<RuntimeOrigin>,
);

pub struct LocalPlurality;
impl Contains<Location> for LocalPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (0, [Plurality { .. }]))
	}
}

pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
	}
}

pub type Barrier = TrailingSetTopicAsId<
	DenyThenTry<
		DenyRecursively<DenyReserveTransferToRelayChain>,
		(
			// Allow local users to buy weight credit.
			TakeWeightCredit,
			// Expected responses are OK.
			AllowKnownQueryResponses<PezkuwiXcm>,
			WithComputedOrigin<
				(
					// If the message is one that immediately attempts to pay for execution, then
					// allow it.
					AllowTopLevelPaidExecutionFrom<Everything>,
					// Parent and its pluralities (i.e. governance bodies) get free execution.
					AllowExplicitUnpaidExecutionFrom<(
						ParentOrParentsPlurality,
						// The rewards pallet here is funded from the Asset Hub, and the
						// treasury there is reached from here. Both directions were
						// configured and only the Asset Hub's door was open: this barrier
						// named the relay and its bodies alone, so a sibling's message was
						// turned away before the origin check that was written for it.
						Equals<AssetHubLocation>,
					)>,
					// Subscriptions for version tracking are OK.
					AllowSubscriptionsFrom<ParentRelayOrSiblingTeyrchains>,
					// HRMP notifications from the relay chain are OK.
					AllowHrmpNotificationsFromRelayChain,
				),
				UniversalLocation,
				ConstU32<8>,
			>,
		),
	>,
>;

/// Locations that will not be charged fees in the executor, neither for execution nor delivery. We
/// only waive fees for system functions, which these locations represent.
pub type WaivedLocations = (
	RelayOrOtherSystemTeyrchains<AllSiblingSystemTeyrchains, Runtime>,
	// The relay's treasury used to be waived here because it paid over XCM. It is retired:
	// the treasury is on the Asset Hub now and pays from its own account, so no treasury
	// sends messages to this chain. Should that ever change back to `PayOverXcm`, the
	// sending body has to be waived again or every payment is charged a fee it cannot pay
	// and is dropped without an error.
	Equals<RootLocation>,
	LocalPlurality,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type XcmEventEmitter = PezkuwiXcm;
	type AssetTransactor = FungibleTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	// People chain does not recognize a reserve location for any asset. Users must teleport HEZ
	// where allowed (e.g. with the Relay Chain).
	type IsReserve = ();
	/// Only allow teleportation of HEZ.
	type IsTeleporter = ConcreteAssetFromSystem<RelayLocation>;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::PeoplePezkuwichainXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	type Trader = UsingComponents<
		WeightToFee,
		RelayLocation,
		AccountId,
		Balances,
		ResolveTo<StakingPotAccountId<Runtime>, Balances>,
	>;
	type ResponseHandler = PezkuwiXcm;
	type AssetTrap = PezkuwiXcm;
	type SubscriptionService = PezkuwiXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		SendXcmFeeToAccount<Self::AssetTransactor, StakingPot>,
	>;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = PezkuwiXcm;
}

/// Converts a local signed origin into an XCM location. Forms the basis for local origins
/// sending/executing XCMs.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
	// Two routers - use UMP to communicate with the relay chain:
	pezcumulus_primitives_utility::ParentAsUmp<TeyrchainSystem, PezkuwiXcm, PriceForParentDelivery>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
)>;

parameter_types! {
	/// The three bodies the register speaks as when it sends a message out.
	///
	/// The relay used to address these same indices inbound, and a converter here turned them
	/// into Root -- so the relay's token-weighted electorate could revoke a citizenship. That
	/// converter is gone and these indices are now outbound only: this chain's head-counted
	/// tracks naming which office decided, never a way in.
	pub const WelatiElectionBodyId: BodyId = BodyId::Index(40);
	pub const WelatiAdminBodyId: BodyId = BodyId::Index(41);
	pub const CitizenshipAdminBodyId: BodyId = BodyId::Index(42);
}

/// The state governance origins, as the voice of the body that holds them.
///
/// A message sent this way carries which office decided it, rather than only that this chain
/// did -- a building asks its own question at its own door, and the answer has to say who is
/// standing there. Only the three administrative origins convert: `ReferendumCanceller` and
/// `ReferendumKiller` act on this chain's own referenda and have nothing to say elsewhere.
pub type GovernanceToPlurality = (
	OriginToPluralityVoice<
		RuntimeOrigin,
		crate::governance::pezpallet_custom_origins::WelatiElection,
		WelatiElectionBodyId,
	>,
	OriginToPluralityVoice<
		RuntimeOrigin,
		crate::governance::pezpallet_custom_origins::WelatiAdmin,
		WelatiAdminBodyId,
	>,
	OriginToPluralityVoice<
		RuntimeOrigin,
		crate::governance::pezpallet_custom_origins::CitizenshipAdmin,
		CitizenshipAdminBodyId,
	>,
);

impl pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// Users may not send arbitrary XCM from here. The state governance origins may, as the
	// voice of their body; `EnsureXcmOrigin`'s root fallback covers a referendum that carries
	// the whole chain rather than one office.
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, GovernanceToPlurality>;
	type XcmRouter = XcmRouter;
	// We support local origins dispatching XCM executions.
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Nothing; // This teyrchain is not meant as a reserve location.
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::PeoplePezkuwichainXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pezpallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type WeightInfo = crate::weights::pezpallet_xcm::WeightInfo<Runtime>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	// Aliasing is disabled: xcm_executor::Config::Aliasers is set to `Nothing`.
	type AuthorizedAliasConsideration = Disabled;
}

impl pezcumulus_pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}
