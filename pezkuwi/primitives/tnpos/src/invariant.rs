// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The security budget, as a condition the chain refuses to run without.
//!
//! The probability argument is in `analysis` and never reaches the runtime: floating point
//! is not deterministic across platforms and a nine-way convolution does not fit a block.
//! What the runtime enforces is the sufficient condition that argument establishes -- every
//! seated stratum meets its floor -- which is integer arithmetic and costs nothing.

use crate::{committee::quorum, stratum::StratumConfig};
use alloc::vec::Vec;

/// Fewer independent gates than this and one collusion decides the chain.
pub const MIN_STRATA: u32 = 5;

/// Below this the committee is too small for the thresholds to mean anything.
pub const MIN_COMMITTEE: u32 = 15;

/// The most seats a committee may carry. The pallet stores the seated committee in a
/// bounded vector of exactly this size, so a configuration above it would pass validation
/// and then fail at an era boundary -- which is the one moment a configuration must not be
/// allowed to fail.
pub const MAX_COMMITTEE: u32 = 64;

/// Why a configuration cannot be seated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InvariantError {
	/// Fewer than `MIN_STRATA` strata clear their floor.
	TooFewStrata,
	/// Fewer than `MIN_COMMITTEE` seats survive.
	CommitteeTooSmall,
	/// `strata` and `eligible` describe different numbers of strata.
	LengthMismatch,
	/// A stratum declares zero seats, which would let it be counted as independent while
	/// carrying nothing.
	EmptyStratum,
	/// The same stratum appears twice. Nine entries naming eight gates is eight gates, and
	/// the security budget is computed from that count.
	DuplicateStratum,
	/// More seats than the pallet can store for a committee.
	CommitteeTooLarge,
	/// The committee size leaves remainder one on division by three, where the fork and
	/// halt thresholds coincide and the safety margin disappears.
	DegenerateCommitteeSize,
}

/// Which strata are seated this era, and how large the committee therefore is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Seating {
	pub seated: Vec<StratumConfig>,
	pub n: u32,
}

impl Seating {
	/// Votes needed to finalise at this committee's actual size.
	pub fn quorum(&self) -> u32 {
		quorum(self.n)
	}
}

/// Seat every stratum that meets its floor; refuse the era if too little survives.
///
/// A stratum that falls short stands down. Its seats are *not* handed to the strata that
/// are populated -- that repair would concentrate exactly the power the strata exist to
/// split, so it is unavailable by construction rather than by policy.
pub fn seat(strata: &[StratumConfig], eligible: &[u32]) -> Result<Seating, InvariantError> {
	if strata.len() != eligible.len() {
		return Err(InvariantError::LengthMismatch);
	}
	for (i, a) in strata.iter().enumerate() {
		if strata.iter().skip(i.saturating_add(1)).any(|b| b.id == a.id) {
			return Err(InvariantError::DuplicateStratum);
		}
	}
	if strata.iter().any(|c| c.seats == 0) {
		return Err(InvariantError::EmptyStratum);
	}

	let mut seated = Vec::with_capacity(strata.len());
	let mut n = 0u32;
	for (cfg, &have) in strata.iter().zip(eligible.iter()) {
		if have >= cfg.min_eligible {
			n = n.saturating_add(cfg.seats);
			seated.push(*cfg);
		}
	}

	if (seated.len() as u32) < MIN_STRATA {
		return Err(InvariantError::TooFewStrata);
	}
	if n < MIN_COMMITTEE {
		return Err(InvariantError::CommitteeTooSmall);
	}
	if n > MAX_COMMITTEE {
		return Err(InvariantError::CommitteeTooLarge);
	}
	// At n % 3 == 1 the fork and halt thresholds are equal (see `committee`'s tests), so a
	// set that can stall the chain can also fork it. Seats come in threes, so this is
	// unreachable with the specified configuration -- which is exactly why it is cheap to
	// refuse rather than rely on nobody ever configuring a stratum differently.
	if n % 3 == 1 {
		return Err(InvariantError::DegenerateCommitteeSize);
	}

	Ok(Seating { seated, n })
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::stratum::StratumId;

	fn nine() -> Vec<StratumConfig> {
		StratumId::ALL
			.iter()
			.map(|&id| StratumConfig { id, seats: 3, min_eligible: 50 })
			.collect()
	}

	#[test]
	fn a_full_house_seats_twenty_seven() {
		let s = seat(&nine(), &[200; 9]).expect("nine healthy strata must seat");
		assert_eq!(s.n, 27);
		assert_eq!(s.seated.len(), 9);
	}

	#[test]
	fn a_short_stratum_is_not_seated() {
		// 49 eligible against a floor of 50: the stratum stands down rather than being
		// seated at a size its draw cannot make safe.
		let mut e = [200u32; 9];
		e[4] = 49;
		let s = seat(&nine(), &e).expect("eight strata still clear the floors");
		assert_eq!(s.n, 24);
		assert!(!s.seated.iter().any(|c| c.id == StratumId::Tiki));
	}

	#[test]
	fn seats_are_never_redistributed() {
		// The forbidden repair: handing a short stratum's seats to the strata that are
		// populated concentrates exactly the power the design exists to split.
		let mut e = [200u32; 9];
		e[4] = 0;
		let s = seat(&nine(), &e).unwrap();
		assert!(s.seated.iter().all(|c| c.seats == 3), "no stratum may grow to absorb seats");
		assert_eq!(s.n, 24);
	}

	#[test]
	fn too_few_strata_is_refused_not_degraded() {
		let mut e = [200u32; 9];
		for slot in e.iter_mut().take(5) {
			*slot = 0;
		}
		assert_eq!(seat(&nine(), &e), Err(InvariantError::TooFewStrata));
	}

	#[test]
	fn four_healthy_strata_are_still_too_few_gates() {
		// Population is not the point: four fully-populated strata still mean one collusion
		// short of deciding the chain, so the count is refused on its own.
		let four: Vec<StratumConfig> = nine().into_iter().take(4).collect();
		assert_eq!(seat(&four, &[200; 4]), Err(InvariantError::TooFewStrata));
	}

	#[test]
	fn a_repeated_stratum_is_refused() {
		// Nine entries naming eight gates is not nine gates. The budget is computed from the
		// number of independent gates, so a duplicate would let a configuration claim an
		// independence it does not have -- and every probability downstream would be wrong.
		let mut dup = nine();
		dup[8].id = dup[0].id;
		assert_eq!(seat(&dup, &[200; 9]), Err(InvariantError::DuplicateStratum));
	}

	#[test]
	fn a_committee_too_large_to_store_is_refused() {
		// The pallet keeps the seated committee in a bounded vector. A configuration whose
		// seats exceed that bound clears every other check here and then fails at an era
		// boundary -- the one place a configuration must never be allowed to fail.
		let huge: Vec<StratumConfig> = StratumId::ALL
			.iter()
			.map(|&id| StratumConfig { id, seats: 10, min_eligible: 50 })
			.collect();
		assert_eq!(seat(&huge, &[200; 9]), Err(InvariantError::CommitteeTooLarge));
	}

	#[test]
	fn a_stratum_carrying_no_seats_is_refused() {
		// A zero-seat stratum would count towards MIN_STRATA while carrying nothing: nine
		// gates on paper, eight in the committee. The number of independent gates is the
		// quantity the entire security budget is computed from, so it has to mean seats.
		let mut with_empty = nine();
		with_empty[3].seats = 0;
		assert_eq!(seat(&with_empty, &[200; 9]), Err(InvariantError::EmptyStratum));
	}

	#[test]
	fn mismatched_input_lengths_are_refused() {
		assert_eq!(seat(&nine(), &[200; 8]), Err(InvariantError::LengthMismatch));
	}

	#[test]
	fn a_degenerate_committee_size_is_refused() {
		// Seven strata of one seat: sixteen would be seated, and sixteen is one mod three.
		let odd: Vec<StratumConfig> = StratumId::ALL
			.iter()
			.take(7)
			.map(|&id| StratumConfig { id, seats: 1, min_eligible: 50 })
			.collect();
		assert_eq!(seat(&odd, &[200; 7]), Err(InvariantError::CommitteeTooSmall));

		let sixteen: Vec<StratumConfig> = StratumId::ALL
			.iter()
			.take(8)
			.map(|&id| StratumConfig { id, seats: 2, min_eligible: 50 })
			.collect();
		assert_eq!(seat(&sixteen, &[200; 8]), Err(InvariantError::DegenerateCommitteeSize));
	}
}
