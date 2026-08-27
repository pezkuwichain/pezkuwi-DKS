use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::pezpallet_prelude::{BoundedVec, Get};
use pezsp_core::H256;
use scale_info::TypeInfo;

/// Citizenship status levels
/// PRIVACY: No personal data stored on-chain, only status and hash
#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen, Copy, Default)]
pub enum KycLevel {
	/// No citizenship application
	#[default]
	#[codec(index = 0)]
	NotStarted,
	/// Application submitted, waiting for referrer approval
	/// TRUSTLESS: Referrer must approve before self-confirmation
	#[codec(index = 1)]
	PendingReferral,
	/// Referrer approved, waiting for applicant's self-confirmation
	/// TRUSTLESS: No admin involved, applicant confirms themselves
	#[codec(index = 2)]
	ReferrerApproved,
	/// Approved citizen with full rights
	#[codec(index = 3)]
	Approved,
	/// Citizenship revoked (by governance or self-renounce)
	#[codec(index = 4)]
	Revoked,
}

/// Privacy-preserving citizenship application
/// SECURITY: No personal data on-chain, only hash
#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
pub struct CitizenshipApplication<AccountId, BlockNumber> {
	/// Hash of identity documents (actual documents stored off-chain/IPFS)
	/// Frontend calculates: H256(name + email + document_cids)
	pub identity_hash: H256,
	/// The existing citizen who vouches for this applicant
	/// TRUSTLESS: Referrer is personally responsible for their referrals
	pub referrer: AccountId,
	/// Who the applicant says brought them to the state, if anyone.
	///
	/// Separate from the guarantor and often a different person. It counts only if that
	/// account also claimed the invitation, so neither side can record it alone.
	pub inviter: Option<AccountId>,
	/// When the application was made.
	///
	/// An applicant waits on their referrer for as long as it takes, but not for ever alone:
	/// after a set period the founder may approve instead, and this is what that period is
	/// measured from.
	pub applied_at: BlockNumber,
}

#[derive(Encode, Decode, Clone, Default, MaxEncodedLen)]
pub struct IdentityInfo<MaxStringLength: Get<u32>> {
	pub name: BoundedVec<u8, MaxStringLength>,
	pub email: BoundedVec<u8, MaxStringLength>,
}

// Manually implement PartialEq to avoid requiring `MaxStringLength: PartialEq`
impl<MaxStringLength: Get<u32>> PartialEq for IdentityInfo<MaxStringLength> {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name && self.email == other.email
	}
}
impl<MaxStringLength: Get<u32>> Eq for IdentityInfo<MaxStringLength> {}

// Manually implement Debug as well for the same reason.
impl<MaxStringLength: Get<u32>> core::fmt::Debug for IdentityInfo<MaxStringLength> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("IdentityInfo")
			.field("name", &self.name)
			.field("email", &self.email)
			.finish()
	}
}

impl<MaxStringLength: Get<u32> + 'static> TypeInfo for IdentityInfo<MaxStringLength>
where
	BoundedVec<u8, MaxStringLength>: TypeInfo,
{
	type Identity = Self;

	fn type_info() -> scale_info::Type {
		scale_info::Type::builder()
			.path(scale_info::Path::new("IdentityInfo", "pezpallet_identity_kyc::types"))
			.composite(
				scale_info::build::Fields::named()
					.field(|f| {
						f.ty::<BoundedVec<u8, MaxStringLength>>()
							.name("name")
							.type_name("BoundedVec<u8, MaxStringLength>")
					})
					.field(|f| {
						f.ty::<BoundedVec<u8, MaxStringLength>>()
							.name("email")
							.type_name("BoundedVec<u8, MaxStringLength>")
					}),
			)
	}
}

#[derive(Encode, Decode, Clone, Default, MaxEncodedLen)]
pub struct KycApplication<MaxStringLength: Get<u32>, MaxCidLength: Get<u32>> {
	pub cids: BoundedVec<BoundedVec<u8, MaxCidLength>, MaxCidLength>,
	pub notes: BoundedVec<u8, MaxStringLength>,
}

// Manually implement PartialEq to avoid requiring generic bounds to be PartialEq
impl<MaxStringLength: Get<u32>, MaxCidLength: Get<u32>> PartialEq
	for KycApplication<MaxStringLength, MaxCidLength>
{
	fn eq(&self, other: &Self) -> bool {
		self.cids == other.cids && self.notes == other.notes
	}
}
impl<MaxStringLength: Get<u32>, MaxCidLength: Get<u32>> Eq
	for KycApplication<MaxStringLength, MaxCidLength>
{
}

// Manually implement Debug as well for the same reason.
impl<MaxStringLength: Get<u32>, MaxCidLength: Get<u32>> core::fmt::Debug
	for KycApplication<MaxStringLength, MaxCidLength>
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("KycApplication")
			.field("cids", &self.cids)
			.field("notes", &self.notes)
			.finish()
	}
}

impl<MaxStringLength: Get<u32> + 'static, MaxCidLength: Get<u32> + 'static> TypeInfo
	for KycApplication<MaxStringLength, MaxCidLength>
where
	BoundedVec<BoundedVec<u8, MaxCidLength>, MaxCidLength>: TypeInfo,
	BoundedVec<u8, MaxStringLength>: TypeInfo,
{
	type Identity = Self;

	fn type_info() -> scale_info::Type {
		scale_info::Type::builder()
			.path(scale_info::Path::new("KycApplication", "pezpallet_identity_kyc::types"))
			.composite(
				scale_info::build::Fields::named()
					.field(|f| {
						f.ty::<BoundedVec<BoundedVec<u8, MaxCidLength>, MaxCidLength>>()
							.name("cids")
							.type_name("BoundedVec<BoundedVec<u8, MaxCidLength>, MaxCidLength>")
					})
					.field(|f| {
						f.ty::<BoundedVec<u8, MaxStringLength>>()
							.name("notes")
							.type_name("BoundedVec<u8, MaxStringLength>")
					}),
			)
	}
}
// --- Interfaces for the Outside World (Traits) ---

/// Interface for querying the KYC status of an account.
pub trait KycStatus<AccountId> {
	fn get_kyc_status(who: &AccountId) -> KycLevel;
}

/// Interface for querying the identity information of an account.
pub trait IdentityInfoProvider<AccountId, MaxStringLength: Get<u32>> {
	fn get_identity_info(who: &AccountId) -> Option<IdentityInfo<MaxStringLength>>;
}

/// Interface defining the actions to be triggered when KYC is approved.
/// This trait is defined in the identity-kyc pallet and is implemented by other
/// pallets (e.g. referral), so that no circular dependency is created.
///
/// UPDATED (Gemini suggestion): Now includes referrer parameter to avoid
/// data loss when identity-kyc and referral have separate storage.
pub trait OnKycApproved<AccountId> {
	/// Called when a citizen is approved
	/// - `who`: The newly approved citizen
	/// - `referrer`: The citizen who vouched for them, and who carries the consequences
	/// - `inviter`: Who they say brought them to the state, if they named anyone. A different
	///   fact from the guarantor and frequently a different person: you can be brought here by
	///   one person and ask another to stand for you.
	fn on_kyc_approved(who: &AccountId, referrer: &AccountId, inviter: Option<&AccountId>);
}

/// No-op implementation for when no hook is needed
impl<AccountId> OnKycApproved<AccountId> for () {
	fn on_kyc_approved(_who: &AccountId, _referrer: &AccountId, _inviter: Option<&AccountId>) {}
}

/// Interface for minting a citizenship NFT.
/// This trait is defined in the identity-kyc pallet and is implemented by the tiki
/// pezpallet, so that no circular dependency is created.
pub trait CitizenNftProvider<AccountId> {
	fn mint_citizen_nft(who: &AccountId) -> pezsp_runtime::DispatchResult;

	/// Mint citizen NFT with self-confirmation (uses force_mint internally)
	fn mint_citizen_nft_confirmed(who: &AccountId) -> pezsp_runtime::DispatchResult;

	/// Burn citizen NFT when user renounces citizenship
	fn burn_citizen_nft(who: &AccountId) -> pezsp_runtime::DispatchResult;
}

/// Hook called when citizenship is revoked (for direct responsibility penalty)
/// Defined here to avoid circular dependency, implemented by referral pezpallet
pub trait OnCitizenshipRevoked<AccountId> {
	fn on_citizenship_revoked(who: &AccountId);
}

/// No-op implementation for when no hook is needed
impl<AccountId> OnCitizenshipRevoked<AccountId> for () {
	fn on_citizenship_revoked(_who: &AccountId) {}
}

/// Losing citizenship concerns more than one pallet: the referral record has a penalty to
/// apply, and the trust score has to stop existing. Both, in order.
impl<AccountId, A, B> OnCitizenshipRevoked<AccountId> for (A, B)
where
	A: OnCitizenshipRevoked<AccountId>,
	B: OnCitizenshipRevoked<AccountId>,
{
	fn on_citizenship_revoked(who: &AccountId) {
		A::on_citizenship_revoked(who);
		B::on_citizenship_revoked(who);
	}
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
	fn kyclevel_indices_are_pinned() {
		let pinned: &[(&str, u8, &dyn Fn() -> Vec<u8>)] = &[
			("NotStarted", 0u8, &|| KycLevel::NotStarted.encode()),
			("PendingReferral", 1u8, &|| KycLevel::PendingReferral.encode()),
			("ReferrerApproved", 2u8, &|| KycLevel::ReferrerApproved.encode()),
			("Approved", 3u8, &|| KycLevel::Approved.encode()),
			("Revoked", 4u8, &|| KycLevel::Revoked.encode()),
		];
		let moved: Vec<&str> = pinned
			.iter()
			.filter(|(_, want, enc)| enc() != vec![*want])
			.map(|(name, _, _)| *name)
			.collect();
		assert!(moved.is_empty(), "`KycLevel` indices moved: {moved:?}");
		assert_eq!(pinned.len(), 5, "a variant was added or removed");
	}
}

/// How many more citizens an account may vouch into the register.
///
/// Asked of the pallet that counts vouching, so the register need not know how the number is
/// arrived at -- only that there is one. `None` means unlimited, which is what the founder is:
/// the root of the tree cannot be rate-limited by a record of vouching it predates.
pub trait VouchingCapacity<AccountId> {
	fn remaining(who: &AccountId) -> Option<u32>;
}

impl<AccountId> VouchingCapacity<AccountId> for () {
	fn remaining(_who: &AccountId) -> Option<u32> {
		None
	}
}
