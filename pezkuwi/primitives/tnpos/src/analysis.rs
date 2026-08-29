// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! What the security floors are worth, in probability.
//!
//! Std-only on purpose. This is the argument behind `invariant`, not a thing the chain
//! computes: the runtime enforces the floors, and these functions are how we know the
//! floors are the right ones. They also generate the table in `docs/TNPOS_DESIGN.md`, so
//! the published numbers and the tested numbers cannot drift apart.

/// Binomial coefficient as f64. Exact for the ranges here (n below a few thousand, k=3).
fn choose(n: u32, k: u32) -> f64 {
	if k > n {
		return 0.0;
	}
	let mut acc = 1.0f64;
	for i in 0..k {
		acc = acc * ((n - i) as f64) / ((i + 1) as f64);
	}
	acc
}

/// How many of one stratum's `seats` an adversary holding `adversary` of its `eligible`
/// members takes: the hypergeometric distribution, indexed 0..=seats.
pub fn stratum_distribution(eligible: u32, adversary: u32, seats: u32) -> Vec<f64> {
	let total = choose(eligible, seats);
	(0..=seats)
		.map(|x| {
			if x > adversary || seats - x > eligible.saturating_sub(adversary) || total == 0.0 {
				0.0
			} else {
				choose(adversary, x) * choose(eligible - adversary, seats - x) / total
			}
		})
		.collect()
}

/// The committee-wide distribution: the convolution of the strata's distributions, since
/// the draws are independent once the strata are.
pub fn committee_distribution(per_stratum: &[Vec<f64>]) -> Vec<f64> {
	let mut acc = vec![1.0f64];
	for d in per_stratum {
		let mut next = vec![0.0f64; acc.len() + d.len() - 1];
		for (i, &pi) in acc.iter().enumerate() {
			if pi == 0.0 {
				continue;
			}
			for (j, &pj) in d.iter().enumerate() {
				next[i + j] += pi * pj;
			}
		}
		acc = next;
	}
	acc
}

/// Probability of `from` or more adversary seats.
pub fn tail(dist: &[f64], from: u32) -> f64 {
	dist.iter().skip(from as usize).sum()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::committee::*;

	fn approx(a: f64, b: f64, tol: f64) {
		assert!((a - b).abs() <= tol, "{a} vs {b}");
	}

	#[test]
	fn a_stratum_distribution_is_a_distribution() {
		let d = stratum_distribution(200, 10, 3);
		approx(d.iter().sum::<f64>(), 1.0, 1e-12);
		assert_eq!(d.len(), 4, "0..=3 adversary seats");
	}

	#[test]
	fn a_captured_stratum_yields_all_its_seats() {
		let d = stratum_distribution(3, 3, 3);
		approx(d[3], 1.0, 1e-12);
	}

	#[test]
	fn a_clean_stratum_yields_none() {
		let d = stratum_distribution(200, 0, 3);
		approx(d[0], 1.0, 1e-12);
	}

	// The number the design is sold on. Spec section 5: one power fully captured plus a
	// five percent presence in each of the other eight puts a fork past sixty years.
	#[test]
	fn the_published_budget_reproduces() {
		let mut per = vec![stratum_distribution(3, 3, 3)];
		for _ in 0..8 {
			per.push(stratum_distribution(200, 10, 3));
		}
		let d = committee_distribution(&per);
		approx(d.iter().sum::<f64>(), 1.0, 1e-9);

		let p_halt = tail(&d, halt_threshold(27));
		let p_fork = tail(&d, fork_threshold(27));
		approx(p_halt, 8.79e-4, 1e-5);
		approx(p_fork, 1.15e-5, 1e-6);

		// Four eras a day; the interval the whitepaper quotes.
		let years = (1.0 / p_fork) / 4.0 / 365.25;
		assert!((55.0..65.0).contains(&years), "fork interval drifted: {years} years");
	}

	// The module doc claims the published numbers and the tested numbers cannot drift
	// apart. `the_published_budget_reproduces` above locks exactly one of the eight rows
	// in section 5's table; this locks the rest, so the claim is actually true.
	#[test]
	fn every_published_row_reproduces() {
		// Two significant figures is what the table is printed to, so a match within ten
		// percent is a match; anything looser would let a genuinely wrong number through,
		// and anything tighter would fail on the table's own rounding.
		fn approx_rel(got: f64, published: f64, rel: f64) {
			assert!((got - published).abs() <= rel * published, "got {got}, published {published}");
		}

		// `n` strata fully captured (adversary equals the tiny pool outright), the rest
		// drawing from `spread`.
		fn captured(n: usize, spread: Vec<f64>) -> Vec<Vec<f64>> {
			let mut per = vec![stratum_distribution(3, 3, 3); n];
			per.extend(std::iter::repeat(spread).take(9 - n));
			per
		}

		// (per-stratum distributions, published P(halt), published P(fork))
		let rows: [(Vec<Vec<f64>>, f64, f64); 8] = [
			(vec![stratum_distribution(200, 4, 3); 9], 8.7e-10, 6.5e-13), // Sybil %2
			(vec![stratum_distribution(200, 10, 3); 9], 3.2e-06, 2.1e-08), // Sybil %5
			(vec![stratum_distribution(200, 20, 3); 9], 8.1e-04, 2.5e-05), // Sybil %10
			(vec![stratum_distribution(200, 40, 3); 9], 7.3e-02, 1.1e-02), // Sybil %20
			(captured(1, stratum_distribution(200, 10, 3)), 8.8e-04, 1.2e-05), // 1 erk TAM + %5
			(captured(1, stratum_distribution(200, 20, 3)), 2.7e-02, 1.6e-03), // 1 erk TAM + %10
			(captured(2, stratum_distribution(200, 10, 3)), 8.4e-02, 3.0e-03), // 2 erk TAM + %5
			(captured(3, stratum_distribution(200, 0, 3)), 1.00, 0.0),    // 3 erk TAM
		];

		for (per, want_halt, want_fork) in rows {
			let d = committee_distribution(&per);
			let got_halt = tail(&d, halt_threshold(27));
			let got_fork = tail(&d, fork_threshold(27));
			if want_fork == 0.0 {
				assert_eq!(got_fork, 0.0, "three captured strata cap the fork chance at zero");
			} else {
				approx_rel(got_fork, want_fork, 0.1);
			}
			approx_rel(got_halt, want_halt, 0.1);
		}
	}

	// This is what the runtime's `min_eligible` floor has to buy. Below fifty eligible
	// members a stratum holding ten adversaries loses all three seats far too often.
	#[test]
	fn the_floor_of_fifty_is_where_the_stratum_becomes_safe() {
		let below = stratum_distribution(20, 10, 3)[3];
		let at = stratum_distribution(50, 10, 3)[3];
		assert!(below > 1e-2, "a pool of twenty is not safe: {below}");
		assert!(at < 1e-2, "the floor must bring it under one percent: {at}");
	}

	#[test]
	fn stratification_bounds_a_concentrated_adversary_where_probability_alone_would_not() {
		// Ninety adversary members against a pool of eighteen hundred, drawn two ways.
		//
		// Spread evenly, the two draws are near-identical: same mean, and stratification
		// only trims the variance. That is not what strata are for, and a test comparing
		// that case would be measuring nothing.
		//
		// The difference shows when the adversary concentrates -- which is the realistic
		// attack, since capital can buy the stake stratum outright and cannot buy the other
		// eight. Drawn as one pool those ninety can reach any number of seats. Drawn by
		// stratum they are capped at that stratum's three, and the fork probability is not
		// merely small but exactly zero: a deterministic bound where the flat draw offers
		// only a probabilistic one. That substitution is the whole argument for strata.
		let mut per = vec![stratum_distribution(200, 0, 3); 9];
		per[0] = stratum_distribution(200, 90, 3);
		let concentrated = tail(&committee_distribution(&per), fork_threshold(27));
		let flat = tail(&stratum_distribution(1800, 90, 27), fork_threshold(27));

		assert_eq!(concentrated, 0.0, "three seats is a cap, not a likelihood");
		assert!(flat > 0.0, "the same adversary keeps a real chance in a flat draw: {flat}");
	}
}
