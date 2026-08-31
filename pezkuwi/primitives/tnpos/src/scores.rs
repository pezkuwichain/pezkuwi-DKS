// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Score providers and the freshness rule that governs them.
//!
//! These traits were duplicated byte for byte across four pallets. One definition lives
//! here; the pallets consume it.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// A score together with when it was last written.
///
/// Scores originate on the People chain and reach the relay chain over XCM. A channel can
/// stall, so a cached score carries its age and is read through `value_if_fresh`.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
pub struct ScoreSnapshot<BlockNumber> {
	pub value: u128,
	pub last_updated: BlockNumber,
}

impl<BlockNumber: Copy + PartialOrd + core::ops::Sub<Output = BlockNumber>>
	ScoreSnapshot<BlockNumber>
{
	/// The value, or `None` if it is older than `max_age` or dated in the future.
	///
	/// Returning the stale value would be the dangerous answer: eligibility computed from a
	/// number nobody is still maintaining. `None` forces the caller to fail closed.
	pub fn value_if_fresh(&self, now: BlockNumber, max_age: BlockNumber) -> Option<u128> {
		if self.last_updated > now {
			return None;
		}
		if now - self.last_updated > max_age {
			return None;
		}
		Some(self.value)
	}
}

/// Every score TNPoS reads about an account, from one place.
pub trait ScoreProvider<AccountId, BlockNumber> {
	fn trust_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
	fn tiki_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
	fn perwerde_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
	fn referral_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
	fn staking_of(who: &AccountId) -> ScoreSnapshot<BlockNumber>;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_fresh_score_reads_its_value() {
		let s = ScoreSnapshot { value: 1_000u128, last_updated: 100u32 };
		assert_eq!(s.value_if_fresh(104, 4), Some(1_000));
	}

	#[test]
	fn a_stale_score_is_absent_not_old() {
		// The failure this guards against: a stalled cross-chain channel leaves the last
		// value in place and the chain keeps treating a months-old number as current.
		// Absent is the honest answer; the caller must then fail closed.
		let s = ScoreSnapshot { value: 1_000u128, last_updated: 100u32 };
		assert_eq!(s.value_if_fresh(105, 4), None);
	}

	#[test]
	fn a_score_from_the_future_is_treated_as_stale() {
		// Clock skew across chains must not silently extend freshness.
		let s = ScoreSnapshot { value: 1_000u128, last_updated: 200u32 };
		assert_eq!(s.value_if_fresh(100, 4), None);
	}
}
