// This file is part of Bizinikiwi.

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

//! The Bizinikiwi runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limits.
#![recursion_limit = "1024"]

extern crate alloc;

#[cfg(feature = "runtime-benchmarks")]
use pezkuwi_sdk::pezsp_core::crypto::FromEntropy;
#[cfg(feature = "runtime-benchmarks")]
use pezpallet_asset_rate::AssetKindFactory;
#[cfg(feature = "runtime-benchmarks")]
use pezpallet_multi_asset_bounties::ArgumentsFactory as PalletMultiAssetBountiesArgumentsFactory;
#[cfg(feature = "runtime-benchmarks")]
use pezpallet_treasury::ArgumentsFactory as PalletTreasuryArgumentsFactory;

use pezkuwi_sdk::*;

use alloc::{vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
pub use pez_node_primitives::{AccountId, Signature};
use pez_node_primitives::{AccountIndex, Balance, BlockNumber, Hash, Moment, Nonce};
use pezframe_election_provider_support::{
	bounds::{ElectionBounds, ElectionBoundsBuilder},
	onchain, BalancingConfig, ElectionDataProvider, SequentialPhragmen, VoteWeight,
};
use pezframe_support::{
	derive_impl,
	dispatch::DispatchClass,
	dynamic_params::{dynamic_pallet_params, dynamic_params},
	genesis_builder_helper::{build_state, get_preset},
	instances::{Instance1, Instance2},
	ord_parameter_types, parameter_types,
	pezpallet_prelude::Get,
	traits::{
		fungible::{
			Balanced, Credit, HoldConsideration, ItemOf, NativeFromLeft, NativeOrWithId, UnionOf,
		},
		tokens::{
			imbalance::{ResolveAssetTo, ResolveTo},
			nonfungibles_v2::Inspect,
			pay::PayAssetFromAccount,
			GetSalary, PayFromAccount, PayWithFungibles,
		},
		AsEnsureOriginWithArg, ConstBool, ConstU128, ConstU16, ConstU32, ConstU64,
		ConstantStoragePrice, Contains, Currency, EitherOfDiverse, EnsureOriginWithArg,
		EqualPrivilegeOnly, InsideBoth, InstanceFilter, KeyOwnerProofSystem, LinearStoragePrice,
		LockIdentifier, Nothing, OnUnbalanced, VariantCountOf, WithdrawReasons,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		ConstantMultiplier, Weight,
	},
	BoundedVec, PalletId,
};
use pezframe_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureRootWithSuccess, EnsureSigned, EnsureSignedBy, EnsureWithSuccess,
};
use pezpallet_asset_conversion::{AccountIdConverter, Ascending, Chain, WithFirstAsset};
use pezpallet_asset_conversion_tx_payment::SwapAssetAdapter;
use pezpallet_assets_precompiles::{InlineIdConfig, ERC20};
use pezpallet_broker::{CoreAssignment, CoreIndex, CoretimeInterface, PartsOf57600, TaskId};
use pezpallet_election_provider_multi_phase::{GeometricDepositBase, SolutionAccuracyOf};
use pezpallet_identity::legacy::IdentityInfo;
use pezpallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pezpallet_nfts::PalletFeatures;
use pezpallet_nis::WithMaximumOf;
use pezpallet_nomination_pools::PoolId;
use pezpallet_revive::evm::runtime::EthExtra;
use pezpallet_session::historical as pezpallet_session_historical;
use pezpallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
pub use pezpallet_transaction_payment::{FungibleAdapter, Multiplier, TargetedFeeAdjustment};
use pezpallet_tx_pause::RuntimeCallNameOf;
use pezsp_api::impl_runtime_apis;
use pezsp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use pezsp_consensus_beefy::{
	ecdsa_crypto::{AuthorityId as BeefyId, Signature as BeefySignature},
	mmr::MmrLeafVersion,
};
use pezsp_consensus_grandpa::AuthorityId as GrandpaId;
use pezsp_core::{crypto::KeyTypeId, OpaqueMetadata};
use pezsp_inherents::{CheckInherentsResult, InherentData};
use pezsp_runtime::{
	curve::PiecewiseLinear,
	generic, impl_opaque_keys, str_array as s,
	traits::{
		self, AccountIdConversion, BlakeTwo256, Block as BlockT, Bounded, ConvertInto,
		MaybeConvert, NumberFor, OpaqueKeys, SaturatedConversion, StaticLookup,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedPointNumber, FixedU128, MultiSignature, MultiSigner, Perbill,
	Percent, Permill, Perquintill, RuntimeDebug,
};
use pezsp_std::{borrow::Cow, prelude::*};
#[cfg(any(feature = "std", test))]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;
use static_assertions::const_assert;

#[cfg(any(feature = "std", test))]
pub use pezframe_system::Call as SystemCall;
#[cfg(any(feature = "std", test))]
pub use pezpallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use pezpallet_sudo::Call as SudoCall;
#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;

pub use pezpallet_staking::StakerStatus;

/// Implementations of some helper traits passed into runtime modules as associated types.
pub mod impls;
#[cfg(not(feature = "runtime-benchmarks"))]
use impls::AllianceIdentityVerifier;
use impls::AllianceProposalProvider;

/// Constant values used within the runtime.
pub mod constants;
use constants::{currency::*, time::*};
use pezsp_runtime::generic::Era;

/// Generated voter bag information.
mod voter_bags;

/// Runtime API definition for assets.
pub mod assets_api;

/// Genesis presets used by this runtime.
pub mod genesis_config_presets;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Max size for serialized extrinsic params for this testing runtime.
/// This is a quite arbitrary but empirically battle tested value.
#[cfg(test)]
pub const CALL_PARAMS_MAX_SIZE: usize = 512;

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is built with \
		 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
		 the flag disabled.",
	)
}

/// Runtime version.
#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("node"),
	impl_name: alloc::borrow::Cow::Borrowed("bizinikiwi-node"),
	authoring_version: 10,
	// Per convention: if the runtime behavior changes, increment spec_version
	// and set impl_version to 0. If only runtime
	// implementation changes and behavior does not, then leave spec_version as
	// is and increment impl_version.
	spec_version: 268,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 2,
	system_version: 1,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: pezsp_consensus_babe::BabeEpochConfiguration =
	pezsp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: pezsp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
	};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time, with maximum proof size.
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
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
	pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

/// Calls that can bypass the safe-mode pezpallet.
pub struct SafeModeWhitelistedCalls;
impl Contains<RuntimeCall> for SafeModeWhitelistedCalls {
	fn contains(call: &RuntimeCall) -> bool {
		match call {
			RuntimeCall::System(_) | RuntimeCall::SafeMode(_) | RuntimeCall::TxPause(_) => true,
			_ => false,
		}
	}
}

/// Calls that cannot be paused by the tx-pause pezpallet.
pub struct TxPauseWhitelistedCalls;
/// Whitelist `Balances::transfer_keep_alive`, all others are pauseable.
impl Contains<RuntimeCallNameOf<Runtime>> for TxPauseWhitelistedCalls {
	fn contains(full_name: &RuntimeCallNameOf<Runtime>) -> bool {
		match (full_name.0.as_slice(), full_name.1.as_slice()) {
			(b"Balances", b"transfer_keep_alive") => true,
			_ => false,
		}
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetRateArguments;
#[cfg(feature = "runtime-benchmarks")]
impl AssetKindFactory<NativeOrWithId<u32>> for AssetRateArguments {
	fn create_asset_kind(seed: u32) -> NativeOrWithId<u32> {
		if seed % 2 > 0 {
			NativeOrWithId::Native
		} else {
			NativeOrWithId::WithId(seed / 2)
		}
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct PalletTreasuryArguments;
#[cfg(feature = "runtime-benchmarks")]
impl PalletTreasuryArgumentsFactory<NativeOrWithId<u32>, AccountId> for PalletTreasuryArguments {
	fn create_asset_kind(seed: u32) -> NativeOrWithId<u32> {
		if seed % 2 > 0 {
			NativeOrWithId::Native
		} else {
			NativeOrWithId::WithId(seed / 2)
		}
	}

	fn create_beneficiary(seed: [u8; 32]) -> AccountId {
		AccountId::from_entropy(&mut seed.as_slice()).unwrap()
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct PalletMultiAssetBountiesArguments;
#[cfg(feature = "runtime-benchmarks")]
impl PalletMultiAssetBountiesArgumentsFactory<NativeOrWithId<u32>, AccountId, u128>
	for PalletMultiAssetBountiesArguments
{
	fn create_asset_kind(seed: u32) -> NativeOrWithId<u32> {
		if seed % 2 > 0 {
			NativeOrWithId::Native
		} else {
			NativeOrWithId::WithId(seed / 2)
		}
	}

	fn create_beneficiary(seed: [u8; 32]) -> AccountId {
		AccountId::from_entropy(&mut seed.as_slice()).unwrap()
	}
}

impl pezpallet_tx_pause::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PauseOrigin = EnsureRoot<AccountId>;
	type UnpauseOrigin = EnsureRoot<AccountId>;
	type WhitelistedCalls = TxPauseWhitelistedCalls;
	type MaxNameLen = ConstU32<256>;
	type WeightInfo = pezpallet_tx_pause::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const EnterDuration: BlockNumber = 4 * HOURS;
	pub const EnterDepositAmount: Balance = 2_000_000 * DOLLARS;
	pub const ExtendDuration: BlockNumber = 2 * HOURS;
	pub const ExtendDepositAmount: Balance = 1_000_000 * DOLLARS;
	pub const ReleaseDelay: u32 = 2 * DAYS;
}

impl pezpallet_safe_mode::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WhitelistedCalls = SafeModeWhitelistedCalls;
	type EnterDuration = EnterDuration;
	type EnterDepositAmount = EnterDepositAmount;
	type ExtendDuration = ExtendDuration;
	type ExtendDepositAmount = ExtendDepositAmount;
	type ForceEnterOrigin = EnsureRootWithSuccess<AccountId, ConstU32<9>>;
	type ForceExtendOrigin = EnsureRootWithSuccess<AccountId, ConstU32<11>>;
	type ForceExitOrigin = EnsureRoot<AccountId>;
	type ForceDepositOrigin = EnsureRoot<AccountId>;
	type ReleaseDelay = ReleaseDelay;
	type Notify = ();
	type WeightInfo = pezpallet_safe_mode::weights::BizinikiwiWeight<Runtime>;
}

#[derive_impl(pezframe_system::config_preludes::SolochainDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type BaseCallFilter = InsideBoth<SafeMode, TxPause>;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type DbWeight = RocksDbWeight;
	type Nonce = Nonce;
	type Hash = Hash;
	type AccountId = AccountId;
	type Lookup = Indices;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type SystemWeightInfo = pezframe_system::weights::BizinikiwiWeight<Runtime>;
	type SS58Prefix = ConstU16<42>;
	type MaxConsumers = ConstU32<16>;
	type MultiBlockMigrator = MultiBlockMigrations;
	type SingleBlockMigrations = Migrations;
}

impl pezpallet_insecure_randomness_collective_flip::Config for Runtime {}

impl pezpallet_example_tasks::Config for Runtime {
	type RuntimeTask = RuntimeTask;
	type WeightInfo = pezpallet_example_tasks::weights::BizinikiwiWeight<Runtime>;
}

impl pezpallet_example_mbm::Config for Runtime {}

impl pezpallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pezpallet_utility::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
}

impl pezpallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = ConstU32<100>;
	type WeightInfo = pezpallet_multisig::weights::BizinikiwiWeight<Runtime>;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
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
	Any,
	NonTransfer,
	Governance,
	Staking,
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
				RuntimeCall::Balances(..)
					| RuntimeCall::Assets(..)
					| RuntimeCall::Uniques(..)
					| RuntimeCall::Nfts(..)
					| RuntimeCall::Vesting(pezpallet_vesting::Call::vested_transfer { .. })
					| RuntimeCall::Indices(pezpallet_indices::Call::transfer { .. })
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Democracy(..)
					| RuntimeCall::Council(..)
					| RuntimeCall::Society(..)
					| RuntimeCall::TechnicalCommittee(..)
					| RuntimeCall::Elections(..)
					| RuntimeCall::Treasury(..)
			),
			ProxyType::Staking => {
				matches!(c, RuntimeCall::Staking(..) | RuntimeCall::FastUnstake(..))
			},
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
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
	type MaxProxies = ConstU32<32>;
	type WeightInfo = pezpallet_proxy::weights::BizinikiwiWeight<Runtime>;
	type MaxPending = ConstU32<32>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
}

impl pezpallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	#[cfg(feature = "runtime-benchmarks")]
	type MaxScheduledPerBlock = ConstU32<512>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = pezpallet_scheduler::weights::BizinikiwiWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

impl pezpallet_glutton::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = EnsureRoot<AccountId>;
	type WeightInfo = pezpallet_glutton::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const PreimageHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Preimage(pezpallet_preimage::HoldReason::Preimage);
}

impl pezpallet_preimage::Config for Runtime {
	type WeightInfo = pezpallet_preimage::weights::BizinikiwiWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<
			dynamic_params::storage::BaseDeposit,
			dynamic_params::storage::ByteDeposit,
			Balance,
		>,
	>;
}

parameter_types! {
	// NOTE: Currently it is not possible to change the epoch duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pezpallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pezpallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type KeyOwnerProof = pezsp_session::MembershipProof;
	type EquivocationReportSystem =
		pezpallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl pezpallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pezpallet_indices::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * DOLLARS;
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pezpallet_balances::Config for Runtime {
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = pezframe_system::Pezpallet<Runtime>;
	type WeightInfo = pezpallet_balances::weights::BizinikiwiWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 10u128);
	pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

impl pezpallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ResolveTo<TreasuryAccount, Balances>>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = pezpallet_revive::evm::fees::BlockRatioFee<1, 1, Self>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = TargetedFeeAdjustment<
		Self,
		TargetBlockFullness,
		AdjustmentVariable,
		MinimumMultiplier,
		MaximumMultiplier,
	>;
	type WeightInfo = pezpallet_transaction_payment::weights::BizinikiwiWeight<Runtime>;
}

pub type AssetsFreezerInstance = pezpallet_assets_freezer::Instance1;
impl pezpallet_assets_freezer::Config<AssetsFreezerInstance> for Runtime {
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeEvent = RuntimeEvent;
}

impl pezpallet_asset_conversion_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = NativeOrWithId<u32>;
	type OnChargeAssetTransaction = SwapAssetAdapter<
		Native,
		NativeAndAssets,
		AssetConversion,
		ResolveAssetTo<TreasuryAccount, NativeAndAssets>,
	>;
	type WeightInfo = pezpallet_asset_conversion_tx_payment::weights::BizinikiwiWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetConversionTxHelper;
}

impl pezpallet_skip_feeless_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pezpallet_timestamp::Config for Runtime {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pezpallet_timestamp::weights::BizinikiwiWeight<Runtime>;
}

impl pezpallet_authorship::Config for Runtime {
	type FindAuthor = pezpallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
		pub mixnet: Mixnet,
		pub beefy: Beefy,
	}
}

impl pezpallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as pezframe_system::Config>::AccountId;
	type ValidatorIdOf = pezsp_runtime::traits::ConvertInto;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pezpallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisablingStrategy = pezpallet_session::disabling::UpToLimitWithReEnablingDisablingStrategy;
	type WeightInfo = pezpallet_session::weights::BizinikiwiWeight<Runtime>;
	type Currency = Balances;
	type KeyDeposit = ();
}

impl pezpallet_session::historical::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type FullIdentification = ();
	type FullIdentificationOf = pezpallet_staking::UnitIdentificationOf<Self>;
}

pezpallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub const SessionsPerEra: pezsp_staking::SessionIndex = 6;
	pub const BondingDuration: pezsp_staking::EraIndex = 24 * 28;
	pub const SlashDeferDuration: pezsp_staking::EraIndex = 24 * 7; // 1/4 the bonding duration.
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxNominators: u32 = 64;
	pub const MaxControllersInDeprecationBatch: u32 = 5900;
	pub OffchainRepeat: BlockNumber = 5;
	pub HistoryDepth: u32 = 84;
}

/// Upper limit on the number of NPOS nominations.
const MAX_QUOTA_NOMINATIONS: u32 = 16;

pub struct StakingBenchmarkingConfig;
impl pezpallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<5000>;
	type MaxValidators = ConstU32<1000>;
}

impl pezpallet_staking::Config for Runtime {
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = pezsp_staking::currency_to_vote::U128CurrencyToVote;
	type RewardRemainder = ResolveTo<TreasuryAccount, Balances>;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Slash = ResolveTo<TreasuryAccount, Balances>; // send the slashed funds to the treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type AdminOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pezpallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
	>;
	type SessionInterface = Self;
	type EraPayout = pezpallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxExposurePageSize = ConstU32<256>;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = VoterList;
	type NominationsQuota = pezpallet_staking::FixedNominationsQuota<MAX_QUOTA_NOMINATIONS>;
	// This a placeholder, to be introduced in the next PR as an instance of bags-list
	type TargetList = pezpallet_staking::UseValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type HistoryDepth = HistoryDepth;
	type EventListeners = (NominationPools, DelegatedStaking);
	type WeightInfo = pezpallet_staking::weights::BizinikiwiWeight<Runtime>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type Filter = Nothing;
	type MaxValidatorSet = ConstU32<1000>;
}

impl pezpallet_fast_unstake::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ControlOrigin = pezframe_system::EnsureRoot<AccountId>;
	type BatchSize = ConstU32<64>;
	type Deposit = ConstU128<{ DOLLARS }>;
	type Currency = Balances;
	type Staking = Staking;
	type MaxErasToCheckPerBlock = ConstU32<1>;
	type WeightInfo = ();
}
parameter_types! {
	// phase durations. 1/4 of the last session for each.
	pub const SignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
	pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;

	// signed config
	pub const SignedRewardBase: Balance = 1 * DOLLARS;
	pub const SignedFixedDeposit: Balance = 1 * DOLLARS;
	pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);
	pub const SignedDepositByte: Balance = 1 * CENTS;

	// miner configs
	pub const MultiPhaseUnsignedPriority: TransactionPriority = StakingUnsignedPriority::get() - 1u64;
	pub MinerMaxWeight: Weight = RuntimeBlockWeights::get()
		.get(DispatchClass::Normal)
		.max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
		.saturating_sub(BlockExecutionWeight::get());
	// Solution can occupy 90% of normal block size
	pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);
}

pezframe_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = pezsp_runtime::PerU16,
		MaxVoters = MaxElectingVotersSolution,
	>(16)
);

parameter_types! {
	// Note: the EPM in this runtime runs the election on-chain. The election bounds must be
	// carefully set so that an election round fits in one block.
	pub ElectionBoundsMultiPhase: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(10_000.into()).targets_count(1_500.into()).build();
	pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(5_000.into()).targets_count(1_250.into()).build();

	pub MaxNominations: u32 = <NposSolution16 as pezframe_election_provider_support::NposSolution>::LIMIT as u32;
	pub MaxElectingVotersSolution: u32 = 40_000;
	// The maximum winners that can be elected by the Election pezpallet which is equivalent to the
	// maximum active validators the staking pezpallet can have.
	pub MaxActiveValidators: u32 = 1000;
}

/// The numbers configured here could always be more than the the maximum limits of staking
/// pezpallet to ensure election snapshot will not run out of memory. For now, we set them to
/// smaller values since the staking is bounded and the weight pipeline takes hours for this single
/// pezpallet.
pub struct ElectionProviderBenchmarkConfig;
impl pezpallet_election_provider_multi_phase::BenchmarkingConfig
	for ElectionProviderBenchmarkConfig
{
	const VOTERS: [u32; 2] = [1000, 2000];
	const TARGETS: [u32; 2] = [500, 1000];
	const ACTIVE_VOTERS: [u32; 2] = [500, 800];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
	const MINER_MAXIMUM_VOTERS: u32 = 1000;
	const MAXIMUM_TARGETS: u32 = 300;
}

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

/// A source of random balance for NposSolver, which is meant to be run by the OCW election miner.
pub struct OffchainRandomBalancing;
impl Get<Option<BalancingConfig>> for OffchainRandomBalancing {
	fn get() -> Option<BalancingConfig> {
		use pezsp_runtime::traits::TrailingZeroInput;
		let iterations = match MINER_MAX_ITERATIONS {
			0 => 0,
			max => {
				let seed = pezsp_io::offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed")
					% max.saturating_add(1);
				random as usize
			},
		};

		let config = BalancingConfig { iterations, tolerance: 0 };
		Some(config)
	}
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type Sort = ConstBool<true>;
	type System = Runtime;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Runtime>>;
	type DataProvider = Staking;
	type WeightInfo = pezframe_election_provider_support::weights::BizinikiwiWeight<Runtime>;
	type Bounds = ElectionBoundsOnChain;
	type MaxBackersPerWinner = MaxElectingVotersSolution;
	type MaxWinnersPerPage = MaxActiveValidators;
}

impl pezpallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = MinerMaxLength;
	type MaxWeight = MinerMaxWeight;
	type Solution = NposSolution16;
	type MaxVotesPerVoter =
	<<Self as pezpallet_election_provider_multi_phase::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
	type MaxWinners = MaxActiveValidators;
	type MaxBackersPerWinner = MaxElectingVotersSolution;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
			<Self as pezpallet_election_provider_multi_phase::Config>::WeightInfo
			as
			pezpallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
	}
}

impl pezpallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type BetterSignedThreshold = ();
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = MultiPhaseUnsignedPriority;
	type MinerConfig = Self;
	type SignedMaxSubmissions = ConstU32<10>;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase =
		GeometricDepositBase<Balance, SignedFixedDeposit, SignedDepositIncreaseFactor>;
	type SignedDepositByte = SignedDepositByte;
	type SignedMaxRefunds = ConstU32<3>;
	type SignedDepositWeight = ();
	type SignedMaxWeight = MinerMaxWeight;
	type SlashHandler = (); // burn slashes
	type RewardHandler = (); // rewards are minted from the void
	type DataProvider = Staking;
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
	type ForceOrigin = EnsureRootOrHalfCouncil;
	type MaxWinners = MaxActiveValidators;
	type ElectionBounds = ElectionBoundsMultiPhase;
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type WeightInfo = pezpallet_election_provider_multi_phase::weights::BizinikiwiWeight<Self>;
	type MaxBackersPerWinner = MaxElectingVotersSolution;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &voter_bags::THRESHOLDS;
	pub const AutoRebagNumber: u32 = 10;
}

type VoterBagsListInstance = pezpallet_bags_list::Instance1;
impl pezpallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pezpallet_bags_list::weights::BizinikiwiWeight<Runtime>;
	/// The voter bags-list is loosely kept up to date, and the real source of truth for the score
	/// of each node is the staking pezpallet.
	type ScoreProvider = Staking;
	type BagThresholds = BagThresholds;
	type MaxAutoRebagPerBlock = AutoRebagNumber;
	type Score = VoteWeight;
}

parameter_types! {
	pub const DelegatedStakingPalletId: PalletId = PalletId(*b"py/dlstk");
	pub const SlashRewardFraction: Perbill = Perbill::from_percent(1);
}

impl pezpallet_delegated_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = DelegatedStakingPalletId;
	type Currency = Balances;
	type OnSlash = ();
	type SlashRewardFraction = SlashRewardFraction;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CoreStaking = Staking;
}

parameter_types! {
	pub const PostUnbondPoolsWindow: u32 = 4;
	pub const NominationPoolsPalletId: PalletId = PalletId(*b"py/nopls");
	pub const MaxPointsToBalance: u8 = 10;
}

use pezsp_runtime::traits::{Convert, Keccak256};
pub struct BalanceToU256;
impl Convert<Balance, pezsp_core::U256> for BalanceToU256 {
	fn convert(balance: Balance) -> pezsp_core::U256 {
		pezsp_core::U256::from(balance)
	}
}
pub struct U256ToBalance;
impl Convert<pezsp_core::U256, Balance> for U256ToBalance {
	fn convert(n: pezsp_core::U256) -> Balance {
		n.try_into().unwrap_or(Balance::max_value())
	}
}

impl pezpallet_nomination_pools::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = BalanceToU256;
	type U256ToBalance = U256ToBalance;
	type StakeAdapter =
		pezpallet_nomination_pools::adapter::DelegateStake<Self, Staking, DelegatedStaking>;
	type PostUnbondingPoolsWindow = PostUnbondPoolsWindow;
	type MaxMetadataLen = ConstU32<256>;
	type MaxUnbonding = ConstU32<8>;
	type PalletId = NominationPoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type AdminOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pezpallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
	>;
	type BlockNumberProvider = System;
	type Filter = Nothing;
}

parameter_types! {
	pub const VoteLockingPeriod: BlockNumber = 30 * DAYS;
}

impl pezpallet_conviction_voting::Config for Runtime {
	type WeightInfo = pezpallet_conviction_voting::weights::BizinikiwiWeight<Self>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type VoteLockingPeriod = VoteLockingPeriod;
	type MaxVotes = ConstU32<512>;
	type MaxTurnout = pezframe_support::traits::TotalIssuanceOf<Balances, Self::AccountId>;
	type Polls = Referenda;
	type BlockNumberProvider = System;
	type VotingHooks = ();
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 100 * DOLLARS;
	pub const UndecidingTimeout: BlockNumber = 28 * DAYS;
}

pub struct TracksInfo;
impl pezpallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = u16;
	type RuntimeOrigin = <RuntimeOrigin as pezframe_support::traits::OriginTrait>::PalletsOrigin;

	fn tracks(
	) -> impl Iterator<Item = Cow<'static, pezpallet_referenda::Track<Self::Id, Balance, BlockNumber>>>
	{
		dynamic_params::referenda::Tracks::get().into_iter().map(Cow::Owned)
	}
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		dynamic_params::referenda::Origins::get()
			.iter()
			.find(|(o, _)| id == o)
			.map(|(_, track_id)| *track_id)
			.ok_or(())
	}
}

impl pezpallet_referenda::Config for Runtime {
	type WeightInfo = pezpallet_referenda::weights::BizinikiwiWeight<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = pezpallet_balances::Pezpallet<Self>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	type CancelOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Slash = ();
	type Votes = pezpallet_conviction_voting::VotesOf<Runtime>;
	type Tally = pezpallet_conviction_voting::TallyOf<Runtime>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

impl pezpallet_referenda::Config<pezpallet_referenda::Instance2> for Runtime {
	type WeightInfo = pezpallet_referenda::weights::BizinikiwiWeight<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = pezpallet_balances::Pezpallet<Self>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	type CancelOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Slash = ();
	type Votes = pezpallet_ranked_collective::Votes;
	type Tally = pezpallet_ranked_collective::TallyOf<Runtime>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

impl pezpallet_ranked_collective::Config for Runtime {
	type WeightInfo = pezpallet_ranked_collective::weights::BizinikiwiWeight<Self>;
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = Self::DemoteOrigin;
	type PromoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type DemoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type ExchangeOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type Polls = RankedPolls;
	type MinRankOfClass = traits::Identity;
	type VoteWeight = pezpallet_ranked_collective::Geometric;
	type MemberSwappedHandler = (CoreFellowship, Salary);
	type MaxMemberCount = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = (CoreFellowship, Salary);
}

impl pezpallet_remark::Config for Runtime {
	type WeightInfo = pezpallet_remark::weights::BizinikiwiWeight<Self>;
	type RuntimeEvent = RuntimeEvent;
}

impl pezpallet_root_testing::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const VotingPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
	pub const MinimumDeposit: Balance = 100 * DOLLARS;
	pub const EnactmentPeriod: BlockNumber = 30 * 24 * 60 * MINUTES;
	pub const CooloffPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
	pub const MaxProposals: u32 = 100;
}

impl pezpallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin =
		pezpallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin =
		pezpallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pezpallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pezpallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
	type InstantOrigin =
		pezpallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type InstantAllowed = ConstBool<true>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin =
		pezpallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pezpallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pezpallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type CooloffPeriod = CooloffPeriod;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = ConstU32<100>;
	type WeightInfo = pezpallet_democracy::weights::BizinikiwiWeight<Runtime>;
	type MaxProposals = MaxProposals;
	type Preimages = Preimage;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
	pub const ProposalDepositOffset: Balance = ExistentialDeposit::get() + ExistentialDeposit::get();
	pub const ProposalHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Council(pezpallet_collective::HoldReason::ProposalSubmission);
}

type CouncilCollective = pezpallet_collective::Instance1;
impl pezpallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pezpallet_collective::PrimeDefaultVote;
	type WeightInfo = pezpallet_collective::weights::BizinikiwiWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
	type DisapproveOrigin = EnsureRoot<Self::AccountId>;
	type KillOrigin = EnsureRoot<Self::AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		ProposalHoldReason,
		pezpallet_collective::deposit::Delayed<
			ConstU32<2>,
			pezpallet_collective::deposit::Linear<ConstU32<2>, ProposalDepositOffset>,
		>,
		u32,
	>;
}

parameter_types! {
	pub const CandidacyBond: Balance = 10 * DOLLARS;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = deposit(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = deposit(0, 32);
	pub const TermDuration: BlockNumber = 7 * DAYS;
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 7;
	pub const MaxVotesPerVoter: u32 = 16;
	pub const MaxVoters: u32 = 256;
	pub const MaxCandidates: u32 = 128;
	pub const ElectionsPhragmenPalletId: LockIdentifier = *b"phrelect";
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pezpallet_elections_phragmen::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = ElectionsPhragmenPalletId;
	type Currency = Balances;
	type ChangeMembers = Council;
	// NOTE: this implies that council's genesis members cannot be set directly and must come from
	// this module.
	type InitializeMembers = Council;
	type CurrencyToVote = pezsp_staking::currency_to_vote::U128CurrencyToVote;
	type CandidacyBond = CandidacyBond;
	type VotingBondBase = VotingBondBase;
	type VotingBondFactor = VotingBondFactor;
	type LoserCandidate = ();
	type KickedMember = ();
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
	type MaxVoters = MaxVoters;
	type MaxVotesPerVoter = MaxVotesPerVoter;
	type MaxCandidates = MaxCandidates;
	type WeightInfo = pezpallet_elections_phragmen::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 5 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pezpallet_collective::Instance2;
impl pezpallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pezpallet_collective::PrimeDefaultVote;
	type WeightInfo = pezpallet_collective::weights::BizinikiwiWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
	type DisapproveOrigin = EnsureRoot<Self::AccountId>;
	type KillOrigin = EnsureRoot<Self::AccountId>;
	type Consideration = ();
}

type EnsureRootOrHalfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pezpallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;
impl pezpallet_membership::Config<pezpallet_membership::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRootOrHalfCouncil;
	type RemoveOrigin = EnsureRootOrHalfCouncil;
	type SwapOrigin = EnsureRootOrHalfCouncil;
	type ResetOrigin = EnsureRootOrHalfCouncil;
	type PrimeOrigin = EnsureRootOrHalfCouncil;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = TechnicalMaxMembers;
	type WeightInfo = pezpallet_membership::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const DataDepositPerByte: Balance = 1 * CENTS;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaximumReasonLength: u32 = 300;
	pub const MaxApprovals: u32 = 100;
	pub const MaxBalance: Balance = Balance::max_value();
	pub const SpendPayoutPeriod: BlockNumber = 30 * DAYS;
}

impl pezpallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pezpallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
	>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = Bounties;
	type WeightInfo = pezpallet_treasury::weights::BizinikiwiWeight<Runtime>;
	type MaxApprovals = MaxApprovals;
	type SpendOrigin = EnsureWithSuccess<EnsureRoot<AccountId>, AccountId, MaxBalance>;
	type AssetKind = NativeOrWithId<u32>;
	type Beneficiary = AccountId;
	type BeneficiaryLookup = Indices;
	type Paymaster = PayAssetFromAccount<NativeAndAssets, TreasuryAccount>;
	type BalanceConverter = AssetRate;
	type PayoutPeriod = SpendPayoutPeriod;
	type BlockNumberProvider = System;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = PalletTreasuryArguments;
}

impl pezpallet_asset_rate::Config for Runtime {
	type CreateOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type AssetKind = NativeOrWithId<u32>;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pezpallet_asset_rate::weights::BizinikiwiWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetRateArguments;
}

parameter_types! {
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 5 * DOLLARS;
	pub const BountyDepositBase: Balance = 1 * DOLLARS;
	pub const CuratorDepositFromFeeMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = 1 * DOLLARS;
	pub const CuratorDepositMax: Balance = 100 * DOLLARS;
	pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
}

impl pezpallet_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositFromFeeMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = pezpallet_bounties::weights::BizinikiwiWeight<Runtime>;
	type ChildBountyManager = ChildBounties;
	type OnSlash = Treasury;
}

parameter_types! {
	/// Allocate at most 20% of each block for message processing.
	///
	/// Is set to 20% since the scheduler can already consume a maximum of 80%.
	pub MessageQueueServiceWeight: Option<Weight> = Some(Perbill::from_percent(20) * RuntimeBlockWeights::get().max_block);
}

impl pezpallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	/// NOTE: Always set this to `NoopMessageProcessor` for benchmarking.
	type MessageProcessor = pezpallet_message_queue::mock_helpers::NoopMessageProcessor<u32>;
	type Size = u32;
	type QueueChangeHandler = ();
	type QueuePausedQuery = ();
	type HeapSize = ConstU32<{ 64 * 1024 }>;
	type MaxStale = ConstU32<128>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = ();
}

parameter_types! {
	pub const ChildBountyValueMinimum: Balance = 1 * DOLLARS;
	pub const MaxActiveChildBountyCount: u32 = 5;
}

impl pezpallet_child_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = pezpallet_child_bounties::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const CuratorDepositFromValueMultiplier: Permill = Permill::from_percent(10);
}

impl pezpallet_multi_asset_bounties::Config for Runtime {
	type Balance = Balance;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pezpallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
	>;
	type SpendOrigin = EnsureWithSuccess<EnsureRoot<AccountId>, AccountId, MaxBalance>;
	type AssetKind = NativeOrWithId<u32>;
	type Beneficiary = AccountId;
	type BeneficiaryLookup = Indices;
	type BountyValueMinimum = BountyValueMinimum;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type WeightInfo = pezpallet_multi_asset_bounties::weights::BizinikiwiWeight<Runtime>;
	type FundingSource =
		pezpallet_multi_asset_bounties::PalletIdAsFundingSource<TreasuryPalletId, Runtime>;
	type BountySource =
		pezpallet_multi_asset_bounties::BountySourceAccount<TreasuryPalletId, Runtime>;
	type ChildBountySource =
		pezpallet_multi_asset_bounties::ChildBountySourceAccount<TreasuryPalletId, Runtime>;
	type Paymaster = PayWithFungibles<NativeAndAssets, AccountId>;
	type BalanceConverter = AssetRate;
	type Preimages = Preimage;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		ProposalHoldReason,
		pezpallet_multi_asset_bounties::CuratorDepositAmount<
			CuratorDepositFromValueMultiplier,
			CuratorDepositMin,
			CuratorDepositMax,
			Balance,
		>,
		Balance,
	>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = PalletMultiAssetBountiesArguments;
}

impl pezpallet_tips::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type Tippers = Elections;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type MaxTipAmount = ConstU128<{ 500 * DOLLARS }>;
	type WeightInfo = pezpallet_tips::weights::BizinikiwiWeight<Runtime>;
	type OnSlash = Treasury;
}

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerChildTrieItem: Balance = deposit(1, 0) / 100;
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub const DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
	pub Schedule: pezpallet_contracts::Schedule<Runtime> = Default::default();
	pub CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
	pub const MaxEthExtrinsicWeight: FixedU128 = FixedU128::from_rational(9, 10);
}

impl pezpallet_contracts::Config for Runtime {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	/// The safest default is to allow no calls at all.
	///
	/// Runtimes should whitelist dispatchables that are allowed to be called from contracts
	/// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
	/// change because that would break already deployed contracts. The `Call` structure itself
	/// is not allowed to change the indices of existing pallets, too.
	type CallFilter = Nothing;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type DefaultDepositLimit = DefaultDepositLimit;
	type CallStack = [pezpallet_contracts::Frame<Self>; 5];
	type WeightPrice = pezpallet_transaction_payment::Pezpallet<Self>;
	type WeightInfo = pezpallet_contracts::weights::BizinikiwiWeight<Self>;
	type ChainExtension = ();
	type Schedule = Schedule;
	type AddressGenerator = pezpallet_contracts::DefaultAddressGenerator;
	type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
	type MaxStorageKeyLen = ConstU32<128>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = EnsureSigned<Self::AccountId>;
	type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
	type MaxTransientStorageSize = ConstU32<{ 1 * 1024 * 1024 }>;
	type RuntimeHoldReason = RuntimeHoldReason;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Migrations = ();
	#[cfg(feature = "runtime-benchmarks")]
	type Migrations = pezpallet_contracts::migration::codegen::BenchMigrations;
	type MaxDelegateDependencies = ConstU32<32>;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type Debug = ();
	type Environment = ();
	type ApiVersion = ();
	type Xcm = ();
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
	type Precompiles =
		(ERC20<Self, InlineIdConfig<0x1>, Instance1>, ERC20<Self, InlineIdConfig<0x2>, Instance2>);
	type AddressMapper = pezpallet_revive::AccountId32Mapper<Self>;
	type RuntimeMemory = ConstU32<{ 128 * 1024 * 1024 }>;
	type PVFMemory = ConstU32<{ 512 * 1024 * 1024 }>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = EnsureSigned<Self::AccountId>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type ChainId = ConstU64<420_420_420>;
	type NativeToEthRatio = ConstU32<1_000_000>; // 10^(18 - 12) Eth is 10^18, Native is 10^12.
	type FindAuthor = <Runtime as pezpallet_authorship::Config>::FindAuthor;
	type AllowEVMBytecode = ConstBool<true>;
	type FeeInfo = pezpallet_revive::evm::fees::Info<Address, Signature, EthExtraImpl>;
	type MaxEthExtrinsicWeight = MaxEthExtrinsicWeight;
	type DebugEnabled = ConstBool<false>;
}

impl pezpallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pezpallet_sudo::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	/// We prioritize im-online heartbeats over election solution submission.
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
	pub const MaxAuthorities: u32 = 1000;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
}

impl<LocalCall> pezframe_system::offchain::CreateTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type Extension = TxExtension;

	fn create_transaction(call: RuntimeCall, extension: TxExtension) -> UncheckedExtrinsic {
		generic::UncheckedExtrinsic::new_transaction(call, extension).into()
	}
}

impl<LocalCall> pezframe_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_signed_transaction<
		C: pezframe_system::offchain::AppCrypto<Self::Public, Self::Signature>,
	>(
		call: RuntimeCall,
		public: <Signature as traits::Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<UncheckedExtrinsic> {
		let tip = 0;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let tx_ext: TxExtension = (
			pezframe_system::AuthorizeCall::<Runtime>::new(),
			pezframe_system::CheckNonZeroSender::<Runtime>::new(),
			pezframe_system::CheckSpecVersion::<Runtime>::new(),
			pezframe_system::CheckTxVersion::<Runtime>::new(),
			pezframe_system::CheckGenesis::<Runtime>::new(),
			pezframe_system::CheckEra::<Runtime>::from(era),
			pezframe_system::CheckNonce::<Runtime>::from(nonce),
			pezframe_system::CheckWeight::<Runtime>::new(),
			pezpallet_skip_feeless_payment::SkipCheckIfFeeless::from(
				pezpallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<Runtime>::from(
					tip, None,
				),
			),
			pezframe_metadata_hash_extension::CheckMetadataHash::new(false),
			pezpallet_revive::evm::tx_extension::SetOrigin::<Runtime>::default(),
			pezframe_system::WeightReclaim::<Runtime>::new(),
		);

		let raw_payload = SignedPayload::new(call, tx_ext)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, tx_ext, _) = raw_payload.deconstruct();
		let transaction =
			generic::UncheckedExtrinsic::new_signed(call, address, signature, tx_ext).into();
		Some(transaction)
	}
}

impl<LocalCall> pezframe_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		generic::UncheckedExtrinsic::new_bare(call).into()
	}
}

impl pezframe_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> pezframe_system::offchain::CreateTransactionBase<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type RuntimeCall = RuntimeCall;
}

impl<C> pezframe_system::offchain::CreateAuthorizedTransaction<C> for Runtime
where
	RuntimeCall: From<C>,
{
	fn create_extension() -> Self::Extension {
		(
			pezframe_system::AuthorizeCall::<Runtime>::new(),
			pezframe_system::CheckNonZeroSender::<Runtime>::new(),
			pezframe_system::CheckSpecVersion::<Runtime>::new(),
			pezframe_system::CheckTxVersion::<Runtime>::new(),
			pezframe_system::CheckGenesis::<Runtime>::new(),
			pezframe_system::CheckEra::<Runtime>::from(Era::Immortal),
			pezframe_system::CheckNonce::<Runtime>::from(0),
			pezframe_system::CheckWeight::<Runtime>::new(),
			pezpallet_skip_feeless_payment::SkipCheckIfFeeless::from(
				pezpallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<Runtime>::from(
					0, None,
				),
			),
			pezframe_metadata_hash_extension::CheckMetadataHash::new(false),
			pezpallet_revive::evm::tx_extension::SetOrigin::<Runtime>::default(),
			pezframe_system::WeightReclaim::<Runtime>::new(),
		)
	}
}

impl pezpallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type NextSessionRotation = Babe;
	type ValidatorSet = Historical;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pezpallet_im_online::weights::BizinikiwiWeight<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

impl pezpallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pezpallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl pezpallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pezpallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type KeyOwnerProof = pezsp_session::MembershipProof;
	type EquivocationReportSystem =
		pezpallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	// difference of 26 bytes on-chain for the registration and 9 bytes on-chain for the identity
	// information, already accounted for by the byte deposit
	pub const BasicDeposit: Balance = deposit(1, 17);
	pub const ByteDeposit: Balance = deposit(0, 1);
	pub const UsernameDeposit: Balance = deposit(0, 32);
	pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pezpallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type UsernameDeposit = UsernameDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = EnsureRootOrHalfCouncil;
	type RegistrarOrigin = EnsureRootOrHalfCouncil;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as traits::Verify>::Signer;
	type UsernameAuthorityOrigin = EnsureRoot<Self::AccountId>;
	type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
	type UsernameGracePeriod = ConstU32<{ 30 * DAYS }>;
	type MaxSuffixLength = ConstU32<7>;
	type MaxUsernameLength = ConstU32<32>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type WeightInfo = pezpallet_identity::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const ConfigDepositBase: Balance = 5 * DOLLARS;
	pub const FriendDepositFactor: Balance = 50 * CENTS;
	pub const MaxFriends: u16 = 9;
	pub const RecoveryDeposit: Balance = 5 * DOLLARS;
}

impl pezpallet_recovery::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pezpallet_recovery::weights::BizinikiwiWeight<Runtime>;
	type RuntimeCall = RuntimeCall;
	type BlockNumberProvider = System;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
}

parameter_types! {
	pub const GraceStrikes: u32 = 10;
	pub const SocietyVotingPeriod: BlockNumber = 80 * HOURS;
	pub const ClaimPeriod: BlockNumber = 80 * HOURS;
	pub const PeriodSpend: Balance = 500 * DOLLARS;
	pub const MaxLockDuration: BlockNumber = 36 * 30 * DAYS;
	pub const ChallengePeriod: BlockNumber = 7 * DAYS;
	pub const MaxPayouts: u32 = 10;
	pub const MaxBids: u32 = 10;
	pub const SocietyPalletId: PalletId = PalletId(*b"py/socie");
}

impl pezpallet_society::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = SocietyPalletId;
	type Currency = Balances;
	type Randomness = RandomnessCollectiveFlip;
	type GraceStrikes = GraceStrikes;
	type PeriodSpend = PeriodSpend;
	type VotingPeriod = SocietyVotingPeriod;
	type ClaimPeriod = ClaimPeriod;
	type MaxLockDuration = MaxLockDuration;
	type FounderSetOrigin =
		pezpallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>;
	type ChallengePeriod = ChallengePeriod;
	type MaxPayouts = MaxPayouts;
	type MaxBids = MaxBids;
	type BlockNumberProvider = System;
	type WeightInfo = pezpallet_society::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * DOLLARS;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pezpallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = pezpallet_vesting::weights::BizinikiwiWeight<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	// `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
	// highest number of schedules that encodes less than 2^10.
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

impl pezpallet_mmr::Config for Runtime {
	const INDEXING_PREFIX: &'static [u8] = b"mmr";
	type Hashing = Keccak256;
	type LeafData = pezpallet_mmr::ParentNumberAndHash<Self>;
	type OnNewRoot = pezpallet_beefy_mmr::DepositBeefyDigest<Runtime>;
	type BlockHashProvider = pezpallet_mmr::DefaultBlockHashProvider<Runtime>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(0, 0);
}

impl pezpallet_beefy_mmr::Config for Runtime {
	type LeafVersion = LeafVersion;
	type BeefyAuthorityToMerkleLeaf = pezpallet_beefy_mmr::BeefyEcdsaToEthereum;
	type LeafExtra = Vec<u8>;
	type BeefyDataProvider = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const LotteryPalletId: PalletId = PalletId(*b"py/lotto");
	pub const MaxCalls: u32 = 10;
	pub const MaxGenerateRandom: u32 = 10;
}

impl pezpallet_lottery::Config for Runtime {
	type PalletId = LotteryPalletId;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type Randomness = RandomnessCollectiveFlip;
	type RuntimeEvent = RuntimeEvent;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type MaxCalls = MaxCalls;
	type ValidateCall = Lottery;
	type MaxGenerateRandom = MaxGenerateRandom;
	type WeightInfo = pezpallet_lottery::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const AssetDeposit: Balance = 100 * DOLLARS;
	pub const ApprovalDeposit: Balance = 1 * DOLLARS;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10 * DOLLARS;
	pub const MetadataDepositPerByte: Balance = 1 * DOLLARS;
}

impl pezpallet_assets::Config<Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type ReserveData = ();
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = pezpallet_assets::weights::BizinikiwiWeight<Runtime>;
	type RemoveItemsLimit = ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

ord_parameter_types! {
	pub const AssetConversionOrigin: AccountId = AccountIdConversion::<AccountId>::into_account_truncating(&AssetConversionPalletId::get());
}

impl pezpallet_assets::Config<Instance2> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type ReserveData = ();
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSignedBy<AssetConversionOrigin, AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pezpallet_assets::weights::BizinikiwiWeight<Runtime>;
	type RemoveItemsLimit = ConstU32<1000>;
	type CallbackHandle = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
	pub const PoolSetupFee: Balance = 1 * DOLLARS; // should be more or equal to the existential deposit
	pub const MintMinLiquidity: Balance = 100;  // 100 is good enough when the main currency has 10-12 decimals.
	pub const LiquidityWithdrawalFee: Permill = Permill::from_percent(0);
	pub const Native: NativeOrWithId<u32> = NativeOrWithId::Native;
}

pub type NativeAndAssets =
	UnionOf<Balances, Assets, NativeFromLeft, NativeOrWithId<u32>, AccountId>;

impl pezpallet_asset_conversion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type HigherPrecisionBalance = pezsp_core::U256;
	type AssetKind = NativeOrWithId<u32>;
	type Assets = NativeAndAssets;
	type PoolId = (Self::AssetKind, Self::AssetKind);
	type PoolLocator = Chain<
		WithFirstAsset<
			Native,
			AccountId,
			NativeOrWithId<u32>,
			AccountIdConverter<AssetConversionPalletId, Self::PoolId>,
		>,
		Ascending<
			AccountId,
			NativeOrWithId<u32>,
			AccountIdConverter<AssetConversionPalletId, Self::PoolId>,
		>,
	>;
	type PoolAssetId = <Self as pezpallet_assets::Config<Instance2>>::AssetId;
	type PoolAssets = PoolAssets;
	type PoolSetupFee = PoolSetupFee;
	type PoolSetupFeeAsset = Native;
	type PoolSetupFeeTarget = ResolveAssetTo<AssetConversionOrigin, Self::Assets>;
	type PalletId = AssetConversionPalletId;
	type LPFee = ConstU32<3>; // means 0.3%
	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
	type WeightInfo = pezpallet_asset_conversion::weights::BizinikiwiWeight<Runtime>;
	type MaxSwapPathLength = ConstU32<4>;
	type MintMinLiquidity = MintMinLiquidity;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

pub type NativeAndAssetsFreezer =
	UnionOf<Balances, AssetsFreezer, NativeFromLeft, NativeOrWithId<u32>, AccountId>;

/// Benchmark Helper
#[cfg(feature = "runtime-benchmarks")]
pub struct AssetRewardsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_asset_rewards::benchmarking::BenchmarkHelper<NativeOrWithId<u32>>
	for AssetRewardsBenchmarkHelper
{
	fn staked_asset() -> NativeOrWithId<u32> {
		NativeOrWithId::<u32>::WithId(100)
	}
	fn reward_asset() -> NativeOrWithId<u32> {
		NativeOrWithId::<u32>::WithId(101)
	}
}

parameter_types! {
	pub const StakingRewardsPalletId: PalletId = PalletId(*b"py/stkrd");
	pub const CreationHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::AssetRewards(pezpallet_asset_rewards::HoldReason::PoolCreation);
	// 1 item, 135 bytes into the storage on pool creation.
	pub const StakePoolCreationDeposit: Balance = deposit(1, 135);
}

impl pezpallet_asset_rewards::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type AssetId = NativeOrWithId<u32>;
	type Balance = Balance;
	type Assets = NativeAndAssets;
	type PalletId = StakingRewardsPalletId;
	type CreatePoolOrigin = EnsureSigned<AccountId>;
	type WeightInfo = ();
	type AssetsFreezer = NativeAndAssetsFreezer;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		CreationHoldReason,
		ConstantStoragePrice<StakePoolCreationDeposit, Balance>,
	>;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetRewardsBenchmarkHelper;
}

impl pezpallet_asset_conversion_ops::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PriorAccountIdConverter = pezpallet_asset_conversion::AccountIdConverterNoSeed<(
		NativeOrWithId<u32>,
		NativeOrWithId<u32>,
	)>;
	type AssetsRefund = <Runtime as pezpallet_asset_conversion::Config>::Assets;
	type PoolAssetsRefund = <Runtime as pezpallet_asset_conversion::Config>::PoolAssets;
	type PoolAssetsTeam = <Runtime as pezpallet_asset_conversion::Config>::PoolAssets;
	type DepositAsset = Balances;
	type WeightInfo = pezpallet_asset_conversion_ops::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const QueueCount: u32 = 300;
	pub const MaxQueueLen: u32 = 1000;
	pub const FifoQueueLen: u32 = 500;
	pub const NisBasePeriod: BlockNumber = 30 * DAYS;
	pub const MinBid: Balance = 100 * DOLLARS;
	pub const MinReceipt: Perquintill = Perquintill::from_percent(1);
	pub const IntakePeriod: BlockNumber = 10;
	pub MaxIntakeWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 10;
	pub const ThawThrottle: (Perquintill, BlockNumber) = (Perquintill::from_percent(25), 5);
	pub Target: Perquintill = Perquintill::zero();
	pub const NisPalletId: PalletId = PalletId(*b"py/nis  ");
}

impl pezpallet_nis::Config for Runtime {
	type WeightInfo = pezpallet_nis::weights::BizinikiwiWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type FundOrigin = pezframe_system::EnsureSigned<AccountId>;
	type Counterpart = ItemOf<Assets, ConstU32<9u32>, AccountId>;
	type CounterpartAmount = WithMaximumOf<ConstU128<21_000_000_000_000_000_000u128>>;
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
	type BenchmarkSetup = SetupAsset;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct SetupAsset;
#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_nis::BenchmarkSetup for SetupAsset {
	fn create_counterpart_asset() {
		let owner = AccountId::from([0u8; 32]);
		// this may or may not fail depending on if the chain spec or runtime genesis is used.
		let _ = Assets::force_create(
			RuntimeOrigin::root(),
			9u32.into(),
			pezsp_runtime::MultiAddress::Id(owner),
			true,
			1,
		);
	}
}

parameter_types! {
	pub const CollectionDeposit: Balance = 100 * DOLLARS;
	pub const ItemDeposit: Balance = 1 * DOLLARS;
	pub const ApprovalsLimit: u32 = 20;
	pub const ItemAttributesApprovalsLimit: u32 = 20;
	pub const MaxTips: u32 = 10;
	pub const MaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;
}

impl pezpallet_uniques::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type AttributeDepositBase = MetadataDepositBase;
	type DepositPerByte = MetadataDepositPerByte;
	type StringLimit = ConstU32<128>;
	type KeyLimit = ConstU32<32>;
	type ValueLimit = ConstU32<64>;
	type WeightInfo = pezpallet_uniques::weights::BizinikiwiWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
}

parameter_types! {
	pub const Budget: Balance = 10_000 * DOLLARS;
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

pub struct SalaryForRank;
impl GetSalary<u16, AccountId, Balance> for SalaryForRank {
	fn get_salary(a: u16, _: &AccountId) -> Balance {
		Balance::from(a) * 1000 * DOLLARS
	}
}

impl pezpallet_salary::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type Members = RankedCollective;
	type Salary = SalaryForRank;
	type RegistrationPeriod = ConstU32<200>;
	type PayoutPeriod = ConstU32<200>;
	type Budget = Budget;
}

impl pezpallet_core_fellowship::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Members = RankedCollective;
	type Balance = Balance;
	type ParamsOrigin = pezframe_system::EnsureRoot<AccountId>;
	type InductOrigin = pezpallet_core_fellowship::EnsureInducted<Runtime, (), 1>;
	type ApproveOrigin = EnsureRootWithSuccess<AccountId, ConstU16<9>>;
	type PromoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<9>>;
	type FastPromoteOrigin = Self::PromoteOrigin;
	type EvidenceSize = ConstU32<16_384>;
	type MaxRank = ConstU16<9>;
}

parameter_types! {
	pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
	pub NewAssetSymbol: BoundedVec<u8, StringLimit> = (*b"FRAC").to_vec().try_into().unwrap();
	pub NewAssetName: BoundedVec<u8, StringLimit> = (*b"Frac").to_vec().try_into().unwrap();
}

impl pezpallet_nft_fractionalization::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Deposit = AssetDeposit;
	type Currency = Balances;
	type NewAssetSymbol = NewAssetSymbol;
	type NewAssetName = NewAssetName;
	type StringLimit = StringLimit;
	type NftCollectionId = <Self as pezpallet_nfts::Config>::CollectionId;
	type NftId = <Self as pezpallet_nfts::Config>::ItemId;
	type AssetBalance = <Self as pezpallet_balances::Config>::Balance;
	type AssetId = <Self as pezpallet_assets::Config<Instance1>>::AssetId;
	type Assets = Assets;
	type Nfts = Nfts;
	type PalletId = NftFractionalizationPalletId;
	type WeightInfo = pezpallet_nft_fractionalization::weights::BizinikiwiWeight<Runtime>;
	type RuntimeHoldReason = RuntimeHoldReason;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub Features: PalletFeatures = PalletFeatures::all_enabled();
	pub const MaxAttributesPerCall: u32 = 10;
}

impl pezpallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type AttributeDepositBase = MetadataDepositBase;
	type DepositPerByte = MetadataDepositPerByte;
	type StringLimit = ConstU32<256>;
	type KeyLimit = ConstU32<64>;
	type ValueLimit = ConstU32<256>;
	type ApprovalsLimit = ApprovalsLimit;
	type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
	type MaxTips = MaxTips;
	type MaxDeadlineDuration = MaxDeadlineDuration;
	type MaxAttributesPerCall = MaxAttributesPerCall;
	type Features = Features;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as traits::Verify>::Signer;
	type WeightInfo = pezpallet_nfts::weights::BizinikiwiWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Locker = ();
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

impl pezpallet_transaction_storage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeCall = RuntimeCall;
	type FeeDestination = ();
	type WeightInfo = pezpallet_transaction_storage::weights::BizinikiwiWeight<Runtime>;
	type MaxBlockTransactions =
		ConstU32<{ pezpallet_transaction_storage::DEFAULT_MAX_BLOCK_TRANSACTIONS }>;
	type MaxTransactionSize =
		ConstU32<{ pezpallet_transaction_storage::DEFAULT_MAX_TRANSACTION_SIZE }>;
}

impl pezpallet_verify_signature::Config for Runtime {
	type Signature = MultiSignature;
	type AccountIdentifier = MultiSigner;
	type WeightInfo = pezpallet_verify_signature::weights::BizinikiwiWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pezpallet_whitelist::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WhitelistOrigin = EnsureRoot<AccountId>;
	type DispatchWhitelistedOrigin = EnsureRoot<AccountId>;
	type Preimages = Preimage;
	type WeightInfo = pezpallet_whitelist::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const MigrationSignedDepositPerItem: Balance = 1 * CENTS;
	pub const MigrationSignedDepositBase: Balance = 20 * DOLLARS;
	pub const MigrationMaxKeyLen: u32 = 512;
}

impl pezpallet_state_trie_migration::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ControlOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type MaxKeyLen = MigrationMaxKeyLen;
	type SignedDepositPerItem = MigrationSignedDepositPerItem;
	type SignedDepositBase = MigrationSignedDepositBase;
	// Warning: this is not advised, as it might allow the chain to be temporarily DOS-ed.
	// Preferably, if the chain's governance/maintenance team is planning on using a specific
	// account for the migration, put it here to make sure only that account can trigger the signed
	// migrations.
	type SignedFilter = EnsureSigned<Self::AccountId>;
	type WeightInfo = ();
}

const ALLIANCE_MOTION_DURATION_IN_BLOCKS: BlockNumber = 5 * DAYS;

parameter_types! {
	pub const AllianceMotionDuration: BlockNumber = ALLIANCE_MOTION_DURATION_IN_BLOCKS;
	pub const AllianceMaxProposals: u32 = 100;
	pub const AllianceMaxMembers: u32 = 100;
}

type AllianceCollective = pezpallet_collective::Instance3;
impl pezpallet_collective::Config<AllianceCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = AllianceMotionDuration;
	type MaxProposals = AllianceMaxProposals;
	type MaxMembers = AllianceMaxMembers;
	type DefaultVote = pezpallet_collective::PrimeDefaultVote;
	type WeightInfo = pezpallet_collective::weights::BizinikiwiWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
	type DisapproveOrigin = EnsureRoot<Self::AccountId>;
	type KillOrigin = EnsureRoot<Self::AccountId>;
	type Consideration = ();
}

parameter_types! {
	pub const MaxFellows: u32 = AllianceMaxMembers::get();
	pub const MaxAllies: u32 = 100;
	pub const AllyDeposit: Balance = 10 * DOLLARS;
	pub const RetirementPeriod: BlockNumber = ALLIANCE_MOTION_DURATION_IN_BLOCKS + (1 * DAYS);
}

impl pezpallet_alliance::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Proposal = RuntimeCall;
	type AdminOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pezpallet_collective::EnsureProportionMoreThan<AccountId, AllianceCollective, 2, 3>,
	>;
	type MembershipManager = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pezpallet_collective::EnsureProportionMoreThan<AccountId, AllianceCollective, 2, 3>,
	>;
	type AnnouncementOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pezpallet_collective::EnsureProportionMoreThan<AccountId, AllianceCollective, 2, 3>,
	>;
	type Currency = Balances;
	type Slashed = Treasury;
	type InitializeMembers = AllianceMotion;
	type MembershipChanged = AllianceMotion;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type IdentityVerifier = AllianceIdentityVerifier;
	#[cfg(feature = "runtime-benchmarks")]
	type IdentityVerifier = ();
	type ProposalProvider = AllianceProposalProvider;
	type MaxProposals = AllianceMaxProposals;
	type MaxFellows = MaxFellows;
	type MaxAllies = MaxAllies;
	type MaxUnscrupulousItems = ConstU32<100>;
	type MaxWebsiteUrlLength = ConstU32<255>;
	type MaxAnnouncementsCount = ConstU32<100>;
	type MaxMembersCount = AllianceMaxMembers;
	type AllyDeposit = AllyDeposit;
	type WeightInfo = pezpallet_alliance::weights::BizinikiwiWeight<Runtime>;
	type RetirementPeriod = RetirementPeriod;
}

impl pezframe_benchmarking_pezpallet_pov::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
	pub StatementCost: Balance = 1 * DOLLARS;
	pub StatementByteCost: Balance = 100 * MILLICENTS;
	pub const MinAllowedStatements: u32 = 4;
	pub const MaxAllowedStatements: u32 = 10;
	pub const MinAllowedBytes: u32 = 1024;
	pub const MaxAllowedBytes: u32 = 4096;
}

impl pezpallet_statement::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type StatementCost = StatementCost;
	type ByteCost = StatementByteCost;
	type MinAllowedStatements = MinAllowedStatements;
	type MaxAllowedStatements = MaxAllowedStatements;
	type MinAllowedBytes = MinAllowedBytes;
	type MaxAllowedBytes = MaxAllowedBytes;
}

parameter_types! {
	pub MbmServiceWeight: Weight = Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
}

impl pezpallet_migrations::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Migrations = ();
	// Benchmarks need mocked migrations to guarantee that they succeed.
	#[cfg(feature = "runtime-benchmarks")]
	type Migrations = pezpallet_migrations::mock_helpers::MockedMigrations;
	type CursorMaxLen = ConstU32<65_536>;
	type IdentifierMaxLen = ConstU32<256>;
	type MigrationStatusHandler = ();
	type FailedMigrationHandler = pezframe_support::migrations::FreezeChainOnFailedMigration;
	type MaxServiceWeight = MbmServiceWeight;
	type WeightInfo = pezpallet_migrations::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const BrokerPalletId: PalletId = PalletId(*b"py/broke");
	pub const MinimumCreditPurchase: Balance =  100 * MILLICENTS;
}

pub struct IntoAuthor;
impl OnUnbalanced<Credit<AccountId, Balances>> for IntoAuthor {
	fn on_nonzero_unbalanced(credit: Credit<AccountId, Balances>) {
		if let Some(author) = Authorship::author() {
			let _ = <Balances as Balanced<_>>::resolve(&author, credit);
		}
	}
}

pub struct CoretimeProvider;
impl CoretimeInterface for CoretimeProvider {
	type AccountId = AccountId;
	type Balance = Balance;
	type RelayChainBlockNumberProvider = System;
	fn request_core_count(_count: CoreIndex) {}
	fn request_revenue_info_at(_when: u32) {}
	fn credit_account(_who: Self::AccountId, _amount: Self::Balance) {}
	fn assign_core(
		_core: CoreIndex,
		_begin: u32,
		_assignment: Vec<(CoreAssignment, PartsOf57600)>,
		_end_hint: Option<u32>,
	) {
	}
}

pub struct SovereignAccountOf;
// Dummy implementation which converts `TaskId` to `AccountId`.
impl MaybeConvert<TaskId, AccountId> for SovereignAccountOf {
	fn maybe_convert(task: TaskId) -> Option<AccountId> {
		let mut account: [u8; 32] = [0; 32];
		account[..4].copy_from_slice(&task.to_le_bytes());
		Some(account.into())
	}
}
impl pezpallet_broker::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnRevenue = IntoAuthor;
	type TimeslicePeriod = ConstU32<2>;
	type MaxLeasedCores = ConstU32<5>;
	type MaxReservedCores = ConstU32<5>;
	type Coretime = CoretimeProvider;
	type ConvertBalance = traits::Identity;
	type WeightInfo = ();
	type PalletId = BrokerPalletId;
	type AdminOrigin = EnsureRoot<AccountId>;
	type SovereignAccountOf = SovereignAccountOf;
	type MaxAutoRenewals = ConstU32<10>;
	type PriceAdapter = pezpallet_broker::CenterTargetPrice<Balance>;
	type MinimumCreditPurchase = MinimumCreditPurchase;
}

parameter_types! {
	pub const MixnetNumCoverToCurrentBlocks: BlockNumber = 3;
	pub const MixnetNumRequestsToCurrentBlocks: BlockNumber = 3;
	pub const MixnetNumCoverToPrevBlocks: BlockNumber = 3;
	pub const MixnetNumRegisterStartSlackBlocks: BlockNumber = 3;
	pub const MixnetNumRegisterEndSlackBlocks: BlockNumber = 3;
	pub const MixnetRegistrationPriority: TransactionPriority = ImOnlineUnsignedPriority::get() - 1;
}

impl pezpallet_mixnet::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
	type MaxExternalAddressSize = ConstU32<128>;
	type MaxExternalAddressesPerMixnode = ConstU32<16>;
	type NextSessionRotation = Babe;
	type NumCoverToCurrentBlocks = MixnetNumCoverToCurrentBlocks;
	type NumRequestsToCurrentBlocks = MixnetNumRequestsToCurrentBlocks;
	type NumCoverToPrevBlocks = MixnetNumCoverToPrevBlocks;
	type NumRegisterStartSlackBlocks = MixnetNumRegisterStartSlackBlocks;
	type NumRegisterEndSlackBlocks = MixnetNumRegisterEndSlackBlocks;
	type RegistrationPriority = MixnetRegistrationPriority;
	type MinMixnodes = ConstU32<7>; // Low to allow small testing networks
}

/// Dynamic parameters that can be changed at runtime through the
/// `pezpallet_parameters::set_parameter`.
#[dynamic_params(RuntimeParameters, pezpallet_parameters::Parameters::<Runtime>)]
pub mod dynamic_params {
	use super::*;

	#[dynamic_pallet_params]
	#[codec(index = 0)]
	pub mod storage {
		/// Configures the base deposit of storing some data.
		#[codec(index = 0)]
		pub static BaseDeposit: Balance = 1 * DOLLARS;

		/// Configures the per-byte deposit of storing some data.
		#[codec(index = 1)]
		pub static ByteDeposit: Balance = 1 * CENTS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 1)]
	pub mod referenda {
		/// The configuration for the tracks
		#[codec(index = 0)]
		pub static Tracks: BoundedVec<
			pezpallet_referenda::Track<u16, Balance, BlockNumber>,
			ConstU32<100>,
		> = BoundedVec::truncate_from(vec![pezpallet_referenda::Track {
			id: 0u16,
			info: pezpallet_referenda::TrackInfo {
				name: s("root"),
				max_deciding: 1,
				decision_deposit: 10,
				prepare_period: 4,
				decision_period: 4,
				confirm_period: 2,
				min_enactment_period: 4,
				min_approval: pezpallet_referenda::Curve::LinearDecreasing {
					length: Perbill::from_percent(100),
					floor: Perbill::from_percent(50),
					ceil: Perbill::from_percent(100),
				},
				min_support: pezpallet_referenda::Curve::LinearDecreasing {
					length: Perbill::from_percent(100),
					floor: Perbill::from_percent(0),
					ceil: Perbill::from_percent(100),
				},
			},
		}]);

		/// A list mapping every origin with a track Id
		#[codec(index = 1)]
		pub static Origins: BoundedVec<(OriginCaller, u16), ConstU32<100>> =
			BoundedVec::truncate_from(vec![(
				OriginCaller::system(pezframe_system::RawOrigin::Root),
				0,
			)]);
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
	fn default() -> Self {
		RuntimeParameters::Storage(dynamic_params::storage::Parameters::BaseDeposit(
			dynamic_params::storage::BaseDeposit,
			Some(1 * DOLLARS),
		))
	}
}

pub struct DynamicParametersManagerOrigin;
impl EnsureOriginWithArg<RuntimeOrigin, RuntimeParametersKey> for DynamicParametersManagerOrigin {
	type Success = ();

	fn try_origin(
		origin: RuntimeOrigin,
		key: &RuntimeParametersKey,
	) -> Result<Self::Success, RuntimeOrigin> {
		match key {
			RuntimeParametersKey::Storage(_) => {
				pezframe_system::ensure_root(origin.clone()).map_err(|_| origin)?;
				return Ok(());
			},
			RuntimeParametersKey::Referenda(_) => {
				pezframe_system::ensure_root(origin.clone()).map_err(|_| origin)?;
				return Ok(());
			},
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_key: &RuntimeParametersKey) -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

impl pezpallet_parameters::Config for Runtime {
	type RuntimeParameters = RuntimeParameters;
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = DynamicParametersManagerOrigin;
	type WeightInfo = ();
}

pub type MetaTxExtension = (
	pezpallet_verify_signature::VerifySignature<Runtime>,
	pezpallet_meta_tx::MetaTxMarker<Runtime>,
	pezframe_system::CheckNonZeroSender<Runtime>,
	pezframe_system::CheckSpecVersion<Runtime>,
	pezframe_system::CheckTxVersion<Runtime>,
	pezframe_system::CheckGenesis<Runtime>,
	pezframe_system::CheckEra<Runtime>,
	pezframe_system::CheckNonce<Runtime>,
	pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

impl pezpallet_meta_tx::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Extension = MetaTxExtension;
	#[cfg(feature = "runtime-benchmarks")]
	type Extension = pezpallet_meta_tx::WeightlessExtension<Runtime>;
}

#[pezframe_support::runtime]
mod runtime {
	use super::*;

	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Runtime;

	#[runtime::pezpallet_index(0)]
	pub type System = pezframe_system::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(1)]
	pub type Utility = pezpallet_utility::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(2)]
	pub type Babe = pezpallet_babe::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(3)]
	pub type Timestamp = pezpallet_timestamp::Pezpallet<Runtime>;

	// Authorship must be before session in order to note author in the correct session and era
	// for im-online and staking.
	#[runtime::pezpallet_index(4)]
	pub type Authorship = pezpallet_authorship::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(5)]
	pub type Indices = pezpallet_indices::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(6)]
	pub type Balances = pezpallet_balances::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(7)]
	pub type TransactionPayment = pezpallet_transaction_payment::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(9)]
	pub type AssetConversionTxPayment = pezpallet_asset_conversion_tx_payment::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(10)]
	pub type ElectionProviderMultiPhase =
		pezpallet_election_provider_multi_phase::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(11)]
	pub type Staking = pezpallet_staking::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(12)]
	pub type Session = pezpallet_session::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(13)]
	pub type Democracy = pezpallet_democracy::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(14)]
	pub type Council = pezpallet_collective::Pezpallet<Runtime, Instance1>;

	#[runtime::pezpallet_index(15)]
	pub type TechnicalCommittee = pezpallet_collective::Pezpallet<Runtime, Instance2>;

	#[runtime::pezpallet_index(16)]
	pub type Elections = pezpallet_elections_phragmen::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(17)]
	pub type TechnicalMembership = pezpallet_membership::Pezpallet<Runtime, Instance1>;

	#[runtime::pezpallet_index(18)]
	pub type Grandpa = pezpallet_grandpa::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(19)]
	pub type Treasury = pezpallet_treasury::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(20)]
	pub type AssetRate = pezpallet_asset_rate::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(21)]
	pub type Contracts = pezpallet_contracts::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(22)]
	pub type Sudo = pezpallet_sudo::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(23)]
	pub type ImOnline = pezpallet_im_online::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(24)]
	pub type AuthorityDiscovery = pezpallet_authority_discovery::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(25)]
	pub type Offences = pezpallet_offences::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(26)]
	pub type Historical = pezpallet_session_historical::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(27)]
	pub type RandomnessCollectiveFlip =
		pezpallet_insecure_randomness_collective_flip::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(28)]
	pub type Identity = pezpallet_identity::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(29)]
	pub type Society = pezpallet_society::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(30)]
	pub type Recovery = pezpallet_recovery::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(31)]
	pub type Vesting = pezpallet_vesting::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(32)]
	pub type Scheduler = pezpallet_scheduler::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(33)]
	pub type Glutton = pezpallet_glutton::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(34)]
	pub type Preimage = pezpallet_preimage::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(35)]
	pub type Proxy = pezpallet_proxy::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(36)]
	pub type Multisig = pezpallet_multisig::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(37)]
	pub type Bounties = pezpallet_bounties::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(38)]
	pub type Tips = pezpallet_tips::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(39)]
	pub type Assets = pezpallet_assets::Pezpallet<Runtime, Instance1>;

	#[runtime::pezpallet_index(40)]
	pub type PoolAssets = pezpallet_assets::Pezpallet<Runtime, Instance2>;

	#[runtime::pezpallet_index(41)]
	pub type Beefy = pezpallet_beefy::Pezpallet<Runtime>;

	// MMR leaf construction must be after session in order to have a leaf's next_auth_set
	// refer to block<N>. See issue pezkuwi-fellows/runtimes#160 for details.
	#[runtime::pezpallet_index(42)]
	pub type Mmr = pezpallet_mmr::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(43)]
	pub type MmrLeaf = pezpallet_beefy_mmr::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(44)]
	pub type Lottery = pezpallet_lottery::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(45)]
	pub type Nis = pezpallet_nis::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(46)]
	pub type Uniques = pezpallet_uniques::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(47)]
	pub type Nfts = pezpallet_nfts::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(48)]
	pub type NftFractionalization = pezpallet_nft_fractionalization::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(49)]
	pub type Salary = pezpallet_salary::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(50)]
	pub type CoreFellowship = pezpallet_core_fellowship::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(51)]
	pub type TransactionStorage = pezpallet_transaction_storage::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(52)]
	pub type VoterList = pezpallet_bags_list::Pezpallet<Runtime, Instance1>;

	#[runtime::pezpallet_index(53)]
	pub type StateTrieMigration = pezpallet_state_trie_migration::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(54)]
	pub type ChildBounties = pezpallet_child_bounties::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(55)]
	pub type Referenda = pezpallet_referenda::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(56)]
	pub type Remark = pezpallet_remark::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(57)]
	pub type RootTesting = pezpallet_root_testing::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(58)]
	pub type ConvictionVoting = pezpallet_conviction_voting::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(59)]
	pub type Whitelist = pezpallet_whitelist::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(60)]
	pub type AllianceMotion = pezpallet_collective::Pezpallet<Runtime, Instance3>;

	#[runtime::pezpallet_index(61)]
	pub type Alliance = pezpallet_alliance::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(62)]
	pub type NominationPools = pezpallet_nomination_pools::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(63)]
	pub type RankedPolls = pezpallet_referenda::Pezpallet<Runtime, Instance2>;

	#[runtime::pezpallet_index(64)]
	pub type RankedCollective = pezpallet_ranked_collective::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(65)]
	pub type AssetConversion = pezpallet_asset_conversion::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(66)]
	pub type FastUnstake = pezpallet_fast_unstake::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(67)]
	pub type MessageQueue = pezpallet_message_queue::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(68)]
	pub type Pov = pezframe_benchmarking_pezpallet_pov::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(69)]
	pub type TxPause = pezpallet_tx_pause::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(70)]
	pub type SafeMode = pezpallet_safe_mode::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(71)]
	pub type Statement = pezpallet_statement::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(72)]
	pub type MultiBlockMigrations = pezpallet_migrations::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(73)]
	pub type Broker = pezpallet_broker::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(74)]
	pub type TasksExample = pezpallet_example_tasks::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(75)]
	pub type Mixnet = pezpallet_mixnet::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(76)]
	pub type Parameters = pezpallet_parameters::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(77)]
	pub type SkipFeelessPayment = pezpallet_skip_feeless_payment::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(78)]
	pub type PalletExampleMbms = pezpallet_example_mbm::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(79)]
	pub type AssetConversionMigration = pezpallet_asset_conversion_ops::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(80)]
	pub type Revive = pezpallet_revive::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(81)]
	pub type VerifySignature = pezpallet_verify_signature::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(82)]
	pub type DelegatedStaking = pezpallet_delegated_staking::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(83)]
	pub type AssetRewards = pezpallet_asset_rewards::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(84)]
	pub type AssetsFreezer = pezpallet_assets_freezer::Pezpallet<Runtime, Instance1>;

	#[runtime::pezpallet_index(85)]
	pub type Oracle = pezpallet_oracle::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(89)]
	pub type MetaTx = pezpallet_meta_tx::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(90)]
	pub type MultiAssetBounties = pezpallet_multi_asset_bounties::Pezpallet<Runtime>;
}

/// The address format for describing accounts.
pub type Address = pezsp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The TransactionExtension to the basic transaction logic.
///
/// When you change this, you **MUST** modify [`sign`] in `bin/node/testing/src/keyring.rs`!
///
/// [`sign`]: <../../testing/src/keyring.rs.html>
pub type TxExtension = (
	pezframe_system::AuthorizeCall<Runtime>,
	pezframe_system::CheckNonZeroSender<Runtime>,
	pezframe_system::CheckSpecVersion<Runtime>,
	pezframe_system::CheckTxVersion<Runtime>,
	pezframe_system::CheckGenesis<Runtime>,
	pezframe_system::CheckEra<Runtime>,
	pezframe_system::CheckNonce<Runtime>,
	pezframe_system::CheckWeight<Runtime>,
	pezpallet_skip_feeless_payment::SkipCheckIfFeeless<
		Runtime,
		pezpallet_asset_conversion_tx_payment::ChargeAssetTxPayment<Runtime>,
	>,
	pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
	pezpallet_revive::evm::tx_extension::SetOrigin<Runtime>,
	pezframe_system::WeightReclaim<Runtime>,
);

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
			pezframe_system::CheckEra::from(crate::generic::Era::Immortal),
			pezframe_system::CheckNonce::<Runtime>::from(nonce),
			pezframe_system::CheckWeight::<Runtime>::new(),
			pezpallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<Runtime>::from(tip, None)
				.into(),
			pezframe_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
			pezpallet_revive::evm::tx_extension::SetOrigin::<Runtime>::new_from_eth_transaction(),
			pezframe_system::WeightReclaim::<Runtime>::new(),
		)
	}
}

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	pezpallet_revive::evm::runtime::UncheckedExtrinsic<Address, Signature, EthExtraImpl>;
/// Unchecked signature payload type as expected by this runtime.
pub type UncheckedSignaturePayload =
	generic::UncheckedSignaturePayload<Address, Signature, TxExtension>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, TxExtension>;
/// Executive: handles dispatch to the various modules.
pub type Executive = pezframe_executive::Executive<
	Runtime,
	Block,
	pezframe_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

// We don't have a limit in the Relay Chain.
const IDENTITY_MIGRATION_KEY_LIMIT: u64 = u64::MAX;

// All migrations executed on runtime upgrade as a nested tuple of types implementing
// `OnRuntimeUpgrade`. Note: These are examples and do not need to be run directly
// after the genesis block.
type Migrations = (
	pezpallet_nomination_pools::migration::versioned::V6ToV7<Runtime>,
	pezpallet_alliance::migration::Migration<Runtime>,
	pezpallet_contracts::Migration<Runtime>,
	pezpallet_identity::migration::versioned::V0ToV1<Runtime, IDENTITY_MIGRATION_KEY_LIMIT>,
);

type EventRecord = pezframe_system::EventRecord<
	<Runtime as pezframe_system::Config>::RuntimeEvent,
	<Runtime as pezframe_system::Config>::Hash,
>;

parameter_types! {
	pub const BeefySetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pezpallet_beefy::Config for Runtime {
	type BeefyId = BeefyId;
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = BeefySetIdSessionEntries;
	type OnNewValidatorSet = MmrLeaf;
	type AncestryHelper = MmrLeaf;
	type WeightInfo = ();
	type KeyOwnerProof = pezsp_session::MembershipProof;
	type EquivocationReportSystem =
		pezpallet_beefy::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	pub const OracleMaxHasDispatchedSize: u32 = 20;
	pub const RootOperatorAccountId: AccountId = AccountId::new([0xffu8; 32]);

	pub const OracleMaxFeedValues: u32 = 10;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct OracleBenchmarkingHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_oracle::BenchmarkHelper<u32, u128, OracleMaxFeedValues>
	for OracleBenchmarkingHelper
{
	fn get_currency_id_value_pairs() -> BoundedVec<(u32, u128), OracleMaxFeedValues> {
		use rand::{distributions::Uniform, prelude::*};

		// Use seeded RNG like in contracts benchmarking
		let mut rng = rand_pcg::Pcg32::seed_from_u64(0x1234567890ABCDEF);
		let max_values = OracleMaxFeedValues::get() as usize;

		// Generate random pairs like in election-provider-multi-phase
		let currency_range = Uniform::new_inclusive(1, 1000);
		let value_range = Uniform::new_inclusive(1000, 1_000_000);

		let pairs: Vec<(u32, u128)> = (0..max_values)
			.map(|_| {
				let currency_id = rng.sample(currency_range);
				let value = rng.sample(value_range);
				(currency_id, value)
			})
			.collect();

		// Use try_from pattern like in core-fellowship and broker
		BoundedVec::try_from(pairs).unwrap_or_default()
	}
}

parameter_types! {
	pub const OraclePalletId: PalletId = PalletId(*b"py/oracl");
}

impl pezpallet_oracle::Config for Runtime {
	type OnNewData = ();
	type CombineData = pezpallet_oracle::DefaultCombineData<Self, ConstU32<5>, ConstU64<3600>>;
	type Time = Timestamp;
	type OracleKey = u32;
	type OracleValue = u128;
	type PalletId = OraclePalletId;
	type Members = TechnicalMembership;
	type WeightInfo = ();
	type MaxHasDispatchedSize = OracleMaxHasDispatchedSize;
	type MaxFeedValues = OracleMaxFeedValues;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = OracleBenchmarkingHelper;
}

/// MMR helper types.
mod mmr {
	use super::*;
	pub use pezpallet_mmr::primitives::*;

	pub type Leaf = <<Runtime as pezpallet_mmr::Config>::LeafData as LeafDataProvider>::LeafData;
	pub type Hash = <Hashing as pezsp_runtime::traits::Hash>::Output;
	pub type Hashing = <Runtime as pezpallet_mmr::Config>::Hashing;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetConversionTxHelper;

#[cfg(feature = "runtime-benchmarks")]
impl
	pezpallet_asset_conversion_tx_payment::BenchmarkHelperTrait<
		AccountId,
		NativeOrWithId<u32>,
		NativeOrWithId<u32>,
	> for AssetConversionTxHelper
{
	fn create_asset_id_parameter(seed: u32) -> (NativeOrWithId<u32>, NativeOrWithId<u32>) {
		(NativeOrWithId::WithId(seed), NativeOrWithId::WithId(seed))
	}

	fn setup_balances_and_pool(asset_id: NativeOrWithId<u32>, account: AccountId) {
		use pezframe_support::{assert_ok, traits::fungibles::Mutate};
		let NativeOrWithId::WithId(asset_idx) = asset_id.clone() else { unimplemented!() };
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			asset_idx.into(),
			account.clone().into(), /* owner */
			true,                   /* is_sufficient */
			1,
		));

		let lp_provider = account.clone();
		let _ = Balances::deposit_creating(&lp_provider, ((u64::MAX as u128) * 100).into());
		assert_ok!(Assets::mint_into(
			asset_idx.into(),
			&lp_provider,
			((u64::MAX as u128) * 100).into()
		));

		let token_native = alloc::boxed::Box::new(NativeOrWithId::Native);
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
			u64::MAX.into(), // 1 desired
			u64::MAX.into(), // 2 desired
			1,               // 1 min
			1,               // 2 min
			lp_provider,
		));
	}
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	pezkuwi_sdk::pezframe_benchmarking::define_benchmarks!(
		[pezframe_benchmarking, BaselineBench::<Runtime>]
		[pezframe_benchmarking_pezpallet_pov, Pov]
		[pezpallet_alliance, Alliance]
		[pezpallet_assets, Assets]
		[pezpallet_babe, Babe]
		[pezpallet_bags_list, VoterList]
		[pezpallet_balances, Balances]
		[pezpallet_beefy_mmr, MmrLeaf]
		[pezpallet_bounties, Bounties]
		[pezpallet_broker, Broker]
		[pezpallet_child_bounties, ChildBounties]
		[pezpallet_collective, Council]
		[pezpallet_conviction_voting, ConvictionVoting]
		[pezpallet_contracts, Contracts]
		[pezpallet_revive, Revive]
		[pezpallet_core_fellowship, CoreFellowship]
		[pezpallet_example_tasks, TasksExample]
		[pezpallet_democracy, Democracy]
		[pezpallet_asset_conversion, AssetConversion]
		[pezpallet_asset_rewards, AssetRewards]
		[pezpallet_asset_conversion_tx_payment, AssetConversionTxPayment]
		[pezpallet_transaction_payment, TransactionPayment]
		[pezpallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[pezpallet_election_provider_support_benchmarking, EPSBench::<Runtime>]
		[pezpallet_elections_phragmen, Elections]
		[pezpallet_fast_unstake, FastUnstake]
		[pezpallet_nis, Nis]
		[pezpallet_parameters, Parameters]
		[pezpallet_grandpa, Grandpa]
		[pezpallet_identity, Identity]
		[pezpallet_im_online, ImOnline]
		[pezpallet_indices, Indices]
		[pezpallet_lottery, Lottery]
		[pezpallet_membership, TechnicalMembership]
		[pezpallet_message_queue, MessageQueue]
		[pezpallet_migrations, MultiBlockMigrations]
		[pezpallet_mmr, Mmr]
		[pezpallet_multi_asset_bounties, MultiAssetBounties]
		[pezpallet_multisig, Multisig]
		[pezpallet_nomination_pools, NominationPoolsBench::<Runtime>]
		[pezpallet_offences, OffencesBench::<Runtime>]
		[pezpallet_oracle, Oracle]
		[pezpallet_preimage, Preimage]
		[pezpallet_proxy, Proxy]
		[pezpallet_ranked_collective, RankedCollective]
		[pezpallet_referenda, Referenda]
		[pezpallet_recovery, Recovery]
		[pezpallet_remark, Remark]
		[pezpallet_salary, Salary]
		[pezpallet_scheduler, Scheduler]
		[pezpallet_glutton, Glutton]
		[pezpallet_session, SessionBench::<Runtime>]
		[pezpallet_society, Society]
		[pezpallet_staking, Staking]
		[pezpallet_state_trie_migration, StateTrieMigration]
		[pezpallet_sudo, Sudo]
		[pezframe_system, SystemBench::<Runtime>]
		[pezframe_system_extensions, SystemExtensionsBench::<Runtime>]
		[pezpallet_timestamp, Timestamp]
		[pezpallet_tips, Tips]
		[pezpallet_transaction_storage, TransactionStorage]
		[pezpallet_treasury, Treasury]
		[pezpallet_asset_rate, AssetRate]
		[pezpallet_uniques, Uniques]
		[pezpallet_nfts, Nfts]
		[pezpallet_nft_fractionalization, NftFractionalization]
		[pezpallet_utility, Utility]
		[pezpallet_vesting, Vesting]
		[pezpallet_whitelist, Whitelist]
		[pezpallet_tx_pause, TxPause]
		[pezpallet_safe_mode, SafeMode]
		[pezpallet_example_mbm, PalletExampleMbms]
		[pezpallet_asset_conversion_ops, AssetConversionMigration]
		[pezpallet_verify_signature, VerifySignature]
		[pezpallet_meta_tx, MetaTx]
	);
}

pezpallet_revive::impl_runtime_apis_plus_revive_traits!(
	Runtime,
	Revive,
	Executive,
	EthExtraImpl,

	impl pezsp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: <Block as BlockT>::LazyBlock) {
			Executive::execute_block(block);
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

	impl pezframe_support::view_functions::runtime_api::RuntimeViewFunction<Block> for Runtime {
		fn execute_view_function(id: pezframe_support::view_functions::ViewFunctionId, input: Vec<u8>) -> Result<Vec<u8>, pezframe_support::view_functions::ViewFunctionDispatchError> {
			Runtime::execute_view_function(id, input)
		}
	}

	impl pezsp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: <Block as BlockT>::LazyBlock, data: InherentData) -> CheckInherentsResult {
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

	impl pezsp_statement_store::runtime_api::ValidateStatement<Block> for Runtime {
		fn validate_statement(
			source: pezsp_statement_store::runtime_api::StatementSource,
			statement: pezsp_statement_store::Statement,
		) -> Result<pezsp_statement_store::runtime_api::ValidStatement, pezsp_statement_store::runtime_api::InvalidStatement> {
			Statement::validate_statement(source, statement)
		}
	}

	impl pezsp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl pezsp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> pezsp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> pezsp_consensus_grandpa::SetId {
			pezpallet_grandpa::CurrentSetId::<Runtime>::get()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: pezsp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: pezsp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: pezsp_consensus_grandpa::SetId,
			authority_id: GrandpaId,
		) -> Option<pezsp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((pezsp_consensus_grandpa::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(pezsp_consensus_grandpa::OpaqueKeyOwnershipProof::new)
		}
	}

	impl pezpallet_nomination_pools_runtime_api::NominationPoolsApi<Block, AccountId, Balance> for Runtime {
		fn pending_rewards(who: AccountId) -> Balance {
			NominationPools::api_pending_rewards(who).unwrap_or_default()
		}

		fn points_to_balance(pool_id: PoolId, points: Balance) -> Balance {
			NominationPools::api_points_to_balance(pool_id, points)
		}

		fn balance_to_points(pool_id: PoolId, new_funds: Balance) -> Balance {
			NominationPools::api_balance_to_points(pool_id, new_funds)
		}

		fn pool_pending_slash(pool_id: PoolId) -> Balance {
			NominationPools::api_pool_pending_slash(pool_id)
		}

		fn member_pending_slash(member: AccountId) -> Balance {
			NominationPools::api_member_pending_slash(member)
		}

		fn pool_needs_delegate_migration(pool_id: PoolId) -> bool {
			NominationPools::api_pool_needs_delegate_migration(pool_id)
		}

		fn member_needs_delegate_migration(member: AccountId) -> bool {
			NominationPools::api_member_needs_delegate_migration(member)
		}

		fn member_total_balance(member: AccountId) -> Balance {
			NominationPools::api_member_total_balance(member)
		}

		fn pool_balance(pool_id: PoolId) -> Balance {
			NominationPools::api_pool_balance(pool_id)
		}

		fn pool_accounts(pool_id: PoolId) -> (AccountId, AccountId) {
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

	impl pezsp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> pezsp_consensus_babe::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
			pezsp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> pezsp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> pezsp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> pezsp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: pezsp_consensus_babe::Slot,
			authority_id: pezsp_consensus_babe::AuthorityId,
		) -> Option<pezsp_consensus_babe::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((pezsp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(pezsp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: pezsp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: pezsp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl pezsp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl pezkuwi_sdk::pezpallet_oracle_runtime_api::OracleApi<Block, u32, u32, u128> for Runtime {
		fn get_value(_provider_id: u32, key: u32) -> Option<u128> {
			// ProviderId is unused as we only have 1 provider
			pezpallet_oracle::Pezpallet::<Runtime>::get(&key).map(|v| v.value)
		}

		fn get_all_values(_provider_id: u32) -> Vec<(u32, Option<u128>)> {
			use pezpallet_oracle::DataProviderExtended;
			pezpallet_oracle::Pezpallet::<Runtime>::get_all_values()
				.map(|(k, v)| (k, v.map(|tv| tv.value)))
				.collect()
		}
	}

	impl pezframe_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl assets_api::AssetsApi<
		Block,
		AccountId,
		Balance,
		u32,
	> for Runtime
	{
		fn account_balances(account: AccountId) -> Vec<(u32, Balance)> {
			Assets::account_balances(account)
		}
	}

	impl pezpallet_contracts::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord> for Runtime
	{
		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pezpallet_contracts::ContractExecResult<Balance, EventRecord> {
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
			Contracts::bare_call(
				origin,
				dest,
				value,
				gas_limit,
				storage_deposit_limit,
				input_data,
				pezpallet_contracts::DebugInfo::UnsafeDebug,
				pezpallet_contracts::CollectEvents::UnsafeCollect,
				pezpallet_contracts::Determinism::Enforced,
			)
		}

		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			code: pezpallet_contracts::Code<Hash>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> pezpallet_contracts::ContractInstantiateResult<AccountId, Balance, EventRecord>
		{
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
			Contracts::bare_instantiate(
				origin,
				value,
				gas_limit,
				storage_deposit_limit,
				code,
				data,
				salt,
				pezpallet_contracts::DebugInfo::UnsafeDebug,
				pezpallet_contracts::CollectEvents::UnsafeCollect,
			)
		}

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
			determinism: pezpallet_contracts::Determinism,
		) -> pezpallet_contracts::CodeUploadResult<Hash, Balance>
		{
			Contracts::bare_upload_code(
				origin,
				code,
				storage_deposit_limit,
				determinism,
			)
		}

		fn get_storage(
			address: AccountId,
			key: Vec<u8>,
		) -> pezpallet_contracts::GetStorageResult {
			Contracts::get_storage(
				address,
				key
			)
		}
	}

	impl pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pezpallet_asset_conversion::AssetConversionApi<
		Block,
		Balance,
		NativeOrWithId<u32>
	> for Runtime
	{
		fn quote_price_exact_tokens_for_tokens(asset1: NativeOrWithId<u32>, asset2: NativeOrWithId<u32>, amount: Balance, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_exact_tokens_for_tokens(asset1, asset2, amount, include_fee)
		}

		fn quote_price_tokens_for_exact_tokens(asset1: NativeOrWithId<u32>, asset2: NativeOrWithId<u32>, amount: Balance, include_fee: bool) -> Option<Balance> {
			AssetConversion::quote_price_tokens_for_exact_tokens(asset1, asset2, amount, include_fee)
		}

		fn get_reserves(asset1: NativeOrWithId<u32>, asset2: NativeOrWithId<u32>) -> Option<(Balance, Balance)> {
			AssetConversion::get_reserves(asset1, asset2).ok()
		}
	}

	impl pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(call: RuntimeCall, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(call: RuntimeCall, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pezpallet_nfts_runtime_api::NftsApi<Block, AccountId, u32, u32> for Runtime {
		fn owner(collection: u32, item: u32) -> Option<AccountId> {
			<Nfts as Inspect<AccountId>>::owner(&collection, &item)
		}

		fn collection_owner(collection: u32) -> Option<AccountId> {
			<Nfts as Inspect<AccountId>>::collection_owner(&collection)
		}

		fn attribute(
			collection: u32,
			item: u32,
			key: Vec<u8>,
		) -> Option<Vec<u8>> {
			<Nfts as Inspect<AccountId>>::attribute(&collection, &item, &key)
		}

		fn custom_attribute(
			account: AccountId,
			collection: u32,
			item: u32,
			key: Vec<u8>,
		) -> Option<Vec<u8>> {
			<Nfts as Inspect<AccountId>>::custom_attribute(
				&account,
				&collection,
				&item,
				&key,
			)
		}

		fn system_attribute(
			collection: u32,
			item: Option<u32>,
			key: Vec<u8>,
		) -> Option<Vec<u8>> {
			<Nfts as Inspect<AccountId>>::system_attribute(&collection, item.as_ref(), &key)
		}

		fn collection_attribute(collection: u32, key: Vec<u8>) -> Option<Vec<u8>> {
			<Nfts as Inspect<AccountId>>::collection_attribute(&collection, &key)
		}
	}

	#[api_version(6)]
	impl pezsp_consensus_beefy::BeefyApi<Block, BeefyId> for Runtime {
		fn beefy_genesis() -> Option<BlockNumber> {
			pezpallet_beefy::GenesisBlock::<Runtime>::get()
		}

		fn validator_set() -> Option<pezsp_consensus_beefy::ValidatorSet<BeefyId>> {
			Beefy::validator_set()
		}

		fn submit_report_double_voting_unsigned_extrinsic(
			equivocation_proof: pezsp_consensus_beefy::DoubleVotingProof<
				BlockNumber,
				BeefyId,
				BeefySignature,
			>,
			key_owner_proof: pezsp_consensus_beefy::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Beefy::submit_unsigned_double_voting_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn submit_report_fork_voting_unsigned_extrinsic(
			equivocation_proof:
				pezsp_consensus_beefy::ForkVotingProof<
					<Block as BlockT>::Header,
					BeefyId,
					pezsp_runtime::OpaqueValue
				>,
			key_owner_proof: pezsp_consensus_beefy::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			Beefy::submit_unsigned_fork_voting_report(
				equivocation_proof.try_into()?,
				key_owner_proof.decode()?,
			)
		}

		fn submit_report_future_block_voting_unsigned_extrinsic(
			equivocation_proof: pezsp_consensus_beefy::FutureBlockVotingProof<BlockNumber, BeefyId>,
			key_owner_proof: pezsp_consensus_beefy::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			Beefy::submit_unsigned_future_block_voting_report(
				equivocation_proof,
				key_owner_proof.decode()?,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: pezsp_consensus_beefy::ValidatorSetId,
			authority_id: BeefyId,
		) -> Option<pezsp_consensus_beefy::OpaqueKeyOwnershipProof> {
			Historical::prove((pezsp_consensus_beefy::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(pezsp_consensus_beefy::OpaqueKeyOwnershipProof::new)
		}
	}

	#[api_version(3)]
	impl pezpallet_mmr::primitives::MmrApi<
		Block,
		mmr::Hash,
		BlockNumber,
	> for Runtime {
		fn mmr_root() -> Result<mmr::Hash, mmr::Error> {
			Ok(pezpallet_mmr::RootHash::<Runtime>::get())
		}

		fn mmr_leaf_count() -> Result<mmr::LeafIndex, mmr::Error> {
			Ok(pezpallet_mmr::NumberOfLeaves::<Runtime>::get())
		}

		fn generate_proof(
			block_numbers: Vec<BlockNumber>,
			best_known_block_number: Option<BlockNumber>,
		) -> Result<(Vec<mmr::EncodableOpaqueLeaf>, mmr::LeafProof<mmr::Hash>), mmr::Error> {
			Mmr::generate_proof(block_numbers, best_known_block_number).map(
				|(leaves, proof)| {
					(
						leaves
							.into_iter()
							.map(|leaf| mmr::EncodableOpaqueLeaf::from_leaf(&leaf))
							.collect(),
						proof,
					)
				},
			)
		}

		fn verify_proof(leaves: Vec<mmr::EncodableOpaqueLeaf>, proof: mmr::LeafProof<mmr::Hash>)
			-> Result<(), mmr::Error>
		{
			let leaves = leaves.into_iter().map(|leaf|
				leaf.into_opaque_leaf()
				.try_decode()
				.ok_or(mmr::Error::Verify)).collect::<Result<Vec<mmr::Leaf>, mmr::Error>>()?;
			Mmr::verify_leaves(leaves, proof)
		}

		fn generate_ancestry_proof(
			prev_block_number: BlockNumber,
			best_known_block_number: Option<BlockNumber>,
		) -> Result<mmr::AncestryProof<mmr::Hash>, mmr::Error> {
			Mmr::generate_ancestry_proof(prev_block_number, best_known_block_number)
		}

		fn verify_proof_stateless(
			root: mmr::Hash,
			leaves: Vec<mmr::EncodableOpaqueLeaf>,
			proof: mmr::LeafProof<mmr::Hash>
		) -> Result<(), mmr::Error> {
			let nodes = leaves.into_iter().map(|leaf|mmr::DataOrHash::Data(leaf.into_opaque_leaf())).collect();
			pezpallet_mmr::verify_leaves_proof::<mmr::Hashing, _>(root, nodes, proof)
		}
	}

	impl pezsp_mixnet::runtime_api::MixnetApi<Block> for Runtime {
		fn session_status() -> pezsp_mixnet::types::SessionStatus {
			Mixnet::session_status()
		}

		fn prev_mixnodes() -> Result<Vec<pezsp_mixnet::types::Mixnode>, pezsp_mixnet::types::MixnodesErr> {
			Mixnet::prev_mixnodes()
		}

		fn current_mixnodes() -> Result<Vec<pezsp_mixnet::types::Mixnode>, pezsp_mixnet::types::MixnodesErr> {
			Mixnet::current_mixnodes()
		}

		fn maybe_register(session_index: pezsp_mixnet::types::SessionIndex, mixnode: pezsp_mixnet::types::Mixnode) -> bool {
			Mixnet::maybe_register(session_index, mixnode)
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

	impl pezpallet_asset_rewards::AssetRewards<Block, Balance> for Runtime {
		fn pool_creation_cost() -> Balance {
			StakePoolCreationDeposit::get()
		}
	}

	#[cfg(feature = "try-runtime")]
	impl pezframe_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: pezframe_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: <Block as BlockT>::LazyBlock,
			state_root_check: bool,
			signature_check: bool,
			select: pezframe_try_runtime::TryStateSelect
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
			use pezframe_benchmarking::{baseline, BenchmarkList};
			use pezframe_support::traits::StorageInfoTrait;

			// Trying to add benchmarks directly to the Session Pezpallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pezpallet_session_benchmarking::Pezpallet as SessionBench;
			use pezpallet_offences_benchmarking::Pezpallet as OffencesBench;
			use pezpallet_election_provider_support_benchmarking::Pezpallet as EPSBench;
			use pezframe_system_benchmarking::Pezpallet as SystemBench;
			use pezframe_system_benchmarking::extensions::Pezpallet as SystemExtensionsBench;
			use baseline::Pezpallet as BaselineBench;
			use pezpallet_nomination_pools_benchmarking::Pezpallet as NominationPoolsBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		#[allow(non_local_definitions)]
		fn dispatch_benchmark(
			config: pezframe_benchmarking::BenchmarkConfig
		) -> Result<Vec<pezframe_benchmarking::BenchmarkBatch>, alloc::string::String> {
			use pezframe_benchmarking::{baseline, BenchmarkBatch};
			use pezsp_storage::TrackedStorageKey;

			// Trying to add benchmarks directly to the Session Pezpallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pezpallet_session_benchmarking::Pezpallet as SessionBench;
			use pezpallet_offences_benchmarking::Pezpallet as OffencesBench;
			use pezpallet_election_provider_support_benchmarking::Pezpallet as EPSBench;
			use pezframe_system_benchmarking::Pezpallet as SystemBench;
			use pezframe_system_benchmarking::extensions::Pezpallet as SystemExtensionsBench;
			use baseline::Pezpallet as BaselineBench;
			use pezpallet_nomination_pools_benchmarking::Pezpallet as NominationPoolsBench;

			impl pezpallet_session_benchmarking::Config for Runtime {}
			impl pezpallet_offences_benchmarking::Config for Runtime {}
			impl pezpallet_election_provider_support_benchmarking::Config for Runtime {}
			impl pezframe_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}
			impl pezpallet_nomination_pools_benchmarking::Config for Runtime {}

			use pezframe_support::traits::WhitelistedStorageKeys;
			let mut whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			// Treasury Account
			// TODO: this is manual for now, someday we might be able to use a
			// macro for this particular key
			let treasury_key = pezframe_system::Account::<Runtime>::hashed_key_for(Treasury::account_id());
			whitelist.push(treasury_key.to_vec().into());

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

);

#[cfg(test)]
mod tests {
	use super::*;
	use pezframe_system::offchain::CreateSignedTransaction;

	#[test]
	fn validate_transaction_submitter_bounds() {
		fn is_submit_signed_transaction<T>()
		where
			T: CreateSignedTransaction<RuntimeCall>,
		{
		}

		is_submit_signed_transaction::<Runtime>();
	}

	#[test]
	fn call_size() {
		let size = core::mem::size_of::<RuntimeCall>();
		assert!(
			size <= CALL_PARAMS_MAX_SIZE,
			"size of RuntimeCall {} is more than {CALL_PARAMS_MAX_SIZE} bytes.
			 Some calls have too big arguments, use Box to reduce the size of RuntimeCall.
			 If the limit is too strong, maybe consider increase the limit.",
			size,
		);
	}
}
