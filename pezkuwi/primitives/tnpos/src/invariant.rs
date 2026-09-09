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

/// Eligible members a stratum needs before it may be seated. Section 5 of the design puts
/// a stratum's chance of losing all three seats to a ten-member adversary under one percent
/// at this floor. Lives here, next to `FloorTooLow`, rather than in the pallet: the floor
/// and the check that enforces it belong together, not on opposite sides of a crate
/// boundary that only one of them crosses.
pub const MIN_ELIGIBLE_PER_STRATUM: u32 = 50;

/// Seats each stratum carries in the specified committee.
///
/// Moved here from the pallet because the court's floor below is derived from it. A floor whose
/// input lives in another crate is a floor that can be changed without anybody seeing what it
/// moved.
pub const SEATS_PER_STRATUM: u32 = 3;

/// The floor for one stratum: the general one everywhere, the seat count on the court.
///
/// **The court is the only exception, and the reason is that its members cannot be
/// manufactured.** Fifty is sized against an adversary who can *make* eligible members: a
/// lottery dilutes a fixed number of infiltrators only when the pool is far larger than they
/// are, and a pool anyone may enter can be filled with people the attacker controls. The Dîwan
/// is eleven by constitution -- six elected by the house, five appointed by the President, nine
/// years each. Holding six of them means holding the house *and* the presidency, and whoever
/// has done that owns the chain already; three validator seats are not what stopped them. The
/// general floor guards against a threat this stratum does not have.
///
/// So the floor here is the seat count. Below three the stratum cannot fill its seats and is
/// not seated -- the committee is twenty-four rather than twenty-seven, still above
/// `MIN_COMMITTEE` and still drawing from more than `MIN_STRATA`, and the seats are not handed
/// to another stratum. At exactly three the draw is not yet a draw. Applying fifty would have
/// left this stratum permanently unseatable, because eleven can never be fifty, and the
/// judicial arm of the design would have been a name with nothing behind it.
pub const fn min_eligible_for(id: crate::stratum::StratumId) -> u32 {
	match id {
		crate::stratum::StratumId::Divan => SEATS_PER_STRATUM,
		_ => MIN_ELIGIBLE_PER_STRATUM,
	}
}

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
	/// A stratum's floor is set below `MIN_ELIGIBLE_PER_STRATUM`. Comparing eligible counts
	/// against a floor nobody validated let a configuration declare a floor of two, pass
	/// every other gate, and seat nine members while the event announced twenty-seven.
	FloorTooLow,
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
	// `min_eligible` is compared against below, never validated on its own -- a
	// configuration could declare a floor under the design minimum, clear every other
	// check here, and then seat a committee smaller than the one its event announces.
	if strata.iter().any(|c| c.min_eligible < min_eligible_for(c.id)) {
		return Err(InvariantError::FloorTooLow);
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
mod the_courts_floor {
	use super::*;
	use crate::stratum::StratumId;

	#[test]
	fn only_the_court_carries_its_own_floor() {
		// Eleven can never be fifty. Applying the general floor here would have left the
		// judicial gate permanently unseatable: the stratum would exist, be counted among the
		// nine, and never once seat a validator.
		assert_eq!(min_eligible_for(StratumId::Divan), SEATS_PER_STRATUM);
		for id in StratumId::ALL.iter().filter(|id| **id != StratumId::Divan) {
			assert_eq!(
				min_eligible_for(*id),
				MIN_ELIGIBLE_PER_STRATUM,
				"the exception widened past the court"
			);
		}
	}

	fn strata() -> Vec<StratumConfig> {
		StratumId::ALL
			.iter()
			.map(|&id| StratumConfig {
				id,
				seats: SEATS_PER_STRATUM,
				min_eligible: min_eligible_for(id),
			})
			.collect()
	}

	fn eligible_with_court(court: u32) -> Vec<u32> {
		StratumId::ALL
			.iter()
			.map(|&id| if id == StratumId::Divan { court } else { MIN_ELIGIBLE_PER_STRATUM })
			.collect()
	}

	#[test]
	fn a_full_bench_seats_where_fifty_judges_never_could() {
		let all = SEATS_PER_STRATUM * StratumId::ALL.len() as u32;
		assert_eq!(seat(&strata(), &eligible_with_court(11)).unwrap().n, all);
		// The whole point: eleven is a full court and would fail the general floor.
		assert!(11 < MIN_ELIGIBLE_PER_STRATUM);
	}

	#[test]
	fn too_few_judges_costs_three_seats_and_nothing_else() {
		// Two is below the seat count, so the stratum is not seated -- and its seats are not
		// handed to anybody. Twenty-four is still a committee: above `MIN_COMMITTEE`, drawn
		// from more than `MIN_STRATA`.
		let short = seat(&strata(), &eligible_with_court(2)).expect("the rest still stands");
		assert_eq!(short.n, SEATS_PER_STRATUM * (StratumId::ALL.len() as u32 - 1));
		assert!(short.n >= MIN_COMMITTEE);
		assert!(short.seated.len() as u32 >= MIN_STRATA);
		assert!(!short.seated.iter().any(|c| c.id == StratumId::Divan));
	}

	#[test]
	fn a_configured_floor_below_the_courts_own_is_still_refused() {
		// The exception is a floor for one stratum, not the removal of the check. A config
		// that declares less than the court's own floor is as invalid as one that declares
		// less than fifty anywhere else.
		let mut low = strata();
		let court = low
			.iter_mut()
			.find(|c| c.id == StratumId::Divan)
			.expect("the court is one of the nine");
		court.min_eligible = SEATS_PER_STRATUM - 1;
		assert!(matches!(seat(&low, &eligible_with_court(11)), Err(InvariantError::FloorTooLow)));
	}
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
	fn a_floor_below_the_design_minimum_is_refused() {
		// Nine strata of three seats each with a floor of two would pass every other check
		// here and then seat a nine-member committee while the event announces twenty-seven.
		let mut low_floor = nine();
		low_floor[0].min_eligible = MIN_ELIGIBLE_PER_STRATUM - 1;
		assert_eq!(seat(&low_floor, &[200; 9]), Err(InvariantError::FloorTooLow));
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
