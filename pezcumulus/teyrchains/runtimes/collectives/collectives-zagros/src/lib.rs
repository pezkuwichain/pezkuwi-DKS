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

//! # Collectives Teyrchain
//!
//! This teyrchain is for collectives that serve the Zagros network.
//! Each collective is defined by a specialized (possibly instanced) pezpallet.
//!
//! ### Governance
//!
//! As a system teyrchain, Collectives defers its governance (namely, its `Root` origin), to
//! its Relay Chain parent, Zagros.
//!
//! ### Collator Selection
//!
//! Collectives uses `pezpallet-collator-selection`, a simple first-come-first-served registration
//! system where collators can reserve a small bond to join the block producer set. There is no
//! slashing. Collective members are generally expected to run collators.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod ambassador;
mod genesis_config_presets;
pub mod impls;
mod weights;
pub mod xcm_config;
// Fellowship configurations.
pub mod fellowship;

// Secretary Configuration
pub mod secretary;

extern crate alloc;

pub use ambassador::pezpallet_ambassador_origins;

use alloc::{vec, vec::Vec};
use ambassador::AmbassadorCoreInstance;
use fellowship::{pezpallet_fellowship_origins, Fellows, FellowshipCoreInstance};
use impls::{AllianceProposalProvider, EqualOrGreatestRootCmp};
use pezcumulus_pezpallet_teyrchain_system::RelayNumberMonotonicallyIncreases;
use pezsp_api::impl_runtime_apis;
use pezsp_core::{crypto::KeyTypeId, OpaqueMetadata};
use pezsp_runtime::{
	generic, impl_opaque_keys,
	traits::{AccountIdConversion, BlakeTwo256, Block as BlockT},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, Perbill,
};

#[cfg(feature = "std")]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pezcumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use pezframe_support::{
	construct_runtime, derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{
		fungible::HoldConsideration, ConstBool, ConstU32, ConstU64, ConstU8, EitherOfDiverse,
		InstanceFilter, LinearStoragePrice, TransformOrigin,
	},
	weights::{ConstantMultiplier, Weight},
	PalletId,
};
use pezframe_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
use pezsp_runtime::RuntimeDebug;
use testnet_teyrchains_constants::zagros::{
	account::*, consensus::*, currency::*, fee::WeightToFee, time::*,
};
pub use teyrchains_common as common;
use teyrchains_common::{
	impls::{DealWithFees, ToParentTreasury},
	message_queue::*,
	AccountId, AuraId, Balance, BlockNumber, Hash, Header, Nonce, Signature,
	AVERAGE_ON_INITIALIZE_RATIO, NORMAL_DISPATCH_RATIO,
};
use xcm_config::{
	GovernanceLocation, LocationToAccountId, TreasurerBodyId, XcmConfig,
	XcmOriginToTransactDispatchOrigin,
};

#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;

// Pezkuwi imports
use pezkuwi_runtime_common::{
	impls::VersionedLocatableAsset, BlockHashCount, SlowAdjustingFeeUpdate,
};
use pezpallet_xcm::{EnsureXcm, IsVoiceOfBody};
use xcm::{prelude::*, Version as XcmVersion};
use xcm_runtime_pezapis::{
	dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
	fees::Error as XcmPaymentApiError,
};

use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("collectives-zagros"),
	impl_name: alloc::borrow::Cow::Borrowed("collectives-zagros"),
	authoring_version: 1,
	spec_version: 1_020_001,
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

/// Privileged origin that represents Root or more than two thirds of the Alliance.
pub type RootOrAllianceTwoThirdsMajority = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pezpallet_collective::EnsureProportionMoreThan<AccountId, AllianceCollective, 2, 3>,
>;

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
	type RuntimeCall = RuntimeCall;
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
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightInfo = weights::pezpallet_transaction_payment::WeightInfo<Runtime>;
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

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 40);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	// One storage item; key size 32, value size 16
	pub const AnnouncementDepositBase: Balance = deposit(1, 48);
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
	/// Fully permissioned proxy. Can execute any call on behalf of _proxied_.
	Any,
	/// Can execute any call that does not transfer funds.
	NonTransfer,
	/// Proxy with the ability to reject time-delay proxy announcements.
	CancelProxy,
	/// Collator selection proxy. Can execute calls related to collator selection mechanism.
	Collator,
	/// Alliance proxy. Allows calls related to the Alliance.
	Alliance,
	/// Fellowship proxy. Allows calls related to the Fellowship.
	Fellowship,
	/// Ambassador proxy. Allows calls related to the Ambassador Program.
	Ambassador,
	/// Secretary proxy. Allows calls related to the Secretary collective
	Secretary,
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
			ProxyType::NonTransfer => !matches!(c, RuntimeCall::Balances { .. }),
			ProxyType::CancelProxy => matches!(
				c,
				RuntimeCall::Proxy(pezpallet_proxy::Call::reject_announcement { .. })
					| RuntimeCall::Utility { .. }
					| RuntimeCall::Multisig { .. }
			),
			ProxyType::Collator => matches!(
				c,
				RuntimeCall::CollatorSelection { .. }
					| RuntimeCall::Utility { .. }
					| RuntimeCall::Multisig { .. }
			),
			ProxyType::Alliance => matches!(
				c,
				RuntimeCall::AllianceMotion { .. }
					| RuntimeCall::Alliance { .. }
					| RuntimeCall::Utility { .. }
					| RuntimeCall::Multisig { .. }
			),
			ProxyType::Fellowship => matches!(
				c,
				RuntimeCall::FellowshipCollective { .. }
					| RuntimeCall::FellowshipReferenda { .. }
					| RuntimeCall::FellowshipCore { .. }
					| RuntimeCall::FellowshipSalary { .. }
					| RuntimeCall::FellowshipTreasury { .. }
					| RuntimeCall::Utility { .. }
					| RuntimeCall::Multisig { .. }
			),
			ProxyType::Ambassador => matches!(
				c,
				RuntimeCall::AmbassadorCollective { .. }
					| RuntimeCall::AmbassadorReferenda { .. }
					| RuntimeCall::AmbassadorContent { .. }
					| RuntimeCall::AmbassadorCore { .. }
					| RuntimeCall::AmbassadorSalary { .. }
					| RuntimeCall::Utility { .. }
					| RuntimeCall::Multisig { .. }
			),
			ProxyType::Secretary => matches!(
				c,
				RuntimeCall::SecretaryCollective { .. }
					| RuntimeCall::SecretarySalary { .. }
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
	type WeightInfo = weights::pezpallet_proxy::WeightInfo<Runtime>;
	type MaxPending = ConstU32<32>;
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

impl teyrchain_info::Config for Runtime {}

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

impl pezcumulus_pezpallet_aura_ext::Config for Runtime {}

parameter_types! {
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(xcm_config::WndLocation::get());
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
	// Enqueue XCMP messages from siblings for later processing.
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = ConstU32<1_000>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	// Most on-chain HRMP channels are configured to use 102400 bytes of max message size, so we
	// need to set the page size larger than that until we reduce the channel size on-chain.
	type MaxPageSize = ConstU32<{ 103 * 1024 }>;
	type ControllerOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Fellows>;
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
	// `StakingAdmin` pluralistic body.
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
	type KickThreshold = ConstU32<PERIOD>;
	type ValidatorId = <Self as pezframe_system::Config>::AccountId;
	type ValidatorIdOf = pezpallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = weights::pezpallet_collator_selection::WeightInfo<Runtime>;
}

pub const ALLIANCE_MOTION_DURATION: BlockNumber = 5 * DAYS;

parameter_types! {
	pub const AllianceMotionDuration: BlockNumber = ALLIANCE_MOTION_DURATION;
	pub MaxProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}
pub const ALLIANCE_MAX_PROPOSALS: u32 = 100;
pub const ALLIANCE_MAX_MEMBERS: u32 = 100;

type AllianceCollective = pezpallet_collective::Instance1;
impl pezpallet_collective::Config<AllianceCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = AllianceMotionDuration;
	type MaxProposals = ConstU32<ALLIANCE_MAX_PROPOSALS>;
	type MaxMembers = ConstU32<ALLIANCE_MAX_MEMBERS>;
	type DefaultVote = pezpallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pezpallet_collective::WeightInfo<Runtime>;
	type MaxProposalWeight = MaxProposalWeight;
	type DisapproveOrigin = EnsureRoot<Self::AccountId>;
	type KillOrigin = EnsureRoot<Self::AccountId>;
	type Consideration = ();
}

pub const MAX_FELLOWS: u32 = ALLIANCE_MAX_MEMBERS;
pub const MAX_ALLIES: u32 = 100;

parameter_types! {
	pub const AllyDeposit: Balance = 1_000 * UNITS; // 1,000 ZGR bond to join as an Ally
	pub ZagrosTreasuryAccount: AccountId = ZAGROS_TREASURY_PALLET_ID.into_account_truncating();
	// The number of blocks a member must wait between giving a retirement notice and retiring.
	// Supposed to be greater than time required to `kick_member` with alliance motion.
	pub const AllianceRetirementPeriod: BlockNumber = (90 * DAYS) + ALLIANCE_MOTION_DURATION;
}

impl pezpallet_alliance::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Proposal = RuntimeCall;
	type AdminOrigin = RootOrAllianceTwoThirdsMajority;
	type MembershipManager = RootOrAllianceTwoThirdsMajority;
	type AnnouncementOrigin = RootOrAllianceTwoThirdsMajority;
	type Currency = Balances;
	type Slashed = ToParentTreasury<ZagrosTreasuryAccount, LocationToAccountId, Runtime>;
	type InitializeMembers = AllianceMotion;
	type MembershipChanged = AllianceMotion;
	type RetirementPeriod = AllianceRetirementPeriod;
	type IdentityVerifier = (); // Don't block accounts on identity criteria
	type ProposalProvider = AllianceProposalProvider<Runtime, AllianceCollective>;
	type MaxProposals = ConstU32<ALLIANCE_MAX_MEMBERS>;
	type MaxFellows = ConstU32<MAX_FELLOWS>;
	type MaxAllies = ConstU32<MAX_ALLIES>;
	type MaxUnscrupulousItems = ConstU32<100>;
	type MaxWebsiteUrlLength = ConstU32<255>;
	type MaxAnnouncementsCount = ConstU32<100>;
	type MaxMembersCount = ConstU32<ALLIANCE_MAX_MEMBERS>;
	type AllyDeposit = AllyDeposit;
	type WeightInfo = weights::pezpallet_alliance::WeightInfo<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
}

#[cfg(not(feature = "runtime-benchmarks"))]
parameter_types! {
	pub const MaxScheduledPerBlock: u32 = 50;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub const MaxScheduledPerBlock: u32 = 200;
}

impl pezpallet_scheduler::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = weights::pezpallet_scheduler::WeightInfo<Runtime>;
	type OriginPrivilegeCmp = EqualOrGreatestRootCmp;
	type Preimages = Preimage;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

parameter_types! {
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pezpallet_preimage::HoldReason::Preimage);
}

impl pezpallet_preimage::Config for Runtime {
	type WeightInfo = weights::pezpallet_preimage::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
}

impl pezpallet_asset_rate::Config for Runtime {
	type WeightInfo = weights::pezpallet_asset_rate::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type CreateOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		EitherOfDiverse<EnsureXcm<IsVoiceOfBody<GovernanceLocation, TreasurerBodyId>>, Fellows>,
	>;
	type RemoveOrigin = Self::CreateOrigin;
	type UpdateOrigin = Self::CreateOrigin;
	type Currency = Balances;
	type AssetKind = VersionedLocatableAsset;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pezkuwi_runtime_common::impls::benchmarks::AssetRateArguments;
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
		Preimage: pezpallet_preimage = 43,
		Scheduler: pezpallet_scheduler = 44,
		AssetRate: pezpallet_asset_rate = 45,

		// The main stage.

		// The Alliance.
		Alliance: pezpallet_alliance = 50,
		AllianceMotion: pezpallet_collective::<Instance1> = 51,

		// The Fellowship.
		// pub type FellowshipCollectiveInstance = pezpallet_ranked_collective::Instance1;
		FellowshipCollective: pezpallet_ranked_collective::<Instance1> = 60,
		// pub type FellowshipReferendaInstance = pezpallet_referenda::Instance1;
		FellowshipReferenda: pezpallet_referenda::<Instance1> = 61,
		FellowshipOrigins: pezpallet_fellowship_origins = 62,
		// pub type FellowshipCoreInstance = pezpallet_core_fellowship::Instance1;
		FellowshipCore: pezpallet_core_fellowship::<Instance1> = 63,
		// pub type FellowshipSalaryInstance = pezpallet_salary::Instance1;
		FellowshipSalary: pezpallet_salary::<Instance1> = 64,
		// pub type FellowshipTreasuryInstance = pezpallet_treasury::Instance1;
		FellowshipTreasury: pezpallet_treasury::<Instance1> = 65,

		// Ambassador Program.
		AmbassadorCollective: pezpallet_ranked_collective::<Instance2> = 70,
		AmbassadorReferenda: pezpallet_referenda::<Instance2> = 71,
		AmbassadorOrigins: pezpallet_ambassador_origins = 72,
		AmbassadorCore: pezpallet_core_fellowship::<Instance2> = 73,
		AmbassadorSalary: pezpallet_salary::<Instance2> = 74,
		AmbassadorContent: pezpallet_collective_content::<Instance1> = 75,

		StateTrieMigration: pezpallet_state_trie_migration = 80,

		// The Secretary Collective
		// pub type SecretaryCollectiveInstance = pezpallet_ranked_collective::instance3;
		SecretaryCollective: pezpallet_ranked_collective::<Instance3> = 90,
		// pub type SecretarySalaryInstance = pezpallet_salary::Instance3;
		SecretarySalary: pezpallet_salary::<Instance3> = 91,
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
		pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
	),
>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
/// All migrations executed on runtime upgrade as a nested tuple of types implementing
/// `OnRuntimeUpgrade`. Included migrations must be idempotent.
type Migrations = (
	// unreleased
	pezpallet_collator_selection::migration::v2::MigrationToV2<Runtime>,
	// unreleased
	pezcumulus_pezpallet_xcmp_queue::migration::v4::MigrationToV4<Runtime>,
	pezcumulus_pezpallet_xcmp_queue::migration::v5::MigrateV4ToV5<Runtime>,
	// permanent
	pezpallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
	// unreleased
	pezpallet_core_fellowship::migration::MigrateV0ToV1<Runtime, FellowshipCoreInstance>,
	// unreleased
	pezpallet_core_fellowship::migration::MigrateV0ToV1<Runtime, AmbassadorCoreInstance>,
	pezcumulus_pezpallet_aura_ext::migration::MigrateV0ToV1<Runtime>,
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

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	pezframe_benchmarking::define_benchmarks!(
		[pezframe_system, SystemBench::<Runtime>]
		[pezframe_system_extensions, SystemExtensionsBench::<Runtime>]
		[pezpallet_balances, Balances]
		[pezpallet_message_queue, MessageQueue]
		[pezpallet_multisig, Multisig]
		[pezpallet_proxy, Proxy]
		[pezpallet_session, SessionBench::<Runtime>]
		[pezpallet_utility, Utility]
		[pezpallet_timestamp, Timestamp]
		[pezpallet_transaction_payment, TransactionPayment]
		[pezpallet_collator_selection, CollatorSelection]
		[pezcumulus_pezpallet_teyrchain_system, TeyrchainSystem]
		[pezcumulus_pezpallet_xcmp_queue, XcmpQueue]
		[pezpallet_alliance, Alliance]
		[pezpallet_collective, AllianceMotion]
		[pezpallet_preimage, Preimage]
		[pezpallet_scheduler, Scheduler]
		[pezpallet_referenda, FellowshipReferenda]
		[pezpallet_ranked_collective, FellowshipCollective]
		[pezpallet_core_fellowship, FellowshipCore]
		[pezpallet_salary, FellowshipSalary]
		[pezpallet_treasury, FellowshipTreasury]
		[pezpallet_referenda, AmbassadorReferenda]
		[pezpallet_ranked_collective, AmbassadorCollective]
		[pezpallet_collective_content, AmbassadorContent]
		[pezpallet_core_fellowship, AmbassadorCore]
		[pezpallet_salary, AmbassadorSalary]
		[pezpallet_ranked_collective, SecretaryCollective]
		[pezpallet_salary, SecretarySalary]
		[pezpallet_asset_rate, AssetRate]
		[pezcumulus_pezpallet_weight_reclaim, WeightReclaim]
		// XCM
		[pezpallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
		// NOTE: Make sure you point to the individual modules below.
		[pezpallet_xcm_benchmarks::fungible, XcmBalances]
		[pezpallet_xcm_benchmarks::generic, XcmGeneric]
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
			let acceptable_assets = vec![AssetId(xcm_config::WndLocation::get())];
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
				LocationToAccountId,
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
			use xcm_config::WndLocation;
			use testnet_teyrchains_constants::zagros::locations::{AssetHubParaId, AssetHubLocation};

			parameter_types! {
				pub ExistentialDepositAsset: Option<Asset> = Some((
					WndLocation::get(),
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
					// Relay/native token can be teleported between Collectives and Relay.
					Some((
						Asset {
							fun: Fungible(ExistentialDeposit::get()),
							id: AssetId(WndLocation::get())
						}.into(),
						AssetHubLocation::get(),
					))
				}

				fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
					// Reserve transfers are disabled on Collectives.
					None
				}

				fn set_up_complex_asset_transfer(
				) -> Option<(Assets, AssetId, Location, alloc::boxed::Box<dyn FnOnce()>)> {
					// Collectives only supports teleports to system teyrchain.
					// Relay/native token can be teleported between Collectives and Relay.
					let native_location = WndLocation::get();
					let dest = AssetHubLocation::get();
					pezpallet_xcm::benchmarking::helpers::native_teleport_as_asset_transfer::<Runtime>(
						native_location,
						dest
					)
				}

				fn get_asset() -> Asset {
					Asset {
						id: AssetId(WndLocation::get()),
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
						TeyrchainSystem
					>;
				fn valid_destination() -> Result<Location, BenchmarkError> {
					Ok(AssetHubLocation::get())
				}
				fn worst_case_holding(_depositable_count: u32) -> Assets {
					// just concrete assets according to relay chain.
					let assets: Vec<Asset> = vec![
						Asset {
							id: AssetId(WndLocation::get()),
							fun: Fungible(1_000_000 * UNITS),
						}
					];
					assets.into()
				}
			}

			parameter_types! {
				pub TrustedTeleporter: Option<(Location, Asset)> = Some((
					AssetHubLocation::get(),
					Asset { fun: Fungible(UNITS), id: AssetId(WndLocation::get()) },
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
						id: AssetId(WndLocation::get()),
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
					let assets: Assets = (AssetId(WndLocation::get()), 1_000 * UNITS).into();
					let ticket = Location { parents: 0, interior: Here };
					Ok((origin, ticket, assets))
				}

				fn worst_case_for_trader() -> Result<(Asset, WeightLimit), BenchmarkError> {
					Ok((Asset {
						id: AssetId(WndLocation::get()),
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
					// Here teyrchain 1000 aliases to an internal account.
					let origin = Location::new(1, [Teyrchain(1000)]);
					let target = Location::new(1, [Teyrchain(1000), AccountId32 { id: [128u8; 32], network: None }]);
					Ok((origin, target))
				}
			}

			type XcmBalances = pezpallet_xcm_benchmarks::fungible::Pezpallet::<Runtime>;
			type XcmGeneric = pezpallet_xcm_benchmarks::generic::Pezpallet::<Runtime>;

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

parameter_types! {
	// The deposit configuration for the singed migration. Specially if you want to allow any signed account to do the migration (see `SignedFilter`, these deposits should be high)
	pub const MigrationSignedDepositPerItem: Balance = CENTS;
	pub const MigrationSignedDepositBase: Balance = 2_000 * CENTS;
	pub const MigrationMaxKeyLen: u32 = 512;
}

impl pezpallet_state_trie_migration::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type SignedDepositPerItem = MigrationSignedDepositPerItem;
	type SignedDepositBase = MigrationSignedDepositBase;
	// An origin that can control the whole pezpallet: should be Root, or a part of your council.
	type ControlOrigin = pezframe_system::EnsureSignedBy<RootMigController, AccountId>;
	// specific account for the migration, can trigger the signed migrations.
	type SignedFilter = pezframe_system::EnsureSignedBy<MigController, AccountId>;

	// Replace this with weight based on your runtime.
	type WeightInfo = pezpallet_state_trie_migration::weights::BizinikiwiWeight<Runtime>;

	type MaxKeyLen = MigrationMaxKeyLen;
}

pezframe_support::ord_parameter_types! {
	pub const MigController: AccountId = AccountId::from(hex_literal::hex!("8458ed39dc4b6f6c7255f7bc42be50c2967db126357c999d44e12ca7ac80dc52"));
	pub const RootMigController: AccountId = AccountId::from(hex_literal::hex!("8458ed39dc4b6f6c7255f7bc42be50c2967db126357c999d44e12ca7ac80dc52"));
}

#[test]
fn ensure_key_ss58() {
	use pezframe_support::traits::SortedMembers;
	use pezsp_core::crypto::Ss58Codec;
	let acc =
		AccountId::from_ss58check("5F4EbSkZz18X36xhbsjvDNs6NuZ82HyYtq5UiJ1h9SBHJXZD").unwrap();
	assert_eq!(acc, MigController::sorted_members()[0]);
	let acc =
		AccountId::from_ss58check("5F4EbSkZz18X36xhbsjvDNs6NuZ82HyYtq5UiJ1h9SBHJXZD").unwrap();
	assert_eq!(acc, RootMigController::sorted_members()[0]);
}
