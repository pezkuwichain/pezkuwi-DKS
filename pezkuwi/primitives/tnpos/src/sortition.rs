// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Drawing a stratum's seats.
//!
//! `Sortition` is where the randomness comes from; `sample_k` is how it is spent. Phase 1
//! supplies a commit-reveal seed behind that trait and phase 2 replaces it with ring-VRF
//! tickets -- the split is what keeps that swap from touching the pallet.

use crate::stratum::StratumId;
use alloc::vec::Vec;

/// Source of a stratum's draw for an era.
pub trait Sortition<AccountId> {
	/// The members seated for `stratum` in `era`, or `None` if this era cannot be drawn --
	/// no seed yet, or not enough tickets. `None` degrades the committee (the stratum is
	/// left unseated); it never falls back to an order an adversary could have chosen.
	fn select(
		era: u32,
		stratum: StratumId,
		candidates: &[AccountId],
		k: u32,
	) -> Option<Vec<AccountId>>;
}

/// An index below `bound`, derived from 64 bits by Lemire's multiply-shift.
///
/// Constant cost and no rejection loop, so the weight stays deterministic. The residual
/// bias is below 2^-32 for any bound this chain will use.
fn index_below(bound: u32, word: u64) -> u32 {
	(((word as u128) * (bound as u128)) >> 64) as u32
}

/// `k` distinct members of `candidates`, drawn by a partial Fisher-Yates shuffle.
///
/// `domain` separates the draws so two strata of the same size do not seat the same
/// positions of their lists.
pub fn sample_k<T: Clone>(candidates: &[T], k: u32, seed: &[u8; 32], domain: &[u8]) -> Vec<T> {
	let n = candidates.len();
	let take = core::cmp::min(k as usize, n);
	if take == 0 {
		return Vec::new();
	}

	let mut idx: Vec<u32> = (0..n as u32).collect();
	let mut out = Vec::with_capacity(take);

	for round in 0..take {
		// One hash per draw, bound to seed, domain and round: no counter state to get wrong.
		let mut preimage = Vec::with_capacity(32 + domain.len() + 4);
		preimage.extend_from_slice(seed);
		preimage.extend_from_slice(domain);
		preimage.extend_from_slice(&(round as u32).to_le_bytes());
		let h = pezsp_io::hashing::blake2_256(&preimage);

		let word = u64::from_le_bytes(
			h[..8].try_into().expect("blake2_256 returns 32 bytes; 8 always fit; qed"),
		);
		let remaining = (n - round) as u32;
		let pick = round + index_below(remaining, word) as usize;

		idx.swap(round, pick);
		out.push(candidates[idx[round] as usize].clone());
	}

	out
}

#[cfg(test)]
mod tests {
	use super::*;

	const SEED: [u8; 32] = [7u8; 32];

	#[test]
	fn picks_exactly_k_distinct_candidates() {
		let pool: Vec<u32> = (0..50).collect();
		let got = sample_k(&pool, 3, &SEED, b"stake");
		assert_eq!(got.len(), 3);
		let mut d = got.clone();
		d.sort();
		d.dedup();
		assert_eq!(d.len(), 3, "a member must not be seated twice");
		assert!(got.iter().all(|x| pool.contains(x)));
	}

	#[test]
	fn takes_everyone_when_k_exceeds_the_pool() {
		let pool: Vec<u32> = (0..2).collect();
		assert_eq!(sample_k(&pool, 3, &SEED, b"stake").len(), 2);
	}

	#[test]
	fn is_deterministic_for_the_same_seed_and_domain() {
		let pool: Vec<u32> = (0..50).collect();
		assert_eq!(sample_k(&pool, 3, &SEED, b"stake"), sample_k(&pool, 3, &SEED, b"stake"));
	}

	#[test]
	fn different_strata_do_not_share_a_draw() {
		// Without domain separation every stratum would seat the same positions of its list.
		let pool: Vec<u32> = (0..200).collect();
		assert_ne!(sample_k(&pool, 3, &SEED, b"stake"), sample_k(&pool, 3, &SEED, b"tiki"));
	}

	#[test]
	fn each_round_draws_from_its_own_randomness() {
		// If `round` were dropped from the hash preimage, every round of one call would
		// share a random word. The shrinking bound would still hand back distinct picks, so
		// the draw would look valid and every other test in this file would still pass --
		// but consecutive picks would track each other almost exactly instead of being
		// independent, and a draw whose second seat is a function of its first is a weaker
		// draw than the security argument assumes. This measures the consequence: over many
		// seeds, two draws from a large pool land next to each other only rarely.
		let pool: Vec<u32> = (0..1_000).collect();
		let trials = 500u32;
		let mut adjacent = 0u32;
		for i in 0..trials {
			let mut seed = [0u8; 32];
			seed[..4].copy_from_slice(&i.to_le_bytes());
			let got = sample_k(&pool, 2, &seed, b"stake");
			if got[0].abs_diff(got[1]) <= 2 {
				adjacent += 1;
			}
		}
		// Independent rounds collide this closely about half a percent of the time; a shared
		// word puts it near certainty. The bar is deliberately loose so this cannot flake.
		assert!(adjacent < trials / 10, "{adjacent} of {trials} draws were adjacent");
	}

	#[test]
	fn every_candidate_is_reachable() {
		// The old implementation read one byte per swap, so with a pool above 256 the tail
		// was unreachable -- members could never be seated at all. This is that regression.
		let pool: Vec<u32> = (0..400).collect();
		let mut seen = alloc::collections::BTreeSet::new();
		for i in 0..2_000u32 {
			let mut seed = [0u8; 32];
			seed[..4].copy_from_slice(&i.to_le_bytes());
			seen.extend(sample_k(&pool, 3, &seed, b"stake"));
		}
		assert!(seen.iter().any(|&x| x >= 300), "high indices were never drawn");
	}
}
