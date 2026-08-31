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
// Not feature-gated. `return_candidacy_deposits` names `Currency<T::AccountId>::Balance` in
// every configuration, so gating the import to `not(runtime-benchmarks)` did not silence a
// warning -- it removed the trait from scope in the build that check-benches runs.
use pezframe_support::traits::Currency;
use pezframe_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	pezpallet_prelude::*,
	traits::{EnsureOrigin, Get, OnUnbalanced, Polling, Randomness, ReservableCurrency},
	weights::Weight,
};
use pezframe_system::pezpallet_prelude::*;
#[cfg(not(any(test, feature = "runtime-benchmarks")))]
use pezpallet_identity_kyc::types::KycLevel;
use pezpallet_identity_kyc::types::KycStatus;
use pezpallet_tiki::{Tiki, TikiScoreProvider};
use pezpallet_trust::TrustScoreProvider;
use pezsp_runtime::traits::{Dispatchable, Saturating};
// Not feature-gated, for the same reason as `Currency` above: `return_candidacy_deposits`
// calls `saturated_into` in every configuration.
use pezsp_runtime::SaturatedConversion;
use pezsp_std::{vec, vec::Vec};
use xcm::latest::prelude::*;

/// `pezpallet-pez-treasury::activate_distribution`, by call index.
///
/// Kept next to the pallet-index constant it is used with, because the pair is the whole
/// address of a call on another chain and neither half means anything alone.
const ACTIVATE_DISTRIBUTION_CALL_INDEX: u8 = 0;

/// `pezpallet-pez-treasury::spend_from_government_pot`, by call index.
const SPEND_FROM_GOVERNMENT_POT_CALL_INDEX: u8 = 1;

/// `pezpallet_treasury::spend` on the Asset Hub's airdrop instance, by call index.
///
/// Five, because that is where `spend` sits in the treasury pallet -- the same number for
/// every instance, since instances share the pallet's call enum and differ only by which
/// pallet index carries them. The instance is addressed by `AirdropPotPalletIndex`, not by
/// this.
const TREASURY_SPEND_CALL_INDEX: u8 = 5;

/// `pezpallet-parameters::set_parameter` on the treasury chain, by call index.
const SET_PARAMETER_CALL_INDEX: u8 = 0;

/// The two variant indices that address HEZ's emission rate inside `RuntimeParameters`.
///
/// `RuntimeParameters::Hez(hez::Parameters::InflationRate(key, value))`. The key is a unit
/// struct and encodes to nothing, so the wire form is the two indices followed by the value.
/// `the_emission_call_encodes_the_way_welati_builds_it` on the treasury chain pins the bytes.
const HEZ_PARAMETERS_VARIANT: u8 = 0;
const INFLATION_RATE_VARIANT: u8 = 0;

/// How many seats change hands per block while a handover is being applied.
///
/// A weight ceiling, not a policy: each seat rewrites a citizen NFT's metadata, and two
/// hundred and one of those in the block that counts the votes would be a block nobody else
/// fits into. At ten a block a full replacement takes about forty blocks -- four minutes --
/// during which the election result already stands in `ParliamentMembers`.
const SEATS_PER_BLOCK: u32 = 10;

/// Interface for getting citizenship information from other pallets.
pub trait CitizenInfo {
	/// Returns total approved citizen count.
	fn citizen_count() -> u32;
}

/// How a backed citizens' initiative reaches the ballot.
///
/// The runtime implements this by submitting to the referenda pallet on the proposer's behalf.
/// It is a trait so that this pallet does not have to know what a referendum is made of --
/// it knows who may ask and when enough people have asked with them, and nothing else.
pub trait InitiativeLaunch<AccountId, Hash> {
	/// Put the proposal to the register on the given track.
	///
	/// `track` is the index the runtime's track list gives the origin being asked for; the
	/// runtime maps it back. `hash`/`len` name a preimage that is already stored.
	fn launch(proposer: &AccountId, track: u16, hash: Hash, len: u32) -> DispatchResult;
}

impl<AccountId, Hash> InitiativeLaunch<AccountId, Hash> for () {
	fn launch(_: &AccountId, _: u16, _: Hash, _: u32) -> DispatchResult {
		Err(pezsp_runtime::DispatchError::Other("no ballot to launch onto"))
	}
}

/// The bench, as the body that votes.
///
/// This pallet decides *who* sits on the court -- six the house elects, five the President
/// appoints, both under conditions this pallet enforces. It does not run the court's
/// deliberations; a collective does that, and the runtime hands one in here. Keeping the two
/// apart is what stops the membership from existing twice: this is the only writer, and the
/// collective is only ever told.
pub trait CourtRoster<AccountId> {
	/// Replace the bench's membership with exactly `members`.
	fn set_members(members: Vec<AccountId>);
}

impl<AccountId> CourtRoster<AccountId> for () {
	fn set_members(_members: Vec<AccountId>) {}
}

/// The house, as the body that resolves.
///
/// Same shape and same reason as `CourtRoster`. This pallet decides *who* sits in Parliament;
/// it does not run the house's business, and a collective does. Without this the only origin
/// meaning "Parliament" was one that accepted a single member -- a name promising a body and
/// a check giving one person. An origin standing for the house as a whole has to be seated
/// from the register, and this is the one direction that seating travels.
pub trait HouseRoster<AccountId> {
	/// Replace the house's membership with exactly `members`.
	fn set_members(members: Vec<AccountId>);
}

impl<AccountId> HouseRoster<AccountId> for () {
	fn set_members(_members: Vec<AccountId>) {}
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(migrations::STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	/// What a forfeit produces: value taken out of an account and not yet given to anyone.
	pub type NegativeImbalanceOf<T> = <<T as Config>::NativeCurrency as Currency<
		<T as pezframe_system::Config>::AccountId,
	>>::NegativeImbalance;

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
		/// Seats on the court, in total. Eleven.
		#[pezpallet::constant]
		type DiwanSize: Get<u32>;

		/// Where the house's membership is mirrored so that Parliament can resolve as a body.
		type HouseRoster: HouseRoster<Self::AccountId>;

		/// Where the bench's membership is mirrored so that the court can vote as a body.
		type CourtRoster: CourtRoster<Self::AccountId>;

		/// How many of those seats the sitting Parliament elects. The remainder is the
		/// President's to appoint, so the two cannot be set inconsistently.
		#[pezpallet::constant]
		type DiwanElectedSeats: Get<u32>;
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

		/// The roll, as the state tally measures support against.
		///
		/// Must be the same source the runtime hands `CitizenTally`, or a referendum would be
		/// counted against one roll and decided against another.
		type Electorate: Get<u32> + 'static;

		/// The state referenda whose questions this pallet's citizens answer.
		///
		/// Voting lives here rather than beside the ballot box because the question of who may
		/// vote is this pallet's: it holds the roll, and it already answers that question for
		/// elections. Two pallets answering it would be two answers.
		type Polls: Polling<
			crate::types::CitizenTally<Self::Electorate>,
			Index = u32,
			Votes = u32,
			Class = u16,
			Moment = BlockNumberFor<Self>,
		>;

		/// How a backed initiative becomes a question on the register's ballot.
		///
		/// A trait rather than the pallet itself: what this pallet knows is who may ask and
		/// when enough people have asked with them. Turning that into a referendum is the
		/// ballot box's job, and the runtime is the only place that can see both.
		type Initiatives: InitiativeLaunch<Self::AccountId, Self::Hash>;

		/// Share of the roll whose backing turns a citizen's initiative into a referendum.
		///
		/// A share rather than a count: fifty thousand citizens make a fixed threshold of
		/// twenty per cent impossible and five million make it free. Every real jurisdiction
		/// that runs initiatives sets it between one and five per cent of the electorate.
		#[pezpallet::constant]
		type InitiativeThreshold: Get<pezsp_runtime::Perbill>;

		/// How long backing may be collected before the initiative lapses.
		///
		/// Without it a petition left open long enough eventually crosses any threshold, and
		/// what it would then show is patience rather than support.
		#[pezpallet::constant]
		type InitiativeWindow: Get<BlockNumberFor<Self>>;

		/// What the proposer puts up. Returned when the threshold is reached, forfeit when the
		/// window closes without it -- so a real initiative is free and a frivolous one is not.
		#[pezpallet::constant]
		type InitiativeDeposit: Get<u128>;

		/// How long a proposer waits after one of their initiatives lapses.
		///
		/// Without it, an initiative the register declined can be reopened the next block, and
		/// the window that was supposed to settle the question settles nothing.
		#[pezpallet::constant]
		type InitiativeCooldown: Get<BlockNumberFor<Self>>;

		/// What becomes of a forfeit deposit. Not nothing: deposits here are HEZ, HEZ
		/// inflates, and destroying it would spread the forfeit across everyone holding
		/// some. A penalty has to become something the state can spend.
		///
		/// A handler rather than an account, for the same reason as `SlashDestination` in
		/// staking-score: the treasury this should reach is on another chain, and an address
		/// cannot teleport. Pointed at a bare address it accumulated where nobody could spend.
		type InitiativeSlashTarget: OnUnbalanced<NegativeImbalanceOf<Self>>;

		/// The body that confirms an appointment the executive has proposed.
		///
		/// A presidential system's central check: the executive names, another body says yes
		/// or no, and neither alone suffices. Deliberately not satisfiable by the President --
		/// he nominates, and one person is not both parties to an appointment.
		type ConfirmationOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Currency used for candidacy deposits
		type NativeCurrency: ReservableCurrency<Self::AccountId>;

		/// Maximum number of endorsers allowed per candidate registration.
		/// Prevents unbounded Vec from consuming excessive weight before validation.
		#[pezpallet::constant]
		type MaxEndorsers: Get<u32>;

		/// How the chain reaches the chain that holds the money.
		type XcmSender: SendXcm;

		/// Where the PEZ treasury lives -- the Asset Hub, as seen from here.
		#[pezpallet::constant]
		type TreasuryChainLocation: Get<Location>;

		/// The index `pezpallet-pez-treasury` occupies in the treasury chain's runtime.
		///
		/// This chain cannot name the other chain's calls by type, so it addresses them by
		/// index. That makes this constant load-bearing in a way a type would not be: if the
		/// treasury chain ever renumbers its pallets, every message sent from here lands on
		/// whatever now sits at this index. `pez_treasury_call_indices_match_the_asset_hub`
		/// in the emulated tests is what holds the two ends together.
		#[pezpallet::constant]
		type TreasuryPalletIndex: Get<u8>;

		/// Where the airdrop pot sits in the treasury chain's runtime.
		///
		/// A second `pezpallet_treasury` instance, so it has its own pallet index and its own
		/// account while sharing the pallet's code. Named separately from
		/// `TreasuryPalletIndex` because they are two different pallets on that chain and a
		/// message addressed to the wrong one is accepted by whatever happens to sit there.
		#[pezpallet::constant]
		type AirdropPotPalletIndex: Get<u8>;

		/// How long an approved airdrop above the ceiling waits before it can be paid.
		///
		/// Only the large ones wait. Below the ceiling two signatures pay immediately, which
		/// is what a listing needs; above it the third signature buys nothing if the payment
		/// lands in the same block as the last approval, because nobody outside the three can
		/// see it in time to object.
		#[pezpallet::constant]
		type LargeAirdropDelay: Get<BlockNumberFor<Self>>;

		/// What one airdrop may move without the Treasurer's signature.
		///
		/// The Asset Hub enforces its own ceiling on the spend origin as well; this one is
		/// here because the decision to escalate has to be made where the offices are, before
		/// a message is sent that the other chain would refuse.
		#[pezpallet::constant]
		type AirdropCeiling: Get<u128>;

		/// The index `pezpallet-parameters` occupies in the treasury chain's runtime.
		#[pezpallet::constant]
		type ParametersPalletIndex: Get<u8>;

		/// The most the emission rate may move in one step.
		///
		/// A ceiling says how high the rate may ever go; this says how fast it may get there.
		/// Without it the ceiling is the only limit and a single call reaches it, which is the
		/// difference between a mandate and a lever. The ceiling itself is not mirrored here --
		/// it lives on the treasury chain, in the code that applies the rate, so there is one
		/// of it rather than two that can disagree.
		#[pezpallet::constant]
		type MaxEmissionStep: Get<pezsp_runtime::Perbill>;

		/// The least time between two changes to the emission rate.
		#[pezpallet::constant]
		type MinEmissionInterval: Get<BlockNumberFor<Self>>;

		/// How long an elected mandate runs.
		///
		/// Four years, for every office the country votes on -- except the court, below.
		#[pezpallet::constant]
		type TermLength: Get<BlockNumberFor<Self>>;

		/// How long a seat on the Diwan runs.
		///
		/// Longer than the political offices, and deliberately so. The Diwan judges the
		/// President and the government; a court elected on the same cycle as the people it
		/// judges is not a check on them, it is an extension of whoever won that cycle. The
		/// length is what makes it independent.
		#[pezpallet::constant]
		type CourtTermLength: Get<BlockNumberFor<Self>>;

		/// How many consecutive terms one person may serve in the same office.
		///
		/// Zero means no limit. What this exists to prevent is not a long career but a
		/// permanent one: an office that can be won indefinitely stops being an office and
		/// becomes a possession.
		#[pezpallet::constant]
		type MaxConsecutiveTerms: Get<u32>;

		/// How many citizens the register must hold before the treasury starts paying them.
		#[pezpallet::constant]
		type PopulationThreshold: Get<u32>;

		/// How often the population is checked.
		///
		/// Once per era rather than on every citizen, because the answer only matters at the
		/// moment it changes from no to yes, and a state that starts paying one era late has
		/// lost nothing. Checking on every approval would put a read on the hot path of the
		/// one call the whole population makes.
		#[pezpallet::constant]
		type PopulationCheckPeriod: Get<BlockNumberFor<Self>>;
	}

	// --- CORE GOVERNANCE STORAGE ---

	/// Storage holding parliament members
	#[pezpallet::storage]
	#[pezpallet::getter(fn parliament_members)]
	pub type ParliamentMembers<T: Config> =
		StorageValue<_, BoundedVec<ParliamentMember<T>, T::ParliamentSize>, ValueQuery>;

	/// Seats waiting to be taken away, and seats waiting to be given, after a handover.
	///
	/// Two hundred and one seats cannot change hands in the block that counts the votes: each
	/// grant and each revocation rewrites the holder's citizen NFT metadata. The result of the
	/// election is final the moment it is recorded in `ParliamentMembers`; the tikis follow
	/// over the next few blocks. Only the difference is queued, so a member returned to their
	/// seat is never briefly unseated.
	#[pezpallet::storage]
	#[pezpallet::getter(fn pending_seat_revokes)]
	pub type PendingSeatRevokes<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, T::ParliamentSize>, ValueQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn pending_seat_grants)]
	pub type PendingSeatGrants<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, T::ParliamentSize>, ValueQuery>;

	/// When the seats being handed out run out.
	///
	/// Held here rather than in each queue entry because every seat in one handover ends on
	/// the same day -- the house has one term, not two hundred and one.
	#[pezpallet::storage]
	pub type PendingSeatTerm<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// Storage holding Diwan members
	#[pezpallet::storage]
	#[pezpallet::getter(fn diwan_members)]
	pub type DiwanMembers<T: Config> =
		StorageValue<_, BoundedVec<DiwanMember<T>, T::DiwanSize>, ValueQuery>;

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

	/// Airdrop proposals, by id. An entry is removed when it is paid or rejected.
	#[pezpallet::storage]
	pub type AirdropProposals<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, AirdropProposal<T>, OptionQuery>;

	/// The next airdrop proposal's id. Never reused, so an id in an event or a log always
	/// names the same proposal however long afterwards it is read.
	#[pezpallet::storage]
	pub type NextAirdropId<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The emission rate this chain last told the treasury chain to apply, and when.
	///
	/// Kept here rather than read back from the treasury chain because a cross-chain read is not
	/// a thing a runtime can do, and because this chain is the only writer: the treasury chain
	/// accepts the change from `EnsureXcm<Equals<PeopleLocation>>` and nothing else, so what was
	/// last sent from here is what is in force there.
	#[pezpallet::storage]
	pub type EmissionRate<T: Config> =
		StorageValue<_, (pezsp_runtime::Perbill, BlockNumberFor<T>), OptionQuery>;

	/// The President's standing nominee for Prime Minister, waiting on the House.
	#[pezpallet::storage]
	pub type PendingPrimeMinister<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Which offices need the confirming body's consent, where the legislature has spoken.
	///
	/// Absent an entry the office's founding rule stands (`requires_parliament_approval`), so
	/// this map holds departures from the founding list rather than a copy of it: an empty map
	/// is the constitution as written, not an executive with no checks on it.
	#[pezpallet::storage]
	pub type ConfirmationRequired<T: Config> =
		StorageMap<_, Twox64Concat, OfficialRole, bool, OptionQuery>;

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

	// --- BUDGET ---

	/// What Parliament has approved and the finance minister has not yet spent.
	///
	/// A running allowance rather than a per-payment authorisation: Parliament decides how
	/// much may be spent, the minister decides on what. That is the split -- a Parliament that
	/// approved every payment would be governing, and a minister who set his own ceiling would
	/// be unaccountable.
	#[pezpallet::storage]
	#[pezpallet::getter(fn approved_budget)]
	pub type ApprovedBudget<T: Config> = StorageValue<_, u128, ValueQuery>;

	// --- TERMS ---

	/// When each elected office's current mandate ends.
	///
	/// Absent means the office has never been filled by a vote. Present means a clock is
	/// running, and the clock is what opens the next election -- not a person deciding to.
	#[pezpallet::storage]
	#[pezpallet::getter(fn term_ends)]
	pub type TermEnds<T: Config> =
		StorageMap<_, Blake2_128Concat, ElectionType, BlockNumberFor<T>, OptionQuery>;

	/// The election currently open for an office, if there is one.
	///
	/// Keeps the scheduler from opening a second election for an office that is already
	/// voting, which it would otherwise do on every block of the run-up.
	#[pezpallet::storage]
	#[pezpallet::getter(fn scheduled_election)]
	pub type OpenElection<T: Config> =
		StorageMap<_, Blake2_128Concat, ElectionType, u32, OptionQuery>;

	/// Who has endorsed whom, said by the endorser.
	///
	/// A candidate used to hand in a list of accounts and the pallet counted them. Nothing
	/// asked those accounts anything, so the thousand endorsements a presidential candidacy
	/// requires were a thousand names typed by the candidate. An endorsement has to be an act
	/// of the person endorsing or it is not an endorsement.
	#[pezpallet::storage]
	#[pezpallet::getter(fn endorsement)]
	pub type Endorsements<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32, // election id
		Blake2_128Concat,
		T::AccountId, // the endorser
		T::AccountId, // whom they endorsed
		OptionQuery,
	>;

	/// How many times running the election for this office has failed to reach quorum.
	///
	/// Reset the moment one succeeds. It exists so a re-run can drop the turnout requirement:
	/// a quorum that the country has already failed to meet, applied again unchanged, produces
	/// the same failure forever, and an office that can never be filled is worse than one
	/// filled by a small turnout.
	#[pezpallet::storage]
	#[pezpallet::getter(fn failed_attempts)]
	pub type FailedAttempts<T: Config> =
		StorageMap<_, Blake2_128Concat, ElectionType, u32, ValueQuery>;

	/// How many terms in a row this account has served in this office.
	///
	/// Reset when somebody else wins it, which is what makes the limit about continuity rather
	/// than a lifetime total.
	#[pezpallet::storage]
	#[pezpallet::getter(fn consecutive_terms)]
	pub type ConsecutiveTerms<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ElectionType,
		Blake2_128Concat,
		T::AccountId,
		u32,
		ValueQuery,
	>;

	// --- POPULATION GATE ---

	/// Whether the treasury has been told the population threshold was reached.
	///
	/// One way only, and it records that the message was *sent*, not that it arrived. A
	/// resend would be worse than a lost message: the receiving side latches too, so a second
	/// message changes nothing there, while a loop that keeps sending would put an XCM in
	/// every era forever. If the first one is lost the fix is a governance message, not an
	/// automatic retry that nobody would notice was running.
	#[pezpallet::storage]
	#[pezpallet::getter(fn population_gate_reported)]
	pub type PopulationGateReported<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Initiatives open for backing, and those that closed but were not yet settled.
	#[pezpallet::storage]
	pub type Initiatives<T: Config> = StorageMap<
		_,
		Twox64Concat,
		u32,
		crate::types::Initiative<T::AccountId, BlockNumberFor<T>, T::Hash>,
		OptionQuery,
	>;

	/// Who has signed which initiative.
	///
	/// Open on purpose: an initiative exists so that citizens can see whether the state has
	/// been captured, and a signature nobody can see says nothing about who is behind it.
	#[pezpallet::storage]
	pub type InitiativeBacking<T: Config> =
		StorageDoubleMap<_, Twox64Concat, u32, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	#[pezpallet::storage]
	pub type NextInitiativeId<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The block from which a proposer whose initiative lapsed may open another.
	#[pezpallet::storage]
	pub type InitiativeCooldownUntil<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

	/// How each citizen answered a state referendum.
	///
	/// Kept so that nobody is counted twice and so that a citizen can change their mind while
	/// the question is still open. Nothing is locked -- the count is of heads, not of tokens --
	/// so the only reason this outlives the poll is to be cleared, which anyone may do once the
	/// poll is over.
	#[pezpallet::storage]
	pub type ReferendumVotes<T: Config> =
		StorageDoubleMap<_, Twox64Concat, u32, Blake2_128Concat, T::AccountId, bool, OptionQuery>;

	// --- Events ---
	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// --- CITIZENS' INITIATIVE EVENTS ---
		/// A citizen opened an initiative and put up the deposit for it.
		InitiativeOpened { id: u32, proposer: T::AccountId, track: u16, closes: BlockNumberFor<T> },
		/// A citizen signed one.
		InitiativeBacked { id: u32, who: T::AccountId, backing: u32 },
		/// Enough citizens signed; the question went to the ballot and the deposit came back.
		InitiativeLaunched { id: u32, backing: u32 },
		/// The window closed without the threshold. The deposit did not come back.
		InitiativeLapsed { id: u32, backing: u32 },

		// --- STATE REFERENDUM EVENTS ---
		/// A citizen answered a state referendum. Recasting the other way replaces the answer
		/// rather than adding one.
		ReferendumVoteCast { who: T::AccountId, poll: u32, aye: bool },
		/// Answers to a finished referendum were discarded.
		ReferendumVotesCleared { poll: u32, removed: u32 },

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
		/// The Treasurer moved the emission rate.
		EmissionRateSet { rate: pezsp_runtime::Perbill, by: T::AccountId },

		/// The President put a name forward for Prime Minister.
		PrimeMinisterNominated { nominee: T::AccountId },

		/// The House refused the President's nominee for Prime Minister.
		PrimeMinisterRejected { nominee: T::AccountId },

		/// The legislature moved an office on or off the list that needs confirming.
		ConfirmationRequirementSet { role: OfficialRole, required: bool },

		/// An appointment was confirmed by the body that confirms it.
		AppointmentConfirmedByParliament {
			process_id: u32,
			appointee: T::AccountId,
			role: OfficialRole,
		},

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

		/// The founding parliament was seated by the President before the first election.
		FoundingParliamentSeated { members: Vec<T::AccountId>, term_ends_at: BlockNumberFor<T> },

		/// Every seat in a handover has changed hands.
		SeatHandoverCompleted,

		/// A seat could not be given to its winner -- most likely because they are no longer
		/// a citizen. The queue moves on rather than stalling; the seat stays empty and its
		/// share of the parliamentary reward stays in the pot.
		SeatCouldNotBeTaken { who: T::AccountId },

		/// Diwan member appointed
		DiwanMemberAppointed { member: T::AccountId, appointed_by: AppointmentAuthority<T> },

		/// A citizen endorsed a candidacy.
		CandidateEndorsed { election_id: u32, endorser: T::AccountId, candidate: T::AccountId },

		/// An election was opened because a mandate was running out.
		ElectionScheduled { election_type: ElectionType },

		/// Not enough of the country voted. The election is over and another will be opened;
		/// the next one does not ask for a quorum the country has already failed to meet.
		ElectionFailedForTurnout { election_id: u32, election_type: ElectionType },

		/// The citizen register passed the population threshold, and the treasury was told.
		PopulationThresholdReported { citizen_count: u32 },

		/// The population report could not be sent. It will be attempted again next era.
		PopulationReportFailed,

		/// Parliament raised the government's spending allowance.
		BudgetApproved { amount: u128, total: u128 },
		/// The finance minister spent against the approved budget.
		BudgetSpent { beneficiary: T::AccountId, amount: u128 },
		/// An airdrop was proposed by the Prime Minister.
		AirdropProposed { id: u32, beneficiary: T::AccountId, amount: u128 },
		/// The President signed it.
		AirdropApprovedByPresident { id: u32 },
		/// The Treasurer signed it. Only large airdrops reach this.
		AirdropApprovedByTreasurer { id: u32 },
		/// Fully signed, and payable from the block named. For a small airdrop that is now;
		/// for a large one it is `LargeAirdropDelay` later, and the wait is the point.
		AirdropReady { id: u32, payable_from: BlockNumberFor<T> },
		/// Sent to the airdrop pot's chain. Sent, not paid: the pot checks the amount against
		/// its own ceiling on arrival, and this chain never hears the answer.
		AirdropSent { id: u32, beneficiary: T::AccountId, amount: u128 },
		/// Withdrawn before payment by the Prime Minister who proposed it, or refused by the
		/// President.
		AirdropCancelled { id: u32 },

		/// The President named a Prime Minister.
		PrimeMinisterAppointed { who: T::AccountId },
		/// The President dismissed the Prime Minister.
		PrimeMinisterDismissed,
		/// The Prime Minister named a minister.
		MinisterAppointed { who: T::AccountId, tiki: Tiki },
		/// The Prime Minister dismissed a minister.
		MinisterDismissed { who: T::AccountId, tiki: Tiki },

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

		// Citizens' initiative errors
		/// No initiative under that id.
		NoSuchInitiative,
		/// The window has closed; nothing more may be added.
		InitiativeClosed,
		/// The window is still open, so it is neither backed enough nor lapsed.
		InitiativeStillOpen,
		/// One signature per citizen. A second says nothing the first did not.
		AlreadyBacked,
		/// Not enough of the roll has signed.
		NotEnoughBacking,
		/// This proposer's last initiative lapsed and the wait has not run out.
		InitiativeCooldown,

		// State referendum errors
		/// Trust of zero is technical death, and the dead do not vote.
		NoTrustToVote,
		/// The referendum is not open for answers -- it never existed, or it has finished.
		ReferendumNotOngoing,
		/// The same answer is already recorded. Changing sides is allowed; repeating is not.
		AlreadyAnsweredThatWay,
		/// Answers are only discarded once the question is settled.
		ReferendumStillOngoing,

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
		/// The caller is not the sitting Prime Minister.
		NotThePrimeMinister,
		/// An elected winner could not be seated in their office.
		CouldNotSeatOffice,
		/// A handover is still being applied; another one cannot be started on top of it.
		SeatingStillInProgress,
		/// The founding parliament cannot be seated once a house already sits.
		ParliamentAlreadySeated,
		/// The nominee holds none of the tikis that qualify somebody for an appointed seat
		/// on the court.
		NotQualifiedForTheCourt,
		/// The President's five seats are all taken.
		AppointedCourtSeatsAreFull,
		/// This account already sits on the court.
		AlreadyOnTheCourt,
		/// Only the sitting house elects the court's six elected seats.
		NotAParliamentMember,
		/// The caller does not hold the finance portfolio.
		NotTheFinanceMinister,
		/// Only the President approves one.
		NotThePresident,
		/// No proposal under that id.
		AirdropNotFound,
		/// Already signed by this office. Signing twice would look like two approvals.
		AlreadyApproved,
		/// Still missing a signature.
		AirdropNotApproved,
		/// Approved, but the delay has not elapsed. Only large airdrops wait.
		AirdropNotYetPayable,
		/// A reason has to be given, and it has to fit.
		AirdropReasonTooLong,
		/// A payment of nothing was requested.
		NothingToSpend,
		/// The payment is more than Parliament has approved.
		BudgetExceeded,
		/// The message to the treasury chain could not be sent.
		CouldNotReachTreasury,
		/// Voting is still open and the threshold has not been reached.
		ProposalStillOpen,
		/// That tiki is not a cabinet post.
		NotACabinetPost,
		NotAuthorizedToApprove,
		AppointmentProcessNotFound,
		NominationNotFound,
		AppointmentAlreadyProcessed,
		RoleAlreadyFilled,
		/// Nobody is both parties to an appointment: the proposer is not the appointee.
		CannotNominateSelf,
		/// There is no nominee for Prime Minister to act on.
		NoNomineeStanding,
		/// Only the Treasurer sets the emission rate.
		NotTheTreasurer,
		/// The step is larger than one change may move the rate.
		EmissionStepTooLarge,
		/// Not enough time has passed since the last change.
		EmissionChangedTooRecently,

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
		/// This account has already endorsed someone in this election.
		AlreadyEndorsed,
		/// This account has already served the most consecutive terms the office allows.
		TermLimitReached,
		/// The endorsement was not made by the account it names.
		EndorsementNotGiven,
		/// Voting on this proposal has not opened, or has closed.
		OutsideVotingWindow,
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		/// Once an era, ask whether the state has enough citizens to start paying them.
		///
		/// The register is here and the money is on the Asset Hub, so nobody over there can
		/// answer this question; they have to be told, once. What is sent is a fact about the
		/// population, not an instruction about the money -- what the treasury does with it
		/// is written into the treasury's own runtime.
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut weight = Self::process_seat_handover();
			weight = weight.saturating_add(Self::schedule_due_elections(n));
			weight = weight.saturating_add(Self::check_population_gate(n));
			weight
		}

		/// What the constitution says must always be true of the government.
		///
		/// These are not bookkeeping checks. Each one, if it broke, would leave the state in a
		/// condition it has no rule for: an office held by someone the register does not name,
		/// a mandate that outlives the office, two elections open for the same seat. The
		/// failures are silent by nature -- everything keeps working, and answers differently
		/// depending on which record is asked.
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use pezframe_support::ensure;

			// One election per office, and it has to be an election that exists and is
			// unfinished. A stale entry here would stop the scheduler from ever opening
			// another -- the office would quietly stop being contested.
			for (election_type, election_id) in OpenElection::<T>::iter() {
				let election = ActiveElections::<T>::get(election_id)
					.ok_or("an office is marked as voting on an election that does not exist")?;
				ensure!(
					election.election_type == election_type,
					"an office is marked as voting on another office's election"
				);
				ensure!(
					election.status != ElectionStatus::Completed,
					"an office is marked as voting on an election that has been counted"
				);
			}

			// A mandate with nobody serving it is not an error -- that is a vacancy, and the
			// scheduler is meant to see it. What must not happen is the reverse: somebody
			// seated in an elected office with no mandate recorded at all, which is an
			// officeholder no clock will ever remove.
			for (election_type, tiki) in [
				(ElectionType::Presidential, Tiki::Serok),
				(ElectionType::SpeakerElection, Tiki::SerokiMeclise),
			] {
				if pezpallet_tiki::Pezpallet::<T>::current_holder(&tiki).is_some() {
					ensure!(
						TermEnds::<T>::contains_key(election_type),
						"an elected office is held under no mandate"
					);
				}
			}

			// The court: eleven seats, six the house elects and five the President appoints,
			// and neither half may grow into the other's. The appointed half additionally
			// has to be qualified -- an unqualified appointee would be a bench that cannot
			// read what it rules on, arrived at by a call that was supposed to prevent it.
			let bench = DiwanMembers::<T>::get();
			ensure!(
				bench.len() as u32 <= T::DiwanSize::get(),
				"the court has more seats filled than it has"
			);
			let elected = bench
				.iter()
				.filter(|m| matches!(m.appointed_by, AppointmentAuthority::Parliament))
				.count() as u32;
			let appointed = bench.len() as u32 - elected;
			ensure!(
				elected <= T::DiwanElectedSeats::get(),
				"the house has taken more of the court than it elects"
			);
			ensure!(
				appointed <= T::DiwanSize::get().saturating_sub(T::DiwanElectedSeats::get()),
				"the President has taken more of the court than they appoint"
			);
			for member in bench.iter() {
				if matches!(member.appointed_by, AppointmentAuthority::President(_)) {
					ensure!(
						Self::qualifies_for_an_appointed_seat(&member.account),
						"an appointed member of the court holds none of the qualifying tikis"
					);
				}
			}

			// The seat and the register of who won it must name the same people.
			//
			// One direction only, and deliberately. A member the Diwan has removed, or who
			// has lost their citizenship, keeps their line in `ParliamentMembers` and loses
			// the tiki -- that gap is the design, and everything that pays or empowers a
			// parliamentarian reads the tiki, so a stale line grants nothing. What must never
			// happen is the reverse: somebody holding the seat whom no election put there.
			// That would be an authority with no mandate behind it, and the rewards pallet
			// would pay it.
			let seated: Vec<T::AccountId> = ParliamentMembers::<T>::get()
				.iter()
				.map(|member| member.account.clone())
				.collect();
			let leaving = PendingSeatRevokes::<T>::get();
			for (account, tikis) in pezpallet_tiki::UserTikis::<T>::iter() {
				if tikis.contains(&Tiki::Parlementer) {
					ensure!(
						seated.contains(&account) || leaving.contains(&account),
						"somebody holds a parliamentary seat that no election gave them"
					);
				}
			}

			// A queued handover has to have a term to hand out, and a finished one must not
			// leave the term behind: a stale term would be given to the next house.
			let handover_running = !PendingSeatGrants::<T>::get().is_empty() || !leaving.is_empty();
			ensure!(
				handover_running == PendingSeatTerm::<T>::exists(),
				"the seat handover queue and the term it hands out disagree"
			);

			// Nobody is serving more consecutive terms than the office allows. The limit is
			// checked when a candidacy is filed; this is what catches a path that seats
			// somebody without going through one.
			let limit = T::MaxConsecutiveTerms::get();
			if limit > 0 {
				for (_, _, served) in ConsecutiveTerms::<T>::iter() {
					ensure!(served <= limit, "somebody has served past the term limit");
				}
			}

			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// Open any election whose time has come, without anyone having to ask.
		///
		/// `initiate_election` requires root, and nothing called it. So an office could be
		/// won once and held forever: the term was recorded, nothing read it, and the only
		/// way a second election ever happened was somebody with sudo remembering. A state
		/// whose elections depend on an outside key is not governing itself.
		///
		/// Two things bring an election: a term running down, and an office falling empty.
		fn schedule_due_elections(n: BlockNumberFor<T>) -> Weight {
			let mut opened = 0u32;
			let lead_time = Self::election_cycle_length();

			for election_type in [
				ElectionType::Presidential,
				ElectionType::Parliamentary,
				ElectionType::SpeakerElection,
				ElectionType::ConstitutionalCourt,
			] {
				if OpenElection::<T>::contains_key(election_type) {
					continue;
				}

				let due = match TermEnds::<T>::get(election_type) {
					// A term is running: open the election early enough that the count is
					// finished by the time the mandate ends. An office that has fallen empty
					// mid-term does not wait for the clock -- the state cannot go without a
					// President because the calendar says the term has three years left.
					Some(ends_at) => {
						n.saturating_add(lead_time) >= ends_at
							|| Self::office_is_vacant(&election_type)
					},
					// No term recorded means the office has never been filled by a vote.
					// Genesis records a term for the founding government, so this is only
					// reached by an office that is meant to exist and does not yet.
					None => false,
				};

				if due
					&& Self::initiate_election(
						pezframe_system::RawOrigin::Root.into(),
						election_type,
						None,
						None,
					)
					.is_ok()
				{
					opened = opened.saturating_add(1);
					Self::deposit_event(Event::ElectionScheduled { election_type });
				}
			}

			T::DbWeight::get().reads_writes(8, (opened as u64).saturating_mul(2))
		}

		/// Whether a single-holder office that should be filled is empty.
		///
		/// Emptied by death, resignation, the loss of citizenship, an impeachment, or a term
		/// running past its grace. Whichever it was, the answer is the same: hold the
		/// election. Bodies rather than single offices -- Parliament, the Diwan -- are not
		/// checked here; losing one member of two hundred is not a vacancy in the office.
		fn office_is_vacant(election_type: &ElectionType) -> bool {
			match election_type {
				ElectionType::Presidential => {
					pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok).is_none()
				},
				ElectionType::SpeakerElection => {
					pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::SerokiMeclise).is_none()
				},
				// Parliament has no vacancy arm, deliberately. The two offices above are
				// held by one person each, so "empty" is one storage read. A house is empty
				// only when none of its two hundred and one members still holds a seat, and
				// asking that on every block means decoding the whole roll on every block.
				//
				// What is left is the clock, which covers the case the design actually has:
				// a house is replaced when its term runs out. A house that lost every seat
				// at once -- every member removed by the Diwan, or stripped of citizenship,
				// inside one term -- would sit empty until the term ended. That is recorded
				// as a gap rather than papered over with a check that costs a block read
				// every block for a case that has never happened.
				_ => false,
			}
		}

		/// Who is acting as President while the office is empty.
		///
		/// The Speaker of Parliament, which is the ordinary rule: the presiding officer of the
		/// elected house, chosen by that house, and already in the country's confidence. There
		/// is no succession here otherwise -- an empty presidency would simply stop every
		/// power that runs through it, including the one that appoints a government.
		///
		/// Acting is not holding. The tiki does not move, so nothing that reads the office
		/// reads the deputy as its holder; what this answers is who may act until the
		/// by-election the scheduler has already opened is counted.
		pub fn acting_president() -> Option<T::AccountId> {
			match pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok) {
				Some(serok) => Some(serok),
				None => pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::SerokiMeclise),
			}
		}

		/// The population gate, once an era.
		fn check_population_gate(n: BlockNumberFor<T>) -> Weight {
			let period = T::PopulationCheckPeriod::get();
			if period.is_zero() || !(n % period).is_zero() {
				return T::DbWeight::get().reads(0);
			}
			if PopulationGateReported::<T>::get() {
				return T::DbWeight::get().reads(1);
			}
			if T::CitizenSource::citizen_count() < T::PopulationThreshold::get() {
				return T::DbWeight::get().reads(2);
			}

			match Self::report_population_threshold_reached() {
				Ok(()) => {
					PopulationGateReported::<T>::put(true);
					Self::deposit_event(Event::PopulationThresholdReported {
						citizen_count: T::CitizenSource::citizen_count(),
					});
				},
				// Not latched, so the next era tries again. A send can fail for reasons that
				// pass -- no open channel yet, a full queue -- and the threshold does not
				// stop being true because the first attempt did not get through.
				Err(e) => {
					log::warn!(target: "welati", "population report could not be sent: {e:?}");
					Self::deposit_event(Event::PopulationReportFailed);
				},
			}

			T::DbWeight::get().reads_writes(3, 1)
		}
	}

	// --- Extrinsics ---
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Tally a proposal and, if it passed, carry it out.
		///
		/// Permissionless, because it decides nothing: the votes are already recorded and the
		/// threshold was fixed when the proposal was made. Anyone may ask for the sum to be
		/// taken. Before this existed, `vote_on_proposal` recorded ayes and nays and no code
		/// path anywhere ever read them -- Parliament could vote and nothing could follow.
		///
		/// A proposal passes the moment its ayes reach the threshold; it fails once voting has
		/// closed without them. Waiting for the deadline even after the votes are in would
		/// hold the state up for no reason.
		#[pezpallet::call_index(22)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::vote_on_proposal())]
		pub fn finalize_proposal(origin: OriginFor<T>, proposal_id: u32) -> DispatchResult {
			ensure_signed(origin)?;

			let mut proposal =
				ActiveProposals::<T>::get(proposal_id).ok_or(Error::<T>::ProposalNotFound)?;
			ensure!(proposal.status == ProposalStatus::Active, Error::<T>::ProposalNotActive);

			let now = pezframe_system::Pezpallet::<T>::block_number();
			let passed = proposal.aye_votes >= proposal.threshold;
			let closed = now > proposal.expires_at;
			ensure!(passed || closed, Error::<T>::ProposalStillOpen);

			proposal.status =
				if passed { ProposalStatus::Approved } else { ProposalStatus::Rejected };

			if passed {
				if let Some(amount) = proposal.budget_amount {
					let total = ApprovedBudget::<T>::mutate(|b| {
						*b = b.saturating_add(amount);
						*b
					});
					Self::deposit_event(Event::BudgetApproved { amount, total });
				}
			}

			let status = proposal.status;
			let (aye, nay, abstain) =
				(proposal.aye_votes, proposal.nay_votes, proposal.abstain_votes);
			ActiveProposals::<T>::insert(proposal_id, proposal);

			Self::deposit_event(Event::ProposalFinalized {
				proposal_id,
				result: status,
				aye_votes: aye,
				nay_votes: nay,
				abstain_votes: abstain,
			});
			Ok(())
		}

		/// Move HEZ's emission rate. The Treasurer's call, and nobody else's.
		///
		/// The office is this state's central bank, and the separation is the reason it exists
		/// separately from the finance minister at all: the one who spends must not be the one
		/// who creates the money. `WezîrêDarayiyê` draws against what Parliament appropriated
		/// and cannot reach this; the Treasurer sets what comes into being and cannot spend a
		/// penny of it.
		///
		/// Three limits, in three different places, and none of them is this officeholder's to
		/// move. The ceiling is `MAX_INFLATION_RATE` in the treasury chain's own code -- raising
		/// it is a runtime upgrade, which on this network is a referendum of the whole register.
		/// The pace is `MaxEmissionStep` and `MinEmissionInterval` here, so a mandate cannot be
		/// spent in a single call. Removal from the office is the court's, not the President's:
		/// a central bank the executive can empty is not independent of the executive.
		///
		/// The rate the treasury chain will *apply* is still clamped there. This call is the
		/// target; the ceiling is the last word.
		#[pezpallet::call_index(14)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn set_emission_rate(
			origin: OriginFor<T>,
			rate: pezsp_runtime::Perbill,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Xezinedar)
					== Some(who.clone()),
				Error::<T>::NotTheTreasurer
			);

			let now = pezframe_system::Pezpallet::<T>::block_number();
			if let Some((last_rate, changed_at)) = EmissionRate::<T>::get() {
				ensure!(
					now.saturating_sub(changed_at) >= T::MinEmissionInterval::get(),
					Error::<T>::EmissionChangedTooRecently
				);
				let step = if rate > last_rate { rate - last_rate } else { last_rate - rate };
				ensure!(step <= T::MaxEmissionStep::get(), Error::<T>::EmissionStepTooLarge);
			}

			// Recorded before sending, and the whole call reverts if the send fails -- so the
			// record cannot claim a change the treasury chain never received.
			EmissionRate::<T>::put((rate, now));
			Self::send_emission_rate(rate).map_err(|_| Error::<T>::CouldNotReachTreasury)?;

			Self::deposit_event(Event::EmissionRateSet { rate, by: who });
			Ok(())
		}

		/// Pay `amount` out of the government pot on the treasury chain.
		///
		/// The finance minister's call, and only within what Parliament approved. Three things
		/// have to be true at once and each is checked by a different body's record: the
		/// caller holds the finance portfolio (the tiki), the allowance covers it (Parliament's
		/// vote), and the pot has the money (the treasury chain's own check, on arrival).
		#[pezpallet::call_index(41)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::vote_on_proposal())]
		pub fn spend_budget(
			origin: OriginFor<T>,
			beneficiary: T::AccountId,
			amount: u128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::WezireDarayiye) == Some(who),
				Error::<T>::NotTheFinanceMinister
			);
			ensure!(amount > 0, Error::<T>::NothingToSpend);

			// Take it out of the allowance before sending. If the send fails the whole call
			// reverts, so the allowance cannot be spent twice; if it were decremented after a
			// successful send there would be a moment where the money is gone and the
			// allowance still says it is available.
			ApprovedBudget::<T>::try_mutate(|budget| -> DispatchResult {
				*budget = budget.checked_sub(amount).ok_or(Error::<T>::BudgetExceeded)?;
				Ok(())
			})?;

			Self::send_government_spend(&beneficiary, amount)
				.map_err(|_| Error::<T>::CouldNotReachTreasury)?;

			Self::deposit_event(Event::BudgetSpent { beneficiary, amount });
			Ok(())
		}

		/// Propose an airdrop out of the pot.
		///
		/// The Prime Minister's call. Nothing moves on this call alone: the President has to
		/// sign, and above `AirdropCeiling` the Treasurer as well, after which a large one
		/// still waits `LargeAirdropDelay` before it can be paid.
		///
		/// Discretionary on purpose, and the reason is a shape the register cannot express: a
		/// listing campaign pays an exchange's customers, and they are that exchange's
		/// customers rather than this chain's citizens. A rule over the register cannot reach
		/// them. What replaces the rule is the signature count, the ceiling, and the fact that
		/// the pot has no key -- so this is the only door.
		#[pezpallet::call_index(56)]
		// `nominate_official`'s weight: the same shape of work -- one storage read for the
		// office check, one bounded write for the record. Listed with the rest of the weights
		// work; this pallet has no benchmark of its own for these four yet.
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn propose_airdrop(
			origin: OriginFor<T>,
			beneficiary: T::AccountId,
			amount: u128,
			reason: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::SerokWeziran)
					== Some(who.clone()),
				Error::<T>::NotThePrimeMinister
			);
			ensure!(amount > 0, Error::<T>::NothingToSpend);
			let reason: BoundedVec<u8, ConstU32<256>> =
				reason.try_into().map_err(|_| Error::<T>::AirdropReasonTooLong)?;

			let id = NextAirdropId::<T>::mutate(|n| {
				let id = *n;
				*n = n.saturating_add(1);
				id
			});
			let now = pezframe_system::Pezpallet::<T>::block_number();
			AirdropProposals::<T>::insert(
				id,
				AirdropProposal::<T> {
					beneficiary: beneficiary.clone(),
					amount,
					reason,
					proposer: who,
					approved_by_president: false,
					approved_by_treasurer: false,
					proposed_at: now,
					// Filled in when the last signature lands; until then there is nothing to
					// wait for and a date here would be a promise the proposal has not earned.
					payable_from: now,
				},
			);
			Self::deposit_event(Event::AirdropProposed { id, beneficiary, amount });
			Ok(())
		}

		/// Sign a proposed airdrop.
		///
		/// The same call for the President and the Treasurer, because which signature a caller
		/// supplies is decided by which office they hold rather than by which extrinsic they
		/// reach for -- and an office that holds neither is refused here rather than leaving a
		/// second entry point to keep in step.
		///
		/// The Treasurer's signature is only required above the ceiling. Below it the
		/// President's approval completes the proposal and it becomes payable at once, which
		/// is what a listing needs.
		#[pezpallet::call_index(57)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::approve_appointment())]
		pub fn approve_airdrop(origin: OriginFor<T>, id: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let holder =
				|t: Tiki| pezpallet_tiki::Pezpallet::<T>::current_holder(&t) == Some(who.clone());

			AirdropProposals::<T>::try_mutate(id, |maybe| -> DispatchResult {
				let p = maybe.as_mut().ok_or(Error::<T>::AirdropNotFound)?;
				let large = p.amount > T::AirdropCeiling::get();

				if holder(Tiki::Serok) {
					ensure!(!p.approved_by_president, Error::<T>::AlreadyApproved);
					p.approved_by_president = true;
					Self::deposit_event(Event::AirdropApprovedByPresident { id });
				} else if holder(Tiki::Xezinedar) {
					ensure!(!p.approved_by_treasurer, Error::<T>::AlreadyApproved);
					p.approved_by_treasurer = true;
					Self::deposit_event(Event::AirdropApprovedByTreasurer { id });
				} else {
					return Err(Error::<T>::NotThePresident.into());
				}

				let complete = p.approved_by_president && (!large || p.approved_by_treasurer);
				if complete {
					let now = pezframe_system::Pezpallet::<T>::block_number();
					// The delay starts when the last signature lands, not when the proposal
					// was made -- otherwise a proposal left sitting would be payable the
					// moment it was signed, and the week nobody could object to it would have
					// passed before there was anything to object to.
					p.payable_from =
						if large { now.saturating_add(T::LargeAirdropDelay::get()) } else { now };
					Self::deposit_event(Event::AirdropReady { id, payable_from: p.payable_from });
				}
				Ok(())
			})
		}

		/// Pay a fully signed airdrop.
		///
		/// Anyone may call this: every condition it checks is already recorded on chain, so
		/// requiring an office here would only mean an approved payment waits on somebody
		/// remembering to press the button. What it does is send; whether the pot accepts is
		/// the pot's own check, against its own ceiling, on arrival.
		#[pezpallet::call_index(58)]
		// `vote_on_proposal`'s, which is what `spend_budget` next door uses for the same
		// read-check-send shape.
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::vote_on_proposal())]
		pub fn pay_airdrop(origin: OriginFor<T>, id: u32) -> DispatchResult {
			ensure_signed(origin)?;
			let p = AirdropProposals::<T>::get(id).ok_or(Error::<T>::AirdropNotFound)?;
			let large = p.amount > T::AirdropCeiling::get();
			ensure!(
				p.approved_by_president && (!large || p.approved_by_treasurer),
				Error::<T>::AirdropNotApproved
			);
			ensure!(
				pezframe_system::Pezpallet::<T>::block_number() >= p.payable_from,
				Error::<T>::AirdropNotYetPayable
			);

			// Removed before sending. If the send fails the whole call reverts, so the
			// proposal cannot be paid twice; removing it afterwards would leave a window in
			// which the money is gone and the proposal still says it is owed.
			AirdropProposals::<T>::remove(id);
			Self::send_airdrop_spend(&p.beneficiary, p.amount)
				.map_err(|_| Error::<T>::CouldNotReachTreasury)?;
			Self::deposit_event(Event::AirdropSent {
				id,
				beneficiary: p.beneficiary,
				amount: p.amount,
			});
			Ok(())
		}

		/// Withdraw a proposal before it is paid.
		///
		/// The Prime Minister who proposed it, or the President -- one to think better of it,
		/// the other to refuse it. Kept as its own call rather than letting a proposal expire:
		/// a refusal that leaves a record is worth more than one that looks like forgetting.
		#[pezpallet::call_index(59)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::approve_appointment())]
		pub fn cancel_airdrop(origin: OriginFor<T>, id: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let p = AirdropProposals::<T>::get(id).ok_or(Error::<T>::AirdropNotFound)?;
			let is_president =
				pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok) == Some(who.clone());
			ensure!(who == p.proposer || is_president, Error::<T>::NotThePresident);
			AirdropProposals::<T>::remove(id);
			Self::deposit_event(Event::AirdropCancelled { id });
			Ok(())
		}

		/// Open a citizens' initiative.
		///
		/// No state anywhere lets one person put a question to the whole country by themselves,
		/// and neither does this. What a citizen may do alone is *ask*: name a proposal, name
		/// the track whose authority it needs, and put up a deposit. It reaches the ballot only
		/// once enough of the register has asked with them.
		///
		/// The proposal is named by a preimage that is already stored, so what is being asked
		/// is readable before anyone signs it.
		#[pezpallet::call_index(52)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::submit_proposal())]
		pub fn open_initiative(
			origin: OriginFor<T>,
			track: u16,
			proposal: T::Hash,
			proposal_len: u32,
		) -> DispatchResult {
			let proposer = ensure_signed(origin)?;

			#[cfg(not(any(test, feature = "runtime-benchmarks")))]
			ensure!(
				<pezpallet_identity_kyc::Pezpallet<T> as KycStatus<T::AccountId>>::get_kyc_status(
					&proposer
				) == KycLevel::Approved,
				Error::<T>::NotACitizen
			);
			ensure!(T::TrustScoreSource::trust_score_of(&proposer) > 0, Error::<T>::NoTrustToVote);

			let now = pezframe_system::Pezpallet::<T>::block_number();
			if let Some(until) = InitiativeCooldownUntil::<T>::get(&proposer) {
				ensure!(now >= until, Error::<T>::InitiativeCooldown);
			}

			let deposit = T::InitiativeDeposit::get();
			T::NativeCurrency::reserve(&proposer, deposit.saturated_into())?;

			let id = NextInitiativeId::<T>::get();
			NextInitiativeId::<T>::put(id.saturating_add(1));
			let closes = now.saturating_add(T::InitiativeWindow::get());

			// The proposer counts as the first signature. Asking is backing.
			Initiatives::<T>::insert(
				id,
				crate::types::Initiative {
					proposer: proposer.clone(),
					track,
					proposal,
					proposal_len,
					closes,
					backing: 1,
					deposit,
				},
			);
			InitiativeBacking::<T>::insert(id, &proposer, ());

			Self::deposit_event(Event::InitiativeOpened { id, proposer, track, closes });
			Ok(())
		}

		/// Sign an open initiative.
		///
		/// Free, and one per citizen. Charging for it would put a price on a constitutional
		/// right, and letting one account sign twice would make the threshold measure balance
		/// rather than people.
		#[pezpallet::call_index(53)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::vote_on_proposal())]
		#[pezpallet::feeless_if(|_origin: &OriginFor<T>, id: &u32| -> bool {
			Initiatives::<T>::contains_key(id)
		})]
		pub fn back_initiative(
			origin: OriginFor<T>,
			#[pezpallet::compact] id: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			#[cfg(not(any(test, feature = "runtime-benchmarks")))]
			ensure!(
				<pezpallet_identity_kyc::Pezpallet<T> as KycStatus<T::AccountId>>::get_kyc_status(
					&who
				) == KycLevel::Approved,
				Error::<T>::NotACitizen
			);
			ensure!(T::TrustScoreSource::trust_score_of(&who) > 0, Error::<T>::NoTrustToVote);
			ensure!(!InitiativeBacking::<T>::contains_key(id, &who), Error::<T>::AlreadyBacked);

			let backing =
				Initiatives::<T>::try_mutate(id, |maybe| -> Result<u32, DispatchError> {
					let init = maybe.as_mut().ok_or(Error::<T>::NoSuchInitiative)?;
					ensure!(
						pezframe_system::Pezpallet::<T>::block_number() <= init.closes,
						Error::<T>::InitiativeClosed
					);
					init.backing = init.backing.saturating_add(1);
					Ok(init.backing)
				})?;

			InitiativeBacking::<T>::insert(id, &who, ());
			Self::deposit_event(Event::InitiativeBacked { id, who, backing });
			Ok(())
		}

		/// Put a sufficiently backed initiative to the register.
		///
		/// Permissionless: the threshold is the decision, and making the proposer come back to
		/// collect it would let an initiative that the register already backed die because one
		/// person went quiet.
		#[pezpallet::call_index(54)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::submit_proposal())]
		pub fn launch_initiative(
			origin: OriginFor<T>,
			#[pezpallet::compact] id: u32,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let init = Initiatives::<T>::get(id).ok_or(Error::<T>::NoSuchInitiative)?;
			ensure!(init.backing >= Self::initiative_threshold(), Error::<T>::NotEnoughBacking);

			T::Initiatives::launch(&init.proposer, init.track, init.proposal, init.proposal_len)?;

			// It did what it was taken for.
			T::NativeCurrency::unreserve(&init.proposer, init.deposit.saturated_into());
			Initiatives::<T>::remove(id);
			let _ = InitiativeBacking::<T>::clear_prefix(id, u32::MAX, None);

			Self::deposit_event(Event::InitiativeLaunched { id, backing: init.backing });
			Ok(())
		}

		/// Close an initiative whose window ran out without the backing.
		///
		/// The deposit is forfeit to the treasury -- not destroyed. Destroying HEZ would pay
		/// the forfeit out to every other holder instead of to the state.
		#[pezpallet::call_index(55)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::vote_on_proposal())]
		pub fn close_lapsed_initiative(
			origin: OriginFor<T>,
			#[pezpallet::compact] id: u32,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let init = Initiatives::<T>::get(id).ok_or(Error::<T>::NoSuchInitiative)?;
			ensure!(
				pezframe_system::Pezpallet::<T>::block_number() > init.closes,
				Error::<T>::InitiativeStillOpen
			);
			ensure!(init.backing < Self::initiative_threshold(), Error::<T>::NotEnoughBacking);

			let deposit: <<T as pezpallet::Config>::NativeCurrency as Currency<
				T::AccountId,
			>>::Balance = init.deposit.saturated_into();
			T::NativeCurrency::unreserve(&init.proposer, deposit);
			let (forfeit, _) = T::NativeCurrency::slash(&init.proposer, deposit);
			T::InitiativeSlashTarget::on_unbalanced(forfeit);

			InitiativeCooldownUntil::<T>::insert(
				&init.proposer,
				pezframe_system::Pezpallet::<T>::block_number()
					.saturating_add(T::InitiativeCooldown::get()),
			);

			Initiatives::<T>::remove(id);
			let _ = InitiativeBacking::<T>::clear_prefix(id, u32::MAX, None);

			Self::deposit_event(Event::InitiativeLapsed { id, backing: init.backing });
			Ok(())
		}

		/// Answer a state referendum: one citizen, one voice.
		///
		/// Nothing is staked and nothing is locked. The count is of people, so weight cannot be
		/// bought, and a citizen who owns nothing answers as loudly as one who owns everything
		/// -- which is the whole reason state questions are settled here rather than where the
		/// tokens are.
		///
		/// Two conditions, and both are about standing rather than stake: the caller has to be
		/// an approved citizen, and their trust has to be above zero. Trust of zero is technical
		/// death, and the dead do not vote.
		///
		/// Answering the other way while the question is open replaces the earlier answer. That
		/// is not a second vote -- the tally moves by one either way, never by two.
		#[pezpallet::call_index(50)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::vote_on_proposal())]
		pub fn answer_referendum(
			origin: OriginFor<T>,
			#[pezpallet::compact] poll: u32,
			aye: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			#[cfg(not(any(test, feature = "runtime-benchmarks")))]
			ensure!(
				<pezpallet_identity_kyc::Pezpallet<T> as KycStatus<T::AccountId>>::get_kyc_status(
					&who
				) == KycLevel::Approved,
				Error::<T>::NotACitizen
			);
			ensure!(T::TrustScoreSource::trust_score_of(&who) > 0, Error::<T>::NoTrustToVote);

			T::Polls::try_access_poll(poll, |status| {
				let (tally, _class) =
					status.ensure_ongoing().ok_or(Error::<T>::ReferendumNotOngoing)?;

				match ReferendumVotes::<T>::get(poll, &who) {
					Some(previous) if previous == aye => {
						return Err(Error::<T>::AlreadyAnsweredThatWay.into())
					},
					// Take the earlier answer back out before putting the new one in, or the
					// citizen would be counted on both sides at once.
					Some(true) => tally.ayes = tally.ayes.saturating_sub(1),
					Some(false) => tally.nays = tally.nays.saturating_sub(1),
					None => {},
				}

				if aye {
					tally.ayes = tally.ayes.saturating_add(1);
				} else {
					tally.nays = tally.nays.saturating_add(1);
				}
				ReferendumVotes::<T>::insert(poll, &who, aye);
				Ok(())
			})?;

			Self::deposit_event(Event::ReferendumVoteCast { who, poll, aye });
			Ok(())
		}

		/// Discard the answers to a referendum that has finished.
		///
		/// Permissionless, because nothing here belongs to anyone: no deposit is held against
		/// these entries and no one is disadvantaged by their removal. Without it the map only
		/// ever grows, one entry per citizen per question, forever.
		///
		/// `limit` bounds the work so a long-running question can be cleared across several
		/// calls rather than in one block nobody else fits into.
		#[pezpallet::call_index(51)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::vote_on_proposal())]
		pub fn clear_referendum_answers(
			origin: OriginFor<T>,
			#[pezpallet::compact] poll: u32,
			limit: u32,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(T::Polls::as_ongoing(poll).is_none(), Error::<T>::ReferendumStillOngoing);

			let removed = ReferendumVotes::<T>::clear_prefix(poll, limit, None);
			Self::deposit_event(Event::ReferendumVotesCleared { poll, removed: removed.unique });
			Ok(())
		}

		/// Endorse a candidacy.
		///
		/// Signed by the endorser, which is the whole point of it. One endorsement per person
		/// per election: endorsing everybody would say nothing, and a nomination threshold
		/// that can be met by one person supporting every candidate is not a threshold.
		///
		/// Only during the candidacy period, so that support is given before campaigning
		/// rather than gathered from people who have already seen the field.
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::register_candidate())]
		pub fn endorse_candidate(
			origin: OriginFor<T>,
			election_id: u32,
			candidate: T::AccountId,
		) -> DispatchResult {
			let endorser = ensure_signed(origin)?;

			let election =
				ActiveElections::<T>::get(election_id).ok_or(Error::<T>::ElectionNotFound)?;
			let now = pezframe_system::Pezpallet::<T>::block_number();
			ensure!(now <= election.candidacy_deadline, Error::<T>::CandidacyPeriodExpired);

			#[cfg(not(any(test, feature = "runtime-benchmarks")))]
			{
				ensure!(
					<pezpallet_identity_kyc::Pezpallet<T> as KycStatus<T::AccountId>>::get_kyc_status(
						&endorser
					) == KycLevel::Approved,
					Error::<T>::NotACitizen
				);
			}

			ensure!(
				!Endorsements::<T>::contains_key(election_id, &endorser),
				Error::<T>::AlreadyEndorsed
			);

			Endorsements::<T>::insert(election_id, &endorser, &candidate);
			Self::deposit_event(Event::CandidateEndorsed { election_id, endorser, candidate });
			Ok(())
		}

		/// Appoint one of the President's five seats on the court. His call alone.
		///
		/// The court is eleven: the house elects six, the President appoints five. That split
		/// *is* the separation -- each half is chosen by a body the other does not control, and
		/// neither can seat a majority. Putting a parliamentary confirmation on top of the
		/// President's five was tried and reverted: it would have let the house reach all
		/// eleven seats while its own six stayed beyond the President's reach, which is not a
		/// stronger separation but a one-sided one. The American analogy does not carry here,
		/// because there the Senate elects no justices at all.
		///
		/// Unlike the elected six, the appointee has to be qualified -- see
		/// `qualifies_for_an_appointed_seat`. There is no matching dismissal call, and that is
		/// the point: a court the President can empty is not a check on the President. A seat
		/// ends when its nine years run out, when the Diwan itself removes the holder, or
		/// when the holder stops being a citizen.
		#[pezpallet::call_index(35)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn appoint_diwan_member(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			Self::ensure_root_or_serok(origin)?;
			Self::a_court_seat_is_open_for(&who)?;

			let president = pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok)
				.unwrap_or_else(|| who.clone());
			let mut bench = DiwanMembers::<T>::get();
			let now = pezframe_system::Pezpallet::<T>::block_number();
			let term_end = now.saturating_add(T::CourtTermLength::get());

			bench
				.try_push(DiwanMember {
					account: who.clone(),
					appointed_at: now,
					term_ends_at: term_end,
					appointed_by: AppointmentAuthority::President(president.clone()),
				})
				.map_err(|_| Error::<T>::DiwanFull)?;
			DiwanMembers::<T>::put(bench);
			Self::publish_the_bench();

			Self::seat_on_the_bench(&who, term_end.saturating_add(Self::election_cycle_length()))?;

			Self::deposit_event(Event::DiwanMemberAppointed {
				member: who,
				appointed_by: AppointmentAuthority::President(president),
			});
			Ok(())
		}

		/// Seat the founding parliament, before the first election has been held.
		///
		/// A state cannot legislate itself into existence: the house that passes the first
		/// budget has to be there before there is anyone to elect it. So the President seats
		/// it, once, and only while no house sits.
		///
		/// It is temporary by construction rather than by promise. The seats carry the
		/// institutional term, exactly as elected ones do, and seating the house starts the
		/// clock that makes the scheduler open the first real election. When that election is
		/// counted these members are replaced wholesale, through the same queue.
		///
		/// No minimum size is imposed. `get_voting_threshold` counts against `ParliamentSize`
		/// -- the constant, not the number of people sitting -- so a founding house of twenty
		/// cannot pass anything a house of two hundred and one could not. The arithmetic
		/// already gates this; a second gate here would only repeat it.
		///
		/// Root is accepted for as long as sudo exists, like every other Presidential power.
		#[pezpallet::call_index(34)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn seat_founding_parliament(
			origin: OriginFor<T>,
			members: Vec<T::AccountId>,
		) -> DispatchResult {
			Self::ensure_root_or_serok(origin)?;
			ensure!(ParliamentMembers::<T>::get().is_empty(), Error::<T>::ParliamentAlreadySeated);
			ensure!(!members.is_empty(), Error::<T>::ParliamentFull);

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			let term_end = Self::begin_term(&ElectionType::Parliamentary);

			let seated: BoundedVec<_, T::ParliamentSize> = members
				.iter()
				.map(|who| ParliamentMember {
					account: who.clone(),
					elected_at: current_block,
					term_ends_at: term_end,
					votes_participated: 0,
					total_votes_eligible: 0,
					participation_rate: 100,
					committees: Default::default(),
				})
				.collect::<Vec<_>>()
				.try_into()
				.map_err(|_| Error::<T>::ParliamentFull)?;

			ParliamentMembers::<T>::put(seated);
			T::HouseRoster::set_members(members.clone());
			Self::queue_seat_handover(&[], &members, term_end)?;

			Self::deposit_event(Event::FoundingParliamentSeated {
				members,
				term_ends_at: term_end,
			});
			Ok(())
		}

		/// Nominate the Prime Minister.
		///
		/// The President names the head of government and the House seats him. The comment
		/// that used to stand here said this was the President's call alone, on the grounds
		/// that what limits him is the budget rather than the appointment. That is a check on
		/// what the office can spend, not on who holds it, and a head of government nobody
		/// else agreed to is an executive appointing his own second executive.
		///
		/// Root is accepted for as long as sudo exists. When it goes, this reads as Serok
		/// alone, and the line below is the only thing that has to change.
		#[pezpallet::call_index(30)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn appoint_prime_minister(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			Self::ensure_root_or_serok(origin)?;
			PendingPrimeMinister::<T>::put(who.clone());
			Self::deposit_event(Event::PrimeMinisterNominated { nominee: who });
			Ok(())
		}

		/// Confirm the President's nominee for Prime Minister, and seat him.
		#[pezpallet::call_index(38)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn confirm_prime_minister(origin: OriginFor<T>) -> DispatchResult {
			T::ConfirmationOrigin::ensure_origin(origin)?;

			let who = PendingPrimeMinister::<T>::take().ok_or(Error::<T>::NoNomineeStanding)?;
			Self::seat_unique_tiki(&who, Tiki::SerokWeziran)?;
			Self::deposit_event(Event::PrimeMinisterAppointed { who });
			Ok(())
		}

		/// Refuse the President's nominee for Prime Minister.
		///
		/// Saying no out loud, rather than by never saying yes. A nomination left standing
		/// forever is a refusal the record cannot see.
		#[pezpallet::call_index(39)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn reject_prime_minister(origin: OriginFor<T>) -> DispatchResult {
			T::ConfirmationOrigin::ensure_origin(origin)?;

			let who = PendingPrimeMinister::<T>::take().ok_or(Error::<T>::NoNomineeStanding)?;
			Self::deposit_event(Event::PrimeMinisterRejected { nominee: who });
			Ok(())
		}

		/// Dismiss the Prime Minister, leaving the office empty.
		///
		/// The cabinet he appointed is left standing. Emptying it here would mean a single
		/// call could unseat the whole government, and the offices that keep the state running
		/// -- the finance minister above all -- would go dark at the moment there is nobody to
		/// refill them.
		#[pezpallet::call_index(31)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn dismiss_prime_minister(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_root_or_serok(origin)?;
			Self::vacate_unique_tiki(Tiki::SerokWeziran)?;
			Self::deposit_event(Event::PrimeMinisterDismissed);
			Ok(())
		}

		/// Appoint a minister. The Prime Minister's call.
		#[pezpallet::call_index(32)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn appoint_minister(
			origin: OriginFor<T>,
			who: T::AccountId,
			tiki: Tiki,
		) -> DispatchResult {
			Self::ensure_prime_minister(origin)?;
			ensure!(Self::is_cabinet_tiki(&tiki), Error::<T>::NotACabinetPost);

			if pezpallet_tiki::Pezpallet::<T>::is_unique_role(&tiki) {
				Self::seat_unique_tiki(&who, tiki)?;
			} else {
				pezpallet_tiki::Pezpallet::<T>::internal_grant_role(&who, tiki)?;
			}

			Self::deposit_event(Event::MinisterAppointed { who, tiki });
			Ok(())
		}

		/// Dismiss a minister. The Prime Minister's call.
		///
		/// Takes the account as well as the post because `Wezir` can be held by several
		/// people at once; naming only the post would be ambiguous for exactly the ministries
		/// the generic tiki exists to cover.
		#[pezpallet::call_index(33)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn dismiss_minister(
			origin: OriginFor<T>,
			who: T::AccountId,
			tiki: Tiki,
		) -> DispatchResult {
			Self::ensure_prime_minister(origin)?;
			ensure!(Self::is_cabinet_tiki(&tiki), Error::<T>::NotACabinetPost);
			pezpallet_tiki::Pezpallet::<T>::internal_revoke_role(&who, tiki)?;
			Self::deposit_event(Event::MinisterDismissed { who, tiki });
			Ok(())
		}

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
			// One election per office at a time. A runoff replaces the round it follows.
			OpenElection::<T>::insert(election_type, election_id);

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
				// The required role has to be the one that is checked. This used to discard
				// it -- `_required_tiki` -- and ask instead whether the candidate's total
				// tiki score was above zero, which is true of every citizen, because
				// citizenship itself is worth ten points. So the Speaker's election, which
				// exists to be contested among sitting members of Parliament, was open to
				// anybody in the country.
				if let Some(required_tiki) = Self::get_required_tiki(&election.election_type) {
					ensure!(
						pezpallet_tiki::Pezpallet::<T>::has_tiki(&candidate, &required_tiki),
						Error::<T>::MissingRequiredTiki
					);
				}
			}

			// Terms served in a row, if the country has set a limit. An office that can be
			// won indefinitely stops being an office and becomes a possession.
			let limit = T::MaxConsecutiveTerms::get();
			if limit > 0 {
				ensure!(
					ConsecutiveTerms::<T>::get(election.election_type, &candidate) < limit,
					Error::<T>::TermLimitReached
				);
			}

			let required_endorsements = Self::get_required_endorsements(&election.election_type);
			ensure!(
				endorsers.len() as u32 >= required_endorsements,
				Error::<T>::InsufficientEndorsements
			);

			// Every name on the list has to have said so itself, on-chain, for this
			// candidate. Deliberately not behind the test bypass the other checks use: the
			// thing this replaced was a check that did not check, and a fix that only runs
			// outside the tests is the same mistake wearing a different hat.
			for endorser in &endorsers {
				ensure!(
					Endorsements::<T>::get(election_id, endorser).as_ref() == Some(&candidate),
					Error::<T>::EndorsementNotGiven
				);
			}

			#[cfg(not(any(test, feature = "runtime-benchmarks")))]
			{
				for endorser in &endorsers {
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

			// The court is the one election with a restricted electorate. Six of its eleven
			// seats belong to the house, so the house votes for them -- not the whole
			// citizenry. The other five are the President's, and are not voted for at all.
			if election.election_type == ElectionType::ConstitutionalCourt {
				ensure!(
					pezpallet_tiki::Pezpallet::<T>::has_tiki(&voter, &Tiki::Parlementer),
					Error::<T>::NotAParliamentMember
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
			// Permissionless. Counting decides nothing: the votes are cast, the rules were
			// fixed when the election opened, and the result is whatever the arithmetic says.
			// Requiring root here meant the count -- not just the calling of an election, the
			// *count* -- waited on an outside key.
			ensure_signed(origin)?;

			let mut election =
				ActiveElections::<T>::get(election_id).ok_or(Error::<T>::ElectionNotFound)?;

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			ensure!(current_block > election.end_block, Error::<T>::ElectionNotActive);

			ensure!(
				election.status != ElectionStatus::Completed,
				Error::<T>::ElectionAlreadyFinalized
			);

			let outcome = match Self::calculate_election_winners(election_id, &election) {
				Ok(outcome) => outcome,
				// An election the country did not turn out for used to stay `Active` for
				// ever: the error left the record untouched, so it could never be counted
				// again and nothing would open a replacement. The office simply stopped
				// existing. A failed election has to end in a definite state.
				Err(Error::<T>::QuorumNotMet) => {
					election.status = ElectionStatus::FailedForTurnout;
					ActiveElections::<T>::insert(election_id, election.clone());
					OpenElection::<T>::remove(election.election_type);
					FailedAttempts::<T>::mutate(election.election_type, |n| {
						*n = n.saturating_add(1)
					});
					Self::return_candidacy_deposits(election_id, &election);
					Self::deposit_event(Event::ElectionFailedForTurnout {
						election_id,
						election_type: election.election_type,
					});
					return Ok(());
				},
				Err(e) => return Err(e.into()),
			};

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
					OpenElection::<T>::remove(election.election_type);
					FailedAttempts::<T>::remove(election.election_type);
					Self::return_candidacy_deposits(election_id, &election);

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
            // Fee exemption must mirror the in-body authorization check exactly
            // (Serok or a Minister) -- NOT the broader is_governance_member set,
            // otherwise unauthorized governance-adjacent accounts (Parliament/Diwan
            // members) could spam this call for free and always fail authorization.
            match ensure_signed(origin.clone()) {
                Ok(who) => {
                    Pezpallet::<T>::is_serok(&who) || Pezpallet::<T>::is_minister(&who)
                },
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
			let is_serok = pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok)
				== Some(nominator.clone());
			let is_minister = Self::is_minister(&nominator);

			ensure!(is_serok || is_minister, Error::<T>::NotAuthorizedToNominate);

			// An appointment has two parties and nobody is both of them. Without this the
			// President -- who may nominate and, below, approve -- appoints himself to any
			// office in the civil service in two calls of his own.
			ensure!(nominator != nominee, Error::<T>::CannotNominateSelf);

			// Only where the office genuinely has one seat. A state has many judges, many
			// notaries and many teachers; treating every appointed post as single-holder
			// meant the second one could never be appointed. `tiki` is the register and it
			// already says which offices are unique -- the Treasurer and the Ambassador among
			// these, not the bench.
			let tiki = Self::tiki_for_role(&role);
			if pezpallet_tiki::Pezpallet::<T>::is_unique_role(&tiki) {
				ensure!(
					pezpallet_tiki::Pezpallet::<T>::current_holder(&tiki).is_none(),
					Error::<T>::RoleAlreadyFilled
				);
			}

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
				// Who confirms depends on the office, and the office says so. `OfficialRole`
				// has carried `requires_parliament_approval` since it was written and nothing
				// ever asked it: every appointment went to the President, including the ones
				// the type says a parliament must confirm.
				//
				// This is the shape of a presidential system. The executive proposes and
				// another body disposes, and which offices need the second body is drawn by
				// law rather than by the executive -- the same line the American constitution
				// draws between principal and inferior officers. `confirmation_is_required`
				// asks the legislature's list first and the founding one only where the
				// legislature has not spoken.
				status: if Self::confirmation_is_required(&role) {
					AppointmentStatus::WaitingParliamentaryApproval
				} else {
					AppointmentStatus::WaitingPresidentialApproval
				},
				documents,
				confirmed_by: None,
				confirmed_at: None,
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
			let is_serok = pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok)
				== Some(approver.clone());
			ensure!(is_serok, Error::<T>::NotAuthorizedToApprove);

			// Get appointment process
			let mut process = AppointmentProcesses::<T>::get(process_id)
				.ok_or(Error::<T>::AppointmentProcessNotFound)?;

			// Check status
			ensure!(
				process.status == AppointmentStatus::WaitingPresidentialApproval,
				Error::<T>::AppointmentAlreadyProcessed
			);

			// Re-validate that the role is still unfilled. Two competing
			// AppointmentProcess entries (different nominees, same role) can both
			// reach WaitingPresidentialApproval; without this check, approving a
			// second stale process would silently overwrite an already-appointed
			// official with no removal event and no error.
			let tiki = Self::tiki_for_role(&process.position);
			if pezpallet_tiki::Pezpallet::<T>::is_unique_role(&tiki) {
				if let Some(current) = pezpallet_tiki::Pezpallet::<T>::current_holder(&tiki) {
					ensure!(current == process.nominee, Error::<T>::RoleAlreadyFilled);
				}
			}

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
			process.confirmed_by = Some(ConfirmedBy::Appointer(approver.clone()));
			process.confirmed_at = Some(current_block);

			// Store updates
			PendingNominations::<T>::insert(process.position, &process.nominee, nomination);
			AppointmentProcesses::<T>::insert(process_id, process.clone());

			// Seat them in the register everything else reads. Writing only to a map of
			// this pallet's own left an appointed official holding no tiki at all: every
			// authority check in the state asks the register, and the register had never
			// heard of them.
			pezpallet_tiki::Pezpallet::<T>::internal_grant_role(&process.nominee, tiki)?;

			Self::deposit_event(Event::AppointmentApproved {
				process_id,
				approver,
				appointee: process.nominee,
				role: process.position,
			});

			Ok(())
		}

		/// Confirm an appointment the executive proposed.
		///
		/// Takes a process number and nothing else, and that is the design rather than an
		/// economy. A confirming body that could name an account would be choosing the
		/// officeholder, which is a parliamentary system; here it answers yes or no to a name
		/// it did not pick. In the American arrangement this repository borrows the shape from,
		/// the Senate has never been able to nominate -- roughly nine cabinet nominees have
		/// ever been rejected outright, and the check works anyway, because a President does
		/// not send a name that will be refused.
		#[pezpallet::call_index(12)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::approve_appointment())]
		pub fn confirm_appointment(origin: OriginFor<T>, process_id: u32) -> DispatchResult {
			T::ConfirmationOrigin::ensure_origin(origin)?;

			let mut process = AppointmentProcesses::<T>::get(process_id)
				.ok_or(Error::<T>::AppointmentProcessNotFound)?;
			ensure!(
				process.status == AppointmentStatus::WaitingParliamentaryApproval,
				Error::<T>::AppointmentAlreadyProcessed
			);

			let tiki = Self::tiki_for_role(&process.position);
			if pezpallet_tiki::Pezpallet::<T>::is_unique_role(&tiki) {
				if let Some(current) = pezpallet_tiki::Pezpallet::<T>::current_holder(&tiki) {
					ensure!(current == process.nominee, Error::<T>::RoleAlreadyFilled);
				}
			}

			let mut nomination = PendingNominations::<T>::get(process.position, &process.nominee)
				.ok_or(Error::<T>::NominationNotFound)?;
			nomination.approved = true;
			nomination.approved_at = Some(pezframe_system::Pezpallet::<T>::block_number());
			nomination.status = NominationStatus::Approved;
			process.status = AppointmentStatus::Approved;
			process.confirmed_by = Some(ConfirmedBy::Parliament);
			process.confirmed_at = nomination.approved_at;

			PendingNominations::<T>::insert(process.position, &process.nominee, nomination);
			AppointmentProcesses::<T>::insert(process_id, process.clone());
			pezpallet_tiki::Pezpallet::<T>::internal_grant_role(&process.nominee, tiki)?;

			Self::deposit_event(Event::AppointmentConfirmedByParliament {
				process_id,
				appointee: process.nominee,
				role: process.position,
			});

			Ok(())
		}

		/// Move an office on or off the list that needs confirming.
		///
		/// The legislature draws this line, and it is the same body that does the confirming --
		/// so it can hand a power away but cannot hand itself one the executive did not offer.
		/// Deliberately not reachable by the President: an executive able to decide which of
		/// his own appointments need consent has no check on him at all.
		///
		/// It binds appointments started after it, not ones already in flight: a process that
		/// is already waiting on a body keeps waiting on that body, because moving the
		/// goalposts under a pending nomination is how a confirmation requirement gets erased
		/// one nomination at a time.
		#[pezpallet::call_index(13)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::nominate_official())]
		pub fn set_confirmation_requirement(
			origin: OriginFor<T>,
			role: OfficialRole,
			required: bool,
		) -> DispatchResult {
			T::ConfirmationOrigin::ensure_origin(origin)?;

			if required == role.requires_parliament_approval() {
				// Back to the founding rule: drop the entry rather than store a copy of it, so
				// the map keeps meaning "where the legislature departed from the constitution".
				ConfirmationRequired::<T>::remove(role);
			} else {
				ConfirmationRequired::<T>::insert(role, required);
			}

			Self::deposit_event(Event::ConfirmationRequirementSet { role, required });

			Ok(())
		}

		#[pezpallet::call_index(20)]
		#[pezpallet::weight(<T as pezpallet::Config>::WeightInfo::submit_proposal())]
		#[pezpallet::feeless_if(|origin: &OriginFor<T>, _title: &BoundedVec<u8, ConstU32<100>>, _description: &BoundedVec<u8, ConstU32<1000>>, decision_type: &CollectiveDecisionType, _priority: &ProposalPriority, _budget_amount: &Option<u128>| -> bool {
            // Fee exemption must mirror can_propose() exactly (Serok, or Parliament
            // member/Serok for everything else) -- NOT the broader is_governance_member
            // set, otherwise Diwan members/Ministers could spam this call for free and
            // always fail authorization.
            match ensure_signed(origin.clone()) {
                Ok(who) => Pezpallet::<T>::can_propose(&who, decision_type).unwrap_or(false),
                Err(_) => false,
            }
        })]
		pub fn submit_proposal(
			origin: OriginFor<T>,
			title: BoundedVec<u8, ConstU32<100>>,
			description: BoundedVec<u8, ConstU32<1000>>,
			decision_type: CollectiveDecisionType,
			priority: ProposalPriority,
			budget_amount: Option<u128>,
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
				budget_amount,
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
		#[pezpallet::feeless_if(|origin: &OriginFor<T>, proposal_id: &u32, _vote: &VoteChoice, _rationale: &Option<BoundedVec<u8, ConstU32<500>>>| -> bool {
            // Fee exemption must mirror the in-body per-decision-type authorization
            // check exactly -- NOT the broader is_governance_member set, otherwise
            // accounts unauthorized for this specific proposal's decision type
            // (e.g. Ministers, who are never authorized to vote) could spam this
            // call for free and always fail authorization.
            match ensure_signed(origin.clone()) {
                Ok(who) => ActiveProposals::<T>::get(proposal_id)
                    .map(|proposal| Pezpallet::<T>::is_authorized_to_vote(&proposal.decision_type, &who))
                    .unwrap_or(false),
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

			// Enforce access control based on decision type (shared with feeless_if
			// via is_authorized_to_vote so the two checks cannot drift apart)
			ensure!(
				Self::is_authorized_to_vote(&proposal.decision_type, &voter),
				Error::<T>::NotAuthorizedToVote
			);

			// The window was written onto the proposal and never read, so a member could vote
			// before debate opened or long after it closed -- and since a proposal passes the
			// moment its ayes reach the threshold, a late vote could carry something the
			// house had already let lapse.
			let now = pezframe_system::Pezpallet::<T>::block_number();
			ensure!(
				now >= proposal.voting_starts_at && now <= proposal.expires_at,
				Error::<T>::OutsideVotingWindow
			);

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
			let current_serok = pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok)
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

		/// What one citizen's vote counts for: one, in every election.
		///
		/// A constitutional rule, not a parameter. Standing decides who may *stand* -- trust
		/// gates candidacy, and stake feeds trust -- but it must not decide how much a vote
		/// weighs. The moment it does, the state is one where the well-placed outvote the
		/// many, which is the thing a chain-based state is supposed not to be.
		///
		/// This used to return `trust_score / 100`, clamped to ten, for the Diwan and the
		/// Speaker. Two things were wrong with it. The first is the rule above. The second
		/// was arithmetic: `total_votes` accumulated the *weights* while turnout divided by
		/// the citizen *count*, so a hundred voters at weight ten read as a thousand -- a
		/// quorum could be cleared with a fiftieth of the participation it asked for.
		///
		/// Do not reintroduce weighting here. If a vote ever needs to count for more than
		/// one, turnout has to stop being measured in the same units as the tally.
		pub fn calculate_vote_weight(_voter: &T::AccountId, _election_type: &ElectionType) -> u32 {
			1
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
			// Enforce the minimum turnout quorum recorded on the election at initiation
			// time. Without this check, elections could be finalized (and officeholders
			// seated) with arbitrarily low citizen participation.
			let total_citizen_count = Self::get_total_citizen_count();
			if total_citizen_count > 0 && FailedAttempts::<T>::get(election.election_type) == 0 {
				let turnout_percentage = ((election.total_votes as u64).saturating_mul(100)
					/ total_citizen_count as u64) as u8;
				ensure!(turnout_percentage >= election.minimum_turnout, Error::<T>::QuorumNotMet);
			}

			let mut candidates_with_votes: Vec<(T::AccountId, u32)> = election
				.candidates
				.iter()
				.filter_map(|candidate| {
					ElectionCandidates::<T>::get(election_id, candidate)
						.map(|info| (candidate.clone(), info.vote_count))
				})
				.collect();

			candidates_with_votes.sort_by_key(|(_, votes)| core::cmp::Reverse(*votes));

			match election.election_type {
				ElectionType::Presidential => {
					if candidates_with_votes.is_empty() {
						return Ok(ElectionOutcome::Winners(Default::default()));
					}
					let total_valid_votes =
						candidates_with_votes.iter().map(|(_, v)| *v).sum::<u32>().max(1);
					let (top_winner, top_vote_count) = candidates_with_votes[0].clone();

					if ((top_vote_count as u64).saturating_mul(100)) / (total_valid_votes as u64)
						>= 50
					{
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
						.take(T::DiwanElectedSeats::get() as usize)
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
						// The ballot moves the tiki. Before this, an election recorded a
						// winner here and the Serok tiki stayed with whoever an admin had
						// last granted it to -- so "the President" meant two different
						// people depending on which pallet was asked.
						let ends_at = Self::begin_term(election_type);
						Self::seat_elected_tiki(winner, Tiki::Serok, ends_at)?;
						Self::record_consecutive_term(election_type, winner);
					}
				},
				ElectionType::Parliamentary => {
					// Guard against wiping the sitting Parliament when an election produces
					// no winners (e.g. zero candidates registered). Leave the previous
					// Parliament untouched in that case, mirroring the Presidential and
					// SpeakerElection arms below.
					if !winners.is_empty() {
						let current_block = pezframe_system::Pezpallet::<T>::block_number();
						let term_end = Self::begin_term(election_type);

						// The Speaker's mandate came from the house that has just been
						// replaced, so it ends with it. Clearing the term here is what makes
						// the scheduler open a Speaker election for the new house.
						let _ = Self::vacate_unique_tiki(Tiki::SerokiMeclise);
						TermEnds::<T>::remove(ElectionType::SpeakerElection);

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

						let outgoing: Vec<T::AccountId> = ParliamentMembers::<T>::get()
							.iter()
							.map(|member| member.account.clone())
							.collect();

						ParliamentMembers::<T>::put(
							parliament_members.map_err(|_| Error::<T>::ParliamentFull)?,
						);
						T::HouseRoster::set_members(winners.to_vec());
						Self::queue_seat_handover(&outgoing, winners, term_end)?;

						Self::deposit_event(Event::ParliamentUpdated {
							new_members: winners.to_vec(),
							term_start: current_block,
						});
					}
				},
				ElectionType::SpeakerElection => {
					if let Some(winner) = winners.first() {
						// The Speaker is chosen from the sitting Parliament, so the mandate
						// is Parliament's remaining one rather than a fresh term of its own.
						// A Speaker elected by the previous Parliament presiding over the
						// next one would be an officeholder nobody in the room had chosen.
						let ends_at = TermEnds::<T>::get(ElectionType::Parliamentary)
							.unwrap_or_else(|| Self::begin_term(election_type));
						TermEnds::<T>::insert(election_type, ends_at);
						Self::seat_elected_tiki(winner, Tiki::SerokiMeclise, ends_at)?;
						Self::record_consecutive_term(election_type, winner);
					}
				},
				ElectionType::ConstitutionalCourt => {
					// Same non-destructive guard as Parliamentary: an empty winners list
					// leaves the sitting Diwan untouched instead of wiping it.
					if !winners.is_empty() {
						let current_block = pezframe_system::Pezpallet::<T>::block_number();
						let term_end = Self::begin_term(election_type);

						// Only the elected half turns over. The President's five sit out
						// their own nine years -- a court that emptied every time the house
						// voted would be a committee of the house, which is the one thing a
						// constitutional court must not be.
						let mut bench: BoundedVec<DiwanMember<T>, T::DiwanSize> =
							DiwanMembers::<T>::get()
								.into_iter()
								.filter(|member| {
									!matches!(member.appointed_by, AppointmentAuthority::Parliament)
								})
								.collect::<Vec<_>>()
								.try_into()
								.map_err(|_| Error::<T>::DiwanFull)?;

						let outgoing: Vec<T::AccountId> = DiwanMembers::<T>::get()
							.iter()
							.filter(|member| {
								matches!(member.appointed_by, AppointmentAuthority::Parliament)
							})
							.map(|member| member.account.clone())
							.collect();

						for winner in winners {
							bench
								.try_push(DiwanMember {
									account: winner.clone(),
									appointed_at: current_block,
									term_ends_at: term_end,
									appointed_by: AppointmentAuthority::Parliament,
								})
								.map_err(|_| Error::<T>::DiwanFull)?;
						}

						DiwanMembers::<T>::put(bench);
						Self::hand_over_bench_seats(&outgoing, winners, term_end)?;
						Self::publish_the_bench();

						for winner in winners {
							Self::deposit_event(Event::DiwanMemberAppointed {
								member: winner.clone(),
								appointed_by: AppointmentAuthority::Parliament,
							});
						}
					}
				},
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
					Ok(pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok)
						== Some(proposer.clone()))
				},
				_ => {
					let is_parliamentarian = ParliamentMembers::<T>::get()
						.iter()
						.any(|member| member.account == *proposer);
					let is_president = pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok)
						== Some(proposer.clone());

					Ok(is_parliamentarian || is_president)
				},
			}
		}

		/// Check voting authority for a given decision type. Shared by
		/// `vote_on_proposal`'s in-body enforcement and its `feeless_if` fee
		/// exemption gate so the two checks can never drift apart.
		pub fn is_authorized_to_vote(
			decision_type: &CollectiveDecisionType,
			voter: &T::AccountId,
		) -> bool {
			match decision_type {
				CollectiveDecisionType::ParliamentSimpleMajority
				| CollectiveDecisionType::ParliamentSuperMajority
				| CollectiveDecisionType::ParliamentAbsoluteMajority
				| CollectiveDecisionType::VetoOverride => Self::is_parliament_member(voter),
				CollectiveDecisionType::ExecutiveDecision => Self::is_serok(voter),
				CollectiveDecisionType::HybridDecision => {
					Self::is_parliament_member(voter) || Self::is_serok(voter)
				},
			}
		}

		/// Accept Root or the sitting President, and nobody else.
		///
		/// Root is here because sudo still exists. Removing sudo later is not a change to this
		/// pallet: it is dropping this arm, after which the President is the only way in.
		fn ensure_root_or_serok(origin: OriginFor<T>) -> DispatchResult {
			match pezframe_system::ensure_signed_or_root(origin)? {
				None => Ok(()),
				Some(who) => {
					ensure!(Self::is_serok(&who), Error::<T>::NotAuthorizedToNominate);
					Ok(())
				},
			}
		}

		/// Accept the account currently holding the `SerokWeziran` tiki.
		///
		/// Read from the tiki rather than from a separate register, because the tiki is what
		/// every other pallet reads. An office recorded in two places is an office that can be
		/// held by two different people depending on who is asking.
		/// How many signatures an initiative needs, as a count of the roll today.
		///
		/// Recomputed rather than stored: the register grows, and a threshold fixed when an
		/// initiative opened would be the share of a country that no longer exists. At least
		/// one, so an empty register cannot let a proposal through unasked.
		pub fn initiative_threshold() -> u32 {
			T::InitiativeThreshold::get().mul_ceil(T::CitizenSource::citizen_count()).max(1)
		}

		fn ensure_prime_minister(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::SerokWeziran) == Some(who),
				Error::<T>::NotThePrimeMinister
			);
			Ok(())
		}

		/// Give every candidate their deposit back.
		///
		/// It was taken to make a candidacy cost something, and once the election is over it
		/// has done its work. Nothing released these before: `reserve` was called at
		/// registration and `unreserve` appeared nowhere in the pallet, so standing for
		/// office locked part of a citizen's balance permanently -- for the winner as much as
		/// for everyone else.
		///
		/// Called on both endings, because a candidate is no more responsible for the
		/// country's turnout than for the result.
		fn return_candidacy_deposits(election_id: u32, election: &ElectionInfo<T>) {
			for candidate in election.candidates.iter() {
				if let Some(info) = ElectionCandidates::<T>::get(election_id, candidate) {
					if info.deposit_paid > 0 {
						let amount: <<T as Config>::NativeCurrency as Currency<
							T::AccountId,
						>>::Balance = info.deposit_paid.saturated_into();
						T::NativeCurrency::unreserve(candidate, amount);
					}
				}
			}
		}

		/// How long a mandate for this office runs.
		fn term_length_of(election_type: &ElectionType) -> BlockNumberFor<T> {
			match election_type {
				ElectionType::ConstitutionalCourt => T::CourtTermLength::get(),
				_ => T::TermLength::get(),
			}
		}

		/// How long an office's *first* term runs.
		///
		/// The same as every later one, except for Parliament, which gets half. That single
		/// shortened term is what staggers the calendar: from then on the house is elected at
		/// the midpoint of a presidency and every one after it, so no single vote hands one
		/// side both the government and the legislature on the same day. It is the same
		/// reasoning that gave the Diwan nine years against the President's four -- an
		/// institution that arrives and leaves with a President has trouble checking one.
		///
		/// Derived from `TermLength` rather than configured separately: whatever the term
		/// becomes, Parliament sits at its midpoint, and the two cannot drift apart by
		/// somebody editing one constant and not the other.
		fn first_term_length_of(election_type: &ElectionType) -> BlockNumberFor<T> {
			match election_type {
				ElectionType::Parliamentary => T::TermLength::get() / 2u32.into(),
				other => Self::term_length_of(other),
			}
		}

		/// How long it takes to run an election from opening to result.
		///
		/// The scheduler subtracts this from the end of a term to decide when to open the
		/// next one, so that the winner is ready on the day the mandate runs out rather than
		/// months afterwards.
		fn election_cycle_length() -> BlockNumberFor<T> {
			T::CandidacyPeriod::get()
				.saturating_add(T::CampaignPeriod::get())
				.saturating_add(T::ElectionPeriod::get())
		}

		/// Record the mandate that starts now, and return when it ends.
		///
		/// Measured from the end of the previous term, not from the moment of counting. A
		/// mandate that started when the votes happened to be finalised would push the next
		/// one back by however late the count was, and the whole calendar would drift a
		/// little further every cycle until elections landed wherever the last delay left
		/// them. Anchoring to the previous term keeps the cycle fixed.
		fn begin_term(election_type: &ElectionType) -> BlockNumberFor<T> {
			let now = pezframe_system::Pezpallet::<T>::block_number();
			let length = Self::term_length_of(election_type);
			let previous = TermEnds::<T>::get(election_type);

			// Anchored to the previous term, and if that one has already run out, to the one
			// after it, and so on until the end lands in the future. Starting from the moment
			// of counting instead would be simpler and would drift: a count that came in
			// late by a month would push every subsequent election back by that month, for
			// good. Stepping whole terms keeps the calendar where it was, however late the
			// arithmetic arrived.
			let mut ends_at = match previous {
				Some(prev) => prev.saturating_add(length),
				// Nobody has held this office by a vote before, so this is where its calendar
				// is set. See `first_term_length_of` for why Parliament's is not a full term.
				None => now.saturating_add(Self::first_term_length_of(election_type)),
			};
			while ends_at <= now {
				ends_at = ends_at.saturating_add(length);
			}
			TermEnds::<T>::insert(election_type, ends_at);
			ends_at
		}

		/// Seat an election winner, with the mandate running until `ends_at`.
		///
		/// The tiki's own expiry is set past the end of the term by one full election cycle.
		/// That gap is deliberate. Expiring exactly at the end of the term would empty the
		/// office the instant the clock ran out, even though the election to replace the
		/// holder may still be counting -- a state with no President for a few weeks because
		/// the arithmetic was tidy. Expiring never would mean a broken election leaves someone
		/// in office indefinitely, which is the failure a term exists to prevent. One cycle of
		/// grace is the smallest window that covers a normal handover and nothing more: if a
		/// successor has not been seated by then, the office empties and the vacancy rules
		/// take over.
		/// Whether `who` may take one of the President's five seats.
		///
		/// The elected six need only citizenship -- the house's choice is a political one and
		/// the vote is its justification. The appointed five are the other half of the
		/// bargain: they are not voted for, so they have to be qualified.
		///
		/// What qualifies somebody is deliberately wider than law. This court rules on
		/// whether a runtime upgrade is constitutional, whether a slash was just, whether an
		/// election counted and whether a citizenship was rightly taken. A bench of lawyers
		/// could not read the first of those. So the pool spans the competences the caseload
		/// actually needs: law, the chain itself, the economy, elections, and the society the
		/// rights belong to.
		pub fn qualifies_for_an_appointed_seat(who: &T::AccountId) -> bool {
			const QUALIFYING: [Tiki; 14] = [
				// Law
				Tiki::Hiquqnas,
				Tiki::Dadger,
				Tiki::Dozger,
				// The chain itself
				Tiki::Bernamenivîs,
				Tiki::PisporêEwlehiyaSîber,
				Tiki::OperatorêTorê,
				// The economy
				Tiki::Aborînas,
				Tiki::Hesabdar,
				Tiki::Plansaz,
				// Elections
				Tiki::Hilbijartinkar,
				Tiki::Statîstîknas,
				Tiki::Piştrastkar,
				// Society
				Tiki::Rewsenbîr,
				Tiki::ParêzvaneÇandî,
			];

			QUALIFYING
				.iter()
				.any(|tiki| pezpallet_tiki::Pezpallet::<T>::has_tiki(who, tiki))
		}

		/// Move the `EndameDiwane` tiki from the outgoing elected members to the incoming.
		///
		/// Only the difference, as with parliamentary seats: a member the house returns keeps
		/// their tiki untouched. Eleven is small enough to do in one block.
		fn hand_over_bench_seats(
			outgoing: &[T::AccountId],
			incoming: &[T::AccountId],
			term_ends_at: BlockNumberFor<T>,
		) -> Result<(), Error<T>> {
			let ends_at = term_ends_at.saturating_add(Self::election_cycle_length());

			for who in outgoing.iter().filter(|who| !incoming.contains(who)) {
				let _ =
					pezpallet_tiki::Pezpallet::<T>::internal_revoke_role(who, Tiki::EndameDiwane);
			}
			for who in incoming {
				Self::seat_on_the_bench(who, ends_at)?;
			}
			Ok(())
		}

		/// Tell the collective who sits on the court.
		///
		/// Called after every change to the bench and nowhere else, so the two can only
		/// disagree for the length of one call. `try_state` holds them to it.
		fn publish_the_bench() {
			T::CourtRoster::set_members(
				DiwanMembers::<T>::get().iter().map(|member| member.account.clone()).collect(),
			);
		}

		/// Give one seat on the court, with its term.
		fn seat_on_the_bench(
			who: &T::AccountId,
			ends_at: BlockNumberFor<T>,
		) -> Result<(), Error<T>> {
			if !pezpallet_tiki::UserTikis::<T>::get(who).contains(&Tiki::EndameDiwane) {
				pezpallet_tiki::Pezpallet::<T>::internal_grant_role_until(
					who,
					Tiki::EndameDiwane,
					ends_at,
				)
				.map_err(|_| Error::<T>::CouldNotSeatOffice)?;
			} else {
				pezpallet_tiki::TikiExpiry::<T>::insert(who, Tiki::EndameDiwane, ends_at);
			}
			pezpallet_tiki::RoleAssignmentTypeOf::<T>::insert(
				who,
				Tiki::EndameDiwane,
				pezpallet_tiki::RoleAssignmentType::Elected,
			);
			Ok(())
		}

		/// Work out which seats change hands, and queue the difference.
		///
		/// Only the difference: a member returned to their seat keeps their tiki untouched,
		/// so the trust score behind it never dips and the NFT is not rewritten for nothing.
		pub(crate) fn queue_seat_handover(
			outgoing: &[T::AccountId],
			incoming: &[T::AccountId],
			term_ends_at: BlockNumberFor<T>,
		) -> Result<(), Error<T>> {
			ensure!(
				PendingSeatRevokes::<T>::get().is_empty()
					&& PendingSeatGrants::<T>::get().is_empty(),
				Error::<T>::SeatingStillInProgress
			);

			let revokes: Vec<T::AccountId> =
				outgoing.iter().filter(|who| !incoming.contains(who)).cloned().collect();
			let grants: Vec<T::AccountId> =
				incoming.iter().filter(|who| !outgoing.contains(who)).cloned().collect();

			let revokes: BoundedVec<_, T::ParliamentSize> =
				revokes.try_into().map_err(|_| Error::<T>::ParliamentFull)?;
			let grants: BoundedVec<_, T::ParliamentSize> =
				grants.try_into().map_err(|_| Error::<T>::ParliamentFull)?;

			// Everyone who kept their seat keeps it under the new term, and that is a
			// bounded rewrite of the expiry alone -- no grant, no metadata, no queue.
			for member in incoming.iter().filter(|who| outgoing.contains(who)) {
				pezpallet_tiki::TikiExpiry::<T>::insert(
					member,
					Tiki::Parlementer,
					Self::seat_expiry(term_ends_at),
				);
			}

			PendingSeatRevokes::<T>::put(revokes);
			PendingSeatGrants::<T>::put(grants);
			PendingSeatTerm::<T>::put(term_ends_at);
			Ok(())
		}

		/// When `who` was seated, if the current house's roll names them.
		///
		/// The roll, not the seat. Whether they still hold it is a question for the tiki --
		/// see the note on `ParliamentRoll` in the rewards pallet for why those are two
		/// different questions with two different answers.
		pub fn seat_taken_at(who: &T::AccountId) -> Option<BlockNumberFor<T>> {
			ParliamentMembers::<T>::get()
				.iter()
				.find(|member| &member.account == who)
				.map(|member| member.elected_at)
		}

		/// When a seat granted for a term ending at `term_ends_at` stops counting.
		///
		/// One election cycle past the end of the term, the same grace every other elected
		/// office gets: the count for the next house has to finish before the last one stops
		/// being Parliament, or there would be a stretch with no legislature at all.
		fn seat_expiry(term_ends_at: BlockNumberFor<T>) -> BlockNumberFor<T> {
			term_ends_at.saturating_add(Self::election_cycle_length())
		}

		/// Apply up to `SEATS_PER_BLOCK` queued seat changes.
		///
		/// Revocations run before grants so that a handover never has more than the house's
		/// size holding the seat at once.
		fn process_seat_handover() -> Weight {
			let mut revokes = PendingSeatRevokes::<T>::get();
			let mut grants = PendingSeatGrants::<T>::get();
			if revokes.is_empty() && grants.is_empty() {
				return T::DbWeight::get().reads(2);
			}

			let term = PendingSeatTerm::<T>::get();
			let mut done = 0u32;

			while done < SEATS_PER_BLOCK && !revokes.is_empty() {
				let who = revokes.remove(0);
				// A seat already gone -- the holder lost their citizenship, or the Diwan
				// removed them -- is not an error, it is the outcome we wanted.
				let _ =
					pezpallet_tiki::Pezpallet::<T>::internal_revoke_role(&who, Tiki::Parlementer);
				done = done.saturating_add(1);
			}

			while done < SEATS_PER_BLOCK && !grants.is_empty() {
				let who = grants.remove(0);
				if Self::seat_parliamentarian(&who, term).is_err() {
					Self::deposit_event(Event::SeatCouldNotBeTaken { who });
				}
				done = done.saturating_add(1);
			}

			let finished = revokes.is_empty() && grants.is_empty();
			PendingSeatRevokes::<T>::put(revokes);
			PendingSeatGrants::<T>::put(grants);
			if finished {
				PendingSeatTerm::<T>::kill();
				Self::deposit_event(Event::SeatHandoverCompleted);
			}

			T::DbWeight::get().reads_writes(3, 3 + (done as u64).saturating_mul(4))
		}

		/// Give one seat, with its term.
		///
		/// Deliberately not `seat_elected_tiki`: that one moves a single-holder office and
		/// begins by reading `TikiHolder`, which the tiki pallet keeps for unique roles only
		/// and whose `try_state` forbids a role with many holders from appearing in. A seat
		/// in a house of two hundred and one is not that kind of office.
		fn seat_parliamentarian(
			who: &T::AccountId,
			term_ends_at: Option<BlockNumberFor<T>>,
		) -> Result<(), Error<T>> {
			let ends_at = term_ends_at.map(Self::seat_expiry);

			if !pezpallet_tiki::UserTikis::<T>::get(who).contains(&Tiki::Parlementer) {
				match ends_at {
					Some(end) => pezpallet_tiki::Pezpallet::<T>::internal_grant_role_until(
						who,
						Tiki::Parlementer,
						end,
					),
					None => {
						pezpallet_tiki::Pezpallet::<T>::internal_grant_role(who, Tiki::Parlementer)
					},
				}
				.map_err(|_| Error::<T>::CouldNotSeatOffice)?;
			} else if let Some(end) = ends_at {
				pezpallet_tiki::TikiExpiry::<T>::insert(who, Tiki::Parlementer, end);
			}

			pezpallet_tiki::RoleAssignmentTypeOf::<T>::insert(
				who,
				Tiki::Parlementer,
				pezpallet_tiki::RoleAssignmentType::Elected,
			);
			Ok(())
		}

		fn seat_elected_tiki(
			who: &T::AccountId,
			tiki: Tiki,
			ends_at: BlockNumberFor<T>,
		) -> Result<(), Error<T>> {
			let grace = ends_at.saturating_add(Self::election_cycle_length());

			// The raw map on purpose, unlike every authority check in this pallet. Seating and
			// vacating have to act on whoever the register physically holds, expired or not:
			// `current_holder` would report a lapsed officeholder as absent and their tiki
			// would be left behind, still in the map, for the next reader to find.
			if let Some(current) = pezpallet_tiki::TikiHolder::<T>::get(tiki) {
				if current != *who {
					pezpallet_tiki::Pezpallet::<T>::internal_revoke_role(&current, tiki)
						.map_err(|_| Error::<T>::CouldNotSeatOffice)?;
				}
			}
			if !pezpallet_tiki::UserTikis::<T>::get(who).contains(&tiki) {
				pezpallet_tiki::Pezpallet::<T>::internal_grant_role(who, tiki)
					.map_err(|_| Error::<T>::CouldNotSeatOffice)?;
			}
			pezpallet_tiki::TikiExpiry::<T>::insert(who, tiki, grace);
			pezpallet_tiki::RoleAssignmentTypeOf::<T>::insert(
				who,
				tiki,
				pezpallet_tiki::RoleAssignmentType::Elected,
			);
			Ok(())
		}

		/// Count this term against the winner, or start their count if the office changed hands.
		fn record_consecutive_term(election_type: &ElectionType, winner: &T::AccountId) {
			let served = ConsecutiveTerms::<T>::get(election_type, winner);
			// Anyone who is not the winner has broken their run, so the map is cleared and
			// only the winner's count survives. Without this a former officeholder's tally
			// would still be standing when they ran again years later.
			let _ = ConsecutiveTerms::<T>::clear_prefix(election_type, u32::MAX, None);
			ConsecutiveTerms::<T>::insert(election_type, winner, served.saturating_add(1));
		}

		/// Move a single-holder office to `to`, removing whoever held it, in one step.
		///
		/// `internal_grant_role` refuses a unique tiki that someone else already holds, so a
		/// handover is necessarily remove-then-grant. Doing both inside one call is what makes
		/// it a handover rather than two events that might not both happen: a ballot that
		/// unseated the incumbent and then failed to seat the winner would leave the office
		/// empty and everything gated on it dead, with nothing to say why.
		/// The office an appointed role *is*. They are the same office under two names, and
		/// the register keeps only one of them.
		pub fn tiki_for_role(role: &OfficialRole) -> Tiki {
			match role {
				OfficialRole::Dadger => Tiki::Dadger,
				OfficialRole::Dozger => Tiki::Dozger,
				OfficialRole::Hiquqnas => Tiki::Hiquqnas,
				OfficialRole::Noter => Tiki::Noter,
				OfficialRole::Xezinedar => Tiki::Xezinedar,
				OfficialRole::Bacgir => Tiki::Bacgir,
				OfficialRole::GerinendeyeCavkaniye => Tiki::GerinendeyeCavkaniye,
				OfficialRole::OperatorêTorê => Tiki::OperatorêTorê,
				OfficialRole::PisporêEwlehiyaSîber => Tiki::PisporêEwlehiyaSîber,
				OfficialRole::GerinendeyeDaneye => Tiki::GerinendeyeDaneye,
				OfficialRole::Berdevk => Tiki::Berdevk,
				OfficialRole::Qeydkar => Tiki::Qeydkar,
				OfficialRole::Balyoz => Tiki::Balyoz,
				OfficialRole::Navbeynkar => Tiki::Navbeynkar,
				OfficialRole::ParêzvaneÇandî => Tiki::ParêzvaneÇandî,
				OfficialRole::Mufetîs => Tiki::Mufetîs,
				OfficialRole::KalîteKontrolker => Tiki::KalîteKontrolker,
				OfficialRole::Bazargan => Tiki::Bazargan,
				OfficialRole::RêveberêProjeyê => Tiki::RêveberêProjeyê,
				OfficialRole::Feqî => Tiki::Feqî,
				OfficialRole::Perwerdekar => Tiki::Perwerdekar,
				OfficialRole::Rewsenbîr => Tiki::Rewsenbîr,
				OfficialRole::Mamoste => Tiki::Mamoste,
				OfficialRole::Mela => Tiki::Mela,
			}
		}

		/// Whether a tiki is one of the appointed civil-service offices, as opposed to an
		/// elected seat or a badge somebody earns.
		pub fn is_appointed_office(tiki: &Tiki) -> bool {
			OfficialRole::iter_all().any(|r| Self::tiki_for_role(&r) == *tiki)
		}

		pub fn seat_unique_tiki(to: &T::AccountId, tiki: Tiki) -> DispatchResult {
			// The raw map on purpose, unlike every authority check in this pallet. Seating and
			// vacating have to act on whoever the register physically holds, expired or not:
			// `current_holder` would report a lapsed officeholder as absent and their tiki
			// would be left behind, still in the map, for the next reader to find.
			if let Some(current) = pezpallet_tiki::TikiHolder::<T>::get(tiki) {
				if current == *to {
					return Ok(());
				}
				pezpallet_tiki::Pezpallet::<T>::internal_revoke_role(&current, tiki)?;
			}
			pezpallet_tiki::Pezpallet::<T>::internal_grant_role(to, tiki)
		}

		/// Remove a single-holder office from whoever holds it. Empty is not an error.
		pub fn vacate_unique_tiki(tiki: Tiki) -> DispatchResult {
			// The raw map on purpose, unlike every authority check in this pallet. Seating and
			// vacating have to act on whoever the register physically holds, expired or not:
			// `current_holder` would report a lapsed officeholder as absent and their tiki
			// would be left behind, still in the map, for the next reader to find.
			match pezpallet_tiki::TikiHolder::<T>::get(tiki) {
				Some(current) => {
					pezpallet_tiki::Pezpallet::<T>::internal_revoke_role(&current, tiki)
				},
				None => Ok(()),
			}
		}

		/// Whether `tiki` is a cabinet post that `SerokWeziran` may fill.
		///
		/// The list itself belongs to `tiki`: that pallet is the register these offices are
		/// written into, and it reads the same list to refuse its own admin call for them.
		/// Two copies would be free to drift, and the drift would show up as an office the
		/// Prime Minister may fill but the register will not accept -- or worse, the reverse.
		pub fn is_cabinet_tiki(tiki: &Tiki) -> bool {
			pezpallet_tiki::Pezpallet::<T>::is_cabinet_tiki(tiki)
		}

		/// Ask the treasury chain to pay `amount` to `beneficiary` from the government pot.
		///
		/// Sent unpaid for the same reason as the population report: this is a sibling system
		/// chain, and a payment Parliament has approved should not fail because a sovereign
		/// account was short of fees.
		/// Tell the airdrop pot's chain to pay.
		///
		/// `pezpallet_treasury::spend` on the pot's instance. Four arguments and each one is
		/// load-bearing in a way the compiler cannot see from here:
		///
		/// - `asset_kind` is `()` -- the pot holds the native token and nothing else.
		/// - `amount` is **compact**, unlike `spend_from_government_pot` next door. That is
		///   not a style difference: the treasury's `spend` declares `#[pezpallet::compact]`,
		///   and sending it at full width decodes as a different value on the other side.
		/// - `beneficiary` is a lookup, and this chain's runtime uses `IdentityLookup`, so the
		///   account id goes on the wire as itself.
		/// - `valid_from` is `None`: the wait already happened here, and asking the other
		///   chain to wait again would put the same delay in two places where only one of them
		///   is the one anybody reads.
		///
		/// `OriginKind::Xcm` so the pot sees this chain speaking as itself, which is what its
		/// `EnsureXcm<Equals<PeopleLocation>>` accepts.
		fn send_airdrop_spend(beneficiary: &T::AccountId, amount: u128) -> Result<(), SendError> {
			let call = (
				T::AirdropPotPalletIndex::get(),
				TREASURY_SPEND_CALL_INDEX,
				(),
				codec::Compact(amount),
				beneficiary,
				Option::<BlockNumberFor<T>>::None,
			)
				.encode();

			let message = Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: call.into(),
				},
			]);

			let (ticket, _) = T::XcmSender::validate(
				&mut Some(T::TreasuryChainLocation::get()),
				&mut Some(message),
			)?;
			T::XcmSender::deliver(ticket).map(|_| ())
		}

		fn send_government_spend(
			beneficiary: &T::AccountId,
			amount: u128,
		) -> Result<(), SendError> {
			// `spend_from_government_pot` takes a plain `Balance`, not a compact one, so the
			// amount goes on the wire at full width. A compact here decodes as a different
			// call on the other side -- and nothing reports it, because the budget below is
			// already docked by the time the message is refused.
			let call = (
				T::TreasuryPalletIndex::get(),
				SPEND_FROM_GOVERNMENT_POT_CALL_INDEX,
				beneficiary,
				amount,
			)
				.encode();

			let message = Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: call.into(),
				},
			]);

			let (ticket, _) = T::XcmSender::validate(
				&mut Some(T::TreasuryChainLocation::get()),
				&mut Some(message),
			)?;
			T::XcmSender::deliver(ticket)?;
			Ok(())
		}

		/// Put the new emission rate on the wire.
		///
		/// `RuntimeParameters::Hez(hez::Parameters::InflationRate(key, Some(rate)))`, addressed
		/// by index because this chain cannot name the other chain's types. The key is a unit
		/// struct and contributes no bytes. `the_emission_call_encodes_the_way_welati_builds_it`
		/// on the treasury chain pins this, for the same reason the treasury call has a pin:
		/// nothing here fails if the encoding drifts -- the `Transact` simply does not decode,
		/// while this chain has already recorded the change.
		fn send_emission_rate(rate: pezsp_runtime::Perbill) -> Result<(), SendError> {
			let call = (
				T::ParametersPalletIndex::get(),
				SET_PARAMETER_CALL_INDEX,
				HEZ_PARAMETERS_VARIANT,
				INFLATION_RATE_VARIANT,
				Some(rate),
			)
				.encode();

			let message = Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: call.into(),
				},
			]);

			let (ticket, _) = T::XcmSender::validate(
				&mut Some(T::TreasuryChainLocation::get()),
				&mut Some(message),
			)?;
			T::XcmSender::deliver(ticket)?;
			Ok(())
		}

		/// Tell the treasury chain that the population threshold has been reached.
		///
		/// The message is an unpaid `Transact`: this chain is a sibling system chain, so the
		/// Asset Hub's barrier lets its instructions through without buying execution. Sending
		/// it as a paid message instead would mean the state's own report could be lost
		/// because a sovereign account somewhere ran short.
		fn report_population_threshold_reached() -> Result<(), SendError> {
			let call = (T::TreasuryPalletIndex::get(), ACTIVATE_DISTRIBUTION_CALL_INDEX).encode();

			let message = Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: call.into(),
				},
			]);

			let (ticket, _) = T::XcmSender::validate(
				&mut Some(T::TreasuryChainLocation::get()),
				&mut Some(message),
			)?;
			T::XcmSender::deliver(ticket)?;
			Ok(())
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
					pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok)
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
		Ok(pezframe_system::RawOrigin::Signed(serok_account).into())
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
		pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok)
			.map(|serok| &serok == who)
			.unwrap_or(false)
	}

	/// Check if account is a Parliament member
	/// Is there a seat for this person on the President's side of the bench?
	fn a_court_seat_is_open_for(who: &T::AccountId) -> DispatchResult {
		let bench = DiwanMembers::<T>::get();
		ensure!(!bench.iter().any(|member| &member.account == who), Error::<T>::AlreadyOnTheCourt);

		let appointed = bench
			.iter()
			.filter(|member| matches!(member.appointed_by, AppointmentAuthority::President(_)))
			.count() as u32;
		ensure!(
			appointed < T::DiwanSize::get().saturating_sub(T::DiwanElectedSeats::get()),
			Error::<T>::AppointedCourtSeatsAreFull
		);

		ensure!(Self::qualifies_for_an_appointed_seat(who), Error::<T>::NotQualifiedForTheCourt);
		Ok(())
	}

	/// Does this office need the confirming body's consent?
	///
	/// The legislature's answer where it has given one, the founding rule otherwise.
	pub fn confirmation_is_required(role: &OfficialRole) -> bool {
		ConfirmationRequired::<T>::get(role).unwrap_or_else(|| role.requires_parliament_approval())
	}

	pub fn is_parliament_member(who: &T::AccountId) -> bool {
		ParliamentMembers::<T>::get().iter().any(|member| &member.account == who)
	}

	/// Check if account is a Diwan member
	pub fn is_diwan_member(who: &T::AccountId) -> bool {
		DiwanMembers::<T>::get().iter().any(|member| &member.account == who)
	}

	/// Check if account is a Minister.
	///
	/// Reads the tiki, which is the only record of who holds a portfolio. There used to be a
	/// separate map here as well; nothing ever wrote to it, so this answered false for
	/// everyone and quietly disabled every authority check built on it. The map is gone --
	/// do not reintroduce one. An office recorded in two places is an office whose holder
	/// depends on who is asking.
	pub fn is_minister(who: &T::AccountId) -> bool {
		pezpallet_tiki::UserTikis::<T>::get(who)
			.iter()
			.any(Pezpallet::<T>::is_cabinet_tiki)
	}

	/// Check if account is an Official (non-minister appointed position)
	pub fn is_official(who: &T::AccountId) -> bool {
		pezpallet_tiki::UserTikis::<T>::get(who)
			.iter()
			.any(|t| Self::is_appointed_office(t))
	}
}
