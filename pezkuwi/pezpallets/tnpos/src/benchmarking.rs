// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarks. `seat_committee` is parameterised by pool size because it iterates
//! `PoolMembers` once per seated stratum; that iteration is the whole cost.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn join() {
		let who = account::<T::AccountId>("member", 0, 0);
		// Make the account eligible via the runtime's BenchmarkHelper.
		// Meclis is gated on trust scores, which every runtime provides.
		T::BenchmarkHelper::make_eligible(&who, StratumId::Meclis);
		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), StratumId::Meclis);
		assert!(PoolMembers::<T>::contains_key(&who));
	}

	#[benchmark]
	fn leave() {
		let who = account::<T::AccountId>("member", 0, 0);
		// Make the account eligible and join first
		T::BenchmarkHelper::make_eligible(&who, StratumId::Meclis);
		Pezpallet::<T>::do_join(who.clone(), StratumId::Meclis).unwrap();
		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()));
		assert!(!PoolMembers::<T>::contains_key(&who));
	}

	#[benchmark]
	fn seat_committee(p: Linear<450, { T::MaxPoolSize::get() }>) {
		// Nine strata of fifty is the smallest pool where every stratum clears its floor and
		// is therefore drawn. Below it the committee degrades and the benchmark stops
		// measuring the thing it exists to measure: `candidates()` scans `PoolMembers` once
		// per seated stratum, so a nine-stratum seating is nine full scans, not one.
		Strata::<T>::put(
			BoundedVec::try_from(
				StratumId::ALL
					.iter()
					.map(|&id| StratumConfig {
						id,
						seats: SEATS_PER_STRATUM,
						min_eligible: MIN_ELIGIBLE_PER_STRATUM,
					})
					.collect::<Vec<_>>(),
			)
			.expect("nine strata fit the bound; qed"),
		);

		for i in 0..p {
			let who: T::AccountId = account("member", i, 0);
			let stratum = StratumId::ALL[(i % 9) as usize];
			PoolMembers::<T>::insert(&who, stratum);
			StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_add(1));
		}

		// The seed carries the era it belongs to, and `select` refuses a mismatch.
		let era = CurrentEra::<T>::get().saturating_add(1);
		NextSeed::<T>::put(([1u8; 32], era));

		#[block]
		{
			let _ = Pezpallet::<T>::do_seat_committee();
		}

		// If this fails the benchmark measured a refusal, not a seating.
		assert_eq!(CurrentCommittee::<T>::get().len() as u32, 9 * SEATS_PER_STRATUM);
	}

	#[cfg(test)]
	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
