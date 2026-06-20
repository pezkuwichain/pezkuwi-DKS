use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::pezpallet_prelude::{BoundedVec, Get, RuntimeDebug};
use scale_info::TypeInfo;

/// An encrypted message stored on-chain.
///
/// PRIVACY: The payload is E2E encrypted (XChaCha20-Poly1305).
/// Only the recipient can decrypt using their x25519 private key.
///
/// Messages are ephemeral — automatically deleted at era boundaries.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxPayloadSize))]
#[codec(mel_bound(
	AccountId: MaxEncodedLen,
	BlockNumber: MaxEncodedLen,
))]
pub struct EncryptedMessage<AccountId, BlockNumber, MaxPayloadSize: Get<u32>> {
	/// Sender's account (public, needed for recipient to identify)
	pub sender: AccountId,
	/// Block number when the message was submitted
	pub block_number: BlockNumber,
	/// Sender's ephemeral x25519 public key for this message (32 bytes)
	/// Used by recipient to derive the shared secret for decryption.
	/// A new ephemeral key per message provides forward secrecy.
	pub ephemeral_public_key: [u8; 32],
	/// XChaCha20-Poly1305 nonce (24 bytes)
	pub nonce: [u8; 24],
	/// Encrypted payload (XChaCha20-Poly1305 ciphertext + 16-byte Poly1305 tag)
	/// Max size bounded by Config::MaxMessageSize
	pub ciphertext: BoundedVec<u8, MaxPayloadSize>,
}

/// Trait for checking citizenship status (implemented by identity-kyc pallet)
pub trait CitizenshipChecker<AccountId> {
	/// Returns true if the account is an approved citizen
	fn is_citizen(who: &AccountId) -> bool;
}

/// No-op implementation for testing
impl<AccountId> CitizenshipChecker<AccountId> for () {
	fn is_citizen(_who: &AccountId) -> bool {
		false
	}
}

/// Trait for checking trust score (implemented by pezpallet-trust)
pub trait TrustScoreChecker<AccountId> {
	/// Returns the trust score for an account
	fn trust_score_of(who: &AccountId) -> u32;
}

/// No-op implementation for testing
impl<AccountId> TrustScoreChecker<AccountId> for () {
	fn trust_score_of(_who: &AccountId) -> u32 {
		0
	}
}
