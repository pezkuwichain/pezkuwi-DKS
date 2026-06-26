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

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

mod genesis_config_presets;
mod xcm_config;

use crate::xcm_config::{RelayLocation, XcmOriginToTransactDispatchOrigin};

pub use pezkuwi_sdk::{pezstaging_teyrchain_info as teyrchain_info, *};
use pezpallet_xcm::{EnsureXcm, IsVoiceOfBody};

use pezcumulus_primitives_core::ParaId;
use pezkuwi_runtime_common::{prod_or_fast, xcm_sender::NoPriceForMessageDelivery};
use teyrchains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};

use alloc::{borrow::Cow, vec, vec::Vec};
use pezcumulus_pezpallet_teyrchain_system::RelayNumberMonotonicallyIncreases;
use pezframe_support::weights::{constants, FixedFee, RuntimeDbWeight};
use pezsp_api::impl_runtime_apis;
use pezsp_core::OpaqueMetadata;
use pezsp_runtime::{
	generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, Hash as HashT},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature, MultiSigner,
};
#[cfg(feature = "std")]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use pezframe_support::{
	construct_runtime, derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstBool, ConstU32, ConstU64, ConstU8, Contains, EitherOfDiverse,
		Everything, HandleMessage, IsInVec, Nothing, QueueFootprint, Randomness, TransformOrigin,
	},
	weights::{
		constants::{BlockExecutionWeight, WEIGHT_REF_TIME_PER_SECOND},
		ConstantMultiplier, IdentityFee, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
		WeightToFeePolynomial,
	},
	BoundedSlice, PalletId, StorageValue,
};
use pezframe_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
pub use pezpallet_balances::Call as BalancesCall;
pub use pezpallet_timestamp::Call as TimestampCall;
pub use pezsp_consensus_aura::sr25519::AuthorityId as AuraId;
#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;
pub use pezsp_runtime::{Perbill, Permill};

use pezcumulus_primitives_core::AggregateMessageOrigin; //, ClaimQueueOffset, CoreSelector};
use pezstaging_xcm::latest::prelude::BodyId;
use teyrchains_common::{AccountId, Signature};

pub type SessionHandlers = ();

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

/// This runtime version.
#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("yet-another-teyrchain"),
	impl_name: Cow::Borrowed("yet-another-teyrchain"),
	authoring_version: 1,
	spec_version: 1_003_000,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 6,
	system_version: 1,
};

pub const MILLISECS_PER_BLOCK: u64 = 2000;

pub const SLOT_DURATION: u64 = 3 * MILLISECS_PER_BLOCK;

pub const EPOCH_DURATION_IN_BLOCKS: u32 = 10 * MINUTES;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const YAP: Balance = 1_000_000_000_000;
pub const NANOYAP: Balance = 1_000;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 95%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(95);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
	pezcumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
/// into the relay chain.
const UNINCLUDED_SEGMENT_CAPACITY: u32 = 10;

/// Build with an offset of 1 behind the relay chain.
const RELAY_PARENT_OFFSET: u32 = 1;

/// How many teyrchain blocks are processed by the relay chain per parent. Limits the
/// number of blocks authored per slot.
const BLOCK_PROCESSING_VELOCITY: u32 = 3;
/// Relay chain slot duration, in milliseconds.
const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = <pezpallet_verify_signature::weights::BizinikiwiWeight::<Runtime> as pezpallet_verify_signature::WeightInfo>::verify_signature();
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
	// We assume the whole teyrchain state fits into the trie cache
	// Numbers are from <https://github.com/pezkuwichain/pezkuwi-sdk/issues/272>
	pub const InMemoryDbWeight: RuntimeDbWeight = RuntimeDbWeight {
		read: 9_000 * constants::WEIGHT_REF_TIME_PER_NANOS,
		write: 28_000 * constants::WEIGHT_REF_TIME_PER_NANOS,
	};
}

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
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = InMemoryDbWeight;
	type BaseCallFilter = pezframe_support::traits::Everything;
	type SystemWeightInfo = pezframe_system::weights::BizinikiwiWeight<Self>;
	type ExtensionsWeightInfo = pezframe_system::BizinikiwiExtensionsWeight<Self>;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = pezcumulus_pezpallet_teyrchain_system::TeyrchainSetCode<Self>;
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
	type SingleBlockMigrations = RemoveCollectiveFlip;
}

impl pezcumulus_pezpallet_weight_reclaim::Config for Runtime {
	type WeightInfo = ();
}

impl pezpallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<0>;
	type WeightInfo = pezpallet_timestamp::weights::BizinikiwiWeight<Self>;
}

parameter_types! {
	pub const Period: u32 = prod_or_fast!(10 * MINUTES, 10);
	pub const Offset: u32 = 0;
}

impl pezpallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as pezframe_system::Config>::AccountId;
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

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const SessionLength: BlockNumber = prod_or_fast!(10 * MINUTES, 10);
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root and the StakingAdmin to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsVoiceOfBody<RelayLocation, StakingAdminBodyId>>,
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
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = NANOYAP;
	pub const TransactionByteFee: u128 = NANOYAP;
}

impl pezpallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pezpallet_balances::weights::BizinikiwiWeight<Self>;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type DoneSlashHandler = ();
}

impl pezpallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pezpallet_transaction_payment::FungibleAdapter<Balances, ()>;
	type WeightToFee = FixedFee<1, <Self as pezpallet_balances::Config>::Balance>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightInfo = pezpallet_transaction_payment::weights::BizinikiwiWeight<Self>;
}

impl pezpallet_sudo::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pezpallet_sudo::weights::BizinikiwiWeight<Runtime>;
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

pub struct DmpSink;
impl HandleMessage for DmpSink {
	type MaxMessageLen = ConstU32<16>;

	fn handle_message(_msg: BoundedSlice<u8, Self::MaxMessageLen>) {}

	fn handle_messages<'a>(_: impl Iterator<Item = BoundedSlice<'a, u8, Self::MaxMessageLen>>) {
		unimplemented!()
	}

	fn sweep_queue() {
		unimplemented!()
	}
}

impl pezcumulus_pezpallet_teyrchain_system::Config for Runtime {
	type WeightInfo = pezcumulus_pezpallet_teyrchain_system::weights::BizinikiwiWeight<Self>;
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = teyrchain_info::Pezpallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpQueue = pezframe_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
	type RelayParentOffset = ConstU32<RELAY_PARENT_OFFSET>;
}

impl pezpallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MessageProcessor = pezstaging_xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		pezstaging_xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	// The XCMP queue pezpallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = pezsp_core::ConstU32<{ 103 * 1024 }>;
	type MaxStale = pezsp_core::ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = ();
}
parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pezpallet_authorship::Config for Runtime {
	type FindAuthor = ();
	type EventHandler = ();
}

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// in Pezkuwichain, extrinsic base weight (smallest non-zero weight) is mapped to 1
		// MILLI_UNIT: in our template, we map to 1/10 of that, or 1/10 MILLI_UNIT
		let p = YAP / 10;
		let q = 100
			* Balance::from(
				pezframe_support::weights::constants::ExtrinsicBaseWeight::get().ref_time(),
			);
		vec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
		.into()
	}
}

impl pezcumulus_pezpallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = TeyrchainSystem;
	type VersionWrapper = ();
	// Enqueue XCMP messages from siblings for later processing.
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = pezsp_core::ConstU32<1_000>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	type MaxPageSize = ConstU32<{ 1 << 16 }>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
	type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
}

impl teyrchain_info::Config for Runtime {}

impl pezcumulus_pezpallet_aura_ext::Config for Runtime {}

impl pezpallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<true>;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

impl pezpallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

#[cfg(feature = "runtime-benchmarks")]
pub struct VerifySignatureBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_verify_signature::BenchmarkHelper<MultiSignature, AccountId>
	for VerifySignatureBenchmarkHelper
{
	fn create_signature(_entropy: &[u8], msg: &[u8]) -> (MultiSignature, AccountId) {
		use pezsp_io::crypto::{sr25519_generate, sr25519_sign};
		use pezsp_runtime::traits::IdentifyAccount;
		let public = sr25519_generate(0.into(), None);
		let who_account: AccountId = MultiSigner::Sr25519(public).into_account().into();
		let signature = MultiSignature::Sr25519(sr25519_sign(0.into(), &public, msg).unwrap());
		(signature, who_account)
	}
}

impl pezpallet_verify_signature::Config for Runtime {
	type Signature = MultiSignature;
	type AccountIdentifier = MultiSigner;
	type WeightInfo = pezpallet_verify_signature::weights::BizinikiwiWeight<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = VerifySignatureBenchmarkHelper;
}

construct_runtime!(
	pub struct Runtime {
		System: pezframe_system = 0,
		Timestamp: pezpallet_timestamp = 1,
		Sudo: pezpallet_sudo = 2,
		TransactionPayment: pezpallet_transaction_payment = 3,
		WeightReclaim: pezcumulus_pezpallet_weight_reclaim = 4,

		TeyrchainSystem: pezcumulus_pezpallet_teyrchain_system = 20,
		TeyrchainInfo: teyrchain_info = 21,

		Authorship: pezpallet_authorship = 25,
		CollatorSelection: pezpallet_collator_selection = 26,
		Session: pezpallet_session = 27,

		Balances: pezpallet_balances = 30,

		Aura: pezpallet_aura = 31,
		AuraExt: pezcumulus_pezpallet_aura_ext = 32,

		Utility: pezpallet_utility = 40,
		VerifySignature: pezpallet_verify_signature = 41,

		XcmpQueue: pezcumulus_pezpallet_xcmp_queue = 51,
		PezkuwiXcm: pezpallet_xcm = 52,
		CumulusXcm: pezcumulus_pezpallet_xcm = 53,
		MessageQueue: pezpallet_message_queue = 54,
	}
);

/// Balance of an account.
pub type Balance = u128;
/// Index of a transaction in the chain.
pub type Nonce = u32;
/// A hash of some data used by the chain.
pub type Hash = <BlakeTwo256 as HashT>::Output;
/// An index to a block.
pub type BlockNumber = u32;
/// The address format for describing accounts.
pub type Address = pezsp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The TransactionExtension to the basic transaction logic.
pub type TxExtension = pezcumulus_pezpallet_weight_reclaim::StorageWeightReclaim<
	Runtime,
	(
		// Uncomment this to enable running signed transactions using v5 extrinsics.
		// pezpallet_verify_signature::VerifySignature<Runtime>,
		pezframe_system::CheckNonZeroSender<Runtime>,
		pezframe_system::CheckSpecVersion<Runtime>,
		pezframe_system::CheckTxVersion<Runtime>,
		pezframe_system::CheckGenesis<Runtime>,
		pezframe_system::CheckEra<Runtime>,
		pezframe_system::CheckNonce<Runtime>,
		pezframe_system::CheckWeight<Runtime>,
		pezpallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	),
>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
/// Executive: handles dispatch to the various modules.
pub type Executive = pezframe_executive::Executive<
	Runtime,
	Block,
	pezframe_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

pub struct RemoveCollectiveFlip;
impl pezframe_support::traits::OnRuntimeUpgrade for RemoveCollectiveFlip {
	fn on_runtime_upgrade() -> Weight {
		use pezframe_support::storage::migration;
		// Remove the storage value `RandomMaterial` from removed pezpallet
		// `RandomnessCollectiveFlip`
		#[allow(deprecated)]
		migration::remove_storage_prefix(b"RandomnessCollectiveFlip", b"RandomMaterial", b"");
		<Runtime as pezframe_system::Config>::DbWeight::get().writes(1)
	}
}

impl_runtime_apis! {
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

	impl pezsp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(
			extrinsic: <Block as BlockT>::Extrinsic,
		) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: pezsp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: <Block as BlockT>::LazyBlock, data: pezsp_inherents::InherentData) -> pezsp_inherents::CheckInherentsResult {
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
		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, pezsp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}

		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}
	}

	impl pezsp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> pezsp_consensus_aura::SlotDuration {
			pezsp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			pezpallet_aura::Authorities::<Runtime>::get().into_inner()
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

	impl pezcumulus_primitives_core::RelayParentOffsetApi<Block> for Runtime {
		fn relay_parent_offset() -> u32 {
			RELAY_PARENT_OFFSET
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

	impl pezcumulus_primitives_core::GetTeyrchainInfo<Block> for Runtime {
		fn teyrchain_id() -> ParaId {
			TeyrchainInfo::teyrchain_id()
		}
	}
}

pezcumulus_pezpallet_teyrchain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = pezcumulus_pezpallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
