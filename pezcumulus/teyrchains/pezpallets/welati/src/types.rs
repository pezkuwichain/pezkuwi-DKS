// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::pezpallet_prelude::*;
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezpallet_tiki::Tiki;
use pezsp_std::prelude::*;
use scale_info::TypeInfo;

#[derive(Debug, Eq, PartialEq)]
pub enum ElectionOutcome<AccountId> {
	/// Winners have been determined.
	Winners(BoundedVec<AccountId, ConstU32<201>>),
	/// A runoff is required; these are the candidates.
	RunoffRequired(BoundedVec<AccountId, ConstU32<2>>),
}

/// Government positions (elected offices)
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum GovernmentPosition {
	/// President (Serok)
	#[codec(index = 0)]
	Serok,
	/// Member of Parliament (Parlementer)
	#[codec(index = 1)]
	Parlementer,
	/// Speaker of Parliament (SerokiMeclise)
	#[codec(index = 2)]
	SerokiMeclise,
	/// Diwan Member (EndameDiwane)
	#[codec(index = 3)]
	EndameDiwane,
}

/// Civil servant roles (appointed positions)
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum OfficialRole {
	// Under the Ministry of Justice
	#[codec(index = 0)]
	Dadger,
	#[codec(index = 1)]
	Dozger,
	#[codec(index = 2)]
	Hiquqnas,
	#[codec(index = 3)]
	Noter,

	// Under the Ministry of the Treasury
	#[codec(index = 4)]
	Xezinedar,
	#[codec(index = 5)]
	Bacgir,
	#[codec(index = 6)]
	GerinendeyeCavkaniye,

	// Under the Ministry of Technology and Infrastructure
	#[codec(index = 7)]
	OperatorêTorê,
	#[codec(index = 8)]
	PisporêEwlehiyaSîber,
	#[codec(index = 9)]
	GerinendeyeDaneye,

	// Under the Ministry of Internal Affairs and Communications
	#[codec(index = 10)]
	Berdevk,
	#[codec(index = 11)]
	Qeydkar,

	// Under the Ministry of Foreign Affairs
	#[codec(index = 12)]
	Balyoz,
	#[codec(index = 13)]
	Navbeynkar,
	#[codec(index = 14)]
	ParêzvaneÇandî,

	// Under the Ministry of Audit
	#[codec(index = 15)]
	Mufetîs,
	#[codec(index = 16)]
	KalîteKontrolker,

	// Under the Ministry of Economy and Trade
	#[codec(index = 17)]
	Bazargan,
	#[codec(index = 18)]
	RêveberêProjeyê,

	// Under the Ministry of National Education and Religious Affairs
	#[codec(index = 19)]
	Feqî,
	#[codec(index = 20)]
	Perwerdekar,
	#[codec(index = 21)]
	Rewsenbîr,
	#[codec(index = 22)]
	Mamoste,

	// Exceptional appointment (directly by Serok)
	#[codec(index = 23)]
	Mela,
}

/// Election types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ElectionType {
	/// Presidential election (special rules)
	#[codec(index = 0)]
	Presidential,
	/// Parliamentary election (201 members)
	#[codec(index = 1)]
	Parliamentary,
	/// Speaker election (among members of parliament)
	#[codec(index = 2)]
	SpeakerElection,
	/// Diwan member election
	#[codec(index = 3)]
	ConstitutionalCourt,
}

/// Vote types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VoteType {
	/// Normal citizen vote
	Citizen,
	/// Weighted vote (based on Trust Score)
	Weighted,
	/// Delegated vote
	Delegated,
}

/// Structure holding nomination information
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct NominationInfo<T: pezframe_system::Config> {
	/// The nominator (Minister)
	pub nominator: T::AccountId,
	/// The nominated person
	pub nominee: T::AccountId,
	/// The block at which the nomination was made
	pub nominated_at: BlockNumberFor<T>,
	/// Whether it has been approved
	pub approved: bool,
	/// The approver (usually Serok)
	pub approver: Option<T::AccountId>,
	/// Approval date
	pub approved_at: Option<BlockNumberFor<T>>,
	/// Nomination status
	pub status: NominationStatus,
}

/// Nomination statuses
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum NominationStatus {
	/// Pending nomination
	Pending,
	/// Approved
	Approved,
	/// Rejected
	Rejected,
	/// Cancelled
	Cancelled,
	/// Expired
	Expired,
}

/// Collective decision types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum CollectiveDecisionType {
	/// Parliament decision (simple majority - 50%+1)
	ParliamentSimpleMajority,
	/// Parliament supermajority decision (2/3)
	ParliamentSuperMajority,
	/// Parliament absolute majority (3/4 - constitutional amendment)
	ParliamentAbsoluteMajority,
	/// Hybrid decision (Parliament + Serok approval)
	HybridDecision,
	/// President's sole decision
	ExecutiveDecision,
	/// Veto override (Parliament overriding a veto with 2/3)
	VetoOverride,
}

/// Status of a collective vote
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ProposalStatus {
	/// In draft (not yet submitted to a vote)
	Draft,
	/// Active vote
	Active,
	/// Accepted
	Approved,
	/// Rejected
	Rejected,
	/// Cancelled
	Cancelled,
	/// Timed out
	Expired,
	/// Vetoed (by Serok)
	Vetoed,
	/// Under constitutional review (at the Diwan)
	UnderConstitutionalReview,
}

/// Collective proposal information
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CollectiveProposal<T: pezframe_system::Config> {
	/// Proposal ID
	pub proposal_id: u32,
	/// Proposal owner
	pub proposer: T::AccountId,
	/// Proposal title
	pub title: BoundedVec<u8, ConstU32<100>>,
	/// Proposal description
	pub description: BoundedVec<u8, ConstU32<1000>>,
	/// Proposal date
	pub proposed_at: BlockNumberFor<T>,
	/// Voting start date
	pub voting_starts_at: BlockNumberFor<T>,
	/// End date
	pub expires_at: BlockNumberFor<T>,
	/// Decision type
	pub decision_type: CollectiveDecisionType,
	/// Current status
	pub status: ProposalStatus,
	/// Aye votes
	pub aye_votes: u32,
	/// Nay votes
	pub nay_votes: u32,
	/// Abstain votes
	pub abstain_votes: u32,
	/// Required minimum number of votes
	pub threshold: u32,
	/// Number of members who voted
	pub votes_cast: u32,
	/// Priority level
	pub priority: ProposalPriority,
	/// The spending allowance this proposal asks Parliament to grant, if it is a budget.
	///
	/// This used to be `Option<Box<RuntimeCall>>` carrying an arbitrary call -- and it was
	/// marked `#[codec(skip)]`, because a `RuntimeCall` has no bounded size and this struct
	/// derives `MaxEncodedLen`. Skipped means the call was dropped on the way into storage and
	/// read back as `None`, every time. So the proposal system appeared to be able to enact
	/// anything and could in fact enact nothing, and no amount of writing a tally would have
	/// changed that.
	///
	/// A number is storable. Carrying arbitrary calls needs the call to be bounded -- stored
	/// by hash with the body in a preimage, the way `pezpallet-democracy` does it -- and that
	/// is a design of its own, not a field.
	pub budget_amount: Option<u128>,
}

/// Proposal priority levels
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ProposalPriority {
	/// Low priority
	Low,
	/// Normal priority
	Normal,
	/// High priority
	High,
	/// Urgent (within 24 hours)
	Urgent,
	/// Critical (immediate)
	Critical,
}

/// Collective vote information
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CollectiveVote<T: pezframe_system::Config> {
	/// Voter
	pub voter: T::AccountId,
	/// Proposal ID
	pub proposal_id: u32,
	/// Vote type
	pub vote: VoteChoice,
	/// Time the vote was cast
	pub voted_at: BlockNumberFor<T>,
	/// Vote rationale (optional)
	pub rationale: Option<BoundedVec<u8, ConstU32<500>>>,
}

/// Vote options
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VoteChoice {
	/// Yes
	Aye,
	/// No
	Nay,
	/// Abstain
	Abstain,
}

/// Parliament member information
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	TypeInfo,
	MaxEncodedLen,
	Default,
	Debug,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ParliamentMember<T: pezframe_system::Config> {
	/// Member account
	pub account: T::AccountId,
	/// Election date
	pub elected_at: BlockNumberFor<T>,
	/// Term end date
	pub term_ends_at: BlockNumberFor<T>,
	/// Number of votes participated in
	pub votes_participated: u32,
	/// Total number of eligible votes
	pub total_votes_eligible: u32,
	/// Participation rate (percentage)
	pub participation_rate: u8,
	/// Special committees
	pub committees: BoundedVec<CommitteeType, ConstU32<5>>,
}

/// Committee types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum CommitteeType {
	/// Budget Committee
	Budget,
	/// Foreign Affairs Committee
	ForeignAffairs,
	/// Justice Committee
	Justice,
	/// Technology Committee
	Technology,
	/// Education Committee
	Education,
	/// Health Committee
	Health,
	/// Constitutional Committee
	Constitutional,
}

/// Diwan member information
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct DiwanMember<T: pezframe_system::Config> {
	/// Member account
	pub account: T::AccountId,
	/// Appointment date
	pub appointed_at: BlockNumberFor<T>,
	/// Term length (9 years)
	pub term_ends_at: BlockNumberFor<T>,
	/// Appointing authority (Parliament/Serok)
	pub appointed_by: AppointmentAuthority<T>,
}

/// Appointment authority
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
/// How a member reached the bench, and the two halves of it.
///
/// The court is eleven: six the house elects, five the President appoints. The two halves
/// answer different needs and carry different conditions. The elected six need no
/// qualification beyond citizenship -- their legitimacy is the vote. The appointed five must
/// each hold one of the qualifying tikis, because a court that rules on whether an upgrade
/// is constitutional, whether a slash was just, or whether an election counted, has to
/// contain people who can read those things.
pub enum AppointmentAuthority<T: pezframe_system::Config> {
	/// Elected by the sitting Parliament. Six seats.
	Parliament,
	/// Appointed by the President, who is named here. Five seats.
	President(T::AccountId),
}

/// Appointment process information
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct AppointmentProcess<T: pezframe_system::Config> {
	/// Process ID
	pub process_id: u32,
	/// Position to be filled by appointment
	pub position: OfficialRole,
	/// Relevant minister (the nominator)
	pub nominating_minister: T::AccountId,
	/// Candidate
	pub nominee: T::AccountId,
	/// Initiation date
	pub initiated_at: BlockNumberFor<T>,
	/// Final decision deadline
	pub deadline: BlockNumberFor<T>,
	/// Current status
	pub status: AppointmentStatus,
	/// Supporting documents/justification
	pub documents: BoundedVec<BoundedVec<u8, ConstU32<1000>>, ConstU32<10>>,
}

/// Appointment process statuses
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum AppointmentStatus {
	/// Waiting for minister nomination
	WaitingNomination,
	/// Waiting for Serok approval
	WaitingPresidentialApproval,
	/// Waiting for parliamentary approval (for some positions)
	WaitingParliamentaryApproval,
	/// Approved
	Approved,
	/// Rejected
	Rejected,
	/// Expired
	Expired,
}

/// Governance metrics
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct GovernanceMetrics<T: pezframe_system::Config> {
	/// Total number of active proposals
	pub active_proposals: u32,
	/// Number of laws passed this term
	pub laws_passed_this_term: u32,
	/// Parliament attendance rate
	pub parliament_attendance_rate: u8,
	/// Number of Diwan decisions
	pub constitutional_decisions: u32,
	/// Average decision time (in blocks)
	pub average_decision_time: BlockNumberFor<T>,
	/// Number of vetoed laws
	pub vetoed_laws: u32,
	/// Number of vetoes overridden
	pub veto_overrides: u32,
}

/// Election statuses
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ElectionStatus {
	/// Candidacy registration period
	CandidacyPeriod,
	/// Campaign period
	CampaignPeriod,
	/// Voting period
	VotingPeriod,
	/// Completed
	Completed,
	/// Cancelled
	Cancelled,
	/// Closed without a result because too little of the country voted.
	///
	/// Distinct from `Cancelled`, which is somebody calling an election off, and from
	/// `Completed`, which produced officeholders. This one is an ending too: the record is
	/// final, the deposits are back, and the scheduler opens another.
	FailedForTurnout,
}

/// Candidate information
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CandidateInfo<T: pezframe_system::Config> {
	pub account: T::AccountId,
	pub district_id: Option<u32>,
	pub registered_at: BlockNumberFor<T>,
	pub endorsers: BoundedVec<T::AccountId, ConstU32<100>>,
	pub vote_count: u32,
	pub deposit_paid: u128,
	pub campaign_data: BoundedVec<u8, ConstU32<500>>,
}

/// Election results
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ElectionResult<T: pezframe_system::Config> {
	pub election_id: u32,
	pub winners: BoundedVec<T::AccountId, ConstU32<201>>, // Max 201 for Parliament
	pub total_votes: u32,
	pub turnout_percentage: u8,
	pub finalized_at: BlockNumberFor<T>,
}

/// Electoral district information
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
#[codec(mel_bound())]
pub struct ElectoralDistrict {
	pub district_id: u32,
	pub name: BoundedVec<u8, ConstU32<50>>,
	pub seat_count: u32,
	pub voter_population: u32,
	pub geographic_bounds: Option<BoundedVec<u8, ConstU32<200>>>,
}

/// Structure holding election information - Extended version
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ElectionInfo<T: pezframe_system::Config> {
	/// Election ID
	pub election_id: u32,
	/// Election type
	pub election_type: ElectionType,
	/// Start block of the election
	pub start_block: BlockNumberFor<T>,
	/// Candidacy registration deadline
	pub candidacy_deadline: BlockNumberFor<T>,
	/// Campaign start
	pub campaign_start: BlockNumberFor<T>,
	/// Voting start
	pub voting_start: BlockNumberFor<T>,
	/// End block of the election
	pub end_block: BlockNumberFor<T>,
	/// List of candidates
	pub candidates: BoundedVec<T::AccountId, ConstU32<500>>, // Generous limit
	/// Total number of votes
	pub total_votes: u32,
	/// Election status
	pub status: ElectionStatus,
	/// Electoral districts
	pub districts: BoundedVec<ElectoralDistrict, ConstU32<50>>,
	/// Minimum turnout rate (as a percentage)
	pub minimum_turnout: u8,
}

/// Structure holding vote information
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ElectionVoteInfo<T: pezframe_system::Config> {
	/// The voter
	pub voter: T::AccountId,
	/// Candidates voted for (for multiple votes)
	pub candidates: BoundedVec<T::AccountId, ConstU32<10>>,
	/// The block at which the vote was cast
	pub vote_block: BlockNumberFor<T>,
	/// Weight of the vote (may be based on Trust Score)
	pub vote_weight: u32,
	/// Vote type (secret/open)
	pub vote_type: VoteType,
	/// Electoral district
	pub district_id: Option<u32>,
}

/// Election security measures
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum SecurityMeasure {
	/// Duplicate vote detection
	DuplicateVoteDetection,
	/// Identity verification
	IdentityVerification,
	/// Vote privacy
	VotePrivacy,
	/// Manipulation prevention
	ManipulationPrevention,
}

/// Vote privacy level
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VotePrivacyLevel {
	/// Fully open
	FullyOpen,
	/// Partially private (only the result is visible)
	PartiallyPrivate,
	/// Fully private
	FullyPrivate,
}

/// Duplicate vote prevention method
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum DuplicateVoteMethod {
	/// Account-based check
	AccountBased,
	/// Identity-based check
	IdentityBased,
	/// Multi-layered check
	MultiLayered,
}

/// Transparency level
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum TransparencyLevel {
	/// Minimal transparency
	Minimal,
	/// Standard transparency
	Standard,
	/// Maximum transparency
	Maximum,
}

/// Audit requirements
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct AuditRequirements {
	/// Is internal audit required?
	pub internal_audit_required: bool,
	/// Is external audit required?
	pub external_audit_required: bool,
	/// Real-time monitoring
	pub real_time_monitoring: bool,
	/// Is an audit report required?
	pub audit_report_required: bool,
}

/// Vote weighting system
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VoteWeightMethod {
	/// Equal weight
	Equal,
	/// Based on Trust Score
	TrustScoreBased,
	/// Position-based
	PositionBased,
}

/// Voter authentication method
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VoterAuthMethod {
	/// KYC-based
	KycBased,
	/// Biometric
	Biometric,
	/// Multi-factor
	MultiFactor,
}

/// Campaign regulations
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CampaignRegulations<T: pezframe_system::Config> {
	/// Campaign duration (number of blocks)
	pub duration_blocks: BlockNumberFor<T>,
	/// Maximum spending limit
	pub spending_limit: Option<u128>,
	/// Allowed activity types
	pub allowed_activities: BoundedVec<CampaignActivityType, ConstU32<20>>,
	/// Prohibited activity types
	pub prohibited_activities: BoundedVec<CampaignActivityType, ConstU32<20>>,
}

/// Campaign activity types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum CampaignActivityType {
	/// Public rally
	PublicRally,
	/// Media advertisement
	MediaAdvertisement,
	/// Door-to-door canvassing
	DoorToDoorCanvassing,
	/// Digital campaign
	DigitalCampaign,
	/// Fundraising event
	FundraisingEvent,
}

/// Candidacy rules
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
pub struct CandidacyRules {
	/// Minimum age requirement
	pub minimum_age: Option<u32>,
	/// Education requirements
	pub education_requirements: Option<EducationLevel>,
	/// Prior experience requirements
	pub experience_requirements: Option<BoundedVec<u8, ConstU32<500>>>,
	/// Disqualifying background conditions
	pub disqualifying_conditions: BoundedVec<DisqualifyingCondition, ConstU32<10>>,
}

/// Education level
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum EducationLevel {
	/// Elementary school
	Elementary,
	/// Middle school
	MiddleSchool,
	/// High school
	HighSchool,
	/// University
	University,
	/// Master's degree
	MastersDegree,
	/// Doctorate
	Doctorate,
}

/// Disqualifying conditions
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum DisqualifyingCondition {
	/// Criminal record
	CriminalRecord,
	/// Financial misconduct
	FinancialMisconduct,
	/// Ethics violation
	EthicsViolation,
	/// Dual citizenship
	DualCitizenship,
	/// Mental incapacity
	MentalIncapacity,
}

/// Parliamentary committee membership details
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct CommitteeMembership<T: pezframe_system::Config> {
	pub committee: CommitteeType,
	pub role: CommitteeRole,
	pub joined_at: BlockNumberFor<T>,
	pub term_ends_at: Option<BlockNumberFor<T>>,
}

/// Role within the committee
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum CommitteeRole {
	/// Chairman
	Chairman,
	/// Vice chairman
	ViceChairman,
	/// Member
	Member,
	/// Rapporteur
	Rapporteur,
}

/// Legislative process stages
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum LegislativeStage {
	/// Draft stage
	Draft,
	/// Committee review
	CommitteeReview,
	/// First reading in the general assembly
	FirstReading,
	/// Returned to committee
	CommitteeRevision,
	/// Second reading in the general assembly
	SecondReading,
	/// Third reading
	ThirdReading,
	/// Sent to the President
	SentToPresident,
	/// Approved
	Approved,
	/// Vetoed
	Vetoed,
	/// Enacted into law
	Enacted,
}

/// Law types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum LawType {
	/// Constitutional amendment
	ConstitutionalAmendment,
	/// Organic law
	OrganicLaw,
	/// Ordinary law
	OrdinaryLaw,
	/// Budget law
	BudgetLaw,
	/// Ratification of an international agreement
	InternationalAgreement,
}

/// Constitutional review types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ConstitutionalReviewType {
	/// Preliminary review (before a law is enacted)
	PreliminaryReview,
	/// Subsequent review (after a law is enacted)
	SubsequentReview,
	/// Individual application
	IndividualApplication,
	/// Abstract norm control
	AbstractNormControl,
}

/// Veto types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum VetoType {
	/// Absolute veto
	AbsoluteVeto,
	/// Line-item veto
	LineItemVeto,
	/// Suspensive veto
	SuspensiveVeto,
}

/// Parliament session types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum SessionType {
	/// Regular session
	RegularSession,
	/// Extraordinary session
	ExtraordinarySession,
	/// Closed session
	ClosedSession,
	/// Emergency session
	EmergencySession,
}

/// Session status
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum SessionStatus {
	/// Scheduled
	Scheduled,
	/// Active
	Active,
	/// Postponed
	Postponed,
	/// Completed
	Completed,
	/// Cancelled
	Cancelled,
}

/// Parliament session information
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub struct ParliamentSession<T: pezframe_system::Config> {
	pub session_id: u32,
	pub session_type: SessionType,
	pub scheduled_start: BlockNumberFor<T>,
	pub actual_start: Option<BlockNumberFor<T>>,
	pub end_time: Option<BlockNumberFor<T>>,
	pub status: SessionStatus,
	pub agenda: BoundedVec<u32, ConstU32<50>>, // Proposal IDs
	pub attendees: BoundedVec<T::AccountId, ConstU32<201>>,
	pub decisions_made: BoundedVec<u32, ConstU32<20>>, // IDs of decisions made
}

/// State budget categories
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum BudgetCategory {
	/// Personnel expenses
	Personnel,
	/// Goods and services procurement
	GoodsAndServices,
	/// Capital expenditures
	CapitalExpenditures,
	/// Transfer payments
	TransferPayments,
	/// Debt service payments
	DebtService,
	/// Contingency appropriations
	Contingency,
}

/// Budget approval status
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum BudgetStatus {
	/// Draft
	Draft,
	/// In Parliament
	InParliament,
	/// Approved
	Approved,
	/// In execution
	InExecution,
	/// Completed
	Completed,
}

/// Helper structures for the helper traits
pub trait GovernmentPositionInfo {
	fn required_trust_score(&self) -> u128;
	fn required_tiki(&self) -> Option<Tiki>;
	fn term_length_blocks(&self) -> u32;
}

impl GovernmentPositionInfo for GovernmentPosition {
	fn required_trust_score(&self) -> u128 {
		match self {
			GovernmentPosition::Serok => 600,
			GovernmentPosition::Parlementer => 300,
			GovernmentPosition::SerokiMeclise => 400,
			GovernmentPosition::EndameDiwane => 750,
		}
	}

	fn required_tiki(&self) -> Option<Tiki> {
		match self {
			GovernmentPosition::Serok => Some(Tiki::Welati),
			GovernmentPosition::Parlementer => Some(Tiki::Welati),
			GovernmentPosition::SerokiMeclise => Some(Tiki::Parlementer),
			GovernmentPosition::EndameDiwane => Some(Tiki::Welati),
		}
	}

	fn term_length_blocks(&self) -> u32 {
		match self {
			GovernmentPosition::Serok => 4 * 365 * 24 * 60 * 10, // 4 years
			GovernmentPosition::Parlementer => 4 * 365 * 24 * 60 * 10, // 4 years
			GovernmentPosition::SerokiMeclise => 2 * 365 * 24 * 60 * 10, // 2 years
			GovernmentPosition::EndameDiwane => 9 * 365 * 24 * 60 * 10, // 9 years
		}
	}
}

pub trait OfficialRoleInfo {
	fn required_trust_score(&self) -> u128;
	fn requires_parliament_approval(&self) -> bool;
}

impl OfficialRoleInfo for OfficialRole {
	fn required_trust_score(&self) -> u128 {
		75 // General requirement specified in the constitution
	}

	fn requires_parliament_approval(&self) -> bool {
		match self {
			// High-level positions require parliamentary approval
			OfficialRole::Dadger
			| OfficialRole::Xezinedar
			| OfficialRole::PisporêEwlehiyaSîber
			| OfficialRole::Mufetîs
			| OfficialRole::Balyoz => true,
			// The others only require Serok approval
			_ => false,
		}
	}
}

/// A referendum tally where every citizen counts once.
///
/// The state's own rule: in state matters one person is one vote, and stake does not weigh
/// (see the note on `cast_vote`). `pezpallet_conviction_voting`'s tally cannot express that --
/// it reads `Currency` and multiplies by conviction -- so state referenda carry this one and
/// economic referenda, where the holder bears the consequence in proportion, carry that one.
///
/// `Electorate` is the roll a proposal must carry, not the number who turned out: `support`
/// is measured against every citizen, so a proposal nobody voted on has no support rather
/// than unanimous support. `approval` is measured against those who did vote.
#[derive(
	pezframe_support::CloneNoBound,
	pezframe_support::PartialEqNoBound,
	pezframe_support::EqNoBound,
	pezframe_support::DebugNoBound,
	TypeInfo,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(Electorate))]
#[codec(mel_bound())]
pub struct CitizenTally<Electorate: 'static> {
	/// Citizens who voted aye.
	pub ayes: u32,
	/// Citizens who voted nay.
	pub nays: u32,
	#[codec(skip)]
	dummy: core::marker::PhantomData<Electorate>,
}

impl<Electorate: Get<u32> + 'static, Class> pezframe_support::traits::VoteTally<u32, Class>
	for CitizenTally<Electorate>
{
	fn new(_: Class) -> Self {
		Self { ayes: 0, nays: 0, dummy: core::marker::PhantomData }
	}

	fn ayes(&self, _: Class) -> u32 {
		self.ayes
	}

	fn support(&self, _: Class) -> pezsp_runtime::Perbill {
		// Against the whole roll. An empty roll is not unanimous consent, it is no consent.
		let electorate = Electorate::get();
		if electorate == 0 {
			pezsp_runtime::Perbill::zero()
		} else {
			pezsp_runtime::Perbill::from_rational(self.ayes, electorate)
		}
	}

	fn approval(&self, _: Class) -> pezsp_runtime::Perbill {
		let cast = self.ayes.saturating_add(self.nays);
		if cast == 0 {
			pezsp_runtime::Perbill::zero()
		} else {
			pezsp_runtime::Perbill::from_rational(self.ayes, cast)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn unanimity(_: Class) -> Self {
		Self { ayes: Electorate::get(), nays: 0, dummy: core::marker::PhantomData }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn rejection(_: Class) -> Self {
		Self { ayes: 0, nays: Electorate::get(), dummy: core::marker::PhantomData }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn from_requirements(
		support: pezsp_runtime::Perbill,
		approval: pezsp_runtime::Perbill,
		_: Class,
	) -> Self {
		// `support` is a share of the whole roll and every voter counts once, so the number who
		// turned out is exactly that share of the electorate; `approval` then splits it.
		let turnout = support.mul_ceil(Electorate::get());
		let ayes = approval.mul_ceil(turnout);
		Self { ayes, nays: turnout.saturating_sub(ayes), dummy: core::marker::PhantomData }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn setup(_: Class, _: pezsp_runtime::Perbill) {}
}

/// A citizens' initiative: a question one citizen asks and others sign onto.
///
/// The proposal itself is not held here, only the preimage that carries it. What this record
/// is for is the part that is about people: who asked, on which track, by when, and how many
/// have joined so far.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct Initiative<AccountId, BlockNumber, Hash> {
	/// Who opened it and whose deposit is at stake.
	pub proposer: AccountId,
	/// The track whose authority the proposal asks for.
	pub track: u16,
	/// The stored preimage of the call a successful referendum would dispatch.
	pub proposal: Hash,
	/// Its length, which the ballot needs in order to look it up.
	pub proposal_len: u32,
	/// After this block no more backing may be added.
	pub closes: BlockNumber,
	/// How many citizens have signed, the proposer included.
	pub backing: u32,
	/// What the proposer put up, returned on success and forfeit on lapse.
	pub deposit: u128,
}

// ===== STORED ENUM ENCODING =====
//
// SCALE encodes a fieldless enum by the variant's position, and three of these are storage
// keys. Insert a variant in the middle -- grouping by ministry, or alphabetising, is the most
// natural thing anyone would do -- and every key already written decodes as a different
// value. It does not break; it quietly means something else. A judge becomes a treasurer.
//
// The explicit indices pin the number to the variant rather than to its position, and this
// holds those numbers to what they were when the chain started. A variant may be added at the
// end with the next free number; nothing here may be renumbered, and a number left behind by
// a removed variant is not reusable.
//
// Generating those indices is itself the hazard this guards against: the first attempt lost
// nineteen variants whose names carry Kurdish letters and silently shifted everything after
// them. Two of the shifts collided and the codec derive refused to compile; the rest would
// have gone through.

#[cfg(test)]
mod stored_enum_encoding {
	use super::*;
	use codec::Encode;

	#[test]
	fn electiontype_indices_are_pinned() {
		let pinned: &[(&str, u8, &dyn Fn() -> Vec<u8>)] = &[
			("Presidential", 0u8, &|| ElectionType::Presidential.encode()),
			("Parliamentary", 1u8, &|| ElectionType::Parliamentary.encode()),
			("SpeakerElection", 2u8, &|| ElectionType::SpeakerElection.encode()),
			("ConstitutionalCourt", 3u8, &|| ElectionType::ConstitutionalCourt.encode()),
		];
		let moved: Vec<&str> = pinned
			.iter()
			.filter(|(_, want, enc)| enc() != vec![*want])
			.map(|(name, _, _)| *name)
			.collect();
		assert!(moved.is_empty(), "`ElectionType` indices moved: {moved:?}");
		assert_eq!(pinned.len(), 4, "a variant was added or removed");
	}

	#[test]
	fn officialrole_indices_are_pinned() {
		let pinned: &[(&str, u8, &dyn Fn() -> Vec<u8>)] = &[
			("Dadger", 0u8, &|| OfficialRole::Dadger.encode()),
			("Dozger", 1u8, &|| OfficialRole::Dozger.encode()),
			("Hiquqnas", 2u8, &|| OfficialRole::Hiquqnas.encode()),
			("Noter", 3u8, &|| OfficialRole::Noter.encode()),
			("Xezinedar", 4u8, &|| OfficialRole::Xezinedar.encode()),
			("Bacgir", 5u8, &|| OfficialRole::Bacgir.encode()),
			("GerinendeyeCavkaniye", 6u8, &|| OfficialRole::GerinendeyeCavkaniye.encode()),
			("OperatoreTore", 7u8, &|| OfficialRole::OperatorêTorê.encode()),
			("PisporeEwlehiyaSiber", 8u8, &|| OfficialRole::PisporêEwlehiyaSîber.encode()),
			("GerinendeyeDaneye", 9u8, &|| OfficialRole::GerinendeyeDaneye.encode()),
			("Berdevk", 10u8, &|| OfficialRole::Berdevk.encode()),
			("Qeydkar", 11u8, &|| OfficialRole::Qeydkar.encode()),
			("Balyoz", 12u8, &|| OfficialRole::Balyoz.encode()),
			("Navbeynkar", 13u8, &|| OfficialRole::Navbeynkar.encode()),
			("ParezvaneCandi", 14u8, &|| OfficialRole::ParêzvaneÇandî.encode()),
			("Mufetis", 15u8, &|| OfficialRole::Mufetîs.encode()),
			("KaliteKontrolker", 16u8, &|| OfficialRole::KalîteKontrolker.encode()),
			("Bazargan", 17u8, &|| OfficialRole::Bazargan.encode()),
			("RêvebereProjeyê", 18u8, &|| OfficialRole::RêveberêProjeyê.encode()),
			("Feqi", 19u8, &|| OfficialRole::Feqî.encode()),
			("Perwerdekar", 20u8, &|| OfficialRole::Perwerdekar.encode()),
			("Rewsenbir", 21u8, &|| OfficialRole::Rewsenbîr.encode()),
			("Mamoste", 22u8, &|| OfficialRole::Mamoste.encode()),
			("Mela", 23u8, &|| OfficialRole::Mela.encode()),
		];
		let moved: Vec<&str> = pinned
			.iter()
			.filter(|(_, want, enc)| enc() != vec![*want])
			.map(|(name, _, _)| *name)
			.collect();
		assert!(moved.is_empty(), "`OfficialRole` indices moved: {moved:?}");
		assert_eq!(pinned.len(), 24, "a variant was added or removed");
	}

	#[test]
	fn governmentposition_indices_are_pinned() {
		let pinned: &[(&str, u8, &dyn Fn() -> Vec<u8>)] = &[
			("Serok", 0u8, &|| GovernmentPosition::Serok.encode()),
			("Parlementer", 1u8, &|| GovernmentPosition::Parlementer.encode()),
			("SerokiMeclise", 2u8, &|| GovernmentPosition::SerokiMeclise.encode()),
			("EndameDiwane", 3u8, &|| GovernmentPosition::EndameDiwane.encode()),
		];
		let moved: Vec<&str> = pinned
			.iter()
			.filter(|(_, want, enc)| enc() != vec![*want])
			.map(|(name, _, _)| *name)
			.collect();
		assert!(moved.is_empty(), "`GovernmentPosition` indices moved: {moved:?}");
		assert_eq!(pinned.len(), 4, "a variant was added or removed");
	}
}
