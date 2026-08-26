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
use pezframe_support::traits::EitherOfDiverse;
use pezframe_support::traits::Equals;
use pezframe_support::{
	parameter_types,
	traits::{
		fungible::HoldConsideration, tokens::imbalance::ResolveTo, ConstU32, LinearStoragePrice,
		WithdrawReasons,
	},
	weights::Weight,
	CloneNoBound, DebugNoBound, EqNoBound, PartialEqNoBound,
};
use pezframe_system::EnsureRoot;
use pezpallet_identity::{Data, IdentityInformationProvider};
use pezpallet_xcm::EnsureXcm;
use pezsp_runtime::traits::{AccountIdConversion, ConvertInto, Verify};
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
	DebugNoBound,
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
	/// Twenty-five citizens brought in: the head of an association.
	pub const AssociationHeadThreshold: u32 = 25;
	/// Fifty: a community moderator.
	pub const CommunityModeratorThreshold: u32 = 50;
	/// Three months. After this the founder may approve an application the referrer has left
	/// waiting -- the applicant is not punished for somebody else's silence.
	pub const ReferralFallbackPeriod: BlockNumber = 90 * DAYS;
}

// OnKycApproved hook → Delegates to Referral pallet for referral confirmation
// Referral pallet implements OnKycApproved trait directly and also triggers TrustScoreUpdater
// OnCitizenshipRevoked → both the referral record and the trust register. The referral side
// applies the penalty to whoever vouched; the trust side removes the standing, which would
// otherwise be frozen at its last value for ever and keep counting towards reward shares.
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
	// Authority hands over in stages: Root, then the Diwan, then the technical body.
	// Citizenship is the court's to decide.
	// The court, and root while sudo exists. Losing citizenship takes every tiki, every
	// office and the vote with it, so it cannot be a thing a council majority does -- the
	// comment above this line always said the Diwan decided citizenship; the type did not.
	type GovernanceOrigin = crate::RootOrDiwan;
	type WeightInfo = pezpallet_identity_kyc::weights::BizinikiwiWeight<Runtime>;
	type OnKycApproved = Referral;
	// Losing citizenship concerns both: the referral record has a penalty to apply, and the
	// trust score has to stop existing rather than being left behind at its last value.
	type OnCitizenshipRevoked = (Referral, Trust);
	type CitizenNftProvider = Tiki;
	type KycApplicationDeposit = KycApplicationDeposit;
	type MaxStringLength = MaxStringLength;
	type MaxCidLength = MaxCidLength;
	type DefaultReferrer = DefaultReferrer;
	type ReferralFallbackPeriod = ReferralFallbackPeriod;
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
	/// A course runs for at least three months and at most a year. Shorter would make it a
	/// way of printing standing; longer and nobody would ever be held to closing it.
	pub const MinCourseDuration: BlockNumber = 90 * DAYS;
	pub const MaxCourseDuration: BlockNumber = 365 * DAYS;
	/// Five teachers ratify a course's results.
	pub const RatificationsRequired: u32 = 5;
	/// How many teachers the minister may seed to get the examining boards started.
	pub const MaxHonoraryMamoste: u32 = 100;
	/// Fifty completed courses count towards standing; study past that is free and unweighted.
	pub const RewardedCourseLimit: u32 = 50;
	/// No title on one course, however large.
	pub const MinCoursesForRole: u32 = 5;
	/// Points for each title. The trust bonuses -- Rewsenbîr 40, Mamoste 70, Axa 250 -- set
	/// the order; these set the distance.
	pub const RewsenbirThreshold: u32 = 5_000;
	pub const MamosteThreshold: u32 = 15_000;
	pub const AxaThreshold: u32 = 40_000;
	/// Pezpallet ID used to derive a keyless sovereign "admin" account for courses created
	/// via Root or Council (i.e. no natural signer/AccountId exists for those origins).
	/// This is NOT an sr25519 keyring account: nobody holds (or can hold) a private key for
	/// a `PalletId`-derived account, so it can never be used to forge `complete_course` /
	/// `archive_course` calls the way a well-known dev seed (e.g. `//Alice`) could.
	pub const PerwerdeAdminPotId: pezframe_support::PalletId = pezframe_support::PalletId(*b"pez/prwd");
}

/// Admin origin for Perwerde pezpallet that supports progressive decentralization
///
/// The order authority is handed over in:
/// 1. Root (sudo), at the start.
/// 2. The Council at half, once elections have run.
/// 3. The President, by appointment.
///
/// Returns an `AccountId`, used as the owner of the course.
///
/// SECURITY: Root and Council origins carry no real signer/AccountId of their own. This
/// origin previously stood in with the well-known dev keypair `//Alice`, whose private key
/// is public knowledge — anyone could derive it and then sign `complete_course` as the
/// "owner" of any Root/Council-created course, forging arbitrary trust-affecting course
/// completions. It now resolves to a `PalletId`-derived sovereign account instead, which has
/// no private key at all, so it can authenticate privileged *creation/archival* (which also
/// goes through `AdminOrigin`) but can never be used to sign a `complete_course` extrinsic.
/// Only Serok-originated courses (which have a genuine on-chain account) can currently be
/// completed via the owner-signature path; enabling completions for Root/Council-created
/// courses requires a follow-up that lets the privileged caller nominate a real owner
/// account explicitly (e.g. an added `owner` parameter on `create_course`).
pub struct PerwerdeAdminOrigin;
impl pezframe_support::traits::EnsureOrigin<RuntimeOrigin> for PerwerdeAdminOrigin {
	type Success = AccountId;
	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		// 1. Root origin kontrolü
		if let Ok(_) = pezframe_system::ensure_root(o.clone()) {
			// A keyless sovereign account for Root: nobody holds a private key to it.
			return Ok(PerwerdeAdminPotId::get().into_account_truncating());
		}

		// 2. Council kontrolü (1/2'den fazla oy)
		if let Ok(_) = pezpallet_collective::EnsureProportionMoreThan::<
			AccountId,
			CouncilCollective,
			1,
			2,
		>::try_origin(o.clone())
		{
			// A keyless sovereign account for the body: nobody holds a private key to it.
			return Ok(PerwerdeAdminPotId::get().into_account_truncating());
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
	type MaxPointsPerCourse = MaxPointsPerCourse;
	type TrustScoreUpdater = TrustScoreNotifier;
	// The education minister seeds the examiner corps and brings fraud to the court; the
	// court annuls. Neither of them grades anybody.
	type EducationMinisterOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pezpallet_tiki::ensure::EnsureTiki<Runtime, EducationMinisterRole>,
	>;
	type FraudOrigin = crate::RootOrDiwan;
	type EarnedRoles = Tiki;
	type TikiSource = Tiki;
	type MinCourseDuration = MinCourseDuration;
	type MaxCourseDuration = MaxCourseDuration;
	type RatificationsRequired = RatificationsRequired;
	type MaxHonoraryMamoste = MaxHonoraryMamoste;
	type RewardedCourseLimit = RewardedCourseLimit;
	type MinCoursesForRole = MinCoursesForRole;
	type RewsenbirThreshold = RewsenbirThreshold;
	type MamosteThreshold = MamosteThreshold;
	type AxaThreshold = AxaThreshold;
}

/// The education portfolio, for origin checks.
pub struct EducationMinisterRole;
impl pezpallet_tiki::ensure::GetTiki for EducationMinisterRole {
	fn tiki() -> pezpallet_tiki::Tiki {
		pezpallet_tiki::Tiki::WezireBelaw
	}
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
	type EarnedRoles = Tiki;
	// How many people someone has to have brought in. Policy, not code: the chain spec sets
	// them and governance can change them without touching the pallet.
	type AssociationHeadThreshold = AssociationHeadThreshold;
	type CommunityModeratorThreshold = CommunityModeratorThreshold;
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
	// Authority hands over in stages: Root, then the technical body, which is what
	// administers NFTs and roles.
	type AdminOrigin = crate::RootOrSerokOrCouncil;
	// Elected/Earned roles (Serok, SerokiMeclise, Parlementer, Axa, ...) are meant to
	// carry evidence of a real election/exam, not just committee say-so. Until a
	// dedicated voting/exam pezpallet exists to escalate to Root on their behalf, require
	// full Root here — deliberately stricter than the Technical Committee threshold used
	// for ordinary admin appointment via `AdminOrigin`.
	type ElectedRoleOrigin = EnsureRoot<AccountId>;
	type EarnedRoleOrigin = EnsureRoot<AccountId>;
	// An office that came from a ballot is not removable by the bodies it exists to check.
	// The court is; `RootOrDiwan` was defined months ago and had no user until now.
	type ImpeachmentOrigin = crate::RootOrDiwan;
	// Honorary citizenship is the head of government's to confer. Root stands alongside only
	// while sudo exists; removing sudo means deleting the first arm here.
	type HonoraryCitizenshipOrigin =
		EitherOfDiverse<EnsureRoot<AccountId>, pezpallet_tiki::ensure::EnsureSerokWeziran<Runtime>>;
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

	#[cfg(feature = "runtime-benchmarks")]
	fn make_noter(who: &AccountId) {
		pezpallet_tiki::UserTikis::<Runtime>::mutate(who, |tikis| {
			let _ = tikis.try_push(pezpallet_tiki::Tiki::Noter);
		});
	}
}

parameter_types! {
	// Real-world analogy: a notary's bond/insurance. An account must hold the
	// Noter tiki *and* post this bond via `register_as_noter` before its
	// submissions are accepted. Slashable by `RootOrDiwanOrTechnical` if a
	// disputed submission is confirmed fraudulent. The tiki role itself
	// supports any number of independently registered accounts — this is not
	// a single hardcoded noter, the same way a state can authorize any number
	// of notaries.
	pub const StakingOracleGracePeriod: BlockNumber = DAYS;
	pub const NoterBondAmount: Balance = 50_000 * UNITS;
	// Real-world analogy: a notarized document's recording/contestability
	// period — a noter-signed submission only takes effect after this many
	// blocks unchallenged. Root/XCM-Transact submissions (chain-authenticated,
	// not a personal key) are exempt. One real hour, using this runtime's
	// actual `HOURS` constant (`testnet_teyrchains_constants::pezkuwichain`,
	// derived from the real 6s slot duration this runtime is configured
	// with) — matches staking-score's own internal `HOUR_IN_BLOCKS`.
	pub const StakingNoterDisputeWindow: BlockNumber = HOURS;
}

impl pezpallet_staking_score::Config for Runtime {
	type WeightInfo = pezpallet_staking_score::weights::BizinikiwiWeight<Runtime>;
	type Balance = Balance;
	type OnStakingUpdate = Trust;
	type NoterChecker = TikiNoterChecker;
	type Currency = Balances;
	type NoterBondAmount = NoterBondAmount;
	type DisputeWindow = StakingNoterDisputeWindow;
	// Lightweight: any single Council member can freeze a suspicious pending
	// submission for review — mirrors the existing VetoOrigin pattern below.
	type DisputeOrigin = pezpallet_collective::EnsureMember<AccountId, CouncilCollective>;
	// Deliberately stronger: slashing a noter's bond needs an actual
	// governance decision, not one member's word.
	type SlashOrigin = crate::RootOrDiwanOrTechnical;
	// How much of the gap between opting in and the data landing the user is not charged
	// for. A day covers a slow bot cycle and the dispute window it then waits out; past
	// that, time is only credited for a stake that has actually existed.
	type OracleGracePeriod = StakingOracleGracePeriod;
	type SlashDestination = RelayTreasuryAccount;
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

// Collective instances
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

/// The Diwan's bench, as a body that votes.
///
/// Membership is not set here: `pezpallet-welati` decides who sits on the court -- six the
/// house elects, five the President appoints -- and pushes the roster in. This instance
/// exists so that the court can *decide*, which a court has to do as a body. Before it, the
/// four powers that answer to the Diwan accepted any single member's signature, so eleven
/// judges were eleven separate keys over citizenship, impeachment and slashing.
///
/// `SetMembersOrigin` is root because the ordinary path is welati's, not a call.
/// Parliament, as a body that can resolve.
///
/// The house is elected by `welati` and mirrored here, exactly as the bench is: one writer,
/// one direction, `set_members` called with the whole roster so the collective cannot hold
/// somebody the register has removed.
///
/// It exists so that "Parliament resolved" is something an origin can say. The alias that
/// used to carry that name accepted a single member -- a name promising a body and a check
/// giving one person -- and nothing used it, which is the only reason it was harmless.
pub type ParliamentCollective = pezpallet_collective::Instance3;

parameter_types! {
	pub const ParliamentMotionDuration: BlockNumber = 7 * DAYS;
	pub const ParliamentMaxProposals: u32 = 100;
}

pub struct ParliamentRoster;
impl pezpallet_welati::HouseRoster<AccountId> for ParliamentRoster {
	fn set_members(members: alloc::vec::Vec<AccountId>) {
		let mut sorted = members;
		sorted.sort();
		sorted.dedup();
		let prime = sorted.first().cloned();
		let _ = pezpallet_collective::Pezpallet::<Runtime, ParliamentCollective>::set_members(
			pezframe_system::RawOrigin::Root.into(),
			sorted,
			prime,
			WelatiParliamentSize::get(),
		);
	}
}

impl pezpallet_collective::Config<ParliamentCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = ParliamentMotionDuration;
	type MaxProposals = ParliamentMaxProposals;
	type MaxMembers = WelatiParliamentSize;
	// A member who does not vote is not counted as agreeing.
	type DefaultVote = pezpallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type WeightInfo = pezpallet_collective::weights::BizinikiwiWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
	type DisapproveOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Consideration = ();
}

pub type DiwanCollective = pezpallet_collective::Instance2;

parameter_types! {
	pub const DiwanMotionDuration: BlockNumber = 14 * DAYS;
	pub const DiwanMaxProposals: u32 = 50;
	pub const DiwanMaxMembers: u32 = 11;
}

/// Pushes welati's bench into the collective that votes it.
///
/// One writer, one direction. welati decides who sits; this only relays. `set_members` is
/// called with the whole roster each time, so the collective cannot hold somebody welati has
/// removed.
pub struct DiwanRoster;
impl pezpallet_welati::CourtRoster<AccountId> for DiwanRoster {
	fn set_members(members: alloc::vec::Vec<AccountId>) {
		let mut sorted = members;
		sorted.sort();
		sorted.dedup();
		let prime = sorted.first().cloned();
		let _ = pezpallet_collective::Pezpallet::<Runtime, DiwanCollective>::set_members(
			pezframe_system::RawOrigin::Root.into(),
			sorted,
			prime,
			DiwanMaxMembers::get(),
		);
	}
}

impl pezpallet_collective::Config<DiwanCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = DiwanMotionDuration;
	type MaxProposals = DiwanMaxProposals;
	type MaxMembers = DiwanMaxMembers;
	// A judge who does not vote is not counted as agreeing. On a court, silence is not
	// assent.
	type DefaultVote = pezpallet_collective::MoreThanMajorityThenPrimeDefaultVote;
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
	/// A perfect record scores this. Election thresholds read as a share of it -- the
	/// presidency asks for a quarter of everything a citizen could be.
	pub const TrustScoreScale: u32 = 1_000;
	/// Education 30, bringing citizens in 25, the offices and titles held 25, and the stake
	/// 20. The stake is already a gate -- nothing without it counts at all -- so its weight
	/// here is what having more of it is worth on top, and that is deliberately the smallest
	/// share: money is a condition of standing, not the largest part of it. On these numbers
	/// somebody with nothing but a maximum stake reaches two hundred, which is a seat in
	/// Parliament and the Speaker's chair, and not the presidency.
	pub const TrustPerwerdeWeight: u32 = 30;
	pub const TrustReferralWeight: u32 = 25;
	pub const TrustTikiWeight: u32 = 25;
	pub const TrustStakingWeight: u32 = 20;
	/// Update interval for trust scores (roughly 1 day in blocks)
	pub const TrustUpdateInterval: BlockNumber = DAYS;
	/// Maximum batch size for trust score updates
	pub const TrustMaxBatchSize: u32 = 100;
}

/// Staking score source for Trust pezpallet
/// Uses the StakingScore pezpallet to get composite staking scores
pub struct StakingScoreSource;
impl pezpallet_trust::StakingScoreProvider<AccountId, BlockNumber> for StakingScoreSource {
	fn max_score() -> pezpallet_staking_score::RawScore {
		pezpallet_staking_score::MAX_STAKING_SCORE
	}

	fn get_staking_score(who: &AccountId) -> (pezpallet_staking_score::RawScore, BlockNumber) {
		// Delegate to StakingScore pezpallet
		<StakingScore as pezpallet_staking_score::StakingScoreProvider<AccountId, BlockNumber>>::get_staking_score(who)
	}
}

/// Referral score source for Trust pezpallet
/// Uses the referral pallet's tiered scoring with penalty system
pub struct ReferralScoreSource;
impl pezpallet_trust::ReferralScoreProvider<AccountId> for ReferralScoreSource {
	fn max_score() -> u32 {
		pezpallet_referral::MAX_REFERRAL_SCORE
	}

	fn get_referral_score(who: &AccountId) -> u32 {
		<Referral as pezpallet_referral::types::ReferralScoreProvider<AccountId>>::get_referral_score(who)
	}
}

/// Perwerde (education) score source for Trust pezpallet
/// Sums completed course points from the Perwerde pallet
pub struct PerwerdeScoreSource;
impl pezpallet_trust::PerwerdeScoreProvider<AccountId> for PerwerdeScoreSource {
	/// Read from the education pallet's own limits rather than written out here: a perfect
	/// record is every rewarded course taken at full value, and if either of those changes
	/// the weighting follows it instead of quietly drifting.
	fn max_score() -> u32 {
		RewardedCourseLimit::get().saturating_mul(MaxPointsPerCourse::get())
	}

	fn get_perwerde_score(who: &AccountId) -> u32 {
		pezpallet_perwerde::Pezpallet::<Runtime>::get_perwerde_score(who)
	}
}

/// Tiki score source for Trust pezpallet
pub struct TikiScoreSource;
impl pezpallet_trust::TikiScoreProvider<AccountId> for TikiScoreSource {
	fn max_score() -> u32 {
		pezpallet_tiki::MAX_TIKI_SCORE
	}

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
	type ScoreScale = TrustScoreScale;
	// What the state considers a citizen to be made of. They add to a hundred; `try_state`
	// insists on it.
	type StakingWeight = TrustStakingWeight;
	type ReferralWeight = TrustReferralWeight;
	type PerwerdeWeight = TrustPerwerdeWeight;
	type TikiWeight = TrustTikiWeight;
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

// =============================================================================
// Preimage Pezpallet Configuration
// =============================================================================
//
// The Scheduler and, after it, state referenda need somewhere to keep a call too large to
// carry inline. Deposits are taken as a hold rather than a reserve so the funds stay
// visible as the depositor's, and they are priced with this chain's `deposit()`, which is a
// hundredth of the relay's -- People is where citizens act, not where the network's
// validators do.
//
// The relay prices its own preimages through `pezpallet_parameters` dynamic params. People
// has no such pallet, so these are ordinary constants; changing them is a runtime upgrade,
// which is the honest cost of not carrying a parameters pallet here.

parameter_types! {
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
	pub const PreimageHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Preimage(pezpallet_preimage::HoldReason::Preimage);
}

impl pezpallet_custom_origins::Config for Runtime {}

impl pezpallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
	type WeightInfo = pezpallet_preimage::weights::BizinikiwiWeight<Runtime>;
}

// The register's ballot box. Everything above decides *who* may put a question; this decides
// how a question is settled, and by whom it is counted.
//
// `CitizenTally` counts heads, not tokens: one citizen, one voice, and `support` measured
// against the whole roll rather than against those who happened to turn up. That is the whole
// reason this sits here rather than on the Asset Hub -- the register is the only place that
// knows who the citizens are.

parameter_types! {
	/// Enough to make a frivolous submission cost something, low enough that a citizen can
	/// afford to be the one who asks.
	pub const StateSubmissionDeposit: Balance = 10 * UNITS;
	/// A hundredth of the register. Every jurisdiction that runs citizens' initiatives sets
	/// this between one and five per cent; this is the low end, because the initiative only
	/// opens a question rather than settling one.
	pub const StateInitiativeThreshold: Perbill = Perbill::from_percent(1);
	/// Long enough that a real initiative can be organised, short enough that a petition
	/// cannot cross the line by patience alone.
	pub const StateInitiativeWindow: BlockNumber = 14 * DAYS;
	pub const StateInitiativeDeposit: u128 = 10 * UNITS;
	/// Long enough that a declined question is not simply re-asked until it sticks.
	pub const StateInitiativeCooldown: BlockNumber = 30 * DAYS;
	pub const StateUndecidingTimeout: BlockNumber = 21 * DAYS;
	pub const StateAlarmInterval: BlockNumber = 1;
	pub const StateMaxQueued: u32 = 20;
}

/// The roll, as the tally measures itself against.
pub struct CitizenRoll;
impl pezsp_core::Get<u32> for CitizenRoll {
	fn get() -> u32 {
		<WelatiCitizenSource as pezpallet_welati::CitizenInfo>::citizen_count()
	}
}

impl pezpallet_referenda::Config for Runtime {
	type WeightInfo = pezpallet_referenda::weights::BizinikiwiWeight<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	// Anyone may ask. What they may ask for is the track's business, not this one's.
	type SubmitOrigin = pezframe_system::EnsureSigned<AccountId>;
	type CancelOrigin = pezframe_support::traits::EitherOf<
		EnsureRoot<AccountId>,
		crate::governance::ReferendumCanceller,
	>;
	type KillOrigin = pezframe_support::traits::EitherOf<
		EnsureRoot<AccountId>,
		crate::governance::ReferendumKiller,
	>;
	// Not `()`: dropping a negative imbalance destroys the tokens, and this chain's supply is
	// fixed and halving -- there is no burn anywhere in it, by decision. This is the same
	// handler `pezpallet_identity` slashes into, one line 60-odd above.
	type Slash = ToParentTreasury<RelayTreasuryAccount, LocationToAccountId, Runtime>;
	type Votes = u32;
	type Tally = pezpallet_welati::types::CitizenTally<CitizenRoll>;
	type SubmissionDeposit = StateSubmissionDeposit;
	type MaxQueued = StateMaxQueued;
	type UndecidingTimeout = StateUndecidingTimeout;
	type AlarmInterval = StateAlarmInterval;
	type Tracks = crate::governance::TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
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
	type Preimages = Preimage;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

// =============================================================================
// Democracy 72 and Elections 73 used to sit here. Both are gone; their indices are not
// reused.
//
// Democracy was a live root path, not a dormant shell. Its listed origins are all
// `EnsureRoot`, but `SubmitOrigin` was `EnsureSigned` and the public queue does not pass
// through them: any account could propose, and `pezpallet_democracy` enacts a passed
// referendum with `RawOrigin::Root`. `Preimages = ()` did not close it either -- `bound()`
// takes the `Bounded::Inline` branch before it ever calls `note()`, so every call whose
// encoding fits inline was reachable. `tiki::grant_tiki` is about thirty-six bytes.
//
// Elections could not reach the Council it existed to fill: `ChangeMembers` and
// `InitializeMembers` were both `()`. The bench is seated from Welati instead.
//
// The header on both blocks said "required by Welati". It was not true in either case:
// `pezpallet_welati::Config` asks only for Tiki, Trust and IdentityKyc.
// =============================================================================

// =============================================================================
// Welati (Governance) Pezpallet Configuration
// =============================================================================

parameter_types! {
	/// Parliament size (201 members like Kurdistan Parliament)
	pub const WelatiParliamentSize: u32 = 201;
	/// Diwan council size
	/// The court: eleven seats.
	pub const WelatiDiwanSize: u32 = 11;
	/// Six of them the sitting house elects; the remaining five are the President's to
	/// appoint, derived rather than declared so the two cannot disagree.
	pub const WelatiDiwanElectedSeats: u32 = 6;
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

	/// An elected mandate: four years.
	pub const WelatiTermLength: BlockNumber = 4 * 365 * DAYS;

	/// A seat on the Diwan: nine years.
	///
	/// The exception, and deliberately. The Diwan judges the President and the government; a
	/// court seated on the same cycle as the people it judges leaves with them, and a court
	/// that leaves with the government it was meant to check was never a check.
	pub const WelatiCourtTermLength: BlockNumber = 9 * 365 * DAYS;

	/// How many terms in a row one person may hold the same elected office.
	///
	/// Two, as in most republics. Zero here would mean no limit.
	pub const WelatiMaxConsecutiveTerms: u32 = 2;

	/// Where the PEZ treasury lives, as seen from here: a sibling teyrchain.
	pub WelatiTreasuryChain: Location = Location::new(1, [Teyrchain(testnet_teyrchains_constants::pezkuwichain::locations::AssetHubParaId::get().into())]);

	/// `PezTreasury`'s index in the Asset Hub runtime (`pezpallet_pez_treasury = 70`).
	///
	/// This chain addresses the treasury's calls by index because it cannot name its types.
	/// The emulated integration test asserts the two ends still agree; if the Asset Hub ever
	/// renumbers, that test is what fails rather than a message landing on the wrong pallet.
	pub const WelatiTreasuryPalletIndex: u8 = 70;

	/// The state starts paying its citizens once there are a hundred thousand of them.
	pub const WelatiPopulationThreshold: u32 = 100_000;

	/// Checked once a day. The answer only matters on the era it flips.
	pub const WelatiPopulationCheckPeriod: BlockNumber = DAYS;
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
		// sp-core stopped re-exporting `hashing`; BlakeTwo256 is the runtime-side path and is
		// already in scope here, so no new dependency is needed for the same digest.
		(<BlakeTwo256 as pezsp_runtime::traits::Hash>::hash(&data), block_number)
	}
}

/// How a backed citizens' initiative reaches the ballot.
///
/// The pallet that collects the signatures cannot see what a referendum is made of, and the
/// ballot cannot see who signed. This is the only place that sees both.
///
/// It submits as the proposer, so the ballot's own submission deposit comes from the person
/// who asked, exactly as it would if they had submitted directly. The initiative's deposit is
/// a separate thing and is returned at the same moment.
pub struct LaunchBackedInitiative;
impl pezpallet_welati::InitiativeLaunch<AccountId, Hash> for LaunchBackedInitiative {
	fn launch(
		proposer: &AccountId,
		track: u16,
		hash: Hash,
		len: u32,
	) -> pezsp_runtime::DispatchResult {
		use pezsp_runtime::traits::Dispatchable;

		// The track list is the one authority on which origin a track speaks for; asking it
		// backwards keeps the two from drifting apart.
		let origin = <crate::governance::TracksInfo as pezpallet_referenda::TracksInfo<
			Balance,
			BlockNumber,
		>>::tracks()
		.find(|t| t.id == track)
		.and_then(|_| Self::origin_for(track))
		.ok_or(pezsp_runtime::DispatchError::Other("no such track"))?;

		let call = RuntimeCall::Referenda(pezpallet_referenda::Call::<Runtime>::submit {
			proposal_origin: alloc::boxed::Box::new(origin),
			proposal: pezframe_support::traits::Bounded::Lookup { hash, len },
			enactment_moment: pezframe_support::traits::schedule::DispatchTime::After(0),
		});
		call.dispatch(RuntimeOrigin::signed(proposer.clone()))
			.map(|_| ())
			.map_err(|e| e.error)
	}
}

impl LaunchBackedInitiative {
	fn origin_for(track: u16) -> Option<OriginCaller> {
		use crate::governance::pezpallet_custom_origins::Origin as StateOrigin;
		Some(match track {
			0 => OriginCaller::system(pezframe_system::RawOrigin::Root),
			40 => OriginCaller::Origins(StateOrigin::WelatiElection),
			41 => OriginCaller::Origins(StateOrigin::WelatiAdmin),
			42 => OriginCaller::Origins(StateOrigin::CitizenshipAdmin),
			_ => return None,
		})
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
	// The same roll the tally divides by, so a question is counted and decided against one
	// register rather than two.
	type Electorate = CitizenRoll;
	type Polls = Referenda;
	type Initiatives = LaunchBackedInitiative;
	type InitiativeThreshold = StateInitiativeThreshold;
	type InitiativeWindow = StateInitiativeWindow;
	type InitiativeDeposit = StateInitiativeDeposit;
	type InitiativeSlashTarget = RelayTreasuryAccount;
	type InitiativeCooldown = StateInitiativeCooldown;
	type KycSource = IdentityKyc;
	type ParliamentSize = WelatiParliamentSize;
	type DiwanSize = WelatiDiwanSize;
	type DiwanElectedSeats = WelatiDiwanElectedSeats;
	type CourtRoster = DiwanRoster;
	type HouseRoster = ParliamentRoster;
	type ElectionPeriod = WelatiElectionPeriod;
	type CandidacyPeriod = WelatiCandidacyPeriod;
	type CampaignPeriod = WelatiCampaignPeriod;
	type ElectoralDistricts = WelatiElectoralDistricts;
	type CandidacyDeposit = WelatiCandidacyDeposit;
	type PresidentialEndorsements = WelatiPresidentialEndorsements;
	type ParliamentaryEndorsements = WelatiParliamentaryEndorsements;
	type NativeCurrency = Balances;
	type MaxEndorsers = WelatiMaxEndorsers;
	type TermLength = WelatiTermLength;
	type CourtTermLength = WelatiCourtTermLength;
	type MaxConsecutiveTerms = WelatiMaxConsecutiveTerms;
	type XcmSender = crate::xcm_config::XcmRouter;
	type TreasuryChainLocation = WelatiTreasuryChain;
	type TreasuryPalletIndex = WelatiTreasuryPalletIndex;
	type PopulationThreshold = WelatiPopulationThreshold;
	type PopulationCheckPeriod = WelatiPopulationCheckPeriod;
}

// =============================================================================
// PEZ Rewards Pezpallet Configuration
// =============================================================================

parameter_types! {
	/// The Asset Hub holds the incentive pot; this chain only instructs payments out of it.
	pub const PezRewardsTreasuryPalletIndex: u8 = 70;
}

/// The trust roll the payroll is drawn against.
///
/// Reads the trust pallet directly rather than duplicating anything: the denominator is the
/// running total that pallet already keeps and already proves, and the freeze is what keeps
/// the rate and the shares belonging to the same roll.
pub struct PezRewardsTrustRoll;
impl pezpallet_pez_rewards::TrustRoll<AccountId, BlockNumber> for PezRewardsTrustRoll {
	fn score_of(who: &AccountId) -> u128 {
		Trust::trust_score_of(who)
	}
	fn total_score() -> u128 {
		Trust::total_active_trust_score()
	}
	fn freeze_until(until: BlockNumber) {
		Trust::freeze_until(until);
	}
}

/// Who sits in Parliament, and who holds the seat.
///
/// Two sources on purpose. `welati::ParliamentMembers` is the roll the election wrote, and it
/// is what makes paying Parliament a lookup over two hundred and one accounts instead of the
/// whole population. The `Parlementer` tiki is the seat itself: a member the Diwan removed, or
/// who lost their citizenship, is still on the roll and no longer holds it.
pub struct PezRewardsParliamentRoll;
impl pezpallet_pez_rewards::ParliamentRoll<AccountId, BlockNumber> for PezRewardsParliamentRoll {
	fn seated_at(who: &AccountId) -> Option<BlockNumber> {
		Welati::seat_taken_at(who)
	}
	fn holds_seat(who: &AccountId) -> bool {
		pezpallet_tiki::Pezpallet::<Runtime>::has_tiki(who, &pezpallet_tiki::Tiki::Parlementer)
	}
}

impl pezpallet_pez_rewards::Config for Runtime {
	type WeightInfo = pezpallet_pez_rewards::weights::BizinikiwiWeight<Runtime>;
	type TrustSource = PezRewardsTrustRoll;
	type ParliamentSource = PezRewardsParliamentRoll;
	// Only the chain that holds the pot may say what the pot has been given. Root is
	// deliberately not accepted: a key that can report funding is a key that can promise a
	// payroll out of money that is not there, and the failure would land on the far side of
	// a bridge with nothing here to show for it.
	type FundingOrigin = EnsureXcm<Equals<WelatiTreasuryChain>>;
	type XcmSender = crate::xcm_config::XcmRouter;
	type TreasuryChainLocation = WelatiTreasuryChain;
	type TreasuryPalletIndex = PezRewardsTreasuryPalletIndex;
	// Only ever used on a chain whose genesis did not start the clock.
	type ForceOrigin = crate::RootOrSerokOrCouncilTwoThirds;
}

// =============================================================================
// Recovery Pezpallet Configuration
// =============================================================================

parameter_types! {
	pub const ConfigDepositBase: Balance = 5 * UNITS;
	pub const FriendDepositFactor: Balance = 50 * CENTS;
	pub const RecoveryDeposit: Balance = 5 * UNITS;
	pub const RecoveryDepositPerItem: Balance = deposit(1, 0);
	pub const RecoveryDepositPerByte: Balance = deposit(0, 1);

	pub const FriendGroupsHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Recovery(pezpallet_recovery::HoldReason::FriendGroupsStorage);
	pub const RecoveryAttemptHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Recovery(pezpallet_recovery::HoldReason::AttemptStorage);
	pub const InheritorHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Recovery(pezpallet_recovery::HoldReason::InheritorStorage);
}

// Recovery moved from fixed reserved deposits to footprint-scaled holds. Holds are typed and
// slashable where reserves were neither, and the deposit now tracks what is actually stored, so
// the new shape is adopted rather than pinned to the old one.
//
// The three deposits that existed before carry over unchanged in meaning:
//   ConfigDepositBase + FriendDepositFactor -> FriendGroupsConsideration, which is the same
//     base-plus-per-friend curve the old pallet computed by hand.
//   RecoveryDeposit -> SecurityDeposit, which is what that deposit already was: the stake an
//     initiator puts at risk when opening a recovery. It is now explicitly slashable.
//
// AttemptConsideration and InheritorConsideration have no predecessor. They pay for storage, so
// they are priced like every other byte this chain stores, with the same `deposit()` helper the
// NFT and assets pallets use here. The inheritor feature itself is new and unexercised.
//
// A slashed security deposit goes to the relay treasury rather than being burnt, matching how
// this runtime already routes identity slashes.
impl pezpallet_recovery::Config for Runtime {
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeHoldReason = RuntimeHoldReason;
	type BlockNumberProvider = System;
	type Currency = Balances;
	type FriendGroupsConsideration = HoldConsideration<
		AccountId,
		Balances,
		FriendGroupsHoldReason,
		LinearStoragePrice<ConfigDepositBase, FriendDepositFactor, Balance>,
	>;
	type AttemptConsideration = HoldConsideration<
		AccountId,
		Balances,
		RecoveryAttemptHoldReason,
		LinearStoragePrice<RecoveryDepositPerItem, RecoveryDepositPerByte, Balance>,
	>;
	type InheritorConsideration = HoldConsideration<
		AccountId,
		Balances,
		InheritorHoldReason,
		LinearStoragePrice<RecoveryDepositPerItem, RecoveryDepositPerByte, Balance>,
	>;
	type SecurityDeposit = RecoveryDeposit;
	type Slash = ResolveTo<RelayTreasuryAccount, Balances>;
	// Documented upstream as never safe to reduce: shrinking it makes stored bounded vectors
	// undecodable. Held at the previous MaxFriends value.
	type MaxFriendsPerConfig = ConstU32<9>;
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
