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

//! # Asset Hub Pezkuwichain Runtime
//!
//! Asset Hub Pezkuwichain, formerly known as "Rockmine", is the test network for its Dicle cousin.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod bag_thresholds;
mod genesis_config_presets;
mod staking;
mod weights;
pub mod xcm_config;

// Re-export staking configurations
pub use staking::*;

extern crate alloc;

use alloc::{vec, vec::Vec};
use pez_assets_common::{
	foreign_creators::ForeignCreators,
	local_and_foreign_assets::{LocalFromLeft, TargetFromLeft},
	matching::{FromNetwork, FromSiblingTeyrchain},
	AssetIdForPoolAssets, AssetIdForPoolAssetsConvert, AssetIdForTrustBackedAssetsConvert,
};
use pezbp_asset_hub_pezkuwichain::CreateForeignAssetDeposit;
use pezcumulus_pezpallet_teyrchain_system::{
	RelayNumberMonotonicallyIncreases, RelaychainDataProvider,
};
use pezcumulus_primitives_core::AggregateMessageOrigin;
use pezsp_api::impl_runtime_apis;
use pezsp_core::{crypto::KeyTypeId, OpaqueMetadata};
use pezsp_runtime::{
	generic, impl_opaque_keys,
	traits::{AccountIdConversion, BlakeTwo256, Block as BlockT, Saturating, Verify},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, Permill, Perquintill,
};
use testnet_teyrchains_constants::pezkuwichain::snowbridge::EthereumNetwork;

#[cfg(feature = "std")]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
pub use pez_assets_common::local_and_foreign_assets::ForeignAssetReserveData;
use pezcumulus_primitives_core::ParaId;
use pezframe_support::{
	construct_runtime, derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	ord_parameter_types, parameter_types,
	traits::{
		fungible, fungible::HoldConsideration, fungibles, tokens::imbalance::ResolveAssetTo,
		AsEnsureOriginWithArg, ConstBool, ConstU128, ConstU32, ConstU64, ConstU8,
		ConstantStoragePrice, EitherOfDiverse, Equals, InstanceFilter, Nothing, TransformOrigin,
	},
	weights::{ConstantMultiplier, Weight},
	BoundedVec, PalletId,
};
use pezframe_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureSigned, EnsureSignedBy,
};
use pezpallet_asset_conversion_tx_payment::SwapAssetAdapter;
use pezpallet_nfts::PalletFeatures;
use pezsp_runtime::{Perbill, RuntimeDebug};
use testnet_teyrchains_constants::pezkuwichain::{
	consensus::*, currency::*, fee::WeightToFee, time::*,
};
use teyrchains_common::{
	impls::DealWithFees,
	message_queue::{NarrowOriginToSibling, ParaIdToSibling},
	AccountId, AssetIdForTrustBackedAssets, AuraId, Balance, BlockNumber, CollectionId, Hash,
	Header, ItemId, Nonce, Signature, AVERAGE_ON_INITIALIZE_RATIO, NORMAL_DISPATCH_RATIO,
};
use xcm_config::{
	ForeignAssetsConvertedConcreteId, GovernanceLocation, LocationToAccountId,
	PoolAssetsConvertedConcreteId, PoolAssetsPalletLocation, TokenLocation,
	TrustBackedAssetsConvertedConcreteId, TrustBackedAssetsPalletLocation, XcmConfig,
};

#[cfg(test)]
mod tests;

#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;

// Pezkuwi imports
use pezkuwi_runtime_common::{prod_or_fast, BlockHashCount, SlowAdjustingFeeUpdate};
use pezpallet_xcm::{EnsureXcm, IsVoiceOfBody};
#[cfg(feature = "runtime-benchmarks")]
use xcm::latest::prelude::{
	Asset, Assets as XcmAssets, Fungible, Here, InteriorLocation, Junction, Junction::*, Location,
	NetworkId, NonFungible, ParentThen, Response, WeightLimit, XCM_VERSION,
};
use xcm::{
	latest::prelude::{AssetId, BodyId},
	Version as XcmVersion, VersionedAsset, VersionedAssetId, VersionedAssets, VersionedLocation,
	VersionedXcm,
};
use xcm_runtime_pezapis::{
	dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
	fees::Error as XcmPaymentApiError,
};

#[cfg(feature = "runtime-benchmarks")]
use pezframe_support::traits::PalletInfoAccess;
use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("asset-hub-pezkuwichain"),
	impl_name: alloc::borrow::Cow::Borrowed("asset-hub-pezkuwichain"),
	authoring_version: 1,
	spec_version: 1_020_007,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 16,
	system_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.
#[derive_impl(pezframe_system::config_preludes::TeyrchainDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type AccountId = AccountId;
	type Nonce = Nonce;
	type Hash = Hash;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type DbWeight = RocksDbWeight;
	type Version = Version;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type SystemWeightInfo = weights::pezframe_system::WeightInfo<Runtime>;
	type ExtensionsWeightInfo = weights::pezframe_system_extensions::WeightInfo<Runtime>;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = pezcumulus_pezpallet_teyrchain_system::TeyrchainSetCode<Self>;
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
	type SingleBlockMigrations = Migrations;
}

impl pezcumulus_pezpallet_weight_reclaim::Config for Runtime {
	type WeightInfo = weights::pezcumulus_pezpallet_weight_reclaim::WeightInfo<Runtime>;
}

impl pezpallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<0>;
	type WeightInfo = weights::pezpallet_timestamp::WeightInfo<Runtime>;
}

impl pezpallet_authorship::Config for Runtime {
	type FindAuthor = pezpallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = (CollatorSelection,);
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pezpallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = weights::pezpallet_balances::WeightInfo<Runtime>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<50>;
	type DoneSlashHandler = ();
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = MILLICENTS;
}

impl pezpallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction =
		pezpallet_transaction_payment::FungibleAdapter<Balances, DealWithFees<Runtime>>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightInfo = weights::pezpallet_transaction_payment::WeightInfo<Runtime>;
}

parameter_types! {
	pub const AssetDeposit: Balance = UNITS / 10; // 1 / 10 UNITS deposit to create asset
	pub const AssetAccountDeposit: Balance = deposit(1, 16);
	pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/pezkuwichain/pezkuwi-sdk/blob/main/bizinikiwi/pezframe/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = deposit(1, 68);
	pub const MetadataDepositPerByte: Balance = deposit(0, 1);
}

/// We allow root to execute privileged asset operations.
pub type AssetsForceOrigin = EnsureRoot<AccountId>;

// Called "Trust Backed" assets because these are generally registered by some account, and users of
// the asset assume it has some claimed backing. The pezpallet is called `Assets` in
// `construct_runtime` to avoid breaking changes on storage reads.
pub type TrustBackedAssetsInstance = pezpallet_assets::Instance1;
type TrustBackedAssetsCall = pezpallet_assets::Call<Runtime, TrustBackedAssetsInstance>;
impl pezpallet_assets::Config<TrustBackedAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetIdForTrustBackedAssets;
	type AssetIdParameter = codec::Compact<AssetIdForTrustBackedAssets>;
	type ReserveData = ();
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = AssetsForceOrigin;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Holder = ();
	type Freezer = AssetsFreezer;
	type Extra = ();
	type WeightInfo = weights::pezpallet_assets_local::WeightInfo<Runtime>;
	type CallbackHandle = pezpallet_assets::AutoIncAssetId<Runtime, TrustBackedAssetsInstance>;
	type AssetAccountDeposit = AssetAccountDeposit;
	type RemoveItemsLimit = pezframe_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

// Allow Freezes for the `Assets` pezpallet
pub type AssetsFreezerInstance = pezpallet_assets_freezer::Instance1;
impl pezpallet_assets_freezer::Config<AssetsFreezerInstance> for Runtime {
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
	pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);
}

ord_parameter_types! {
	pub const AssetConversionOrigin: pezsp_runtime::AccountId32 =
		AccountIdConversion::<pezsp_runtime::AccountId32>::into_account_truncating(&AssetConversionPalletId::get());
}

pub type PoolAssetsInstance = pezpallet_assets::Instance3;
impl pezpallet_assets::Config<PoolAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetId = AssetIdForPoolAssets;
	type AssetIdParameter = u32;
	type ReserveData = ();
	type Currency = Balances;
	type CreateOrigin =
		AsEnsureOriginWithArg<EnsureSignedBy<AssetConversionOrigin, pezsp_runtime::AccountId32>>;
	type ForceOrigin = AssetsForceOrigin;
	// Deposits are zero because creation/admin is limited to Asset Conversion pezpallet.
	type AssetDeposit = ConstU128<0>;
	type AssetAccountDeposit = ConstU128<0>;
	type MetadataDepositBase = ConstU128<0>;
	type MetadataDepositPerByte = ConstU128<0>;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = ConstU32<50>;
	type Holder = ();
	type Freezer = PoolAssetsFreezer;
	type Extra = ();
	type WeightInfo = weights::pezpallet_assets_pool::WeightInfo<Runtime>;
	type CallbackHandle = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

// Allow Freezes for the `PoolAssets` pezpallet
pub type PoolAssetsFreezerInstance = pezpallet_assets_freezer::Instance3;
impl pezpallet_assets_freezer::Config<PoolAssetsFreezerInstance> for Runtime {
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeEvent = RuntimeEvent;
}

/// Union fungibles implementation for `Assets` and `ForeignAssets`.
pub type LocalAndForeignAssets = fungibles::UnionOf<
	Assets,
	ForeignAssets,
	LocalFromLeft<
		AssetIdForTrustBackedAssetsConvert<TrustBackedAssetsPalletLocation, xcm::v5::Location>,
		AssetIdForTrustBackedAssets,
		xcm::v5::Location,
	>,
	xcm::v5::Location,
	AccountId,
>;

/// Union fungibles implementation for `AssetsFreezer` and `ForeignAssetsFreezer`.
pub type LocalAndForeignAssetsFreezer = fungibles::UnionOf<
	AssetsFreezer,
	ForeignAssetsFreezer,
	LocalFromLeft<
		AssetIdForTrustBackedAssetsConvert<TrustBackedAssetsPalletLocation, xcm::v5::Location>,
		AssetIdForTrustBackedAssets,
		xcm::v5::Location,
	>,
	xcm::v5::Location,
	AccountId,
>;

/// Union fungibles implementation for [`LocalAndForeignAssets`] and [`Balances`].
pub type NativeAndNonPoolAssets = fungible::UnionOf<
	Balances,
	LocalAndForeignAssets,
	TargetFromLeft<TokenLocation, xcm::v5::Location>,
	xcm::v5::Location,
	AccountId,
>;

/// Union fungibles implementation for [`LocalAndForeignAssetsFreezer`] and [`Balances`].
pub type NativeAndNonPoolAssetsFreezer = fungible::UnionOf<
	Balances,
	LocalAndForeignAssetsFreezer,
	TargetFromLeft<TokenLocation, xcm::v5::Location>,
	xcm::v5::Location,
	AccountId,
>;

/// Union fungibles implementation for [`PoolAssets`] and [`NativeAndNonPoolAssets`].
///
/// NOTE: Should be kept updated to include ALL balances and assets in the runtime.
pub type NativeAndAllAssets = fungibles::UnionOf<
	PoolAssets,
	NativeAndNonPoolAssets,
	LocalFromLeft<
		AssetIdForPoolAssetsConvert<PoolAssetsPalletLocation, xcm::v5::Location>,
		AssetIdForPoolAssets,
		xcm::v5::Location,
	>,
	xcm::v5::Location,
	AccountId,
>;

/// Union fungibles implementation for [`PoolAssetsFreezer`] and [`NativeAndNonPoolAssetsFreezer`].
///
/// NOTE: Should be kept updated to include ALL balances and assets in the runtime.
pub type NativeAndAllAssetsFreezer = fungibles::UnionOf<
	PoolAssetsFreezer,
	NativeAndNonPoolAssetsFreezer,
	LocalFromLeft<
		AssetIdForPoolAssetsConvert<PoolAssetsPalletLocation, xcm::v5::Location>,
		AssetIdForPoolAssets,
		xcm::v5::Location,
	>,
	xcm::v5::Location,
	AccountId,
>;

pub type PoolIdToAccountId = pezpallet_asset_conversion::AccountIdConverter<
	AssetConversionPalletId,
	(xcm::v5::Location, xcm::v5::Location),
>;

impl pezpallet_asset_conversion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type HigherPrecisionBalance = pezsp_core::U256;
	type AssetKind = xcm::v5::Location;
	type Assets = NativeAndNonPoolAssets;
	type PoolId = (Self::AssetKind, Self::AssetKind);
	type PoolLocator = pezpallet_asset_conversion::WithFirstAsset<
		TokenLocation,
		AccountId,
		Self::AssetKind,
		PoolIdToAccountId,
	>;
	type PoolAssetId = u32;
	type PoolAssets = PoolAssets;
	type PoolSetupFee = ConstU128<0>; // Asset class deposit fees are sufficient to prevent spam
	type PoolSetupFeeAsset = TokenLocation;
	type PoolSetupFeeTarget = ResolveAssetTo<AssetConversionOrigin, Self::Assets>;
	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
	type LPFee = ConstU32<3>;
	type PalletId = AssetConversionPalletId;
	type MaxSwapPathLength = ConstU32<3>;
	type MintMinLiquidity = ConstU128<100>;
	type WeightInfo = weights::pezpallet_asset_conversion::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pez_assets_common::benchmarks::AssetPairFactory<
		TokenLocation,
		teyrchain_info::Pezpallet<Runtime>,
		xcm_config::TrustBackedAssetsPalletIndex,
		xcm::v5::Location,
	>;
}

impl pezpallet_asset_conversion_ops::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PriorAccountIdConverter = pezpallet_asset_conversion::AccountIdConverterNoSeed<
		<Runtime as pezpallet_asset_conversion::Config>::PoolId,
	>;
	type AssetsRefund = <Runtime as pezpallet_asset_conversion::Config>::Assets;
	type PoolAssetsRefund = <Runtime as pezpallet_asset_conversion::Config>::PoolAssets;
	type PoolAssetsTeam = <Runtime as pezpallet_asset_conversion::Config>::PoolAssets;
	type DepositAsset = Balances;
	type WeightInfo = weights::pezpallet_asset_conversion_ops::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ForeignAssetsAssetDeposit: Balance = CreateForeignAssetDeposit::get();
	pub const ForeignAssetsAssetAccountDeposit: Balance = AssetAccountDeposit::get();
	pub const ForeignAssetsApprovalDeposit: Balance = ApprovalDeposit::get();
	pub const ForeignAssetsAssetsStringLimit: u32 = AssetsStringLimit::get();
	pub const ForeignAssetsMetadataDepositBase: Balance = MetadataDepositBase::get();
	pub const ForeignAssetsMetadataDepositPerByte: Balance = MetadataDepositPerByte::get();
}

/// Assets managed by some foreign location. Note: we do not declare a `ForeignAssetsCall` type, as
/// this type is used in proxy definitions. We assume that a foreign location would not want to set
/// an individual, local account as a proxy for the issuance of their assets. This issuance should
/// be managed by the foreign location's governance.
pub type ForeignAssetsInstance = pezpallet_assets::Instance2;
impl pezpallet_assets::Config<ForeignAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = xcm::v5::Location;
	type AssetIdParameter = xcm::v5::Location;
	type ReserveData = ForeignAssetReserveData;
	type Currency = Balances;
	type CreateOrigin = ForeignCreators<
		(
			FromSiblingTeyrchain<teyrchain_info::Pezpallet<Runtime>, xcm::v5::Location>,
			FromNetwork<xcm_config::UniversalLocation, EthereumNetwork, xcm::v5::Location>,
			xcm_config::bridging::to_zagros::ZagrosOrEthereumAssetFromAssetHubZagros,
		),
		LocationToAccountId,
		AccountId,
		xcm::v5::Location,
	>;
	type ForceOrigin = AssetsForceOrigin;
	type AssetDeposit = ForeignAssetsAssetDeposit;
	type MetadataDepositBase = ForeignAssetsMetadataDepositBase;
	type MetadataDepositPerByte = ForeignAssetsMetadataDepositPerByte;
	type ApprovalDeposit = ForeignAssetsApprovalDeposit;
	type StringLimit = ForeignAssetsAssetsStringLimit;
	type Holder = ();
	type Freezer = ForeignAssetsFreezer;
	type Extra = ();
	type WeightInfo = weights::pezpallet_assets_foreign::WeightInfo<Runtime>;
	type CallbackHandle = ();
	type AssetAccountDeposit = ForeignAssetsAssetAccountDeposit;
	type RemoveItemsLimit = pezframe_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pez_assets_common::benchmarks::LocationAssetsBenchmarkHelper;
}

// Allow Freezes for the `ForeignAssets` pezpallet
pub type ForeignAssetsFreezerInstance = pezpallet_assets_freezer::Instance2;
impl pezpallet_assets_freezer::Config<ForeignAssetsFreezerInstance> for Runtime {
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u32 = 100;
}

impl pezpallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = weights::pezpallet_multisig::WeightInfo<Runtime>;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

impl pezpallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pezpallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	// One storage item; key size 32, value size 16
	pub const AnnouncementDepositBase: Balance = deposit(1, 48);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	/// Fully permissioned proxy. Can execute any call on behalf of _proxied_.
	Any,
	/// Can execute any call that does not transfer funds or assets.
	NonTransfer,
	/// Proxy with the ability to reject time-delay proxy announcements.
	CancelProxy,
	/// Assets proxy. Can execute any call from `assets`, **including asset transfers**.
	Assets,
	/// Owner proxy. Can execute calls related to asset ownership.
	AssetOwner,
	/// Asset manager. Can execute calls related to asset management.
	AssetManager,
	/// Collator selection proxy. Can execute calls related to collator selection mechanism.
	Collator,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => !matches!(
				c,
				RuntimeCall::Balances { .. }
					| RuntimeCall::Assets { .. }
					| RuntimeCall::NftFractionalization { .. }
					| RuntimeCall::Nfts { .. }
					| RuntimeCall::Uniques { .. }
			),
			ProxyType::CancelProxy => matches!(
				c,
				RuntimeCall::Proxy(pezpallet_proxy::Call::reject_announcement { .. })
					| RuntimeCall::Utility { .. }
					| RuntimeCall::Multisig { .. }
			),
			ProxyType::Assets => {
				matches!(
					c,
					RuntimeCall::Assets { .. }
						| RuntimeCall::Utility { .. }
						| RuntimeCall::Multisig { .. }
						| RuntimeCall::NftFractionalization { .. }
						| RuntimeCall::Nfts { .. }
						| RuntimeCall::Uniques { .. }
				)
			},
			ProxyType::AssetOwner => matches!(
				c,
				RuntimeCall::Assets(TrustBackedAssetsCall::create { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::start_destroy { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::destroy_accounts { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::destroy_approvals { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::finish_destroy { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::transfer_ownership { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::set_team { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::set_metadata { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::clear_metadata { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::set_min_balance { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::create { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::destroy { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::redeposit { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::transfer_ownership { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::set_team { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::set_collection_max_supply { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::lock_collection { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::create { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::destroy { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::transfer_ownership { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::set_team { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::set_metadata { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::set_attribute { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::set_collection_metadata { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::clear_metadata { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::clear_attribute { .. })
					| RuntimeCall::Uniques(
						pezpallet_uniques::Call::clear_collection_metadata { .. }
					) | RuntimeCall::Uniques(pezpallet_uniques::Call::set_collection_max_supply { .. })
					| RuntimeCall::Utility { .. }
					| RuntimeCall::Multisig { .. }
			),
			ProxyType::AssetManager => matches!(
				c,
				RuntimeCall::Assets(TrustBackedAssetsCall::mint { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::burn { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::freeze { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::block { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::thaw { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::freeze_asset { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::thaw_asset { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::touch_other { .. })
					| RuntimeCall::Assets(TrustBackedAssetsCall::refund_other { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::force_mint { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::update_mint_settings { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::mint_pre_signed { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::set_attributes_pre_signed { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::lock_item_transfer { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::unlock_item_transfer { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::lock_item_properties { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::set_metadata { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::clear_metadata { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::set_collection_metadata { .. })
					| RuntimeCall::Nfts(pezpallet_nfts::Call::clear_collection_metadata { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::mint { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::burn { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::freeze { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::thaw { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::freeze_collection { .. })
					| RuntimeCall::Uniques(pezpallet_uniques::Call::thaw_collection { .. })
					| RuntimeCall::Utility { .. }
					| RuntimeCall::Multisig { .. }
			),
			ProxyType::Collator => matches!(
				c,
				RuntimeCall::CollatorSelection { .. }
					| RuntimeCall::Utility { .. }
					| RuntimeCall::Multisig { .. }
			),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::Assets, ProxyType::AssetOwner) => true,
			(ProxyType::Assets, ProxyType::AssetManager) => true,
			(ProxyType::NonTransfer, ProxyType::Collator) => true,
			_ => false,
		}
	}
}

impl pezpallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = weights::pezpallet_proxy::WeightInfo<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl pezcumulus_pezpallet_teyrchain_system::Config for Runtime {
	type WeightInfo = weights::pezcumulus_pezpallet_teyrchain_system::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = teyrchain_info::Pezpallet<Runtime>;
	type DmpQueue = pezframe_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type OutboundXcmpMessageSource = XcmpQueue;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
	type RelayParentOffset = ConstU32<0>;
}

type ConsensusHook = pezcumulus_pezpallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,
	RELAY_CHAIN_SLOT_DURATION_MILLIS,
	BLOCK_PROCESSING_VELOCITY,
	UNINCLUDED_SEGMENT_CAPACITY,
>;

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pezpallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezpallet_message_queue::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pezpallet_message_queue::mock_helpers::NoopMessageProcessor<
		pezcumulus_primitives_core::AggregateMessageOrigin,
	>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	// The XCMP queue pezpallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = pezsp_core::ConstU32<{ 103 * 1024 }>;
	type MaxStale = pezsp_core::ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = MessageQueueServiceWeight;
}

impl teyrchain_info::Config for Runtime {}

impl pezcumulus_pezpallet_aura_ext::Config for Runtime {}

parameter_types! {
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(xcm_config::TokenLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

pub type PriceForSiblingTeyrchainDelivery = pezkuwi_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	TransactionByteFee,
	XcmpQueue,
>;

impl pezcumulus_pezpallet_xcmp_queue::Config for Runtime {
	type WeightInfo = weights::pezcumulus_pezpallet_xcmp_queue::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = TeyrchainSystem;
	type VersionWrapper = PezkuwiXcm;
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = ConstU32<1_000>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	// Most on-chain HRMP channels are configured to use 102400 bytes of max message size, so we
	// need to set the page size larger than that until we reduce the channel size on-chain.
	type MaxPageSize = ConstU32<{ 103 * 1024 }>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = xcm_config::XcmOriginToTransactDispatchOrigin;
	type PriceForSiblingDelivery = PriceForSiblingTeyrchainDelivery;
}

impl pezcumulus_pezpallet_xcmp_queue::migration::v5::V5Config for Runtime {
	// This must be the same as the `ChannelInfo` from the `Config`:
	type ChannelList = TeyrchainSystem;
}

parameter_types! {
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

parameter_types! {
	pub const Period: u32 = prod_or_fast!(6 * HOURS, 20);
	pub const Offset: u32 = 0;
}

impl pezpallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as pezframe_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pezpallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pezpallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pezpallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = StakingSessionManager;
	// Essentially just Aura, but let's be pedantic.
	type SessionHandler = <SessionKeys as pezsp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisablingStrategy = ();
	type WeightInfo = weights::pezpallet_session::WeightInfo<Runtime>;
	type Currency = Balances;
	type KeyDeposit = ();
}

impl pezpallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<true>;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const SessionLength: BlockNumber = prod_or_fast!(6 * HOURS, 20);
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root and the `StakingAdmin` to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsVoiceOfBody<GovernanceLocation, StakingAdminBodyId>>,
>;

impl pezpallet_collator_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type UpdateOrigin = CollatorSelectionUpdateOrigin;
	type PotId = PotId;
	type MaxCandidates = ConstU32<100>;
	type MinEligibleCollators = ConstU32<4>;
	type MaxInvulnerables = ConstU32<20>;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type ValidatorId = <Self as pezframe_system::Config>::AccountId;
	type ValidatorIdOf = pezpallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = weights::pezpallet_collator_selection::WeightInfo<Runtime>;
}

parameter_types! {
	pub StakingPot: AccountId = CollatorSelection::account_id();
}

impl pezpallet_asset_conversion_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = xcm::v5::Location;
	type OnChargeAssetTransaction = SwapAssetAdapter<
		TokenLocation,
		NativeAndNonPoolAssets,
		AssetConversion,
		ResolveAssetTo<StakingPot, NativeAndNonPoolAssets>,
	>;
	type WeightInfo = weights::pezpallet_asset_conversion_tx_payment::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetConversionTxHelper;
}

parameter_types! {
	pub const UniquesCollectionDeposit: Balance = UNITS / 10; // 1 / 10 UNIT deposit to create a collection
	pub const UniquesItemDeposit: Balance = UNITS / 1_000; // 1 / 1000 UNIT deposit to mint an item
	pub const UniquesMetadataDepositBase: Balance = deposit(1, 129);
	pub const UniquesAttributeDepositBase: Balance = deposit(1, 0);
	pub const UniquesDepositPerByte: Balance = deposit(0, 1);
}

impl pezpallet_uniques::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = CollectionId;
	type ItemId = ItemId;
	type Currency = Balances;
	type ForceOrigin = AssetsForceOrigin;
	type CollectionDeposit = UniquesCollectionDeposit;
	type ItemDeposit = UniquesItemDeposit;
	type MetadataDepositBase = UniquesMetadataDepositBase;
	type AttributeDepositBase = UniquesAttributeDepositBase;
	type DepositPerByte = UniquesDepositPerByte;
	type StringLimit = ConstU32<128>;
	type KeyLimit = ConstU32<32>;
	type ValueLimit = ConstU32<64>;
	type WeightInfo = weights::pezpallet_uniques::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
}

parameter_types! {
	pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
	pub NewAssetSymbol: BoundedVec<u8, AssetsStringLimit> = (*b"FRAC").to_vec().try_into().unwrap();
	pub NewAssetName: BoundedVec<u8, AssetsStringLimit> = (*b"Frac").to_vec().try_into().unwrap();
}

impl pezpallet_nft_fractionalization::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Deposit = AssetDeposit;
	type Currency = Balances;
	type NewAssetSymbol = NewAssetSymbol;
	type NewAssetName = NewAssetName;
	type StringLimit = AssetsStringLimit;
	type NftCollectionId = <Self as pezpallet_nfts::Config>::CollectionId;
	type NftId = <Self as pezpallet_nfts::Config>::ItemId;
	type AssetBalance = <Self as pezpallet_balances::Config>::Balance;
	type AssetId = <Self as pezpallet_assets::Config<TrustBackedAssetsInstance>>::AssetId;
	type Assets = Assets;
	type Nfts = Nfts;
	type PalletId = NftFractionalizationPalletId;
	type WeightInfo = weights::pezpallet_nft_fractionalization::WeightInfo<Runtime>;
	type RuntimeHoldReason = RuntimeHoldReason;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub NftsPalletFeatures: PalletFeatures = PalletFeatures::all_enabled();
	pub const NftsMaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;
	// re-use the Uniques deposits
	pub const NftsCollectionDeposit: Balance = UniquesCollectionDeposit::get();
	pub const NftsItemDeposit: Balance = UniquesItemDeposit::get();
	pub const NftsMetadataDepositBase: Balance = UniquesMetadataDepositBase::get();
	pub const NftsAttributeDepositBase: Balance = UniquesAttributeDepositBase::get();
	pub const NftsDepositPerByte: Balance = UniquesDepositPerByte::get();
}

impl pezpallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = CollectionId;
	type ItemId = ItemId;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = AssetsForceOrigin;
	type Locker = ();
	type CollectionDeposit = NftsCollectionDeposit;
	type ItemDeposit = NftsItemDeposit;
	type MetadataDepositBase = NftsMetadataDepositBase;
	type AttributeDepositBase = NftsAttributeDepositBase;
	type DepositPerByte = NftsDepositPerByte;
	type StringLimit = ConstU32<256>;
	type KeyLimit = ConstU32<64>;
	type ValueLimit = ConstU32<256>;
	type ApprovalsLimit = ConstU32<20>;
	type ItemAttributesApprovalsLimit = ConstU32<30>;
	type MaxTips = ConstU32<10>;
	type MaxDeadlineDuration = NftsMaxDeadlineDuration;
	type MaxAttributesPerCall = ConstU32<10>;
	type Features = NftsPalletFeatures;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as Verify>::Signer;
	type WeightInfo = weights::pezpallet_nfts::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

/// XCM router instance to BridgeHub with bridging capabilities for `Zagros` global
/// consensus with dynamic fees and back-pressure.
pub type ToZagrosXcmRouterInstance = pezpallet_xcm_bridge_hub_router::Instance3;
impl pezpallet_xcm_bridge_hub_router::Config<ToZagrosXcmRouterInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezpallet_xcm_bridge_hub_router::WeightInfo<Runtime>;

	type UniversalLocation = xcm_config::UniversalLocation;
	type SiblingBridgeHubLocation = xcm_config::bridging::SiblingBridgeHub;
	type BridgedNetworkId = xcm_config::bridging::to_zagros::ZagrosNetwork;
	type Bridges = xcm_config::bridging::NetworkExportTable;
	type DestinationVersion = PezkuwiXcm;

	type BridgeHubOrigin = pezframe_support::traits::EitherOfDiverse<
		EnsureRoot<AccountId>,
		EnsureXcm<Equals<Self::SiblingBridgeHubLocation>>,
	>;
	type ToBridgeHubSender = XcmpQueue;
	type LocalXcmChannelManager =
		pezcumulus_pezpallet_xcmp_queue::bridging::InAndOutXcmpChannelStatusProvider<Runtime>;

	type ByteFee = xcm_config::bridging::XcmBridgeHubRouterByteFee;
	type FeeAsset = xcm_config::bridging::XcmBridgeHubRouterFeeAssetId;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct PalletAssetRewardsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_asset_rewards::benchmarking::BenchmarkHelper<xcm::v5::Location>
	for PalletAssetRewardsBenchmarkHelper
{
	fn staked_asset() -> Location {
		Location::new(
			0,
			[PalletInstance(<Assets as PalletInfoAccess>::index() as u8), GeneralIndex(100)],
		)
	}
	fn reward_asset() -> Location {
		Location::new(
			0,
			[PalletInstance(<Assets as PalletInfoAccess>::index() as u8), GeneralIndex(101)],
		)
	}
}

parameter_types! {
	pub const AssetRewardsPalletId: PalletId = PalletId(*b"py/astrd");
	pub const RewardsPoolCreationHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::AssetRewards(pezpallet_asset_rewards::HoldReason::PoolCreation);
	// 1 item, 135 bytes into the storage on pool creation.
	pub const StakePoolCreationDeposit: Balance = deposit(1, 135);
}

impl pezpallet_asset_rewards::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = AssetRewardsPalletId;
	type Balance = Balance;
	type Assets = NativeAndAllAssets;
	type AssetsFreezer = NativeAndAllAssetsFreezer;
	type AssetId = xcm::v5::Location;
	type CreatePoolOrigin = EnsureSigned<AccountId>;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		RewardsPoolCreationHoldReason,
		ConstantStoragePrice<StakePoolCreationDeposit, Balance>,
	>;
	type WeightInfo = weights::pezpallet_asset_rewards::WeightInfo<Runtime>;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = PalletAssetRewardsBenchmarkHelper;
}

// -----------------------------------------------------------------------------
// NIS Pezpallet
// -----------------------------------------------------------------------------
parameter_types! {
	pub const QueueCount: u32 = 10;
	pub const MaxQueueLen: u32 = 100;
	pub const FifoQueueLen: u32 = 50;
	pub const NisBasePeriod: BlockNumber = 30 * DAYS;
	pub const MinBid: Balance = 10 * UNITS;
	pub const MinReceipt: Perquintill = Perquintill::from_percent(1);
	pub const IntakePeriod: BlockNumber = 10;
	pub MaxIntakeWeight: Weight = RuntimeBlockWeights::get().max_block / 10;
	pub const ThawThrottle: (Perquintill, BlockNumber) = (Perquintill::from_percent(25), 5);
	pub Target: Perquintill = Perquintill::zero();
	pub const NisPalletId: PalletId = PalletId(*b"py/nis  ");
}

use pezframe_support::traits::fungible::ItemOf;
use pezpallet_nis::WithMaximumOf;

impl pezpallet_nis::Config for Runtime {
	type WeightInfo = pezpallet_nis::weights::BizinikiwiWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type FundOrigin = pezframe_system::EnsureSigned<AccountId>;
	type Counterpart = ItemOf<Assets, PezAssetId, AccountId>;
	type CounterpartAmount = WithMaximumOf<ConstU128<{ u128::MAX }>>;
	type Deficit = ();
	type IgnoredIssuance = ();
	type Target = Target;
	type PalletId = NisPalletId;
	type QueueCount = QueueCount;
	type MaxQueueLen = MaxQueueLen;
	type FifoQueueLen = FifoQueueLen;
	type BasePeriod = NisBasePeriod;
	type MinBid = MinBid;
	type MinReceipt = MinReceipt;
	type IntakePeriod = IntakePeriod;
	type MaxIntakeWeight = MaxIntakeWeight;
	type ThawThrottle = ThawThrottle;
	type RuntimeHoldReason = RuntimeHoldReason;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = ();
}

// -----------------------------------------------------------------------------
// Treasury Pezpallet
// -----------------------------------------------------------------------------
parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(2);
	pub const MaxApprovals: u32 = 100;
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

impl pezpallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type MaxApprovals = MaxApprovals;
	type WeightInfo = ();
	type SpendFunds = ();
	type SpendOrigin =
		pezframe_system::EnsureRootWithSuccess<AccountId, ConstU128<{ Balance::max_value() }>>;
	type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = pezsp_runtime::traits::IdentityLookup<Self::Beneficiary>;
	type Paymaster =
		pezframe_support::traits::tokens::pay::PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = pezframe_support::traits::tokens::UnityAssetBalanceConversion;
	type PayoutPeriod = PayoutSpendPeriod;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

// -----------------------------------------------------------------------------
// AssetRate Pezpallet
// -----------------------------------------------------------------------------
impl pezpallet_asset_rate::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type CreateOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type AssetKind = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

// -----------------------------------------------------------------------------
// Bounties Pezpallet
// -----------------------------------------------------------------------------
parameter_types! {
	pub const BountiesDepositBase: Balance = 100 * CENTS;
	pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 5 * UNITS;
	pub const DataDepositPerByte: Balance = 1 * CENTS;
}

impl pezpallet_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = BountiesDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = BountyCuratorDeposit;
	type CuratorDepositMin = ConstU128<{ 1 * UNITS }>;
	type CuratorDepositMax = ConstU128<{ 100 * UNITS }>;
	type BountyValueMinimum = BountyValueMinimum;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = ();
	type ChildBountyManager = ChildBounties;
	type OnSlash = Treasury;
}

// -----------------------------------------------------------------------------
// ChildBounties Pezpallet
// -----------------------------------------------------------------------------
parameter_types! {
	pub const MaxActiveChildBountyCount: u32 = 100;
	pub const ChildBountyValueMinimum: Balance = 1 * UNITS;
}

impl pezpallet_child_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = ();
}

// =============================================================================
// PezkuwiChain Custom Asset Hub Pallets Configuration
// =============================================================================

// -----------------------------------------------------------------------------
// PEZ Treasury Pezpallet
// -----------------------------------------------------------------------------

parameter_types! {
	pub const PezTreasuryPalletId: PalletId = PalletId(*b"pez/trea");
	pub const PezIncentivePotId: PalletId = PalletId(*b"pez/incv");
	pub const PezGovernmentPotId: PalletId = PalletId(*b"pez/govr");
	pub const PezAssetId: u32 = 1; // PEZ token asset ID
	pub PezPresaleAccount: AccountId = PalletId(*b"pez/pres").into_account_truncating();
	pub PezFounderAccount: AccountId = PalletId(*b"pez/foun").into_account_truncating();
}

impl pezpallet_pez_treasury::Config for Runtime {
	type Assets = Assets;
	type WeightInfo = pezpallet_pez_treasury::weights::BizinikiwiWeight<Runtime>;
	type PezAssetId = PezAssetId;
	type TreasuryPalletId = PezTreasuryPalletId;
	type IncentivePotId = PezIncentivePotId;
	type GovernmentPotId = PezGovernmentPotId;
	type PresaleAccount = PezPresaleAccount;
	type FounderAccount = PezFounderAccount;
	type ForceOrigin = EnsureRoot<AccountId>;
}

// -----------------------------------------------------------------------------
// Presale Pezpallet
// -----------------------------------------------------------------------------

parameter_types! {
	pub const PresalePalletId: PalletId = PalletId(*b"pez/sale");
	pub PresalePlatformTreasury: AccountId = PalletId(*b"pez/plat").into_account_truncating();
	pub PresaleStakingRewardPool: AccountId = PalletId(*b"pez/stak").into_account_truncating();
	pub const PresalePlatformFeePercent: u8 = 2; // 2% platform fee
	pub const PresaleMaxContributors: u32 = 10_000;
	pub const PresaleMaxBonusTiers: u32 = 5;
	pub const PresaleMaxWhitelistedAccounts: u32 = 1_000;
}

impl pezpallet_presale::Config for Runtime {
	type AssetId = AssetIdForTrustBackedAssets;
	type Balance = Balance;
	type Assets = Assets;
	type PalletId = PresalePalletId;
	type PlatformTreasury = PresalePlatformTreasury;
	type StakingRewardPool = PresaleStakingRewardPool;
	type PlatformFeePercent = PresalePlatformFeePercent;
	type MaxContributors = PresaleMaxContributors;
	type MaxBonusTiers = PresaleMaxBonusTiers;
	type MaxWhitelistedAccounts = PresaleMaxWhitelistedAccounts;
	type CreatePresaleOrigin = EnsureSigned<AccountId>;
	type EmergencyOrigin = EnsureRoot<AccountId>;
	type PresaleWeightInfo = pezpallet_presale::BizinikiwiWeight<Runtime>;
}

// -----------------------------------------------------------------------------
// Token Wrapper Pezpallet
// -----------------------------------------------------------------------------

parameter_types! {
	pub const TokenWrapperPalletId: PalletId = PalletId(*b"pez/wrap");
	pub const WrappedAssetId: AssetIdForTrustBackedAssets = 2; // wHEZ asset ID
}

impl pezpallet_token_wrapper::Config for Runtime {
	type WeightInfo = pezpallet_token_wrapper::weights::BizinikiwiWeight<Runtime>;
	type Currency = Balances;
	type AssetId = AssetIdForTrustBackedAssets;
	type Assets = Assets;
	type PalletId = TokenWrapperPalletId;
	type WrapperAssetId = WrappedAssetId;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime
	{
		// System support stuff.
		System: pezframe_system = 0,
		TeyrchainSystem: pezcumulus_pezpallet_teyrchain_system = 1,
		Timestamp: pezpallet_timestamp = 3,
		TeyrchainInfo: teyrchain_info = 4,
		WeightReclaim: pezcumulus_pezpallet_weight_reclaim = 5,

		// Monetary stuff.
		Balances: pezpallet_balances = 10,
		TransactionPayment: pezpallet_transaction_payment = 11,
		AssetTxPayment: pezpallet_asset_conversion_tx_payment = 13,

		// Collator support. the order of these 5 are important and shall not change.
		Authorship: pezpallet_authorship = 20,
		CollatorSelection: pezpallet_collator_selection = 21,
		Session: pezpallet_session = 22,
		Aura: pezpallet_aura = 23,
		AuraExt: pezcumulus_pezpallet_aura_ext = 24,

		// XCM helpers.
		XcmpQueue: pezcumulus_pezpallet_xcmp_queue = 30,
		PezkuwiXcm: pezpallet_xcm = 31,
		CumulusXcm: pezcumulus_pezpallet_xcm = 32,
		MessageQueue: pezpallet_message_queue = 34,

		// Handy utilities.
		Utility: pezpallet_utility = 40,
		Multisig: pezpallet_multisig = 41,
		Proxy: pezpallet_proxy = 42,

		// Bridge utilities.
		ToZagrosXcmRouter: pezpallet_xcm_bridge_hub_router::<Instance3> = 45,

		// The main stage.
		Assets: pezpallet_assets::<Instance1> = 50,
		Uniques: pezpallet_uniques = 51,
		Nfts: pezpallet_nfts = 52,
		ForeignAssets: pezpallet_assets::<Instance2> = 53,
		NftFractionalization: pezpallet_nft_fractionalization = 54,
		PoolAssets: pezpallet_assets::<Instance3> = 55,
		AssetConversion: pezpallet_asset_conversion = 56,

		AssetsFreezer: pezpallet_assets_freezer::<Instance1> = 57,
		ForeignAssetsFreezer: pezpallet_assets_freezer::<Instance2> = 58,
		PoolAssetsFreezer: pezpallet_assets_freezer::<Instance3> = 59,

		AssetRewards: pezpallet_asset_rewards = 60,

		Nis: pezpallet_nis = 61,
		AssetRate: pezpallet_asset_rate = 62,
		Bounties: pezpallet_bounties = 63,
		ChildBounties: pezpallet_child_bounties = 64,
		Treasury: pezpallet_treasury = 65,

		// PezkuwiChain Custom Pallets
		PezTreasury: pezpallet_pez_treasury = 70,
		Presale: pezpallet_presale = 71,
		TokenWrapper: pezpallet_token_wrapper = 73,

		// Staking
		Staking: pezpallet_staking_async = 80,
		NominationPools: pezpallet_nomination_pools = 81,
		VoterList: pezpallet_bags_list::<Instance1> = 83,
		DelegatedStaking: pezpallet_delegated_staking = 84,
		StakingRcClient: pezpallet_staking_async_rc_client = 89,

		// Staking election apparatus.
		MultiBlockElection: pezpallet_election_provider_multi_block = 85,
		MultiBlockElectionVerifier: pezpallet_election_provider_multi_block::verifier = 86,
		MultiBlockElectionUnsigned: pezpallet_election_provider_multi_block::unsigned = 87,
		MultiBlockElectionSigned: pezpallet_election_provider_multi_block::signed = 88,

		// TODO: the pezpallet instance should be removed once all pools have migrated
		// to the new account IDs.
		AssetConversionMigration: pezpallet_asset_conversion_ops = 200,
	}
);

/// The address format for describing accounts.
pub type Address = pezsp_runtime::MultiAddress<AccountId, ()>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The extension to the basic transaction logic.
pub type TxExtension = pezcumulus_pezpallet_weight_reclaim::StorageWeightReclaim<
	Runtime,
	(
		pezframe_system::AuthorizeCall<Runtime>,
		pezframe_system::CheckNonZeroSender<Runtime>,
		pezframe_system::CheckSpecVersion<Runtime>,
		pezframe_system::CheckTxVersion<Runtime>,
		pezframe_system::CheckGenesis<Runtime>,
		pezframe_system::CheckEra<Runtime>,
		pezframe_system::CheckNonce<Runtime>,
		pezframe_system::CheckWeight<Runtime>,
		pezpallet_asset_conversion_tx_payment::ChargeAssetTxPayment<Runtime>,
		pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
	),
>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
/// One-time migration to fix ActiveEra.start which was set to 0 at genesis.
/// Without this, the first era's duration would be calculated as (now - 0) = ~56 years,
/// though MaxEraDuration caps it to 6 hours. This migration sets it to the current timestamp
/// so the first era duration is calculated correctly from the upgrade moment.
pub struct FixActiveEraStart;
impl pezframe_support::traits::OnRuntimeUpgrade for FixActiveEraStart {
	fn on_runtime_upgrade() -> Weight {
		let now_ms = pezpallet_timestamp::Now::<Runtime>::get();
		if now_ms > 0 {
			pezpallet_staking_async::ActiveEra::<Runtime>::mutate(|era| {
				if let Some(ref mut info) = era {
					info.start = Some(now_ms);
					log::info!(
						target: "runtime::staking",
						"FixActiveEraStart: Set ActiveEra.start to {}",
						now_ms,
					);
				}
			});
		}
		<Runtime as pezframe_system::Config>::DbWeight::get().reads_writes(2, 1)
	}
}

/// Migrations to apply on runtime upgrade.
pub type Migrations = (
	FixActiveEraStart,
	InitStorageVersions,
	// unreleased
	pezcumulus_pezpallet_xcmp_queue::migration::v4::MigrationToV4<Runtime>,
	pezcumulus_pezpallet_xcmp_queue::migration::v5::MigrateV4ToV5<Runtime>,
	pezpallet_collator_selection::migration::v2::MigrationToV2<Runtime>,
	pezframe_support::migrations::RemovePallet<StateTrieMigrationName, RocksDbWeight>,
	// unreleased
	pezpallet_assets::migration::next_asset_id::SetNextAssetId<
		ConstU32<50_000_000>,
		Runtime,
		TrustBackedAssetsInstance,
	>,
	pezpallet_session::migrations::v1::MigrateV0ToV1<
		Runtime,
		pezpallet_session::migrations::v1::InitOffenceSeverity<Runtime>,
	>,
	// permanent
	pezpallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
	pezcumulus_pezpallet_aura_ext::migration::MigrateV0ToV1<Runtime>,
);

parameter_types! {
	pub const StateTrieMigrationName: &'static str = "StateTrieMigration";
}

/// Migration to initialize storage versions for pallets added after genesis.
///
/// This is now done automatically (see <https://github.com/pezkuwichain/pezkuwi-sdk/issues/248>),
/// but some pallets had made it in and had storage set in them for this teyrchain before it was
/// merged.
pub struct InitStorageVersions;

impl pezframe_support::traits::OnRuntimeUpgrade for InitStorageVersions {
	fn on_runtime_upgrade() -> Weight {
		use pezframe_support::traits::{GetStorageVersion, StorageVersion};

		let mut writes = 0;

		if PezkuwiXcm::on_chain_storage_version() == StorageVersion::new(0) {
			PezkuwiXcm::in_code_storage_version().put::<PezkuwiXcm>();
			writes.saturating_inc();
		}

		if Multisig::on_chain_storage_version() == StorageVersion::new(0) {
			Multisig::in_code_storage_version().put::<Multisig>();
			writes.saturating_inc();
		}

		if Assets::on_chain_storage_version() == StorageVersion::new(0) {
			Assets::in_code_storage_version().put::<Assets>();
			writes.saturating_inc();
		}

		if Uniques::on_chain_storage_version() == StorageVersion::new(0) {
			Uniques::in_code_storage_version().put::<Uniques>();
			writes.saturating_inc();
		}

		if Nfts::on_chain_storage_version() == StorageVersion::new(0) {
			Nfts::in_code_storage_version().put::<Nfts>();
			writes.saturating_inc();
		}

		if ForeignAssets::on_chain_storage_version() == StorageVersion::new(0) {
			ForeignAssets::in_code_storage_version().put::<ForeignAssets>();
			writes.saturating_inc();
		}

		if PoolAssets::on_chain_storage_version() == StorageVersion::new(0) {
			PoolAssets::in_code_storage_version().put::<PoolAssets>();
			writes.saturating_inc();
		}

		<Runtime as pezframe_system::Config>::DbWeight::get().reads_writes(7, writes)
	}
}

/// Executive: handles dispatch to the various modules.
pub type Executive = pezframe_executive::Executive<
	Runtime,
	Block,
	pezframe_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetConversionTxHelper;

#[cfg(feature = "runtime-benchmarks")]
impl
	pezpallet_asset_conversion_tx_payment::BenchmarkHelperTrait<
		AccountId,
		pezcumulus_primitives_core::Location,
		pezcumulus_primitives_core::Location,
	> for AssetConversionTxHelper
{
	fn create_asset_id_parameter(seed: u32) -> (Location, Location) {
		// Use a different teyrchain' foreign assets pezpallet so that the asset is indeed foreign.
		let asset_id = Location::new(
			1,
			[
				pezcumulus_primitives_core::Junction::Teyrchain(3000),
				pezcumulus_primitives_core::Junction::PalletInstance(53),
				pezcumulus_primitives_core::Junction::GeneralIndex(seed.into()),
			],
		);
		(asset_id.clone(), asset_id)
	}

	fn setup_balances_and_pool(asset_id: pezcumulus_primitives_core::Location, account: AccountId) {
		use pezframe_support::{assert_ok, traits::fungibles::Mutate};
		assert_ok!(ForeignAssets::force_create(
			RuntimeOrigin::root(),
			asset_id.clone().into(),
			account.clone().into(), /* owner */
			true,                   /* is_sufficient */
			1,
		));

		let lp_provider = account.clone();
		use pezframe_support::traits::Currency;
		let _ = Balances::deposit_creating(&lp_provider, u64::MAX.into());
		assert_ok!(ForeignAssets::mint_into(
			asset_id.clone().into(),
			&lp_provider,
			u64::MAX.into()
		));

		let token_native = alloc::boxed::Box::new(TokenLocation::get());
		let token_second = alloc::boxed::Box::new(asset_id);

		assert_ok!(AssetConversion::create_pool(
			RuntimeOrigin::signed(lp_provider.clone()),
			token_native.clone(),
			token_second.clone()
		));

		assert_ok!(AssetConversion::add_liquidity(
			RuntimeOrigin::signed(lp_provider.clone()),
			token_native,
			token_second,
			(u32::MAX / 8).into(), // 1 desired
			u32::MAX.into(),       // 2 desired
			1,                     // 1 min
			1,                     // 2 min
			lp_provider,
		));
	}
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	pezframe_benchmarking::define_benchmarks!(
		[pezframe_system, SystemBench::<Runtime>]
		[pezframe_system_extensions, SystemExtensionsBench::<Runtime>]
		[pezpallet_assets, Local]
		[pezpallet_assets, Foreign]
		[pezpallet_assets, Pool]
		[pezpallet_asset_conversion, AssetConversion]
		[pezpallet_asset_rewards, AssetRewards]
		[pezpallet_asset_conversion_tx_payment, AssetTxPayment]
		[pezpallet_balances, Balances]
		[pezpallet_message_queue, MessageQueue]
		[pezpallet_multisig, Multisig]
		[pezpallet_nft_fractionalization, NftFractionalization]
		[pezpallet_nfts, Nfts]
		[pezpallet_proxy, Proxy]
		[pezpallet_session, SessionBench::<Runtime>]
		[pezpallet_uniques, Uniques]
		[pezpallet_utility, Utility]
		[pezpallet_timestamp, Timestamp]
		[pezpallet_transaction_payment, TransactionPayment]
		[pezpallet_collator_selection, CollatorSelection]
		[pezcumulus_pezpallet_teyrchain_system, TeyrchainSystem]
		[pezcumulus_pezpallet_xcmp_queue, XcmpQueue]
		[pezpallet_xcm_bridge_hub_router, ToZagros]
		[pezpallet_asset_conversion_ops, AssetConversionMigration]
		// XCM
		[pezpallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
		// NOTE: Make sure you point to the individual modules below.
		[pezpallet_xcm_benchmarks::fungible, XcmBalances]
		[pezpallet_xcm_benchmarks::generic, XcmGeneric]
		[pezcumulus_pezpallet_weight_reclaim, WeightReclaim]
		// PezkuwiChain Custom Pallets
		[pezpallet_pez_treasury, PezTreasury]
		[pezpallet_presale, Presale]
		[pezpallet_token_wrapper, TokenWrapper]
	);
}

impl_runtime_apis! {
	impl pezsp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> pezsp_consensus_aura::SlotDuration {
			pezsp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
		}

		fn authorities() -> Vec<AuraId> {
			pezpallet_aura::Authorities::<Runtime>::get().into_inner()
		}
	}

	impl pezcumulus_primitives_core::RelayParentOffsetApi<Block> for Runtime {
		fn relay_parent_offset() -> u32 {
			0
		}
	}

	impl pezcumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
		fn can_build_upon(
			included_hash: <Block as BlockT>::Hash,
			slot: pezcumulus_primitives_aura::Slot,
		) -> bool {
			ConsensusHook::can_build_upon(included_hash, slot)
		}
	}

	impl pezsp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: <Block as BlockT>::LazyBlock) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> pezsp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl pezsp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> alloc::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl pezsp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: pezsp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: <Block as BlockT>::LazyBlock,
			data: pezsp_inherents::InherentData,
		) -> pezsp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl pezsp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl pezsp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl pezsp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl pezframe_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pezpallet_asset_conversion::AssetConversionApi<
		Block,
		Balance,
		xcm::v5::Location,
	> for Runtime
	{
		fn quote_price_exact_tokens_for_tokens(asset1: xcm::v5::Location, asset2: xcm::v5::Location, amount: Balance, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_exact_tokens_for_tokens(asset1, asset2, amount, include_fee)
		}
		fn quote_price_tokens_for_exact_tokens(asset1: xcm::v5::Location, asset2: xcm::v5::Location, amount: Balance, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_tokens_for_exact_tokens(asset1, asset2, amount, include_fee)
		}
		fn get_reserves(asset1: xcm::v5::Location, asset2: xcm::v5::Location) -> Option<(Balance, Balance)> {
			AssetConversion::get_reserves(asset1, asset2).ok()
		}
	}

	impl pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pezpallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pezpallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pezpallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pezpallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pez_assets_common::runtime_api::FungiblesApi<
		Block,
		AccountId,
	> for Runtime
	{
		fn query_account_balances(account: AccountId) -> Result<xcm::VersionedAssets, pez_assets_common::runtime_api::FungiblesAccessError> {
			use pez_assets_common::fungible_conversion::{convert, convert_balance};
			Ok([
				// collect pezpallet_balance
				{
					let balance = Balances::free_balance(account.clone());
					if balance > 0 {
						vec![convert_balance::<TokenLocation, Balance>(balance)?]
					} else {
						vec![]
					}
				},
				// collect pezpallet_assets (TrustBackedAssets)
				convert::<_, _, _, _, TrustBackedAssetsConvertedConcreteId>(
					Assets::account_balances(account.clone())
						.iter()
						.filter(|(_, balance)| balance > &0)
				)?,
				// collect pezpallet_assets (ForeignAssets)
				convert::<_, _, _, _, ForeignAssetsConvertedConcreteId>(
					ForeignAssets::account_balances(account.clone())
						.iter()
						.filter(|(_, balance)| balance > &0)
				)?,
				// collect pezpallet_assets (PoolAssets)
				convert::<_, _, _, _, PoolAssetsConvertedConcreteId>(
					PoolAssets::account_balances(account)
						.iter()
						.filter(|(_, balance)| balance > &0)
				)?,
				// collect ... e.g. other tokens
			].concat().into())
		}
	}

	impl xcm_runtime_pezapis::fees::XcmPaymentApi<Block> for Runtime {
		fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {
			let native_token = xcm_config::TokenLocation::get();
			// We accept the native token to pay fees.
			let mut acceptable_assets = vec![AssetId(native_token.clone())];
			// We also accept all assets in a pool with the native token.
			acceptable_assets.extend(
				pez_assets_common::PoolAdapter::<Runtime>::get_assets_in_pool_with(native_token)
				.map_err(|()| XcmPaymentApiError::VersionedConversionFailed)?
			);
			PezkuwiXcm::query_acceptable_payment_assets(xcm_version, acceptable_assets)
		}

		fn query_weight_to_asset_fee(weight: Weight, asset: VersionedAssetId) -> Result<u128, XcmPaymentApiError> {
			type Trader = <XcmConfig as xcm_executor::Config>::Trader;
			PezkuwiXcm::query_weight_to_asset_fee::<Trader>(weight, asset)
		}

		fn query_xcm_weight(message: VersionedXcm<()>) -> Result<Weight, XcmPaymentApiError> {
			PezkuwiXcm::query_xcm_weight(message)
		}

		fn query_delivery_fees(destination: VersionedLocation, message: VersionedXcm<()>, asset_id: VersionedAssetId) -> Result<VersionedAssets, XcmPaymentApiError> {
			type AssetExchanger = <XcmConfig as xcm_executor::Config>::AssetExchanger;
			PezkuwiXcm::query_delivery_fees::<AssetExchanger>(destination, message, asset_id)
		}
	}

	impl xcm_runtime_pezapis::dry_run::DryRunApi<Block, RuntimeCall, RuntimeEvent, OriginCaller> for Runtime {
		fn dry_run_call(origin: OriginCaller, call: RuntimeCall, result_xcms_version: XcmVersion) -> Result<CallDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			PezkuwiXcm::dry_run_call::<Runtime, xcm_config::XcmRouter, OriginCaller, RuntimeCall>(origin, call, result_xcms_version)
		}

		fn dry_run_xcm(origin_location: VersionedLocation, xcm: VersionedXcm<RuntimeCall>) -> Result<XcmDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			PezkuwiXcm::dry_run_xcm::<xcm_config::XcmRouter>(origin_location, xcm)
		}
	}

	impl xcm_runtime_pezapis::conversions::LocationToAccountApi<Block, AccountId> for Runtime {
		fn convert_location(location: VersionedLocation) -> Result<
			AccountId,
			xcm_runtime_pezapis::conversions::Error
		> {
			xcm_runtime_pezapis::conversions::LocationToAccountHelper::<
				AccountId,
				xcm_config::LocationToAccountId,
			>::convert_location(location)
		}
	}

	impl pezcumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> pezcumulus_primitives_core::CollationInfo {
			TeyrchainSystem::collect_collation_info(header)
		}
	}

	impl pezpallet_asset_rewards::AssetRewards<Block, Balance> for Runtime {
		fn pool_creation_cost() -> Balance {
			StakePoolCreationDeposit::get()
		}
	}

	impl pezpallet_nomination_pools_runtime_api::NominationPoolsApi<
		Block,
		AccountId,
		Balance,
	> for Runtime {
		fn pending_rewards(member: AccountId) -> Balance {
			NominationPools::api_pending_rewards(member).unwrap_or_default()
		}

		fn points_to_balance(pool_id: pezpallet_nomination_pools::PoolId, points: Balance) -> Balance {
			NominationPools::api_points_to_balance(pool_id, points)
		}

		fn balance_to_points(pool_id: pezpallet_nomination_pools::PoolId, new_funds: Balance) -> Balance {
			NominationPools::api_balance_to_points(pool_id, new_funds)
		}

		fn pool_pending_slash(pool_id: pezpallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_pending_slash(pool_id)
		}

		fn member_pending_slash(member: AccountId) -> Balance {
			NominationPools::api_member_pending_slash(member)
		}

		fn pool_needs_delegate_migration(pool_id: pezpallet_nomination_pools::PoolId) -> bool {
			NominationPools::api_pool_needs_delegate_migration(pool_id)
		}

		fn member_needs_delegate_migration(member: AccountId) -> bool {
			NominationPools::api_member_needs_delegate_migration(member)
		}

		fn member_total_balance(member: AccountId) -> Balance {
			NominationPools::api_member_total_balance(member)
		}

		fn pool_balance(pool_id: pezpallet_nomination_pools::PoolId) -> Balance {
			NominationPools::api_pool_balance(pool_id)
		}

		fn pool_accounts(pool_id: pezpallet_nomination_pools::PoolId) -> (AccountId, AccountId) {
			NominationPools::api_pool_accounts(pool_id)
		}
	}

	impl pezpallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
		fn nominations_quota(balance: Balance) -> u32 {
			Staking::api_nominations_quota(balance)
		}

		fn eras_stakers_page_count(era: pezsp_staking::EraIndex, account: AccountId) -> pezsp_staking::Page {
			Staking::api_eras_stakers_page_count(era, account)
		}

		fn pending_rewards(era: pezsp_staking::EraIndex, account: AccountId) -> bool {
			Staking::api_pending_rewards(era, account)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl pezframe_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: pezframe_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: <Block as BlockT>::LazyBlock,
			state_root_check: bool,
			signature_check: bool,
			select: pezframe_try_runtime::TryStateSelect,
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl pezframe_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<pezframe_benchmarking::BenchmarkList>,
			Vec<pezframe_support::traits::StorageInfo>,
		) {
			use pezframe_benchmarking::BenchmarkList;
			use pezframe_support::traits::StorageInfoTrait;
			use pezframe_system_benchmarking::Pezpallet as SystemBench;
			use pezframe_system_benchmarking::extensions::Pezpallet as SystemExtensionsBench;
			use pezcumulus_pezpallet_session_benchmarking::Pezpallet as SessionBench;
			use pezpallet_xcm::benchmarking::Pezpallet as PalletXcmExtrinsicsBenchmark;
			use pezpallet_xcm_bridge_hub_router::benchmarking::Pezpallet as XcmBridgeHubRouterBench;

			// This is defined once again in dispatch_benchmark, because list_benchmarks!
			// and add_benchmarks! are macros exported by define_benchmarks! macros and those types
			// are referenced in that call.
			type XcmBalances = pezpallet_xcm_benchmarks::fungible::Pezpallet::<Runtime>;
			type XcmGeneric = pezpallet_xcm_benchmarks::generic::Pezpallet::<Runtime>;

			// Benchmark files generated for `Assets/ForeignAssets` instances are by default
			// `pezpallet_assets_assets.rs / pezpallet_assets_foreign_assets`, which is not really nice,
			// so with this redefinition we can change names to nicer:
			// `pezpallet_assets_local.rs / pezpallet_assets_foreign.rs`.
			type Local = pezpallet_assets::Pezpallet::<Runtime, TrustBackedAssetsInstance>;
			type Foreign = pezpallet_assets::Pezpallet::<Runtime, ForeignAssetsInstance>;
			type Pool = pezpallet_assets::Pezpallet::<Runtime, PoolAssetsInstance>;

			type ToZagros = XcmBridgeHubRouterBench<Runtime, ToZagrosXcmRouterInstance>;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		#[allow(non_local_definitions)]
		fn dispatch_benchmark(
			config: pezframe_benchmarking::BenchmarkConfig
		) -> Result<Vec<pezframe_benchmarking::BenchmarkBatch>, alloc::string::String> {
			use pezframe_benchmarking::{BenchmarkBatch, BenchmarkError};
			use pezframe_support::assert_ok;
			use pezsp_storage::TrackedStorageKey;

			use pezframe_system_benchmarking::Pezpallet as SystemBench;
			use pezframe_system_benchmarking::extensions::Pezpallet as SystemExtensionsBench;
			impl pezframe_system_benchmarking::Config for Runtime {
				fn setup_set_code_requirements(code: &alloc::vec::Vec<u8>) -> Result<(), BenchmarkError> {
					TeyrchainSystem::initialize_for_set_code_benchmark(code.len() as u32);
					Ok(())
				}

				fn verify_set_code() {
					System::assert_last_event(pezcumulus_pezpallet_teyrchain_system::Event::<Runtime>::ValidationFunctionStored.into());
				}
			}

			use pezcumulus_pezpallet_session_benchmarking::Pezpallet as SessionBench;
			impl pezcumulus_pezpallet_session_benchmarking::Config for Runtime {}

			use pezpallet_xcm_bridge_hub_router::benchmarking::{
				Pezpallet as XcmBridgeHubRouterBench,
				Config as XcmBridgeHubRouterConfig,
			};

			use testnet_teyrchains_constants::pezkuwichain::locations::{PeopleParaId, PeopleLocation};
			parameter_types! {
				pub ExistentialDepositAsset: Option<Asset> = Some((
					TokenLocation::get(),
					ExistentialDeposit::get()
				).into());

				pub const RandomParaId: ParaId = ParaId::new(43211234);
			}

			use pezpallet_xcm::benchmarking::Pezpallet as PalletXcmExtrinsicsBenchmark;
			impl pezpallet_xcm::benchmarking::Config for Runtime {
				type DeliveryHelper = (
				pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
						xcm_config::XcmConfig,
						ExistentialDepositAsset,
						PriceForSiblingTeyrchainDelivery,
						RandomParaId,
						TeyrchainSystem
					>,
				pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
						xcm_config::XcmConfig,
						ExistentialDepositAsset,
						PriceForSiblingTeyrchainDelivery,
						PeopleParaId,
						TeyrchainSystem
					>);

				fn reachable_dest() -> Option<Location> {
					Some(PeopleLocation::get())
				}

				fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
					// Relay/native token can be teleported between AH and Relay.
					Some((
						Asset {
							fun: Fungible(ExistentialDeposit::get()),
							id: AssetId(TokenLocation::get())
						},
						PeopleLocation::get(),
					))
				}

				fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
					// We get an account to create USDT and give it enough ZGR to exist.
					let account = pezframe_benchmarking::whitelisted_caller();
					assert_ok!(<Balances as fungible::Mutate<_>>::mint_into(
						&account,
						ExistentialDeposit::get() + (1_000 * UNITS)
					));

					// We then create USDT.
					let usdt_id = 1984u32;
					let usdt_location = Location::new(0, [PalletInstance(50), GeneralIndex(usdt_id.into())]);
					assert_ok!(Assets::force_create(
						RuntimeOrigin::root(),
						usdt_id.into(),
						account.clone().into(),
						true,
						1
					));

					// And return USDT as the reserve transferable asset.
					Some((
						Asset { fun: Fungible(ExistentialDeposit::get()), id: AssetId(usdt_location) },
						ParentThen(Teyrchain(RandomParaId::get().into()).into()).into(),
					))
				}

				fn set_up_complex_asset_transfer(
				) -> Option<(XcmAssets, AssetId, Location, alloc::boxed::Box<dyn FnOnce()>)> {
					let dest = PeopleLocation::get();

					let fee_amount = EXISTENTIAL_DEPOSIT;
					let fee_asset: Asset = (TokenLocation::get(), fee_amount).into();

					let who = pezframe_benchmarking::whitelisted_caller();
					// Give some multiple of the existential deposit
					let balance = fee_amount + EXISTENTIAL_DEPOSIT * 1000;
					let _ = <Balances as pezframe_support::traits::Currency<_>>::make_free_balance_be(
						&who, balance,
					);
					// verify initial balance
					assert_eq!(Balances::free_balance(&who), balance);

					// set up local asset
					let asset_amount = 10u128;
					let initial_asset_amount = asset_amount * 10;
					let (asset_id, _, _) = pezpallet_assets::benchmarking::create_default_minted_asset::<
						Runtime,
						pezpallet_assets::Instance1
					>(true, initial_asset_amount);
					let asset_location = Location::new(
						0,
						[PalletInstance(50), GeneralIndex(u32::from(asset_id).into())]
					);
					let transfer_asset: Asset = (asset_location, asset_amount).into();

					let assets: XcmAssets = vec![fee_asset.clone(), transfer_asset].into();
					let fee_asset_id = fee_asset.id;

					// verify transferred successfully
					let verify = alloc::boxed::Box::new(move || {
						// verify native balance after transfer, decreased by transferred fee amount
						// (plus transport fees)
						assert!(Balances::free_balance(&who) <= balance - fee_amount);
						// verify asset balance decreased by exactly transferred amount
						assert_eq!(
							Assets::balance(asset_id.into(), &who),
							initial_asset_amount - asset_amount,
						);
					});
					Some((assets, fee_asset_id, dest, verify))
				}

				fn get_asset() -> Asset {
					use pezframe_benchmarking::whitelisted_caller;
					use pezframe_support::traits::tokens::fungible::{Inspect, Mutate};
					let account = whitelisted_caller();
					assert_ok!(<Balances as Mutate<_>>::mint_into(
						&account,
						<Balances as Inspect<_>>::minimum_balance(),
					));
					let asset_id = 1984;
					assert_ok!(Assets::force_create(
						RuntimeOrigin::root(),
						asset_id.into(),
						account.into(),
						true,
						1u128,
					));
					let amount = 1_000_000u128;
					let asset_location = Location::new(0, [PalletInstance(50), GeneralIndex(u32::from(asset_id).into())]);

					Asset {
						id: AssetId(asset_location),
						fun: Fungible(amount),
					}
				}
			}

			impl XcmBridgeHubRouterConfig<ToZagrosXcmRouterInstance> for Runtime {
				fn make_congested() {
					pezcumulus_pezpallet_xcmp_queue::bridging::suspend_channel_for_benchmarks::<Runtime>(
						xcm_config::bridging::SiblingBridgeHubParaId::get().into()
					);
				}
				fn ensure_bridged_target_destination() -> Result<Location, BenchmarkError> {
					TeyrchainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(
						xcm_config::bridging::SiblingBridgeHubParaId::get().into()
					);
					let bridged_asset_hub = xcm_config::bridging::to_zagros::AssetHubZagros::get();
					let _ = PezkuwiXcm::force_xcm_version(
						RuntimeOrigin::root(),
						alloc::boxed::Box::new(bridged_asset_hub.clone()),
						XCM_VERSION,
					).map_err(|e| {
						tracing::error!(
							target: "bridges::benchmark",
							error=?e,
							origin=?RuntimeOrigin::root(),
							location=?bridged_asset_hub,
							version=?XCM_VERSION,
							"Failed to dispatch `force_xcm_version`"
						);
						BenchmarkError::Stop("XcmVersion was not stored!")
					})?;
					Ok(bridged_asset_hub)
				}
			}

			use xcm_config::{TokenLocation, MaxAssetsIntoHolding};
			use pezpallet_xcm_benchmarks::asset_instance_from;

			impl pezpallet_xcm_benchmarks::Config for Runtime {
				type XcmConfig = xcm_config::XcmConfig;
				type AccountIdConverter = xcm_config::LocationToAccountId;
				type DeliveryHelper = pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
						xcm_config::XcmConfig,
						ExistentialDepositAsset,
						PriceForSiblingTeyrchainDelivery,
						PeopleParaId,
						TeyrchainSystem
					>;
				fn valid_destination() -> Result<Location, BenchmarkError> {
					Ok(PeopleLocation::get())
				}
				fn worst_case_holding(depositable_count: u32) -> XcmAssets {
					// A mix of fungible, non-fungible, and concrete assets.
					let holding_non_fungibles = MaxAssetsIntoHolding::get() / 2 - depositable_count;
					let holding_fungibles = holding_non_fungibles.saturating_sub(2);  // -2 for two `iter::once` bellow
					let fungibles_amount: u128 = 100;
					(0..holding_fungibles)
						.map(|i| {
							Asset {
								id: GeneralIndex(i as u128).into(),
								fun: Fungible(fungibles_amount * (i + 1) as u128), // non-zero amount
							}
						})
						.chain(core::iter::once(Asset { id: Here.into(), fun: Fungible(u128::MAX) }))
						.chain(core::iter::once(Asset { id: AssetId(TokenLocation::get()), fun: Fungible(1_000_000 * UNITS) }))
						.chain((0..holding_non_fungibles).map(|i| Asset {
							id: GeneralIndex(i as u128).into(),
							fun: NonFungible(asset_instance_from(i)),
						}))
						.collect::<Vec<_>>()
						.into()
				}
			}

			parameter_types! {
				pub TrustedTeleporter: Option<(Location, Asset)> = Some((
					PeopleLocation::get(),
					Asset { fun: Fungible(UNITS), id: AssetId(TokenLocation::get()) },
				));
				pub const CheckedAccount: Option<(AccountId, xcm_builder::MintLocation)> = None;
				// AssetHubPezkuwichain trusts AssetHubZagros as reserve for WNDs
				pub TrustedReserve: Option<(Location, Asset)> = Some(
					(
						xcm_config::bridging::to_zagros::AssetHubZagros::get(),
						Asset::from((xcm_config::bridging::to_zagros::WndLocation::get(), 1000000000000 as u128))
					)
				);
			}

			impl pezpallet_xcm_benchmarks::fungible::Config for Runtime {
				type TransactAsset = Balances;

				type CheckedAccount = CheckedAccount;
				type TrustedTeleporter = TrustedTeleporter;
				type TrustedReserve = TrustedReserve;

				fn get_asset() -> Asset {
					use pezframe_support::traits::tokens::fungible::{Inspect, Mutate};
					let (account, _) = pezpallet_xcm_benchmarks::account_and_location::<Runtime>(1);
					assert_ok!(<Balances as Mutate<_>>::mint_into(
						&account,
						<Balances as Inspect<_>>::minimum_balance(),
					));
					let asset_id = 1984;
					assert_ok!(Assets::force_create(
						RuntimeOrigin::root(),
						asset_id.into(),
						account.clone().into(),
						true,
						1u128,
					));
					let amount = 1_000_000u128;
					let asset_location = Location::new(0, [PalletInstance(50), GeneralIndex(u32::from(asset_id).into())]);

					Asset {
						id: AssetId(asset_location),
						fun: Fungible(amount),
					}
				}
			}

			impl pezpallet_xcm_benchmarks::generic::Config for Runtime {
				type TransactAsset = Balances;
				type RuntimeCall = RuntimeCall;

				fn worst_case_response() -> (u64, Response) {
					(0u64, Response::Version(Default::default()))
				}

				fn worst_case_asset_exchange() -> Result<(XcmAssets, XcmAssets), BenchmarkError> {
					Err(BenchmarkError::Skip)
				}

				fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
					xcm_config::bridging::BridgingBenchmarksHelper::prepare_universal_alias()
					.ok_or(BenchmarkError::Skip)
				}

				fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
					Ok((
						PeopleLocation::get(),
						pezframe_system::Call::remark_with_event {remark: vec![]}.into()
					))
				}

				fn subscribe_origin() -> Result<Location, BenchmarkError> {
					Ok(PeopleLocation::get())
				}

				fn claimable_asset() -> Result<(Location, Location, XcmAssets), BenchmarkError> {
					let origin = PeopleLocation::get();
					let assets: XcmAssets = (TokenLocation::get(), 1_000 * UNITS).into();
					let ticket = Location { parents: 0, interior: Here };
					Ok((origin, ticket, assets))
				}

				fn worst_case_for_trader() -> Result<(Asset, WeightLimit), BenchmarkError> {
					Ok((Asset {
						id: AssetId(TokenLocation::get()),
						fun: Fungible(1_000_000 * UNITS),
					}, WeightLimit::Limited(Weight::from_parts(5000, 5000))))
				}

				fn unlockable_asset() -> Result<(Location, Location, Asset), BenchmarkError> {
					Err(BenchmarkError::Skip)
				}

				fn export_message_origin_and_destination(
				) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
					Err(BenchmarkError::Skip)
				}

				fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
					// Any location can alias to an internal location.
					// Here teyrchain 1001 aliases to an internal account.
					Ok((
						Location::new(1, [Teyrchain(1001)]),
						Location::new(1, [Teyrchain(1001), AccountId32 { id: [111u8; 32], network: None }]),
					))
				}
			}

			type XcmBalances = pezpallet_xcm_benchmarks::fungible::Pezpallet::<Runtime>;
			type XcmGeneric = pezpallet_xcm_benchmarks::generic::Pezpallet::<Runtime>;

			type Local = pezpallet_assets::Pezpallet::<Runtime, TrustBackedAssetsInstance>;
			type Foreign = pezpallet_assets::Pezpallet::<Runtime, ForeignAssetsInstance>;
			type Pool = pezpallet_assets::Pezpallet::<Runtime, PoolAssetsInstance>;

			type ToZagros = XcmBridgeHubRouterBench<Runtime, ToZagrosXcmRouterInstance>;

			use pezframe_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	impl pezsp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> pezsp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<pezsp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, &genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<pezsp_genesis_builder::PresetId> {
			genesis_config_presets::preset_names()
		}
	}

	impl xcm_runtime_pezapis::trusted_query::TrustedQueryApi<Block> for Runtime {
		fn is_trusted_reserve(asset: VersionedAsset, location: VersionedLocation) -> xcm_runtime_pezapis::trusted_query::XcmTrustedQueryResult {
			PezkuwiXcm::is_trusted_reserve(asset, location)
		}
		fn is_trusted_teleporter(asset: VersionedAsset, location: VersionedLocation) -> xcm_runtime_pezapis::trusted_query::XcmTrustedQueryResult {
			PezkuwiXcm::is_trusted_teleporter(asset, location)
		}
	}

	impl pezcumulus_primitives_core::GetTeyrchainInfo<Block> for Runtime {
		fn teyrchain_id() -> ParaId {
			TeyrchainInfo::teyrchain_id()
		}
	}

	impl pezcumulus_primitives_core::TargetBlockRate<Block> for Runtime {
		fn target_block_rate() -> u32 {
			1
		}
	}

	impl xcm_runtime_pezapis::authorized_aliases::AuthorizedAliasersApi<Block> for Runtime {
		fn authorized_aliasers(target: VersionedLocation) -> Result<
			Vec<xcm_runtime_pezapis::authorized_aliases::OriginAliaser>,
			xcm_runtime_pezapis::authorized_aliases::Error
		> {
			PezkuwiXcm::authorized_aliasers(target)
		}
		fn is_authorized_alias(origin: VersionedLocation, target: VersionedLocation) -> Result<
			bool,
			xcm_runtime_pezapis::authorized_aliases::Error
		> {
			PezkuwiXcm::is_authorized_alias(origin, target)
		}
	}
}

pezcumulus_pezpallet_teyrchain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = pezcumulus_pezpallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
