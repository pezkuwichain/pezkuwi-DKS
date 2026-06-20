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

use super::*;
use crate::xcm_config::LocationToAccountId;
use codec::{Decode, Encode, MaxEncodedLen};
use enumflags2::{bitflags, BitFlags};
use pezframe_support::{
	parameter_types,
	traits::{ConstU32, WithdrawReasons},
	weights::Weight,
	CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound,
};
use pezframe_system::EnsureRoot;
use pezpallet_identity::{Data, IdentityInformationProvider};
use pezsp_runtime::{
	traits::{AccountIdConversion, ConvertInto, Verify},
	RuntimeDebug,
};
use scale_info::TypeInfo;
use testnet_teyrchains_constants::pezkuwichain::currency::UNITS;
use teyrchains_common::{impls::ToParentTreasury, DAYS, HOURS};

parameter_types! {
	//   27 | Min encoded size of `Registration`
	// - 10 | Min encoded size of `IdentityInfo`
	// -----|
	//   17 | Min size without `IdentityInfo` (accounted for in byte deposit)
	pub const BasicDeposit: Balance = deposit(1, 17);
	pub const ByteDeposit: Balance = deposit(0, 1);
	pub const UsernameDeposit: Balance = deposit(0, 32);
	pub const SubAccountDeposit: Balance = deposit(1, 53);
	pub RelayTreasuryAccount: AccountId =
		teyrchains_common::TREASURY_PALLET_ID.into_account_truncating();
}

impl pezpallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type UsernameDeposit = UsernameDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = ConstU32<100>;
	type IdentityInformation = IdentityInfo;
	type MaxRegistrars = ConstU32<20>;
	type Slashed = ToParentTreasury<RelayTreasuryAccount, LocationToAccountId, Runtime>;
	type ForceOrigin = EnsureRoot<Self::AccountId>;
	type RegistrarOrigin = EnsureRoot<Self::AccountId>;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as Verify>::Signer;
	type UsernameAuthorityOrigin = EnsureRoot<Self::AccountId>;
	type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
	type UsernameGracePeriod = ConstU32<{ 3 * DAYS }>;
	type MaxSuffixLength = ConstU32<7>;
	type MaxUsernameLength = ConstU32<32>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type WeightInfo = weights::pezpallet_identity::WeightInfo<Runtime>;
}

/// The fields that we use to identify the owner of an account with. Each corresponds to a field
/// in the `IdentityInfo` struct.
#[bitflags]
#[repr(u64)]
#[derive(Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub enum IdentityField {
	Display,
	Legal,
	Web,
	Matrix,
	Email,
	PgpFingerprint,
	Image,
	Twitter,
	GitHub,
	Discord,
}

/// Information concerning the identity of the controller of an account.
#[derive(
	CloneNoBound,
	Encode,
	Decode,
	DecodeWithMemTracking,
	EqNoBound,
	MaxEncodedLen,
	PartialEqNoBound,
	RuntimeDebugNoBound,
	TypeInfo,
)]
#[codec(mel_bound())]
pub struct IdentityInfo {
	/// A reasonable display name for the controller of the account. This should be whatever the
	/// account is typically known as and should not be confusable with other entities, given
	/// reasonable context.
	///
	/// Stored as UTF-8.
	pub display: Data,

	/// The full legal name in the local jurisdiction of the entity. This might be a bit
	/// long-winded.
	///
	/// Stored as UTF-8.
	pub legal: Data,

	/// A representative website held by the controller of the account.
	///
	/// NOTE: `https://` is automatically prepended.
	///
	/// Stored as UTF-8.
	pub web: Data,

	/// The Matrix (e.g. for Element) handle held by the controller of the account. Previously,
	/// this was called `riot`.
	///
	/// Stored as UTF-8.
	pub matrix: Data,

	/// The email address of the controller of the account.
	///
	/// Stored as UTF-8.
	pub email: Data,

	/// The PGP/GPG public key of the controller of the account.
	pub pgp_fingerprint: Option<[u8; 20]>,

	/// A graphic image representing the controller of the account. Should be a company,
	/// organization or project logo or a headshot in the case of a human.
	pub image: Data,

	/// The Twitter identity. The leading `@` character may be elided.
	pub twitter: Data,

	/// The GitHub username of the controller of the account.
	pub github: Data,

	/// The Discord username of the controller of the account.
	pub discord: Data,
}

impl IdentityInformationProvider for IdentityInfo {
	type FieldsIdentifier = u64;

	fn has_identity(&self, fields: Self::FieldsIdentifier) -> bool {
		self.fields().bits() & fields == fields
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn create_identity_info() -> Self {
		let data = Data::Raw(alloc::vec![0; 32].try_into().unwrap());

		IdentityInfo {
			display: data.clone(),
			legal: data.clone(),
			web: data.clone(),
			matrix: data.clone(),
			email: data.clone(),
			pgp_fingerprint: Some([0; 20]),
			image: data.clone(),
			twitter: data.clone(),
			github: data.clone(),
			discord: data,
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn all_fields() -> Self::FieldsIdentifier {
		use enumflags2::BitFlag;
		IdentityField::all().bits()
	}
}

impl IdentityInfo {
	pub(crate) fn fields(&self) -> BitFlags<IdentityField> {
		let mut res = <BitFlags<IdentityField>>::empty();
		if !self.display.is_none() {
			res.insert(IdentityField::Display);
		}
		if !self.legal.is_none() {
			res.insert(IdentityField::Legal);
		}
		if !self.web.is_none() {
			res.insert(IdentityField::Web);
		}
		if !self.matrix.is_none() {
			res.insert(IdentityField::Matrix);
		}
		if !self.email.is_none() {
			res.insert(IdentityField::Email);
		}
		if self.pgp_fingerprint.is_some() {
			res.insert(IdentityField::PgpFingerprint);
		}
		if !self.image.is_none() {
			res.insert(IdentityField::Image);
		}
		if !self.twitter.is_none() {
			res.insert(IdentityField::Twitter);
		}
		if !self.github.is_none() {
			res.insert(IdentityField::GitHub);
		}
		if !self.discord.is_none() {
			res.insert(IdentityField::Discord);
		}
		res
	}
}

/// A `Default` identity. This is given to users who get a username but have not set an identity.
impl Default for IdentityInfo {
	fn default() -> Self {
		IdentityInfo {
			display: Data::None,
			legal: Data::None,
			web: Data::None,
			matrix: Data::None,
			email: Data::None,
			pgp_fingerprint: None,
			image: Data::None,
			twitter: Data::None,
			github: Data::None,
			discord: Data::None,
		}
	}
}

// =============================================================================
// PezkuwiChain Custom People Pallets Configuration
// =============================================================================
// NOTE: These configurations are placeholders. Full implementation requires
// additional pezpallet API alignment. See compile errors for specific issues.
// =============================================================================

parameter_types! {
	/// Deposit required for KYC application (spam prevention)
	pub const KycApplicationDeposit: Balance = UNITS; // 1 PEZ
	/// Maximum string length for identity fields
	pub const MaxStringLength: u32 = 128;
	/// Maximum CID (IPFS) length
	pub const MaxCidLength: u32 = 64;
}

// OnKycApproved hook → Delegates to Referral pallet for referral confirmation
// Referral pallet implements OnKycApproved trait directly and also triggers TrustScoreUpdater
// OnCitizenshipRevoked hook → Delegates to Referral pallet for penalty tracking
// Referral pallet implements OnCitizenshipRevoked trait directly and also triggers TrustScoreUpdater
// CitizenNftProvider → Delegates to Tiki pallet for citizenship NFT minting/burning

/// Adapter struct that bridges each pallet's local TrustScoreUpdater trait
/// to the Trust pallet's on_score_component_changed implementation.
/// This avoids cyclic dependencies between component pallets and pezpallet-trust.
pub struct TrustScoreNotifier;

impl pezpallet_referral::TrustScoreUpdater<AccountId> for TrustScoreNotifier {
	fn on_score_component_changed(who: &AccountId) {
		use pezpallet_trust::TrustScoreUpdater;
		<Trust as TrustScoreUpdater<AccountId>>::on_score_component_changed(who);
	}
}

impl pezpallet_tiki::TrustScoreUpdater<AccountId> for TrustScoreNotifier {
	fn on_score_component_changed(who: &AccountId) {
		use pezpallet_trust::TrustScoreUpdater;
		<Trust as TrustScoreUpdater<AccountId>>::on_score_component_changed(who);
	}
}

impl pezpallet_perwerde::TrustScoreUpdater<AccountId> for TrustScoreNotifier {
	fn on_score_component_changed(who: &AccountId) {
		use pezpallet_trust::TrustScoreUpdater;
		<Trust as TrustScoreUpdater<AccountId>>::on_score_component_changed(who);
	}
}

impl pezpallet_identity_kyc::Config for Runtime {
	type Currency = Balances;
	// Kademeli yetki devri: Root → Diwan → Teknik Komisyon
	// Vatandaşlık kararları için Divan (Anayasa Mahkemesi) yetkili
	type GovernanceOrigin = crate::RootOrDiwanOrTechnical;
	type WeightInfo = pezpallet_identity_kyc::weights::BizinikiwiWeight<Runtime>;
	type OnKycApproved = Referral;
	type OnCitizenshipRevoked = Referral;
	type CitizenNftProvider = Tiki;
	type KycApplicationDeposit = KycApplicationDeposit;
	type MaxStringLength = MaxStringLength;
	type MaxCidLength = MaxCidLength;
	type DefaultReferrer = DefaultReferrer;
}

// =============================================================================
// Perwerde (Education) Pezpallet Configuration
// =============================================================================

parameter_types! {
	pub const MaxCourseNameLength: u32 = 128;
	pub const MaxCourseDescLength: u32 = 512;
	pub const MaxCourseLinkLength: u32 = 256;
	pub const MaxStudentsPerCourse: u32 = 1000;
	pub const MaxCoursesPerStudent: u32 = 50;
	pub const MaxPointsPerCourse: u32 = 1000;
}

/// Admin origin for Perwerde pezpallet that supports progressive decentralization
///
/// Yetki devri sırası:
/// 1. Root (Sudo) - Başlangıç aşaması
/// 2. Council (1/2 çoğunluk) - Seçimler sonrası
/// 3. Serok atayabilir - Cumhurbaşkanlığı yetkisi
///
/// Bu origin AccountId döndürür (kurs sahibi olarak kullanılır)
pub struct PerwerdeAdminOrigin;
impl pezframe_support::traits::EnsureOrigin<RuntimeOrigin> for PerwerdeAdminOrigin {
	type Success = AccountId;
	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		// 1. Root origin kontrolü
		if let Ok(_) = pezframe_system::ensure_root(o.clone()) {
			// Root için varsayılan admin hesabı
			return Ok(pezsp_keyring::Sr25519Keyring::Alice.to_account_id());
		}

		// 2. Council kontrolü (1/2'den fazla oy)
		if let Ok(_) = pezpallet_collective::EnsureProportionMoreThan::<
			AccountId,
			CouncilCollective,
			1,
			2,
		>::try_origin(o.clone())
		{
			// Komisyon için varsayılan admin hesabı
			return Ok(pezsp_keyring::Sr25519Keyring::Alice.to_account_id());
		}

		// 3. Serok (Cumhurbaşkanı) kontrolü
		if let Ok(serok) = pezpallet_welati::EnsureSerok::<Runtime>::try_origin(o.clone()) {
			return Ok(serok);
		}

		Err(o)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

impl pezpallet_perwerde::Config for Runtime {
	type AdminOrigin = PerwerdeAdminOrigin;
	type WeightInfo = pezpallet_perwerde::weights::BizinikiwiWeight<Runtime>;
	type MaxCourseNameLength = MaxCourseNameLength;
	type MaxCourseDescLength = MaxCourseDescLength;
	type MaxCourseLinkLength = MaxCourseLinkLength;
	type MaxStudentsPerCourse = MaxStudentsPerCourse;
	type MaxCoursesPerStudent = MaxCoursesPerStudent;
	type MaxPointsPerCourse = MaxPointsPerCourse;
	type TrustScoreUpdater = TrustScoreNotifier;
}

// =============================================================================
// Referral Pezpallet Configuration
// =============================================================================

parameter_types! {
	/// Default referrer account - Founder address
	/// SS58: 5CyuFfbF95rzBxru7c9yEsX4XmQXUxpLUcbj9RLg9K1cGiiF
	pub DefaultReferrer: AccountId = AccountId::from([
		0x28, 0x92, 0x5e, 0xd8, 0xb4, 0xc0, 0xc9, 0x54,
		0x02, 0xb3, 0x15, 0x63, 0x25, 0x1f, 0xd3, 0x18,
		0x41, 0x43, 0x51, 0x11, 0x4b, 0x1c, 0x77, 0x97,
		0xee, 0x78, 0x86, 0x66, 0xd2, 0x7d, 0x63, 0x05,
	]);
	/// Penalty per revocation (trust score reduction)
	pub const PenaltyPerRevocation: u32 = 10;
}

impl pezpallet_referral::Config for Runtime {
	type WeightInfo = pezpallet_referral::weights::BizinikiwiWeight<Runtime>;
	type DefaultReferrer = DefaultReferrer;
	type PenaltyPerRevocation = PenaltyPerRevocation;
	type TrustScoreUpdater = TrustScoreNotifier;
}

// =============================================================================
// NFTs Pezpallet Configuration (required by Tiki)
// =============================================================================

parameter_types! {
	pub const NftsCollectionDeposit: Balance = 10 * UNITS;
	pub const NftsItemDeposit: Balance = UNITS / 100;
	pub const NftsMetadataDepositBase: Balance = deposit(1, 129);
	pub const NftsAttributeDepositBase: Balance = deposit(1, 0);
	pub const NftsDepositPerByte: Balance = deposit(0, 1);
	pub NftsPalletFeatures: pezpallet_nfts::PalletFeatures = pezpallet_nfts::PalletFeatures::all_enabled();
	pub const NftsMaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;
	pub const NftsMaxAttributesPerCall: u32 = 10;
}

impl pezpallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = EnsureRoot<AccountId>;
	type CreateOrigin =
		pezframe_support::traits::AsEnsureOriginWithArg<pezframe_system::EnsureSigned<AccountId>>;
	type Locker = ();
	type CollectionDeposit = NftsCollectionDeposit;
	type ItemDeposit = NftsItemDeposit;
	type MetadataDepositBase = NftsMetadataDepositBase;
	type AttributeDepositBase = NftsAttributeDepositBase;
	type DepositPerByte = NftsDepositPerByte;
	type StringLimit = ConstU32<256>;
	type KeyLimit = ConstU32<64>;
	type ValueLimit = ConstU32<256>;
	type ApprovalsLimit = ConstU32<20>;
	type ItemAttributesApprovalsLimit = ConstU32<30>;
	type MaxTips = ConstU32<10>;
	type MaxDeadlineDuration = NftsMaxDeadlineDuration;
	type MaxAttributesPerCall = NftsMaxAttributesPerCall;
	type Features = NftsPalletFeatures;
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as pezsp_runtime::traits::Verify>::Signer;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type WeightInfo = pezpallet_nfts::weights::BizinikiwiWeight<Runtime>;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

// =============================================================================
// Tiki (Role NFT) Pezpallet Configuration
// =============================================================================

parameter_types! {
	/// Collection ID for Tiki (Role) NFTs - Collection 0 is reserved for citizenship/roles
	pub const TikiCollectionId: u32 = 0;
	/// Maximum number of roles a user can hold
	pub const MaxTikisPerUser: u32 = 20;
}

impl pezpallet_tiki::Config for Runtime {
	// Kademeli yetki devri: Root → Teknik Komisyon
	// NFT/Rol yönetimi için Teknik Komisyon yetkili
	type AdminOrigin = crate::RootOrTechnicalCommittee;
	type WeightInfo = pezpallet_tiki::weights::BizinikiwiWeight<Runtime>;
	type TikiCollectionId = TikiCollectionId;
	type MaxTikisPerUser = MaxTikisPerUser;
	type Tiki = pezpallet_tiki::Tiki;
	type TrustScoreUpdater = TrustScoreNotifier;
}

// =============================================================================
// Staking Score Pezpallet Configuration
// =============================================================================

parameter_types! {
	/// Update interval for staking scores (blocks)
	pub const StakingScoreUpdateInterval: BlockNumber = HOURS;
}

/// Noter authority checker backed by the Tiki pallet.
/// Accounts holding the `Noter` tiki role can submit staking details.
pub struct TikiNoterChecker;
impl pezpallet_staking_score::NoterCheck<AccountId> for TikiNoterChecker {
	fn is_noter(who: &AccountId) -> bool {
		pezpallet_tiki::Pezpallet::<Runtime>::has_tiki(who, &pezpallet_tiki::Tiki::Noter)
	}
}

impl pezpallet_staking_score::Config for Runtime {
	type WeightInfo = pezpallet_staking_score::weights::BizinikiwiWeight<Runtime>;
	type Balance = Balance;
	type OnStakingUpdate = Trust;
	type NoterChecker = TikiNoterChecker;
}

// =============================================================================
// Collective Pezpallet Configuration (for governance)
// =============================================================================
//
// Pezkuwichain Komisyon Yapısı:
// - Council (Instance1): Genel Konsey - Ana yönetişim organı
//
// Ek komisyonlar (EducationCommittee, TechnicalCommittee, TreasuryCommittee)
// runtime upgrade ile eklenecek. Şu an Welati pezpallet'in EnsureSerok,
// EnsureParlementer ve EnsureDiwan origin'leri kullanılıyor.
//
// Bu komisyonlar başlangıçta Root (Sudo) tarafından yönetilir.
// Welati pezpallet'i aracılığıyla seçimler yapıldığında yetki devredilir.
// =============================================================================

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
	pub MaxProposalWeight: Weight = pezsp_runtime::Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

// Instance tanımları
pub type CouncilCollective = pezpallet_collective::Instance1;

/// Council (Genel Konsey) - Ana yönetişim organı
impl pezpallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pezpallet_collective::PrimeDefaultVote;
	type WeightInfo = pezpallet_collective::weights::BizinikiwiWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
	type DisapproveOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Consideration = ();
}

// =============================================================================
// Trust Score Pezpallet Configuration
// =============================================================================

parameter_types! {
	/// Base multiplier for trust score calculation
	pub const ScoreMultiplierBase: u128 = 10_000;
	/// Update interval for trust scores (roughly 1 day in blocks)
	pub const TrustUpdateInterval: BlockNumber = DAYS;
	/// Maximum batch size for trust score updates
	pub const TrustMaxBatchSize: u32 = 100;
}

/// Staking score source for Trust pezpallet
/// Uses the StakingScore pezpallet to get composite staking scores
pub struct StakingScoreSource;
impl pezpallet_trust::StakingScoreProvider<AccountId, BlockNumber> for StakingScoreSource {
	fn get_staking_score(who: &AccountId) -> (pezpallet_staking_score::RawScore, BlockNumber) {
		// Delegate to StakingScore pezpallet
		<StakingScore as pezpallet_staking_score::StakingScoreProvider<AccountId, BlockNumber>>::get_staking_score(who)
	}
}

/// Referral score source for Trust pezpallet
/// Uses the referral pallet's tiered scoring with penalty system
pub struct ReferralScoreSource;
impl pezpallet_trust::ReferralScoreProvider<AccountId> for ReferralScoreSource {
	fn get_referral_score(who: &AccountId) -> u32 {
		<Referral as pezpallet_referral::types::ReferralScoreProvider<AccountId>>::get_referral_score(who)
	}
}

/// Perwerde (education) score source for Trust pezpallet
/// Sums completed course points from the Perwerde pallet
pub struct PerwerdeScoreSource;
impl pezpallet_trust::PerwerdeScoreProvider<AccountId> for PerwerdeScoreSource {
	fn get_perwerde_score(who: &AccountId) -> u32 {
		pezpallet_perwerde::Pezpallet::<Runtime>::get_perwerde_score(who)
	}
}

/// Tiki score source for Trust pezpallet
pub struct TikiScoreSource;
impl pezpallet_trust::TikiScoreProvider<AccountId> for TikiScoreSource {
	fn get_tiki_score(who: &AccountId) -> u32 {
		<Tiki as pezpallet_tiki::TikiScoreProvider<AccountId>>::get_tiki_score(who)
	}
}

/// Citizenship status source for Trust pezpallet - uses real IdentityKyc
#[cfg(not(feature = "runtime-benchmarks"))]
pub struct CitizenshipSource;
#[cfg(not(feature = "runtime-benchmarks"))]
impl pezpallet_trust::CitizenshipStatusProvider<AccountId> for CitizenshipSource {
	fn is_citizen(who: &AccountId) -> bool {
		IdentityKyc::is_citizen(who)
	}
}

/// Mock citizenship source for benchmarks - always returns true
#[cfg(feature = "runtime-benchmarks")]
pub struct CitizenshipSource;
#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_trust::CitizenshipStatusProvider<AccountId> for CitizenshipSource {
	fn is_citizen(_who: &AccountId) -> bool {
		// Always return true for benchmark purposes
		true
	}
}

impl pezpallet_trust::Config for Runtime {
	type WeightInfo = pezpallet_trust::weights::BizinikiwiWeight<Runtime>;
	type Score = u128;
	type ScoreMultiplierBase = ScoreMultiplierBase;
	type UpdateInterval = TrustUpdateInterval;
	type MaxBatchSize = TrustMaxBatchSize;
	type StakingScoreSource = StakingScoreSource;
	type ReferralScoreSource = ReferralScoreSource;
	type PerwerdeScoreSource = PerwerdeScoreSource;
	type TikiScoreSource = TikiScoreSource;
	type CitizenshipSource = CitizenshipSource;
}

// =============================================================================
// Messaging Pezpallet Configuration (PEZkurd-P2Pmessage)
// =============================================================================

/// Messaging citizenship checker — bridges to IdentityKyc pallet
#[cfg(not(feature = "runtime-benchmarks"))]
pub struct MessagingCitizenshipChecker;
#[cfg(not(feature = "runtime-benchmarks"))]
impl pezpallet_messaging::types::CitizenshipChecker<AccountId> for MessagingCitizenshipChecker {
	fn is_citizen(who: &AccountId) -> bool {
		IdentityKyc::is_citizen(who)
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct MessagingCitizenshipChecker;
#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_messaging::types::CitizenshipChecker<AccountId> for MessagingCitizenshipChecker {
	fn is_citizen(_who: &AccountId) -> bool {
		true
	}
}

/// Messaging trust score checker — bridges to Trust pallet
#[cfg(not(feature = "runtime-benchmarks"))]
pub struct MessagingTrustScoreChecker;
#[cfg(not(feature = "runtime-benchmarks"))]
impl pezpallet_messaging::types::TrustScoreChecker<AccountId> for MessagingTrustScoreChecker {
	fn trust_score_of(who: &AccountId) -> u32 {
		// Trust pallet returns u128, we cap at u32::MAX for messaging
		let score: u128 = Trust::trust_score_of(who);
		score.min(u32::MAX as u128) as u32
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct MessagingTrustScoreChecker;
#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_messaging::types::TrustScoreChecker<AccountId> for MessagingTrustScoreChecker {
	fn trust_score_of(_who: &AccountId) -> u32 {
		100 // High trust for benchmarks
	}
}

parameter_types! {
	/// Minimum trust score to use messaging (20 out of ~10000 scale)
	pub const MessagingMinTrustScore: u32 = 20;
	/// Maximum encrypted payload per message (512 bytes)
	pub const MessagingMaxMessageSize: u32 = 512;
	/// Maximum messages in inbox per era per recipient
	pub const MessagingMaxInboxSize: u32 = 50;
	/// Maximum messages a citizen can send per era
	pub const MessagingMaxMessagesPerEra: u32 = 50;
	/// Era length: 3600 blocks = ~6 hours at 6s/block on People Chain
	pub const MessagingEraLength: BlockNumber = 6 * HOURS;
}

impl pezpallet_messaging::Config for Runtime {
	type WeightInfo = pezpallet_messaging::weights::BizinikiwiWeight<Runtime>;
	type CitizenshipChecker = MessagingCitizenshipChecker;
	type TrustScoreChecker = MessagingTrustScoreChecker;
	type MinTrustScore = MessagingMinTrustScore;
	type MaxMessageSize = MessagingMaxMessageSize;
	type MaxInboxSize = MessagingMaxInboxSize;
	type MaxMessagesPerEra = MessagingMaxMessagesPerEra;
	type EraLength = MessagingEraLength;
}

// =============================================================================
// Assets Pezpallet Configuration (required by PEZ Rewards)
// =============================================================================

parameter_types! {
	pub const AssetsAssetDeposit: Balance = 10 * UNITS;
	pub const AssetsAssetAccountDeposit: Balance = deposit(1, 16);
	pub const AssetsApprovalDeposit: Balance = deposit(1, 20);
	pub const AssetsStringLimit: u32 = 50;
	pub const AssetsMetadataDepositBase: Balance = deposit(1, 68);
	pub const AssetsMetadataDepositPerByte: Balance = deposit(0, 1);
}

impl pezpallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin =
		pezframe_support::traits::AsEnsureOriginWithArg<pezframe_system::EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetsAssetDeposit;
	type AssetAccountDeposit = AssetsAssetAccountDeposit;
	type MetadataDepositBase = AssetsMetadataDepositBase;
	type MetadataDepositPerByte = AssetsMetadataDepositPerByte;
	type ApprovalDeposit = AssetsApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = weights::pezpallet_assets::WeightInfo<Runtime>;
	type CallbackHandle = ();
	type RemoveItemsLimit = ConstU32<1000>;
	type ReserveData = ();
	type Holder = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

// =============================================================================
// Scheduler Pezpallet Configuration (required by Welati & Democracy)
// =============================================================================

parameter_types! {
	pub MaximumSchedulerWeight: Weight = pezsp_runtime::Perbill::from_percent(80) * RuntimeBlockWeights::get().max_block;
}

impl pezpallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = pezpallet_scheduler::weights::BizinikiwiWeight<Runtime>;
	type OriginPrivilegeCmp = pezframe_support::traits::EqualPrivilegeOnly;
	type Preimages = ();
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

// =============================================================================
// Democracy Pezpallet Configuration (required by Welati)
// =============================================================================

parameter_types! {
	pub const DemocracyLaunchPeriod: BlockNumber = 7 * DAYS;
	pub const DemocracyVotingPeriod: BlockNumber = 7 * DAYS;
	pub const DemocracyFastTrackVotingPeriod: BlockNumber = HOURS;
	pub const DemocracyMinimumDeposit: Balance = 10 * UNITS;
	pub const DemocracyEnactmentPeriod: BlockNumber = DAYS;
	pub const DemocracyCooloffPeriod: BlockNumber = 7 * DAYS;
	pub const DemocracyMaxVotes: u32 = 100;
	pub const DemocracyMaxProposals: u32 = 100;
}

impl pezpallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = DemocracyEnactmentPeriod;
	type LaunchPeriod = DemocracyLaunchPeriod;
	type VotingPeriod = DemocracyVotingPeriod;
	type VoteLockingPeriod = DemocracyEnactmentPeriod;
	type MinimumDeposit = DemocracyMinimumDeposit;
	type InstantAllowed = ConstBool<true>;
	type FastTrackVotingPeriod = DemocracyFastTrackVotingPeriod;
	type CooloffPeriod = DemocracyCooloffPeriod;
	type MaxVotes = DemocracyMaxVotes;
	type MaxProposals = DemocracyMaxProposals;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
	type ExternalOrigin = EnsureRoot<AccountId>;
	type ExternalMajorityOrigin = EnsureRoot<AccountId>;
	type ExternalDefaultOrigin = EnsureRoot<AccountId>;
	type FastTrackOrigin = EnsureRoot<AccountId>;
	type InstantOrigin = EnsureRoot<AccountId>;
	type CancellationOrigin = EnsureRoot<AccountId>;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	type CancelProposalOrigin = EnsureRoot<AccountId>;
	type VetoOrigin = pezframe_system::EnsureSigned<AccountId>;
	type Slash = ();
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type Preimages = ();
	type SubmitOrigin = pezframe_system::EnsureSigned<AccountId>;
	type WeightInfo = pezpallet_democracy::weights::BizinikiwiWeight<Runtime>;
}

// =============================================================================
// Elections Phragmen Pezpallet Configuration (required by Welati)
// =============================================================================

parameter_types! {
	pub const ElectionsCandidacyBond: Balance = 10 * UNITS;
	pub const ElectionsVotingBondBase: Balance = UNITS;
	pub const ElectionsVotingBondFactor: Balance = UNITS / 10;
	pub const ElectionsDesiredMembers: u32 = 13;
	pub const ElectionsDesiredRunnersUp: u32 = 7;
	pub const ElectionsTermDuration: BlockNumber = 7 * DAYS;
	pub const ElectionsPalletId: pezframe_support::traits::LockIdentifier = *b"phrelect";
}

impl pezpallet_elections_phragmen::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type PalletId = ElectionsPalletId;
	type ChangeMembers = ();
	type InitializeMembers = ();
	type CurrencyToVote = pezsp_staking::currency_to_vote::U128CurrencyToVote;
	type CandidacyBond = ElectionsCandidacyBond;
	type VotingBondBase = ElectionsVotingBondBase;
	type VotingBondFactor = ElectionsVotingBondFactor;
	type LoserCandidate = ();
	type KickedMember = ();
	type DesiredMembers = ElectionsDesiredMembers;
	type DesiredRunnersUp = ElectionsDesiredRunnersUp;
	type TermDuration = ElectionsTermDuration;
	type MaxCandidates = ConstU32<64>;
	type MaxVoters = ConstU32<512>;
	type MaxVotesPerVoter = ConstU32<16>;
	type WeightInfo = pezpallet_elections_phragmen::weights::BizinikiwiWeight<Runtime>;
}

// =============================================================================
// Welati (Governance) Pezpallet Configuration
// =============================================================================

parameter_types! {
	/// Parliament size (201 members like Kurdistan Parliament)
	pub const WelatiParliamentSize: u32 = 201;
	/// Diwan council size
	pub const WelatiDiwanSize: u32 = 50;
	/// Election period (~4 months = ~120 days)
	pub const WelatiElectionPeriod: BlockNumber = 120 * DAYS;
	/// Candidacy period (~3 days)
	pub const WelatiCandidacyPeriod: BlockNumber = 3 * DAYS;
	/// Campaign period (~10 days)
	pub const WelatiCampaignPeriod: BlockNumber = 10 * DAYS;
	/// Number of electoral districts
	pub const WelatiElectoralDistricts: u32 = 10;
	/// Candidacy deposit (100 PEZ)
	pub const WelatiCandidacyDeposit: u128 = 100 * UNITS as u128;
	/// Presidential endorsements required
	pub const WelatiPresidentialEndorsements: u32 = 1000;
	/// Parliamentary endorsements required
	pub const WelatiParliamentaryEndorsements: u32 = 100;
	/// Maximum endorsers per candidate registration
	pub const WelatiMaxEndorsers: u32 = 1000;
}

/// Randomness source for elections (using timestamp for now)
pub struct TimestampRandomness;
impl pezframe_support::traits::Randomness<Hash, BlockNumber> for TimestampRandomness {
	fn random(subject: &[u8]) -> (Hash, BlockNumber) {
		let block_number = pezframe_system::Pezpallet::<Runtime>::block_number();
		let timestamp = pezpallet_timestamp::Pezpallet::<Runtime>::get();
		let mut data = subject.to_vec();
		data.extend_from_slice(&timestamp.to_le_bytes());
		data.extend_from_slice(&block_number.to_le_bytes());
		let hash = pezsp_core::hashing::blake2_256(&data);
		(Hash::from(hash), block_number)
	}
}

/// Citizen count provider for Welati
pub struct WelatiCitizenSource;
impl pezpallet_welati::CitizenInfo for WelatiCitizenSource {
	fn citizen_count() -> u32 {
		IdentityKyc::citizen_count()
	}
}

/// Trust score source for Welati
pub struct WelatiTrustScoreSource;
impl pezpallet_trust::TrustScoreProvider<AccountId> for WelatiTrustScoreSource {
	fn trust_score_of(who: &AccountId) -> u128 {
		Trust::trust_score_of(who)
	}
}

/// Tiki score source for Welati
pub struct WelatiTikiScoreSource;
impl pezpallet_tiki::TikiScoreProvider<AccountId> for WelatiTikiScoreSource {
	fn get_tiki_score(who: &AccountId) -> u32 {
		<Tiki as pezpallet_tiki::TikiScoreProvider<AccountId>>::get_tiki_score(who)
	}
}

impl pezpallet_welati::Config for Runtime {
	type WeightInfo = ();
	type Randomness = TimestampRandomness;
	type RuntimeCall = RuntimeCall;
	type TrustScoreSource = WelatiTrustScoreSource;
	type TikiSource = WelatiTikiScoreSource;
	type CitizenSource = WelatiCitizenSource;
	type KycSource = IdentityKyc;
	type ParliamentSize = WelatiParliamentSize;
	type DiwanSize = WelatiDiwanSize;
	type ElectionPeriod = WelatiElectionPeriod;
	type CandidacyPeriod = WelatiCandidacyPeriod;
	type CampaignPeriod = WelatiCampaignPeriod;
	type ElectoralDistricts = WelatiElectoralDistricts;
	type CandidacyDeposit = WelatiCandidacyDeposit;
	type PresidentialEndorsements = WelatiPresidentialEndorsements;
	type ParliamentaryEndorsements = WelatiParliamentaryEndorsements;
	type NativeCurrency = Balances;
	type MaxEndorsers = WelatiMaxEndorsers;
}

// =============================================================================
// PEZ Rewards Pezpallet Configuration
// =============================================================================

parameter_types! {
	/// PEZ Asset ID
	pub const PezAssetId: u32 = 1;
	/// Incentive Pot Pezpallet ID
	pub const IncentivePotId: pezframe_support::PalletId = pezframe_support::PalletId(*b"pez/incv");
	/// Clawback recipient (QaziMuhammed account - placeholder)
	pub ClawbackRecipient: AccountId = pezsp_keyring::Sr25519Keyring::Bob.to_account_id();
}

/// Trust score source for PEZ Rewards
pub struct PezRewardsTrustScoreSource;
impl pezpallet_trust::TrustScoreProvider<AccountId> for PezRewardsTrustScoreSource {
	fn trust_score_of(who: &AccountId) -> u128 {
		Trust::trust_score_of(who)
	}
}

impl pezpallet_pez_rewards::Config for Runtime {
	type Assets = Assets;
	type PezAssetId = PezAssetId;
	type WeightInfo = pezpallet_pez_rewards::weights::BizinikiwiWeight<Runtime>;
	type TrustScoreSource = PezRewardsTrustScoreSource;
	type IncentivePotId = IncentivePotId;
	type ClawbackRecipient = ClawbackRecipient;
	// Kademeli yetki devri: Root → Hazine Komisyonu
	// PEZ ödül dağıtımı için Hazine Komisyonu yetkili
	type ForceOrigin = crate::RootOrTreasuryCommittee;
	type CollectionId = u32;
	type ItemId = u32;
}

// =============================================================================
// Recovery Pezpallet Configuration
// =============================================================================

parameter_types! {
	pub const ConfigDepositBase: Balance = 5 * UNITS;
	pub const FriendDepositFactor: Balance = 50 * CENTS;
	pub const MaxFriends: u16 = 9;
	pub const RecoveryDeposit: Balance = 5 * UNITS;
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

// =============================================================================
// Society Pezpallet Configuration
// =============================================================================

parameter_types! {
	pub const GraceStrikes: u32 = 10;
	pub const SocietyVotingPeriod: BlockNumber = 80 * HOURS;
	pub const ClaimPeriod: BlockNumber = 80 * HOURS;
	pub const PeriodSpend: Balance = 500 * UNITS;
	pub const MaxLockDuration: BlockNumber = 36 * 30 * DAYS;
	pub const ChallengePeriod: BlockNumber = 7 * DAYS;
	pub const MaxPayouts: u32 = 10;
	pub const MaxBids: u32 = 10;
	pub const SocietyPalletId: pezframe_support::PalletId = pezframe_support::PalletId(*b"py/socie");
}

impl pezpallet_society::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = SocietyPalletId;
	type Currency = Balances;
	type Randomness = TimestampRandomness;
	type GraceStrikes = GraceStrikes;
	type PeriodSpend = PeriodSpend;
	type VotingPeriod = SocietyVotingPeriod;
	type ClaimPeriod = ClaimPeriod;
	type MaxLockDuration = MaxLockDuration;
	type FounderSetOrigin = EnsureRoot<AccountId>;
	type ChallengePeriod = ChallengePeriod;
	type MaxPayouts = MaxPayouts;
	type MaxBids = MaxBids;
	type BlockNumberProvider = System;
	type WeightInfo = ();
}

// =============================================================================
// Vesting Pezpallet Configuration
// =============================================================================

parameter_types! {
	pub const MinVestedTransfer: Balance = UNITS;
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
