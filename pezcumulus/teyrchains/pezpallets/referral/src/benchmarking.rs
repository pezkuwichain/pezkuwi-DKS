// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-referral

use super::*;
use pezframe_benchmarking::v2::*;
use pezframe_support::weights::Weight;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn initiate_referral() {
		let inviter: T::AccountId = account("inviter", 0, 0);
		let referred: T::AccountId = account("referred", 0, 1);

		// The call refuses an account that already has an inviter or is already a citizen, so
		// start from neither: both checks read, and the claim is written.
		InvitedBy::<T>::remove(&referred);
		pezpallet_identity_kyc::KycStatuses::<T>::remove(&referred);

		#[extrinsic_call]
		initiate_referral(RawOrigin::Signed(inviter.clone()), referred.clone());

		assert!(Invitations::<T>::contains_key(&referred, &inviter));
	}

	#[benchmark]
	fn withdraw_referral() {
		let claimant: T::AccountId = account("claimant", 0, 0);
		let referred: T::AccountId = account("referred", 0, 1);
		Invitations::<T>::insert(&referred, &claimant, ());

		#[extrinsic_call]
		withdraw_referral(RawOrigin::Signed(claimant.clone()), referred.clone());

		assert!(!Invitations::<T>::contains_key(&referred, &claimant));
	}

	/// One step of the idle-time cleanup: a queued citizen with `n` claims left about them, all
	/// removed and the citizen popped off the queue.
	#[benchmark]
	fn clean_claims(n: Linear<1, CLAIMS_PER_STEP>) {
		let citizen: T::AccountId = account("citizen", 0, 0);
		for i in 0..n {
			Invitations::<T>::insert(&citizen, account::<T::AccountId>("claimant", i, 0), ());
		}
		let tail = CleanupTail::<T>::get();
		CleanupQueue::<T>::insert(tail, &citizen);
		CleanupTail::<T>::put(tail + 1);

		#[block]
		{
			Pezpallet::<T>::clean_up_claims(Weight::MAX);
		}

		assert!(Invitations::<T>::iter_key_prefix(&citizen).next().is_none());
		assert_eq!(CleanupHead::<T>::get(), CleanupTail::<T>::get());
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
