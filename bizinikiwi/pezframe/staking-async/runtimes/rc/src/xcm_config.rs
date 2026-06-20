// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Bizinikiwi.

// Bizinikiwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bizinikiwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bizinikiwi.  If not, see <http://www.gnu.org/licenses/>.

//! XCM configurations for Zagros.

use super::{
	teyrchains_origin, AccountId, AllPalletsWithSystem, Balances, Dmp, FellowshipAdmin,
	GeneralAdmin, ParaId, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, StakingAdmin,
	TransactionByteFee, Treasury, WeightToFee, XcmPallet,
};
use crate::{governance::pezpallet_custom_origins::Treasurer, Balance, RuntimeHoldReason};
use pezframe_support::{
	parameter_types,
	traits::{
		fungible::HoldConsideration, Contains, Equals, Everything, LinearStoragePrice, Nothing,
	},
};
use pezframe_system::EnsureRoot;
use pezkuwi_runtime_common::{
	xcm_sender::{ChildTeyrchainRouter, ExponentialPrice},
	ToAuthor,
};
use pezpallet_staking_async_rc_runtime_constants::{
	currency::CENTS, system_teyrchain::*, xcm::body::FELLOWSHIP_ADMIN_INDEX,
};
use pezpallet_xcm::XcmPassthrough;
use pezsp_core::ConstU32;
use xcm::latest::{prelude::*, ZAGROS_GENESIS_HASH};
use xcm_builder::{
	AccountId32Aliases, AliasChildLocation, AllowExplicitUnpaidExecutionFrom,
	AllowKnownQueryResponses, AllowSubscriptionsFrom, AllowTopLevelPaidExecutionFrom,
	ChildTeyrchainAsNative, ChildTeyrchainConvertsVia, DescribeAllTerminal, DescribeFamily,
	FrameTransactionalProcessor, FungibleAdapter, HashedDescription, IsChildSystemTeyrchain,
	IsConcrete, MintLocation, OriginToPluralityVoice, SendXcmFeeToAccount,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
	TrailingSetTopicAsId, UsingComponents, WeightInfoBounds, WithComputedOrigin, WithUniqueTopic,
	XcmFeeManagerFromComponents,
};
use xcm_executor::XcmExecutor;

parameter_types! {
	pub const TokenLocation: Location = Here.into_location();
	pub const RootLocation: Location = Location::here();
	pub const ThisNetwork: NetworkId = ByGenesis(ZAGROS_GENESIS_HASH);
	pub UniversalLocation: InteriorLocation = [GlobalConsensus(ThisNetwork::get())].into();
	pub CheckAccount: AccountId = XcmPallet::check_account();
	pub LocalCheckAccount: (AccountId, MintLocation) = (CheckAccount::get(), MintLocation::Local);
	pub TreasuryAccount: AccountId = Treasury::account_id();
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(TokenLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
	/// Zagros does not have mint authority anymore after the Asset Hub migration.
	pub TeleportTracking: Option<(AccountId, MintLocation)> = None;
}

pub type LocationConverter = (
	// We can convert a child teyrchain using the standard `AccountId` conversion.
	ChildTeyrchainConvertsVia<ParaId, AccountId>,
	// We can directly alias an `AccountId32` into a local account.
	AccountId32Aliases<ThisNetwork, AccountId>,
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

pub type LocalAssetTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<TokenLocation>,
	// We can convert the Locations with our converter above:
	LocationConverter,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// Teleport tracking
	TeleportTracking,
>;

type LocalOriginConverter = (
	// If the origin kind is `Sovereign`, then return a `Signed` origin with the account determined
	// by the `LocationConverter` converter.
	SovereignSignedViaLocation<LocationConverter, RuntimeOrigin>,
	// If the origin kind is `Native` and the XCM origin is a child teyrchain, then we can express
	// it with the special `teyrchains_origin::Origin` origin variant.
	ChildTeyrchainAsNative<teyrchains_origin::Origin, RuntimeOrigin>,
	// If the origin kind is `Native` and the XCM origin is the `AccountId32` location, then it can
	// be expressed using the `Signed` origin variant.
	SignedAccountId32AsNative<ThisNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pezpallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

pub type PriceForChildTeyrchainDelivery =
	ExponentialPrice<FeeAssetId, BaseDeliveryFee, TransactionByteFee, Dmp>;

/// The XCM router. When we want to send an XCM message, we use this type. It amalgamates all of our
/// individual routers.
pub type XcmRouter = WithUniqueTopic<
	// Only one router so far - use DMP to communicate with child teyrchains.
	ChildTeyrchainRouter<Runtime, XcmPallet, PriceForChildTeyrchainDelivery>,
>;

parameter_types! {
	pub AssetHub: Location = Teyrchain(ASSET_HUB_ID).into_location();
	pub Collectives: Location = Teyrchain(COLLECTIVES_ID).into_location();
	pub BridgeHub: Location = Teyrchain(BRIDGE_HUB_ID).into_location();
	pub Encointer: Location = Teyrchain(ENCOINTER_ID).into_location();
	pub People: Location = Teyrchain(PEOPLE_ID).into_location();
	pub Broker: Location = Teyrchain(BROKER_ID).into_location();
	pub Zgr: AssetFilter = Wild(AllOf { fun: WildFungible, id: AssetId(TokenLocation::get()) });
	pub WndForAssetHub: (AssetFilter, Location) = (Zgr::get(), AssetHub::get());
	pub WndForCollectives: (AssetFilter, Location) = (Zgr::get(), Collectives::get());
	pub WndForBridgeHub: (AssetFilter, Location) = (Zgr::get(), BridgeHub::get());
	pub WndForEncointer: (AssetFilter, Location) = (Zgr::get(), Encointer::get());
	pub WndForPeople: (AssetFilter, Location) = (Zgr::get(), People::get());
	pub WndForBroker: (AssetFilter, Location) = (Zgr::get(), Broker::get());
	pub MaxInstructions: u32 = 100;
	pub MaxAssetsIntoHolding: u32 = 64;
}

pub type TrustedTeleporters = (
	xcm_builder::Case<WndForAssetHub>,
	xcm_builder::Case<WndForCollectives>,
	xcm_builder::Case<WndForBridgeHub>,
	xcm_builder::Case<WndForEncointer>,
	xcm_builder::Case<WndForPeople>,
	xcm_builder::Case<WndForBroker>,
);

pub struct OnlyTeyrchains;
impl Contains<Location> for OnlyTeyrchains {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (0, [Teyrchain(_)]))
	}
}

pub struct Fellows;
impl Contains<Location> for Fellows {
	fn contains(location: &Location) -> bool {
		matches!(
			location.unpack(),
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
			// Subscriptions for version tracking are OK.
			AllowSubscriptionsFrom<OnlyTeyrchains>,
			// Messages from system teyrchains or the Fellows plurality need not pay for execution.
			AllowExplicitUnpaidExecutionFrom<(IsChildSystemTeyrchain<ParaId>, Fellows)>,
		),
		UniversalLocation,
		ConstU32<8>,
	>,
)>;

/// Locations that will not be charged fees in the executor, neither for execution nor delivery.
/// We only waive fees for system functions, which these locations represent.
pub type WaivedLocations = (SystemTeyrchains, Equals<RootLocation>, LocalPlurality);

/// We let locations alias into child locations of their own.
/// This is a very simple aliasing rule, mimicking the behaviour of
/// the `DescendOrigin` instruction.
pub type Aliasers = AliasChildLocation;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = ();
	type XcmEventEmitter = XcmPallet;
	type IsTeleporter = TrustedTeleporters;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::ZagrosXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
	type Trader =
		UsingComponents<WeightToFee, TokenLocation, AccountId, Balances, ToAuthor<Runtime>>;
	type ResponseHandler = XcmPallet;
	type AssetTrap = XcmPallet;
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = XcmPallet;
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
	type Aliasers = Aliasers;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = XcmPallet;
}

parameter_types! {
	// `GeneralAdmin` pluralistic body.
	pub const GeneralAdminBodyId: BodyId = BodyId::Administration;
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
	// FellowshipAdmin pluralistic body.
	pub const FellowshipAdminBodyId: BodyId = BodyId::Index(FELLOWSHIP_ADMIN_INDEX);
	// `Treasurer` pluralistic body.
	pub const TreasurerBodyId: BodyId = BodyId::Treasury;

	pub const DepositPerItem: Balance = crate::deposit(1, 0);
	pub const DepositPerByte: Balance = crate::deposit(0, 1);
	pub const AuthorizeAliasHoldReason: RuntimeHoldReason = RuntimeHoldReason::XcmPallet(pezpallet_xcm::HoldReason::AuthorizeAlias);
}

/// Type to convert the `GeneralAdmin` origin to a Plurality `Location` value.
pub type GeneralAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, GeneralAdmin, GeneralAdminBodyId>;

/// location of this chain.
pub type LocalOriginToLocation = (
	GeneralAdminToPlurality,
	// And a usual Signed origin to be used in XCM as a corresponding AccountId32
	SignedToAccountId32<RuntimeOrigin, AccountId, ThisNetwork>,
);

/// Type to convert the `StakingAdmin` origin to a Plurality `Location` value.
pub type StakingAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, StakingAdmin, StakingAdminBodyId>;

/// Type to convert the `FellowshipAdmin` origin to a Plurality `Location` value.
pub type FellowshipAdminToPlurality =
	OriginToPluralityVoice<RuntimeOrigin, FellowshipAdmin, FellowshipAdminBodyId>;

/// Type to convert the `Treasurer` origin to a Plurality `Location` value.
pub type TreasurerToPlurality = OriginToPluralityVoice<RuntimeOrigin, Treasurer, TreasurerBodyId>;

/// Type to convert a pezpallet `Origin` type value into a `Location` value which represents an
/// interior location of this chain for a destination chain.
pub type LocalPalletOriginToLocation = (
	// GeneralAdmin origin to be used in XCM as a corresponding Plurality `Location` value.
	GeneralAdminToPlurality,
	// StakingAdmin origin to be used in XCM as a corresponding Plurality `Location` value.
	StakingAdminToPlurality,
	// FellowshipAdmin origin to be used in XCM as a corresponding Plurality `Location` value.
	FellowshipAdminToPlurality,
	// `Treasurer` origin to be used in XCM as a corresponding Plurality `Location` value.
	TreasurerToPlurality,
);

impl pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// Note that this configuration of `SendXcmOrigin` is different from the one present in
	// production.
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<
		RuntimeOrigin,
		(LocalPalletOriginToLocation, LocalOriginToLocation),
	>;
	type XcmRouter = XcmRouter;
	// Anyone can execute XCM messages locally.
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = WeightInfoBounds<
		crate::weights::xcm::ZagrosXcmWeight<RuntimeCall>,
		RuntimeCall,
		MaxInstructions,
	>;
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
	type AuthorizedAliasConsideration = HoldConsideration<
		AccountId,
		Balances,
		AuthorizeAliasHoldReason,
		LinearStoragePrice<DepositPerItem, DepositPerByte, Balance>,
	>;
}
