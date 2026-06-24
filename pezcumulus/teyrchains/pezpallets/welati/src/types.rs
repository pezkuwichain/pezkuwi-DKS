// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::pezpallet_prelude::*;
use pezframe_system::{pezpallet_prelude::BlockNumberFor, Config as SystemConfig};
use pezpallet_tiki::Tiki;
use pezsp_runtime::RuntimeDebug;
use pezsp_std::prelude::*;
use scale_info::TypeInfo;

#[derive(RuntimeDebug, Eq, PartialEq)]
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
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum GovernmentPosition {
	/// President (Serok)
	Serok,
	/// Member of Parliament (Parlementer)
	Parlementer,
	/// Speaker of Parliament (SerokiMeclise)
	MeclisBaskanı,
	/// Diwan Member (EndameDiwane)
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
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum OfficialRole {
	// Under the Ministry of Justice
	Dadger,
	Dozger,
	Hiquqnas,
	Noter,

	// Under the Ministry of the Treasury
	Xezinedar,
	Bacgir,
	GerinendeyeCavkaniye,

	// Under the Ministry of Technology and Infrastructure
	OperatoreTore,
	PisporeEwlehiyaSiber,
	GerinendeyeDaneye,

	// Under the Ministry of Internal Affairs and Communications
	Berdevk,
	Qeydkar,

	// Under the Ministry of Foreign Affairs
	Balyoz,
	Navbeynkar,
	ParezvaneCandi,

	// Under the Ministry of Audit
	Mufetis,
	KaliteKontrolker,

	// Under the Ministry of Economy and Trade
	Bazargan,
	RêvebereProjeyê,

	// Under the Ministry of National Education and Religious Affairs
	Feqi,
	Perwerdekar,
	Rewsenbir,
	Mamoste,

	// Exceptional appointment (directly by Serok)
	Mela,
}

/// Minister positions (Wezîr subcategories)
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum MinisterRole {
	/// Minister of Justice
	AdvaletWeziri,
	/// Minister of the Treasury
	XezineWeziri,
	/// Minister of Technology and Infrastructure
	TeknolojîWeziri,
	/// Minister of Internal Affairs and Communications
	NavxweWeziri,
	/// Minister of Foreign Affairs
	DerveWeziri,
	/// Minister of Audit
	DenetimWeziri,
	/// Minister of Economy and Trade
	AbûrîWeziri,
	/// Minister of National Education and Religious Affairs
	PerwerdeDiyanetWeziri,
}

/// Election types
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ElectionType {
	/// Presidential election (special rules)
	Presidential,
	/// Parliamentary election (201 members)
	Parliamentary,
	/// Speaker election (among members of parliament)
	SpeakerElection,
	/// Diwan member election
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	/// Diwan decision (constitutional review - 2/3)
	ConstitutionalReview,
	/// Diwan unanimous decision (all members)
	ConstitutionalUnanimous,
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
	RuntimeDebug,
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
	/// UPDATED: The call (extrinsic) to be executed if the proposal is accepted.
	#[codec(skip)]
	pub call: Option<Box<<T as SystemConfig>::RuntimeCall>>,
}

/// Proposal priority levels
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
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
	/// Area of specialization
	pub specialization: ConstitutionalSpecialization,
	/// Number of decisions made
	pub decisions_made: u32,
}

/// Appointment authority
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[codec(mel_bound())]
#[scale_info(skip_type_params(T))]
pub enum AppointmentAuthority<T: pezframe_system::Config> {
	/// Appointed by Parliament (6 members)
	Parliament,
	/// Appointed by Serok (5 members)
	President(T::AccountId),
}

/// Constitutional areas of specialization
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Eq,
	PartialEq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Copy,
)]
#[codec(mel_bound())]
pub enum ConstitutionalSpecialization {
	/// Fundamental rights and freedoms
	FundamentalRights,
	/// State organization
	StateOrganization,
	/// Economic order
	EconomicOrder,
	/// Social rights
	SocialRights,
	/// Judicial independence
	JudicialIndependence,
	/// Local governments
	LocalGovernment,
	/// International law
	InternationalLaw,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
	RuntimeDebug,
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
			GovernmentPosition::MeclisBaskanı => 400,
			GovernmentPosition::EndameDiwane => 750,
		}
	}

	fn required_tiki(&self) -> Option<Tiki> {
		match self {
			GovernmentPosition::Serok => Some(Tiki::Welati),
			GovernmentPosition::Parlementer => Some(Tiki::Welati),
			GovernmentPosition::MeclisBaskanı => Some(Tiki::Parlementer),
			GovernmentPosition::EndameDiwane => Some(Tiki::Welati),
		}
	}

	fn term_length_blocks(&self) -> u32 {
		match self {
			GovernmentPosition::Serok => 4 * 365 * 24 * 60 * 10, // 4 years
			GovernmentPosition::Parlementer => 4 * 365 * 24 * 60 * 10, // 4 years
			GovernmentPosition::MeclisBaskanı => 2 * 365 * 24 * 60 * 10, // 2 years
			GovernmentPosition::EndameDiwane => 9 * 365 * 24 * 60 * 10, // 9 years
		}
	}
}

pub trait OfficialRoleInfo {
	fn required_trust_score(&self) -> u128;
	fn nominating_minister(&self) -> MinisterRole;
	fn requires_parliament_approval(&self) -> bool;
}

impl OfficialRoleInfo for OfficialRole {
	fn required_trust_score(&self) -> u128 {
		75 // General requirement specified in the constitution
	}

	fn nominating_minister(&self) -> MinisterRole {
		match self {
			OfficialRole::Dadger
			| OfficialRole::Dozger
			| OfficialRole::Hiquqnas
			| OfficialRole::Noter => MinisterRole::AdvaletWeziri,

			OfficialRole::Xezinedar | OfficialRole::Bacgir | OfficialRole::GerinendeyeCavkaniye => {
				MinisterRole::XezineWeziri
			},

			OfficialRole::OperatoreTore
			| OfficialRole::PisporeEwlehiyaSiber
			| OfficialRole::GerinendeyeDaneye => MinisterRole::TeknolojîWeziri,

			OfficialRole::Berdevk | OfficialRole::Qeydkar => MinisterRole::NavxweWeziri,

			OfficialRole::Balyoz | OfficialRole::Navbeynkar | OfficialRole::ParezvaneCandi => {
				MinisterRole::DerveWeziri
			},

			OfficialRole::Mufetis | OfficialRole::KaliteKontrolker => MinisterRole::DenetimWeziri,

			OfficialRole::Bazargan | OfficialRole::RêvebereProjeyê => MinisterRole::AbûrîWeziri,

			OfficialRole::Feqi
			| OfficialRole::Perwerdekar
			| OfficialRole::Rewsenbir
			| OfficialRole::Mamoste => MinisterRole::PerwerdeDiyanetWeziri,

			// Mela is a special case - appointed directly by Serok
			OfficialRole::Mela => MinisterRole::AdvaletWeziri, // Placeholder
		}
	}

	fn requires_parliament_approval(&self) -> bool {
		match self {
			// High-level positions require parliamentary approval
			OfficialRole::Dadger
			| OfficialRole::Xezinedar
			| OfficialRole::PisporeEwlehiyaSiber
			| OfficialRole::Mufetis
			| OfficialRole::Balyoz => true,
			// The others only require Serok approval
			_ => false,
		}
	}
}
