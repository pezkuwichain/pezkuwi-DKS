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

//! Teyrchain runtime mock.

mod contracts_config;
use crate::{
	mocks::msg_queue::pezpallet as mock_msg_queue,
	primitives::{AccountId, AssetIdForAssets, Balance},
};
use core::marker::PhantomData;
use pezframe_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{
		AsEnsureOriginWithArg, Contains, ContainsPair, Disabled, Everything, EverythingBut, Nothing,
	},
	weights::{
		constants::{WEIGHT_PROOF_SIZE_PER_MB, WEIGHT_REF_TIME_PER_SECOND},
		Weight,
	},
};
use pezframe_system::{EnsureRoot, EnsureSigned};
use pezpallet_xcm::XcmPassthrough;
use pezsp_core::{ConstU32, ConstU64, H256};
use pezsp_runtime::traits::{Get, IdentityLookup, MaybeEquivalence};

use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowTopLevelPaidExecutionFrom,
	ConvertedConcreteId, EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds,
	FrameTransactionalProcessor, FungibleAdapter, FungiblesAdapter, IsConcrete, NativeAsset,
	NoChecking, ParentAsSuperuser, ParentIsPreset, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation, WithComputedOrigin,
};
use xcm_executor::{traits::JustTry, Config, XcmExecutor};

pub type SovereignAccountOf =
	(AccountId32Aliases<RelayNetwork, AccountId>, ParentIsPreset<AccountId>);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Block = Block;
	type Hash = H256;
	type Hashing = ::pezsp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pezpallet_balances::Config for Runtime {
	type AccountStore = System;
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const AssetDeposit: u128 = 1_000_000;
	pub const MetadataDepositBase: u128 = 1_000_000;
	pub const MetadataDepositPerByte: u128 = 100_000;
	pub const AssetAccountDeposit: u128 = 1_000_000;
	pub const ApprovalDeposit: u128 = 1_000_000;
	pub const AssetsStringLimit: u32 = 50;
	pub const RemoveItemsLimit: u32 = 50;
}

impl pezpallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetIdForAssets;
	type ReserveData = ();
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type AssetAccountDeposit = AssetAccountDeposit;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
	type RemoveItemsLimit = RemoveItemsLimit;
	type AssetIdParameter = AssetIdForAssets;
	type CallbackHandle = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
	pub const ReservedDmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
}

parameter_types! {
	pub const KsmLocation: Location = Location::parent();
	pub const TokenLocation: Location = Here.into_location();
	pub const RelayNetwork: NetworkId = ByGenesis([0; 32]);
	pub UniversalLocation: InteriorLocation = [GlobalConsensus(RelayNetwork::get()), Teyrchain(MsgQueue::teyrchain_id().into())].into();
}

pub type XcmOriginToCallOrigin = (
	SovereignSignedViaLocation<SovereignAccountOf, RuntimeOrigin>,
	ParentAsSuperuser<RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	pub const XcmInstructionWeight: Weight = Weight::from_parts(1_000, 1_000);
	pub TokensPerSecondPerMegabyte: (AssetId, u128, u128) = (AssetId(Parent.into()), 1_000_000_000_000, 1024 * 1024);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
	pub ForeignPrefix: Location = (Parent,).into();
	pub CheckingAccount: AccountId = PezkuwiXcm::check_account();
	pub TrustedLockPairs: (Location, AssetFilter) =
	(Parent.into(), Wild(AllOf { id: AssetId(Parent.into()), fun: WildFungible }));
}

pub fn estimate_message_fee(number_of_instructions: u64) -> u128 {
	let weight = estimate_weight(number_of_instructions);

	estimate_fee_for_weight(weight)
}

pub fn estimate_weight(number_of_instructions: u64) -> Weight {
	XcmInstructionWeight::get().saturating_mul(number_of_instructions)
}

pub fn estimate_fee_for_weight(weight: Weight) -> u128 {
	let (_, units_per_second, units_per_mb) = TokensPerSecondPerMegabyte::get();

	units_per_second * (weight.ref_time() as u128) / (WEIGHT_REF_TIME_PER_SECOND as u128)
		+ units_per_mb * (weight.proof_size() as u128) / (WEIGHT_PROOF_SIZE_PER_MB as u128)
}

pub type LocalBalancesTransactor =
	FungibleAdapter<Balances, IsConcrete<TokenLocation>, SovereignAccountOf, AccountId, ()>;

pub struct FromLocationToAsset<Location, AssetId>(PhantomData<(Location, AssetId)>);
impl MaybeEquivalence<Location, AssetIdForAssets>
	for FromLocationToAsset<Location, AssetIdForAssets>
{
	fn convert(value: &Location) -> Option<AssetIdForAssets> {
		match value.unpack() {
			(1, []) => Some(0 as AssetIdForAssets),
			(1, [Teyrchain(para_id)]) => Some(*para_id as AssetIdForAssets),
			_ => None,
		}
	}

	fn convert_back(_id: &AssetIdForAssets) -> Option<Location> {
		None
	}
}

pub type ForeignAssetsTransactor = FungiblesAdapter<
	Assets,
	ConvertedConcreteId<
		AssetIdForAssets,
		Balance,
		FromLocationToAsset<Location, AssetIdForAssets>,
		JustTry,
	>,
	SovereignAccountOf,
	AccountId,
	NoChecking,
	CheckingAccount,
>;

/// Means for transacting assets on this chain
pub type AssetTransactors = (LocalBalancesTransactor, ForeignAssetsTransactor);

pub struct ParentRelay;
impl Contains<Location> for ParentRelay {
	fn contains(location: &Location) -> bool {
		location.contains_parents_only(1)
	}
}
pub struct ThisTeyrchain;
impl Contains<Location> for ThisTeyrchain {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (0, [Junction::AccountId32 { .. }]))
	}
}

pub type XcmRouter = crate::TeyrchainXcmRouter<MsgQueue>;

pub type Barrier = (
	xcm_builder::AllowUnpaidExecutionFrom<ThisTeyrchain>,
	WithComputedOrigin<
		(AllowExplicitUnpaidExecutionFrom<ParentRelay>, AllowTopLevelPaidExecutionFrom<Everything>),
		UniversalLocation,
		ConstU32<1>,
	>,
);

parameter_types! {
	pub NftCollectionOne: AssetFilter
		= Wild(AllOf { fun: WildNonFungible, id: AssetId((Parent, GeneralIndex(1)).into()) });
	pub NftCollectionOneForRelay: (AssetFilter, Location)
		= (NftCollectionOne::get(), Parent.into());
	pub RelayNativeAsset: AssetFilter = Wild(AllOf { fun: WildFungible, id: AssetId((Parent, Here).into()) });
	pub RelayNativeAssetForRelay: (AssetFilter, Location) = (RelayNativeAsset::get(), Parent.into());
}
pub type TrustedTeleporters =
	(xcm_builder::Case<NftCollectionOneForRelay>, xcm_builder::Case<RelayNativeAssetForRelay>);
pub type TrustedReserves = EverythingBut<xcm_builder::Case<NftCollectionOneForRelay>>;

pub struct XcmConfig;
impl Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type XcmEventEmitter = PezkuwiXcm;
	type AssetTransactor = AssetTransactors;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = (NativeAsset, TrustedReserves);
	type IsTeleporter = TrustedTeleporters;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<XcmInstructionWeight, RuntimeCall, MaxInstructions>;
	type Trader = FixedRateOfFungible<TokensPerSecondPerMegabyte, ()>;
	type ResponseHandler = PezkuwiXcm;
	type AssetTrap = PezkuwiXcm;
	type AssetLocker = PezkuwiXcm;
	type AssetExchanger = ();
	type AssetClaims = PezkuwiXcm;
	type SubscriptionService = PezkuwiXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type FeeManager = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
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

impl mock_msg_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

pub struct TrustedLockerCase<T>(PhantomData<T>);
impl<T: Get<(Location, AssetFilter)>> ContainsPair<Location, Asset> for TrustedLockerCase<T> {
	fn contains(origin: &Location, asset: &Asset) -> bool {
		let (o, a) = T::get();
		a.matches(asset) && &o == origin
	}
}

impl pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<XcmInstructionWeight, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pezpallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = IsConcrete<TokenLocation>;
	type TrustedLockers = TrustedLockerCase<TrustedLockPairs>;
	type SovereignAccountOf = SovereignAccountOf;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type WeightInfo = pezpallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	// Aliasing is disabled: xcm_executor::Config::Aliasers is set to `Nothing`.
	type AuthorizedAliasConsideration = Disabled;
}

type Block = pezframe_system::mocking::MockBlock<Runtime>;

impl pezpallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

construct_runtime!(
	pub enum Runtime
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		Timestamp: pezpallet_timestamp,
		MsgQueue: mock_msg_queue,
		PezkuwiXcm: pezpallet_xcm,
		Contracts: pezpallet_contracts,
		Assets: pezpallet_assets,
	}
);
