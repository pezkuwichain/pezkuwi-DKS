// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

extern crate alloc;

use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pezframe_support::{traits::ConstU32, BoundedVec};
use pezsp_runtime::RuntimeDebug;
use scale_info::TypeInfo;

/// Types of validators in the pool
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
#[codec(mel_bound())]
pub enum ValidatorPoolCategory {
	/// Stake-based validators (high stake + trust score)
	StakeValidator { min_stake: u128, trust_threshold: u128 },
	/// Parliamentary validators (elected parliament members)
	ParliamentaryValidator,
	/// Merit-based validators (special Tikis + community support)
	MeritValidator {
		special_tikis: BoundedVec<u8, ConstU32<5>>, // Tiki types they hold
		community_threshold: u32,                   // Minimum referral count
	},
}

/// Performance metrics for a validator
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
#[codec(mel_bound())]
pub struct ValidatorPerformance {
	/// Total blocks produced
	pub blocks_produced: u32,
	/// Total blocks missed
	pub blocks_missed: u32,
	/// Era points earned
	pub era_points: u32,
	/// Last era when this validator was active
	pub last_active_era: u32,
	/// Reputation score (0-100)
	pub reputation_score: u8,
}

/// Current validator set for an era
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
#[codec(mel_bound())]
pub struct ValidatorSet<AccountId>
where
	AccountId: Encode + Decode + DecodeWithMemTracking + Clone + PartialEq + Eq + MaxEncodedLen,
{
	/// Era index
	pub era_index: u32,
	/// Stake-based validators (target: 10)
	pub stake_validators: BoundedVec<AccountId, ConstU32<10>>,
	/// Parliamentary validators (target: 6)
	pub parliamentary_validators: BoundedVec<AccountId, ConstU32<6>>,
	/// Merit-based validators (target: 5)
	pub merit_validators: BoundedVec<AccountId, ConstU32<5>>,
}

impl<AccountId> ValidatorSet<AccountId>
where
	AccountId: Encode + Decode + DecodeWithMemTracking + Clone + PartialEq + Eq + MaxEncodedLen,
{
	/// Get all validators in the set
	pub fn all_validators(&self) -> Vec<AccountId> {
		let mut all = Vec::new();
		all.extend(self.stake_validators.iter().cloned());
		all.extend(self.parliamentary_validators.iter().cloned());
		all.extend(self.merit_validators.iter().cloned());
		all
	}

	/// Get total validator count
	pub fn total_count(&self) -> u32 {
		self.stake_validators.len() as u32
			+ self.parliamentary_validators.len() as u32
			+ self.merit_validators.len() as u32
	}
}

/// Trait for referral system integration
pub trait ReferralProvider<AccountId> {
	/// Get referral count for an account
	fn get_referral_count(who: &AccountId) -> u32;
}

/// Trait for Perwerde system integration  
pub trait PerwerdeProvider<AccountId> {
	/// Get Perwerde score for an account
	fn get_perwerde_score(who: &AccountId) -> u32;
}

/// Default implementation for tests
impl<AccountId> ReferralProvider<AccountId> for () {
	fn get_referral_count(_who: &AccountId) -> u32 {
		0
	}
}

impl<AccountId> PerwerdeProvider<AccountId> for () {
	fn get_perwerde_score(_who: &AccountId) -> u32 {
		0
	}
}

// ============================================================================
// SHADOW MODE TYPES
// ============================================================================

/// Operation mode for the validator pool
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Default,
	serde::Serialize,
	serde::Deserialize,
)]
pub enum OperationMode {
	/// Shadow mode: TNPoS runs in parallel but doesn't control consensus
	/// NPoS remains the authority, TNPoS results are recorded for comparison
	#[default]
	Shadow,
	/// Active mode: TNPoS directly controls validator selection
	/// Used on Zagros testnet and after full transition
	Active,
}

/// Shadow mode comparison result for a single era
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
#[codec(mel_bound())]
pub struct ShadowComparison<AccountId>
where
	AccountId: Encode + Decode + DecodeWithMemTracking + Clone + PartialEq + Eq + MaxEncodedLen,
{
	/// Era when this comparison was made
	pub era_index: u32,
	/// Number of validators that appear in both NPoS and TNPoS selections
	pub overlap_count: u32,
	/// Validators selected by TNPoS but not by NPoS
	pub tnpos_only: BoundedVec<AccountId, ConstU32<21>>,
	/// Validators selected by NPoS but not by TNPoS
	pub npos_only: BoundedVec<AccountId, ConstU32<21>>,
	/// TNPoS selection would have included more stake validators
	pub stake_diff: i32,
	/// TNPoS selection would have included more parliamentary validators
	pub parliamentary_diff: i32,
	/// TNPoS selection would have included more merit validators
	pub merit_diff: i32,
}

/// Cumulative shadow mode statistics across multiple eras
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
#[codec(mel_bound())]
pub struct ShadowStatistics {
	/// Total number of eras analyzed
	pub eras_analyzed: u32,
	/// Total overlapping validators across all eras
	pub total_overlap: u32,
	/// Total TNPoS-only selections across all eras
	pub total_tnpos_only: u32,
	/// Total NPoS-only selections across all eras
	pub total_npos_only: u32,
	/// Average overlap percentage (stored as basis points, e.g., 5000 = 50%)
	pub avg_overlap_bps: u32,
	/// Number of eras where TNPoS would have performed better (based on block production)
	pub tnpos_better_eras: u32,
	/// Number of eras where NPoS performed better
	pub npos_better_eras: u32,
	/// Total blocks that would have been produced by TNPoS selections
	pub tnpos_projected_blocks: u64,
	/// Total blocks actually produced by NPoS selections
	pub npos_actual_blocks: u64,
	/// Eras where TNPoS had higher stake representation
	pub higher_stake_eras: u32,
	/// Eras where TNPoS had higher trust representation
	pub higher_trust_eras: u32,
}

/// Detailed per-era shadow analysis data
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
#[codec(mel_bound())]
pub struct EraAnalysis {
	/// Era index
	pub era_index: u32,
	/// Block number when analysis was recorded
	pub recorded_at_block: u32,
	/// Total stake of TNPoS selected validators
	pub tnpos_total_stake: u128,
	/// Total stake of NPoS selected validators
	pub npos_total_stake: u128,
	/// Average trust score of TNPoS selected validators
	pub tnpos_avg_trust: u32,
	/// Average trust score of NPoS selected validators
	pub npos_avg_trust: u32,
	/// Number of stake validators in TNPoS selection
	pub tnpos_stake_count: u8,
	/// Number of parliamentary validators in TNPoS selection
	pub tnpos_parliamentary_count: u8,
	/// Number of merit validators in TNPoS selection
	pub tnpos_merit_count: u8,
	/// Blocks produced this era (filled at era end)
	pub blocks_produced: u32,
	/// Blocks missed this era (filled at era end)
	pub blocks_missed: u32,
}

/// Category distribution snapshot
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
#[codec(mel_bound())]
pub struct CategoryDistribution {
	/// Stake validators count
	pub stake: u8,
	/// Parliamentary validators count
	pub parliamentary: u8,
	/// Merit validators count
	pub merit: u8,
	/// Target stake count
	pub target_stake: u8,
	/// Target parliamentary count
	pub target_parliamentary: u8,
	/// Target merit count
	pub target_merit: u8,
}
