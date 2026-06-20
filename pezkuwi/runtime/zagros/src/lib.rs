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

//! The Zagros runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `#[pezframe_support::runtime]!` does a lot of recursion and requires us to increase the limit.
#![recursion_limit = "512"]

extern crate alloc;

use alloc::{
	collections::{btree_map::BTreeMap, vec_deque::VecDeque},
	vec,
	vec::Vec,
};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pezframe_election_provider_support::{
	bounds::ElectionBoundsBuilder, onchain, SequentialPhragmen,
};
use pezframe_support::{
	derive_impl,
	dynamic_params::{dynamic_pallet_params, dynamic_params},
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{
		fungible::HoldConsideration, tokens::UnityOrOuterConversion, AsEnsureOriginWithArg,
		ConstU32, Contains, EitherOf, EitherOfDiverse, EnsureOriginWithArg, FromContains,
		InstanceFilter, KeyOwnerProofSystem, LinearStoragePrice, Nothing, ProcessMessage,
		ProcessMessageError, VariantCountOf, WithdrawReasons,
	},
	weights::{ConstantMultiplier, WeightMeter},
	PalletId,
};
use pezframe_system::{EnsureRoot, EnsureSigned};
use pezkuwi_primitives::{
	async_backing::Constraints, slashing, AccountId, AccountIndex, ApprovalVotingParams, Balance,
	BlockNumber, CandidateEvent, CandidateHash,
	CommittedCandidateReceiptV2 as CommittedCandidateReceipt, CoreIndex, CoreState, DisputeState,
	ExecutorParams, GroupRotationInfo, Hash, Id as ParaId, InboundDownwardMessage,
	InboundHrmpMessage, Moment, NodeFeatures, Nonce, OccupiedCoreAssumption,
	PersistedValidationData, PvfCheckStatement, ScrapedOnChainVotes, SessionInfo, Signature,
	ValidationCode, ValidationCodeHash, ValidatorId, ValidatorIndex, ValidatorSignature,
	TEYRCHAIN_KEY_TYPE_ID,
};
use pezkuwi_runtime_common::{
	assigned_slots, auctions, crowdloan,
	elections::OnChainAccuracy,
	identity_migrator, impl_runtime_weights,
	impls::{
		ContainsParts, LocatableAssetConverter, ToAuthor, VersionedLocatableAsset,
		VersionedLocationConverter,
	},
	paras_registrar, paras_sudo_wrapper, prod_or_fast, slots,
	traits::OnSwap,
	BalanceToU256, BlockHashCount, BlockLength, SlowAdjustingFeeUpdate, U256ToBalance,
};
use pezkuwi_runtime_teyrchains::{
	assigner_coretime as teyrchains_assigner_coretime, configuration as teyrchains_configuration,
	configuration::ActiveConfigHrmpChannelSizeAndCapacityRatio,
	coretime, disputes as teyrchains_disputes,
	disputes::slashing as teyrchains_slashing,
	dmp as teyrchains_dmp, hrmp as teyrchains_hrmp, inclusion as teyrchains_inclusion,
	inclusion::{AggregateMessageOrigin, UmpQueueId},
	initializer as teyrchains_initializer, on_demand as teyrchains_on_demand,
	origin as teyrchains_origin, paras as teyrchains_paras,
	paras_inherent as teyrchains_paras_inherent, reward_points as teyrchains_reward_points,
	runtime_api_impl::{
		v13 as teyrchains_runtime_api_impl, vstaging as teyrchains_staging_runtime_api_impl,
	},
	scheduler as teyrchains_scheduler, session_info as teyrchains_session_info,
	shared as teyrchains_shared,
};
use pezpallet_grandpa::{fg_primitives, AuthorityId as GrandpaId};
use pezpallet_identity::legacy::IdentityInfo;
use pezpallet_nomination_pools::PoolId;
use pezpallet_session::historical as session_historical;
use pezpallet_staking::UseValidatorsMap;
use pezpallet_staking_async_ah_client as ah_client;
use pezpallet_staking_async_rc_client as rc_client;
use pezpallet_transaction_payment::{FeeDetails, FungibleAdapter, RuntimeDispatchInfo};
use pezpallet_xcm::{EnsureXcm, IsVoiceOfBody};
use pezsp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use pezsp_consensus_beefy::{
	ecdsa_crypto::{AuthorityId as BeefyId, Signature as BeefySignature},
	mmr::{BeefyDataProvider, MmrLeafVersion},
};
use pezsp_core::{ConstBool, ConstU8, ConstUint, OpaqueMetadata, RuntimeDebug, H256};
#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;
use pezsp_runtime::{
	generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, BlakeTwo256, Block as BlockT, ConvertInto, Get, IdentityLookup,
		Keccak256, OpaqueKeys, SaturatedConversion, Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU128, KeyTypeId, MultiSignature, MultiSigner, Percent, Permill,
};
use pezsp_staking::{EraIndex, SessionIndex};
#[cfg(any(feature = "std", test))]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;
use scale_info::TypeInfo;
use xcm::{
	latest::prelude::*, Version as XcmVersion, VersionedAsset, VersionedAssetId, VersionedAssets,
	VersionedLocation, VersionedXcm,
};
use xcm_builder::PayOverXcm;
use xcm_runtime_pezapis::{
	dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
	fees::Error as XcmPaymentApiError,
};

pub use pezframe_system::Call as SystemCall;
pub use pezpallet_balances::Call as BalancesCall;
pub use pezpallet_election_provider_multi_phase::{Call as EPMCall, GeometricDepositBase};
pub use pezpallet_timestamp::Call as TimestampCall;

/// Constant values used within the runtime.
use zagros_runtime_constants::{
	currency::*,
	fee::*,
	system_teyrchain::{coretime::TIMESLICE_PERIOD, ASSET_HUB_ID, BROKER_ID},
	time::*,
};

mod bag_thresholds;
mod genesis_config_presets;
mod weights;
pub mod xcm_config;

// Implemented types.
mod impls;
use impls::ToTeyrchainIdentityReaper;

// Governance and configurations.
pub mod governance;
use governance::{
	pezpallet_custom_origins, AuctionAdmin, FellowshipAdmin, GeneralAdmin, LeaseAdmin,
	StakingAdmin, Treasurer, TreasurySpender,
};
use xcm_config::XcmConfig;

#[cfg(test)]
mod tests;

impl_runtime_weights!(zagros_runtime_constants);

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(feature = "std")]
pub mod fast_runtime_binary {
	include!(concat!(env!("OUT_DIR"), "/fast_runtime_binary.rs"));
}

/// Runtime version (Zagros).
#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("zagros"),
	impl_name: alloc::borrow::Cow::Borrowed("parity-zagros"),
	authoring_version: 2,
	spec_version: 1_020_001,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 27,
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

/// A type to identify calls to the Identity pezpallet. These will be filtered to prevent
/// invocation, locking the state of the pezpallet and preventing further updates to identities and
/// sub-identities. The locked state will be the genesis state of a new system chain and then
/// removed from the Relay Chain.
pub struct IsIdentityCall;
impl Contains<RuntimeCall> for IsIdentityCall {
	fn contains(c: &RuntimeCall) -> bool {
		matches!(c, RuntimeCall::Identity(_))
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
	type Hash = Hash;
	type AccountId = AccountId;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type DbWeight = RocksDbWeight;
	type Version = Version;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type SystemWeightInfo = weights::pezframe_system::WeightInfo<Runtime>;
	type ExtensionsWeightInfo = weights::pezframe_system_extensions::WeightInfo<Runtime>;
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
	type MultiBlockMigrator = MultiBlockMigrations;
	type SingleBlockMigrations = Migrations;
}

parameter_types! {
	pub MaximumSchedulerWeight: pezframe_support::weights::Weight = Perbill::from_percent(80) *
		BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pezpallet_scheduler::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	// The goal of having ScheduleOrigin include AuctionAdmin is to allow the auctions track of
	// OpenGov to schedule periodic auctions.
	type ScheduleOrigin = EitherOf<EnsureRoot<AccountId>, AuctionAdmin>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = weights::pezpallet_scheduler::WeightInfo<Runtime>;
	type OriginPrivilegeCmp = pezframe_support::traits::EqualPrivilegeOnly;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

parameter_types! {
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pezpallet_preimage::HoldReason::Preimage);
}

/// Dynamic params that can be adjusted at runtime.
#[dynamic_params(RuntimeParameters, pezpallet_parameters::Parameters::<Runtime>)]
pub mod dynamic_params {
	use super::*;

	/// Parameters used to calculate era payouts, see
	/// [`pezkuwi_runtime_common::impls::EraPayoutParams`].
	#[dynamic_pallet_params]
	#[codec(index = 0)]
	pub mod inflation {
		/// Minimum inflation rate used to calculate era payouts.
		#[codec(index = 0)]
		pub static MinInflation: Perquintill = Perquintill::from_rational(25u64, 1000u64);

		/// Maximum inflation rate used to calculate era payouts.
		#[codec(index = 1)]
		pub static MaxInflation: Perquintill = Perquintill::from_rational(10u64, 100u64);

		/// Ideal stake ratio used to calculate era payouts.
		#[codec(index = 2)]
		pub static IdealStake: Perquintill = Perquintill::from_rational(50u64, 100u64);

		/// Falloff used to calculate era payouts.
		#[codec(index = 3)]
		pub static Falloff: Perquintill = Perquintill::from_rational(50u64, 1000u64);

		/// Whether to use auction slots or not in the calculation of era payouts. If set to true,
		/// the `legacy_auction_proportion` of 60% will be used in the calculation of era payouts.
		#[codec(index = 4)]
		pub static UseAuctionSlots: bool = false;
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
	fn default() -> Self {
		RuntimeParameters::Inflation(dynamic_params::inflation::Parameters::MinInflation(
			dynamic_params::inflation::MinInflation,
			Some(Perquintill::from_rational(25u64, 1000u64)),
		))
	}
}

impl pezpallet_parameters::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeParameters = RuntimeParameters;
	type AdminOrigin = DynamicParameterOrigin;
	type WeightInfo = weights::pezpallet_parameters::WeightInfo<Runtime>;
}

/// Defines what origin can modify which dynamic parameters.
pub struct DynamicParameterOrigin;
impl EnsureOriginWithArg<RuntimeOrigin, RuntimeParametersKey> for DynamicParameterOrigin {
	type Success = ();

	fn try_origin(
		origin: RuntimeOrigin,
		key: &RuntimeParametersKey,
	) -> Result<Self::Success, RuntimeOrigin> {
		use crate::RuntimeParametersKey::*;

		match key {
			Inflation(_) => pezframe_system::ensure_root(origin.clone()),
		}
		.map_err(|_| origin)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_key: &RuntimeParametersKey) -> Result<RuntimeOrigin, ()> {
		// Provide the origin for the parameter returned by `Default`:
		Ok(RuntimeOrigin::root())
	}
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

parameter_types! {
	pub const EpochDuration: u64 = prod_or_fast!(
		EPOCH_DURATION_IN_SLOTS as u64,
		2 * MINUTES as u64
	);
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pezpallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;

	// session module is the trigger
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
	pub const IndexDeposit: Balance = 100 * CENTS;
}

impl pezpallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezpallet_indices::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const MaxLocks: u32 = 50;
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
	type WeightInfo = weights::pezpallet_balances::WeightInfo<Runtime>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const BeefySetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pezpallet_beefy::Config for Runtime {
	type BeefyId = BeefyId;
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type MaxSetIdSessionEntries = BeefySetIdSessionEntries;
	type OnNewValidatorSet = BeefyMmrLeaf;
	type AncestryHelper = BeefyMmrLeaf;
	type WeightInfo = ();
	type KeyOwnerProof = pezsp_session::MembershipProof;
	type EquivocationReportSystem =
		pezpallet_beefy::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl pezpallet_mmr::Config for Runtime {
	const INDEXING_PREFIX: &'static [u8] = mmr::INDEXING_PREFIX;
	type Hashing = Keccak256;
	type OnNewRoot = pezpallet_beefy_mmr::DepositBeefyDigest<Runtime>;
	type LeafData = pezpallet_beefy_mmr::Pezpallet<Runtime>;
	type BlockHashProvider = pezpallet_mmr::DefaultBlockHashProvider<Runtime>;
	type WeightInfo = weights::pezpallet_mmr::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = teyrchains_paras::benchmarking::mmr_setup::MmrSetup<Runtime>;
}

/// MMR helper types.
mod mmr {
	use super::Runtime;
	pub use pezpallet_mmr::primitives::*;

	pub type Leaf = <<Runtime as pezpallet_mmr::Config>::LeafData as LeafDataProvider>::LeafData;
	pub type Hashing = <Runtime as pezpallet_mmr::Config>::Hashing;
	pub type Hash = <Hashing as pezsp_runtime::traits::Hash>::Output;
}

parameter_types! {
	pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(0, 0);
}

/// A BEEFY data provider that merkelizes all the teyrchain heads at the current block
/// (sorted by their teyrchain id).
pub struct ParaHeadsRootProvider;
impl BeefyDataProvider<H256> for ParaHeadsRootProvider {
	fn extra_data() -> H256 {
		let para_heads: Vec<(u32, Vec<u8>)> =
			teyrchains_paras::Pezpallet::<Runtime>::sorted_para_heads();
		pez_binary_merkle_tree::merkle_root::<mmr::Hashing, _>(
			para_heads.into_iter().map(|pair| pair.encode()),
		)
		.into()
	}
}

impl pezpallet_beefy_mmr::Config for Runtime {
	type LeafVersion = LeafVersion;
	type BeefyAuthorityToMerkleLeaf = pezpallet_beefy_mmr::BeefyEcdsaToEthereum;
	type LeafExtra = H256;
	type BeefyDataProvider = ParaHeadsRootProvider;
	type WeightInfo = weights::pezpallet_beefy_mmr::WeightInfo<Runtime>;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pezpallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ToAuthor<Runtime>>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type WeightInfo = weights::pezpallet_transaction_payment::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}
impl pezpallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pezpallet_timestamp::WeightInfo<Runtime>;
}

impl pezpallet_authorship::Config for Runtime {
	type FindAuthor = pezpallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = StakingAhClient;
}

parameter_types! {
	pub const Period: BlockNumber = 10 * MINUTES;
	pub const Offset: BlockNumber = 0;
	pub const KeyDeposit: Balance = deposit(1, 5 * 32 + 33);
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub para_validator: Initializer,
		pub para_assignment: ParaSessionInfo,
		pub authority_discovery: AuthorityDiscovery,
		pub beefy: Beefy,
	}
}

impl pezpallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = session_historical::NoteHistoricalRoot<Self, StakingAhClient>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisablingStrategy = pezpallet_session::disabling::UpToLimitWithReEnablingDisablingStrategy;
	type WeightInfo = weights::pezpallet_session::WeightInfo<Runtime>;
	type Currency = Balances;
	type KeyDeposit = KeyDeposit;
}

impl pezpallet_session::historical::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type FullIdentification = pezsp_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pezpallet_staking::DefaultExposureOf<Self>;
}

pub struct MaybeSignedPhase;

impl Get<u32> for MaybeSignedPhase {
	fn get() -> u32 {
		// 1 day = 4 eras -> 1 week = 28 eras. We want to disable signed phase once a week to test
		// the fallback unsigned phase is able to compute elections on Zagros.
		if pezpallet_staking::CurrentEra::<Runtime>::get().unwrap_or(1).is_multiple_of(28) {
			0
		} else {
			SignedPhase::get()
		}
	}
}

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	pub SignedPhase: u32 = prod_or_fast!(
		EPOCH_DURATION_IN_SLOTS / 4,
		(1 * MINUTES).min(EpochDuration::get().saturated_into::<u32>() / 2)
	);
	pub UnsignedPhase: u32 = prod_or_fast!(
		EPOCH_DURATION_IN_SLOTS / 4,
		(1 * MINUTES).min(EpochDuration::get().saturated_into::<u32>() / 2)
	);

	// signed config
	pub const SignedMaxSubmissions: u32 = 128;
	pub const SignedMaxRefunds: u32 = 128 / 4;
	pub const SignedFixedDeposit: Balance = deposit(2, 0);
	pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);
	pub const SignedDepositByte: Balance = deposit(0, 10) / 1024;
	// Each good submission will get 1 ZGR as reward
	pub SignedRewardBase: Balance = 1 * UNITS;

	// 1 hour session, 15 minutes unsigned phase, 4 offchain executions.
	pub OffchainRepeat: BlockNumber = UnsignedPhase::get() / 4;

	pub const MaxElectingVoters: u32 = 22_500;
	/// We take the top 22500 nominators as electing voters and all of the validators as electable
	/// targets. Whilst this is the case, we cannot and shall not increase the size of the
	/// validator intentions.
	pub ElectionBounds: pezframe_election_provider_support::bounds::ElectionBounds =
		ElectionBoundsBuilder::default().voters_count(MaxElectingVoters::get().into()).build();
	// Maximum winners that can be chosen as active validators
	pub const MaxActiveValidators: u32 = 1000;
	// One page only, fill the whole page with the `MaxActiveValidators`.
	pub const MaxWinnersPerPage: u32 = MaxActiveValidators::get();
	// Unbonded, thus the max backers per winner maps to the max electing voters limit.
	pub const MaxBackersPerWinner: u32 = MaxElectingVoters::get();
}

pezframe_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = pezsp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(16)
);

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type Sort = ConstBool<true>;
	type System = Runtime;
	type Solver = SequentialPhragmen<AccountId, OnChainAccuracy>;
	type DataProvider = Staking;
	type WeightInfo = weights::pezframe_election_provider_support::WeightInfo<Runtime>;
	type Bounds = ElectionBounds;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type MaxWinnersPerPage = MaxWinnersPerPage;
}

impl pezpallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = OffchainSolutionLengthLimit;
	type MaxWeight = OffchainSolutionWeightLimit;
	type Solution = NposCompactSolution16;
	type MaxVotesPerVoter = <
    <Self as pezpallet_election_provider_multi_phase::Config>::DataProvider
    as
    pezframe_election_provider_support::ElectionDataProvider
    >::MaxVotesPerVoter;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type MaxWinners = MaxWinnersPerPage;

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
	type SignedPhase = MaybeSignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type SignedMaxSubmissions = SignedMaxSubmissions;
	type SignedMaxRefunds = SignedMaxRefunds;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase =
		GeometricDepositBase<Balance, SignedFixedDeposit, SignedDepositIncreaseFactor>;
	type SignedDepositByte = SignedDepositByte;
	type SignedDepositWeight = ();
	type SignedMaxWeight =
		<Self::MinerConfig as pezpallet_election_provider_multi_phase::MinerConfig>::MaxWeight;
	type MinerConfig = Self;
	type SlashHandler = (); // burn slashes
	type RewardHandler = (); // rewards are minted from the void
	type BetterSignedThreshold = ();
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = NposSolutionPriority;
	type MaxWinners = MaxWinnersPerPage;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type DataProvider = Staking;
	#[cfg(any(feature = "fast-runtime", feature = "runtime-benchmarks"))]
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	#[cfg(not(any(feature = "fast-runtime", feature = "runtime-benchmarks")))]
	type Fallback = pezframe_election_provider_support::NoElection<(
		AccountId,
		BlockNumber,
		Staking,
		MaxWinnersPerPage,
		MaxBackersPerWinner,
	)>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<
		AccountId,
		pezpallet_election_provider_multi_phase::SolutionAccuracyOf<Self>,
		(),
	>;
	type BenchmarkingConfig = pezkuwi_runtime_common::elections::BenchmarkConfig;
	type ForceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pezpallet_election_provider_multi_phase::WeightInfo<Self>;
	type ElectionBounds = ElectionBounds;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &bag_thresholds::THRESHOLDS;
	pub const AutoRebagNumber: u32 = 10;
}

type VoterBagsListInstance = pezpallet_bags_list::Instance1;
impl pezpallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezpallet_bags_list::WeightInfo<Runtime>;
	type ScoreProvider = Staking;
	type BagThresholds = BagThresholds;
	type MaxAutoRebagPerBlock = AutoRebagNumber;
	type Score = pezsp_npos_elections::VoteWeight;
}

pub struct EraPayout;
impl pezpallet_staking::EraPayout<Balance> for EraPayout {
	fn era_payout(
		_total_staked: Balance,
		_total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		const MILLISECONDS_PER_YEAR: u64 = (1000 * 3600 * 24 * 36525) / 100;
		// A normal-sized era will have 1 / 365.25 here:
		let relative_era_len =
			FixedU128::from_rational(era_duration_millis.into(), MILLISECONDS_PER_YEAR.into());

		// Fixed total TI that we use as baseline for the issuance.
		let fixed_total_issuance: i128 = 5_216_342_402_773_185_773;
		let fixed_inflation_rate = FixedU128::from_rational(8, 100);
		let yearly_emission = fixed_inflation_rate.saturating_mul_int(fixed_total_issuance);

		let era_emission = relative_era_len.saturating_mul_int(yearly_emission);
		// 15% to treasury, as per Pezkuwi ref 1139.
		let to_treasury = FixedU128::from_rational(15, 100).saturating_mul_int(era_emission);
		let to_stakers = era_emission.saturating_sub(to_treasury);

		(to_stakers.saturated_into(), to_treasury.saturated_into())
	}
}

parameter_types! {
	// Six sessions in an era (6 hours).
	pub const SessionsPerEra: SessionIndex = prod_or_fast!(6, 2);
	// 2 eras for unbonding (12 hours).
	pub const BondingDuration: EraIndex = 2;
	// 1 era in which slashes can be cancelled (6 hours).
	pub const SlashDeferDuration: EraIndex = 1;
	pub const MaxExposurePageSize: u32 = 64;
	// Note: this is not really correct as Max Nominators is (MaxExposurePageSize * page_count) but
	// this is an unbounded number. We just set it to a reasonably high value, 1 full page
	// of nominators.
	pub const MaxNominators: u32 = 64;
	pub const MaxNominations: u32 = <NposCompactSolution16 as pezframe_election_provider_support::NposSolution>::LIMIT as u32;
	pub const MaxControllersInDeprecationBatch: u32 = 751;
}

impl pezpallet_staking::Config for Runtime {
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	type UnixTime = Timestamp;
	// Zagros's total issuance is already more than `u64::MAX`, this will work better.
	type CurrencyToVote = pezsp_staking::currency_to_vote::SaturatingCurrencyToVote;
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = EitherOf<EnsureRoot<AccountId>, StakingAdmin>;
	type SessionInterface = Self;
	type EraPayout = EraPayout;
	type MaxExposurePageSize = MaxExposurePageSize;
	type NextNewSession = Session;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = VoterList;
	type TargetList = UseValidatorsMap<Self>;
	type MaxValidatorSet = MaxActiveValidators;
	type NominationsQuota = pezpallet_staking::FixedNominationsQuota<{ MaxNominations::get() }>;
	type MaxUnlockingChunks = pezframe_support::traits::ConstU32<32>;
	type HistoryDepth = pezframe_support::traits::ConstU32<84>;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type BenchmarkingConfig = pezkuwi_runtime_common::StakingBenchmarkingConfig;
	type EventListeners = (NominationPools, DelegatedStaking);
	type WeightInfo = weights::pezpallet_staking::WeightInfo<Runtime>;
	// Genesis benchmarking setup needs this until we remove the pezpallet completely.
	#[cfg(not(feature = "on-chain-release-build"))]
	type Filter = Nothing;
	#[cfg(feature = "on-chain-release-build")]
	type Filter = pezframe_support::traits::Everything;
}

#[derive(Encode, Decode)]
enum AssetHubRuntimePallets<AccountId> {
	// Audit: `StakingRcClient` in asset-hub-zagros
	#[codec(index = 89)]
	RcClient(RcClientCalls<AccountId>),
}

#[derive(Encode, Decode)]
enum RcClientCalls<AccountId> {
	#[codec(index = 0)]
	RelaySessionReport(rc_client::SessionReport<AccountId>),
	#[codec(index = 1)]
	RelayNewOffencePaged(Vec<(SessionIndex, rc_client::Offence<AccountId>)>),
}

pub struct AssetHubLocation;
impl Get<Location> for AssetHubLocation {
	fn get() -> Location {
		Location::new(0, [Junction::Teyrchain(ASSET_HUB_ID)])
	}
}

pub struct EnsureAssetHub;
impl pezframe_support::traits::EnsureOrigin<RuntimeOrigin> for EnsureAssetHub {
	type Success = ();
	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match <RuntimeOrigin as Into<Result<teyrchains_origin::Origin, RuntimeOrigin>>>::into(
			o.clone(),
		) {
			Ok(teyrchains_origin::Origin::Teyrchain(id)) if id == ASSET_HUB_ID.into() => Ok(()),
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

pub struct SessionReportToXcm;
impl pezsp_runtime::traits::Convert<rc_client::SessionReport<AccountId>, Xcm<()>>
	for SessionReportToXcm
{
	fn convert(a: rc_client::SessionReport<AccountId>) -> Xcm<()> {
		Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			Instruction::Transact {
				origin_kind: OriginKind::Superuser,
				fallback_max_weight: None,
				call: AssetHubRuntimePallets::RcClient(RcClientCalls::RelaySessionReport(a))
					.encode()
					.into(),
			},
		])
	}
}

pub struct QueuedOffenceToXcm;
impl pezsp_runtime::traits::Convert<Vec<ah_client::QueuedOffenceOf<Runtime>>, Xcm<()>>
	for QueuedOffenceToXcm
{
	fn convert(offences: Vec<ah_client::QueuedOffenceOf<Runtime>>) -> Xcm<()> {
		Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			Instruction::Transact {
				origin_kind: OriginKind::Superuser,
				fallback_max_weight: None,
				call: AssetHubRuntimePallets::RcClient(RcClientCalls::RelayNewOffencePaged(
					offences,
				))
				.encode()
				.into(),
			},
		])
	}
}

pub struct StakingXcmToAssetHub;
impl ah_client::SendToAssetHub for StakingXcmToAssetHub {
	type AccountId = AccountId;

	fn relay_session_report(
		session_report: rc_client::SessionReport<Self::AccountId>,
	) -> Result<(), ()> {
		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			AssetHubLocation,
			rc_client::SessionReport<AccountId>,
			SessionReportToXcm,
		>::send(session_report)
	}

	fn relay_new_offence_paged(
		offences: Vec<ah_client::QueuedOffenceOf<Runtime>>,
	) -> Result<(), ()> {
		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			AssetHubLocation,
			Vec<ah_client::QueuedOffenceOf<Runtime>>,
			QueuedOffenceToXcm,
		>::send(offences)
	}
}

impl ah_client::Config for Runtime {
	type CurrencyBalance = Balance;
	type AssetHubOrigin =
		pezframe_support::traits::EitherOfDiverse<EnsureRoot<AccountId>, EnsureAssetHub>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type SessionInterface = Self;
	type SendToAssetHub = StakingXcmToAssetHub;
	type MinimumValidatorSetSize = ConstU32<1>;
	type UnixTime = Timestamp;
	type PointsPerBlock = ConstU32<20>;
	type MaxOffenceBatchSize = ConstU32<50>;
	type Fallback = Staking;
	type MaximumValidatorsWithPoints = ConstU32<{ MaxActiveValidators::get() * 4 }>;
	type MaxSessionReportRetries = ConstU32<5>;
}

impl pezpallet_fast_unstake::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BatchSize = pezframe_support::traits::ConstU32<64>;
	type Deposit = pezframe_support::traits::ConstU128<{ UNITS }>;
	type ControlOrigin = EnsureRoot<AccountId>;
	type Staking = Staking;
	type MaxErasToCheckPerBlock = ConstU32<1>;
	type WeightInfo = weights::pezpallet_fast_unstake::WeightInfo<Runtime>;
}

parameter_types! {
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(2);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	// The asset's interior location for the paying account. This is the Treasury
	// pezpallet instance (which sits at index 37).
	pub TreasuryInteriorLocation: InteriorLocation = PalletInstance(37).into();

	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 100 * CENTS;
	pub const DataDepositPerByte: Balance = 1 * CENTS;
	pub const MaxApprovals: u32 = 100;
	pub const MaxAuthorities: u32 = 100_000;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxBalance: Balance = Balance::max_value();
}

impl pezpallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type MaxApprovals = MaxApprovals;
	type WeightInfo = weights::pezpallet_treasury::WeightInfo<Runtime>;
	type SpendFunds = ();
	type SpendOrigin = TreasurySpender;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayOverXcm<
		TreasuryInteriorLocation,
		crate::xcm_config::XcmRouter,
		crate::XcmPallet,
		ConstU32<{ 6 * HOURS }>,
		Self::Beneficiary,
		Self::AssetKind,
		LocatableAssetConverter,
		VersionedLocationConverter,
	>;
	type BalanceConverter = UnityOrOuterConversion<
		ContainsParts<
			FromContains<
				xcm_builder::IsChildSystemTeyrchain<ParaId>,
				xcm_builder::IsParentsOnly<ConstU8<1>>,
			>,
		>,
		AssetRate,
	>;
	type PayoutPeriod = PayoutSpendPeriod;
	type BlockNumberProvider = System;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pezkuwi_runtime_common::impls::benchmarks::TreasuryArguments;
}

impl pezpallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = session_historical::IdentificationTuple<Self>;
	type OnOffenceHandler = StakingAhClient;
}

impl pezpallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const NposSolutionPriority: TransactionPriority = TransactionPriority::max_value() / 2;
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

impl pezframe_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> pezframe_system::offchain::CreateTransactionBase<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type RuntimeCall = RuntimeCall;
	type Extrinsic = UncheckedExtrinsic;
}

impl<LocalCall> pezframe_system::offchain::CreateTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type Extension = TxExtension;

	fn create_transaction(call: RuntimeCall, extension: TxExtension) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_transaction(call, extension)
	}
}

/// Submits a transaction with the node's public and signature type. Adheres to the signed extension
/// format of the chain.
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
		use pezsp_runtime::traits::StaticLookup;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;

		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
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
			pezframe_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(true),
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
		let address = <Runtime as pezframe_system::Config>::Lookup::unlookup(account);
		let transaction = UncheckedExtrinsic::new_signed(call, address, signature, tx_ext);
		Some(transaction)
	}
}

impl<LocalCall> pezframe_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_bare(call)
	}
}

impl<LocalCall> pezframe_system::offchain::CreateAuthorizedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
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
			pezframe_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
			pezframe_system::WeightReclaim::<Runtime>::new(),
		)
	}
}

parameter_types! {
	// Minimum 100 bytes/HEZ deposited (1 CENT/byte)
	pub const BasicDeposit: Balance = 1000 * CENTS;       // 258 bytes on-chain
	pub const ByteDeposit: Balance = deposit(0, 1);
	pub const UsernameDeposit: Balance = deposit(0, 32);
	pub const SubAccountDeposit: Balance = 200 * CENTS;   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pezpallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Slashed = ();
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type UsernameDeposit = UsernameDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
	type RegistrarOrigin = EitherOf<EnsureRoot<Self::AccountId>, GeneralAdmin>;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as Verify>::Signer;
	type UsernameAuthorityOrigin = EnsureRoot<Self::AccountId>;
	type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
	type UsernameGracePeriod = ConstU32<{ 30 * DAYS }>;
	type MaxSuffixLength = ConstU32<7>;
	type MaxUsernameLength = ConstU32<32>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type WeightInfo = weights::pezpallet_identity::WeightInfo<Runtime>;
}

impl pezpallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pezpallet_utility::WeightInfo<Runtime>;
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

parameter_types! {
	pub const ConfigDepositBase: Balance = 500 * CENTS;
	pub const FriendDepositFactor: Balance = 50 * CENTS;
	pub const MaxFriends: u16 = 9;
	pub const RecoveryDeposit: Balance = 500 * CENTS;
}

impl pezpallet_recovery::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type BlockNumberProvider = System;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * CENTS;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pezpallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = weights::pezpallet_vesting::WeightInfo<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

impl pezpallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = weights::pezpallet_sudo::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
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
	TypeInfo,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
	Staking,
	SudoBalances,
	IdentityJudgement,
	CancelProxy,
	Auction,
	NominationPools,
	ParaRegistration,
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
			ProxyType::NonTransfer => matches!(
				c,
				RuntimeCall::System(..) |
				RuntimeCall::Babe(..) |
				RuntimeCall::Timestamp(..) |
				RuntimeCall::Indices(pezpallet_indices::Call::claim{..}) |
				RuntimeCall::Indices(pezpallet_indices::Call::free{..}) |
				RuntimeCall::Indices(pezpallet_indices::Call::freeze{..}) |
				// Specifically omitting Indices `transfer`, `force_transfer`
				// Specifically omitting the entire Balances pezpallet
				RuntimeCall::Staking(..) |
				RuntimeCall::Session(..) |
				RuntimeCall::Grandpa(..) |
				RuntimeCall::Utility(..) |
				RuntimeCall::Identity(..) |
				RuntimeCall::ConvictionVoting(..) |
				RuntimeCall::Referenda(..) |
				RuntimeCall::Whitelist(..) |
				RuntimeCall::Recovery(pezpallet_recovery::Call::as_recovered{..}) |
				RuntimeCall::Recovery(pezpallet_recovery::Call::vouch_recovery{..}) |
				RuntimeCall::Recovery(pezpallet_recovery::Call::claim_recovery{..}) |
				RuntimeCall::Recovery(pezpallet_recovery::Call::close_recovery{..}) |
				RuntimeCall::Recovery(pezpallet_recovery::Call::remove_recovery{..}) |
				RuntimeCall::Recovery(pezpallet_recovery::Call::cancel_recovered{..}) |
				// Specifically omitting Recovery `create_recovery`, `initiate_recovery`
				RuntimeCall::Vesting(pezpallet_vesting::Call::vest{..}) |
				RuntimeCall::Vesting(pezpallet_vesting::Call::vest_other{..}) |
				// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
				RuntimeCall::Scheduler(..) |
				// Specifically omitting Sudo pezpallet
				RuntimeCall::Proxy(..) |
				RuntimeCall::Multisig(..) |
				RuntimeCall::Registrar(paras_registrar::Call::register{..}) |
				RuntimeCall::Registrar(paras_registrar::Call::deregister{..}) |
				// Specifically omitting Registrar `swap`
				RuntimeCall::Registrar(paras_registrar::Call::reserve{..}) |
				RuntimeCall::Crowdloan(..) |
				RuntimeCall::Slots(..) |
				RuntimeCall::Auctions(..) | // Specifically omitting the entire XCM Pezpallet
				RuntimeCall::VoterList(..) |
				RuntimeCall::NominationPools(..) |
				RuntimeCall::FastUnstake(..)
			),
			ProxyType::Staking => {
				matches!(
					c,
					RuntimeCall::Staking(..)
						| RuntimeCall::Session(..)
						| RuntimeCall::Utility(..)
						| RuntimeCall::FastUnstake(..)
						| RuntimeCall::VoterList(..)
						| RuntimeCall::NominationPools(..)
				)
			},
			ProxyType::NominationPools => {
				matches!(c, RuntimeCall::NominationPools(..) | RuntimeCall::Utility(..))
			},
			ProxyType::SudoBalances => match c {
				RuntimeCall::Sudo(pezpallet_sudo::Call::sudo { call: ref x }) => {
					matches!(x.as_ref(), &RuntimeCall::Balances(..))
				},
				RuntimeCall::Utility(..) => true,
				_ => false,
			},
			ProxyType::Governance => matches!(
				c,
				// OpenGov calls
				RuntimeCall::ConvictionVoting(..)
					| RuntimeCall::Referenda(..)
					| RuntimeCall::Whitelist(..)
			),
			ProxyType::IdentityJudgement => matches!(
				c,
				RuntimeCall::Identity(pezpallet_identity::Call::provide_judgement { .. })
					| RuntimeCall::Utility(..)
			),
			ProxyType::CancelProxy => {
				matches!(c, RuntimeCall::Proxy(pezpallet_proxy::Call::reject_announcement { .. }))
			},
			ProxyType::Auction => matches!(
				c,
				RuntimeCall::Auctions(..)
					| RuntimeCall::Crowdloan(..)
					| RuntimeCall::Registrar(..)
					| RuntimeCall::Slots(..)
			),
			ProxyType::ParaRegistration => matches!(
				c,
				RuntimeCall::Registrar(paras_registrar::Call::reserve { .. })
					| RuntimeCall::Registrar(paras_registrar::Call::register { .. })
					| RuntimeCall::Utility(pezpallet_utility::Call::batch { .. })
					| RuntimeCall::Utility(pezpallet_utility::Call::batch_all { .. })
					| RuntimeCall::Utility(pezpallet_utility::Call::force_batch { .. })
					| RuntimeCall::Proxy(pezpallet_proxy::Call::remove_proxy { .. })
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
	type MaxProxies = MaxProxies;
	type WeightInfo = weights::pezpallet_proxy::WeightInfo<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

impl teyrchains_origin::Config for Runtime {}

impl teyrchains_configuration::Config for Runtime {
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_configuration::WeightInfo<Runtime>;
}

impl teyrchains_shared::Config for Runtime {
	type DisabledValidators = Session;
}

impl teyrchains_session_info::Config for Runtime {
	type ValidatorSet = Historical;
}

impl teyrchains_inclusion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DisputesHandler = ParasDisputes;
	type RewardValidators =
		teyrchains_reward_points::RewardValidatorsWithEraPoints<Runtime, StakingAhClient>;
	type MessageQueue = MessageQueue;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_inclusion::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl teyrchains_paras::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_paras::WeightInfo<Runtime>;
	type UnsignedPriority = ParasUnsignedPriority;
	type QueueFootprinter = ParaInclusion;
	type NextSessionRotation = Babe;
	type OnNewHead = ();
	type AssignCoretime = CoretimeAssignmentProvider;
	type Fungible = Balances;
	// Per day the cooldown is removed earlier, it should cost 1000.
	type CooldownRemovalMultiplier = ConstUint<{ 1000 * UNITS / DAYS as u128 }>;
	type AuthorizeCurrentCodeOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		// Collectives DDay plurality mapping.
		AsEnsureOriginWithArg<
			EnsureXcm<IsVoiceOfBody<xcm_config::Collectives, xcm_config::DDayBodyId>>,
		>,
	>;
}

parameter_types! {
	/// Amount of weight that can be spent per block to service messages.
	///
	/// # WARNING
	///
	/// This is not a good value for para-chains since the `Scheduler` already uses up to 80% block weight.
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(20) * BlockWeights::get().max_block;
	pub const MessageQueueHeapSize: u32 = 128 * 1024;
	pub const MessageQueueMaxStale: u32 = 48;
}

/// Message processor to handle any messages that were enqueued into the `MessageQueue` pezpallet.
pub struct MessageProcessor;
impl ProcessMessage for MessageProcessor {
	type Origin = AggregateMessageOrigin;

	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		meter: &mut WeightMeter,
		id: &mut [u8; 32],
	) -> Result<bool, ProcessMessageError> {
		let para = match origin {
			AggregateMessageOrigin::Ump(UmpQueueId::Para(para)) => para,
		};
		xcm_builder::ProcessXcmMessage::<
			Junction,
			xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
			RuntimeCall,
		>::process_message(message, Junction::Teyrchain(para.into()), meter, id)
	}
}

impl pezpallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Size = u32;
	type HeapSize = MessageQueueHeapSize;
	type MaxStale = MessageQueueMaxStale;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = MessageQueueServiceWeight;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = MessageProcessor;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor =
		pezpallet_message_queue::mock_helpers::NoopMessageProcessor<AggregateMessageOrigin>;
	type QueueChangeHandler = ParaInclusion;
	type QueuePausedQuery = ();
	type WeightInfo = weights::pezpallet_message_queue::WeightInfo<Runtime>;
}

impl teyrchains_dmp::Config for Runtime {}

parameter_types! {
	pub const HrmpChannelSizeAndCapacityWithSystemRatio: Percent = Percent::from_percent(100);
}

impl teyrchains_hrmp::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type ChannelManager = EnsureRoot<AccountId>;
	type Currency = Balances;
	type DefaultChannelSizeAndCapacityWithSystem = ActiveConfigHrmpChannelSizeAndCapacityRatio<
		Runtime,
		HrmpChannelSizeAndCapacityWithSystemRatio,
	>;
	type VersionWrapper = crate::XcmPallet;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_hrmp::WeightInfo<Self>;
}

impl teyrchains_paras_inherent::Config for Runtime {
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_paras_inherent::WeightInfo<Runtime>;
}

impl teyrchains_scheduler::Config for Runtime {
	// If you change this, make sure the `Assignment` type of the new provider is binary compatible,
	// otherwise provide a migration.
	type AssignmentProvider = CoretimeAssignmentProvider;
}

parameter_types! {
	pub const BrokerId: u32 = BROKER_ID;
	pub const BrokerPalletId: PalletId = PalletId(*b"py/broke");
	pub MaxXcmTransactWeight: Weight = Weight::from_parts(200_000_000, 20_000);
}

pub struct BrokerPot;
impl Get<InteriorLocation> for BrokerPot {
	fn get() -> InteriorLocation {
		Junction::AccountId32 { network: None, id: BrokerPalletId::get().into_account_truncating() }
			.into()
	}
}

impl coretime::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type BrokerId = BrokerId;
	type BrokerPotLocation = BrokerPot;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_coretime::WeightInfo<Runtime>;
	type SendXcm = crate::xcm_config::XcmRouter;
	type AssetTransactor = crate::xcm_config::LocalAssetTransactor;
	type AccountToLocation = xcm_builder::AliasesIntoAccountId32<
		xcm_config::ThisNetwork,
		<Runtime as pezframe_system::Config>::AccountId,
	>;
	type MaxXcmTransactWeight = MaxXcmTransactWeight;
}

parameter_types! {
	pub const OnDemandTrafficDefaultValue: FixedU128 = FixedU128::from_u32(1);
	// Keep 2 timeslices worth of revenue information.
	pub const MaxHistoricalRevenue: BlockNumber = 2 * TIMESLICE_PERIOD;
	pub const OnDemandPalletId: PalletId = PalletId(*b"py/ondmd");
}

impl teyrchains_on_demand::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type TrafficDefaultValue = OnDemandTrafficDefaultValue;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_on_demand::WeightInfo<Runtime>;
	type MaxHistoricalRevenue = MaxHistoricalRevenue;
	type PalletId = OnDemandPalletId;
}

impl teyrchains_assigner_coretime::Config for Runtime {}

impl teyrchains_initializer::Config for Runtime {
	type Randomness = pezpallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_initializer::WeightInfo<Runtime>;
	type CoretimeOnNewSession = Coretime;
}

impl paras_sudo_wrapper::Config for Runtime {}

parameter_types! {
	pub const PermanentSlotLeasePeriodLength: u32 = 26;
	pub const TemporarySlotLeasePeriodLength: u32 = 1;
	pub const MaxTemporarySlotPerLeasePeriod: u32 = 5;
}

impl assigned_slots::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssignSlotOrigin = EnsureRoot<AccountId>;
	type Leaser = Slots;
	type PermanentSlotLeasePeriodLength = PermanentSlotLeasePeriodLength;
	type TemporarySlotLeasePeriodLength = TemporarySlotLeasePeriodLength;
	type MaxTemporarySlotPerLeasePeriod = MaxTemporarySlotPerLeasePeriod;
	type WeightInfo = weights::pezkuwi_runtime_common_assigned_slots::WeightInfo<Runtime>;
}

impl teyrchains_disputes::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RewardValidators =
		teyrchains_reward_points::RewardValidatorsWithEraPoints<Runtime, StakingAhClient>;
	type SlashingHandler = teyrchains_slashing::SlashValidatorsForDisputes<ParasSlashing>;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_disputes::WeightInfo<Runtime>;
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
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_disputes_slashing::WeightInfo<Runtime>;
	type BenchmarkingConfig = teyrchains_slashing::BenchConfig<300>;
}

parameter_types! {
	pub const ParaDeposit: Balance = 2000 * CENTS;
	pub const RegistrarDataDepositPerByte: Balance = deposit(0, 1);
}

impl paras_registrar::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnSwap = (Crowdloan, Slots, SwapLeases);
	type ParaDeposit = ParaDeposit;
	type DataDepositPerByte = RegistrarDataDepositPerByte;
	type WeightInfo = weights::pezkuwi_runtime_common_paras_registrar::WeightInfo<Runtime>;
}

parameter_types! {
	pub const LeasePeriod: BlockNumber = 28 * DAYS;
}

impl slots::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Registrar = Registrar;
	type LeasePeriod = LeasePeriod;
	type LeaseOffset = ();
	type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, LeaseAdmin>;
	type WeightInfo = weights::pezkuwi_runtime_common_slots::WeightInfo<Runtime>;
}

parameter_types! {
	pub const CrowdloanId: PalletId = PalletId(*b"py/cfund");
	pub const SubmissionDeposit: Balance = 100 * 100 * CENTS;
	pub const MinContribution: Balance = 100 * CENTS;
	pub const RemoveKeysLimit: u32 = 500;
	// Allow 32 bytes for an additional memo to a crowdloan.
	pub const MaxMemoLength: u8 = 32;
}

impl crowdloan::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = CrowdloanId;
	type SubmissionDeposit = SubmissionDeposit;
	type MinContribution = MinContribution;
	type RemoveKeysLimit = RemoveKeysLimit;
	type Registrar = Registrar;
	type Auctioneer = Auctions;
	type MaxMemoLength = MaxMemoLength;
	type WeightInfo = weights::pezkuwi_runtime_common_crowdloan::WeightInfo<Runtime>;
}

parameter_types! {
	// The average auction is 7 days long, so this will be 70% for ending period.
	// 5 Days = 72000 Blocks @ 6 sec per block
	pub const EndingPeriod: BlockNumber = 5 * DAYS;
	// ~ 1000 samples per day -> ~ 20 blocks per sample -> 2 minute samples
	pub const SampleLength: BlockNumber = 2 * MINUTES;
}

impl auctions::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Leaser = Slots;
	type Registrar = Registrar;
	type EndingPeriod = EndingPeriod;
	type SampleLength = SampleLength;
	type Randomness = pezpallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type InitiateOrigin = EitherOf<EnsureRoot<Self::AccountId>, AuctionAdmin>;
	type WeightInfo = weights::pezkuwi_runtime_common_auctions::WeightInfo<Runtime>;
}

impl identity_migrator::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Reaper = EnsureSigned<AccountId>;
	type ReapIdentityHandler = ToTeyrchainIdentityReaper<Runtime, Self::AccountId>;
	type WeightInfo = weights::pezkuwi_runtime_common_identity_migrator::WeightInfo<Runtime>;
}

parameter_types! {
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	pub const MaxPointsToBalance: u8 = 10;
}

impl pezpallet_nomination_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezpallet_nomination_pools::WeightInfo<Self>;
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = BalanceToU256;
	type U256ToBalance = U256ToBalance;
	type StakeAdapter =
		pezpallet_nomination_pools::adapter::DelegateStake<Self, Staking, DelegatedStaking>;
	type PostUnbondingPoolsWindow = ConstU32<4>;
	type MaxMetadataLen = ConstU32<256>;
	// we use the same number of allowed unlocking chunks as with staking.
	type MaxUnbonding = <Self as pezpallet_staking::Config>::MaxUnlockingChunks;
	type PalletId = PoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type AdminOrigin = EitherOf<EnsureRoot<AccountId>, StakingAdmin>;
	type BlockNumberProvider = System;
	type Filter = Nothing;
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

impl pezpallet_root_testing::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

impl pezpallet_root_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OffenceHandler = StakingAhClient;
	type ReportOffence = Offences;
}

parameter_types! {
	pub MbmServiceWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
}

impl pezpallet_migrations::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Migrations = pezpallet_identity::migration::v2::LazyMigrationV1ToV2<Runtime>;
	// Benchmarks need mocked migrations to guarantee that they succeed.
	#[cfg(feature = "runtime-benchmarks")]
	type Migrations = pezpallet_migrations::mock_helpers::MockedMigrations;
	type CursorMaxLen = ConstU32<65_536>;
	type IdentifierMaxLen = ConstU32<256>;
	type MigrationStatusHandler = ();
	type FailedMigrationHandler = pezframe_support::migrations::FreezeChainOnFailedMigration;
	type MaxServiceWeight = MbmServiceWeight;
	type WeightInfo = weights::pezpallet_migrations::WeightInfo<Runtime>;
}

parameter_types! {
	// The deposit configuration for the singed migration. Specially if you want to allow any signed account to do the migration (see `SignedFilter`, these deposits should be high)
	pub const MigrationSignedDepositPerItem: Balance = 1 * CENTS;
	pub const MigrationSignedDepositBase: Balance = 20 * CENTS * 100;
	pub const MigrationMaxKeyLen: u32 = 512;
}

impl pezpallet_asset_rate::Config for Runtime {
	type WeightInfo = weights::pezpallet_asset_rate::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type CreateOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type AssetKind = <Runtime as pezpallet_treasury::Config>::AssetKind;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pezkuwi_runtime_common::impls::benchmarks::AssetRateArguments;
}

// Notify `coretime` pezpallet when a lease swap occurs
pub struct SwapLeases;
impl OnSwap for SwapLeases {
	fn on_swap(one: ParaId, other: ParaId) {
		coretime::Pezpallet::<Runtime>::on_legacy_lease_swap(one, other);
	}
}

pub type MetaTxExtension = (
	pezpallet_verify_signature::VerifySignature<Runtime>,
	pezpallet_meta_tx::MetaTxMarker<Runtime>,
	pezframe_system::CheckNonZeroSender<Runtime>,
	pezframe_system::CheckSpecVersion<Runtime>,
	pezframe_system::CheckTxVersion<Runtime>,
	pezframe_system::CheckGenesis<Runtime>,
	pezframe_system::CheckMortality<Runtime>,
	pezframe_system::CheckNonce<Runtime>,
	pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

impl pezpallet_meta_tx::Config for Runtime {
	type WeightInfo = weights::pezpallet_meta_tx::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Extension = MetaTxExtension;
	#[cfg(feature = "runtime-benchmarks")]
	type Extension = pezpallet_meta_tx::WeightlessExtension<Runtime>;
}

impl pezpallet_verify_signature::Config for Runtime {
	type Signature = MultiSignature;
	type AccountIdentifier = MultiSigner;
	type WeightInfo = weights::pezpallet_verify_signature::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

#[pezframe_support::runtime(legacy_ordering)]
mod runtime {
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

	// Basic stuff; balances is uncallable initially.
	#[runtime::pezpallet_index(0)]
	pub type System = pezframe_system;

	// Babe must be before session.
	#[runtime::pezpallet_index(1)]
	pub type Babe = pezpallet_babe;

	#[runtime::pezpallet_index(2)]
	pub type Timestamp = pezpallet_timestamp;
	#[runtime::pezpallet_index(3)]
	pub type Indices = pezpallet_indices;
	#[runtime::pezpallet_index(4)]
	pub type Balances = pezpallet_balances;
	#[runtime::pezpallet_index(26)]
	pub type TransactionPayment = pezpallet_transaction_payment;

	// Consensus support.
	// Authorship must be before session in order to note author in the correct session and era.
	#[runtime::pezpallet_index(5)]
	pub type Authorship = pezpallet_authorship;
	#[runtime::pezpallet_index(6)]
	pub type Staking = pezpallet_staking;
	#[runtime::pezpallet_index(7)]
	pub type Offences = pezpallet_offences;
	#[runtime::pezpallet_index(27)]
	pub type Historical = session_historical;
	#[runtime::pezpallet_index(70)]
	pub type Parameters = pezpallet_parameters;

	#[runtime::pezpallet_index(8)]
	pub type Session = pezpallet_session;
	#[runtime::pezpallet_index(10)]
	pub type Grandpa = pezpallet_grandpa;
	#[runtime::pezpallet_index(12)]
	pub type AuthorityDiscovery = pezpallet_authority_discovery;

	// Utility module.
	#[runtime::pezpallet_index(16)]
	pub type Utility = pezpallet_utility;

	// Less simple identity module.
	#[runtime::pezpallet_index(17)]
	pub type Identity = pezpallet_identity;

	// Social recovery module.
	#[runtime::pezpallet_index(18)]
	pub type Recovery = pezpallet_recovery;

	// Vesting. Usable initially, but removed once all vesting is finished.
	#[runtime::pezpallet_index(19)]
	pub type Vesting = pezpallet_vesting;

	// System scheduler.
	#[runtime::pezpallet_index(20)]
	pub type Scheduler = pezpallet_scheduler;

	// Preimage registrar.
	#[runtime::pezpallet_index(28)]
	pub type Preimage = pezpallet_preimage;

	// Sudo.
	#[runtime::pezpallet_index(21)]
	pub type Sudo = pezpallet_sudo;

	// Proxy module. Late addition.
	#[runtime::pezpallet_index(22)]
	pub type Proxy = pezpallet_proxy;

	// Multisig module. Late addition.
	#[runtime::pezpallet_index(23)]
	pub type Multisig = pezpallet_multisig;

	// Election pezpallet. Only works with staking, but placed here to maintain indices.
	#[runtime::pezpallet_index(24)]
	pub type ElectionProviderMultiPhase = pezpallet_election_provider_multi_phase;

	// Provides a semi-sorted list of nominators for staking.
	#[runtime::pezpallet_index(25)]
	pub type VoterList = pezpallet_bags_list<Instance1>;

	// Nomination pools for staking.
	#[runtime::pezpallet_index(29)]
	pub type NominationPools = pezpallet_nomination_pools;

	// Fast unstake pezpallet = extension to staking.
	#[runtime::pezpallet_index(30)]
	pub type FastUnstake = pezpallet_fast_unstake;

	// OpenGov
	#[runtime::pezpallet_index(31)]
	pub type ConvictionVoting = pezpallet_conviction_voting;
	#[runtime::pezpallet_index(32)]
	pub type Referenda = pezpallet_referenda;
	#[runtime::pezpallet_index(35)]
	pub type Origins = pezpallet_custom_origins;
	#[runtime::pezpallet_index(36)]
	pub type Whitelist = pezpallet_whitelist;

	// Treasury
	#[runtime::pezpallet_index(37)]
	pub type Treasury = pezpallet_treasury;

	// Staking extension for delegation
	#[runtime::pezpallet_index(38)]
	pub type DelegatedStaking = pezpallet_delegated_staking;

	// Teyrchains pallets. Start indices at 40 to leave room.
	#[runtime::pezpallet_index(41)]
	pub type TeyrchainsOrigin = teyrchains_origin;
	#[runtime::pezpallet_index(42)]
	pub type Configuration = teyrchains_configuration;
	#[runtime::pezpallet_index(43)]
	pub type ParasShared = teyrchains_shared;
	#[runtime::pezpallet_index(44)]
	pub type ParaInclusion = teyrchains_inclusion;
	#[runtime::pezpallet_index(45)]
	pub type ParaInherent = teyrchains_paras_inherent;
	#[runtime::pezpallet_index(46)]
	pub type ParaScheduler = teyrchains_scheduler;
	#[runtime::pezpallet_index(47)]
	pub type Paras = teyrchains_paras;
	#[runtime::pezpallet_index(48)]
	pub type Initializer = teyrchains_initializer;
	#[runtime::pezpallet_index(49)]
	pub type Dmp = teyrchains_dmp;
	// RIP Ump 50
	#[runtime::pezpallet_index(51)]
	pub type Hrmp = teyrchains_hrmp;
	#[runtime::pezpallet_index(52)]
	pub type ParaSessionInfo = teyrchains_session_info;
	#[runtime::pezpallet_index(53)]
	pub type ParasDisputes = teyrchains_disputes;
	#[runtime::pezpallet_index(54)]
	pub type ParasSlashing = teyrchains_slashing;
	#[runtime::pezpallet_index(56)]
	pub type OnDemandAssignmentProvider = teyrchains_on_demand;
	#[runtime::pezpallet_index(57)]
	pub type CoretimeAssignmentProvider = teyrchains_assigner_coretime;

	// Teyrchain Onboarding Pallets. Start indices at 60 to leave room.
	#[runtime::pezpallet_index(60)]
	pub type Registrar = paras_registrar;
	#[runtime::pezpallet_index(61)]
	pub type Slots = slots;
	#[runtime::pezpallet_index(62)]
	pub type ParasSudoWrapper = paras_sudo_wrapper;
	#[runtime::pezpallet_index(63)]
	pub type Auctions = auctions;
	#[runtime::pezpallet_index(64)]
	pub type Crowdloan = crowdloan;
	#[runtime::pezpallet_index(65)]
	pub type AssignedSlots = assigned_slots;
	#[runtime::pezpallet_index(66)]
	pub type Coretime = coretime;
	#[runtime::pezpallet_index(67)]
	pub type StakingAhClient = pezpallet_staking_async_ah_client;

	// Migrations pezpallet
	#[runtime::pezpallet_index(98)]
	pub type MultiBlockMigrations = pezpallet_migrations;

	// Pezpallet for sending XCM.
	#[runtime::pezpallet_index(99)]
	pub type XcmPallet = pezpallet_xcm;

	// Generalized message queue
	#[runtime::pezpallet_index(100)]
	pub type MessageQueue = pezpallet_message_queue;

	// Asset rate.
	#[runtime::pezpallet_index(101)]
	pub type AssetRate = pezpallet_asset_rate;

	// Root testing pezpallet.
	#[runtime::pezpallet_index(102)]
	pub type RootTesting = pezpallet_root_testing;

	#[runtime::pezpallet_index(103)]
	pub type MetaTx = pezpallet_meta_tx::Pezpallet<Runtime>;

	#[runtime::pezpallet_index(104)]
	pub type VerifySignature = pezpallet_verify_signature::Pezpallet<Runtime>;

	// Root offences pezpallet
	#[runtime::pezpallet_index(105)]
	pub type RootOffences = pezpallet_root_offences;

	// BEEFY Bridges support.
	#[runtime::pezpallet_index(200)]
	pub type Beefy = pezpallet_beefy;
	// MMR leaf construction must be after session in order to have a leaf's next_auth_set
	// refer to block<N>. See issue pezkuwi-fellows/runtimes#160 for details.
	#[runtime::pezpallet_index(201)]
	pub type Mmr = pezpallet_mmr;
	#[runtime::pezpallet_index(202)]
	pub type BeefyMmrLeaf = pezpallet_beefy_mmr;

	// Pezpallet for migrating Identity to a teyrchain. To be removed post-migration.
	#[runtime::pezpallet_index(248)]
	pub type IdentityMigrator = identity_migrator;
}

/// The address format for describing accounts.
pub type Address = pezsp_runtime::MultiAddress<AccountId, ()>;
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
	pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
	pezframe_system::WeightReclaim<Runtime>,
);

parameter_types! {
	/// Bounding number of agent pot accounts to be migrated in a single block.
	pub const MaxAgentsToMigrate: u32 = 300;
}

/// All migrations that will run on the next runtime upgrade.
///
/// This contains the combined migrations of the last 10 releases. It allows to skip runtime
/// upgrades in case governance decides to do so. THE ORDER IS IMPORTANT.
pub type Migrations = migrations::Unreleased;

/// The runtime migrations per release.
#[allow(deprecated, missing_docs)]
pub mod migrations {
	use super::*;

	/// Unreleased migrations. Add new ones here:
	pub type Unreleased = (
		// This is only needed for Zagros.
		pezpallet_delegated_staking::migration::unversioned::ProxyDelegatorMigration<
			Runtime,
			MaxAgentsToMigrate,
		>,
		teyrchains_shared::migration::MigrateToV1<Runtime>,
		teyrchains_scheduler::migration::MigrateV2ToV3<Runtime>,
		pezpallet_staking::migrations::v16::MigrateV15ToV16<Runtime>,
		pezpallet_session::migrations::v1::MigrateV0ToV1<
			Runtime,
			pezpallet_staking::migrations::v17::MigrateDisabledToSession<Runtime>,
		>,
		// permanent
		pezpallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
	);
}

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

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	pezframe_benchmarking::define_benchmarks!(
		// Pezkuwi
		// NOTE: Make sure to prefix these with `runtime_common::` so
		// the that path resolves correctly in the generated file.
		[pezkuwi_runtime_common::assigned_slots, AssignedSlots]
		[pezkuwi_runtime_common::auctions, Auctions]
		[pezkuwi_runtime_common::crowdloan, Crowdloan]
		[pezkuwi_runtime_common::identity_migrator, IdentityMigrator]
		[pezkuwi_runtime_common::paras_registrar, Registrar]
		[pezkuwi_runtime_common::slots, Slots]
		[pezkuwi_runtime_teyrchains::configuration, Configuration]
		[pezkuwi_runtime_teyrchains::disputes, ParasDisputes]
		[pezkuwi_runtime_teyrchains::disputes::slashing, ParasSlashing]
		[pezkuwi_runtime_teyrchains::hrmp, Hrmp]
		[pezkuwi_runtime_teyrchains::inclusion, ParaInclusion]
		[pezkuwi_runtime_teyrchains::initializer, Initializer]
		[pezkuwi_runtime_teyrchains::paras, Paras]
		[pezkuwi_runtime_teyrchains::paras_inherent, ParaInherent]
		[pezkuwi_runtime_teyrchains::on_demand, OnDemandAssignmentProvider]
		[pezkuwi_runtime_teyrchains::coretime, Coretime]
		// Bizinikiwi
		[pezpallet_bags_list, VoterList]
		[pezpallet_balances, Balances]
		[pezpallet_beefy_mmr, BeefyMmrLeaf]
		[pezpallet_conviction_voting, ConvictionVoting]
		[pezpallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[pezframe_election_provider_support, ElectionProviderBench::<Runtime>]
		[pezpallet_fast_unstake, FastUnstake]
		[pezpallet_identity, Identity]
		[pezpallet_indices, Indices]
		[pezpallet_message_queue, MessageQueue]
		[pezpallet_migrations, MultiBlockMigrations]
		[pezpallet_mmr, Mmr]
		[pezpallet_multisig, Multisig]
		[pezpallet_nomination_pools, NominationPoolsBench::<Runtime>]
		[pezpallet_offences, OffencesBench::<Runtime>]
		[pezpallet_parameters, Parameters]
		[pezpallet_preimage, Preimage]
		[pezpallet_proxy, Proxy]
		[pezpallet_recovery, Recovery]
		[pezpallet_referenda, Referenda]
		[pezpallet_scheduler, Scheduler]
		[pezpallet_session, SessionBench::<Runtime>]
		[pezpallet_staking, Staking]
		[pezpallet_sudo, Sudo]
		[pezframe_system, SystemBench::<Runtime>]
		[pezframe_system_extensions, SystemExtensionsBench::<Runtime>]
		[pezpallet_timestamp, Timestamp]
		[pezpallet_transaction_payment, TransactionPayment]
		[pezpallet_treasury, Treasury]
		[pezpallet_utility, Utility]
		[pezpallet_vesting, Vesting]
		[pezpallet_whitelist, Whitelist]
		[pezpallet_asset_rate, AssetRate]
		[pezpallet_meta_tx, MetaTx]
		[pezpallet_verify_signature, VerifySignature]
		// XCM
		[pezpallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
		// NOTE: Make sure you point to the individual modules below.
		[pezpallet_xcm_benchmarks::fungible, XcmBalances]
		[pezpallet_xcm_benchmarks::generic, XcmGeneric]
	);
}

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

	#[api_version(15)]
	impl pezkuwi_primitives::runtime_api::TeyrchainHost<Block> for Runtime {
		fn validators() -> Vec<ValidatorId> {
			teyrchains_runtime_api_impl::validators::<Runtime>()
		}

		fn validator_groups() -> (Vec<Vec<ValidatorIndex>>, GroupRotationInfo<BlockNumber>) {
			teyrchains_runtime_api_impl::validator_groups::<Runtime>()
		}

		fn availability_cores() -> Vec<CoreState<Hash, BlockNumber>> {
			teyrchains_runtime_api_impl::availability_cores::<Runtime>()
		}

		fn persisted_validation_data(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<PersistedValidationData<Hash, BlockNumber>> {
			teyrchains_runtime_api_impl::persisted_validation_data::<Runtime>(para_id, assumption)
		}

		fn assumed_validation_data(
			para_id: ParaId,
			expected_persisted_validation_data_hash: Hash,
		) -> Option<(PersistedValidationData<Hash, BlockNumber>, ValidationCodeHash)> {
			teyrchains_runtime_api_impl::assumed_validation_data::<Runtime>(
				para_id,
				expected_persisted_validation_data_hash,
			)
		}

		fn check_validation_outputs(
			para_id: ParaId,
			outputs: pezkuwi_primitives::CandidateCommitments,
		) -> bool {
			teyrchains_runtime_api_impl::check_validation_outputs::<Runtime>(para_id, outputs)
		}

		fn session_index_for_child() -> SessionIndex {
			teyrchains_runtime_api_impl::session_index_for_child::<Runtime>()
		}

		fn validation_code(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<ValidationCode> {
			teyrchains_runtime_api_impl::validation_code::<Runtime>(para_id, assumption)
		}

		fn candidate_pending_availability(para_id: ParaId) -> Option<CommittedCandidateReceipt<Hash>> {
			#[allow(deprecated)]
			teyrchains_runtime_api_impl::candidate_pending_availability::<Runtime>(para_id)
		}

		fn candidate_events() -> Vec<CandidateEvent<Hash>> {
			teyrchains_runtime_api_impl::candidate_events::<Runtime, _>(|ev| {
				match ev {
					RuntimeEvent::ParaInclusion(ev) => {
						Some(ev)
					}
					_ => None,
				}
			})
		}

		fn session_info(index: SessionIndex) -> Option<SessionInfo> {
			teyrchains_runtime_api_impl::session_info::<Runtime>(index)
		}

		fn session_executor_params(session_index: SessionIndex) -> Option<ExecutorParams> {
			teyrchains_runtime_api_impl::session_executor_params::<Runtime>(session_index)
		}

		fn dmq_contents(recipient: ParaId) -> Vec<InboundDownwardMessage<BlockNumber>> {
			teyrchains_runtime_api_impl::dmq_contents::<Runtime>(recipient)
		}

		fn inbound_hrmp_channels_contents(
			recipient: ParaId
		) -> BTreeMap<ParaId, Vec<InboundHrmpMessage<BlockNumber>>> {
			teyrchains_runtime_api_impl::inbound_hrmp_channels_contents::<Runtime>(recipient)
		}

		fn validation_code_by_hash(hash: ValidationCodeHash) -> Option<ValidationCode> {
			teyrchains_runtime_api_impl::validation_code_by_hash::<Runtime>(hash)
		}

		fn on_chain_votes() -> Option<ScrapedOnChainVotes<Hash>> {
			teyrchains_runtime_api_impl::on_chain_votes::<Runtime>()
		}

		fn submit_pvf_check_statement(
			stmt: PvfCheckStatement,
			signature: ValidatorSignature,
		) {
			teyrchains_runtime_api_impl::submit_pvf_check_statement::<Runtime>(stmt, signature)
		}

		fn pvfs_require_precheck() -> Vec<ValidationCodeHash> {
			teyrchains_runtime_api_impl::pvfs_require_precheck::<Runtime>()
		}

		fn validation_code_hash(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<ValidationCodeHash>
		{
			teyrchains_runtime_api_impl::validation_code_hash::<Runtime>(para_id, assumption)
		}

		fn disputes() -> Vec<(SessionIndex, CandidateHash, DisputeState<BlockNumber>)> {
			teyrchains_runtime_api_impl::get_session_disputes::<Runtime>()
		}

		fn unapplied_slashes(
		) -> Vec<(SessionIndex, CandidateHash, slashing::LegacyPendingSlashes)> {
			teyrchains_runtime_api_impl::unapplied_slashes::<Runtime>()
		}

		fn unapplied_slashes_v2(
		) -> Vec<(SessionIndex, CandidateHash, slashing::PendingSlashes)> {
			teyrchains_runtime_api_impl::unapplied_slashes_v2::<Runtime>()
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
			teyrchains_runtime_api_impl::submit_unsigned_slashing_report::<Runtime>(
				dispute_proof,
				key_ownership_proof,
			)
		}

		fn minimum_backing_votes() -> u32 {
			teyrchains_runtime_api_impl::minimum_backing_votes::<Runtime>()
		}

		fn para_backing_state(para_id: ParaId) -> Option<pezkuwi_primitives::async_backing::BackingState> {
			#[allow(deprecated)]
			teyrchains_runtime_api_impl::backing_state::<Runtime>(para_id)
		}

		fn async_backing_params() -> pezkuwi_primitives::AsyncBackingParams {
			#[allow(deprecated)]
			teyrchains_runtime_api_impl::async_backing_params::<Runtime>()
		}

		fn approval_voting_params() -> ApprovalVotingParams {
			teyrchains_runtime_api_impl::approval_voting_params::<Runtime>()
		}

		fn disabled_validators() -> Vec<ValidatorIndex> {
			teyrchains_runtime_api_impl::disabled_validators::<Runtime>()
		}

		fn node_features() -> NodeFeatures {
			teyrchains_runtime_api_impl::node_features::<Runtime>()
		}

		fn claim_queue() -> BTreeMap<CoreIndex, VecDeque<ParaId>> {
			teyrchains_runtime_api_impl::claim_queue::<Runtime>()
		}

		fn candidates_pending_availability(para_id: ParaId) -> Vec<CommittedCandidateReceipt<Hash>> {
			teyrchains_runtime_api_impl::candidates_pending_availability::<Runtime>(para_id)
		}

		fn backing_constraints(para_id: ParaId) -> Option<Constraints> {
			teyrchains_runtime_api_impl::backing_constraints::<Runtime>(para_id)
		}

		fn scheduling_lookahead() -> u32 {
			teyrchains_runtime_api_impl::scheduling_lookahead::<Runtime>()
		}

		fn validation_code_bomb_limit() -> u32 {
			teyrchains_runtime_api_impl::validation_code_bomb_limit::<Runtime>()
		}

		fn para_ids() -> Vec<ParaId> {
			teyrchains_staging_runtime_api_impl::para_ids::<Runtime>()
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
			use codec::Encode;

			Historical::prove((pezsp_consensus_beefy::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(pezsp_consensus_beefy::OpaqueKeyOwnershipProof::new)
		}
	}

	#[api_version(3)]
	impl mmr::MmrApi<Block, Hash, BlockNumber> for Runtime {
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

		fn generate_ancestry_proof(
			prev_block_number: BlockNumber,
			best_known_block_number: Option<BlockNumber>,
		) -> Result<mmr::AncestryProof<mmr::Hash>, mmr::Error> {
			Mmr::generate_ancestry_proof(prev_block_number, best_known_block_number)
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

		fn verify_proof_stateless(
			root: mmr::Hash,
			leaves: Vec<mmr::EncodableOpaqueLeaf>,
			proof: mmr::LeafProof<mmr::Hash>
		) -> Result<(), mmr::Error> {
			let nodes = leaves.into_iter().map(|leaf|mmr::DataOrHash::Data(leaf.into_opaque_leaf())).collect();
			pezpallet_mmr::verify_leaves_proof::<mmr::Hashing, _>(root, nodes, proof)
		}
	}

	impl pezpallet_beefy_mmr::BeefyMmrApi<Block, Hash> for RuntimeApi {
		fn authority_set_proof() -> pezsp_consensus_beefy::mmr::BeefyAuthoritySet<Hash> {
			BeefyMmrLeaf::authority_set_proof()
		}

		fn next_authority_set_proof() -> pezsp_consensus_beefy::mmr::BeefyNextAuthoritySet<Hash> {
			BeefyMmrLeaf::next_authority_set_proof()
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
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				pezsp_runtime::traits::NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: fg_primitives::AuthorityId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
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
			teyrchains_runtime_api_impl::relevant_authority_ids::<Runtime>()
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

	impl xcm_runtime_pezapis::fees::XcmPaymentApi<Block> for Runtime {
		fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {
			let acceptable_assets = vec![AssetId(xcm_config::TokenLocation::get())];
			XcmPallet::query_acceptable_payment_assets(xcm_version, acceptable_assets)
		}

		fn query_weight_to_asset_fee(weight: Weight, asset: VersionedAssetId) -> Result<u128, XcmPaymentApiError> {
			type Trader = <XcmConfig as xcm_executor::Config>::Trader;
			XcmPallet::query_weight_to_asset_fee::<Trader>(weight, asset)
		}

		fn query_xcm_weight(message: VersionedXcm<()>) -> Result<Weight, XcmPaymentApiError> {
			XcmPallet::query_xcm_weight(message)
		}

		fn query_delivery_fees(destination: VersionedLocation, message: VersionedXcm<()>, asset_id: VersionedAssetId) -> Result<VersionedAssets, XcmPaymentApiError> {
			type AssetExchanger = <XcmConfig as xcm_executor::Config>::AssetExchanger;
			XcmPallet::query_delivery_fees::<AssetExchanger>(destination, message, asset_id)
		}
	}

	impl xcm_runtime_pezapis::dry_run::DryRunApi<Block, RuntimeCall, RuntimeEvent, OriginCaller> for Runtime {
		fn dry_run_call(origin: OriginCaller, call: RuntimeCall, result_xcms_version: XcmVersion) -> Result<CallDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			XcmPallet::dry_run_call::<Runtime, xcm_config::XcmRouter, OriginCaller, RuntimeCall>(origin, call, result_xcms_version)
		}

		fn dry_run_xcm(origin_location: VersionedLocation, xcm: VersionedXcm<RuntimeCall>) -> Result<XcmDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			XcmPallet::dry_run_xcm::<xcm_config::XcmRouter>(origin_location, xcm)
		}
	}

	impl xcm_runtime_pezapis::conversions::LocationToAccountApi<Block, AccountId> for Runtime {
		fn convert_location(location: VersionedLocation) -> Result<
			AccountId,
			xcm_runtime_pezapis::conversions::Error
		> {
			xcm_runtime_pezapis::conversions::LocationToAccountHelper::<
				AccountId,
				xcm_config::LocationConverter,
			>::convert_location(location)
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

	#[cfg(feature = "try-runtime")]
	impl pezframe_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: pezframe_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade zagros.");
		  // TODO:: remove once https://github.com/pezkuwichain/pezkuwi-sdk/issues/302 is resolved.
			let excluded_pallets = vec![
				b"Staking".to_vec(),          // replaced by staking-async
				b"NominationPools".to_vec(),  // moved to AH
				b"FastUnstake".to_vec(),      // deprecated
				b"DelegatedStaking".to_vec(), // moved to AH
			];
			let config = pezframe_executive::TryRuntimeUpgradeConfig::new(checks)
				.with_try_state_select(pezframe_try_runtime::TryStateSelect::AllExcept(
					excluded_pallets,
				));
			let weight = Executive::try_runtime_upgrade_with_config(config).unwrap();
			(weight, BlockWeights::get().max_block)
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

			use pezpallet_session_benchmarking::Pezpallet as SessionBench;
			use pezpallet_offences_benchmarking::Pezpallet as OffencesBench;
			use pezpallet_election_provider_support_benchmarking::Pezpallet as ElectionProviderBench;
			use pezpallet_xcm::benchmarking::Pezpallet as PalletXcmExtrinsicsBenchmark;
			use pezframe_system_benchmarking::Pezpallet as SystemBench;
			use pezframe_system_benchmarking::extensions::Pezpallet as SystemExtensionsBench;
			use pezpallet_nomination_pools_benchmarking::Pezpallet as NominationPoolsBench;

			type XcmBalances = pezpallet_xcm_benchmarks::fungible::Pezpallet::<Runtime>;
			type XcmGeneric = pezpallet_xcm_benchmarks::generic::Pezpallet::<Runtime>;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			return (list, storage_info)
		}

		#[allow(non_local_definitions)]
		fn dispatch_benchmark(
			config: pezframe_benchmarking::BenchmarkConfig,
		) -> Result<
			Vec<pezframe_benchmarking::BenchmarkBatch>,
			alloc::string::String,
		> {
			use pezframe_support::traits::WhitelistedStorageKeys;
			use pezframe_benchmarking::{BenchmarkBatch, BenchmarkError};
			use pezsp_storage::TrackedStorageKey;
			// Trying to add benchmarks directly to some pallets caused cyclic dependency issues.
			// To get around that, we separated the benchmarks into its own crate.
			use pezpallet_session_benchmarking::Pezpallet as SessionBench;
			use pezpallet_offences_benchmarking::Pezpallet as OffencesBench;
			use pezpallet_election_provider_support_benchmarking::Pezpallet as ElectionProviderBench;
			use pezpallet_xcm::benchmarking::Pezpallet as PalletXcmExtrinsicsBenchmark;
			use pezframe_system_benchmarking::Pezpallet as SystemBench;
			use pezframe_system_benchmarking::extensions::Pezpallet as SystemExtensionsBench;
			use pezpallet_nomination_pools_benchmarking::Pezpallet as NominationPoolsBench;

			impl pezpallet_session_benchmarking::Config for Runtime {}
			impl pezpallet_offences_benchmarking::Config for Runtime {}
			impl pezpallet_election_provider_support_benchmarking::Config for Runtime {}

			use xcm_config::{AssetHub, TokenLocation};

			use alloc::boxed::Box;

			parameter_types! {
				pub ExistentialDepositAsset: Option<Asset> = Some((
					TokenLocation::get(),
					ExistentialDeposit::get()
				).into());
				pub AssetHubParaId: ParaId = zagros_runtime_constants::system_teyrchain::ASSET_HUB_ID.into();
				pub const RandomParaId: ParaId = ParaId::new(43211234);
			}

			impl pezpallet_xcm::benchmarking::Config for Runtime {
				type DeliveryHelper = (
					pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
						xcm_config::XcmConfig,
						ExistentialDepositAsset,
						xcm_config::PriceForChildTeyrchainDelivery,
						AssetHubParaId,
						Dmp,
					>,
					pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
						xcm_config::XcmConfig,
						ExistentialDepositAsset,
						xcm_config::PriceForChildTeyrchainDelivery,
						RandomParaId,
						Dmp,
					>
				);

				fn reachable_dest() -> Option<Location> {
					Some(crate::xcm_config::AssetHub::get())
				}

				fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
					// Relay/native token can be teleported to/from AH.
					Some((
						Asset { fun: Fungible(ExistentialDeposit::get()), id: AssetId(Here.into()) },
						crate::xcm_config::AssetHub::get(),
					))
				}

				fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
					None
				}

				fn set_up_complex_asset_transfer(
				) -> Option<(Assets, AssetId, Location, Box<dyn FnOnce()>)> {
					// Relay supports only native token, either reserve transfer it to non-system teyrchains,
					// or teleport it to system teyrchain. Use the teleport case for benchmarking as it's
					// slightly heavier.

					// Relay/native token can be teleported to/from AH.
					let native_location = Here.into();
					let dest = crate::xcm_config::AssetHub::get();
					pezpallet_xcm::benchmarking::helpers::native_teleport_as_asset_transfer::<Runtime>(
						native_location,
						dest
					)
				}

				fn get_asset() -> Asset {
					Asset {
						id: AssetId(Location::here()),
						fun: Fungible(ExistentialDeposit::get()),
					}
				}
			}
			impl pezframe_system_benchmarking::Config for Runtime {}
			impl pezpallet_nomination_pools_benchmarking::Config for Runtime {}
			impl pezkuwi_runtime_teyrchains::disputes::slashing::benchmarking::Config for Runtime {}

			use xcm::latest::{
				AssetId, Fungibility::*, InteriorLocation, Junction, Junctions::*,
				Asset, Assets, Location, NetworkId, Response,
			};

			impl pezpallet_xcm_benchmarks::Config for Runtime {
				type XcmConfig = xcm_config::XcmConfig;
				type AccountIdConverter = xcm_config::LocationConverter;
				type DeliveryHelper = pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
					xcm_config::XcmConfig,
					ExistentialDepositAsset,
					xcm_config::PriceForChildTeyrchainDelivery,
					AssetHubParaId,
					Dmp,
				>;
				fn valid_destination() -> Result<Location, BenchmarkError> {
					Ok(AssetHub::get())
				}
				fn worst_case_holding(_depositable_count: u32) -> Assets {
					// Zagros only knows about ZGR.
					vec![Asset{
						id: AssetId(TokenLocation::get()),
						fun: Fungible(1_000_000 * UNITS),
					}].into()
				}
			}

			parameter_types! {
				pub TrustedTeleporter: Option<(Location, Asset)> = Some((
					AssetHub::get(),
					Asset { fun: Fungible(1 * UNITS), id: AssetId(TokenLocation::get()) },
				));
				pub const TrustedReserve: Option<(Location, Asset)> = None;
				pub const CheckedAccount: Option<(AccountId, xcm_builder::MintLocation)> = None;
			}

			impl pezpallet_xcm_benchmarks::fungible::Config for Runtime {
				type TransactAsset = Balances;

				type CheckedAccount = CheckedAccount;
				type TrustedTeleporter = TrustedTeleporter;
				type TrustedReserve = TrustedReserve;

				fn get_asset() -> Asset {
					Asset {
						id: AssetId(TokenLocation::get()),
						fun: Fungible(1 * UNITS),
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
					// Zagros doesn't support asset exchanges
					Err(BenchmarkError::Skip)
				}

				fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
					// The XCM executor of Zagros doesn't have a configured `UniversalAliases`
					Err(BenchmarkError::Skip)
				}

				fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
					Ok((AssetHub::get(), pezframe_system::Call::remark_with_event { remark: vec![] }.into()))
				}

				fn subscribe_origin() -> Result<Location, BenchmarkError> {
					Ok(AssetHub::get())
				}

				fn claimable_asset() -> Result<(Location, Location, Assets), BenchmarkError> {
					let origin = AssetHub::get();
					let assets: Assets = (AssetId(TokenLocation::get()), 1_000 * UNITS).into();
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
					// Zagros doesn't support asset locking
					Err(BenchmarkError::Skip)
				}

				fn export_message_origin_and_destination(
				) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
					// Zagros doesn't support exporting messages
					Err(BenchmarkError::Skip)
				}

				fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
					let origin = Location::new(0, [Teyrchain(1000)]);
					let target = Location::new(0, [Teyrchain(1000), AccountId32 { id: [128u8; 32], network: None }]);
					Ok((origin, target))
				}
			}

			type XcmBalances = pezpallet_xcm_benchmarks::fungible::Pezpallet::<Runtime>;
			type XcmGeneric = pezpallet_xcm_benchmarks::generic::Pezpallet::<Runtime>;

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
		fn is_trusted_reserve(asset: VersionedAsset, location: VersionedLocation) -> Result<bool, xcm_runtime_pezapis::trusted_query::Error> {
			XcmPallet::is_trusted_reserve(asset, location)
		}
		fn is_trusted_teleporter(asset: VersionedAsset, location: VersionedLocation) -> Result<bool, xcm_runtime_pezapis::trusted_query::Error> {
			XcmPallet::is_trusted_teleporter(asset, location)
		}
	}
}
