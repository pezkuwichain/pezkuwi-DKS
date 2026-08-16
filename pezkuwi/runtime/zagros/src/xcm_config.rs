// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! XCM configuration for Pezkuwichain.

use super::{
	teyrchains_origin, AccountId, AllPalletsWithSystem, Balances, Dmp, Fellows, ParaId, Runtime,
	RuntimeCall, RuntimeEvent, RuntimeOrigin, TransactionByteFee, Treasurer, Treasury, WeightToFee,
	XcmPallet,
};

use crate::governance::{CitizenshipAdmin, StakingAdmin, WelatiAdmin, WelatiElection};

use pezframe_support::{
	parameter_types,
	traits::{Contains, Disabled, Equals, Everything, Nothing},
	weights::Weight,
};
use pezframe_system::EnsureRoot;
use pezkuwi_runtime_common::{
	xcm_sender::{ChildTeyrchainRouter, ExponentialPrice},
	ToAuthor,
};
use pezpallet_xcm::XcmPassthrough;
use pezsp_core::ConstU32;
use xcm::latest::{prelude::*, ZAGROS_GENESIS_HASH};
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses,
	AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom, ChildTeyrchainAsNative,
	ChildTeyrchainConvertsVia, DescribeAllTerminal, DescribeFamily, FixedWeightBounds,
	FrameTransactionalProcessor, FungibleAdapter, HashedDescription, IsChildSystemTeyrchain,
	IsConcrete, MintLocation, OriginToPluralityVoice, SendXcmFeeToAccount,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
	TrailingSetTopicAsId, UsingComponents, WeightInfoBounds, WithComputedOrigin, WithUniqueTopic,
	XcmFeeManagerFromComponents,
};
use xcm_executor::XcmExecutor;
use zagros_runtime_constants::{currency::CENTS, system_teyrchain::*};

parameter_types! {
	pub TokenLocation: Location = Here.into_location();
	pub RootLocation: Location = Location::here();
	// This chain is Zagros, not the mainnet it mirrors. Mirroring the runtimes copied
	// the mainnet's XCM identity along with its structure, which made three of the five
	// Zagros chains announce themselves as the Pezkuwichain network — the same
	// impersonation the chain-spec reset exists to end, one layer lower.
	pub const ThisNetwork: NetworkId = NetworkId::ByGenesis(ZAGROS_GENESIS_HASH);
	pub UniversalLocation: InteriorLocation = ThisNetwork::get().into();
	pub CheckAccount: AccountId = XcmPallet::check_account();
	/// Track what teleports away and what comes back.
	///
	/// The comment this replaces said the relay has no mint authority since the Asset Hub
	/// migration, but `None` does not remove mint authority — it removes the accounting. The
	/// chain still accepts a teleport from any trusted teyrchain and mints on receipt; it simply
	/// keeps no record that the supply ever left. Measured on the mainnet, which carries the same
	/// setting: the checking account has never existed and no teyrchain sovereign account exists
	/// either, so nothing has ever backed a teleport in this direction.
	///
	/// With `MintLocation::Local` the chain can only mint back what previously left, because an
	/// arriving teleport is checked in against this account. That is the property worth having,
	/// and it is what the relay-side teleport test describes. It requires the account to be
	/// seeded, which is a genesis matter rather than a runtime one.
	pub TeleportTracking: Option<(AccountId, MintLocation)> = Some((CheckAccount::get(), MintLocation::Local));
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

pub type LocationConverter = (
	// We can convert a child teyrchain using the standard `AccountId` conversion.
	ChildTeyrchainConvertsVia<ParaId, AccountId>,
	// We can directly alias an `AccountId32` into a local account.
	AccountId32Aliases<ThisNetwork, AccountId>,
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

/// Our asset transactor. This is what allows us to interest with the runtime facilities from the
/// point of view of XCM-only concepts like `Location` and `Asset`.
///
/// Ours is only aware of the Balances pezpallet, which is mapped to `RocLocation`.
pub type LocalAssetTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<TokenLocation>,
	// We can convert the Locations with our converter above:
	LocationConverter,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	TeleportTracking,
>;

/// The means that we convert the XCM message origin location into a local dispatch origin.
type LocalOriginConverter = (
	// A `Signed` origin of the sovereign account that the original location controls.
	SovereignSignedViaLocation<LocationConverter, RuntimeOrigin>,
	// A child teyrchain, natively expressed, has the `Teyrchain` origin.
	ChildTeyrchainAsNative<teyrchains_origin::Origin, RuntimeOrigin>,
	// The AccountId32 location type can be expressed natively as a `Signed` origin.
	SignedAccountId32AsNative<ThisNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pezpallet's Xcm origin. Without this
	// there is no converter for `OriginKind::Xcm` at all, so a `Transact` sent that way is
	// rejected with `BadOrigin` no matter who sent it — which is how the Fellowship's whitelist
	// call was refused even before its origin could be judged.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	/// The amount of weight an XCM operation takes. This is a safe overestimate.
	pub const BaseXcmWeight: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(TokenLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

pub type PriceForChildTeyrchainDelivery =
	ExponentialPrice<FeeAssetId, BaseDeliveryFee, TransactionByteFee, Dmp>;

/// The XCM router. When we want to send an XCM message, we use this type. It amalgamates all of our
/// individual routers.
pub type XcmRouter = WithUniqueTopic<
	// Only one router so far - use DMP to communicate with child teyrchains.
	ChildTeyrchainRouter<Runtime, XcmPallet, PriceForChildTeyrchainDelivery>,
>;

parameter_types! {
	pub Tyr: AssetFilter = Wild(AllOf { fun: WildFungible, id: AssetId(TokenLocation::get()) });
	pub AssetHub: Location = Teyrchain(ASSET_HUB_ID).into_location();
	pub Collectives: Location = Teyrchain(COLLECTIVES_ID).into_location();
	pub BridgeHub: Location = Teyrchain(BRIDGE_HUB_ID).into_location();
	pub People: Location = Teyrchain(PEOPLE_ID).into_location();
	pub Broker: Location = Teyrchain(BROKER_ID).into_location();
	pub TyrForAssetHub: (AssetFilter, Location) = (Tyr::get(), AssetHub::get());
	pub TyrForCollectives: (AssetFilter, Location) = (Tyr::get(), Collectives::get());
	pub TyrForBridgeHub: (AssetFilter, Location) = (Tyr::get(), BridgeHub::get());
	pub TyrForPeople: (AssetFilter, Location) = (Tyr::get(), People::get());
	pub TyrForBroker: (AssetFilter, Location) = (Tyr::get(), Broker::get());
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}
/// The system teyrchains this relay will teleport its native token to and from.
///
/// One entry per chain that actually exists in this ecosystem. The list the mirror brought over
/// named chains from another network — `Tick` (100), `Trick` (110), `Track` (120) and
/// `Encointer` (1003) have no runtime here, and `Contracts` resolved to 1002, which is the
/// Bridge Hub, so it trusted the same chain twice. Collectives, which does exist and whose own
/// teleport tests exercise this, had been dropped.
pub type TrustedTeleporters = (
	xcm_builder::Case<TyrForAssetHub>,
	xcm_builder::Case<TyrForCollectives>,
	xcm_builder::Case<TyrForBridgeHub>,
	xcm_builder::Case<TyrForPeople>,
	xcm_builder::Case<TyrForBroker>,
);

pub struct OnlyTeyrchains;
impl Contains<Location> for OnlyTeyrchains {
	fn contains(loc: &Location) -> bool {
		matches!(loc.unpack(), (0, [Teyrchain(_)]))
	}
}

/// The Fellowship's voice on the Collectives chain, as this chain sees it.
///
/// The Fellowship is a body on Collectives, not here, so a message it sends arrives as that
/// chain's id followed by its plurality. `IsChildSystemTeyrchain` matches a bare teyrchain and
/// stops there, which is why the trailing `Plurality` made every Fellowship message fail the
/// barrier before it could be read — including the one that whitelists a call for the
/// `whitelisted_caller` track, leaving that track reachable by root alone.
pub struct FellowsPlurality;
impl Contains<Location> for FellowsPlurality {
	fn contains(loc: &Location) -> bool {
		matches!(
			loc.unpack(),
			(0, [Teyrchain(COLLECTIVES_ID), Plurality { id: BodyId::Technical, .. }])
		)
	}
}

pub struct LocalPlurality;
impl Contains<Location> for LocalPlurality {
	fn contains(loc: &Location) -> bool {
		matches!(loc.unpack(), (0, [Plurality { .. }]))
	}
}

/// The barriers one of which must be passed for an XCM message to be executed.
pub type Barrier = TrailingSetTopicAsId<(
	// Weight that is paid for may be consumed.
	TakeWeightCredit,
	// Expected responses are OK.
	AllowKnownQueryResponses<XcmPallet>,
	WithComputedOrigin<
		(
			// If the message is one that immediately attempts to pay for execution, then allow it.
			AllowTopLevelPaidExecutionFrom<Everything>,
			// Messages from system teyrchains or the Fellows plurality need not pay for execution.
			AllowExplicitUnpaidExecutionFrom<(IsChildSystemTeyrchain<ParaId>, FellowsPlurality)>,
			// Subscriptions for version tracking are OK.
			AllowSubscriptionsFrom<OnlyTeyrchains>,
		),
		UniversalLocation,
		ConstU32<8>,
	>,
)>;

/// Locations that will not be charged fees in the executor, neither for execution nor delivery.
/// We only waive fees for system functions, which these locations represent.
pub type WaivedLocations = (SystemTeyrchains, Equals<RootLocation>, LocalPlurality);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type XcmEventEmitter = XcmPallet;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = ();
	type IsTeleporter = TrustedTeleporters;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::PezkuwichainXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	type Trader =
		UsingComponents<WeightToFee, TokenLocation, AccountId, Balances, ToAuthor<Runtime>>;
	type ResponseHandler = XcmPallet;
	type AssetTrap = XcmPallet;
	type AssetLocker = ();
	type AssetExchanger = ();
	type SubscriptionService = XcmPallet;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		SendXcmFeeToAccount<Self::AssetTransactor, TreasuryAccount>,
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
	type XcmRecorder = XcmPallet;
}

parameter_types! {
	/// Collective pluralistic body.
	pub const CollectiveBodyId: BodyId = BodyId::Unit;
	/// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
	/// Fellows pluralistic body.
	pub const FellowsBodyId: BodyId = BodyId::Technical;
	/// Treasury pluralistic body.
	pub const TreasuryBodyId: BodyId = BodyId::Treasury;
	/// Welati Election pluralistic body (People Chain governance via XCM).
	pub const WelatiElectionBodyId: BodyId = BodyId::Index(40);
	/// Welati Admin pluralistic body (People Chain tiki/appointment admin via XCM).
	pub const WelatiAdminBodyId: BodyId = BodyId::Index(41);
	/// Citizenship Admin pluralistic body (People Chain citizenship mgmt via XCM).
	pub const CitizenshipAdminBodyId: BodyId = BodyId::Index(42);
}

/// Type to convert an `Origin` type value into a `Location` value which represents an interior
/// location of this chain.
pub type LocalOriginToLocation = (
	// And a usual Signed origin to be used in XCM as a corresponding AccountId32
	SignedToAccountId32<RuntimeOrigin, AccountId, ThisNetwork>,
);

/// Type to convert the `StakingAdmin` origin to a Plurality `Location` value.
pub type StakingAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, StakingAdmin, StakingAdminBodyId>;

/// Type to convert the Fellows origin to a Plurality `Location` value.
pub type FellowsToPlurality = OriginToPluralityVoice<RuntimeOrigin, Fellows, FellowsBodyId>;

/// Type to convert the Treasury origin to a Plurality `Location` value.
pub type TreasurerToPlurality = OriginToPluralityVoice<RuntimeOrigin, Treasurer, TreasuryBodyId>;

/// Welati governance origin to Plurality converters (RC → People Chain via XCM).
pub type WelatiElectionToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, WelatiElection, WelatiElectionBodyId>;
pub type WelatiAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, WelatiAdmin, WelatiAdminBodyId>;
pub type CitizenshipAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, CitizenshipAdmin, CitizenshipAdminBodyId>;

/// Type to convert a pezpallet `Origin` type value into a `Location` value which represents an
/// interior location of this chain for a destination chain.
pub type LocalPalletOriginToLocation = (
	// StakingAdmin origin to be used in XCM as a corresponding Plurality `Location` value.
	StakingAdminToPlurality,
	// Fellows origin to be used in XCM as a corresponding Plurality `Location` value.
	FellowsToPlurality,
	// Treasurer origin to be used in XCM as a corresponding Plurality `Location` value.
	TreasurerToPlurality,
	// Welati governance origins — enable RC OpenGov to dispatch XCM to People Chain.
	WelatiElectionToPlurality,
	WelatiAdminToPlurality,
	CitizenshipAdminToPlurality,
);

impl pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// Production relay: only governance-controlled pallet origins (StakingAdmin, Fellows,
	// Treasurer, Welati bodies) may originate raw `pezpallet_xcm::send` messages. Ordinary signed
	// accounts are intentionally excluded here (unlike the zagros/testnet config) so that no
	// funded relay account can craft arbitrary XCM programs toward any current or future
	// teyrchain. Local execution for signed accounts is still permitted via `ExecuteXcmOrigin`
	// below, which is scoped to the caller's own derived-sovereign origin and does not grant
	// send-to-any-destination capability.
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalPalletOriginToLocation>;
	type XcmRouter = XcmRouter;
	// Anyone can execute XCM messages locally.
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	// Anyone is able to use reserve transfers regardless of who they are and what they want to
	// transfer.
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pezpallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = IsConcrete<TokenLocation>;
	type TrustedLockers = ();
	type SovereignAccountOf = LocationConverter;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type WeightInfo = crate::weights::pezpallet_xcm::WeightInfo<Runtime>;
	type AdminOrigin = EnsureRoot<AccountId>;
	// Aliasing is disabled: xcm_executor::Config::Aliasers is set to `Nothing`.
	type AuthorizedAliasConsideration = Disabled;
}
