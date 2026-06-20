// This file is part of Bizinikiwi.

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

//! Manually estimated weights for `pezpallet_staking_score`
//!
//! These weights are conservative overestimates pending proper benchmark runs.
//! They account for the `OnStakingUpdate` callback cost (trust pallet update).
//!
//! DATE: 2026-02-16
//! UPDATED: 2026-02-16 (noter delegation + zero-stake cleanup)
//! TODO: Run proper benchmarks to replace these estimates.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]
#![allow(dead_code)]

use pezframe_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pezpallet_staking_score`.
pub trait WeightInfo {
	fn start_score_tracking() -> Weight;
	fn receive_staking_details() -> Weight;
}

/// Weights for `pezpallet_staking_score` using the Bizinikiwi node and recommended hardware.
pub struct BizinikiwiWeight<T>(PhantomData<T>);
impl<T: pezframe_system::Config> WeightInfo for BizinikiwiWeight<T> {
	/// `start_score_tracking` — user opt-in, no stake check needed.
	///
	/// Pallet operations:
	///   StakingScore::StakingStartBlock (r:1 w:1) — check + insert
	///
	/// OnStakingUpdate callback (trust pallet):
	///   StakingScore::CachedStakingDetails (r:2 w:0) — iter_prefix for score calc
	///   StakingScore::StakingStartBlock    (r:1 w:0) — duration lookup in score calc
	///   Trust::TrustScores                 (r:1 w:1) — update trust score
	///   Trust::TotalActiveTrustScore       (r:1 w:1) — update aggregate
	///   IdentityKyc::KycStatuses           (r:1 w:0) — citizenship gate
	///
	/// Total: r:7 w:3
	fn start_score_tracking() -> Weight {
		// ~30us execution + 7 reads + 3 writes
		// Proof size: StakingStartBlock(52) + CachedStakingDetails(77*2) +
		//             TrustScores(48) + TotalActiveTrustScore(16) + KycStatuses(34) = ~7500
		Weight::from_parts(30_000_000, 7_500)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}

	/// `receive_staking_details` — worst case: noter signed + zero-stake cleanup.
	///
	/// Origin check (noter path):
	///   Tiki::UserTikis (r:1 w:0) — noter authority check via has_tiki()
	///
	/// Zero-stake cleanup path (worst case):
	///   StakingScore::CachedStakingDetails (r:1 w:1) — remove entry for source
	///   StakingScore::CachedStakingDetails (r:2 w:0) — iter_prefix remaining check
	///   StakingScore::StakingStartBlock    (r:1 w:1) — remove if no remaining stake
	///
	/// OnStakingUpdate callback (trust pallet):
	///   StakingScore::CachedStakingDetails (r:2 w:0) — iter for score calc (overlaps)
	///   StakingScore::StakingStartBlock    (r:1 w:0) — duration lookup (overlaps)
	///   Trust::TrustScores                 (r:1 w:1) — update trust score
	///   Trust::TotalActiveTrustScore       (r:1 w:1) — update aggregate
	///   IdentityKyc::KycStatuses           (r:1 w:0) — citizenship gate
	///
	/// Note: Some reads overlap (CachedStakingDetails iter + StakingStartBlock read
	/// are done both in cleanup and in the trust callback score calculation).
	/// Counting unique reads conservatively:
	///
	/// Total: r:10 w:4
	fn receive_staking_details() -> Weight {
		// ~40us execution + 10 reads + 4 writes
		// Proof size: Tiki::UserTikis(200) + CachedStakingDetails(77*2) +
		//             StakingStartBlock(52) + TrustScores(48) +
		//             TotalActiveTrustScore(16) + KycStatuses(34) = ~9500
		Weight::from_parts(40_000_000, 9_500)
			.saturating_add(T::DbWeight::get().reads(10_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn start_score_tracking() -> Weight {
		Weight::from_parts(30_000_000, 7_500)
			.saturating_add(RocksDbWeight::get().reads(7_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}

	fn receive_staking_details() -> Weight {
		Weight::from_parts(40_000_000, 9_500)
			.saturating_add(RocksDbWeight::get().reads(10_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
}
