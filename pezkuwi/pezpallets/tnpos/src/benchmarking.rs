// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarks. `seat_committee` is parameterised by pool size because it iterates
//! `PoolMembers` once per seated stratum; that iteration is the whole cost.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;
use pezsp_io::hashing::blake2_256;

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
	fn commit_seed() {
		let who = account::<T::AccountId>("member", 0, 0);
		T::BenchmarkHelper::make_eligible(&who, StratumId::Meclis);
		Pezpallet::<T>::do_join(who.clone(), StratumId::Meclis).unwrap();

		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), [7u8; 32]);

		let era = CurrentEra::<T>::get().saturating_add(1);
		assert!(SeedCommitments::<T>::contains_key(era, &who));
	}

	#[benchmark]
	fn reveal_seed() {
		let who = account::<T::AccountId>("member", 0, 0);
		T::BenchmarkHelper::make_eligible(&who, StratumId::Meclis);
		Pezpallet::<T>::do_join(who.clone(), StratumId::Meclis).unwrap();

		let preimage = [7u8; 32];
		Pezpallet::<T>::do_commit_seed(who.clone(), blake2_256(&preimage)).unwrap();

		// Move past the commit half so the reveal window is open; `do_reveal_seed` refuses
		// a reveal while commits are still being accepted.
		let half = T::EraLength::get() / 2u32.into();
		pezframe_system::Pezpallet::<T>::set_block_number(
			EraStart::<T>::get().saturating_add(half),
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), preimage);

		assert!(NextSeed::<T>::get().is_some());
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
			// Populating `PoolMembers` directly keeps this benchmark's setup cost linear
			// without paying for a full `join` per account, but the account still has to be
			// drawable: on a real runtime `candidates()` filters on session keys, and an
			// account nobody arranged keys for would make the draw come back short.
			T::BenchmarkHelper::make_eligible(&who, stratum);
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

	#[benchmark]
	fn set_strata() {
		let strata: Vec<StratumConfig> = StratumId::ALL
			.iter()
			.map(|&id| StratumConfig {
				id,
				seats: SEATS_PER_STRATUM,
				min_eligible: MIN_ELIGIBLE_PER_STRATUM,
			})
			.collect();

		#[extrinsic_call]
		_(RawOrigin::Root, strata);

		assert_eq!(Strata::<T>::get().len(), 9);
	}

	#[benchmark]
	fn report_offence() {
		// Seat a real committee so the offence exercises the removal path, not a no-op on
		// an account that was never seated.
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

		for i in 0..(9 * MIN_ELIGIBLE_PER_STRATUM) {
			let who: T::AccountId = account("member", i, 0);
			let stratum = StratumId::ALL[(i % 9) as usize];
			// Same reasoning as `seat_committee`: the seat these accounts fill has to be
			// real, so whatever the runtime requires to draw an account must be arranged
			// here, not only the raw pool membership this loop writes directly.
			T::BenchmarkHelper::make_eligible(&who, stratum);
			PoolMembers::<T>::insert(&who, stratum);
			StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_add(1));
		}

		let era = CurrentEra::<T>::get().saturating_add(1);
		NextSeed::<T>::put(([1u8; 32], era));
		Pezpallet::<T>::do_seat_committee().expect("nine full strata must seat");

		let victim = CurrentCommittee::<T>::get()[0].clone();

		#[extrinsic_call]
		_(RawOrigin::Root, victim.clone(), Offence::Equivocation);

		assert!(!CurrentCommittee::<T>::get().contains(&victim));
	}

	#[cfg(test)]
	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
