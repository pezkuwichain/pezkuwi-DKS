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

//! The Pezkuwi runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

extern crate alloc;

use alloc::{
	collections::{btree_map::BTreeMap, vec_deque::VecDeque},
	vec,
	vec::Vec,
};
use codec::Encode;
use pezpallet_transaction_payment::FungibleAdapter;

use pezkuwi_runtime_teyrchains::{
	assigner_coretime as teyrchains_assigner_coretime, configuration as teyrchains_configuration,
	configuration::ActiveConfigHrmpChannelSizeAndCapacityRatio,
	coretime, disputes as teyrchains_disputes,
	disputes::slashing as teyrchains_slashing,
	dmp as teyrchains_dmp, hrmp as teyrchains_hrmp, inclusion as teyrchains_inclusion,
	initializer as teyrchains_initializer, on_demand as teyrchains_on_demand,
	origin as teyrchains_origin, paras as teyrchains_paras,
	paras_inherent as teyrchains_paras_inherent,
	runtime_api_impl::{v13 as runtime_impl, vstaging as pezstaging_runtime_impl},
	scheduler as teyrchains_scheduler, session_info as teyrchains_session_info,
	shared as teyrchains_shared,
};

use pezframe_election_provider_support::{
	bounds::{ElectionBounds, ElectionBoundsBuilder},
	onchain, SequentialPhragmen,
};
use pezframe_support::{
	construct_runtime, derive_impl,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{KeyOwnerProofSystem, WithdrawReasons},
	PalletId,
};
use pezkuwi_primitives::{
	async_backing::Constraints, slashing, AccountId, AccountIndex, Balance, BlockNumber,
	CandidateEvent, CandidateHash, CommittedCandidateReceiptV2 as CommittedCandidateReceipt,
	CoreIndex, CoreState, DisputeState, ExecutorParams, GroupRotationInfo, Hash as HashT,
	Id as ParaId, InboundDownwardMessage, InboundHrmpMessage, Moment, Nonce,
	OccupiedCoreAssumption, PersistedValidationData, ScrapedOnChainVotes,
	SessionInfo as SessionInfoData, Signature, ValidationCode, ValidationCodeHash, ValidatorId,
	ValidatorIndex, TEYRCHAIN_KEY_TYPE_ID,
};
use pezkuwi_runtime_common::{
	claims, impl_runtime_weights, paras_sudo_wrapper, BlockHashCount, BlockLength,
	SlowAdjustingFeeUpdate,
};
use pezkuwi_runtime_teyrchains::reward_points::RewardValidatorsWithEraPoints;
use pezpallet_grandpa::{fg_primitives, AuthorityId as GrandpaId};
use pezpallet_session::historical as session_historical;
use pezpallet_timestamp::Now;
use pezpallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
use pezsp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use pezsp_consensus_beefy::ecdsa_crypto::{AuthorityId as BeefyId, Signature as BeefySignature};
use pezsp_core::{ConstBool, ConstU32, ConstUint, Get, OpaqueMetadata};
use pezsp_mmr_primitives as mmr;
use pezsp_runtime::{
	curve::PiecewiseLinear,
	generic, impl_opaque_keys,
	traits::{
		BlakeTwo256, Block as BlockT, ConvertInto, OpaqueKeys, SaturatedConversion, StaticLookup,
		Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU128, KeyTypeId, Perbill, Percent,
};
use pezsp_staking::SessionIndex;
#[cfg(any(feature = "std", test))]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;
use xcm::latest::{Assets, InteriorLocation, Location, SendError, SendResult, SendXcm, XcmHash};

pub use paras_sudo_wrapper::Call as ParasSudoWrapperCall;
pub use pezpallet_balances::Call as BalancesCall;
#[cfg(feature = "std")]
pub use pezpallet_staking::StakerStatus;
pub use pezpallet_sudo::Call as SudoCall;
pub use pezpallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;
pub use teyrchains_paras::Call as ParasCall;

/// Constant values used within the runtime.
use peztest_runtime_constants::{currency::*, fee::*, time::*};
pub mod xcm_config;

impl_runtime_weights!(peztest_runtime_constants);

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Runtime version (Test).
#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("pezkuwi-test-runtime"),
	impl_name: alloc::borrow::Cow::Borrowed("parity-pezkuwi-test-runtime"),
	authoring_version: 2,
	spec_version: 1056,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: pezsp_consensus_babe::BabeEpochConfiguration =
	pezsp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: pezsp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

pezsp_api::decl_runtime_apis! {
	pub trait GetLastTimestamp {
		/// Returns the last timestamp of a runtime.
		fn get_last_timestamp() -> u64;
	}
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(pezframe_system::config_preludes::RelayChainDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type BlockWeights = BlockWeights;
	type BlockLength = BlockLength;
	type Nonce = Nonce;
	type Hash = HashT;
	type AccountId = AccountId;
	type Lookup = Indices;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
}

impl<C> pezframe_system::offchain::CreateTransactionBase<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type RuntimeCall = RuntimeCall;
	type Extrinsic = UncheckedExtrinsic;
}

impl<C> pezframe_system::offchain::CreateBare<C> for Runtime
where
	RuntimeCall: From<C>,
{
	fn create_bare(call: Self::RuntimeCall) -> Self::Extrinsic {
		UncheckedExtrinsic::new_bare(call)
	}
}

impl<C> pezframe_system::offchain::CreateTransaction<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extension = TxExtension;

	fn create_transaction(call: Self::RuntimeCall, extension: Self::Extension) -> Self::Extrinsic {
		UncheckedExtrinsic::new_transaction(call, extension)
	}
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
			pezframe_system::CheckMortality::<Runtime>::from(generic::Era::Immortal),
			pezframe_system::CheckNonce::<Runtime>::from(0),
			pezframe_system::CheckWeight::<Runtime>::new(),
			pezpallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
			pezframe_system::WeightReclaim::<Runtime>::new(),
		)
	}
}

parameter_types! {
	pub storage EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS as u64;
	pub storage ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pezpallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;

	// session module is the trigger
	type EpochChangeTrigger = pezpallet_babe::ExternalTrigger;

	type DisabledValidators = ();

	type WeightInfo = ();

	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;

	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pezpallet_babe::AuthorityId)>>::Proof;

	type EquivocationReportSystem = ();
}

parameter_types! {
	pub storage IndexDeposit: Balance = 1 * DOLLARS;
}

impl pezpallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * CENTS;
	pub storage MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pezpallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub storage TransactionByteFee: Balance = 10 * MILLICENTS;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pezpallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee = pezframe_support::weights::ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type WeightInfo = ();
}

parameter_types! {
	pub storage SlotDuration: u64 = SLOT_DURATION;
	pub storage MinimumPeriod: u64 = SlotDuration::get() / 2;
}
impl pezpallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

impl pezpallet_authorship::Config for Runtime {
	type FindAuthor = pezpallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = Staking;
}

parameter_types! {
	pub storage Period: BlockNumber = 10 * MINUTES;
	pub storage Offset: BlockNumber = 0;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub para_validator: Initializer,
		pub para_assignment: ParaSessionInfo,
		pub authority_discovery: AuthorityDiscovery,
	}
}

impl pezpallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = pezsp_runtime::traits::ConvertInto;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = Staking;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisablingStrategy = pezpallet_session::disabling::UpToLimitWithReEnablingDisablingStrategy;
	type WeightInfo = ();
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
	// Six sessions in an era (6 hours).
	pub storage SessionsPerEra: SessionIndex = 6;
	// 28 eras for unbonding (7 days).
	pub storage BondingDuration: pezsp_staking::EraIndex = 28;
	// 27 eras in which slashes can be cancelled (a bit less than 7 days).
	pub storage SlashDeferDuration: pezsp_staking::EraIndex = 27;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxExposurePageSize: u32 = 64;
	pub const MaxNominators: u32 = 256;
	pub const MaxAuthorities: u32 = 100_000;
	pub const OnChainMaxWinners: u32 = MaxAuthorities::get();
	// Unbounded number of election targets and voters.
	pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default().build();
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<AccountId, pezkuwi_runtime_common::elections::OnChainAccuracy>;
	type DataProvider = Staking;
	type WeightInfo = ();
	type Bounds = ElectionBoundsOnChain;
	type MaxWinnersPerPage = OnChainMaxWinners;
	type MaxBackersPerWinner = ConstU32<{ u32::MAX }>;
	type Sort = ConstBool<true>;
}

/// Upper limit on the number of NPOS nominations.
const MAX_QUOTA_NOMINATIONS: u32 = 16;

impl pezpallet_staking::Config for Runtime {
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = pezkuwi_runtime_common::CurrencyToVote;
	type RewardRemainder = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeEvent = RuntimeEvent;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = pezframe_system::EnsureNever<()>;
	type SessionInterface = Self;
	type EraPayout = pezpallet_staking::ConvertCurve<RewardCurve>;
	type MaxExposurePageSize = MaxExposurePageSize;
	type NextNewSession = Session;
	type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	// Use the nominator map to iter voter AND no-ops for all SortedListProvider hooks. The
	// migration to bags-list is a no-op, but the storage version will be updated.
	type VoterList = pezpallet_staking::UseNominatorsAndValidatorsMap<Runtime>;
	type TargetList = pezpallet_staking::UseValidatorsMap<Runtime>;
	type NominationsQuota = pezpallet_staking::FixedNominationsQuota<MAX_QUOTA_NOMINATIONS>;
	type MaxUnlockingChunks = pezframe_support::traits::ConstU32<32>;
	type MaxControllersInDeprecationBatch = ConstU32<5900>;
	type HistoryDepth = pezframe_support::traits::ConstU32<84>;
	type BenchmarkingConfig = pezkuwi_runtime_common::StakingBenchmarkingConfig;
	type EventListeners = ();
	type WeightInfo = ();
	type MaxValidatorSet = MaxAuthorities;
	type Filter = pezframe_support::traits::Nothing;
}

parameter_types! {
	pub MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pezpallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;

	type KeyOwnerProof = pezsp_core::Void;
	type EquivocationReportSystem = ();
}

impl<LocalCall> pezframe_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_signed_transaction<
		C: pezframe_system::offchain::AppCrypto<Self::Public, Self::Signature>,
	>(
		call: RuntimeCall,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: <Runtime as pezframe_system::Config>::Nonce,
	) -> Option<UncheckedExtrinsic> {
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;

		let current_block = System::block_number().saturated_into::<u64>().saturating_sub(1);
		let tip = 0;
		let tx_ext: TxExtension = (
			pezframe_system::AuthorizeCall::<Runtime>::new(),
			pezframe_system::CheckNonZeroSender::<Runtime>::new(),
			pezframe_system::CheckSpecVersion::<Runtime>::new(),
			pezframe_system::CheckTxVersion::<Runtime>::new(),
			pezframe_system::CheckGenesis::<Runtime>::new(),
			pezframe_system::CheckMortality::<Runtime>::from(generic::Era::mortal(
				period,
				current_block,
			)),
			pezframe_system::CheckNonce::<Runtime>::from(nonce),
			pezframe_system::CheckWeight::<Runtime>::new(),
			pezpallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			pezframe_system::WeightReclaim::<Runtime>::new(),
		)
			.into();
		let raw_payload = SignedPayload::new(call, tx_ext)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, tx_ext, _) = raw_payload.deconstruct();
		let address = Indices::unlookup(account);
		let transaction = UncheckedExtrinsic::new_signed(call, address, signature, tx_ext);
		Some(transaction)
	}
}

impl pezframe_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
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
	pub storage LeasePeriod: BlockNumber = 100_000;
	pub storage EndingPeriod: BlockNumber = 1000;
}

parameter_types! {
	pub Prefix: &'static [u8] = b"Pay HEZ to the Pezkuwi account:";
}

impl claims::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type Prefix = Prefix;
	type MoveClaimOrigin = pezframe_system::EnsureRoot<AccountId>;
	type WeightInfo = claims::TestWeightInfo;
}

parameter_types! {
	pub storage MinVestedTransfer: Balance = 100 * DOLLARS;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pezpallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

impl pezpallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = ();
}

impl teyrchains_configuration::Config for Runtime {
	type WeightInfo = teyrchains_configuration::TestWeightInfo;
}

impl teyrchains_shared::Config for Runtime {
	type DisabledValidators = Session;
}

impl teyrchains_inclusion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DisputesHandler = ParasDisputes;
	type RewardValidators = RewardValidatorsWithEraPoints<Runtime, Staking>;
	type MessageQueue = ();
	type WeightInfo = ();
}

impl teyrchains_disputes::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RewardValidators = ();
	type SlashingHandler = teyrchains_slashing::SlashValidatorsForDisputes<ParasSlashing>;
	type WeightInfo = teyrchains_disputes::TestWeightInfo;
}

impl teyrchains_slashing::Config for Runtime {
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, ValidatorId)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		ValidatorId,
	)>>::IdentificationTuple;
	type HandleReports = teyrchains_slashing::SlashingReportHandler<
		Self::KeyOwnerIdentification,
		Offences,
		ReportLongevity,
	>;
	type WeightInfo = teyrchains_disputes::slashing::TestWeightInfo;
	type BenchmarkingConfig = teyrchains_slashing::BenchConfig<1000>;
}

impl teyrchains_paras_inherent::Config for Runtime {
	type WeightInfo = teyrchains_paras_inherent::TestWeightInfo;
}

impl teyrchains_initializer::Config for Runtime {
	type Randomness = pezpallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type ForceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type CoretimeOnNewSession = Coretime;
}

impl teyrchains_session_info::Config for Runtime {
	type ValidatorSet = Historical;
}

parameter_types! {
	pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl teyrchains_paras::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = teyrchains_paras::TestWeightInfo;
	type UnsignedPriority = ParasUnsignedPriority;
	type QueueFootprinter = ParaInclusion;
	type NextSessionRotation = Babe;
	type OnNewHead = ();
	type AssignCoretime = CoretimeAssignmentProvider;
	type Fungible = Balances;
	type CooldownRemovalMultiplier = ConstUint<1>;
	type AuthorizeCurrentCodeOrigin = pezframe_system::EnsureRoot<AccountId>;
}

parameter_types! {
	pub const BrokerId: u32 = 10u32;
	pub MaxXcmTransactWeight: Weight = Weight::from_parts(10_000_000, 10_000);
}

pub struct BrokerPot;
impl Get<InteriorLocation> for BrokerPot {
	fn get() -> InteriorLocation {
		unimplemented!()
	}
}

parameter_types! {
	pub const OnDemandTrafficDefaultValue: FixedU128 = FixedU128::from_u32(1);
	// Keep 2 timeslices worth of revenue information.
	pub const MaxHistoricalRevenue: BlockNumber = 2 * 5;
	pub const OnDemandPalletId: PalletId = PalletId(*b"py/ondmd");
}

impl teyrchains_dmp::Config for Runtime {}

parameter_types! {
	pub const HrmpChannelSizeAndCapacityWithSystemRatio: Percent = Percent::from_percent(100);
}

impl teyrchains_hrmp::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type ChannelManager = pezframe_system::EnsureRoot<AccountId>;
	type Currency = Balances;
	type DefaultChannelSizeAndCapacityWithSystem = ActiveConfigHrmpChannelSizeAndCapacityRatio<
		Runtime,
		HrmpChannelSizeAndCapacityWithSystemRatio,
	>;
	type VersionWrapper = crate::Xcm;
	type WeightInfo = teyrchains_hrmp::TestWeightInfo;
}

impl teyrchains_on_demand::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type TrafficDefaultValue = OnDemandTrafficDefaultValue;
	type WeightInfo = teyrchains_on_demand::TestWeightInfo;
	type MaxHistoricalRevenue = MaxHistoricalRevenue;
	type PalletId = OnDemandPalletId;
}

impl teyrchains_assigner_coretime::Config for Runtime {}

impl teyrchains_scheduler::Config for Runtime {
	type AssignmentProvider = CoretimeAssignmentProvider;
}

pub struct DummyXcmSender;
impl SendXcm for DummyXcmSender {
	type Ticket = ();
	fn validate(
		_: &mut Option<Location>,
		_: &mut Option<xcm::latest::Xcm<()>>,
	) -> SendResult<Self::Ticket> {
		Ok(((), Assets::new()))
	}

	/// Actually carry out the delivery operation for a previously validated message sending.
	fn deliver(_ticket: Self::Ticket) -> Result<XcmHash, SendError> {
		Ok([0u8; 32])
	}
}

impl coretime::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type BrokerId = BrokerId;
	type WeightInfo = crate::coretime::TestWeightInfo;
	type SendXcm = DummyXcmSender;
	type MaxXcmTransactWeight = MaxXcmTransactWeight;
	type BrokerPotLocation = BrokerPot;
	type AssetTransactor = ();
	type AccountToLocation = ();
}

impl paras_sudo_wrapper::Config for Runtime {}

impl teyrchains_origin::Config for Runtime {}

impl pezpallet_test_notifier::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
}

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet_test_notifier {
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	use pezpallet_xcm::ensure_response;
	use pezsp_runtime::DispatchResult;
	use xcm::latest::prelude::*;
	use xcm_executor::traits::QueryHandler as XcmQueryHandler;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config + pezpallet_xcm::Config {
		#[allow(deprecated)]
		type RuntimeEvent: IsType<<Self as pezframe_system::Config>::RuntimeEvent>
			+ From<Event<Self>>;
		type RuntimeOrigin: IsType<<Self as pezframe_system::Config>::RuntimeOrigin>
			+ Into<Result<pezpallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>;
		type RuntimeCall: IsType<<Self as pezpallet_xcm::Config>::RuntimeCall> + From<Call<Self>>;
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		QueryPrepared(QueryId),
		NotifyQueryPrepared(QueryId),
		ResponseReceived(Location, QueryId, Response),
	}

	#[pezpallet::error]
	pub enum Error<T> {
		UnexpectedId,
		BadAccountFormat,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(1_000_000)]
		pub fn prepare_new_query(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let id = who
				.using_encoded(|mut d| <[u8; 32]>::decode(&mut d))
				.map_err(|_| Error::<T>::BadAccountFormat)?;
			let qid = <pezpallet_xcm::Pezpallet<T> as XcmQueryHandler>::new_query(
				Junction::AccountId32 { network: None, id },
				100u32.into(),
				Here,
			);
			Self::deposit_event(Event::<T>::QueryPrepared(qid));
			Ok(())
		}

		#[pezpallet::call_index(1)]
		#[pezpallet::weight(1_000_000)]
		pub fn prepare_new_notify_query(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let id = who
				.using_encoded(|mut d| <[u8; 32]>::decode(&mut d))
				.map_err(|_| Error::<T>::BadAccountFormat)?;
			let call =
				Call::<T>::notification_received { query_id: 0, response: Default::default() };
			let qid = pezpallet_xcm::Pezpallet::<T>::new_notify_query(
				Junction::AccountId32 { network: None, id },
				<T as Config>::RuntimeCall::from(call),
				100u32.into(),
				Here,
			);
			Self::deposit_event(Event::<T>::NotifyQueryPrepared(qid));
			Ok(())
		}

		#[pezpallet::call_index(2)]
		#[pezpallet::weight(1_000_000)]
		pub fn notification_received(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let responder = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;
			Self::deposit_event(Event::<T>::ResponseReceived(responder, query_id, response));
			Ok(())
		}
	}
}

construct_runtime! {
	pub enum Runtime
	{
		// Basic stuff; balances is uncallable initially.
		System: pezframe_system,

		// Must be before session.
		Babe: pezpallet_babe,

		Timestamp: pezpallet_timestamp,
		Indices: pezpallet_indices,
		Balances: pezpallet_balances,
		TransactionPayment: pezpallet_transaction_payment,

		// Consensus support.
		Authorship: pezpallet_authorship,
		Staking: pezpallet_staking,
		Offences: pezpallet_offences,
		Historical: session_historical,
		Session: pezpallet_session,
		Grandpa: pezpallet_grandpa,
		AuthorityDiscovery: pezpallet_authority_discovery,

		// Claims. Usable initially.
		Claims: claims,

		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: pezpallet_vesting,

		// Teyrchains runtime modules
		Configuration: teyrchains_configuration,
		ParaInclusion: teyrchains_inclusion,
		ParaInherent: teyrchains_paras_inherent,
		Initializer: teyrchains_initializer,
		Paras: teyrchains_paras,
		ParasShared: teyrchains_shared,
		Scheduler: teyrchains_scheduler,
		ParasSudoWrapper: paras_sudo_wrapper,
		ParasOrigin: teyrchains_origin,
		ParaSessionInfo: teyrchains_session_info,
		Hrmp: teyrchains_hrmp,
		Dmp: teyrchains_dmp,
		Xcm: pezpallet_xcm,
		ParasDisputes: teyrchains_disputes,
		ParasSlashing: teyrchains_slashing,
		OnDemandAssignmentProvider: teyrchains_on_demand,
		CoretimeAssignmentProvider: teyrchains_assigner_coretime,
		Coretime: coretime,

		Sudo: pezpallet_sudo,

		TestNotifier: pezpallet_test_notifier,
	}
}

/// The address format for describing accounts.
pub type Address = pezsp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// `BlockId` type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The extension to the basic transaction logic.
pub type TxExtension = (
	pezframe_system::AuthorizeCall<Runtime>,
	pezframe_system::CheckNonZeroSender<Runtime>,
	pezframe_system::CheckSpecVersion<Runtime>,
	pezframe_system::CheckTxVersion<Runtime>,
	pezframe_system::CheckGenesis<Runtime>,
	pezframe_system::CheckMortality<Runtime>,
	pezframe_system::CheckNonce<Runtime>,
	pezframe_system::CheckWeight<Runtime>,
	pezpallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	pezframe_system::WeightReclaim<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
/// Unchecked signature payload type as expected by this runtime.
pub type UncheckedSignaturePayload =
	generic::UncheckedSignaturePayload<Address, Signature, TxExtension>;

/// Executive: handles dispatch to the various modules.
pub type Executive = pezframe_executive::Executive<
	Runtime,
	Block,
	pezframe_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;

pub type Hash = <Block as BlockT>::Hash;
pub type Extrinsic = <Block as BlockT>::Extrinsic;

pezsp_api::impl_runtime_apis! {
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

		fn metadata_versions() -> Vec<u32> {
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

	impl pezsp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			runtime_impl::relevant_authority_ids::<Runtime>()
		}
	}

	#[api_version(15)]
	impl pezkuwi_primitives::runtime_api::TeyrchainHost<Block> for Runtime {
		fn validators() -> Vec<ValidatorId> {
			runtime_impl::validators::<Runtime>()
		}

		fn validator_groups() -> (Vec<Vec<ValidatorIndex>>, GroupRotationInfo<BlockNumber>) {
			runtime_impl::validator_groups::<Runtime>()
		}

		fn availability_cores() -> Vec<CoreState<Hash, BlockNumber>> {
			runtime_impl::availability_cores::<Runtime>()
		}

		fn persisted_validation_data(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<PersistedValidationData<Hash, BlockNumber>>
		{
			runtime_impl::persisted_validation_data::<Runtime>(para_id, assumption)
		}

		fn assumed_validation_data(
			para_id: ParaId,
			expected_persisted_validation_data_hash: Hash,
		) -> Option<(PersistedValidationData<Hash, BlockNumber>, ValidationCodeHash)> {
			runtime_impl::assumed_validation_data::<Runtime>(
				para_id,
				expected_persisted_validation_data_hash,
			)
		}

		fn check_validation_outputs(
			para_id: ParaId,
			outputs: pezkuwi_primitives::CandidateCommitments,
		) -> bool {
			runtime_impl::check_validation_outputs::<Runtime>(para_id, outputs)
		}

		fn session_index_for_child() -> SessionIndex {
			runtime_impl::session_index_for_child::<Runtime>()
		}

		fn validation_code(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<ValidationCode>
		{
			runtime_impl::validation_code::<Runtime>(para_id, assumption)
		}

		fn candidate_pending_availability(para_id: ParaId) -> Option<CommittedCandidateReceipt<Hash>> {
			#[allow(deprecated)]
			runtime_impl::candidate_pending_availability::<Runtime>(para_id)
		}

		fn candidate_events() -> Vec<CandidateEvent<Hash>> {
			runtime_impl::candidate_events::<Runtime, _>(|trait_event| trait_event.try_into().ok())
		}

		fn session_info(index: SessionIndex) -> Option<SessionInfoData> {
			runtime_impl::session_info::<Runtime>(index)
		}

		fn session_executor_params(session_index: SessionIndex) -> Option<ExecutorParams> {
			runtime_impl::session_executor_params::<Runtime>(session_index)
		}

		fn dmq_contents(
			recipient: ParaId,
		) -> Vec<InboundDownwardMessage<BlockNumber>> {
			runtime_impl::dmq_contents::<Runtime>(recipient)
		}

		fn inbound_hrmp_channels_contents(
			recipient: ParaId,
		) -> BTreeMap<ParaId, Vec<InboundHrmpMessage<BlockNumber>>> {
			runtime_impl::inbound_hrmp_channels_contents::<Runtime>(recipient)
		}

		fn validation_code_by_hash(hash: ValidationCodeHash) -> Option<ValidationCode> {
			runtime_impl::validation_code_by_hash::<Runtime>(hash)
		}

		fn on_chain_votes() -> Option<ScrapedOnChainVotes<Hash>> {
			runtime_impl::on_chain_votes::<Runtime>()
		}

		fn submit_pvf_check_statement(
			stmt: pezkuwi_primitives::PvfCheckStatement,
			signature: pezkuwi_primitives::ValidatorSignature,
		) {
			runtime_impl::submit_pvf_check_statement::<Runtime>(stmt, signature)
		}

		fn pvfs_require_precheck() -> Vec<ValidationCodeHash> {
			runtime_impl::pvfs_require_precheck::<Runtime>()
		}

		fn validation_code_hash(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<ValidationCodeHash>
		{
			runtime_impl::validation_code_hash::<Runtime>(para_id, assumption)
		}

		fn disputes() -> Vec<(SessionIndex, CandidateHash, DisputeState<BlockNumber>)> {
			runtime_impl::get_session_disputes::<Runtime>()
		}

		fn unapplied_slashes(
		) -> Vec<(SessionIndex, CandidateHash, slashing::LegacyPendingSlashes)> {
			runtime_impl::unapplied_slashes::<Runtime>()
		}

		fn unapplied_slashes_v2(
		) -> Vec<(SessionIndex, CandidateHash, slashing::PendingSlashes)> {
			runtime_impl::unapplied_slashes_v2::<Runtime>()
		}

		fn key_ownership_proof(
			validator_id: ValidatorId,
		) -> Option<slashing::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((TEYRCHAIN_KEY_TYPE_ID, validator_id))
				.map(|p| p.encode())
				.map(slashing::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_dispute_lost(
			dispute_proof: slashing::DisputeProof,
			key_ownership_proof: slashing::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			runtime_impl::submit_unsigned_slashing_report::<Runtime>(
				dispute_proof,
				key_ownership_proof,
			)
		}

		fn minimum_backing_votes() -> u32 {
			runtime_impl::minimum_backing_votes::<Runtime>()
		}

		fn para_backing_state(para_id: ParaId) -> Option<pezkuwi_primitives::async_backing::BackingState> {
			#[allow(deprecated)]
			runtime_impl::backing_state::<Runtime>(para_id)
		}

		fn async_backing_params() -> pezkuwi_primitives::AsyncBackingParams {
			#[allow(deprecated)]
			runtime_impl::async_backing_params::<Runtime>()
		}

		fn approval_voting_params() -> pezkuwi_primitives::ApprovalVotingParams {
			runtime_impl::approval_voting_params::<Runtime>()
		}

		fn disabled_validators() -> Vec<ValidatorIndex> {
			runtime_impl::disabled_validators::<Runtime>()
		}

		fn node_features() -> pezkuwi_primitives::NodeFeatures {
			runtime_impl::node_features::<Runtime>()
		}

		fn claim_queue() -> BTreeMap<CoreIndex, VecDeque<ParaId>> {
			runtime_impl::claim_queue::<Runtime>()
		}

		fn candidates_pending_availability(para_id: ParaId) -> Vec<CommittedCandidateReceipt<Hash>> {
			runtime_impl::candidates_pending_availability::<Runtime>(para_id)
		}

		fn backing_constraints(para_id: ParaId) -> Option<Constraints> {
			runtime_impl::backing_constraints::<Runtime>(para_id)
		}

		fn scheduling_lookahead() -> u32 {
			runtime_impl::scheduling_lookahead::<Runtime>()
		}

		fn validation_code_bomb_limit() -> u32 {
			runtime_impl::validation_code_bomb_limit::<Runtime>()
		}

		fn para_ids() -> Vec<ParaId> {
			pezstaging_runtime_impl::para_ids::<Runtime>()
		}
	}

	#[api_version(6)]
	impl pezsp_consensus_beefy::BeefyApi<Block, BeefyId> for Runtime {
		fn beefy_genesis() -> Option<BlockNumber> {
			// dummy implementation due to lack of BEEFY pezpallet.
			None
		}

		fn validator_set() -> Option<pezsp_consensus_beefy::ValidatorSet<BeefyId>> {
			// dummy implementation due to lack of BEEFY pezpallet.
			None
		}

		fn submit_report_double_voting_unsigned_extrinsic(
			_equivocation_proof: pezsp_consensus_beefy::DoubleVotingProof<
				BlockNumber,
				BeefyId,
				BeefySignature,
			>,
			_key_owner_proof: pezsp_consensus_beefy::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn submit_report_fork_voting_unsigned_extrinsic(
			_equivocation_proof:
				pezsp_consensus_beefy::ForkVotingProof<
					<Block as BlockT>::Header,
					BeefyId,
					pezsp_runtime::OpaqueValue
				>,
			_key_owner_proof: pezsp_consensus_beefy::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn submit_report_future_block_voting_unsigned_extrinsic(
			_equivocation_proof: pezsp_consensus_beefy::FutureBlockVotingProof<BlockNumber, BeefyId>,
			_key_owner_proof: pezsp_consensus_beefy::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: pezsp_consensus_beefy::ValidatorSetId,
			_authority_id: BeefyId,
		) -> Option<pezsp_consensus_beefy::OpaqueKeyOwnershipProof> {
			None
		}
	}

	#[api_version(3)]
	impl mmr::MmrApi<Block, Hash, BlockNumber> for Runtime {
		fn mmr_root() -> Result<Hash, mmr::Error> {
			Err(mmr::Error::PalletNotIncluded)
		}

		fn mmr_leaf_count() -> Result<mmr::LeafIndex, mmr::Error> {
			Err(mmr::Error::PalletNotIncluded)
		}

		fn generate_proof(
			_block_numbers: Vec<BlockNumber>,
			_best_known_block_number: Option<BlockNumber>,
		) -> Result<(Vec<mmr::EncodableOpaqueLeaf>, mmr::LeafProof<Hash>), mmr::Error> {
			Err(mmr::Error::PalletNotIncluded)
		}

		fn generate_ancestry_proof(
			_prev_block_number: BlockNumber,
			_best_known_block_number: Option<BlockNumber>,
		) -> Result<mmr::AncestryProof<Hash>, mmr::Error> {
			Err(mmr::Error::PalletNotIncluded)
		}

		fn verify_proof(_leaves: Vec<mmr::EncodableOpaqueLeaf>, _proof: mmr::LeafProof<Hash>)
			-> Result<(), mmr::Error>
		{
			Err(mmr::Error::PalletNotIncluded)
		}

		fn verify_proof_stateless(
			_root: Hash,
			_leaves: Vec<mmr::EncodableOpaqueLeaf>,
			_proof: mmr::LeafProof<Hash>
		) -> Result<(), mmr::Error> {
			Err(mmr::Error::PalletNotIncluded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> Vec<(GrandpaId, u64)> {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			pezpallet_grandpa::CurrentSetId::<Runtime>::get()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				pezsp_runtime::traits::NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: fg_primitives::AuthorityId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			None
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
			_authority_id: pezsp_consensus_babe::AuthorityId,
		) -> Option<pezsp_consensus_babe::OpaqueKeyOwnershipProof> {
			None
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: pezsp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			_key_owner_proof: pezsp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}
	}

	impl pezsp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, pezsp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl pezframe_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
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

	impl crate::GetLastTimestamp<Block> for Runtime {
		fn get_last_timestamp() -> u64 {
			Now::<Runtime>::get()
		}
	}

	impl pezsp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> pezsp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<pezsp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<pezsp_genesis_builder::PresetId> {
			vec![]
		}
	}
}
