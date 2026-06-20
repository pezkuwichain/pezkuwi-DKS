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

//! # Bridge Hub Zagros Runtime
//!
//! This runtime currently supports bridging between:
//! - Pezkuwichain <> Zagros

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod bridge_common_config;
pub mod bridge_to_ethereum_config;
pub mod bridge_to_pezkuwichain_config;
mod genesis_config_presets;
mod weights;
pub mod xcm_config;

extern crate alloc;

use alloc::{vec, vec::Vec};
use pezbridge_runtime_common::extensions::{
	CheckAndBoostBridgeGrandpaTransactions, CheckAndBoostBridgeTeyrchainsTransactions,
};
use pezcumulus_pezpallet_teyrchain_system::RelayNumberMonotonicallyIncreases;
use pezcumulus_primitives_core::ParaId;
use pezsp_api::impl_runtime_apis;
use pezsp_core::{crypto::KeyTypeId, OpaqueMetadata};
use pezsp_runtime::{
	generic, impl_opaque_keys,
	traits::Block as BlockT,
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};
#[cfg(feature = "std")]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;

use pezbridge_hub_common::{
	message_queue::{NarrowOriginToSibling, ParaIdToSibling},
	AggregateMessageOrigin,
};
use pezframe_support::{
	construct_runtime, derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{ConstBool, ConstU32, ConstU64, ConstU8, Get, TransformOrigin},
	weights::{ConstantMultiplier, Weight},
	PalletId,
};
use pezframe_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
pub use pezsp_consensus_aura::sr25519::AuthorityId as AuraId;
pub use pezsp_runtime::{MultiAddress, Perbill, Permill};
use xcm_config::{XcmConfig, XcmOriginToTransactDispatchOrigin, XcmRouter};

use xcm_runtime_pezapis::{
	dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
	fees::Error as XcmPaymentApiError,
};

use pezbp_runtime::HeaderId;
use pezpallet_bridge_messages::LaneIdOf;
#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;

use pezkuwi_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};

#[cfg(feature = "runtime-benchmarks")]
use xcm::latest::PEZKUWICHAIN_GENESIS_HASH;
use xcm::prelude::*;

use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

use pezsnowbridge_core::{AgentId, PricingParameters};
use pezsnowbridge_outbound_queue_primitives::v1::{Command, Fee};
use testnet_teyrchains_constants::zagros::{consensus::*, currency::*, fee::WeightToFee, time::*};
use teyrchains_common::{
	impls::DealWithFees, AccountId, Balance, BlockNumber, Hash, Header, Nonce, Signature,
	AVERAGE_ON_INITIALIZE_RATIO, NORMAL_DISPATCH_RATIO,
};
use xcm::{Version as XcmVersion, VersionedLocation};

use zagros_runtime_constants::system_teyrchain::{ASSET_HUB_ID, BRIDGE_HUB_ID};

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

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
		(
			pezframe_system::AuthorizeCall<Runtime>,
			pezframe_system::CheckNonZeroSender<Runtime>,
			pezframe_system::CheckSpecVersion<Runtime>,
			pezframe_system::CheckTxVersion<Runtime>,
			pezframe_system::CheckGenesis<Runtime>,
			pezframe_system::CheckEra<Runtime>,
			pezframe_system::CheckNonce<Runtime>,
			pezframe_system::CheckWeight<Runtime>,
		),
		pezpallet_transaction_payment::ChargeTransactionPayment<Runtime>,
		BridgeRejectObsoleteHeadersAndMessages,
		(bridge_to_pezkuwichain_config::OnBridgeHubZagrosRefundBridgeHubPezkuwichainMessages,),
		pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
	),
>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// Migrations to apply on runtime upgrade.
pub type Migrations = (
	pezpallet_collator_selection::migration::v2::MigrationToV2<Runtime>,
	pezpallet_multisig::migrations::v1::MigrateToV1<Runtime>,
	InitStorageVersions,
	// unreleased
	pezcumulus_pezpallet_xcmp_queue::migration::v4::MigrationToV4<Runtime>,
	pezcumulus_pezpallet_xcmp_queue::migration::v5::MigrateV4ToV5<Runtime>,
	pezpallet_bridge_messages::migration::v1::MigrationToV1<
		Runtime,
		bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance,
	>,
	bridge_to_pezkuwichain_config::migration::FixMessagesV1Migration<
		Runtime,
		bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance,
	>,
	pezframe_support::migrations::RemoveStorage<
		BridgePezkuwichainMessagesPalletName,
		OutboundLanesCongestedSignalsKey,
		RocksDbWeight,
	>,
	pezpallet_bridge_relayers::migration::v1::MigrationToV1<
		Runtime,
		bridge_common_config::BridgeRelayersInstance,
		pezbp_messages::LegacyLaneId,
	>,
	pezpallet_bridge_relayers::migration::v2::MigrationToV2<
		Runtime,
		bridge_common_config::BridgeRelayersInstance,
		pezbp_messages::LegacyLaneId,
	>,
	pezsnowbridge_pezpallet_system::migration::v0::InitializeOnUpgrade<
		Runtime,
		ConstU32<BRIDGE_HUB_ID>,
		ConstU32<ASSET_HUB_ID>,
	>,
	pezsnowbridge_pezpallet_system::migration::FeePerGasMigrationV0ToV1<Runtime>,
	bridge_to_ethereum_config::migrations::MigrationForXcmV5<Runtime>,
	pezpallet_session::migrations::v1::MigrateV0ToV1<
		Runtime,
		pezpallet_session::migrations::v1::InitOffenceSeverity<Runtime>,
	>,
	// permanent
	pezpallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
	pezcumulus_pezpallet_aura_ext::migration::MigrateV0ToV1<Runtime>,
);

parameter_types! {
	pub const BridgePezkuwichainMessagesPalletName: &'static str = "BridgePezkuwichainMessages";
	pub const OutboundLanesCongestedSignalsKey: &'static str = "OutboundLanesCongestedSignals";
}

/// Migration to initialize storage versions for pallets added after genesis.
///
/// Ideally this would be done automatically (see
/// <https://github.com/pezkuwichain/pezkuwi-sdk/issues/248>), but it probably won't be ready for some
/// time and it's beneficial to get try-runtime-cli on-runtime-upgrade checks into the CI, so we're
/// doing it manually.
pub struct InitStorageVersions;

impl pezframe_support::traits::OnRuntimeUpgrade for InitStorageVersions {
	fn on_runtime_upgrade() -> Weight {
		use pezframe_support::traits::{GetStorageVersion, StorageVersion};
		use pezsp_runtime::traits::Saturating;

		let mut writes = 0;

		if PezkuwiXcm::on_chain_storage_version() == StorageVersion::new(0) {
			PezkuwiXcm::in_code_storage_version().put::<PezkuwiXcm>();
			writes.saturating_inc();
		}

		if Balances::on_chain_storage_version() == StorageVersion::new(0) {
			Balances::in_code_storage_version().put::<Balances>();
			writes.saturating_inc();
		}

		<Runtime as pezframe_system::Config>::DbWeight::get().reads_writes(2, writes)
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

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("bridge-hub-zagros"),
	impl_name: alloc::borrow::Cow::Borrowed("bridge-hub-zagros"),
	authoring_version: 1,
	spec_version: 1_020_002,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 6,
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
	pub const SS58Prefix: u16 = 42;
}

// Configure FRAME pallets to include in runtime.

#[derive_impl(pezframe_system::config_preludes::TeyrchainDefaultConfig)]
impl pezframe_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The block type.
	type Block = Block;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pezpallet_balances::AccountData<Balance>;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Weight information for the extrinsics of this pezpallet.
	type SystemWeightInfo = weights::pezframe_system::WeightInfo<Runtime>;
	/// Weight information for the transaction extensions of this pezpallet.
	type ExtensionsWeightInfo = weights::pezframe_system_extensions::WeightInfo<Runtime>;
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
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = weights::pezpallet_balances::WeightInfo<Runtime>;
	type MaxLocks = ConstU32<50>;
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
	pub const TransactionByteFee: Balance = MILLICENTS;
}

impl pezpallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction =
		pezpallet_transaction_payment::FungibleAdapter<Balances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type WeightInfo = weights::pezpallet_transaction_payment::WeightInfo<Runtime>;
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
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpQueue = pezframe_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
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

impl teyrchain_info::Config for Runtime {}

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pezpallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezpallet_message_queue::WeightInfo<Runtime>;
	// Use the NoopMessageProcessor exclusively for benchmarks, not for tests with the
	// runtime-benchmarks feature as tests require the BridgeHubMessageRouter to process messages.
	// The "test" feature flag doesn't work, hence the reliance on the "std" feature, which is
	// enabled during tests.
	#[cfg(all(not(feature = "std"), feature = "runtime-benchmarks"))]
	type MessageProcessor =
		pezpallet_message_queue::mock_helpers::NoopMessageProcessor<AggregateMessageOrigin>;
	#[cfg(any(feature = "std", not(feature = "runtime-benchmarks")))]
	type MessageProcessor = pezbridge_hub_common::BridgeHubDualMessageRouter<
		xcm_builder::ProcessXcmMessage<
			AggregateMessageOrigin,
			xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
			RuntimeCall,
		>,
		EthereumOutboundQueue,
		EthereumOutboundQueueV2,
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
	pub FeeAssetId: AssetId = AssetId(xcm_config::ZagrosLocation::get());
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
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = weights::pezcumulus_pezpallet_xcmp_queue::WeightInfo<Runtime>;
	type PriceForSiblingDelivery = PriceForSiblingTeyrchainDelivery;
}

impl pezcumulus_pezpallet_xcmp_queue::migration::v5::V5Config for Runtime {
	// This must be the same as the `ChannelInfo` from the `Config`:
	type ChannelList = TeyrchainSystem;
}

parameter_types! {
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

pub const PERIOD: u32 = 6 * HOURS;
pub const OFFSET: u32 = 0;

impl pezpallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as pezframe_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pezpallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pezpallet_session::PeriodicSessions<ConstU32<PERIOD>, ConstU32<OFFSET>>;
	type NextSessionRotation =
		pezpallet_session::PeriodicSessions<ConstU32<PERIOD>, ConstU32<OFFSET>>;
	type SessionManager = CollatorSelection;
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
	pub const SessionLength: BlockNumber = 6 * HOURS;
}

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
	type KickThreshold = ConstU32<PERIOD>;
	type ValidatorId = <Self as pezframe_system::Config>::AccountId;
	type ValidatorIdOf = pezpallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = weights::pezpallet_collator_selection::WeightInfo<Runtime>;
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
	type WeightInfo = weights::pezpallet_multisig::WeightInfo<Runtime>;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

impl pezpallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pezpallet_utility::WeightInfo<Runtime>;
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
		WeightReclaim: pezcumulus_pezpallet_weight_reclaim = 4,

		// Monetary stuff.
		Balances: pezpallet_balances = 10,
		TransactionPayment: pezpallet_transaction_payment = 11,

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

		// Handy utilities.
		Utility: pezpallet_utility = 40,
		Multisig: pezpallet_multisig = 36,

		// Bridging stuff.
		BridgeRelayers: pezpallet_bridge_relayers = 41,
		BridgePezkuwichainGrandpa: pezpallet_bridge_grandpa::<Instance1> = 42,
		BridgePezkuwichainTeyrchains: pezpallet_bridge_teyrchains::<Instance1> = 43,
		BridgePezkuwichainMessages: pezpallet_bridge_messages::<Instance1> = 44,
		XcmOverBridgeHubPezkuwichain: pezpallet_xcm_bridge_hub::<Instance1> = 45,

		EthereumInboundQueue: pezsnowbridge_pezpallet_inbound_queue = 80,
		EthereumOutboundQueue: pezsnowbridge_pezpallet_outbound_queue = 81,
		EthereumBeaconClient: pezsnowbridge_pezpallet_ethereum_client = 82,
		EthereumSystem: pezsnowbridge_pezpallet_system = 83,

		EthereumSystemV2: pezsnowbridge_pezpallet_system_v2 = 90,
		EthereumInboundQueueV2: pezsnowbridge_pezpallet_inbound_queue_v2 = 91,
		EthereumOutboundQueueV2: pezsnowbridge_pezpallet_outbound_queue_v2 = 92,

		// Message Queue. Importantly, is registered last so that messages are processed after
		// the `on_initialize` hooks of bridging pallets.
		MessageQueue: pezpallet_message_queue = 250,
	}
);

pezbridge_runtime_common::generate_bridge_reject_obsolete_headers_and_messages! {
	RuntimeCall, AccountId,
	// Grandpa
	CheckAndBoostBridgeGrandpaTransactions<
		Runtime,
		bridge_to_pezkuwichain_config::BridgeGrandpaPezkuwichainInstance,
		bridge_to_pezkuwichain_config::PriorityBoostPerRelayHeader,
		xcm_config::TreasuryAccount,
	>,
	// Teyrchains
	CheckAndBoostBridgeTeyrchainsTransactions<
		Runtime,
		bridge_to_pezkuwichain_config::BridgeTeyrchainPezkuwichainInstance,
		pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichain,
		bridge_to_pezkuwichain_config::PriorityBoostPerTeyrchainHeader,
		xcm_config::TreasuryAccount,
	>,
	// Messages
	BridgePezkuwichainMessages
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	pezframe_benchmarking::define_benchmarks!(
		[pezframe_system, SystemBench::<Runtime>]
		[pezframe_system_extensions, SystemExtensionsBench::<Runtime>]
		[pezpallet_balances, Balances]
		[pezpallet_message_queue, MessageQueue]
		[pezpallet_multisig, Multisig]
		[pezpallet_session, SessionBench::<Runtime>]
		[pezpallet_utility, Utility]
		[pezpallet_timestamp, Timestamp]
		[pezpallet_transaction_payment, TransactionPayment]
		[pezpallet_collator_selection, CollatorSelection]
		[pezcumulus_pezpallet_teyrchain_system, TeyrchainSystem]
		[pezcumulus_pezpallet_xcmp_queue, XcmpQueue]
		// XCM
		[pezpallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
		// NOTE: Make sure you point to the individual modules below.
		[pezpallet_xcm_benchmarks::fungible, XcmBalances]
		[pezpallet_xcm_benchmarks::generic, XcmGeneric]
		// Bridge pallets
		[pezpallet_bridge_relayers, BridgeRelayersBench::<Runtime>]
		[pezpallet_bridge_grandpa, PezkuwichainFinality]
		[pezpallet_bridge_teyrchains, WithinPezkuwichain]
		[pezpallet_bridge_messages, ZagrosToPezkuwichain]
		// Ethereum Bridge V1
		[pezsnowbridge_pezpallet_system, EthereumSystem]
		[pezsnowbridge_pezpallet_ethereum_client, EthereumBeaconClient]
		[pezsnowbridge_pezpallet_inbound_queue, EthereumInboundQueue]
		[pezsnowbridge_pezpallet_outbound_queue, EthereumOutboundQueue]
		// Ethereum Bridge V2
		[pezsnowbridge_pezpallet_system_v2, EthereumSystemV2]
		[pezsnowbridge_pezpallet_inbound_queue_v2, EthereumInboundQueueV2]
		[pezsnowbridge_pezpallet_outbound_queue_v2, EthereumOutboundQueueV2]

		[pezcumulus_pezpallet_weight_reclaim, WeightReclaim]
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

	impl xcm_runtime_pezapis::fees::XcmPaymentApi<Block> for Runtime {
		fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {
			let acceptable_assets = vec![AssetId(xcm_config::ZagrosLocation::get())];
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

	impl pezcumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> pezcumulus_primitives_core::CollationInfo {
			TeyrchainSystem::collect_collation_info(header)
		}
	}

	impl pezbp_pezkuwichain::PezkuwichainFinalityApi<Block> for Runtime {
		fn best_finalized() -> Option<HeaderId<pezbp_pezkuwichain::Hash, pezbp_pezkuwichain::BlockNumber>> {
			BridgePezkuwichainGrandpa::best_finalized()
		}
		fn free_headers_interval() -> Option<pezbp_pezkuwichain::BlockNumber> {
			<Runtime as pezpallet_bridge_grandpa::Config<
				bridge_to_pezkuwichain_config::BridgeGrandpaPezkuwichainInstance
			>>::FreeHeadersInterval::get()
		}
		fn synced_headers_grandpa_info(
		) -> Vec<pezbp_header_pez_chain::StoredHeaderGrandpaInfo<pezbp_pezkuwichain::Header>> {
			BridgePezkuwichainGrandpa::synced_headers_grandpa_info()
		}
	}

	impl pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichainFinalityApi<Block> for Runtime {
		fn best_finalized() -> Option<HeaderId<Hash, BlockNumber>> {
			BridgePezkuwichainTeyrchains::best_teyrchain_head_id::<
				pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichain
			>().unwrap_or(None)
		}
		fn free_headers_interval() -> Option<pezbp_bridge_hub_pezkuwichain::BlockNumber> {
			// "free interval" is not currently used for teyrchains
			None
		}
	}

	impl pezbp_bridge_hub_pezkuwichain::FromBridgeHubPezkuwichainInboundLaneApi<Block> for Runtime {
		fn message_details(
			lane: LaneIdOf<Runtime, bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance>,
			messages: Vec<(pezbp_messages::MessagePayload, pezbp_messages::OutboundMessageDetails)>,
		) -> Vec<pezbp_messages::InboundMessageDetails> {
			pezbridge_runtime_common::messages_api::inbound_message_details::<
				Runtime,
				bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance,
			>(lane, messages)
		}
	}

	impl pezbp_bridge_hub_pezkuwichain::ToBridgeHubPezkuwichainOutboundLaneApi<Block> for Runtime {
		fn message_details(
			lane: LaneIdOf<Runtime, bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance>,
			begin: pezbp_messages::MessageNonce,
			end: pezbp_messages::MessageNonce,
		) -> Vec<pezbp_messages::OutboundMessageDetails> {
			pezbridge_runtime_common::messages_api::outbound_message_details::<
				Runtime,
				bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance,
			>(lane, begin, end)
		}
	}

	impl pezsnowbridge_outbound_queue_runtime_api::OutboundQueueApi<Block, Balance> for Runtime {
		fn prove_message(leaf_index: u64) -> Option<pezsnowbridge_merkle_tree::MerkleProof> {
			pezsnowbridge_pezpallet_outbound_queue::api::prove_message::<Runtime>(leaf_index)
		}

		fn calculate_fee(command: Command, parameters: Option<PricingParameters<Balance>>) -> Fee<Balance> {
			pezsnowbridge_pezpallet_outbound_queue::api::calculate_fee::<Runtime>(command, parameters)
		}
	}

	impl pezsnowbridge_outbound_queue_v2_runtime_api::OutboundQueueV2Api<Block, Balance> for Runtime {
		fn prove_message(leaf_index: u64) -> Option<pezsnowbridge_merkle_tree::MerkleProof> {
			pezsnowbridge_pezpallet_outbound_queue_v2::api::prove_message::<Runtime>(leaf_index)
		}
	}

	impl pezsnowbridge_system_runtime_api::ControlApi<Block> for Runtime {
		fn agent_id(location: VersionedLocation) -> Option<AgentId> {
			pezsnowbridge_pezpallet_system::api::agent_id::<Runtime>(location)
		}
	}

	impl pezsnowbridge_system_v2_runtime_api::ControlV2Api<Block> for Runtime {
		fn agent_id(location: VersionedLocation) -> Option<AgentId> {
			pezsnowbridge_pezpallet_system_v2::api::agent_id::<Runtime>(location)
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

			// This is defined once again in dispatch_benchmark, because list_benchmarks!
			// and add_benchmarks! are macros exported by define_benchmarks! macros and those types
			// are referenced in that call.
			type XcmBalances = pezpallet_xcm_benchmarks::fungible::Pezpallet::<Runtime>;
			type XcmGeneric = pezpallet_xcm_benchmarks::generic::Pezpallet::<Runtime>;

			use pezpallet_bridge_relayers::benchmarking::Pezpallet as BridgeRelayersBench;
			// Change weight file names.
			type PezkuwichainFinality = BridgePezkuwichainGrandpa;
			type WithinPezkuwichain = pezpallet_bridge_teyrchains::benchmarking::Pezpallet::<Runtime, bridge_to_pezkuwichain_config::BridgeTeyrchainPezkuwichainInstance>;
			type ZagrosToPezkuwichain = pezpallet_bridge_messages::benchmarking::Pezpallet ::<Runtime, bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance>;

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

			use xcm::latest::prelude::*;
			use xcm_config::ZagrosLocation;
			use testnet_teyrchains_constants::zagros::locations::{AssetHubParaId, AssetHubLocation};
			parameter_types! {
				pub ExistentialDepositAsset: Option<Asset> = Some((
					ZagrosLocation::get(),
					ExistentialDeposit::get()
				).into());
			}

			use pezpallet_xcm::benchmarking::Pezpallet as PalletXcmExtrinsicsBenchmark;
			impl pezpallet_xcm::benchmarking::Config for Runtime {
				type DeliveryHelper = pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
						xcm_config::XcmConfig,
						ExistentialDepositAsset,
						PriceForSiblingTeyrchainDelivery,
						AssetHubParaId,
						TeyrchainSystem,
					>;

				fn reachable_dest() -> Option<Location> {
					Some(AssetHubLocation::get())
				}

				fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
					// Relay/native token can be teleported between BH and Relay.
					Some((
						Asset {
							fun: Fungible(ExistentialDeposit::get()),
							id: AssetId(ZagrosLocation::get())
						},
						AssetHubLocation::get(),
					))
				}

				fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
					// Reserve transfers are disabled on BH.
					None
				}

				fn set_up_complex_asset_transfer(
				) -> Option<(Assets, AssetId, Location, alloc::boxed::Box<dyn FnOnce()>)> {
					// BH only supports teleports to system teyrchain.
					// Relay/native token can be teleported between BH and Relay.
					let native_location = ZagrosLocation::get();
					let dest = AssetHubLocation::get();
					pezpallet_xcm::benchmarking::helpers::native_teleport_as_asset_transfer::<Runtime>(
						native_location,
						dest
					)
				}

				fn get_asset() -> Asset {
					Asset {
						id: AssetId(Location::parent()),
						fun: Fungible(ExistentialDeposit::get()),
					}
				}
			}

			impl pezpallet_xcm_benchmarks::Config for Runtime {
				type XcmConfig = xcm_config::XcmConfig;
				type AccountIdConverter = xcm_config::LocationToAccountId;
				type DeliveryHelper = pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
						xcm_config::XcmConfig,
						ExistentialDepositAsset,
						PriceForSiblingTeyrchainDelivery,
						AssetHubParaId,
						TeyrchainSystem,
					>;
				fn valid_destination() -> Result<Location, BenchmarkError> {
					Ok(AssetHubLocation::get())
				}
				fn worst_case_holding(_depositable_count: u32) -> Assets {
					// just assets according to relay chain.
					let assets: Vec<Asset> = vec![
						Asset {
							id: AssetId(ZagrosLocation::get()),
							fun: Fungible(1_000_000 * UNITS),
						}
					];
					assets.into()
				}
			}

			parameter_types! {
				pub TrustedTeleporter: Option<(Location, Asset)> = Some((
					AssetHubLocation::get(),
					Asset { fun: Fungible(UNITS), id: AssetId(ZagrosLocation::get()) },
				));
				pub const CheckedAccount: Option<(AccountId, xcm_builder::MintLocation)> = None;
				pub const TrustedReserve: Option<(Location, Asset)> = None;
			}

			impl pezpallet_xcm_benchmarks::fungible::Config for Runtime {
				type TransactAsset = Balances;

				type CheckedAccount = CheckedAccount;
				type TrustedTeleporter = TrustedTeleporter;
				type TrustedReserve = TrustedReserve;

				fn get_asset() -> Asset {
					Asset {
						id: AssetId(ZagrosLocation::get()),
						fun: Fungible(UNITS),
					}
				}
			}

			impl pezpallet_xcm_benchmarks::generic::Config for Runtime {
				type TransactAsset = Balances;
				type RuntimeCall = RuntimeCall;

				fn worst_case_response() -> (u64, Response) {
					(0u64, Response::Version(Default::default()))
				}

				fn worst_case_asset_exchange() -> Result<(Assets, Assets), BenchmarkError> {
					Err(BenchmarkError::Skip)
				}

				fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
					Err(BenchmarkError::Skip)
				}

				fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
					Ok((AssetHubLocation::get(), pezframe_system::Call::remark_with_event { remark: vec![] }.into()))
				}

				fn subscribe_origin() -> Result<Location, BenchmarkError> {
					Ok(AssetHubLocation::get())
				}

				fn claimable_asset() -> Result<(Location, Location, Assets), BenchmarkError> {
					let origin = AssetHubLocation::get();
					let assets: Assets = (AssetId(ZagrosLocation::get()), 1_000 * UNITS).into();
					let ticket = Location { parents: 0, interior: Here };
					Ok((origin, ticket, assets))
				}

				fn worst_case_for_trader() -> Result<(Asset, WeightLimit), BenchmarkError> {
					Ok((Asset {
						id: AssetId(ZagrosLocation::get()),
						fun: Fungible(1_000_000 * UNITS),
					}, WeightLimit::Limited(Weight::from_parts(5000, 5000))))
				}

				fn unlockable_asset() -> Result<(Location, Location, Asset), BenchmarkError> {
					Err(BenchmarkError::Skip)
				}

				fn export_message_origin_and_destination(
				) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
					// save XCM version for remote bridge hub
					let _ = PezkuwiXcm::force_xcm_version(
						RuntimeOrigin::root(),
						alloc::boxed::Box::new(bridge_to_pezkuwichain_config::BridgeHubPezkuwichainLocation::get()),
						XCM_VERSION,
					).map_err(|e| {
						let origin = RuntimeOrigin::root();
						let bridge = bridge_to_pezkuwichain_config::BridgeHubPezkuwichainLocation::get();
						tracing::error!(
							target: "xcm::export_message_origin_and_destination",
							?origin,
							?bridge,
							?XCM_VERSION,
							?e,
							"Failed to dispatch `force_xcm_version`",
						);
						BenchmarkError::Stop("XcmVersion was not stored!")
					})?;

					let sibling_teyrchain_location = Location::new(1, [Teyrchain(5678)]);

					// fund SA
					use pezframe_support::traits::fungible::Mutate;
					use xcm_executor::traits::ConvertLocation;
					pezframe_support::assert_ok!(
						Balances::mint_into(
							&xcm_config::LocationToAccountId::convert_location(&sibling_teyrchain_location).expect("valid AccountId"),
							bridge_to_pezkuwichain_config::BridgeDeposit::get()
								.saturating_add(ExistentialDeposit::get())
								.saturating_add(UNITS * 5)
						)
					);

					// open bridge
					let bridge_destination_universal_location: InteriorLocation = [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)), Teyrchain(8765)].into();
					let locations = XcmOverBridgeHubPezkuwichain::bridge_locations(
						sibling_teyrchain_location.clone(),
						bridge_destination_universal_location.clone(),
					)?;
					XcmOverBridgeHubPezkuwichain::do_open_bridge(
						locations,
						pezbp_messages::LegacyLaneId([1, 2, 3, 4]),
						true,
					).map_err(|e| {
						tracing::error!(
							target: "xcm::export_message_origin_and_destination",
							?sibling_teyrchain_location,
							?bridge_destination_universal_location,
							?e,
							"Failed to `XcmOverBridgeHubPezkuwichain::open_bridge`",
						);
						BenchmarkError::Stop("Bridge was not opened!")
					})?;

					Ok(
						(
							sibling_teyrchain_location,
							NetworkId::ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
							[Teyrchain(8765)].into()
						)
					)
				}

				fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
					// Any location can alias to an internal location.
					// Here teyrchain 1000 aliases to an internal account.
					let origin = Location::new(1, [Teyrchain(1000)]);
					let target = Location::new(1, [Teyrchain(1000), AccountId32 { id: [128u8; 32], network: None }]);
					Ok((origin, target))
				}
			}

			type XcmBalances = pezpallet_xcm_benchmarks::fungible::Pezpallet::<Runtime>;
			type XcmGeneric = pezpallet_xcm_benchmarks::generic::Pezpallet::<Runtime>;

			type PezkuwichainFinality = BridgePezkuwichainGrandpa;
			type WithinPezkuwichain = pezpallet_bridge_teyrchains::benchmarking::Pezpallet::<Runtime, bridge_to_pezkuwichain_config::BridgeTeyrchainPezkuwichainInstance>;
			type ZagrosToPezkuwichain = pezpallet_bridge_messages::benchmarking::Pezpallet ::<Runtime, bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance>;

			use pezbridge_runtime_common::messages_benchmarking::{
				prepare_message_delivery_proof_from_teyrchain,
				prepare_message_proof_from_teyrchain,
				generate_xcm_builder_bridge_message_sample,
			};
			use pezpallet_bridge_messages::benchmarking::{
				Config as BridgeMessagesConfig,
				MessageDeliveryProofParams,
				MessageProofParams,
			};

			impl BridgeMessagesConfig<bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance> for Runtime {
				fn is_relayer_rewarded(relayer: &Self::AccountId) -> bool {
					let bench_lane_id = <Self as BridgeMessagesConfig<bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance>>::bench_lane_id();
					use pezbp_runtime::Chain;
					let bridged_chain_id =<Self as pezpallet_bridge_messages::Config<bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance>>::BridgedChain::ID;
					pezpallet_bridge_relayers::Pezpallet::<Runtime, bridge_common_config::BridgeRelayersInstance>::relayer_reward(
						relayer,
						bridge_common_config::BridgeReward::PezkuwichainZagros(
							pezbp_relayers::RewardsAccountParams::new(
								bench_lane_id,
								bridged_chain_id,
								pezbp_relayers::RewardsAccountOwner::BridgedChain
							)
						)
					).is_some()
				}

				fn prepare_message_proof(
					params: MessageProofParams<LaneIdOf<Runtime, bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance>>,
				) -> (bridge_to_pezkuwichain_config::FromPezkuwichainBridgeHubMessagesProof<bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance>, Weight) {
					use pezcumulus_primitives_core::XcmpMessageSource;
					assert!(XcmpQueue::take_outbound_messages(usize::MAX).is_empty());
					TeyrchainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(42.into());
					let universal_source = bridge_to_pezkuwichain_config::open_bridge_for_benchmarks::<
						Runtime,
						bridge_to_pezkuwichain_config::XcmOverBridgeHubPezkuwichainInstance,
						xcm_config::LocationToAccountId,
					>(params.lane, 42);
					prepare_message_proof_from_teyrchain::<
						Runtime,
						bridge_to_pezkuwichain_config::BridgeGrandpaPezkuwichainInstance,
						bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance,
					>(params, generate_xcm_builder_bridge_message_sample(universal_source))
				}

				fn prepare_message_delivery_proof(
					params: MessageDeliveryProofParams<AccountId, LaneIdOf<Runtime, bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance>>,
				) -> bridge_to_pezkuwichain_config::ToPezkuwichainBridgeHubMessagesDeliveryProof<bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance> {
					let _ = bridge_to_pezkuwichain_config::open_bridge_for_benchmarks::<
						Runtime,
						bridge_to_pezkuwichain_config::XcmOverBridgeHubPezkuwichainInstance,
						xcm_config::LocationToAccountId,
					>(params.lane, 42);
					prepare_message_delivery_proof_from_teyrchain::<
						Runtime,
						bridge_to_pezkuwichain_config::BridgeGrandpaPezkuwichainInstance,
						bridge_to_pezkuwichain_config::WithBridgeHubPezkuwichainMessagesInstance,
					>(params)
				}

				fn is_message_successfully_dispatched(_nonce: pezbp_messages::MessageNonce) -> bool {
					use pezcumulus_primitives_core::XcmpMessageSource;
					!XcmpQueue::take_outbound_messages(usize::MAX).is_empty()
				}
			}

			use pezbridge_runtime_common::teyrchains_benchmarking::prepare_teyrchain_heads_proof;
			use pezpallet_bridge_teyrchains::benchmarking::Config as BridgeTeyrchainsConfig;
			use pezpallet_bridge_relayers::benchmarking::{
				Pezpallet as BridgeRelayersBench,
				Config as BridgeRelayersConfig,
			};

			impl BridgeTeyrchainsConfig<bridge_to_pezkuwichain_config::BridgeTeyrchainPezkuwichainInstance> for Runtime {
				fn teyrchains() -> Vec<pezbp_pezkuwi_core::teyrchains::ParaId> {
					use pezbp_runtime::Teyrchain;
					vec![pezbp_pezkuwi_core::teyrchains::ParaId(pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichain::TEYRCHAIN_ID)]
				}

				fn prepare_teyrchain_heads_proof(
					teyrchains: &[pezbp_pezkuwi_core::teyrchains::ParaId],
					teyrchain_head_size: u32,
					proof_params: pezbp_runtime::UnverifiedStorageProofParams,
				) -> (
					pezbp_teyrchains::RelayBlockNumber,
					pezbp_teyrchains::RelayBlockHash,
					pezbp_pezkuwi_core::teyrchains::ParaHeadsProof,
					Vec<(pezbp_pezkuwi_core::teyrchains::ParaId, pezbp_pezkuwi_core::teyrchains::ParaHash)>,
				) {
					prepare_teyrchain_heads_proof::<Runtime, bridge_to_pezkuwichain_config::BridgeTeyrchainPezkuwichainInstance>(
						teyrchains,
						teyrchain_head_size,
						proof_params,
					)
				}
			}

			impl BridgeRelayersConfig<bridge_common_config::BridgeRelayersInstance> for Runtime {
				fn bench_reward() -> Self::Reward {
					pezbp_relayers::RewardsAccountParams::new(
						pezbp_messages::LegacyLaneId::default(),
						*b"test",
						pezbp_relayers::RewardsAccountOwner::ThisChain
					).into()
				}

				fn prepare_rewards_account(
					reward_kind: Self::Reward,
					reward: Balance,
				) -> Option<pezpallet_bridge_relayers::BeneficiaryOf<Runtime, bridge_common_config::BridgeRelayersInstance>> {
					let bridge_common_config::BridgeReward::PezkuwichainZagros(reward_kind) = reward_kind else {
						panic!("Unexpected reward_kind: {:?} - not compatible with `bench_reward`!", reward_kind);
					};
					let rewards_account = pezbp_relayers::PayRewardFromAccount::<
						Balances,
						AccountId,
						pezbp_messages::LegacyLaneId,
						u128,
					>::rewards_account(reward_kind);
					Self::deposit_account(rewards_account, reward);

					None
				}

				fn deposit_account(account: AccountId, balance: Balance) {
					use pezframe_support::traits::fungible::Mutate;
					Balances::mint_into(&account, balance.saturating_add(ExistentialDeposit::get())).unwrap();
				}
			}

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
}

pezcumulus_pezpallet_teyrchain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = pezcumulus_pezpallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;
	use pezsp_runtime::{
		generic::Era,
		traits::{TransactionExtension, Zero},
	};

	#[test]
	fn ensure_transaction_extension_definition_is_compatible_with_relay() {
		use pezbp_pezkuwi_core::SuffixedCommonTransactionExtensionExt;

		pezsp_io::TestExternalities::default().execute_with(|| {
			pezframe_system::BlockHash::<Runtime>::insert(BlockNumber::zero(), Hash::default());
			let payload: TxExtension = (
				(
					pezframe_system::AuthorizeCall::<Runtime>::new(),
					pezframe_system::CheckNonZeroSender::new(),
					pezframe_system::CheckSpecVersion::new(),
					pezframe_system::CheckTxVersion::new(),
					pezframe_system::CheckGenesis::new(),
					pezframe_system::CheckEra::from(Era::Immortal),
					pezframe_system::CheckNonce::from(10),
					pezframe_system::CheckWeight::new(),
				),
				pezpallet_transaction_payment::ChargeTransactionPayment::from(10),
				BridgeRejectObsoleteHeadersAndMessages,
				(
					bridge_to_pezkuwichain_config::OnBridgeHubZagrosRefundBridgeHubPezkuwichainMessages::default(),
				),
				pezframe_metadata_hash_extension::CheckMetadataHash::new(false),
			).into();

			{
				let bh_indirect_payload =
					pezbp_bridge_hub_zagros::TransactionExtension::from_params(
						VERSION.spec_version,
						VERSION.transaction_version,
						pezbp_runtime::TransactionEra::Immortal,
						System::block_hash(BlockNumber::zero()),
						10,
						10,
						(((), ()), ((), ())),
					);
				assert_eq!(payload.encode().split_last().unwrap().1, bh_indirect_payload.encode());
				assert_eq!(
					TxExtension::implicit(&payload).unwrap().encode().split_last().unwrap().1,
					pezsp_runtime::traits::TransactionExtension::<RuntimeCall>::implicit(
						&bh_indirect_payload
					)
					.unwrap()
					.encode()
				)
			}
		});
	}
}
