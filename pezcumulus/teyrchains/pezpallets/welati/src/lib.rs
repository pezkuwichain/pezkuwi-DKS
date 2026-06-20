// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # Welati (Governance) Pezpallet
//!
//! A comprehensive governance pezpallet implementing elections, voting, and government structure
//! management.
//!
//! ## Overview
//!
//! The Welati pezpallet provides complete governance infrastructure including:
//! - **Presidential Elections**: Direct democratic election of Serok (President)
//! - **Parliamentary Elections**: District-based representation in parliament
//! - **Cabinet Formation**: Prime Minister selection and ministerial appointments
//! - **Diwan Council**: Advisory council elections
//! - **Proposal System**: Legislative proposals and voting mechanisms
//! - **Official Appointments**: Non-elected government positions
//!
//! ## Government Structure
//!
//! ### Executive Branch
//! - **Serok** (President): Head of state, elected by popular vote
//! - **SerokWeziran** (Prime Minister): Head of government, appointed by President
//! - **Ministers**: Cabinet members appointed by PM, confirmed by Parliament
//!   - Minister of Finance (WezireDarayiye)
//!   - Minister of Defense (WezireParez)
//!   - Minister of Justice (WezireDad)
//!   - Minister of Education (WezireBelaw)
//!   - Minister of Health (WezireTend)
//!   - Minister of Water Resources (WezireAva)
//!   - Minister of Culture (WezireCand)
//!
//! ### Legislative Branch
//! - **Parliament**: Elected representatives (size configurable)
//! - **Parliamentary Speaker** (SerokiMeclise): Elected from parliament members
//! - **District System**: Electoral districts for regional representation
//!
//! ### Advisory Council
//! - **Diwan**: Council of appointed and elected advisors
//! - **Diwan Members** (EndameDiwane): Mixed selection process
//!
//! ## Election System
//!
//! ### Presidential Election
//! - Requires minimum endorsements from citizens
//! - Candidacy period for registration
//! - Campaign period for public engagement
//! - Direct popular vote
//! - Winner takes office immediately
//!
//! ### Parliamentary Election
//! - District-based representation
//! - Multiple seats per district
//! - Trust-score weighted voting
//! - Proportional representation within districts
//!
//! ### Election Phases
//! 1. **Candidacy Period**: Citizens register as candidates
//! 2. **Campaign Period**: Candidates campaign for votes
//! 3. **Voting Period**: Citizens cast votes
//! 4. **Finalization**: Results calculated, winners take office
//!
//! ## Proposal & Voting System
//!
//! ### Proposal Types
//! - Legislative proposals
//! - Constitutional amendments
//! - Budget proposals
//! - Appointments confirmation
//!
//! ### Voting Mechanism
//! - Parliament members vote on proposals
//! - Voting power based on trust scores
//! - Quorum requirements
//! - Multiple voting options (yes/no/abstain)
//!
//! ## Integration with Roles (Tiki)
//!
//! Elections automatically assign Tiki (role NFTs):
//! - Presidential winner gets Serok tiki
//! - Parliament winners get Parlementer tiki
//! - Appointed ministers get respective Wezire tikis
//! - Diwan members get EndameDiwane tiki
//!
//! ## Interface
//!
//! ### Election Extrinsics
//! - `initiate_election(election_type)` - Start new election process
//! - `register_candidate(election_id, district)` - Register as candidate
//! - `cast_vote(election_id, candidate, vote_weight)` - Cast vote in election
//! - `finalize_election(election_id)` - Calculate results and assign positions
//!
//! ### Appointment Extrinsics
//! - `nominate_official(position, nominee)` - Nominate for government position
//! - `approve_appointment(position, nominee)` - Confirm appointment (Parliament)
//!
//! ### Proposal Extrinsics
//! - `submit_proposal(title, description, call)` - Submit legislative proposal
//! - `vote_on_proposal(proposal_id, vote)` - Vote on active proposal
//!
//! ### Storage
//! - `CurrentOfficials` - Current government position holders
//! - `CurrentMinisters` - Current cabinet ministers
//! - `ParliamentMembers` - Active parliament members
//! - `DiwanMembers` - Active Diwan council members
//! - `ActiveElections` - Ongoing election processes
//! - `Proposals` - Legislative proposals and their status
//!
//! ## Security & Requirements
//! - KYC approval required for all participation
//! - Trust score minimums for candidacy
//! - Endorsement requirements prevent spam candidates
//! - Deposit required for candidacy (slashed if withdrawn)
//! - Vote weighting prevents sybil attacks
//!
//! ## Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_welati::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type WeightInfo = pezpallet_welati::weights::BizinikiwiWeight<Runtime>;
//!     type Randomness = RandomnessCollectiveFlip;
//!     type RuntimeCall = RuntimeCall;
//!     type TrustScoreSource = Trust;
//!     type TikiSource = Tiki;
//!     type CitizenSource = IdentityKyc;
//!     type KycSource = IdentityKyc;
//!     type ParliamentSize = ConstU32<201>;
//!     type DiwanSize = ConstU32<50>;
//!     type ElectionPeriod = ConstU32<1_728_000>; // ~4 months
//!     type CandidacyPeriod = ConstU32<43_200>; // ~3 days
//!     type CampaignPeriod = ConstU32<144_000>; // ~10 days
//!     type ElectoralDistricts = ConstU32<10>;
//!     type CandidacyDeposit = ConstU128<100_000_000_000_000>; // 100 tokens
//!     type PresidentialEndorsements = ConstU32<1000>;
//!     type ParliamentaryEndorsements = ConstU32<100>;
//! }
//! ```

pub use pezpallet::*;
pub mod migrations;
pub mod types;
pub mod weights; // Storage migrations

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use crate::types::*;

/// Weight functions trait for this pezpallet.
pub trait WeightInfo {
	fn initiate_election() -> Weight;
	fn register_candidate() -> Weight;
	fn cast_vote() -> Weight;
	fn finalize_election() -> Weight;
	fn nominate_official() -> Weight;
	fn approve_appointment() -> Weight;
	fn submit_proposal() -> Weight;
	fn vote_on_proposal() -> Weight;
}

// Unit type implementation for tests
impl WeightInfo for () {
	fn initiate_election() -> Weight {
		Weight::from_parts(12_265_000, 1489)
	}
	fn register_candidate() -> Weight {
		Weight::from_parts(21_958_000, 32819)
	}
	fn cast_vote() -> Weight {
		Weight::from_parts(29_505_000, 32819)
	}
	fn finalize_election() -> Weight {
		Weight::from_parts(28_574_000, 32819)
	}
	fn nominate_official() -> Weight {
		Weight::from_parts(26_238_000, 3638)
	}
	fn approve_appointment() -> Weight {
		Weight::from_parts(27_599_000, 13584)
	}
	fn submit_proposal() -> Weight {
		Weight::from_parts(21_824_000, 12542)
	}
	fn vote_on_proposal() -> Weight {
		Weight::from_parts(23_225_000, 12542)
	}
}
#[cfg(not(feature = "runtime-benchmarks"))]
use pezframe_support::traits::Currency;
use pezframe_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	pezpallet_prelude::*,
	traits::{EnsureOrigin, Get, Randomness, ReservableCurrency},
	weights::Weight,
};
use pezframe_system::pezpallet_prelude::*;
#[cfg(not(any(test, feature = "runtime-benchmarks")))]
use pezpallet_identity_kyc::types::KycLevel;
use pezpallet_identity_kyc::types::KycStatus;
use pezpallet_tiki::{Tiki, TikiScoreProvider};
use pezpallet_trust::TrustScoreProvider;
use pezsp_runtime::traits::Dispatchable;
#[cfg(not(feature = "runtime-benchmarks"))]
use pezsp_runtime::SaturatedConversion;
use pezsp_std::{boxed::Box, vec, vec::Vec};

/// Interface for getting citizenship information from other pallets.
pub trait CitizenInfo {
	/// Returns total approved citizen count.
	fn citizen_count() -> u32;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(migrations::STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config:
		pezframe_system::Config<RuntimeEvent: From<Event<Self>>>
		+ pezpallet_tiki::Config
		+ pezpallet_trust::Config
		+ pezpallet_identity_kyc::Config
		+ core::fmt::Debug
	{
		type WeightInfo: crate::WeightInfo;
		type Randomness: Randomness<Self::Hash, BlockNumberFor<Self>>;
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo
			+ From<pezframe_system::Call<Self>>;

		type TrustScoreSource: TrustScoreProvider<Self::AccountId>;
		type TikiSource: TikiScoreProvider<Self::AccountId>;
		type CitizenSource: CitizenInfo;
		type KycSource: KycStatus<Self::AccountId>;

		#[pezpallet::constant]
		type ParliamentSize: Get<u32>;
		#[pezpallet::constant]
		type DiwanSize: Get<u32>;
		#[pezpallet::constant]
		type ElectionPeriod: Get<BlockNumberFor<Self>>;
		#[pezpallet::constant]
		type CandidacyPeriod: Get<BlockNumberFor<Self>>;
		#[pezpallet::constant]
		type CampaignPeriod: Get<BlockNumberFor<Self>>;
		#[pezpallet::constant]
		type ElectoralDistricts: Get<u32>;
		#[pezpallet::constant]
		type CandidacyDeposit: Get<u128>;
		#[pezpallet::constant]
		type PresidentialEndorsements: Get<u32>;
		type ParliamentaryEndorsements: Get<u32>;

		/// Currency used for candidacy deposits
		type NativeCurrency: ReservableCurrency<Self::AccountId>;

		/// Maximum number of endorsers allowed per candidate registration.
		/// Prevents unbounded Vec from consuming excessive weight before validation.
		#[pezpallet::constant]
		type MaxEndorsers: Get<u32>;
	}

	// --- CORE GOVERNANCE STORAGE ---

	/// Storage holding current government positions
	#[pezpallet::storage]
	#[pezpallet::getter(fn current_officials)]
	pub type CurrentOfficials<T: Config> =
		StorageMap<_, Blake2_128Concat, GovernmentPosition, T::AccountId, OptionQuery>;

	/// Storage holding current ministers
	#[pezpallet::storage]
	#[pezpallet::getter(fn current_ministers)]
	pub type CurrentMinisters<T: Config> =
		StorageMap<_, Blake2_128Concat, MinisterRole, T::AccountId, OptionQuery>;

	/// Storage holding parliament members
	#[pezpallet::storage]
	#[pezpallet::getter(fn parliament_members)]
	pub type ParliamentMembers<T: Config> =
		StorageValue<_, BoundedVec<ParliamentMember<T>, T::ParliamentSize>, ValueQuery>;

	/// Storage holding Diwan members
	#[pezpallet::storage]
	#[pezpallet::getter(fn diwan_members)]
	pub type DiwanMembers<T: Config> =
		StorageValue<_, BoundedVec<DiwanMember<T>, T::DiwanSize>, ValueQuery>;

	/// Storage holding appointed government officials (OfficialRole)
	#[pezpallet::storage]
	#[pezpallet::getter(fn appointed_officials)]
	pub type AppointedOfficials<T: Config> =
		StorageMap<_, Blake2_128Concat, OfficialRole, T::AccountId, OptionQuery>;

	// --- ELECTION SYSTEM STORAGE ---

	/// Storage holding active elections
	#[pezpallet::storage]
	pub type ActiveElections<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, ElectionInfo<T>, OptionQuery>;

	/// Next election ID
	#[pezpallet::storage]
	pub type NextElectionId<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Storage holding election candidates
	#[pezpallet::storage]
	pub type ElectionCandidates<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32, // election_id
		Blake2_128Concat,
		T::AccountId,     // candidate
		CandidateInfo<T>, // candidate details
		OptionQuery,
	>;

	/// Storage holding election votes
	#[pezpallet::storage]
	pub type ElectionVotes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32, // election_id
		Blake2_128Concat,
		T::AccountId,        // voter
		ElectionVoteInfo<T>, // vote info
		OptionQuery,
	>;

	/// Storage holding election results
	#[pezpallet::storage]
	pub type ElectionResults<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, ElectionResult<T>, OptionQuery>;

	/// Storage holding electoral districts
	#[pezpallet::storage]
	pub type ElectoralDistrictConfig<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, ElectoralDistrict, ValueQuery>;

	// --- APPOINTMENT SYSTEM STORAGE ---

	/// Storage holding pending nominations
	#[pezpallet::storage]
	pub type PendingNominations<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		OfficialRole,
		Blake2_128Concat,
		T::AccountId,
		NominationInfo<T>,
		OptionQuery,
	>;

	/// Storage holding appointment processes
	#[pezpallet::storage]
	pub type AppointmentProcesses<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, AppointmentProcess<T>, OptionQuery>;

	/// Next appointment process ID
	#[pezpallet::storage]
	pub type NextAppointmentId<T: Config> = StorageValue<_, u32, ValueQuery>;

	// --- COLLECTIVE DECISION STORAGE ---

	/// Storage holding active proposals
	#[pezpallet::storage]
	pub type ActiveProposals<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, CollectiveProposal<T>, OptionQuery>;

	/// Next proposal ID
	#[pezpallet::storage]
	pub type NextProposalId<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Storage holding collective votes
	#[pezpallet::storage]
	pub type CollectiveVotes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32, // proposal_id
		Blake2_128Concat,
		T::AccountId, // voter
		CollectiveVote<T>,
		OptionQuery,
	>;

	/// Storage holding governance metrics
	#[pezpallet::storage]
	pub type GovernanceStats<T: Config> = StorageValue<_, GovernanceMetrics<T>, OptionQuery>;

	// --- Events ---
	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// --- ELECTION EVENTS ---
		/// Election started
		ElectionStarted {
			election_id: u32,
			election_type: ElectionType,
			start_block: BlockNumberFor<T>,
			end_block: BlockNumberFor<T>,
		},

		/// Candidate registered
		CandidateRegistered { election_id: u32, candidate: T::AccountId, deposit_paid: u128 },

		/// Vote cast
		VoteCast {
			election_id: u32,
			voter: T::AccountId,
			candidates: Vec<T::AccountId>,
			district_id: Option<u32>,
		},

		/// Election finalized
		ElectionFinalized {
			election_id: u32,
			winners: Vec<T::AccountId>,
			total_votes: u32,
			turnout_percentage: u8,
		},

		// --- APPOINTMENT EVENTS ---
		/// Official nominated
		OfficialNominated {
			process_id: u32,
			nominator: T::AccountId,
			nominee: T::AccountId,
			role: OfficialRole,
		},

		/// Appointment approved
		AppointmentApproved {
			process_id: u32,
			approver: T::AccountId,
			appointee: T::AccountId,
			role: OfficialRole,
		},

		/// Appointment rejected
		AppointmentRejected {
			process_id: u32,
			rejector: T::AccountId,
			nominee: T::AccountId,
			role: OfficialRole,
			reason: BoundedVec<u8, ConstU32<500>>,
		},

		// --- COLLECTIVE DECISION EVENTS ---
		/// Proposal submitted
		ProposalSubmitted {
			proposal_id: u32,
			proposer: T::AccountId,
			decision_type: CollectiveDecisionType,
			voting_deadline: BlockNumberFor<T>,
		},

		/// Collective vote cast
		CollectiveVoteCast { proposal_id: u32, voter: T::AccountId, vote: VoteChoice },

		/// Proposal finalized
		ProposalFinalized {
			proposal_id: u32,
			result: ProposalStatus,
			aye_votes: u32,
			nay_votes: u32,
			abstain_votes: u32,
		},

		// --- GOVERNANCE EVENTS ---
		/// Parliament updated
		ParliamentUpdated { new_members: Vec<T::AccountId>, term_start: BlockNumberFor<T> },

		/// Diwan member appointed
		DiwanMemberAppointed {
			member: T::AccountId,
			appointed_by: AppointmentAuthority<T>,
			specialization: ConstitutionalSpecialization,
		},

		/// Veto applied
		VetoApplied {
			proposal_id: u32,
			vetoed_by: T::AccountId,
			reason: BoundedVec<u8, ConstU32<1000>>,
		},
	}

	#[pezpallet::error]
	pub enum Error<T> {
		// General errors
		InsufficientTrustScore,
		MissingRequiredTiki,
		NotACitizen,

		// Election errors
		ElectionNotFound,
		ElectionNotActive,
		ElectionAlreadyStarted,
		ElectionAlreadyFinalized,
		CandidacyPeriodExpired,
		CampaignPeriodNotStarted,
		VotingPeriodNotStarted,
		VotingPeriodExpired,
		AlreadyCandidate,
		AlreadyVoted,
		InvalidDistrict,
		InsufficientEndorsements,
		DepositRequired,
		TooManyCandidates,
		InvalidInitialCandidates,

		// Appointment errors
		NotAuthorizedToNominate,
		NotAuthorizedToApprove,
		AppointmentProcessNotFound,
		NominationNotFound,
		AppointmentAlreadyProcessed,
		RoleAlreadyFilled,

		// Collective decision errors
		ProposalNotFound,
		ProposalNotActive,
		NotAuthorizedToPropose,
		NotAuthorizedToVote,
		ProposalAlreadyVoted,
		QuorumNotMet,
		ProposalExecutionFailed,

		// System errors
		ParliamentFull,
		DiwanFull,
		InvalidElectionType,
		CalculationOverflow,
		RunoffElectionFailed,
		/// Candidate cannot afford the required deposit
		InsufficientDeposit,
		/// Too many endorsers provided
		TooManyEndorsers,
	}

	// --- Extrinsics ---
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Initiates a new election
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::initiate_election())]
		pub fn initiate_election(
			origin: OriginFor<T>,
			election_type: ElectionType,
			districts: Option<Vec<ElectoralDistrict>>,
			initial_candidates: Option<BoundedVec<T::AccountId, ConstU32<2>>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let election_id = NextElectionId::<T>::get();
			NextElectionId::<T>::put(election_id.saturating_add(1));

			let current_block = <pezframe_system::Pezpallet<T>>::block_number();

			let candidacy_deadline;
			let campaign_start;
			let voting_start;
			let voting_end;
			let initial_status;
			let candidates_list;

			if let Some(runoff_candidates) = initial_candidates {
				ensure!(
					election_type == ElectionType::Presidential,
					Error::<T>::InvalidElectionType
				);
				ensure!(runoff_candidates.len() == 2, Error::<T>::InvalidInitialCandidates);

				candidacy_deadline = current_block;
				campaign_start = current_block;
				let runoff_campaign_period = T::CampaignPeriod::get() / 3u32.into();
				let campaign_end = campaign_start + runoff_campaign_period;
				voting_start = campaign_end;
				voting_end = voting_start + T::ElectionPeriod::get();
				initial_status = ElectionStatus::CampaignPeriod;
				candidates_list = BoundedVec::try_from(runoff_candidates.to_vec())
					.map_err(|_| Error::<T>::TooManyCandidates)?;

				for candidate in runoff_candidates.iter() {
					let candidate_info = CandidateInfo {
						account: candidate.clone(),
						district_id: None,
						registered_at: current_block,
						endorsers: Default::default(),
						vote_count: 0,
						deposit_paid: 0,
						campaign_data: Default::default(),
					};
					ElectionCandidates::<T>::insert(election_id, candidate, candidate_info);
				}
			} else {
				candidacy_deadline = current_block + T::CandidacyPeriod::get();
				campaign_start = candidacy_deadline;
				let campaign_end = campaign_start + T::CampaignPeriod::get();
				voting_start = campaign_end;
				voting_end = voting_start + T::ElectionPeriod::get();
				initial_status = ElectionStatus::CandidacyPeriod;
				candidates_list = Default::default();
			}

			let districts_bounded: BoundedVec<ElectoralDistrict, ConstU32<50>> = districts
				.unwrap_or_default()
				.try_into()
				.map_err(|_| Error::<T>::InvalidDistrict)?;

			let election_info = ElectionInfo {
				election_id,
				election_type,
				start_block: current_block,
				candidacy_deadline,
				campaign_start,
				voting_start,
				end_block: voting_end,
				candidates: candidates_list,
				total_votes: 0,
				status: initial_status,
				districts: districts_bounded,
				minimum_turnout: Self::get_minimum_turnout(&election_type),
			};

			ActiveElections::<T>::insert(election_id, election_info);

			Self::deposit_event(Event::ElectionStarted {
				election_id,
				election_type,
				start_block: current_block,
				end_block: voting_end,
			});

			Ok(())
		}

		/// Register as election candidate
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::register_candidate())]
		pub fn register_candidate(
			origin: OriginFor<T>,
			election_id: u32,
			district_id: Option<u32>,
			endorsers: Vec<T::AccountId>,
		) -> DispatchResult {
			let candidate = ensure_signed(origin)?;

			// H7 fix: Validate endorsers count early, before any storage reads,
			// to prevent large Vecs from consuming excessive weight.
			ensure!(endorsers.len() as u32 <= T::MaxEndorsers::get(), Error::<T>::TooManyEndorsers);

			let mut election =
				ActiveElections::<T>::get(election_id).ok_or(Error::<T>::ElectionNotFound)?;

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			ensure!(
				current_block <= election.candidacy_deadline,
				Error::<T>::CandidacyPeriodExpired
			);

			// KYC check is always active (bypass only in unit tests and benchmarks)
			#[cfg(not(any(test, feature = "runtime-benchmarks")))]
			{
				ensure!(
					<pezpallet_identity_kyc::Pezpallet<T> as KycStatus<T::AccountId>>::get_kyc_status(
						&candidate
					) == KycLevel::Approved,
					Error::<T>::NotACitizen
				);
			}

			#[cfg(not(feature = "runtime-benchmarks"))]
			{
				let trust_score = T::TrustScoreSource::trust_score_of(&candidate);
				let required_score = Self::get_required_trust_score(&election.election_type);
				ensure!(trust_score >= required_score, Error::<T>::InsufficientTrustScore);
			}

			#[cfg(not(feature = "runtime-benchmarks"))]
			{
				if let Some(_required_tiki) = Self::get_required_tiki(&election.election_type) {
					let user_tiki_score = T::TikiSource::get_tiki_score(&candidate);
					ensure!(user_tiki_score > 0, Error::<T>::MissingRequiredTiki);
				}
			}

			let required_endorsements = Self::get_required_endorsements(&election.election_type);
			ensure!(
				endorsers.len() as u32 >= required_endorsements,
				Error::<T>::InsufficientEndorsements
			);

			// BYPASS ENDORSER KYC CHECK FOR TESTS AND BENCHMARKS
			#[cfg(not(any(test, feature = "runtime-benchmarks")))]
			{
				for endorser in &endorsers {
					ensure!(
						<pezpallet_identity_kyc::Pezpallet<T> as KycStatus<T::AccountId>>::get_kyc_status(
							endorser
						) == KycLevel::Approved,
						Error::<T>::NotACitizen
					);

					let endorser_trust = T::TrustScoreSource::trust_score_of(endorser);
					ensure!(endorser_trust >= 40u128, Error::<T>::InsufficientTrustScore);
				}
			}

			ensure!(
				!ElectionCandidates::<T>::contains_key(election_id, &candidate),
				Error::<T>::AlreadyCandidate
			);

			// H6 fix: Actually reserve the candidacy deposit from the candidate's balance.
			// Skip in benchmarks where accounts may not be funded.
			#[cfg(not(feature = "runtime-benchmarks"))]
			{
				let deposit_amount: <<T as Config>::NativeCurrency as Currency<
					T::AccountId,
				>>::Balance = T::CandidacyDeposit::get().saturated_into();
				T::NativeCurrency::reserve(&candidate, deposit_amount)
					.map_err(|_| Error::<T>::InsufficientDeposit)?;
			}

			let candidate_info = CandidateInfo {
				account: candidate.clone(),
				district_id,
				registered_at: current_block,
				endorsers: endorsers
					.try_into()
					.map_err(|_| Error::<T>::InsufficientEndorsements)?,
				vote_count: 0,
				deposit_paid: T::CandidacyDeposit::get(),
				campaign_data: Default::default(),
			};

			ElectionCandidates::<T>::insert(election_id, &candidate, candidate_info);
			election
				.candidates
				.try_push(candidate.clone())
				.map_err(|_| Error::<T>::ParliamentFull)?;
			ActiveElections::<T>::insert(election_id, election);

			Self::deposit_event(Event::CandidateRegistered {
				election_id,
				candidate,
				deposit_paid: T::CandidacyDeposit::get(),
			});

			Ok(())
		}

		/// Cast vote
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::cast_vote())]
		pub fn cast_vote(
			origin: OriginFor<T>,
			election_id: u32,
			candidates: Vec<T::AccountId>,
			district_id: Option<u32>,
		) -> DispatchResult {
			let voter = ensure_signed(origin)?;

			let mut election =
				ActiveElections::<T>::get(election_id).ok_or(Error::<T>::ElectionNotFound)?;

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			ensure!(
				current_block >= election.voting_start && current_block <= election.end_block,
				Error::<T>::VotingPeriodNotStarted
			);

			// BYPASS KYC CHECK FOR TESTS AND BENCHMARKS
			#[cfg(not(any(test, feature = "runtime-benchmarks")))]
			{
				ensure!(
					<pezpallet_identity_kyc::Pezpallet<T> as KycStatus<T::AccountId>>::get_kyc_status(
						&voter
					) == KycLevel::Approved,
					Error::<T>::NotACitizen
				);
			}

			ensure!(
				!ElectionVotes::<T>::contains_key(election_id, &voter),
				Error::<T>::AlreadyVoted
			);

			for candidate in &candidates {
				ensure!(
					ElectionCandidates::<T>::contains_key(election_id, candidate),
					Error::<T>::ElectionNotFound
				);
			}

			let vote_weight = Self::calculate_vote_weight(&voter, &election.election_type);

			let vote_info = ElectionVoteInfo {
				voter: voter.clone(),
				candidates: candidates
					.clone()
					.try_into()
					.map_err(|_| Error::<T>::CalculationOverflow)?,
				vote_block: current_block,
				vote_weight,
				vote_type: VoteType::Citizen,
				district_id,
			};

			ElectionVotes::<T>::insert(election_id, &voter, vote_info);

			for candidate in &candidates {
				ElectionCandidates::<T>::mutate(election_id, candidate, |info| {
					if let Some(candidate_info) = info {
						candidate_info.vote_count =
							candidate_info.vote_count.saturating_add(vote_weight);
					}
				});
			}

			election.total_votes = election.total_votes.saturating_add(vote_weight);
			ActiveElections::<T>::insert(election_id, election);

			Self::deposit_event(Event::VoteCast { election_id, voter, candidates, district_id });

			Ok(())
		}

		/// Finalizes election and determines winners
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::finalize_election())]
		pub fn finalize_election(origin: OriginFor<T>, election_id: u32) -> DispatchResult {
			ensure_root(origin)?;

			let mut election =
				ActiveElections::<T>::get(election_id).ok_or(Error::<T>::ElectionNotFound)?;

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			ensure!(current_block > election.end_block, Error::<T>::ElectionNotActive);

			ensure!(
				election.status != ElectionStatus::Completed,
				Error::<T>::ElectionAlreadyFinalized
			);

			let outcome = Self::calculate_election_winners(election_id, &election)?;

			match outcome {
				ElectionOutcome::Winners(winners) => {
					Self::assign_election_winners(&election.election_type, &winners)?;

					let total_citizen_count = Self::get_total_citizen_count();
					let turnout_percentage = if total_citizen_count > 0 {
						((election.total_votes as u64).saturating_mul(100)
							/ total_citizen_count as u64) as u8
					} else {
						0
					};

					let result = ElectionResult {
						election_id,
						winners: winners.clone(),
						total_votes: election.total_votes,
						turnout_percentage,
						finalized_at: current_block,
					};

					ElectionResults::<T>::insert(election_id, result);
					election.status = ElectionStatus::Completed;
					ActiveElections::<T>::insert(election_id, election.clone());

					Self::deposit_event(Event::ElectionFinalized {
						election_id,
						winners: winners.into_inner(),
						total_votes: election.total_votes,
						turnout_percentage,
					});
				},
				ElectionOutcome::RunoffRequired(candidates) => {
					Self::initiate_election(
						pezframe_system::RawOrigin::Root.into(),
						ElectionType::Presidential,
						None,
						Some(candidates),
					)?;

					election.status = ElectionStatus::Completed;
					ActiveElections::<T>::insert(election_id, election);
				},
			}

			Ok(())
		}

		#[pezpallet::call_index(10)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		#[pezpallet::feeless_if(|origin: &OriginFor<T>, _nominee: &T::AccountId, _role: &OfficialRole, _justification: &BoundedVec<u8, ConstU32<1000>>| -> bool {
            // Governance members are exempt from fees when performing official duties
            match ensure_signed(origin.clone()) {
                Ok(who) => Pezpallet::<T>::is_governance_member(&who),
                Err(_) => false,
            }
        })]
		pub fn nominate_official(
			origin: OriginFor<T>,
			nominee: T::AccountId,
			role: OfficialRole,
			justification: BoundedVec<u8, ConstU32<1000>>,
		) -> DispatchResult {
			let nominator = ensure_signed(origin)?;

			// Verify nominator is authorized (must be a minister or Serok)
			// For simplicity, we'll require Serok or any minister can nominate
			let is_serok =
				CurrentOfficials::<T>::get(GovernmentPosition::Serok) == Some(nominator.clone());
			let is_minister =
				CurrentMinisters::<T>::iter().any(|(_, minister)| minister == nominator);

			ensure!(is_serok || is_minister, Error::<T>::NotAuthorizedToNominate);

			// Check if role is already filled
			ensure!(!AppointedOfficials::<T>::contains_key(role), Error::<T>::RoleAlreadyFilled);

			// Check if this specific nominee already has a pending nomination for this role
			ensure!(
				!PendingNominations::<T>::contains_key(role, &nominee),
				Error::<T>::RoleAlreadyFilled
			);

			// Create new appointment process
			let process_id = NextAppointmentId::<T>::get();
			NextAppointmentId::<T>::mutate(|id| *id = id.saturating_add(1));

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			let deadline = current_block + BlockNumberFor::<T>::from(14400u32 * 7u32); // 7 days

			// Create nomination info
			let nomination = NominationInfo {
				nominator: nominator.clone(),
				nominee: nominee.clone(),
				nominated_at: current_block,
				approved: false,
				approver: None,
				approved_at: None,
				status: NominationStatus::Pending,
			};

			// Store nomination
			PendingNominations::<T>::insert(role, &nominee, nomination);

			// Create appointment process
			let documents: BoundedVec<BoundedVec<u8, ConstU32<1000>>, ConstU32<10>> =
				vec![justification].try_into().map_err(|_| Error::<T>::CalculationOverflow)?;

			let appointment_process = AppointmentProcess {
				process_id,
				position: role,
				nominating_minister: nominator.clone(),
				nominee: nominee.clone(),
				initiated_at: current_block,
				deadline,
				status: AppointmentStatus::WaitingPresidentialApproval,
				documents,
			};

			AppointmentProcesses::<T>::insert(process_id, appointment_process);

			Self::deposit_event(Event::OfficialNominated { process_id, nominator, nominee, role });

			Ok(())
		}

		#[pezpallet::call_index(11)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::approve_appointment())]
		#[pezpallet::feeless_if(|origin: &OriginFor<T>, _process_id: &u32| -> bool {
            // Serok (President) is exempt from fees when approving appointments
            match ensure_signed(origin.clone()) {
                Ok(who) => Pezpallet::<T>::is_serok(&who),
                Err(_) => false,
            }
        })]
		pub fn approve_appointment(origin: OriginFor<T>, process_id: u32) -> DispatchResult {
			let approver = ensure_signed(origin)?;

			// Verify approver is authorized (typically Serok)
			let is_serok =
				CurrentOfficials::<T>::get(GovernmentPosition::Serok) == Some(approver.clone());
			ensure!(is_serok, Error::<T>::NotAuthorizedToApprove);

			// Get appointment process
			let mut process = AppointmentProcesses::<T>::get(process_id)
				.ok_or(Error::<T>::AppointmentProcessNotFound)?;

			// Check status
			ensure!(
				process.status == AppointmentStatus::WaitingPresidentialApproval,
				Error::<T>::AppointmentAlreadyProcessed
			);

			// Get nomination
			let mut nomination = PendingNominations::<T>::get(process.position, &process.nominee)
				.ok_or(Error::<T>::NominationNotFound)?;

			// Update nomination
			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			nomination.approved = true;
			nomination.approver = Some(approver.clone());
			nomination.approved_at = Some(current_block);
			nomination.status = NominationStatus::Approved;

			// Update process status
			process.status = AppointmentStatus::Approved;

			// Store updates
			PendingNominations::<T>::insert(process.position, &process.nominee, nomination);
			AppointmentProcesses::<T>::insert(process_id, process.clone());

			// Assign the official to the role
			AppointedOfficials::<T>::insert(process.position, &process.nominee);

			Self::deposit_event(Event::AppointmentApproved {
				process_id,
				approver,
				appointee: process.nominee,
				role: process.position,
			});

			Ok(())
		}

		#[pezpallet::call_index(20)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::submit_proposal())]
		#[pezpallet::feeless_if(|origin: &OriginFor<T>, _title: &BoundedVec<u8, ConstU32<100>>, _description: &BoundedVec<u8, ConstU32<1000>>, _decision_type: &CollectiveDecisionType, _priority: &ProposalPriority, _call: &Option<Box<<T as pezframe_system::Config>::RuntimeCall>>| -> bool {
            // Governance members are exempt from fees when submitting proposals
            match ensure_signed(origin.clone()) {
                Ok(who) => Pezpallet::<T>::is_governance_member(&who),
                Err(_) => false,
            }
        })]
		pub fn submit_proposal(
			origin: OriginFor<T>,
			title: BoundedVec<u8, ConstU32<100>>,
			description: BoundedVec<u8, ConstU32<1000>>,
			decision_type: CollectiveDecisionType,
			priority: ProposalPriority,
			call: Option<Box<<T as pezframe_system::Config>::RuntimeCall>>,
		) -> DispatchResult {
			let proposer = ensure_signed(origin)?;
			ensure!(
				Self::can_propose(&proposer, &decision_type)?,
				Error::<T>::NotAuthorizedToPropose
			);

			let proposal_id = NextProposalId::<T>::get();
			NextProposalId::<T>::put(proposal_id.saturating_add(1));

			let current_block = <pezframe_system::Pezpallet<T>>::block_number();
			let voting_starts_at = current_block + 14400u32.into();
			let expires_at = voting_starts_at + T::ElectionPeriod::get();

			let proposal = CollectiveProposal {
				proposal_id,
				proposer: proposer.clone(),
				title,
				description,
				proposed_at: current_block,
				voting_starts_at,
				expires_at,
				decision_type,
				status: ProposalStatus::Active,
				aye_votes: 0,
				nay_votes: 0,
				abstain_votes: 0,
				threshold: Self::get_voting_threshold(&decision_type),
				votes_cast: 0,
				priority,
				call,
			};

			ActiveProposals::<T>::insert(proposal_id, proposal);

			Self::deposit_event(Event::ProposalSubmitted {
				proposal_id,
				proposer,
				decision_type,
				voting_deadline: expires_at,
			});

			Ok(())
		}

		#[pezpallet::call_index(21)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::vote_on_proposal())]
		#[pezpallet::feeless_if(|origin: &OriginFor<T>, _proposal_id: &u32, _vote: &VoteChoice, _rationale: &Option<BoundedVec<u8, ConstU32<500>>>| -> bool {
            // Governance members are exempt from fees when voting
            match ensure_signed(origin.clone()) {
                Ok(who) => Pezpallet::<T>::is_governance_member(&who),
                Err(_) => false,
            }
        })]
		pub fn vote_on_proposal(
			origin: OriginFor<T>,
			proposal_id: u32,
			vote: VoteChoice,
			rationale: Option<BoundedVec<u8, ConstU32<500>>>,
		) -> DispatchResult {
			let voter = ensure_signed(origin)?;
			ensure!(ActiveProposals::<T>::contains_key(proposal_id), Error::<T>::ProposalNotFound);

			// Check if voter has already voted on this proposal
			ensure!(
				!CollectiveVotes::<T>::contains_key(proposal_id, &voter),
				Error::<T>::ProposalAlreadyVoted
			);

			// Check if voter is authorized (must be a parliament member)
			let proposal =
				ActiveProposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;

			// Enforce access control based on decision type
			match proposal.decision_type {
				CollectiveDecisionType::ParliamentSimpleMajority
				| CollectiveDecisionType::ParliamentSuperMajority
				| CollectiveDecisionType::ParliamentAbsoluteMajority
				| CollectiveDecisionType::VetoOverride => {
					// Parliament members only
					let members = ParliamentMembers::<T>::get();
					let is_member = members.iter().any(|m| m.account == voter);
					ensure!(is_member, Error::<T>::NotAuthorizedToVote);
				},
				CollectiveDecisionType::ConstitutionalReview
				| CollectiveDecisionType::ConstitutionalUnanimous => {
					// Diwan members only
					ensure!(Self::is_diwan_member(&voter), Error::<T>::NotAuthorizedToVote,);
				},
				CollectiveDecisionType::ExecutiveDecision => {
					// Serok (President) only
					let serok = CurrentOfficials::<T>::get(GovernmentPosition::Serok);
					ensure!(serok.as_ref() == Some(&voter), Error::<T>::NotAuthorizedToVote,);
				},
				CollectiveDecisionType::HybridDecision => {
					// Parliament members OR Serok
					let members = ParliamentMembers::<T>::get();
					let is_parliament = members.iter().any(|m| m.account == voter);
					let serok = CurrentOfficials::<T>::get(GovernmentPosition::Serok);
					let is_serok = serok.as_ref() == Some(&voter);
					ensure!(is_parliament || is_serok, Error::<T>::NotAuthorizedToVote,);
				},
			}

			// Record the vote
			let vote_info = CollectiveVote {
				voter: voter.clone(),
				proposal_id,
				vote,
				voted_at: pezframe_system::Pezpallet::<T>::block_number(),
				rationale,
			};

			CollectiveVotes::<T>::insert(proposal_id, &voter, vote_info);

			// Update proposal vote counts
			ActiveProposals::<T>::mutate(proposal_id, |proposal_opt| {
				if let Some(proposal) = proposal_opt {
					match vote {
						VoteChoice::Aye => {
							proposal.aye_votes = proposal.aye_votes.saturating_add(1)
						},
						VoteChoice::Nay => {
							proposal.nay_votes = proposal.nay_votes.saturating_add(1)
						},
						VoteChoice::Abstain => {
							proposal.abstain_votes = proposal.abstain_votes.saturating_add(1)
						},
					}
					proposal.votes_cast = proposal.votes_cast.saturating_add(1);
				}
			});

			Ok(())
		}
	}

	// ====== PUBLIC GETTERS FOR TESTS ======
	impl<T: Config> Pezpallet<T> {
		pub fn active_elections(election_id: u32) -> Option<ElectionInfo<T>> {
			ActiveElections::<T>::get(election_id)
		}

		pub fn next_election_id() -> u32 {
			NextElectionId::<T>::get()
		}

		pub fn election_candidates(
			election_id: u32,
			candidate: T::AccountId,
		) -> Option<CandidateInfo<T>> {
			ElectionCandidates::<T>::get(election_id, candidate)
		}

		pub fn election_votes(
			election_id: u32,
			voter: T::AccountId,
		) -> Option<ElectionVoteInfo<T>> {
			ElectionVotes::<T>::get(election_id, voter)
		}

		pub fn election_results(election_id: u32) -> Option<ElectionResult<T>> {
			ElectionResults::<T>::get(election_id)
		}

		pub fn next_appointment_id() -> u32 {
			NextAppointmentId::<T>::get()
		}

		pub fn appointment_processes(process_id: u32) -> Option<AppointmentProcess<T>> {
			AppointmentProcesses::<T>::get(process_id)
		}

		pub fn next_proposal_id() -> u32 {
			NextProposalId::<T>::get()
		}

		pub fn active_proposals(proposal_id: u32) -> Option<CollectiveProposal<T>> {
			ActiveProposals::<T>::get(proposal_id)
		}

		pub fn collective_votes(
			proposal_id: u32,
			voter: T::AccountId,
		) -> Option<CollectiveVote<T>> {
			CollectiveVotes::<T>::get(proposal_id, voter)
		}
	}

	// ====== HELPER FUNCTIONS ======
	impl<T: Config> Pezpallet<T> {
		/// Serok origin check
		pub fn ensure_serok(origin: OriginFor<T>) -> Result<T::AccountId, DispatchError> {
			let who = ensure_signed(origin)?;
			let current_serok = CurrentOfficials::<T>::get(GovernmentPosition::Serok)
				.ok_or(DispatchError::BadOrigin)?;
			ensure!(who == current_serok, DispatchError::BadOrigin);
			Ok(who)
		}

		/// Checks if caller is a Parliament member
		pub fn ensure_parliament_member(
			origin: OriginFor<T>,
		) -> Result<T::AccountId, DispatchError> {
			let who = ensure_signed(origin)?;
			let is_member = ParliamentMembers::<T>::get().iter().any(|m| m.account == who);
			ensure!(is_member, DispatchError::BadOrigin);
			Ok(who)
		}

		/// Minimum Trust Score by election type
		pub fn get_required_trust_score(election_type: &ElectionType) -> u128 {
			match election_type {
				ElectionType::Presidential => 250,
				ElectionType::Parliamentary => 100,
				ElectionType::SpeakerElection => 200,
				ElectionType::ConstitutionalCourt => 275,
			}
		}

		/// Required Tiki by election type
		pub fn get_required_tiki(election_type: &ElectionType) -> Option<Tiki> {
			match election_type {
				ElectionType::Presidential | ElectionType::Parliamentary => Some(Tiki::Welati),
				ElectionType::SpeakerElection => Some(Tiki::Parlementer),
				ElectionType::ConstitutionalCourt => Some(Tiki::Welati),
			}
		}

		/// Required number of endorsers
		pub fn get_required_endorsements(election_type: &ElectionType) -> u32 {
			match election_type {
				ElectionType::Presidential => T::PresidentialEndorsements::get(),
				ElectionType::Parliamentary => T::ParliamentaryEndorsements::get(),
				_ => 0,
			}
		}

		/// Minimum turnout rate
		pub fn get_minimum_turnout(election_type: &ElectionType) -> u8 {
			match election_type {
				ElectionType::Presidential => 50,
				ElectionType::Parliamentary => 40,
				_ => 30,
			}
		}

		/// Calculate vote weight
		pub fn calculate_vote_weight(voter: &T::AccountId, election_type: &ElectionType) -> u32 {
			match election_type {
				ElectionType::Presidential | ElectionType::Parliamentary => 1,
				_ => {
					let trust_score = T::TrustScoreSource::trust_score_of(voter);
					let weight = (trust_score / 100) as u32;
					weight.clamp(1, 10)
				},
			}
		}

		/// Total citizen count
		fn get_total_citizen_count() -> u32 {
			T::CitizenSource::citizen_count()
		}

		/// Calculates election winners or determines if runoff is needed
		fn calculate_election_winners(
			election_id: u32,
			election: &ElectionInfo<T>,
		) -> Result<ElectionOutcome<T::AccountId>, Error<T>> {
			let mut candidates_with_votes: Vec<(T::AccountId, u32)> = election
				.candidates
				.iter()
				.filter_map(|candidate| {
					ElectionCandidates::<T>::get(election_id, candidate)
						.map(|info| (candidate.clone(), info.vote_count))
				})
				.collect();

			candidates_with_votes.sort_by(|a, b| b.1.cmp(&a.1));

			match election.election_type {
				ElectionType::Presidential => {
					if candidates_with_votes.is_empty() {
						return Ok(ElectionOutcome::Winners(Default::default()));
					}
					let total_valid_votes =
						candidates_with_votes.iter().map(|(_, v)| *v).sum::<u32>().max(1);
					let (top_winner, top_vote_count) = candidates_with_votes[0].clone();

					if (top_vote_count * 100) / total_valid_votes >= 50 {
						let winners_vec: BoundedVec<_, _> = vec![top_winner]
							.try_into()
							.map_err(|_| Error::<T>::CalculationOverflow)?;
						Ok(ElectionOutcome::Winners(winners_vec))
					} else {
						let runoff_candidates: BoundedVec<_, _> = candidates_with_votes
							.into_iter()
							.take(2)
							.map(|(acc, _)| acc)
							.collect::<Vec<_>>()
							.try_into()
							.map_err(|_| Error::<T>::CalculationOverflow)?;
						Ok(ElectionOutcome::RunoffRequired(runoff_candidates))
					}
				},
				ElectionType::Parliamentary => {
					let winner_count = T::ParliamentSize::get() as usize;
					let winners: BoundedVec<_, _> = candidates_with_votes
						.into_iter()
						.take(winner_count)
						.map(|(account, _)| account)
						.collect::<Vec<_>>()
						.try_into()
						.map_err(|_| Error::<T>::ParliamentFull)?;
					Ok(ElectionOutcome::Winners(winners))
				},
				ElectionType::SpeakerElection => {
					let winners: BoundedVec<_, _> = candidates_with_votes
						.into_iter()
						.take(1)
						.map(|(account, _)| account)
						.collect::<Vec<_>>()
						.try_into()
						.map_err(|_| Error::<T>::CalculationOverflow)?;
					Ok(ElectionOutcome::Winners(winners))
				},
				ElectionType::ConstitutionalCourt => {
					let winners: BoundedVec<_, _> = candidates_with_votes
						.into_iter()
						.take(T::DiwanSize::get() as usize)
						.map(|(account, _)| account)
						.collect::<Vec<_>>()
						.try_into()
						.map_err(|_| Error::<T>::DiwanFull)?;
					Ok(ElectionOutcome::Winners(winners))
				},
			}
		}

		/// Assign winners to positions
		fn assign_election_winners(
			election_type: &ElectionType,
			winners: &[T::AccountId],
		) -> Result<(), Error<T>> {
			match election_type {
				ElectionType::Presidential => {
					if let Some(winner) = winners.first() {
						CurrentOfficials::<T>::insert(GovernmentPosition::Serok, winner);
					}
				},
				ElectionType::Parliamentary => {
					let current_block = pezframe_system::Pezpallet::<T>::block_number();
					let term_end = current_block
						+ BlockNumberFor::<T>::from(4u32 * 365u32 * 24u32 * 60u32 * 10u32);

					let parliament_members: Result<BoundedVec<_, _>, _> = winners
						.iter()
						.map(|winner| ParliamentMember {
							account: winner.clone(),
							elected_at: current_block,
							term_ends_at: term_end,
							votes_participated: 0,
							total_votes_eligible: 0,
							participation_rate: 100,
							committees: Default::default(),
						})
						.collect::<Vec<_>>()
						.try_into();

					ParliamentMembers::<T>::put(
						parliament_members.map_err(|_| Error::<T>::ParliamentFull)?,
					);

					Self::deposit_event(Event::ParliamentUpdated {
						new_members: winners.to_vec(),
						term_start: current_block,
					});
				},
				ElectionType::SpeakerElection => {
					if let Some(winner) = winners.first() {
						CurrentOfficials::<T>::insert(GovernmentPosition::MeclisBaskanı, winner);
					}
				},
				_ => {},
			}
			Ok(())
		}

		/// Check proposal authority
		fn can_propose(
			proposer: &T::AccountId,
			decision_type: &CollectiveDecisionType,
		) -> Result<bool, Error<T>> {
			match decision_type {
				CollectiveDecisionType::ExecutiveDecision => {
					Ok(CurrentOfficials::<T>::get(GovernmentPosition::Serok)
						== Some(proposer.clone()))
				},
				_ => {
					let is_parliamentarian = ParliamentMembers::<T>::get()
						.iter()
						.any(|member| member.account == *proposer);
					let is_president = CurrentOfficials::<T>::get(GovernmentPosition::Serok)
						== Some(proposer.clone());

					Ok(is_parliamentarian || is_president)
				},
			}
		}

		/// Calculate voting threshold
		fn get_voting_threshold(decision_type: &CollectiveDecisionType) -> u32 {
			match decision_type {
				CollectiveDecisionType::ParliamentSimpleMajority => {
					(T::ParliamentSize::get() / 2) + 1
				},
				CollectiveDecisionType::ParliamentSuperMajority => {
					(T::ParliamentSize::get() * 2) / 3
				},
				CollectiveDecisionType::ParliamentAbsoluteMajority => {
					(T::ParliamentSize::get() * 3) / 4
				},
				CollectiveDecisionType::ConstitutionalReview => (T::DiwanSize::get() * 2) / 3,
				CollectiveDecisionType::ConstitutionalUnanimous => T::DiwanSize::get(),
				_ => T::ParliamentSize::get() / 2 + 1,
			}
		}
	}
}

// ====== ORIGIN IMPLEMENTATIONS ======

/// For Serok origin check
pub struct EnsureSerok<T>(pezsp_std::marker::PhantomData<T>);

impl<T: pezpallet::Config> EnsureOrigin<<T as pezframe_system::Config>::RuntimeOrigin>
	for EnsureSerok<T>
{
	type Success = T::AccountId;

	fn try_origin(
		o: <T as pezframe_system::Config>::RuntimeOrigin,
	) -> Result<Self::Success, <T as pezframe_system::Config>::RuntimeOrigin> {
		match o.clone().into() {
			Ok(pezframe_system::RawOrigin::Signed(who)) => {
				if let Some(current_serok) =
					pezpallet::Pezpallet::<T>::current_officials(GovernmentPosition::Serok)
				{
					if who == current_serok {
						return Ok(who);
					}
				}
				Err(o)
			},
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<<T as pezframe_system::Config>::RuntimeOrigin, ()> {
		let serok_account: T::AccountId = pezframe_benchmarking::account("serok", 0, 0);
		pezpallet::CurrentOfficials::<T>::insert(GovernmentPosition::Serok, serok_account.clone());
		Ok(pezframe_system::RawOrigin::Signed(serok_account).into())
	}
}

/// For Parliament member origin check
pub struct EnsureParlementer<T>(pezsp_std::marker::PhantomData<T>);

impl<T: pezpallet::Config> EnsureOrigin<<T as pezframe_system::Config>::RuntimeOrigin>
	for EnsureParlementer<T>
{
	type Success = T::AccountId;

	fn try_origin(
		o: <T as pezframe_system::Config>::RuntimeOrigin,
	) -> Result<Self::Success, <T as pezframe_system::Config>::RuntimeOrigin> {
		match o.clone().into() {
			Ok(pezframe_system::RawOrigin::Signed(who)) => {
				let parliament_members = pezpallet::Pezpallet::<T>::parliament_members();
				if parliament_members.iter().any(|member| member.account == who) {
					return Ok(who);
				}
				Err(o)
			},
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<<T as pezframe_system::Config>::RuntimeOrigin, ()> {
		let parlementer_account: T::AccountId = pezframe_benchmarking::account("parlementer", 0, 0);
		let member = ParliamentMember {
			account: parlementer_account.clone(),
			elected_at: 0u32.into(),
			term_ends_at: u32::MAX.into(),
			votes_participated: 0,
			total_votes_eligible: 0,
			participation_rate: 100,
			committees: Default::default(),
		};
		let members: BoundedVec<_, T::ParliamentSize> = vec![member].try_into().unwrap();
		ParliamentMembers::<T>::put(members);
		Ok(pezframe_system::RawOrigin::Signed(parlementer_account).into())
	}
}

/// For Diwan origin check
pub struct EnsureDiwan<T>(pezsp_std::marker::PhantomData<T>);

impl<T: pezpallet::Config> EnsureOrigin<<T as pezframe_system::Config>::RuntimeOrigin>
	for EnsureDiwan<T>
{
	type Success = T::AccountId;

	fn try_origin(
		o: <T as pezframe_system::Config>::RuntimeOrigin,
	) -> Result<Self::Success, <T as pezframe_system::Config>::RuntimeOrigin> {
		match o.clone().into() {
			Ok(pezframe_system::RawOrigin::Signed(who)) => {
				let diwan_members = pezpallet::Pezpallet::<T>::diwan_members();
				if diwan_members.iter().any(|member| member.account == who) {
					return Ok(who);
				}
				Err(o)
			},
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<<T as pezframe_system::Config>::RuntimeOrigin, ()> {
		let diwan_account: T::AccountId = pezframe_benchmarking::account("diwan", 0, 0);
		let president_account: T::AccountId = pezframe_benchmarking::account("president", 0, 0);
		let member = DiwanMember {
			account: diwan_account.clone(),
			appointed_at: 0u32.into(),
			term_ends_at: u32::MAX.into(),
			appointed_by: AppointmentAuthority::President(president_account),
			specialization: ConstitutionalSpecialization::FundamentalRights,
			decisions_made: 0,
		};
		let members: BoundedVec<_, T::DiwanSize> = vec![member].try_into().map_err(|_| ())?;
		DiwanMembers::<T>::put(members);
		Ok(pezframe_system::RawOrigin::Signed(diwan_account).into())
	}
}

// ====== HELPER FUNCTIONS FOR FEE EXEMPTION ======

impl<T: Config> Pezpallet<T> {
	/// Check if an account is any type of governance member
	/// Used for fee exemption in governance-related transactions
	pub fn is_governance_member(who: &T::AccountId) -> bool {
		Self::is_serok(who)
			|| Self::is_parliament_member(who)
			|| Self::is_diwan_member(who)
			|| Self::is_minister(who)
	}

	/// Check if account is Serok (President)
	pub fn is_serok(who: &T::AccountId) -> bool {
		CurrentOfficials::<T>::get(GovernmentPosition::Serok)
			.map(|serok| &serok == who)
			.unwrap_or(false)
	}

	/// Check if account is a Parliament member
	pub fn is_parliament_member(who: &T::AccountId) -> bool {
		ParliamentMembers::<T>::get().iter().any(|member| &member.account == who)
	}

	/// Check if account is a Diwan member
	pub fn is_diwan_member(who: &T::AccountId) -> bool {
		DiwanMembers::<T>::get().iter().any(|member| &member.account == who)
	}

	/// Check if account is a Minister
	pub fn is_minister(who: &T::AccountId) -> bool {
		CurrentMinisters::<T>::iter().any(|(_, minister)| &minister == who)
	}

	/// Check if account is an Official (non-minister appointed position)
	pub fn is_official(who: &T::AccountId) -> bool {
		AppointedOfficials::<T>::iter().any(|(_, official)| &official == who)
	}
}
