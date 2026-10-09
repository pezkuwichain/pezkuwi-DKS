// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-trust
//!
//! These benchmarks measure the performance of trust score operations.

use super::*;

use pezframe_benchmarking::{v2::*, whitelisted_caller};
use pezframe_system::RawOrigin;
use pezsp_runtime::traits::Zero;

#[benchmarks]
mod benchmarks {
	use super::*;

	/// Make `account` a citizen the way the chain does: an approved KYC record, which is what
	/// every runtime's `CitizenshipSource` reads. The People runtimes used to answer `true` here
	/// under `runtime-benchmarks` instead, so the measured weight never paid for that read.
	fn setup_citizen<T: Config>(account: &T::AccountId) {
		pezpallet_identity_kyc::KycStatuses::<T>::insert(
			account,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);
		TrustScores::<T>::insert(account, T::Score::zero());
	}

	#[benchmark]
	fn force_recalculate_trust_score() -> Result<(), BenchmarkError> {
		// Setup
		let account: T::AccountId = whitelisted_caller();
		setup_citizen::<T>(&account);

		#[extrinsic_call]
		force_recalculate_trust_score(RawOrigin::Root, account.clone());

		// Verify - trust score should be calculated (may be zero if no component scores)
		assert!(TrustScores::<T>::contains_key(&account));
		Ok(())
	}

	#[benchmark]
	fn update_all_trust_scores() {
		// Setup - Ensure no batch update is in progress
		crate::BatchUpdateInProgress::<T>::put(false);

		#[extrinsic_call]
		update_all_trust_scores(RawOrigin::Root);

		// Verify - The function completed (may or may not have set BatchUpdateInProgress
		// depending on whether there are citizens to process)
		// We just verify it doesn't panic
	}

	#[benchmark]
	fn periodic_trust_score_update() {
		// Setup - Ensure no batch update is in progress
		crate::BatchUpdateInProgress::<T>::put(false);

		#[extrinsic_call]
		periodic_trust_score_update(RawOrigin::Root);

		// Verify - The function completed successfully
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
