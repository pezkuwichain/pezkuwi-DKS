use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::pezpallet_prelude::{BoundedVec, Get, RuntimeDebug};
use pezsp_core::H256;
use scale_info::TypeInfo;

/// Citizenship status levels
/// PRIVACY: No personal data stored on-chain, only status and hash
#[derive(
	Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Copy, Default,
)]
pub enum KycLevel {
	/// No citizenship application
	#[default]
	NotStarted,
	/// Application submitted, waiting for referrer approval
	/// TRUSTLESS: Referrer must approve before self-confirmation
	PendingReferral,
	/// Referrer approved, waiting for applicant's self-confirmation
	/// TRUSTLESS: No admin involved, applicant confirms themselves
	ReferrerApproved,
	/// Approved citizen with full rights
	Approved,
	/// Citizenship revoked (by governance or self-renounce)
	Revoked,
}

/// Privacy-preserving citizenship application
/// SECURITY: No personal data on-chain, only hash
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct CitizenshipApplication<AccountId> {
	/// Hash of identity documents (actual documents stored off-chain/IPFS)
	/// Frontend calculates: H256(name + email + document_cids)
	pub identity_hash: H256,
	/// The existing citizen who vouches for this applicant
	/// TRUSTLESS: Referrer is personally responsible for their referrals
	pub referrer: AccountId,
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
// --- Dış Dünya İçin Arayüzler (Traits) ---

/// Bir hesabın KYC durumunu sorgulamak için arayüz.
pub trait KycStatus<AccountId> {
	fn get_kyc_status(who: &AccountId) -> KycLevel;
}

/// Bir hesabın kimlik bilgilerini sorgulamak için arayüz.
pub trait IdentityInfoProvider<AccountId, MaxStringLength: Get<u32>> {
	fn get_identity_info(who: &AccountId) -> Option<IdentityInfo<MaxStringLength>>;
}

/// KYC onaylandığında tetiklenecek eylemleri tanımlayan arayüz.
/// Bu trait identity-kyc palletinde tanımlanır ve diğer palletler (örn. referral)
/// tarafından implement edilir, böylece circular dependency oluşmaz.
///
/// UPDATED (Gemini suggestion): Now includes referrer parameter to avoid
/// data loss when identity-kyc and referral have separate storage.
pub trait OnKycApproved<AccountId> {
	/// Called when a citizen is approved
	/// - `who`: The newly approved citizen
	/// - `referrer`: The citizen who vouched for them (from identity-kyc storage)
	fn on_kyc_approved(who: &AccountId, referrer: &AccountId);
}

/// No-op implementation for when no hook is needed
impl<AccountId> OnKycApproved<AccountId> for () {
	fn on_kyc_approved(_who: &AccountId, _referrer: &AccountId) {}
}

/// Vatandaşlık NFT'si mintlemek için arayüz.
/// Bu trait identity-kyc palletinde tanımlanır ve tiki pezpallet tarafından
/// implement edilir, böylece circular dependency oluşmaz.
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
