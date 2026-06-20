// This file is part of Pezcumulus.
// SPDX-License-Identifier: Unlicense

// This is free and unencumbered software released into the public domain.

// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.

// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.

// For more information, please refer to <http://unlicense.org/>

//! The PenPal runtime is designed as a test runtime that can be created with an arbitrary `ParaId`,
//! such that multiple instances of the teyrchain can be on the same parent relay. Ensure that you
//! have enough nodes running to support this or you will get scheduling errors.
//!
//! The PenPal runtime's primary use is for testing interactions between System teyrchains and
//! other chains that are not trusted teleporters.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod genesis_config_presets;
mod weights;
pub mod xcm_config;

extern crate alloc;

use alloc::{vec, vec::Vec};
pub use pez_assets_common::local_and_foreign_assets::ForeignAssetReserveData;
use pez_assets_common::{
	foreign_creators::ForeignCreators,
	local_and_foreign_assets::{LocalFromLeft, TargetFromLeft},
	AssetIdForTrustBackedAssetsConvert,
};
use pezcumulus_pezpallet_teyrchain_system::RelayNumberStrictlyIncreases;
use pezcumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use pezframe_support::{
	construct_runtime, derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	ord_parameter_types, parameter_types,
	pezpallet_prelude::Weight,
	traits::{
		tokens::{fungible, fungibles, imbalance::ResolveAssetTo},
		AsEnsureOriginWithArg, ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, Everything,
		TransformOrigin,
	},
	weights::{
		constants::WEIGHT_REF_TIME_PER_SECOND, ConstantMultiplier, FeePolynomial,
		WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	},
	PalletId,
};
use pezframe_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureSigned, EnsureSignedBy,
};
use pezkuwi_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};
use pezpallet_revive::evm::runtime::EthExtra;
use pezsp_api::impl_runtime_apis;
pub use pezsp_consensus_aura::sr25519::AuthorityId as AuraId;
use pezsp_core::{crypto::KeyTypeId, OpaqueMetadata};
use pezsp_runtime::{
	generic, impl_opaque_keys,
	traits::{AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU128,
};
pub use pezsp_runtime::{traits::ConvertInto, MultiAddress, Perbill, Permill};
use smallvec::smallvec;
use testnet_teyrchains_constants::zagros::{consensus::*, time::*};
use teyrchains_common::{
	impls::{AssetsToBlockAuthor, NonZeroIssuance},
	message_queue::{NarrowOriginToSibling, ParaIdToSibling},
	AccountId, Balance, BlockNumber, Hash, Header, Nonce, Signature,
};
use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};
use xcm::{
	latest::prelude::{AssetId as AssetLocationId, BodyId},
	Version as XcmVersion, VersionedAsset, VersionedAssetId, VersionedAssets, VersionedLocation,
	VersionedXcm,
};
use xcm_runtime_pezapis::{
	dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
	fees::Error as XcmPaymentApiError,
};

#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;
#[cfg(feature = "std")]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;
use xcm_config::{
	ForeignAssetsAssetId, LocationToAccountId, XcmConfig, XcmOriginToTransactDispatchOrigin,
};

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

// Id used for identifying assets.
pub type AssetId = u32;

/// The extension to the basic transaction logic.
pub type TxExtension = (
	pezframe_system::AuthorizeCall<Runtime>,
	pezframe_system::CheckNonZeroSender<Runtime>,
	pezframe_system::CheckSpecVersion<Runtime>,
	pezframe_system::CheckTxVersion<Runtime>,
	pezframe_system::CheckGenesis<Runtime>,
	pezframe_system::CheckEra<Runtime>,
	pezframe_system::CheckNonce<Runtime>,
	pezframe_system::CheckWeight<Runtime>,
	pezpallet_asset_tx_payment::ChargeAssetTxPayment<Runtime>,
	pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
	pezpallet_revive::evm::tx_extension::SetOrigin<Runtime>,
	pezframe_system::WeightReclaim<Runtime>,
);

/// Default extensions applied to Ethereum transactions.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EthExtraImpl;

impl EthExtra for EthExtraImpl {
	type Config = Runtime;
	type Extension = TxExtension;

	fn get_eth_extension(nonce: u32, tip: Balance) -> Self::Extension {
		(
			pezframe_system::AuthorizeCall::<Runtime>::new(),
			pezframe_system::CheckNonZeroSender::<Runtime>::new(),
			pezframe_system::CheckSpecVersion::<Runtime>::new(),
			pezframe_system::CheckTxVersion::<Runtime>::new(),
			pezframe_system::CheckGenesis::<Runtime>::new(),
			pezframe_system::CheckEra::<Runtime>::from(generic::Era::Immortal),
			pezframe_system::CheckNonce::<Runtime>::from(nonce),
			pezframe_system::CheckWeight::<Runtime>::new(),
			pezpallet_asset_tx_payment::ChargeAssetTxPayment::<Runtime>::from(tip, None),
			pezframe_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
			pezpallet_revive::evm::tx_extension::SetOrigin::<Runtime>::new_from_eth_transaction(),
			pezframe_system::WeightReclaim::<Runtime>::new(),
		)
			.into()
	}
}

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	pezpallet_revive::evm::runtime::UncheckedExtrinsic<Address, Signature, EthExtraImpl>;

pub type Migrations = (
	pezpallet_balances::migration::MigrateToTrackInactive<Runtime, xcm_config::CheckingAccount>,
	pezpallet_collator_selection::migration::v1::MigrateToV1<Runtime>,
	pezpallet_session::migrations::v1::MigrateV0ToV1<
		Runtime,
		pezpallet_session::migrations::v1::InitOffenceSeverity<Runtime>,
	>,
);

/// Executive: handles dispatch to the various modules.
pub type Executive = pezframe_executive::Executive<
	Runtime,
	Block,
	pezframe_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl pezframe_support::weights::WeightToFee for WeightToFee {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		let time_poly: FeePolynomial<Balance> = RefTimeToFee::polynomial().into();
		let proof_poly: FeePolynomial<Balance> = ProofSizeToFee::polynomial().into();

		// Take the maximum instead of the sum to charge by the more scarce resource.
		time_poly.eval(weight.ref_time()).max(proof_poly.eval(weight.proof_size()))
	}
}

/// Maps the reference time component of `Weight` to a fee.
pub struct RefTimeToFee;
impl WeightToFeePolynomial for RefTimeToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		let p = MILLIUNIT / 10;
		let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());

		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

/// Maps the proof size component of `Weight` to a fee.
pub struct ProofSizeToFee;
impl WeightToFeePolynomial for ProofSizeToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// Map 10kb proof to 1 CENT.
		let p = MILLIUNIT / 10;
		let q = 10_000;

		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}
/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	use pezsp_runtime::{generic, traits::BlakeTwo256};

	pub use pezsp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("penpal-teyrchain"),
	impl_name: alloc::borrow::Cow::Borrowed("penpal-teyrchain"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLIUNIT: Balance = 1_000_000_000;
pub const MICROUNIT: Balance = 1_000_000;

/// The existential deposit. Set to 1/10 of the Connected Relay Chain.
pub const EXISTENTIAL_DEPOSIT: Balance = MILLIUNIT;

/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
/// `Operational` extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// We allow for 0.5 of a second of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
	pezcumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;

	// This part is copied from Bizinikiwi's `bin/node/runtime/src/lib.rs`.
	//  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
	// `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
	// the lazy contract deletion.
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
	pub const SS58Prefix: u16 = 42;
}

// Configure FRAME pallets to include in runtime.

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The block type.
	type Block = Block;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	/// The data to be stored in an account.
	type AccountData = pezpallet_balances::AccountData<Balance>;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = Everything;
	/// Weight information for the extrinsics of this pezpallet.
	type SystemWeightInfo = ();
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// This is used as an identifier of the chain. 42 is the generic bizinikiwi prefix.
	type SS58Prefix = SS58Prefix;
	/// The action to take on a Runtime Upgrade
	type OnSetCode = pezcumulus_pezpallet_teyrchain_system::TeyrchainSetCode<Self>;
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
	type SingleBlockMigrations = Migrations;
}

impl pezpallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
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
	type WeightInfo = pezpallet_balances::weights::BizinikiwiWeight<Runtime>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type DoneSlashHandler = ();
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = 10 * MICROUNIT;
}

impl pezpallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pezpallet_transaction_payment::FungibleAdapter<Balances, ()>;
	type WeightToFee = pezpallet_revive::evm::fees::BlockRatioFee<1, 1, Self>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightInfo = ();
}

parameter_types! {
	pub const AssetDeposit: Balance = 0;
	pub const AssetAccountDeposit: Balance = 0;
	pub const ApprovalDeposit: Balance = 0;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 0;
	pub const MetadataDepositPerByte: Balance = 0;
}

// /// We allow root and the Relay Chain council to execute privileged asset operations.
// pub type AssetsForceOrigin =
// 	EnsureOneOf<EnsureRoot<AccountId>, EnsureXcm<IsMajorityOfBody<KsmLocation, ExecutiveBody>>>;

pub type TrustBackedAssetsInstance = pezpallet_assets::Instance1;

impl pezpallet_assets::Config<TrustBackedAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type AssetIdParameter = codec::Compact<AssetId>;
	type ReserveData = ();
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pezpallet_assets::weights::BizinikiwiWeight<Runtime>;
	type CallbackHandle = ();
	type AssetAccountDeposit = AssetAccountDeposit;
	type RemoveItemsLimit = pezframe_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	// we just reuse the same deposits
	pub const ForeignAssetsAssetDeposit: Balance = AssetDeposit::get();
	pub const ForeignAssetsAssetAccountDeposit: Balance = AssetAccountDeposit::get();
	pub const ForeignAssetsApprovalDeposit: Balance = ApprovalDeposit::get();
	pub const ForeignAssetsAssetsStringLimit: u32 = AssetsStringLimit::get();
	pub const ForeignAssetsMetadataDepositBase: Balance = MetadataDepositBase::get();
	pub const ForeignAssetsMetadataDepositPerByte: Balance = MetadataDepositPerByte::get();
}

/// Another pezpallet assets instance to store foreign assets from bridgehub.
pub type ForeignAssetsInstance = pezpallet_assets::Instance2;
impl pezpallet_assets::Config<ForeignAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = ForeignAssetsAssetId;
	type AssetIdParameter = ForeignAssetsAssetId;
	type ReserveData = ForeignAssetReserveData;
	type Currency = Balances;
	// This is to allow any other remote location to create foreign assets. Used in tests, not
	// recommended on real chains.
	type CreateOrigin =
		ForeignCreators<Everything, LocationToAccountId, AccountId, xcm::latest::Location>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = ForeignAssetsAssetDeposit;
	type MetadataDepositBase = ForeignAssetsMetadataDepositBase;
	type MetadataDepositPerByte = ForeignAssetsMetadataDepositPerByte;
	type ApprovalDeposit = ForeignAssetsApprovalDeposit;
	type StringLimit = ForeignAssetsAssetsStringLimit;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pezpallet_assets::weights::BizinikiwiWeight<Runtime>;
	type CallbackHandle = ();
	type AssetAccountDeposit = ForeignAssetsAssetAccountDeposit;
	type RemoveItemsLimit = pezframe_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pez_assets_common::benchmarks::LocationAssetsBenchmarkHelper;
}

parameter_types! {
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
	pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);
}

ord_parameter_types! {
	pub const AssetConversionOrigin: pezsp_runtime::AccountId32 =
		AccountIdConversion::<pezsp_runtime::AccountId32>::into_account_truncating(&AssetConversionPalletId::get());
}

pub type AssetsForceOrigin = EnsureRoot<AccountId>;

pub type PoolAssetsInstance = pezpallet_assets::Instance3;
impl pezpallet_assets::Config<PoolAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetId = u32;
	type AssetIdParameter = u32;
	type ReserveData = ();
	type Currency = Balances;
	type CreateOrigin =
		AsEnsureOriginWithArg<EnsureSignedBy<AssetConversionOrigin, pezsp_runtime::AccountId32>>;
	type ForceOrigin = AssetsForceOrigin;
	type AssetDeposit = ConstU128<0>;
	type AssetAccountDeposit = ConstU128<0>;
	type MetadataDepositBase = ConstU128<0>;
	type MetadataDepositPerByte = ConstU128<0>;
	type ApprovalDeposit = ConstU128<0>;
	type StringLimit = ConstU32<50>;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pezpallet_assets::weights::BizinikiwiWeight<Runtime>;
	type CallbackHandle = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

/// Union fungibles implementation for `Assets` and `ForeignAssets`.
pub type LocalAndForeignAssets = fungibles::UnionOf<
	Assets,
	ForeignAssets,
	LocalFromLeft<
		AssetIdForTrustBackedAssetsConvert<
			xcm_config::TrustBackedAssetsPalletLocation,
			xcm::latest::Location,
		>,
		teyrchains_common::AssetIdForTrustBackedAssets,
		xcm::latest::Location,
	>,
	xcm::latest::Location,
	AccountId,
>;

/// Union fungibles implementation for [`LocalAndForeignAssets`] and `Balances`.
pub type NativeAndAssets = fungible::UnionOf<
	Balances,
	LocalAndForeignAssets,
	TargetFromLeft<xcm_config::RelayLocation, xcm::latest::Location>,
	xcm::latest::Location,
	AccountId,
>;

pub type PoolIdToAccountId = pezpallet_asset_conversion::AccountIdConverter<
	AssetConversionPalletId,
	(xcm::latest::Location, xcm::latest::Location),
>;

impl pezpallet_asset_conversion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type HigherPrecisionBalance = pezsp_core::U256;
	type AssetKind = xcm::latest::Location;
	type Assets = NativeAndAssets;
	type PoolId = (Self::AssetKind, Self::AssetKind);
	type PoolLocator = pezpallet_asset_conversion::WithFirstAsset<
		xcm_config::RelayLocation,
		AccountId,
		Self::AssetKind,
		PoolIdToAccountId,
	>;
	type PoolAssetId = u32;
	type PoolAssets = PoolAssets;
	type PoolSetupFee = ConstU128<0>; // Asset class deposit fees are sufficient to prevent spam
	type PoolSetupFeeAsset = xcm_config::RelayLocation;
	type PoolSetupFeeTarget = ResolveAssetTo<AssetConversionOrigin, Self::Assets>;
	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
	type LPFee = ConstU32<3>;
	type PalletId = AssetConversionPalletId;
	type MaxSwapPathLength = ConstU32<3>;
	type MintMinLiquidity = ConstU128<100>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pez_assets_common::benchmarks::AssetPairFactory<
		xcm_config::RelayLocation,
		teyrchain_info::Pezpallet<Runtime>,
		xcm_config::TrustBackedAssetsPalletIndex,
		xcm::latest::Location,
	>;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

type ConsensusHook = pezcumulus_pezpallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,
	RELAY_CHAIN_SLOT_DURATION_MILLIS,
	BLOCK_PROCESSING_VELOCITY,
	UNINCLUDED_SEGMENT_CAPACITY,
>;

impl pezcumulus_pezpallet_teyrchain_system::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = teyrchain_info::Pezpallet<Runtime>;
	type DmpQueue = pezframe_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type OutboundXcmpMessageSource = XcmpQueue;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
	type ConsensusHook = pezcumulus_pezpallet_aura_ext::FixedVelocityConsensusHook<
		Runtime,
		RELAY_CHAIN_SLOT_DURATION_MILLIS,
		BLOCK_PROCESSING_VELOCITY,
		UNINCLUDED_SEGMENT_CAPACITY,
	>;

	type RelayParentOffset = ConstU32<0>;
}

impl teyrchain_info::Config for Runtime {}

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pezpallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
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

impl pezcumulus_pezpallet_aura_ext::Config for Runtime {}

parameter_types! {
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetLocationId = AssetLocationId(xcm_config::RelayLocation::get());
	/// The base fee for the message delivery fees (3 CENTS).
	pub const BaseDeliveryFee: u128 = (1_000_000_000_000u128 / 100).saturating_mul(3);
}

pub type PriceForSiblingTeyrchainDelivery = pezkuwi_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	TransactionByteFee,
	XcmpQueue,
>;

impl pezcumulus_pezpallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = TeyrchainSystem;
	type VersionWrapper = PezkuwiXcm;
	// Enqueue XCMP messages from siblings for later processing.
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = ConstU32<1_000>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	// Most on-chain HRMP channels are configured to use 102400 bytes of max message size, so we
	// need to set the page size larger than that until we reduce the channel size on-chain.
	type MaxPageSize = ConstU32<{ 103 * 1024 }>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
	type PriceForSiblingDelivery = PriceForSiblingTeyrchainDelivery;
}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}
impl pezpallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as pezframe_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pezpallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pezpallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pezpallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but let's be pedantic.
	type SessionHandler = <SessionKeys as pezsp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisablingStrategy = ();
	type WeightInfo = ();
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
	pub const SessionLength: BlockNumber = 6 * HOURS;
	pub const ExecutiveBody: BodyId = BodyId::Executive;
}

// We allow root only to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EnsureRoot<AccountId>;

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
	type WeightInfo = ();
}

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetTxHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_asset_tx_payment::BenchmarkHelperTrait<AccountId, u32, u32> for AssetTxHelper {
	fn create_asset_id_parameter(_id: u32) -> (u32, u32) {
		unimplemented!("Penpal uses default weights");
	}
	fn setup_balances_and_pool(_asset_id: u32, _account: AccountId) {
		unimplemented!("Penpal uses default weights");
	}
}

impl pezpallet_asset_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Fungibles = Assets;
	type OnChargeAssetTransaction = pezpallet_asset_tx_payment::FungiblesAdapter<
		pezpallet_assets::BalanceToAssetBalance<
			Balances,
			Runtime,
			ConvertInto,
			TrustBackedAssetsInstance,
		>,
		AssetsToBlockAuthor<Runtime, TrustBackedAssetsInstance>,
	>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetTxHelper;
}

parameter_types! {
	pub const DepositPerItem: Balance = 0;
	pub const DepositPerChildTrieItem: Balance = 0;
	pub const DepositPerByte: Balance = 0;
	pub CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
	pub const MaxEthExtrinsicWeight: FixedU128 = FixedU128::from_rational(9, 10);
}

impl pezpallet_revive::Config for Runtime {
	type Time = Timestamp;
	type Balance = Balance;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type RuntimeOrigin = RuntimeOrigin;
	type DepositPerItem = DepositPerItem;
	type DepositPerChildTrieItem = DepositPerChildTrieItem;
	type DepositPerByte = DepositPerByte;
	type WeightInfo = pezpallet_revive::weights::BizinikiwiWeight<Self>;
	type Precompiles = ();
	type AddressMapper = pezpallet_revive::AccountId32Mapper<Self>;
	type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
	type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
	type UnsafeUnstableInterface = ConstBool<true>;
	type AllowEVMBytecode = ConstBool<true>;
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = EnsureSigned<Self::AccountId>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type ChainId = ConstU64<420_420_999>;
	type NativeToEthRatio = ConstU32<1_000_000>; // 10^(18 - 12) Eth is 10^18, Native is 10^12.
	type FindAuthor = <Runtime as pezpallet_authorship::Config>::FindAuthor;
	type FeeInfo = pezpallet_revive::evm::fees::Info<Address, Signature, EthExtraImpl>;
	type MaxEthExtrinsicWeight = MaxEthExtrinsicWeight;
	type DebugEnabled = ConstBool<false>;
}

impl pezpallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pezpallet_sudo::weights::BizinikiwiWeight<Runtime>;
}

impl pezpallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pezpallet_utility::weights::BizinikiwiWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime
	{
		// System support stuff.
		System: pezframe_system = 0,
		TeyrchainSystem: pezcumulus_pezpallet_teyrchain_system = 1,
		Timestamp: pezpallet_timestamp = 2,
		TeyrchainInfo: teyrchain_info = 3,

		// Monetary stuff.
		Balances: pezpallet_balances = 10,
		TransactionPayment: pezpallet_transaction_payment = 11,
		AssetTxPayment: pezpallet_asset_tx_payment = 12,

		// Collator support. The order of these 4 are important and shall not change.
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

		// The main stage.
		Assets: pezpallet_assets::<Instance1> = 50,
		ForeignAssets: pezpallet_assets::<Instance2> = 51,
		PoolAssets: pezpallet_assets::<Instance3> = 52,
		AssetConversion: pezpallet_asset_conversion = 53,

		Revive: pezpallet_revive = 60,

		Sudo: pezpallet_sudo = 255,
	}
);

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	pezframe_benchmarking::define_benchmarks!(
		[pezframe_system, SystemBench::<Runtime>]
		[pezframe_system_extensions, SystemExtensionsBench::<Runtime>]
		[pezpallet_balances, Balances]
		[pezpallet_message_queue, MessageQueue]
		[pezpallet_session, SessionBench::<Runtime>]
		[pezpallet_sudo, Sudo]
		[pezpallet_timestamp, Timestamp]
		[pezpallet_collator_selection, CollatorSelection]
		[pezcumulus_pezpallet_teyrchain_system, TeyrchainSystem]
		[pezcumulus_pezpallet_xcmp_queue, XcmpQueue]
		[pezpallet_utility, Utility]
	);
}

pezpallet_revive::impl_runtime_apis_plus_revive_traits!(
	Runtime,
	Revive,
	Executive,
	EthExtraImpl,

	impl pezsp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> pezsp_consensus_aura::SlotDuration {
			pezsp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
		}

		fn authorities() -> Vec<AuraId> {
			pezpallet_aura::Authorities::<Runtime>::get().into_inner()
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

	impl pezcumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> pezcumulus_primitives_core::CollationInfo {
			TeyrchainSystem::collect_collation_info(header)
		}
	}

	impl xcm_runtime_pezapis::fees::XcmPaymentApi<Block> for Runtime {
		fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {
			let acceptable_assets = vec![AssetLocationId(xcm_config::RelayLocation::get())];
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

	impl xcm_runtime_pezapis::trusted_query::TrustedQueryApi<Block> for Runtime {
		fn is_trusted_reserve(asset: VersionedAsset, location: VersionedLocation) -> xcm_runtime_pezapis::trusted_query::XcmTrustedQueryResult {
			PezkuwiXcm::is_trusted_reserve(asset, location)
		}
		fn is_trusted_teleporter(asset: VersionedAsset, location: VersionedLocation) -> xcm_runtime_pezapis::trusted_query::XcmTrustedQueryResult {
			PezkuwiXcm::is_trusted_teleporter(asset, location)
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

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		#[allow(non_local_definitions)]
		fn dispatch_benchmark(
			config: pezframe_benchmarking::BenchmarkConfig
		) -> Result<Vec<pezframe_benchmarking::BenchmarkBatch>, alloc::string::String> {
			use pezframe_benchmarking::BenchmarkBatch;
			use pezsp_storage::TrackedStorageKey;

			use pezframe_system_benchmarking::Pezpallet as SystemBench;
			use pezframe_system_benchmarking::extensions::Pezpallet as SystemExtensionsBench;
			impl pezframe_system_benchmarking::Config for Runtime {}

			use pezcumulus_pezpallet_session_benchmarking::Pezpallet as SessionBench;
			impl pezcumulus_pezpallet_session_benchmarking::Config for Runtime {}

			use pezframe_support::traits::WhitelistedStorageKeys;
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pezpallet.".into()) }
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

	impl pezcumulus_primitives_core::GetTeyrchainInfo<Block> for Runtime {
		fn teyrchain_id() -> ParaId {
			TeyrchainInfo::teyrchain_id()
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
);

pezcumulus_pezpallet_teyrchain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = pezcumulus_pezpallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
