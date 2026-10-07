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

///! Staking, and election related pezpallet configurations.
use super::*;
use pezcumulus_primitives_core::relay_chain::SessionIndex;
use pezframe_election_provider_support::{ElectionDataProvider, SequentialPhragmen};
use pezframe_support::traits::tokens::imbalance::ResolveTo;
use pezkuwi_runtime_common::{prod_or_fast, BalanceToU256, U256ToBalance};
use pezpallet_election_provider_multi_block::{self as multi_block, SolutionAccuracyOf};
use pezpallet_staking_async::UseValidatorsMap;
use pezpallet_staking_async_rc_client as rc_client;
use pezsp_runtime::{
	transaction_validity::TransactionPriority, FixedPointNumber, FixedU128, SaturatedConversion,
};
use xcm::latest::prelude::*;

parameter_types! {
	/// Number of election pages that we operate upon. 32 * 6s block = 192s = 3.2min snapshots
	pub Pages: u32 = 32;

	/// Compatible with Pezkuwi, we allow up to 22_500 nominators to be considered for election
	pub MaxElectingVoters: u32 = 22_500;

	/// Maximum number of validators that we may want to elect. 1000 is the end target.
	pub const MaxValidatorSet: u32 = 1000;

	/// Number of nominators per page of the snapshot, and consequently number of backers in the solution.
	pub VoterSnapshotPerBlock: u32 = MaxElectingVoters::get() / Pages::get();

	/// Number of validators per page of the snapshot.
	pub TargetSnapshotPerBlock: u32 = MaxValidatorSet::get();

	// 10 mins for each pages
	pub storage SignedPhase: u32 = prod_or_fast!(
		10 * MINUTES,
		4 * MINUTES
	);
	pub storage UnsignedPhase: u32 = prod_or_fast!(
		10 * MINUTES,
		(1 * MINUTES)
	);

	/// validate up to 4 signed solution. Each solution.
	pub storage SignedValidationPhase: u32 = prod_or_fast!(Pages::get() * 4, Pages::get());

	/// Abandon an election that has produced nothing usable for this long, and start over.
	///
	/// A full election takes snapshot (`Pages` + 1) + signed + signed-validation + unsigned +
	/// export, roughly 400 blocks here. With `AreWeDone = RevertToSignedIfNotQueuedOf` a failing
	/// election loops back to the signed phase instead of ever returning to `Phase::Off`, which is
	/// what let round 673 stay frozen for four days in July 2026 — staking's own stall detection
	/// waits for an *idle* election, so it could never count. Two hours leaves room for about three
	/// honest attempts before the round is given up on.
	pub storage StalledRoundTimeout: BlockNumber = prod_or_fast!(2 * HOURS, 10 * MINUTES);

	/// In each page, we may observe up to all of the validators.
	pub MaxWinnersPerPage: u32 = MaxValidatorSet::get();

	/// In each page of the election, we allow up to all of the nominators of that page to be present.
	pub MaxBackersPerWinner: u32 = VoterSnapshotPerBlock::get();

	/// Total number of backers per winner across all pages.
	pub MaxBackersPerWinnerFinal: u32 = MaxElectingVoters::get();

	/// Size of the exposures. This should be small enough to make the reward payouts feasible.
	pub MaxExposurePageSize: u32 = 512;
}

pezframe_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution16::<
		// allows up to 4bn nominators
		VoterIndex = u32,
		// allows up to 64k validators
		TargetIndex = u16,
		Accuracy = pezsp_runtime::PerU16,
		MaxVoters = VoterSnapshotPerBlock,
	>(16)
);

ord_parameter_types! {
	// Reference account, carried over with the upstream constant: 5GBoBNFP9TA7nAk82i6SUZJimerbdhxaRgyC2PVcdYQMdb8e
	pub const ZagrosStakingMiner: AccountId = AccountId::from(hex_literal::hex!("b65991822483a6c3bd24b1dcf6afd3e270525da1f9c8c22a4373d1e1079e236a"));
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub BenchElectionBounds: pezframe_election_provider_support::bounds::ElectionBounds =
		pezframe_election_provider_support::bounds::ElectionBoundsBuilder::default().build();
}

#[cfg(feature = "runtime-benchmarks")]
pub struct OnChainConfig;

#[cfg(feature = "runtime-benchmarks")]
impl pezframe_election_provider_support::onchain::Config for OnChainConfig {
	// unbounded
	type Bounds = BenchElectionBounds;
	// We should not need sorting, as our bounds are large enough for the number of
	// nominators/validators in this test setup.
	type Sort = ConstBool<false>;
	type DataProvider = Staking;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type MaxWinnersPerPage = MaxWinnersPerPage;
	type Solver = pezframe_election_provider_support::SequentialPhragmen<AccountId, Perbill>;
	type System = Runtime;
	type WeightInfo = ();
}

impl multi_block::Config for Runtime {
	type StalledRoundTimeout = StalledRoundTimeout;
	type Signed = MultiBlockElectionSigned;
	type Pages = Pages;
	type UnsignedPhase = UnsignedPhase;
	type SignedPhase = SignedPhase;
	type SignedValidationPhase = SignedValidationPhase;
	type VoterSnapshotPerBlock = VoterSnapshotPerBlock;
	type TargetSnapshotPerBlock = TargetSnapshotPerBlock;
	type AdminOrigin =
		EitherOfDiverse<EnsureRoot<AccountId>, EnsureSignedBy<ZagrosStakingMiner, AccountId>>;
	type ManagerOrigin =
		EitherOfDiverse<EnsureRoot<AccountId>, EnsureSignedBy<ZagrosStakingMiner, AccountId>>;
	type DataProvider = Staking;
	type MinerConfig = Self;
	type Verifier = MultiBlockElectionVerifier;
	// we chill and do nothing in the fallback.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Fallback = multi_block::Continue<Self>;
	#[cfg(feature = "runtime-benchmarks")]
	type Fallback = pezframe_election_provider_support::onchain::OnChainExecution<OnChainConfig>;
	// Revert back to signed phase if nothing is submitted and queued, so we prolong the election.
	type AreWeDone = multi_block::RevertToSignedIfNotQueuedOf<Self>;
	type OnRoundRotation = multi_block::CleanRound<Self>;
	type WeightInfo = weights::pezpallet_election_provider_multi_block::WeightInfo<Runtime>;
}

impl multi_block::verifier::Config for Runtime {
	type MaxWinnersPerPage = MaxWinnersPerPage;
	type MaxBackersPerWinner = MaxBackersPerWinner;
	type MaxBackersPerWinnerFinal = MaxBackersPerWinnerFinal;
	type SolutionDataProvider = MultiBlockElectionSigned;
	type WeightInfo =
		weights::pezpallet_election_provider_multi_block_verifier::WeightInfo<Runtime>;
}

parameter_types! {
	pub BailoutGraceRatio: Perbill = Perbill::from_percent(50);
	pub EjectGraceRatio: Perbill = Perbill::from_percent(50);
	pub DepositBase: Balance = 5 * UNITS;
	pub DepositPerPage: Balance = 1 * UNITS;
	pub RewardBase: Balance = 10 * UNITS;
	pub MaxSubmissions: u32 = 8;
}

impl multi_block::signed::Config for Runtime {
	type Currency = Balances;
	type BailoutGraceRatio = BailoutGraceRatio;
	type EjectGraceRatio = EjectGraceRatio;
	type DepositBase = DepositBase;
	type DepositPerPage = DepositPerPage;
	type InvulnerableDeposit = ();
	type RewardBase = RewardBase;
	type MaxSubmissions = MaxSubmissions;
	type EstimateCallFee = TransactionPayment;
	type WeightInfo = weights::pezpallet_election_provider_multi_block_signed::WeightInfo<Runtime>;
}

parameter_types! {
	/// Priority of the offchain miner transactions.
	pub MinerTxPriority: TransactionPriority = TransactionPriority::max_value() / 2;
	/// Try and run the OCW miner 4 times during the unsigned phase.
	pub OffchainRepeat: BlockNumber = UnsignedPhase::get() / 4;
	/// Pages the offchain miner puts in its one unsigned transaction. The `submit_unsigned`
	/// benchmark mines exactly this many pages. At 32 it measured 12 MB of proof against the
	/// 10 MiB `MAX_POV_SIZE` -- the unsigned phase could never land a solution; at 2 it is
	/// 417 KB.
	/// Upstream's Asset Hub uses 2; the signed phase covers the pages the miner leaves out.
	pub storage MinerPages: u32 = 2;
}

impl multi_block::unsigned::Config for Runtime {
	type MinerPages = MinerPages;
	type OffchainStorage = ConstBool<true>;
	type OffchainSolver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Runtime>>;
	type MinerTxPriority = MinerTxPriority;
	type OffchainRepeat = OffchainRepeat;
	type WeightInfo =
		weights::pezpallet_election_provider_multi_block_unsigned::WeightInfo<Runtime>;
}

parameter_types! {
	/// Miner transaction can fill up to 75% of the block size.
	pub MinerMaxLength: u32 = Perbill::from_rational(75u32, 100) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);
}

impl multi_block::unsigned::miner::MinerConfig for Runtime {
	type AccountId = AccountId;
	type Hash = Hash;
	type MaxBackersPerWinner = <Self as multi_block::verifier::Config>::MaxBackersPerWinner;
	type MaxBackersPerWinnerFinal =
		<Self as multi_block::verifier::Config>::MaxBackersPerWinnerFinal;
	type MaxWinnersPerPage = <Self as multi_block::verifier::Config>::MaxWinnersPerPage;
	type MaxVotesPerVoter =
		<<Self as multi_block::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
	type MaxLength = MinerMaxLength;
	type Pages = Pages;
	type Solution = NposCompactSolution16;
	type VoterSnapshotPerBlock = <Runtime as multi_block::Config>::VoterSnapshotPerBlock;
	type TargetSnapshotPerBlock = <Runtime as multi_block::Config>::TargetSnapshotPerBlock;
	// for prod, use whatever solver we are using in the miner -- phragmen algorithm
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Solver = <Runtime as multi_block::unsigned::Config>::OffchainSolver;
	// for benchmarks, use the faster solver
	#[cfg(feature = "runtime-benchmarks")]
	type Solver = pezframe_election_provider_support::QuickDirtySolver<AccountId, Perbill>;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &bag_thresholds::THRESHOLDS;
	pub const AutoRebagNumber: u32 = 10;
}

type VoterBagsListInstance = pezpallet_bags_list::Instance1;
impl pezpallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = Staking;
	type BagThresholds = BagThresholds;
	type Score = pezsp_npos_elections::VoteWeight;
	type MaxAutoRebagPerBlock = AutoRebagNumber;
	type WeightInfo = weights::pezpallet_bags_list::WeightInfo<Runtime>;
}

/// The most this chain will emit in a year, whatever the parameter says.
///
/// The rate is policy and lives in storage; this is not. A ceiling the same body could raise
/// is not a ceiling, so it is compiled in and only a runtime upgrade moves it -- which is the
/// distinction the whole arrangement rests on: the constitution is code, policy is storage.
pub const MAX_INFLATION_RATE: Perbill = Perbill::from_percent(10);

/// The base the emission is measured against: 200M HEZ at twelve decimals.
///
/// Not a parameter. How much HEZ there is meant to be is the token's identity rather than a
/// policy about it, and measuring emission against a movable base would make the rate mean
/// nothing.
pub const HEZ_ISSUANCE_BASE: u128 = 200_000_000_000_000_000_000;

/// The election's targets: the committee People seated, among those who validate here.
///
/// The Asset Hub runs its own election only to build the exposures -- who backs whom, and with
/// how much -- that payout and slashing read. Who sits is People's to decide, so the targets are
/// the validators in People's latest snapshot. The filter steps aside when there is nothing to
/// filter by -- no snapshot yet (genesis, or People silent since launch), or a snapshot none of
/// whose members validates here -- because an election with no target stops the era clock, and
/// work by anyone outside the committee is paid to the treasury, not to them.
///
/// `count` is the whole validator set: staking reads it as an upper bound when it sizes the
/// target snapshot, and `try_state` holds it equal to `Validators`' count.
pub struct CommitteeTargets;
type AllValidators = UseValidatorsMap<Runtime>;

impl CommitteeTargets {
	/// Whether the latest snapshot names at least one validator here.
	fn filtering() -> bool {
		pezpallet_komite::Committee::<Runtime>::get().map_or(false, |s| {
			s.members
				.iter()
				.any(|(m, _)| pezpallet_staking_async::Validators::<Runtime>::contains_key(m))
		})
	}
}

impl pezframe_election_provider_support::SortedListProvider<AccountId> for CommitteeTargets {
	type Error =
		<AllValidators as pezframe_election_provider_support::SortedListProvider<AccountId>>::Error;
	type Score =
		<AllValidators as pezframe_election_provider_support::SortedListProvider<AccountId>>::Score;

	fn iter() -> alloc::boxed::Box<dyn Iterator<Item = AccountId>> {
		if !Self::filtering() {
			return AllValidators::iter();
		}
		alloc::boxed::Box::new(AllValidators::iter().filter(|v| Komite::is_member(v)))
	}
	fn iter_from(
		start: &AccountId,
	) -> Result<alloc::boxed::Box<dyn Iterator<Item = AccountId>>, Self::Error> {
		let all = AllValidators::iter_from(start)?;
		if !Self::filtering() {
			return Ok(all);
		}
		Ok(alloc::boxed::Box::new(all.filter(|v| Komite::is_member(v))))
	}
	fn lock() {
		AllValidators::lock()
	}
	fn unlock() {
		AllValidators::unlock()
	}
	fn count() -> u32 {
		AllValidators::count()
	}
	fn contains(id: &AccountId) -> bool {
		AllValidators::contains(id) && (!Self::filtering() || Komite::is_member(id))
	}
	fn on_insert(id: AccountId, score: Self::Score) -> Result<(), Self::Error> {
		AllValidators::on_insert(id, score)
	}
	fn on_update(id: &AccountId, score: Self::Score) -> Result<(), Self::Error> {
		AllValidators::on_update(id, score)
	}
	fn get_score(id: &AccountId) -> Result<Self::Score, Self::Error> {
		AllValidators::get_score(id)
	}
	fn on_remove(id: &AccountId) -> Result<(), Self::Error> {
		AllValidators::on_remove(id)
	}
	fn unsafe_regenerate(
		all: impl IntoIterator<Item = AccountId>,
		score_of: alloc::boxed::Box<dyn Fn(&AccountId) -> Option<Self::Score>>,
	) -> u32 {
		AllValidators::unsafe_regenerate(all, score_of)
	}
	fn unsafe_clear() {
		AllValidators::unsafe_clear()
	}
	#[cfg(feature = "try-runtime")]
	fn try_state() -> Result<(), pezsp_runtime::TryRuntimeError> {
		AllValidators::try_state()
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn score_update_worst_case(who: &AccountId, is_increase: bool) -> Self::Score {
		AllValidators::score_update_worst_case(who, is_increase)
	}
}

/// Ask the election for as many winners as the committee has validators here, so a member who
/// has not started validating cannot make the election fail for want of a target. A committee
/// with none leaves the count it last asked for, as `CommitteeTargets` leaves the targets.
pub struct SetValidatorCountToSeatedValidators;
impl pezpallet_komite::OnCommittee<AccountId> for SetValidatorCountToSeatedValidators {
	fn on_committee(members: &[(AccountId, u128)]) {
		let here = members
			.iter()
			.filter(|(m, _)| pezpallet_staking_async::Validators::<Runtime>::contains_key(m))
			.count() as u32;
		if here > 0 {
			pezpallet_staking_async::ValidatorCount::<Runtime>::put(here);
		}
	}
}

pub struct EraPayout;
impl pezpallet_staking_async::EraPayout<Balance> for EraPayout {
	/// Neither argument is read, and the names say so.
	///
	/// Upstream's payout is a function of the staking ratio: emit more when little is staked,
	/// less when much is. This one is a flat share of a fixed base, so how much is staked and
	/// how much exists change nothing. The comment on `pezpallet_staking_async::Config` below
	/// used to describe the upstream behaviour as if it were this one.
	fn era_payout(
		_total_staked: Balance,
		_total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		const MILLISECONDS_PER_YEAR: u64 = (1000 * 3600 * 24 * 36525) / 100;
		// A normal-sized era will have 1 / 365.25 here:
		let relative_era_len =
			FixedU128::from_rational(era_duration_millis.into(), MILLISECONDS_PER_YEAR.into());

		use pezframe_support::traits::Get;
		let rate = crate::dynamic_params::hez::InflationRate::get().min(MAX_INFLATION_RATE);
		let yearly_emission = rate.mul_floor(HEZ_ISSUANCE_BASE);

		let era_emission = relative_era_len.saturating_mul_int(yearly_emission);
		let to_treasury = crate::dynamic_params::hez::TreasuryShare::get().mul_floor(era_emission);
		let to_stakers = era_emission.saturating_sub(to_treasury);

		(to_stakers.saturated_into(), to_treasury.saturated_into())
	}
}

parameter_types! {
	// Six sessions per era; the era itself lasts five hours (see `era_length`).
	pub const SessionsPerEra: SessionIndex = prod_or_fast!(6, 2);
	/// Duration of a relay session in our blocks. Needs to be hardcoded per-runtime.
	pub const RelaySessionDuration: BlockNumber = 1 * HOURS;
	// 2 eras for unbonding (10 hours).
	pub const BondingDuration: pezsp_staking::EraIndex = 2;
	// 1 era in which slashes can be cancelled (5 hours).
	pub const SlashDeferDuration: pezsp_staking::EraIndex = 1;
	pub const MaxControllersInDeprecationBatch: u32 = 751;
	// alias for 16, which is the max nominations per nominator in the runtime.
	pub const MaxNominations: u32 = <NposCompactSolution16 as pezframe_election_provider_support::NposSolution>::LIMIT as u32;
	pub const MaxEraDuration: u64 = RelaySessionDuration::get() as u64 * RELAY_CHAIN_SLOT_DURATION_MILLIS as u64 * SessionsPerEra::get() as u64;
	pub MaxPruningItems: u32 = 100;
}

parameter_types! {
	/// Era reward pots, used only when minting is disabled. Kept distinct from `PotId`.
	pub const StakingPotsPalletId: PalletId = PalletId(*b"py/stkpt");
	/// Eras a nominator must wait to fast-unbond. Matches upstream.
	pub const NominatorFastUnbondDuration: pezsp_staking::EraIndex = 2;
}

impl pezpallet_staking_async::Config for Runtime {
	// Upstream added a non-minting reward mode where staking pays out of a pre-funded pot
	// instead of creating tokens. This runtime keeps the legacy minting mode: inflation here
	// is a flat share of a fixed 200M base (see EraPayout above), not a function of the
	// staking ratio -- the sentence that used to stand here described upstream, not this,
	// which is exactly the case the pallet documents legacy mode as being kept for. The switch
	// to non-minting is one-way - once eras carry funded pots, going back would orphan them and
	// double-mint - so it is not a default to drift into. Revisit with the genesis spec.
	type DisableMinting = ConstBool<false>;
	// Only read in non-minting mode, but the pot addresses must still be well-defined and
	// distinct from the existing PotStake account, so they get their own id rather than
	// borrowing one.
	type RewardPots = pezpallet_staking_async::Seed<StakingPotsPalletId>;
	// Non-minting mode hands expired unclaimed rewards here; in minting mode they are never
	// created, so there is nothing to route.
	type UnclaimedRewardHandler = ();
	type StakerRewardCalculator =
		pezpallet_staking_async::reward::DefaultStakerRewardCalculator<Runtime>;
	type NominatorFastUnbondDuration = NominatorFastUnbondDuration;
	type Filter = ();
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type RuntimeHoldReason = RuntimeHoldReason;
	// U128, not Saturating: HEZ has 12 decimals and total issuance (~204M HEZ = ~2.04e20 planck)
	// far exceeds u64::MAX (~1.8446e19). SaturatingCurrencyToVote casts balance->u64 saturating, so
	// any stash bonding more than ~18.45M HEZ collapses to u64::MAX. Two large nominators (~41M and
	// ~40M HEZ) both saturated to u64::MAX, becoming tied voters; the resulting tied edge weights hit
	// the `reduce_4` "duplicate/corrupt input" panic in the offchain election miner, so no solution
	// was ever produced and no era exposures were written. U128CurrencyToVote scales by
	// (total_issuance / u64::MAX) instead, keeping every stake distinct and inside u64 range.
	type CurrencyToVote = pezsp_staking::currency_to_vote::U128CurrencyToVote;
	type RewardRemainder = ResolveTo<xcm_config::TreasuryAccount, Balances>;
	type Slash = ResolveTo<xcm_config::TreasuryAccount, Balances>;
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = EnsureRoot<AccountId>;
	type EraPayout = EraPayout;
	type MaxExposurePageSize = MaxExposurePageSize;
	type ElectionProvider = MultiBlockElection;
	type VoterList = VoterList;
	type TargetList = CommitteeTargets;
	type MaxValidatorSet = MaxValidatorSet;
	type NominationsQuota =
		pezpallet_staking_async::FixedNominationsQuota<{ MaxNominations::get() }>;
	type MaxUnlockingChunks = pezframe_support::traits::ConstU32<32>;
	type HistoryDepth = pezframe_support::traits::ConstU32<84>;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type EventListeners = (NominationPools, DelegatedStaking);
	type PlanningEraOffset =
		pezpallet_staking_async::PlanningEraOffsetOf<Runtime, RelaySessionDuration, ConstU32<5>>;
	type RcClientInterface = ElectionStaysHome;
	type MaxEraDuration = MaxEraDuration;
	type MaxPruningItems = MaxPruningItems;
	type WeightInfo = weights::pezpallet_staking_async::WeightInfo<Runtime>;
}

// Must mirror the relay chain's `SessionKeys` exactly - same fields, same order - because the
// validator set is decoded on this side using this definition. Verified against
// pezkuwi/runtime/{zagros,pezkuwichain}/src/lib.rs, which declare these six in this order.
pezsp_runtime::impl_opaque_keys! {
	pub struct RelayChainSessionKeys {
		pub grandpa: pezsp_consensus_grandpa::AuthorityId,
		pub babe: pezsp_consensus_babe::AuthorityId,
		pub para_validator: pezkuwi_primitives::ValidatorId,
		pub para_assignment: pezkuwi_primitives::AssignmentId,
		pub authority_discovery: pezsp_authority_discovery::AuthorityId,
		pub beefy: pezsp_consensus_beefy::ecdsa_crypto::AuthorityId,
	}
}

impl pezpallet_staking_async_rc_client::Config for Runtime {
	// Export the validator set at the end of session 4 within an era, as upstream does.
	type ValidatorSetExportSession = ConstU32<4>;
	type RelayChainSessionKeys = RelayChainSessionKeys;
	type Currency = Balances;
	// Held while a validator's session keys are registered here. Six keys plus SCALE overhead;
	// upstream charges 10 UNITS for the same payload and this chain's UNITS is the same size.
	type KeyDeposit = ConstU128<{ 10 * UNITS }>;
	type WeightInfo = ();
	type RelayChainOrigin = EnsureRoot<AccountId>;
	type AHStakingInterface = RelayReportsAreWork;
	type SendToRelayChain = StakingXcmToRelayChain;
	type MaxValidatorSetRetries = ConstU32<64>;
}

/// What a relay session report is to this chain: a record of the work its validators did.
///
/// The relay sends a report every session, with the era points of the validators that
/// validated. `staking-async` would also end a session with it and, with an activation
/// timestamp, start an era. This chain's era clock is its own (`StakingSessionManager`): the
/// validator set comes from People, so the relay's activation never names an era of ours, and
/// the relay's session numbers are not ours. Two writers to one clock is how an era is skipped
/// or never starts. So the report is taken for its points and nothing else.
pub struct RelayReportsAreWork;
impl rc_client::AHStakingInterface for RelayReportsAreWork {
	type AccountId = AccountId;
	type MaxValidatorSet = <Staking as rc_client::AHStakingInterface>::MaxValidatorSet;

	fn on_relay_session_report(report: rc_client::SessionReport<AccountId>) -> Weight {
		let weight =
			<Staking as rc_client::AHStakingInterface>::weigh_on_relay_session_report(&report);
		Staking::note_era_points(report.validator_points);
		weight
	}
	fn weigh_on_relay_session_report(report: &rc_client::SessionReport<AccountId>) -> Weight {
		<Staking as rc_client::AHStakingInterface>::weigh_on_relay_session_report(report)
	}
	fn on_new_offences(
		slash_session: SessionIndex,
		offences: Vec<rc_client::Offence<AccountId>>,
	) -> Weight {
		<Staking as rc_client::AHStakingInterface>::on_new_offences(slash_session, offences)
	}
	fn weigh_on_new_offences(offence_count: u32) -> Weight {
		<Staking as rc_client::AHStakingInterface>::weigh_on_new_offences(offence_count)
	}
	fn active_era_start_session_index() -> SessionIndex {
		<Staking as rc_client::AHStakingInterface>::active_era_start_session_index()
	}
	fn is_validator(who: &AccountId) -> bool {
		<Staking as rc_client::AHStakingInterface>::is_validator(who)
	}
}

/// Forwards session events to CollatorSelection and turns the staking era clock.
///
/// This chain's sessions are the only clock of its eras: the relay's reports arrive too (measured
/// live: `LastSessionReportEndingIndex` 200 against this chain's session 13 on 2026-10-04), but
/// they only record work (`RelayReportsAreWork`). An era is `SessionsPerEra - PlanningEraOffset + 1`
/// of these sessions (see `era_length`).
pub struct StakingSessionManager;

impl pezpallet_session::SessionManager<AccountId> for StakingSessionManager {
	fn new_session(new_index: u32) -> Option<Vec<AccountId>> {
		<CollatorSelection as pezpallet_session::SessionManager<AccountId>>::new_session(new_index)
	}

	fn end_session(end_index: u32) {
		// Forward to CollatorSelection first
		<CollatorSelection as pezpallet_session::SessionManager<AccountId>>::end_session(end_index);

		// Build local SessionReport for staking era progression
		let current_era = pezpallet_staking_async::CurrentEra::<Runtime>::get().unwrap_or(0);
		let active_era_idx = pezpallet_staking_async::ActiveEra::<Runtime>::get()
			.map(|e| e.index)
			.unwrap_or(0);

		// Start a planned era once its election is delivered, so its exposures are whole when
		// it starts -- or, if the election has not finished, once the active era has run for
		// the payout cap, so an election that never finishes cannot stop the clock.
		let activation_timestamp = if current_era > active_era_idx
			&& (Self::election_delivered() || Self::era_reached_payout_cap(end_index))
		{
			let now_ms = pezpallet_timestamp::Now::<Runtime>::get();
			Some((now_ms, current_era))
		} else {
			None
		};

		// Points come from the relay's reports (`RelayReportsAreWork`); this report only turns
		// the clock. It used to give every validator here an equal 20, which counted nothing.
		let report =
			rc_client::SessionReport::new_terminal(end_index, Vec::new(), activation_timestamp);

		let _ = <Staking as rc_client::AHStakingInterface>::on_relay_session_report(report);
	}

	fn start_session(start_index: u32) {
		<CollatorSelection as pezpallet_session::SessionManager<AccountId>>::start_session(
			start_index,
		);
	}
}

impl StakingSessionManager {
	/// The planned era's election has finished and every page of it has been fetched.
	///
	/// The election provider is `Off` only before an election starts and after it is exported,
	/// and one is started the moment an era is planned; `NextElectionPage` is `None` once the
	/// last page is in. Together, with an era planned, they mean its exposures are all stored.
	fn election_delivered() -> bool {
		use pezframe_election_provider_support::ElectionProvider;
		<<Runtime as pezpallet_staking_async::Config>::ElectionProvider as ElectionProvider>::status()
			.is_err() && pezpallet_staking_async::NextElectionPage::<Runtime>::get().is_none()
	}

	/// The active era, if a new one started at the session after `end_index`, would have run
	/// `SessionsPerEra` sessions: `MaxEraDuration`, the longest an era is paid for.
	fn era_reached_payout_cap(end_index: u32) -> bool {
		let started = pezpallet_staking_async::BondedEras::<Runtime>::get()
			.last()
			.map(|(_, session)| *session)
			.unwrap_or(0);
		(end_index + 1).saturating_sub(started) >= SessionsPerEra::get()
	}
}

#[derive(Encode, Decode)]
// Call indices taken from zagros-next runtime.
pub enum RelayChainRuntimePallets {
	// Audit: index of `AssetHubStakingClient` in zagros.
	#[codec(index = 67)]
	AhClient(AhClientCalls),
}

#[derive(Encode, Decode)]
pub enum AhClientCalls {
	// index of `fn validator_set` in `staking-async-ah-client`.
	#[codec(index = 0)]
	ValidatorSet(rc_client::ValidatorSetReport<AccountId>),
	// index of `fn set_keys_from_ah` in `staking-async-ah-client`.
	// The proof of possession is checked here, so only the keys travel to the relay.
	#[codec(index = 3)]
	SetKeys { stash: AccountId, keys: Vec<u8> },
	// index of `fn purge_keys_from_ah` in `staking-async-ah-client`.
	#[codec(index = 4)]
	PurgeKeys { stash: AccountId },
}

pub struct KeysMessageToXcm;
impl pezsp_runtime::traits::Convert<rc_client::KeysMessage<AccountId>, Xcm<()>>
	for KeysMessageToXcm
{
	fn convert(msg: rc_client::KeysMessage<AccountId>) -> Xcm<()> {
		let encoded_call = match msg {
			rc_client::KeysMessage::SetKeys { stash, keys } => {
				RelayChainRuntimePallets::AhClient(AhClientCalls::SetKeys { stash, keys }).encode()
			},
			rc_client::KeysMessage::PurgeKeys { stash } => {
				RelayChainRuntimePallets::AhClient(AhClientCalls::PurgeKeys { stash }).encode()
			},
		};
		rc_client::build_transact_xcm(encoded_call)
	}
}

pub struct ValidatorSetToXcm;
impl pezsp_runtime::traits::Convert<rc_client::ValidatorSetReport<AccountId>, Xcm<()>>
	for ValidatorSetToXcm
{
	fn convert(report: rc_client::ValidatorSetReport<AccountId>) -> Xcm<()> {
		Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			Instruction::Transact {
				origin_kind: OriginKind::Native,
				fallback_max_weight: None,
				call: RelayChainRuntimePallets::AhClient(AhClientCalls::ValidatorSet(report))
					.encode()
					.into(),
			},
		])
	}
}

parameter_types! {
	pub RelayLocation: Location = Location::parent();
	/// Relay-side cost of set/purge keys. Held above the benchmarked figure on purpose:
	/// undercharging strands the message, overpaying only costs the sender a little.
	pub RemoteKeysExecutionWeight: Weight = Weight::from_parts(200_000_000, 20_000);
}

/// Asset Hub elects, and its result stops here.
///
/// `staking-async` calls this when its election finishes, and until 2026-08-29 it went
/// straight to `rc_client`, which put the set in `OutgoingValidatorSet` and shipped it to the
/// relay every era. That is no longer who decides: the committee is drawn on the People chain
/// by TNPoS, from nine strata whose scores are written there.
///
/// The election still runs, and it still matters -- it is the stake stratum's internal
/// ranking, three seats out of twenty-seven, and the exposure it builds is what slashing and
/// the payout machinery read. What it no longer does is leave the chain.
///
/// This is dropped rather than the export being disabled, and the difference is the point:
/// `rc_client`'s exporter is still live and still the one path to the relay. If both this and
/// the People chain wrote `OutgoingValidatorSet`, the later write would win and the validator
/// set would be whichever message happened to land second.
pub struct ElectionStaysHome;
impl rc_client::RcClientInterface for ElectionStaysHome {
	type AccountId = AccountId;

	fn validator_set(new_validator_set: Vec<Self::AccountId>, id: u32, _prune_up_to: Option<u32>) {
		log::debug!(
			target: "runtime::staking",
			"election for era {id} produced {} stashes; kept local, the committee comes from People",
			new_validator_set.len(),
		);
	}
}

pub struct StakingXcmToRelayChain;

impl rc_client::SendToRelayChain for StakingXcmToRelayChain {
	type AccountId = AccountId;
	type Balance = Balance;

	fn validator_set(report: rc_client::ValidatorSetReport<Self::AccountId>) -> Result<(), ()> {
		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			RelayLocation,
			rc_client::ValidatorSetReport<Self::AccountId>,
			ValidatorSetToXcm,
		>::send(report)
	}

	fn set_keys(
		stash: Self::AccountId,
		keys: Vec<u8>,
		max_delivery_and_remote_execution_fee: Option<Self::Balance>,
	) -> Result<Self::Balance, rc_client::SendKeysError<Self::Balance>> {
		let execution_cost = <WeightToFee as pezframe_support::weights::WeightToFee>::weight_to_fee(
			&RemoteKeysExecutionWeight::get(),
		);

		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			RelayLocation,
			rc_client::KeysMessage<Self::AccountId>,
			KeysMessageToXcm,
		>::send_with_fees::<
			xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
			RuntimeCall,
			AccountId,
			rc_client::AccountId32ToLocation,
			Self::Balance,
		>(
			rc_client::KeysMessage::set_keys(stash.clone(), keys),
			stash,
			max_delivery_and_remote_execution_fee,
			execution_cost,
		)
	}

	fn purge_keys(
		stash: Self::AccountId,
		max_delivery_and_remote_execution_fee: Option<Self::Balance>,
	) -> Result<Self::Balance, rc_client::SendKeysError<Self::Balance>> {
		let execution_cost = <WeightToFee as pezframe_support::weights::WeightToFee>::weight_to_fee(
			&RemoteKeysExecutionWeight::get(),
		);

		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			RelayLocation,
			rc_client::KeysMessage<Self::AccountId>,
			KeysMessageToXcm,
		>::send_with_fees::<
			xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
			RuntimeCall,
			AccountId,
			rc_client::AccountId32ToLocation,
			Self::Balance,
		>(
			rc_client::KeysMessage::purge_keys(stash.clone()),
			stash,
			max_delivery_and_remote_execution_fee,
			execution_cost,
		)
	}
}

parameter_types! {
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	pub const MaxPointsToBalance: u8 = 10;
}

impl pezpallet_nomination_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
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
	type MaxUnbonding = <Self as pezpallet_staking_async::Config>::MaxUnlockingChunks;
	type PalletId = PoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type AdminOrigin = EnsureRoot<AccountId>;
	type BlockNumberProvider = RelaychainDataProvider<Runtime>;
	type Filter = Nothing;
	type WeightInfo = weights::pezpallet_nomination_pools::WeightInfo<Self>;
}

parameter_types! {
	pub const DelegatedStakingPalletId: PalletId = PalletId(*b"py/dlstk");
	pub const SlashRewardFraction: Perbill = Perbill::from_percent(1);
}

impl pezpallet_delegated_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = DelegatedStakingPalletId;
	type Currency = Balances;
	type OnSlash = ResolveTo<xcm_config::TreasuryAccount, Balances>;
	type SlashRewardFraction = SlashRewardFraction;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CoreStaking = Staking;
}

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

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
		let tx_ext = TxExtension::from((
			pezframe_system::AuthorizeCall::<Runtime>::new(),
			pezframe_system::CheckNonZeroSender::<Runtime>::new(),
			pezframe_system::CheckSpecVersion::<Runtime>::new(),
			pezframe_system::CheckTxVersion::<Runtime>::new(),
			pezframe_system::CheckGenesis::<Runtime>::new(),
			pezframe_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			pezframe_system::CheckNonce::<Runtime>::from(nonce),
			pezframe_system::CheckWeight::<Runtime>::new(),
			pezpallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<Runtime>::from(tip, None),
			pezframe_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(true),
		));
		let raw_payload = SignedPayload::new(call, tx_ext)
			.map_err(|e| {
				tracing::warn!(target: "runtime::staking", error=?e, "Unable to create signed payload");
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, tx_ext, _) = raw_payload.deconstruct();
		let address = <Runtime as pezframe_system::Config>::Lookup::unlookup(account);
		let transaction = UncheckedExtrinsic::new_signed(call, address, signature, tx_ext);
		Some(transaction)
	}
}

impl<LocalCall> pezframe_system::offchain::CreateInherent<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_bare(call)
	}
}

#[cfg(test)]
mod era_length {
	use super::*;
	use pezframe_support::traits::Get;

	/// An era never outlasts the payout it is minted for.
	///
	/// `EraPayout` mints for the era's elapsed time, capped at `MaxEraDuration`. An era is not
	/// `SessionsPerEra` sessions: the next era is planned once `SessionsPerEra - PlanningEraOffset`
	/// sessions have passed and activated at the following session end, so it lasts
	/// `SessionsPerEra - PlanningEraOffset + 1` sessions (five hours in production, one session in a
	/// fast runtime). If that ever exceeds the cap, every era is paid for the cap and the rest is
	/// never minted -- with six-hour sessions it was a thirty-hour era paid for six, a fifth of the
	/// set inflation.
	#[test]
	fn an_era_never_outlasts_the_payout_cap() {
		// The planning offset reads the election's phase lengths, which are storage parameters.
		pezsp_io::TestExternalities::default().execute_with(|| {
			let per_era = SessionsPerEra::get();
			let offset = <<Runtime as pezpallet_staking_async::Config>::PlanningEraOffset as Get<
				SessionIndex,
			>>::get()
			.min(per_era);
			let era_sessions = (per_era - offset + 1) as u64;
			// This chain's own block time: `Period` counts its blocks, not the relay's.
			let era_ms = era_sessions * crate::Period::get() as u64 * crate::MILLISECS_PER_BLOCK;
			assert!(
				era_ms <= MaxEraDuration::get(),
				"an era is {era_sessions} sessions, {era_ms} ms, past the payout cap of {} ms",
				MaxEraDuration::get()
			);
		});
	}
}
