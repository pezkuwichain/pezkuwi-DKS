// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Committee arithmetic.
//!
//! GRANDPA finalises on more than two thirds of the voter weight, so the quorum is not a
//! parameter this chain gets to choose -- it is structural. What follows derives the two
//! thresholds that matter from it.

/// Votes needed to finalise: strictly more than two thirds of `n`.
///
/// The doubling happens in `u64`. These are called at runtime on a committee size read
/// from storage -- not only const-evaluated -- and the runtime is built without overflow
/// checks, so a bare `2 * n` would wrap silently and hand back a quorum smaller than the
/// committee it is meant to bind. `+ 1` cannot overflow, because `2n/3 < n` for every `n`.
pub const fn quorum(n: u32) -> u32 {
	(((2u64 * n as u64) / 3) as u32).saturating_add(1)
}

/// Seats an adversary needs to stop the committee reaching quorum.
///
/// Recoverable: a stalled committee is re-sampled. Compare `fork_threshold`, which is not.
pub const fn halt_threshold(n: u32) -> u32 {
	n.saturating_sub(quorum(n)).saturating_add(1)
}

/// Seats an adversary needs before two conflicting quorums can intersect in adversary-only
/// members -- that is, before the chain can fork. Not recoverable.
pub const fn fork_threshold(n: u32) -> u32 {
	(2u64 * quorum(n) as u64).saturating_sub(n as u64) as u32
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn thresholds_match_the_specified_committee() {
		// Spec section 3: n=27 => q=19, halt at 9 seats, fork at 11.
		assert_eq!(quorum(27), 19);
		assert_eq!(halt_threshold(27), 9);
		assert_eq!(fork_threshold(27), 11);
	}

	#[test]
	fn one_third_of_the_strata_is_what_it_costs() {
		// Three powers of three seats each can stall a 27-seat committee but cannot fork it;
		// forking takes a fourth. This is the property the design is bought for.
		assert!(9 >= halt_threshold(27));
		assert!(9 < fork_threshold(27));
		assert!(12 >= fork_threshold(27));
	}

	#[test]
	fn the_thresholds_hold_at_the_top_of_the_domain() {
		// `Seating::quorum` calls these on a committee size read from storage, and the
		// runtime is built without overflow checks. A bare `2 * n` would wrap here and
		// return a quorum smaller than the committee it is supposed to bind.
		let n = u32::MAX;
		assert!(quorum(n) <= n);
		assert!(halt_threshold(n) >= 1);
		assert!(fork_threshold(n) >= halt_threshold(n));
	}

	#[test]
	fn quorum_never_exceeds_the_committee() {
		// A degraded committee still has to have a reachable quorum.
		for n in 1..=64u32 {
			assert!(quorum(n) <= n, "quorum({n}) = {} exceeds n", quorum(n));
			assert!(halt_threshold(n) >= 1);
		}
	}

	#[test]
	fn safety_margin_exceeds_liveness_margin_at_every_size_this_design_produces() {
		// Every stratum carries three seats, so a committee -- full or degraded -- is always
		// a multiple of three. Across that whole family the fork threshold stays strictly
		// above the halt threshold, which is what makes stalling the recoverable failure and
		// forking the one the budget is spent on.
		for n in (3..=64u32).step_by(3) {
			assert!(fork_threshold(n) > halt_threshold(n), "n = {n}");
		}
	}

	#[test]
	fn a_committee_of_size_one_mod_three_collapses_the_two_margins() {
		// Not a defect in the formulas -- a fact about them, written down so nobody "fixes"
		// it later. Where n leaves remainder one on division by three, q lands such that the
		// thresholds coincide: every set large enough to stall is large enough to fork, and
		// the safety margin is gone. Sizes of this shape must never be seated, which is a
		// condition `invariant::seat` enforces rather than something callers must remember.
		for n in (4..=64u32).step_by(3) {
			assert_eq!(fork_threshold(n), halt_threshold(n), "n = {n}");
		}
	}
}
