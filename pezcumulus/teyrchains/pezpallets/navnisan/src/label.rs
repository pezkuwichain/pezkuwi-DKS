//! What a label may be, decided once, here.
//!
//! The chain does not normalise: it accepts canonical bytes or refuses them, so two byte
//! strings can never show the same name. Wallets lower-case first and apply the same rules.

use pezframe_support::{traits::ConstU32, BoundedVec};

pub const MAX_LABEL: u32 = 63;
/// Labels under `.kurd`: `x.dks.gov.kurd` is three.
pub const MAX_DEPTH: u32 = 4;
pub const MAX_TEXTS: u32 = 8;
pub const MAX_TEXT_KEY: u32 = 32;
pub const MAX_TEXT_VALUE: u32 = 256;
/// Shortest name anyone may buy; shorter ones are only granted.
pub const MIN_BOUGHT: usize = 3;

pub type Label = BoundedVec<u8, ConstU32<MAX_LABEL>>;
pub type TextKey = BoundedVec<u8, ConstU32<MAX_TEXT_KEY>>;
pub type TextValue = BoundedVec<u8, ConstU32<MAX_TEXT_VALUE>>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Use {
	/// A name bought at the top level.
	Buy,
	/// A name Parliament gives at the top level, or a locked label.
	Grant,
	/// A subname.
	Sub,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LabelError {
	Empty,
	TooLong,
	TooShortToBuy,
	BadChar,
	EdgeHyphen,
	DoubleHyphen,
}

/// `a-z`, `0-9` and `-`; no hyphen at either end; **no `--` anywhere**.
///
/// The double-hyphen rule is wider than DNS's (which reserves only positions three and four,
/// for `xn--`). A wallet keys a site's storage and permissions by its full name, and zone
/// names are shown flattened with `--` (`uni--edu`), so a bought `uni-x--edu` must not be able
/// to exist at all. It also leaves `xn--` free for Kurdish letters later.
pub fn validate(raw: &[u8], usage: Use) -> Result<Label, LabelError> {
	if raw.is_empty() {
		return Err(LabelError::Empty);
	}
	if raw.len() > MAX_LABEL as usize {
		return Err(LabelError::TooLong);
	}
	if !raw.iter().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-') {
		return Err(LabelError::BadChar);
	}
	if raw[0] == b'-' || raw[raw.len() - 1] == b'-' {
		return Err(LabelError::EdgeHyphen);
	}
	if raw.windows(2).any(|w| w == b"--") {
		return Err(LabelError::DoubleHyphen);
	}
	if usage == Use::Buy && raw.len() < MIN_BOUGHT {
		return Err(LabelError::TooShortToBuy);
	}
	Ok(BoundedVec::truncate_from(raw.to_vec()))
}

/// `[a-z0-9._-]`, as ENS text keys (`email`, `url`, `com.twitter`).
pub fn is_text_key(key: &[u8]) -> bool {
	!key.is_empty()
		&& key.iter().all(|b| {
			b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
		})
}

/// An EIP-1577 contenthash: `0xe3` (ipfs-ns) or `0xe5` (ipns-ns), then a CIDv1 whose
/// multihash length matches the bytes that follow. Cheap to check here, and it keeps junk out
/// of every wallet.
pub fn is_contenthash(bytes: &[u8]) -> bool {
	let Some((&ns, rest)) = bytes.split_first() else { return false };
	if ns != 0xe3 && ns != 0xe5 {
		return false;
	}
	// The namespace codes are varints: 0xe3 0x01 and 0xe5 0x01.
	let Some((&0x01, rest)) = rest.split_first() else { return false };
	// CIDv1.
	let Some((&0x01, rest)) = rest.split_first() else { return false };
	// Content codec (one-byte varint for the codecs in use: dag-pb 0x70, raw 0x55, libp2p-key 0x72).
	let Some((&codec, rest)) = rest.split_first() else { return false };
	if codec >= 0x80 {
		return false;
	}
	// Multihash: function code, digest length, digest.
	let Some((&func, rest)) = rest.split_first() else { return false };
	if func >= 0x80 {
		return false;
	}
	let Some((&len, digest)) = rest.split_first() else { return false };
	len as usize == digest.len() && len > 0
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn labels() {
		assert!(validate(b"dks", Use::Buy).is_ok());
		assert!(validate(b"serok-1", Use::Buy).is_ok());
		assert_eq!(validate(b"ab", Use::Buy), Err(LabelError::TooShortToBuy));
		assert!(validate(b"ab", Use::Grant).is_ok());
		assert!(validate(b"a", Use::Sub).is_ok());
		assert_eq!(validate(b"", Use::Sub), Err(LabelError::Empty));
		assert_eq!(validate(b"Dks", Use::Sub), Err(LabelError::BadChar));
		assert_eq!(validate(b"d.k", Use::Sub), Err(LabelError::BadChar));
		assert_eq!(validate("şar".as_bytes(), Use::Sub), Err(LabelError::BadChar));
		assert_eq!(validate(b"-dks", Use::Sub), Err(LabelError::EdgeHyphen));
		assert_eq!(validate(b"dks-", Use::Sub), Err(LabelError::EdgeHyphen));
		assert_eq!(validate(b"uni-x--edu", Use::Buy), Err(LabelError::DoubleHyphen));
		assert_eq!(validate(b"xn--abc", Use::Grant), Err(LabelError::DoubleHyphen));
		assert_eq!(validate(&[b'a'; 64], Use::Sub), Err(LabelError::TooLong));
		assert!(validate(&[b'a'; 63], Use::Buy).is_ok());
	}

	#[test]
	fn contenthash() {
		// ipfs-ns, CIDv1, dag-pb, sha2-256 (0x12), 32 bytes.
		let mut ok = alloc::vec![0xe3, 0x01, 0x01, 0x70, 0x12, 0x20];
		ok.extend([7u8; 32]);
		assert!(is_contenthash(&ok));
		let mut short = ok.clone();
		short.pop();
		assert!(!is_contenthash(&short));
		let mut bad_ns = ok.clone();
		bad_ns[0] = 0xe4;
		assert!(!is_contenthash(&bad_ns));
		assert!(!is_contenthash(&[]));
	}
}
