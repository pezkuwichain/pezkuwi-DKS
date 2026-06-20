// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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
	AccountId, AllPalletsWithSystem, Balance, Balances, BaseDeliveryFee, FeeAssetId, PezkuwiXcm,
	Runtime, RuntimeCall, RuntimeEvent, RuntimeHoldReason, RuntimeOrigin, TeyrchainInfo,
	TeyrchainSystem, TransactionByteFee, WeightToFee, XcmOverBridgeHubPezkuwichain, XcmpQueue,
};
use crate::bridge_to_ethereum_config::SnowbridgeFrontendLocation;
use pezbridge_hub_common::DenyExportMessageFrom;
use pezframe_support::{
	parameter_types,
	traits::{
		fungible::HoldConsideration, tokens::imbalance::ResolveTo, ConstU32, Contains, Equals,
		Everything, EverythingBut, LinearStoragePrice, Nothing,
	},
};
use pezframe_system::EnsureRoot;
use pezkuwi_runtime_common::xcm_sender::ExponentialPrice;
use pezkuwi_teyrchain_primitives::primitives::Sibling;
use pezpallet_collator_selection::StakingPotAccountId;
use pezpallet_xcm::{AuthorizedAliasers, XcmPassthrough};
use pezsp_runtime::traits::AccountIdConversion;
use testnet_teyrchains_constants::zagros::{
	locations::AssetHubLocation, snowbridge::EthereumNetwork,
};
use teyrchains_common::{
	xcm_config::{
		AllSiblingSystemTeyrchains, ConcreteAssetFromSystem, ParentRelayOrSiblingTeyrchains,
		RelayOrOtherSystemTeyrchains,
	},
	TREASURY_PALLET_ID,
};
use xcm::latest::{prelude::*, ZAGROS_GENESIS_HASH};
use xcm_builder::{
	AccountId32Aliases, AliasChildLocation, AllowExplicitUnpaidExecutionFrom,
	AllowHrmpNotificationsFromRelayChain, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, DenyRecursively, DenyReserveTransferToRelayChain, DenyThenTry,
	DescribeAllTerminal, DescribeFamily, EnsureXcmOrigin, ExternalConsensusLocationsConverterFor,
	FrameTransactionalProcessor, FungibleAdapter, HashedDescription, IsConcrete,
	LocationAsSuperuser, ParentAsSuperuser, ParentIsPreset, RelayChainAsNative,
	SendXcmFeeToAccount, SiblingTeyrchainAsNative, SiblingTeyrchainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
	TrailingSetTopicAsId, UsingComponents, WeightInfoBounds, WithComputedOrigin, WithUniqueTopic,
	XcmFeeManagerFromComponents,
};
use xcm_executor::XcmExecutor;

// Re-export
pub use testnet_teyrchains_constants::zagros::locations::GovernanceLocation;

parameter_types! {
	pub const RootLocation: Location = Location::here();
	pub const ZagrosLocation: Location = Location::parent();
	pub const RelayNetwork: NetworkId = NetworkId::ByGenesis(ZAGROS_GENESIS_HASH);
	pub RelayChainOrigin: RuntimeOrigin = pezcumulus_pezpallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(RelayNetwork::get()), Teyrchain(TeyrchainInfo::teyrchain_id().into())].into();
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
	pub TreasuryAccount: AccountId = TREASURY_PALLET_ID.into_account_truncating();
	pub RelayTreasuryLocation: Location = (Parent, PalletInstance(zagros_runtime_constants::TREASURY_PALLET_ID)).into();
}

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
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
	// Different global consensus locations sovereign accounts.
	ExternalConsensusLocationsConverterFor<UniversalLocation, AccountId>,
);

/// Means for transacting the native currency on this chain.
pub type FungibleTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<ZagrosLocation>,
	// Do a simple punn to convert an AccountId32 Location into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports of `Balances`.
	(),
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Governance location can gain root.
	LocationAsSuperuser<Equals<GovernanceLocation>, RuntimeOrigin>,
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
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
	// Xcm origins can be represented natively under the Xcm pezpallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
	fn contains(location: &Location) -> bool {
		let result = matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]));
		tracing::trace!(target: "xcm::contains", ?location, ?result, "ParentOrParentsPlurality matches");
		result
	}
}

pub type Barrier = TrailingSetTopicAsId<
	DenyThenTry<
		(
			DenyRecursively<DenyReserveTransferToRelayChain>,
			DenyRecursively<
				DenyExportMessageFrom<
					EverythingBut<Equals<AssetHubLocation>>,
					Equals<EthereumNetwork>,
				>,
			>,
		),
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
					// Parent, its pluralities (i.e. governance bodies) and relay treasury
					// pezpallet get free execution.
					AllowExplicitUnpaidExecutionFrom<(
						ParentOrParentsPlurality,
						Equals<RelayTreasuryLocation>,
						Equals<SnowbridgeFrontendLocation>,
						Equals<GovernanceLocation>,
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

/// Locations that will not be charged fees in the executor,
/// either execution or delivery.
/// We only waive fees for system functions, which these locations represent.
pub type WaivedLocations = (
	Equals<RootLocation>,
	RelayOrOtherSystemTeyrchains<AllSiblingSystemTeyrchains, Runtime>,
	Equals<RelayTreasuryLocation>,
);

/// Cases where a remote origin is accepted as trusted Teleporter for a given asset:
/// - NativeToken with the parent Relay Chain and sibling teyrchains.
pub type TrustedTeleporters = ConcreteAssetFromSystem<ZagrosLocation>;

/// Defines origin aliasing rules for this chain.
///
/// - Allow any origin to alias into a child sub-location (equivalent to DescendOrigin),
/// - Allow origins explicitly authorized by the alias target location.
pub type TrustedAliasers = (AliasChildLocation, AuthorizedAliasers<Runtime>);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type XcmEventEmitter = PezkuwiXcm;
	type AssetTransactor = FungibleTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	// BridgeHub does not recognize a reserve location for any asset. Users must teleport Native
	// token where allowed (e.g. with the Relay Chain).
	type IsReserve = ();
	type IsTeleporter = TrustedTeleporters;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::BridgeHubZagrosXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	type Trader = UsingComponents<
		WeightToFee,
		ZagrosLocation,
		AccountId,
		Balances,
		ResolveTo<StakingPotAccountId<Runtime>, Balances>,
	>;
	type ResponseHandler = PezkuwiXcm;
	type AssetTrap = PezkuwiXcm;
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = PezkuwiXcm;
	type SubscriptionService = PezkuwiXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		SendXcmFeeToAccount<Self::AssetTransactor, TreasuryAccount>,
	>;
	type MessageExporter = (
		XcmOverBridgeHubPezkuwichain,
		crate::bridge_to_ethereum_config::SnowbridgeExporterV2,
		crate::bridge_to_ethereum_config::SnowbridgeExporter,
	);
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = TrustedAliasers;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = PezkuwiXcm;
}

pub type PriceForParentDelivery =
	ExponentialPrice<FeeAssetId, BaseDeliveryFee, TransactionByteFee, TeyrchainSystem>;

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
	pub const DepositPerItem: Balance = crate::deposit(1, 0);
	pub const DepositPerByte: Balance = crate::deposit(0, 1);
	pub const AuthorizeAliasHoldReason: RuntimeHoldReason = RuntimeHoldReason::PezkuwiXcm(pezpallet_xcm::HoldReason::AuthorizeAlias);
}

impl pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmRouter = XcmRouter;
	// We want to disallow users sending (arbitrary) XCMs from this chain.
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, ()>;
	// We support local origins dispatching XCM executions.
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Nothing; // This teyrchain is not meant as a reserve location.
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::BridgeHubZagrosXcmWeight<RuntimeCall>,
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
	// xcm_executor::Config::Aliasers also uses pezpallet_xcm::AuthorizedAliasers.
	type AuthorizedAliasConsideration = HoldConsideration<
		AccountId,
		Balances,
		AuthorizeAliasHoldReason,
		LinearStoragePrice<DepositPerItem, DepositPerByte, Balance>,
	>;
}

impl pezcumulus_pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}
