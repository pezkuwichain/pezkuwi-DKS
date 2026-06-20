// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::pezpallet_prelude::RuntimeDebug;
use scale_info::TypeInfo;

// --- GENERAL TYPES ---

/// Structure representing a simple NFT.
/// Note: The actual NFT structure will be more detailed in `pezpallet-tiki`.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
pub struct Tiki {
	pub id: u32,
	// metadata and other fields can be added in the future.
}

/// Raw score type to be used in scoring.
pub type RawScore = u32;

/// Referrer statistics for direct responsibility tracking
/// Used to apply penalties when referrals turn out to be malicious
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
pub struct ReferrerStats {
	/// Total number of successful referrals
	pub total_referrals: u32,
	/// Number of referrals that were later revoked (bad referrals)
	pub revoked_referrals: u32,
	/// Penalty score (affects trust score negatively)
	/// Formula: revoked_referrals * PenaltyPerRevocation
	pub penalty_score: u32,
}

impl ReferrerStats {
	/// Check if referrer has a good track record
	/// Returns true if less than 10% of referrals were revoked
	pub fn has_good_track_record(&self) -> bool {
		if self.total_referrals == 0 {
			return true;
		}
		// Good if less than 10% revoked
		self.revoked_referrals * 10 < self.total_referrals
	}

	/// Calculate adjusted referral score with penalty
	/// Good referrals contribute positively, bad ones contribute negatively
	pub fn adjusted_score(&self, penalty_per_revocation: u32) -> i32 {
		let positive = self.total_referrals.saturating_sub(self.revoked_referrals) as i32;
		let negative = (self.revoked_referrals * penalty_per_revocation) as i32;
		positive.saturating_sub(negative)
	}
}

// --- EXTERNAL INTERFACES (TRAITS) ---

/// Interface for querying an account's inviter.
pub trait InviterProvider<AccountId> {
	fn get_inviter(who: &AccountId) -> Option<AccountId>;
}

/// Interface for calculating an account's referral score.
pub trait ReferralScoreProvider<AccountId> {
	type Score;
	fn get_referral_score(who: &AccountId) -> Self::Score;
}
