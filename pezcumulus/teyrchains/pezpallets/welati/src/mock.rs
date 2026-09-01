// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{self as pezpallet_welati, *};
use pezframe_support::traits::EnsureOrigin;
use pezframe_support::{
	assert_ok, construct_runtime, derive_impl, parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstU128, ConstU32, ConstU64, Everything, PollStatus, Polling,
		Randomness,
	},
	BoundedVec,
};
use pezframe_system::RawOrigin;
use pezsp_core::H256;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use std::collections::BTreeMap;

#[cfg(feature = "runtime-benchmarks")]
use pezsp_runtime::testing::{TestSignature, UintAuthorityId};
#[cfg(feature = "runtime-benchmarks")]
use pezsp_runtime::RuntimeAppPublic;

type Block = pezframe_system::mocking::MockBlock<Test>;
type AccountId = u64;
type Balance = u128;

// Runtime with pezpallet-identity included for pezpallet-tiki dependency
construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		Balances: pezpallet_balances,
		Timestamp: pezpallet_timestamp,
		Nfts: pezpallet_nfts,
		Identity: pezpallet_identity,
		IdentityKyc: pezpallet_identity_kyc,
		Tiki: pezpallet_tiki,
		Trust: pezpallet_trust,
		StakingScore: pezpallet_staking_score,
		Referral: pezpallet_referral,
		Welati: pezpallet_welati,
	}
);

parameter_types! {
	pub const ReferralFallbackPeriod: u64 = 100;
	pub const OracleGracePeriod: u64 = 100;
	pub const TrustScoreScale: u32 = 1_000;
	pub const TrustStakingWeight: u32 = 20;
	pub const TrustReferralWeight: u32 = 25;
	pub const TrustPerwerdeWeight: u32 = 30;
	pub const TrustTikiWeight: u32 = 25;
	pub const AssociationHeadThreshold: u32 = 25;
	pub const CommunityModeratorThreshold: u32 = 50;
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig as pezframe_system::DefaultConfig)]
impl pezframe_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = pezframe_support::weights::constants::RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pezpallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

impl pezpallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

// Mock Randomness - DEFINE ONLY ONCE
pub struct MockRandomness;
impl Randomness<H256, u64> for MockRandomness {
	fn random(_subject: &[u8]) -> (H256, u64) {
		(H256::default(), 0)
	}
}

// NFTs Configuration
parameter_types! {
	pub const CollectionDeposit: Balance = 0;
	pub const ItemDeposit: Balance = 0;
	pub const StringLimit: u32 = 64;
	pub const KeyLimit: u32 = 32;
	pub const ValueLimit: u32 = 64;
	pub const ApprovalsLimit: u32 = 1;
	pub const ItemAttributesApprovalsLimit: u32 = 1;
	pub const MaxTips: u32 = 1;
	pub const MaxDeadlineDuration: u64 = 1000;
	pub const MaxAttributesPerCall: u32 = 1;
}

// Custom BenchmarkHelper for pezpallet_nfts (uses u64 AccountId in mock)
#[cfg(feature = "runtime-benchmarks")]
pub struct NftsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_nfts::BenchmarkHelper<u32, u32, UintAuthorityId, AccountId, TestSignature>
	for NftsBenchmarkHelper
{
	fn collection(i: u16) -> u32 {
		i.into()
	}
	fn item(i: u16) -> u32 {
		i.into()
	}
	fn signer() -> (UintAuthorityId, AccountId) {
		let signer = UintAuthorityId(0);
		let account: AccountId = 1u64;
		(signer, account)
	}
	fn sign(signer: &UintAuthorityId, data: &[u8]) -> TestSignature {
		<UintAuthorityId as RuntimeAppPublic>::sign(signer, &data.to_vec()).unwrap()
	}
}

impl pezpallet_nfts::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<pezframe_system::EnsureSigned<AccountId>>;
	type ForceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type Locker = ();
	type CollectionDeposit = CollectionDeposit;
	type ItemDeposit = ItemDeposit;
	type MetadataDepositBase = ConstU128<0>;
	type AttributeDepositBase = ConstU128<0>;
	type DepositPerByte = ConstU128<0>;
	type StringLimit = StringLimit;
	type KeyLimit = KeyLimit;
	type ValueLimit = ValueLimit;
	type ApprovalsLimit = ApprovalsLimit;
	type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
	type MaxTips = MaxTips;
	type MaxDeadlineDuration = MaxDeadlineDuration;
	type MaxAttributesPerCall = MaxAttributesPerCall;
	type Features = ();
	type OffchainSignature = pezsp_runtime::testing::TestSignature;
	type OffchainPublic = pezsp_runtime::testing::UintAuthorityId;
	type WeightInfo = ();
	type BlockNumberProvider = System;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = NftsBenchmarkHelper;
}

// Identity Configuration - MINIMAL for pezpallet-tiki dependency
parameter_types! {
	pub const BasicDeposit: Balance = 10;
	pub const ByteDeposit: Balance = 1;
	pub const SubAccountDeposit: Balance = 10;
	pub const MaxSubAccounts: u32 = 2;
	pub const MaxRegistrars: u32 = 2;
	pub const MaxAdditionalFields: u32 = 2;
	pub const UsernameDeposit: Balance = 100;
	pub const MaxUsernameLength: u32 = 32;
	pub const MaxSuffixLength: u32 = 7;
	pub const PendingUsernameExpiration: u64 = 100;
	pub const UsernameGracePeriod: u64 = 100;
}

// Custom BenchmarkHelper for pezpallet_identity (uses TestSignature in mock)
#[cfg(feature = "runtime-benchmarks")]
pub struct IdentityBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_identity::BenchmarkHelper<UintAuthorityId, TestSignature>
	for IdentityBenchmarkHelper
{
	fn sign_message(message: &[u8]) -> (UintAuthorityId, TestSignature) {
		let signer = UintAuthorityId(0);
		let signature =
			<UintAuthorityId as RuntimeAppPublic>::sign(&signer, &message.to_vec()).unwrap();
		(signer, signature)
	}
}

impl pezpallet_identity::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Slashed = ();
	type ForceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type RegistrarOrigin = pezframe_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type BasicDeposit = BasicDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxRegistrars = MaxRegistrars;
	type IdentityInformation = pezpallet_identity::legacy::IdentityInfo<MaxAdditionalFields>;
	type ByteDeposit = ByteDeposit;
	type UsernameDeposit = UsernameDeposit;
	type MaxUsernameLength = MaxUsernameLength;
	type MaxSuffixLength = MaxSuffixLength;
	type PendingUsernameExpiration = PendingUsernameExpiration;
	type UsernameGracePeriod = UsernameGracePeriod;
	type UsernameAuthorityOrigin = pezframe_system::EnsureRoot<AccountId>;
	type OffchainSignature = pezsp_runtime::testing::TestSignature;
	type SigningPublicKey = pezsp_runtime::testing::UintAuthorityId;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = IdentityBenchmarkHelper;
}

// Identity KYC Configuration
parameter_types! {
	pub const KycApplicationDeposit: Balance = 1_000;
	pub const MaxStringLength: u32 = 128;
	pub const MaxCidLength: u32 = 64;
}

pub struct NoOpOnKycApproved;
impl pezpallet_identity_kyc::types::OnKycApproved<AccountId> for NoOpOnKycApproved {
	fn on_kyc_approved(_who: &AccountId, _referrer: &AccountId, _inviter: Option<&AccountId>) {}
}

pub struct NoOpOnCitizenshipRevoked;
impl pezpallet_identity_kyc::types::OnCitizenshipRevoked<AccountId> for NoOpOnCitizenshipRevoked {
	fn on_citizenship_revoked(_who: &AccountId) {}
}

pub struct NoOpCitizenNftProvider;
impl pezpallet_identity_kyc::types::CitizenNftProvider<AccountId> for NoOpCitizenNftProvider {
	fn mint_citizen_nft(_who: &AccountId) -> Result<(), pezsp_runtime::DispatchError> {
		Ok(())
	}

	fn mint_citizen_nft_confirmed(_who: &AccountId) -> Result<(), pezsp_runtime::DispatchError> {
		Ok(())
	}

	fn burn_citizen_nft(_who: &AccountId) -> Result<(), pezsp_runtime::DispatchError> {
		Ok(())
	}
}

pub struct DefaultReferrerKyc;
impl pezframe_support::traits::Get<AccountId> for DefaultReferrerKyc {
	fn get() -> AccountId {
		1
	}
}

parameter_types! {
	/// The register's own tests exercise vouching; here it only has to compile.
	pub const VouchingWaitingPeriod: u64 = 0;
}

impl pezpallet_identity_kyc::Config for Test {
	type OnCitizenshipRestored = ();
	type VouchingWaitingPeriod = VouchingWaitingPeriod;
	type VouchingCapacity = ();
	type Currency = Balances;
	type GovernanceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type OnKycApproved = NoOpOnKycApproved;
	type OnCitizenshipRevoked = NoOpOnCitizenshipRevoked;
	type CitizenNftProvider = NoOpCitizenNftProvider;
	type KycApplicationDeposit = KycApplicationDeposit;
	type MaxStringLength = MaxStringLength;
	type MaxCidLength = MaxCidLength;
	type DefaultReferrer = DefaultReferrerKyc;
	type ReferralFallbackPeriod = ReferralFallbackPeriod;
}

// Staking Score Configuration
parameter_types! {
	pub const StakingScoreNoterBondAmount: Balance = 1_000;
	pub const StakingScoreDisputeWindow: u64 = 10;
	pub const StakingScoreSlashDestination: AccountId = 999;
}

impl pezpallet_staking_score::Config for Test {
	type Balance = Balance;
	type OnStakingUpdate = ();
	type WeightInfo = ();
	type NoterChecker = ();
	type Currency = Balances;
	type NoterBondAmount = StakingScoreNoterBondAmount;
	type DisputeWindow = StakingScoreDisputeWindow;
	type DisputeOrigin = pezframe_system::EnsureSigned<AccountId>;
	type SlashOrigin = pezframe_system::EnsureRoot<AccountId>;
	type SlashDestination = SlashesTo<StakingScoreSlashDestination>;
	type OracleGracePeriod = OracleGracePeriod;
}

// Referral Configuration
parameter_types! {
	pub const DefaultReferrerAccount: AccountId = 1;
	pub const PenaltyPerRevocation: u32 = 10;
}

parameter_types! {
	pub const InitialVouchingCapacity: u32 = 5;
	pub const SettledVouchesPerPlace: u32 = 3;
	pub const MaxVouchingCapacity: u32 = 50;
	pub const SuspensionRevocationFloor: u32 = 3;
	pub const SuspensionRevocationPercent: u32 = 20;
}

impl pezpallet_referral::Config for Test {
	type InitialVouchingCapacity = InitialVouchingCapacity;
	type SettledVouchesPerPlace = SettledVouchesPerPlace;
	type MaxVouchingCapacity = MaxVouchingCapacity;
	type SuspensionRevocationFloor = SuspensionRevocationFloor;
	type SuspensionRevocationPercent = SuspensionRevocationPercent;
	type WeightInfo = ();
	type DefaultReferrer = DefaultReferrerAccount;
	type PenaltyPerRevocation = PenaltyPerRevocation;
	type TrustScoreUpdater = ();
	type EarnedRoles = ();
	type AssociationHeadThreshold = AssociationHeadThreshold;
	type CommunityModeratorThreshold = CommunityModeratorThreshold;
}

// Tiki Configuration
parameter_types! {
	pub const MaxTikisPerUser: u32 = 50;
	pub const TikiCollectionId: u32 = 0;
}

impl pezpallet_tiki::Config for Test {
	type AdminOrigin = pezframe_system::EnsureRoot<AccountId>;
	type ElectedRoleOrigin = pezframe_system::EnsureRoot<AccountId>;
	type EarnedRoleOrigin = pezframe_system::EnsureRoot<AccountId>;
	type ImpeachmentOrigin = pezframe_system::EnsureRoot<AccountId>;
	type HonoraryCitizenshipOrigin = pezframe_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type MaxTikisPerUser = MaxTikisPerUser;
	type Tiki = pezpallet_tiki::Tiki;
	type TikiCollectionId = TikiCollectionId;
	type TrustScoreUpdater = ();
}

// Mock implementations for required traits - PROVIDE HIGH SCORES
pub struct MockStakingScoreProvider;
impl pezpallet_staking_score::StakingScoreProvider<AccountId, u64> for MockStakingScoreProvider {
	fn max_score() -> u32 {
		100
	}

	fn get_staking_score(_account: &AccountId) -> (u32, u64) {
		(1000, 0) // High score
	}
}

pub struct MockReferralScoreProvider;
impl pezpallet_trust::ReferralScoreProvider<AccountId> for MockReferralScoreProvider {
	fn max_score() -> u32 {
		500
	}

	fn get_referral_score(_account: &AccountId) -> u32 {
		500 // High score
	}
}

pub struct MockPerwerdeScoreProvider;
impl pezpallet_trust::PerwerdeScoreProvider<AccountId> for MockPerwerdeScoreProvider {
	fn max_score() -> u32 {
		50_000
	}

	fn get_perwerde_score(_account: &AccountId) -> u32 {
		750 // High score
	}
}

pub struct MockTikiScoreProvider;

// Implementation for `pezpallet_trust`
impl pezpallet_trust::TikiScoreProvider<AccountId> for MockTikiScoreProvider {
	fn max_score() -> u32 {
		1_000
	}

	fn get_tiki_score(_account: &AccountId) -> u32 {
		100
	}
}

// Implementation for `pezpallet_tiki`, which `pezpallet_welati` requires
impl pezpallet_tiki::TikiScoreProvider<AccountId> for MockTikiScoreProvider {
	fn get_tiki_score(_account: &AccountId) -> u32 {
		1000 // High Tiki score - passes all checks
	}
}

pub struct MockCitizenshipStatusProvider;
impl pezpallet_trust::CitizenshipStatusProvider<AccountId> for MockCitizenshipStatusProvider {
	fn is_citizen(_account: &AccountId) -> bool {
		true // Everyone is a citizen
	}
}

// MOCK TRUST PROVIDER - HIGH SCORE FOR EVERYONE
thread_local! {
	/// What the mock register scores everyone at.
	///
	/// Settable because trust of zero is a rule, not an edge: it is technical death, and a
	/// constant high score can only show the living half of it. Starts high so the tests
	/// written before it was settable keep behaving as they did.
	pub static TRUST_SCORE: core::cell::Cell<u128> = const { core::cell::Cell::new(1000) };
}

/// Set what the register scores everyone at.
pub fn set_trust_score(n: u128) {
	TRUST_SCORE.with(|c| c.set(n));
}

pub struct MockTrustProvider;
impl pezpallet_trust::TrustScoreProvider<AccountId> for MockTrustProvider {
	fn trust_score_of(_account: &AccountId) -> u128 {
		TRUST_SCORE.with(|c| c.get())
	}
}

// CitizenInfo trait implementation for MockTrustProvider
thread_local! {
	/// How many citizens the mock register reports.
	///
	/// Settable because the population gate has two sides -- below the threshold nothing may
	/// be sent, above it exactly one message may be -- and a constant can only show one of
	/// them. Starts above the mock threshold so the tests written before the gate existed
	/// keep behaving as they did.
	pub static CITIZEN_COUNT: core::cell::Cell<u32> = const { core::cell::Cell::new(110) };
}

/// Set what the register reports.
pub fn set_citizen_count(n: u32) {
	CITIZEN_COUNT.with(|c| c.set(n));
}

impl CitizenInfo for MockTrustProvider {
	fn citizen_count() -> u32 {
		CITIZEN_COUNT.with(|c| c.get())
	}
}

// Trust Configuration
parameter_types! {
	pub const ScoreMultiplierBase: u128 = 100;
	pub const UpdateInterval: u64 = 1000;
	pub const MaxBatchSize: u32 = 100;
}

impl pezpallet_trust::Config for Test {
	type WeightInfo = ();
	type Score = u128;
	type ScoreScale = TrustScoreScale;
	type StakingWeight = TrustStakingWeight;
	type ReferralWeight = TrustReferralWeight;
	type PerwerdeWeight = TrustPerwerdeWeight;
	type TikiWeight = TrustTikiWeight;
	type UpdateInterval = UpdateInterval;
	type MaxBatchSize = MaxBatchSize;
	type StakingScoreSource = MockStakingScoreProvider;
	type ReferralScoreSource = MockReferralScoreProvider;
	type PerwerdeScoreSource = MockPerwerdeScoreProvider;
	type TikiScoreSource = MockTikiScoreProvider;
	type CitizenshipSource = MockCitizenshipStatusProvider;
}

// Welati Configuration - DEFINE ONLY ONCE
parameter_types! {
	pub const ParliamentSize: u32 = 201;
	pub const DiwanSize: u32 = 11;
	pub const DiwanElectedSeats: u32 = 6;
	// Kept in the same proportion as the real chain -- a term much longer than the time it
	// takes to run an election -- but small enough that a test can watch a whole term go by.
	// They used to be the real block counts, which made one election cycle nearly eight times
	// longer than a term in this mock: every office was permanently overdue for an election,
	// and every test that ran a term walked through eight hundred thousand blocks to do it.
	pub const ElectionPeriod: u64 = 30;
	pub const CandidacyPeriod: u64 = 10;
	pub const CampaignPeriod: u64 = 20;
	pub const ElectoralDistricts: u32 = 10;
	pub const CandidacyDeposit: u128 = 10_000;
	pub const PresidentialEndorsements: u32 = 100;
	pub const ParliamentaryEndorsements: u32 = 50;
	pub const MaxEndorsers: u32 = 100;
	/// Short enough that a test can watch a term run out, long enough that an election fits
	/// inside it several times over.
	pub const TermLength: u64 = 1_000;
	pub const CourtTermLength: u64 = 2_250;
	pub const MaxConsecutiveTerms: u32 = 2;

	/// Stands in for the Asset Hub. The mock never delivers anything there; what the tests
	/// check is which messages the pallet decides to send, not that they arrive.
	pub TreasuryChain: xcm::latest::Location = xcm::latest::Location::new(1, [xcm::latest::Junction::Teyrchain(1000)]);
	pub const TreasuryPalletIndex: u8 = 70;
	pub const ParametersPalletIndex: u8 = 79;
	pub const AirdropPotPalletIndex: u8 = 68;
	pub const PresalePotPalletIndex: u8 = 69;
	// Short, so a lock can be waited out in a test without a hundred thousand blocks.
	pub const PresaleLockMonth: u64 = 30;
	/// Small next to the balances the tests use, so a test can cross it on purpose without
	/// having to mint a realistic amount first.
	pub const AirdropCeiling: u128 = 1_000;
	pub const LargeAirdropDelay: u64 = 100;
	/// One point per step and ten blocks between them. The production values are a point per
	/// quarter; what the tests need is the shape, not the calendar.
	pub const MaxEmissionStep: pezsp_runtime::Perbill = pezsp_runtime::Perbill::from_percent(1);
	pub const MinEmissionInterval: u64 = 10;
	/// Small enough that a test can cross it.
	pub const PopulationThreshold: u32 = 100;
	pub const PopulationCheckPeriod: u64 = 10;
}

/// Records what the pallet tried to send instead of sending it.
///
/// A test that only checked storage could not tell "decided not to send" from "sent and it
/// vanished". Keeping the messages makes the difference visible.
pub struct RecordingXcmSender;

thread_local! {
	pub static SENT_XCM: core::cell::RefCell<Vec<(xcm::latest::Location, xcm::latest::Xcm<()>)>> =
		const { core::cell::RefCell::new(Vec::new()) };
}

impl xcm::latest::SendXcm for RecordingXcmSender {
	type Ticket = (xcm::latest::Location, xcm::latest::Xcm<()>);

	fn validate(
		dest: &mut Option<xcm::latest::Location>,
		msg: &mut Option<xcm::latest::Xcm<()>>,
	) -> xcm::latest::SendResult<Self::Ticket> {
		let pair = (dest.take().unwrap(), msg.take().unwrap());
		Ok((pair, xcm::latest::Assets::new()))
	}

	fn deliver(ticket: Self::Ticket) -> Result<xcm::latest::XcmHash, xcm::latest::SendError> {
		SENT_XCM.with(|q| q.borrow_mut().push(ticket));
		Ok([0u8; 32])
	}
}

/// Everything the pallet has tried to send so far.
pub fn sent_xcm() -> Vec<(xcm::latest::Location, xcm::latest::Xcm<()>)> {
	SENT_XCM.with(|q| q.borrow().clone())
}

/// Forget what was sent, so a test can assert about one stretch of blocks.
pub fn clear_sent_xcm() {
	SENT_XCM.with(|q| q.borrow_mut().clear());
}

// --- State referenda, as the voting extrinsic sees them ---
//
// A map standing in for `pezpallet_referenda`. The pallet under test only ever reaches a poll
// through `Polling`, so the whole of what it can observe is here: whether a question is open,
// what class it belongs to, and the running count. Wiring the real pallet in would test that
// pallet, not this one.

/// How many citizens the roll holds, as the tally divides by.
parameter_types! {
	pub static MockElectorate: u32 = 100;
	pub static MockPolls: BTreeMap<u32, MockPollState> =
		vec![(1u32, MockPollState::Ongoing(<CitizenTally<MockElectorate> as pezframe_support::traits::VoteTally<u32, u16>>::new(0), 0u16))].into_iter().collect();
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MockPollState {
	Ongoing(CitizenTally<MockElectorate>, u16),
	Completed(u64, bool),
}

pub struct TestPolls;
impl Polling<CitizenTally<MockElectorate>> for TestPolls {
	type Index = u32;
	type Votes = u32;
	type Moment = u64;
	type Class = u16;

	fn classes() -> Vec<u16> {
		vec![0]
	}

	fn as_ongoing(index: u32) -> Option<(CitizenTally<MockElectorate>, u16)> {
		match MockPolls::get().remove(&index) {
			Some(MockPollState::Ongoing(tally, class)) => Some((tally, class)),
			_ => None,
		}
	}

	fn access_poll<R>(
		index: u32,
		f: impl FnOnce(PollStatus<&mut CitizenTally<MockElectorate>, u64, u16>) -> R,
	) -> R {
		let mut polls = MockPolls::get();
		let r = match polls.get_mut(&index) {
			Some(MockPollState::Ongoing(ref mut tally, class)) => {
				f(PollStatus::Ongoing(tally, *class))
			},
			Some(MockPollState::Completed(when, ok)) => f(PollStatus::Completed(*when, *ok)),
			None => f(PollStatus::None),
		};
		MockPolls::set(polls);
		r
	}

	fn try_access_poll<R>(
		index: u32,
		f: impl FnOnce(
			PollStatus<&mut CitizenTally<MockElectorate>, u64, u16>,
		) -> Result<R, pezsp_runtime::DispatchError>,
	) -> Result<R, pezsp_runtime::DispatchError> {
		let mut polls = MockPolls::get();
		let r = match polls.get_mut(&index) {
			Some(MockPollState::Ongoing(ref mut tally, class)) => {
				f(PollStatus::Ongoing(tally, *class))
			},
			Some(MockPollState::Completed(when, ok)) => f(PollStatus::Completed(*when, *ok)),
			None => f(PollStatus::None),
		}?;
		MockPolls::set(polls);
		Ok(r)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn create_ongoing(class: u16) -> Result<u32, ()> {
		let mut polls = MockPolls::get();
		let i = polls.keys().next_back().map_or(0, |x| x + 1);
		polls.insert(i, MockPollState::Ongoing(<CitizenTally<MockElectorate> as pezframe_support::traits::VoteTally<u32, u16>>::new(0), class));
		MockPolls::set(polls);
		Ok(i)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn end_ongoing(index: u32, approved: bool) -> Result<(), ()> {
		let mut polls = MockPolls::get();
		match polls.get(&index) {
			Some(MockPollState::Ongoing(..)) => {},
			_ => return Err(()),
		}
		polls.insert(index, MockPollState::Completed(0, approved));
		MockPolls::set(polls);
		Ok(())
	}
}

// --- The ballot box an initiative reaches ---
//
// The real runtime does this by calling `Referenda::submit`. All that matters here is that the
// pallet calls at the right moment with the right arguments; the ballot box itself is another
// pallet's test.
thread_local! {
	pub static LAUNCHED: core::cell::RefCell<Vec<(AccountId, u16, H256, u32)>> =
		const { core::cell::RefCell::new(Vec::new()) };
}

/// What reached the ballot, in order.
pub fn launched() -> Vec<(AccountId, u16, H256, u32)> {
	LAUNCHED.with(|l| l.borrow().clone())
}

pub struct MockInitiativeLaunch;
impl pezpallet_welati::InitiativeLaunch<AccountId, H256> for MockInitiativeLaunch {
	fn launch(proposer: &AccountId, track: u16, hash: H256, len: u32) -> DispatchResult {
		LAUNCHED.with(|l| l.borrow_mut().push((*proposer, track, hash, len)));
		Ok(())
	}
}

parameter_types! {
	pub const MockInitiativeThreshold: pezsp_runtime::Perbill =
		pezsp_runtime::Perbill::from_percent(1);
	pub const MockInitiativeWindow: u64 = 100;
	pub const MockInitiativeDeposit: u128 = 10;
	pub const MockInitiativeSlashTarget: AccountId = 999;
	pub const MockInitiativeCooldown: u64 = 50;
}

/// Stands in for the >1/2 Parliament proportion the runtime uses.
///
/// It accepts a single sitting member rather than a majority, which is looser than production,
/// but the property the tests are here to hold is the one it keeps exactly: the Serok's own
/// signed origin does not satisfy it. A mock narrower than production would have made that
/// unreachable instead of merely untested.
pub struct ParliamentBody;
impl EnsureOrigin<RuntimeOrigin> for ParliamentBody {
	type Success = ();

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match o.clone().into() {
			Ok(RawOrigin::Root) => Ok(()),
			Ok(RawOrigin::Signed(who)) if Welati::is_parliament_member(&who) => Ok(()),
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RawOrigin::Root.into())
	}
}

impl pezpallet_welati::Config for Test {
	type WeightInfo = ();
	type Randomness = MockRandomness;
	type RuntimeCall = RuntimeCall;
	type TrustScoreSource = MockTrustProvider; // Use the mock provider
	type TikiSource = MockTikiScoreProvider; // Use the mock Tiki provider
	type CitizenSource = MockTrustProvider; // Use the mock provider
	type Electorate = MockElectorate;
	type Polls = TestPolls;
	type Initiatives = MockInitiativeLaunch;
	type InitiativeThreshold = MockInitiativeThreshold;
	type InitiativeWindow = MockInitiativeWindow;
	type InitiativeDeposit = MockInitiativeDeposit;
	type InitiativeSlashTarget = SlashesTo<MockInitiativeSlashTarget>;
	type ConfirmationOrigin = ParliamentBody;
	type InitiativeCooldown = MockInitiativeCooldown;
	type KycSource = IdentityKyc;
	type ParliamentSize = ParliamentSize;
	type DiwanSize = DiwanSize;
	type DiwanElectedSeats = DiwanElectedSeats;
	// The court's deliberations are the collective's job on a real runtime; the mock only
	// needs the membership rules, so nothing is relayed here.
	type HouseRoster = ();
	type CourtRoster = ();
	type ElectionPeriod = ElectionPeriod;
	type CandidacyPeriod = CandidacyPeriod;
	type CampaignPeriod = CampaignPeriod;
	type ElectoralDistricts = ElectoralDistricts;
	type CandidacyDeposit = CandidacyDeposit;
	type PresidentialEndorsements = PresidentialEndorsements;
	type ParliamentaryEndorsements = ParliamentaryEndorsements;
	type NativeCurrency = Balances;
	type MaxEndorsers = MaxEndorsers;
	type TermLength = TermLength;
	type CourtTermLength = CourtTermLength;
	type MaxConsecutiveTerms = MaxConsecutiveTerms;
	type XcmSender = RecordingXcmSender;
	type TreasuryChainLocation = TreasuryChain;
	type TreasuryPalletIndex = TreasuryPalletIndex;
	type ParametersPalletIndex = ParametersPalletIndex;
	type AirdropPotPalletIndex = AirdropPotPalletIndex;
	type AirdropCeiling = AirdropCeiling;
	type PresalePotPalletIndex = PresalePotPalletIndex;
	type PresaleLockMonth = PresaleLockMonth;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = WelatiBenchmarkHelper;
	type LargeAirdropDelay = LargeAirdropDelay;
	type MaxEmissionStep = MaxEmissionStep;
	type MinEmissionInterval = MinEmissionInterval;
	type PopulationThreshold = PopulationThreshold;
	type PopulationCheckPeriod = PopulationCheckPeriod;
}

// CRITICAL: CitizenInfo trait implementation - DEFINE ONLY ONCE
impl CitizenInfo for Trust {
	fn citizen_count() -> u32 {
		110
	}
}

// Test externalities builder
pub struct ExtBuilder {
	balances: Vec<(AccountId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: (1..=110).map(|i| (i as AccountId, 100_000_000_000_000)).collect() }
	}
}

impl ExtBuilder {
	pub fn build(self) -> pezsp_io::TestExternalities {
		let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		pezpallet_balances::GenesisConfig::<Test> { balances: self.balances, dev_accounts: None }
			.assimilate_storage(&mut t)
			.unwrap();

		let mut ext = pezsp_io::TestExternalities::new(t);
		ext.execute_with(|| {
			System::set_block_number(1);

			assert_ok!(Nfts::create(RuntimeOrigin::signed(1), 1, Default::default()));

			setup_test_users();
		});
		ext
	}
}

// SIMPLIFIED TEST USER SETUP - LEAVE EMPTY, MOCK PROVIDERS ARE SUFFICIENT
/// Make the accounts the tests use into citizens.
///
/// This used to do nothing, on the grounds that the mock providers reported a high trust score
/// and a non-zero tiki score for everybody. That was enough while `register_candidate` only
/// asked whether the tiki score was above zero -- a question every citizen passes, since
/// citizenship itself is worth points, and which the mock answered "yes" to for accounts that
/// were not citizens at all. Now that the check asks for the *specific* role an office
/// requires, the candidates have to actually hold it.
pub fn setup_test_users() {
	for who in 1..=12u64 {
		assert_ok!(pezpallet_tiki::Pezpallet::<Test>::mint_citizen_nft_for_user(&who));
	}
}

// CRITICAL HELPER FUNCTION FOR TESTS
pub fn add_parliament_member(account: AccountId) {
	let member = ParliamentMember {
		account,
		elected_at: System::block_number(),
		term_ends_at: System::block_number() + 100_000,
		votes_participated: 0,
		total_votes_eligible: 0,
		participation_rate: 100,
		committees: BoundedVec::default(),
	};

	let mut members = ParliamentMembers::<Test>::get();
	if members.try_push(member).is_ok() {
		ParliamentMembers::<Test>::put(members);
	}
}

pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 0 {
			System::on_finalize(System::block_number());
			Welati::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		Welati::on_initialize(System::block_number());
		System::on_initialize(System::block_number());
		check_invariants();
	}
}

/// Assert the pallet's `try_state` invariant.
///
/// Run after every block the tests advance through, so the constitutional rules are checked
/// against the histories the tests actually produce -- an election counted late, a vacancy, a
/// handover -- rather than in a test of its own that would only ever see the states somebody
/// thought to write down.
pub fn check_invariants() {
	#[cfg(feature = "try-runtime")]
	{
		use pezframe_support::traits::Hooks;
		<Welati as Hooks<u64>>::try_state(System::block_number()).expect("try_state failed");
	}
}

/// Give `who` a citizen NFT, so a tiki can be granted to them.
///
/// `internal_grant_role` refuses anyone without one, which is the rule that keeps offices from
/// being handed to accounts that are not citizens. Tests that appoint someone have to satisfy
/// it the same way the chain does.
pub fn make_citizen(who: AccountId) {
	// Idempotent: `setup_test_users` already made the low-numbered accounts citizens, and a
	// test that names one of them should not have to know which.
	if pezpallet_tiki::CitizenNft::<Test>::get(who).is_none() {
		assert_ok!(pezpallet_tiki::Pezpallet::<Test>::mint_citizen_nft_for_user(&who));
	}
}

/// Who holds a single-holder office right now.
pub fn holder_of(tiki: pezpallet_tiki::Tiki) -> Option<AccountId> {
	pezpallet_tiki::TikiHolder::<Test>::get(tiki)
}

/// Seat `who` as President, the way a won election would.
///
/// Including the mandate. A counted election calls `begin_term`, and `try_state` holds the
/// chain to it: an elected office with no term recorded is an officeholder no clock will ever
/// remove. A helper that seated the tiki and skipped the clock would put the tests in a state
/// the chain cannot reach.
/// Put a Prime Minister in the chair the way the state does, in both halves.
///
/// The President names and the House seats, and a test about what a Prime Minister can do
/// afterwards should not have to spell that out. `ParliamentBody` takes Root here; the tests
/// that are actually about the separation use a seated member and check that the President's
/// own origin is refused.
pub fn install_prime_minister(nominating: RuntimeOrigin, who: AccountId) {
	assert_ok!(Welati::appoint_prime_minister(nominating, who));
	assert_ok!(Welati::confirm_prime_minister(RuntimeOrigin::root()));
}

pub fn seat_president(who: AccountId) {
	make_citizen(who);
	assert_ok!(Welati::seat_unique_tiki(&who, pezpallet_tiki::Tiki::Serok));
	pezpallet_tiki::TikiHolder::<Test>::insert(pezpallet_tiki::Tiki::Serok, who);
	crate::TermEnds::<Test>::insert(
		ElectionType::Presidential,
		System::block_number() + TermLength::get(),
	);
}

/// Have each account endorse `candidate`, then hand the list back for the candidacy.
///
/// A candidate used to submit a list of names and the pallet counted them. Now every name has
/// to have said so itself, so a test that registers a candidacy has to produce the
/// endorsements the same way a real one would.
pub fn endorsed_by(
	election_id: u32,
	candidate: AccountId,
	endorsers: Vec<AccountId>,
) -> Vec<AccountId> {
	for endorser in &endorsers {
		assert_ok!(Welati::endorse_candidate(
			RuntimeOrigin::signed(*endorser),
			election_id,
			candidate
		));
	}
	endorsers
}

pub fn last_event() -> RuntimeEvent {
	System::events().pop().expect("Event expected").event
}

/// Mock handler: puts the slashed value into an account instead of dropping it, so a test can
/// assert where it went. Dropping the imbalance would destroy the tokens and the test would
/// still pass, which is the failure mode the real handler exists to prevent.
pub struct SlashesTo<A>(core::marker::PhantomData<A>);
impl<A: pezframe_support::traits::Get<AccountId>>
	pezframe_support::traits::OnUnbalanced<pezpallet_balances::NegativeImbalance<Test>>
	for SlashesTo<A>
{
	fn on_nonzero_unbalanced(amount: pezpallet_balances::NegativeImbalance<Test>) {
		use pezframe_support::traits::Currency;
		Balances::resolve_creating(&A::get(), amount);
	}
}

/// What the benchmarks need arranged here, mirroring what the People runtimes arrange.
///
/// A no-op `()` would compile and then fail every benchmark that seats somebody, which is how
/// `approve_appointment` came to have a recorded weight nobody could regenerate. The mock is
/// the local gate for the benchmark suite, so it has to do the real thing.
#[cfg(feature = "runtime-benchmarks")]
pub struct WelatiBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_welati::BenchmarkHelper<AccountId> for WelatiBenchmarkHelper {
	/// `RecordingXcmSender` accepts everything, so there is no route to open.
	fn ensure_treasury_reachable() {}

	fn make_citizen(who: &AccountId) {
		if pezpallet_tiki::CitizenNft::<Test>::get(who).is_none() {
			pezpallet_tiki::Pezpallet::<Test>::mint_citizen_nft_for_user(who)
				.expect("the mock's genesis creates the collection");
		}
	}
}
