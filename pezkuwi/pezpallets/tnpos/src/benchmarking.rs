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
		// Make the account eligible via the runtime's BenchmarkHelper: Meclis is a seat in
		// the house, which the helper arranges.
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
						min_eligible: pezkuwi_tnpos_primitives::invariant::min_eligible_for(id),
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
		// The worst case for the bond gate: a full report naming every member. Each draw
		// reads it once and checks every candidate against it.
		let mut bonded: alloc::collections::BTreeSet<T::AccountId> =
			(0..p).map(|i| account("member", i, 0)).collect();
		let mut i = 0u32;
		while (bonded.len() as u32) < T::MaxBonded::get() {
			bonded.insert(account("bonded", i, 0));
			i += 1;
		}
		BondedOnAssetHub::<T>::put(
			BoundedBTreeSet::<T::AccountId, T::MaxBonded>::try_from(bonded)
				.expect("bounded by MaxBonded; qed"),
		);

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

	/// A full report replacing a full report: the set is built from `n` accounts and written.
	#[benchmark]
	fn note_bonded(n: Linear<1, { T::MaxBonded::get() }>) -> Result<(), BenchmarkError> {
		let held: BoundedBTreeSet<T::AccountId, T::MaxBonded> = (0..T::MaxBonded::get())
			.map(|i| account::<T::AccountId>("held", i, 0))
			.collect::<alloc::collections::BTreeSet<_>>()
			.try_into()
			.map_err(|_| BenchmarkError::Stop("held report too large"))?;
		BondedOnAssetHub::<T>::put(held);
		let bonded: BoundedVec<T::AccountId, T::MaxBonded> = (0..n)
			.map(|i| account("bonded", i, 0))
			.collect::<Vec<_>>()
			.try_into()
			.map_err(|_| BenchmarkError::Stop("report too large"))?;
		let origin = T::BondBenchmarkHelper::bond_origin();

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, 1, bonded);

		assert_eq!(BondedOnAssetHub::<T>::get().map(|s| s.len() as u32), Some(n));
		Ok(())
	}

	#[benchmark]
	fn set_strata() {
		let strata: Vec<StratumConfig> = StratumId::ALL
			.iter()
			.map(|&id| StratumConfig {
				id,
				seats: SEATS_PER_STRATUM,
				min_eligible: pezkuwi_tnpos_primitives::invariant::min_eligible_for(id),
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
						min_eligible: pezkuwi_tnpos_primitives::invariant::min_eligible_for(id),
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

	/// A key change by a member of a full bond report who already holds keys from an era ago:
	/// the report is read whole, both of the old records are read and rewritten, the six
	/// ownership proofs are checked and the message goes to the relay.
	#[benchmark]
	fn set_relay_keys() -> Result<(), BenchmarkError> {
		let who = account::<T::AccountId>("member", 0, 0);
		T::BenchmarkHelper::make_eligible(&who, StratumId::Meclis);
		put_full_bond_report::<T>(&who)?;
		RelayKeysChangedAt::<T>::insert(&who, pezframe_system::Pezpallet::<T>::block_number());
		pezframe_system::Pezpallet::<T>::set_block_number(
			pezframe_system::Pezpallet::<T>::block_number().saturating_add(T::EraLength::get()),
		);
		let (keys, proof) = T::BenchmarkHelper::relay_keys(&who);
		T::BenchmarkHelper::ensure_relay_reachable();

		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), keys.clone(), proof);

		assert_eq!(RelayKeys::<T>::get(&who).map(|k| k.into_inner()), Some(keys));
		Ok(())
	}

	/// Keys withdrawn by a pool member, which takes the member out of the pool as well.
	#[benchmark]
	fn purge_relay_keys() {
		let who = account::<T::AccountId>("member", 0, 0);
		T::BenchmarkHelper::make_eligible(&who, StratumId::Meclis);
		Pezpallet::<T>::do_join(who.clone(), StratumId::Meclis).unwrap();
		T::BenchmarkHelper::ensure_relay_reachable();

		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()));

		assert!(!RelayKeys::<T>::contains_key(&who));
		assert!(!PoolMembers::<T>::contains_key(&who));
	}

	/// A move into the stratum `join` is measured on, checked against a full bond report.
	#[benchmark]
	fn switch_stratum() -> Result<(), BenchmarkError> {
		let who = account::<T::AccountId>("member", 0, 0);
		T::BenchmarkHelper::make_eligible(&who, StratumId::WelatiLottery);
		Pezpallet::<T>::do_join(who.clone(), StratumId::WelatiLottery).unwrap();
		T::BenchmarkHelper::make_eligible(&who, StratumId::Meclis);
		put_full_bond_report::<T>(&who)?;

		#[extrinsic_call]
		_(RawOrigin::Signed(who.clone()), StratumId::Meclis);

		assert_eq!(PoolMembers::<T>::get(&who), Some(StratumId::Meclis));
		Ok(())
	}

	/// A session report naming `n` validators, half of them failed: the most that can be
	/// marked, since a session where more than half failed marks nobody. Every failed
	/// validator's record is already full, so each mark also drops the oldest.
	#[benchmark]
	fn note_session_performance(n: Linear<0, 1_000>) -> Result<(), BenchmarkError> {
		let failed: Vec<T::AccountId> = (0..n / 2).map(|i| account("failed", i, 0)).collect();
		let scored: Vec<T::AccountId> = (n / 2..n).map(|i| account("scored", i, 0)).collect();
		for who in &failed {
			CoFailures::<T>::insert(
				who,
				BoundedVec::try_from(alloc::vec![0u32; 8])
					.map_err(|_| BenchmarkError::Stop("eight marks fit the bound"))?,
			);
		}
		let origin = T::PerformanceOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, scored, failed);

		assert_eq!(SessionsObserved::<T>::get(), 1);
		Ok(())
	}

	#[cfg(test)]
	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}

/// A bond report at its bound that names `who`: the largest set the gates ever read.
fn put_full_bond_report<T: Config>(who: &T::AccountId) -> Result<(), BenchmarkError> {
	let mut bonded = alloc::collections::BTreeSet::new();
	bonded.insert(who.clone());
	let mut i = 0u32;
	while (bonded.len() as u32) < T::MaxBonded::get() {
		bonded.insert(account::<T::AccountId>("bonded", i, 0));
		i += 1;
	}
	BondedOnAssetHub::<T>::put(
		BoundedBTreeSet::<T::AccountId, T::MaxBonded>::try_from(bonded)
			.map_err(|_| BenchmarkError::Stop("bounded by MaxBonded"))?,
	);
	Ok(())
}
