// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarks for `pezpallet-pez-rewards`.
//!
//! Three calls, all constant work. There is no worst case to search for: finalising and
//! closing an epoch happen in `on_initialize` and read a fixed number of keys, and a claim
//! reads one score, one roll and one seat regardless of how many citizens there are.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pezpallet as PezRewards;
use pezframe_benchmarking::v2::*;
use pezframe_support::{assert_ok, traits::EnsureOrigin};
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn initialize_rewards_system() {
		#[extrinsic_call]
		_(RawOrigin::Root);

		assert!(EpochInfo::<T>::exists());
	}

	#[benchmark]
	fn note_incentive_funding() -> Result<(), BenchmarkError> {
		// Ask the configured origin for one it will accept, rather than assuming Root.
		//
		// The mock binds `FundingOrigin` to `EnsureRoot`, so `RawOrigin::Root` passes every test
		// in this crate. The People runtimes bind it to
		// `EnsureXcm<Equals<WelatiTreasuryChain>>` -- only the treasury chain, over XCM -- so the
		// same line benchmarked against a real runtime returns `Bad origin` and takes the whole
		// pallet's weights down with it. Measured 2026-09-20.
		let origin =
			T::FundingOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, 1_000_000_000_000u128);

		assert_eq!(ReportedIncentiveTotal::<T>::get(), 1_000_000_000_000u128);
		Ok(())
	}

	#[benchmark]
	fn claim_reward() {
		let caller: T::AccountId = whitelisted_caller();
		let now = pezframe_system::Pezpallet::<T>::block_number();

		// Worth paying, on the branch that costs the most.
		//
		// `entitlement` short-circuits: an account that holds no seat never reaches
		// `seated_at`, and `seated_at` is the expensive half -- it decodes the whole
		// `ParliamentMembers` roll and scans it. Measuring an unseated caller would price this
		// call below what every seated member actually pays for it.
		T::BenchmarkHelper::make_claimable(&caller);

		assert_ok!(PezRewards::<T>::do_initialize_rewards_system());
		ReportedIncentiveTotal::<T>::put(1_000_000_000_000u128);
		assert_ok!(PezRewards::<T>::do_finalize_epoch(now));

		// The rate is whatever the roll produced; the benchmark only needs the call to reach
		// the payment, so a pool is written directly with a rate that cannot round to zero.
		EpochRewardPools::<T>::mutate(0, |maybe| {
			if let Some(pool) = maybe {
				pool.reward_per_trust_point = 1;
				pool.seat_share = 1;
			}
		});

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), 0);

		assert!(
			ClaimedRewards::<T>::contains_key(0, &caller),
			"claim_reward returned without recording a payment; the benchmark measured a \
			 refusal rather than the call"
		);
	}

	impl_benchmark_test_suite!(PezRewards, crate::mock::new_test_ext(), crate::mock::Test);
}
