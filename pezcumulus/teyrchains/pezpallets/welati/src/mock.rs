// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{self as pezpallet_welati, *};
use pezframe_support::{
	assert_ok, construct_runtime, derive_impl, parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128, ConstU32, ConstU64, Everything, Randomness},
	BoundedVec,
};
use pezsp_core::H256;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

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

// Mock Randomness - SADECE BİR KEZ TANIMLA
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
	fn on_kyc_approved(_who: &AccountId, _referrer: &AccountId) {}
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

impl pezpallet_identity_kyc::Config for Test {
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
}

// Staking Score Configuration
impl pezpallet_staking_score::Config for Test {
	type Balance = Balance;
	type OnStakingUpdate = ();
	type WeightInfo = ();
	type NoterChecker = ();
}

// Referral Configuration
parameter_types! {
	pub const DefaultReferrerAccount: AccountId = 1;
	pub const PenaltyPerRevocation: u32 = 10;
}

impl pezpallet_referral::Config for Test {
	type WeightInfo = ();
	type DefaultReferrer = DefaultReferrerAccount;
	type PenaltyPerRevocation = PenaltyPerRevocation;
	type TrustScoreUpdater = ();
}

// Tiki Configuration
parameter_types! {
	pub const MaxTikisPerUser: u32 = 50;
	pub const TikiCollectionId: u32 = 0;
}

impl pezpallet_tiki::Config for Test {
	type AdminOrigin = pezframe_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type MaxTikisPerUser = MaxTikisPerUser;
	type Tiki = pezpallet_tiki::Tiki;
	type TikiCollectionId = TikiCollectionId;
	type TrustScoreUpdater = ();
}

// Mock implementations for required traits - YÜKSEK SKORLAR VER
pub struct MockStakingScoreProvider;
impl pezpallet_staking_score::StakingScoreProvider<AccountId, u64> for MockStakingScoreProvider {
	fn get_staking_score(_account: &AccountId) -> (u32, u64) {
		(1000, 0) // Yüksek skor
	}
}

pub struct MockReferralScoreProvider;
impl pezpallet_trust::ReferralScoreProvider<AccountId> for MockReferralScoreProvider {
	fn get_referral_score(_account: &AccountId) -> u32 {
		500 // Yüksek skor
	}
}

pub struct MockPerwerdeScoreProvider;
impl pezpallet_trust::PerwerdeScoreProvider<AccountId> for MockPerwerdeScoreProvider {
	fn get_perwerde_score(_account: &AccountId) -> u32 {
		750 // Yüksek skor
	}
}

pub struct MockTikiScoreProvider;

// `pezpallet_trust` için implementasyon
impl pezpallet_trust::TikiScoreProvider<AccountId> for MockTikiScoreProvider {
	fn get_tiki_score(_account: &AccountId) -> u32 {
		100
	}
}

// `pezpallet_welati`'nin ihtiyaç duyduğu `pezpallet_tiki` için implementasyon
impl pezpallet_tiki::TikiScoreProvider<AccountId> for MockTikiScoreProvider {
	fn get_tiki_score(_account: &AccountId) -> u32 {
		1000 // Yüksek Tiki score - tüm kontrolleri geçer
	}
}

pub struct MockCitizenshipStatusProvider;
impl pezpallet_trust::CitizenshipStatusProvider<AccountId> for MockCitizenshipStatusProvider {
	fn is_citizen(_account: &AccountId) -> bool {
		true // Herkes vatandaş
	}
}

// MOCK TRUST PROVIDER - HERKES İÇİN YÜKSEK SKOR
pub struct MockTrustProvider;
impl pezpallet_trust::TrustScoreProvider<AccountId> for MockTrustProvider {
	fn trust_score_of(_account: &AccountId) -> u128 {
		1000u128 // Herkes için yüksek trust score
	}
}

// CitizenInfo trait implementation for MockTrustProvider
impl CitizenInfo for MockTrustProvider {
	fn citizen_count() -> u32 {
		110
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
	type ScoreMultiplierBase = ScoreMultiplierBase;
	type UpdateInterval = UpdateInterval;
	type MaxBatchSize = MaxBatchSize;
	type StakingScoreSource = MockStakingScoreProvider;
	type ReferralScoreSource = MockReferralScoreProvider;
	type PerwerdeScoreSource = MockPerwerdeScoreProvider;
	type TikiScoreSource = MockTikiScoreProvider;
	type CitizenshipSource = MockCitizenshipStatusProvider;
}

// Welati Configuration - SADECE BİR KEZ TANIMLA
parameter_types! {
	pub const ParliamentSize: u32 = 201;
	pub const DiwanSize: u32 = 11;
	pub const ElectionPeriod: u64 = 432_000;
	pub const CandidacyPeriod: u64 = 86_400;
	pub const CampaignPeriod: u64 = 259_200;
	pub const ElectoralDistricts: u32 = 10;
	pub const CandidacyDeposit: u128 = 10_000;
	pub const PresidentialEndorsements: u32 = 100;
	pub const ParliamentaryEndorsements: u32 = 50;
	pub const MaxEndorsers: u32 = 100;
}

impl pezpallet_welati::Config for Test {
	type WeightInfo = ();
	type Randomness = MockRandomness;
	type RuntimeCall = RuntimeCall;
	type TrustScoreSource = MockTrustProvider; // Mock provider kullan
	type TikiSource = MockTikiScoreProvider; // Mock Tiki provider kullan
	type CitizenSource = MockTrustProvider; // Mock provider kullan
	type KycSource = IdentityKyc;
	type ParliamentSize = ParliamentSize;
	type DiwanSize = DiwanSize;
	type ElectionPeriod = ElectionPeriod;
	type CandidacyPeriod = CandidacyPeriod;
	type CampaignPeriod = CampaignPeriod;
	type ElectoralDistricts = ElectoralDistricts;
	type CandidacyDeposit = CandidacyDeposit;
	type PresidentialEndorsements = PresidentialEndorsements;
	type ParliamentaryEndorsements = ParliamentaryEndorsements;
	type NativeCurrency = Balances;
	type MaxEndorsers = MaxEndorsers;
}

// CRITICAL: CitizenInfo trait implementation - SADECE BİR KEZ TANIMLA
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

// SIMPLIFIED TEST USER SETUP - BOŞ BIRAK, MOCK PROVIDERS YETERLI
pub fn setup_test_users() {
	// Mock provider'lar zaten herkesin yüksek trust score'u olmasını sağlıyor
	// ve TikiScoreProvider da herkesin Tiki'ye sahip olduğunu söylüyor
	// Bu sayede pezpallet-tiki ile uğraşmak zorunda kalmıyoruz

	// Sadece NFTs collection'ı oluşturuldu, bu yeterli
	// Testlerde KYC kontrolü zaten bypass ediliyor
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
	}
}

pub fn last_event() -> RuntimeEvent {
	System::events().pop().expect("Event expected").event
}
