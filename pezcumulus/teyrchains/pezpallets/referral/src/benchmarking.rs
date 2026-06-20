// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-referral

use super::*;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn initiate_referral() {
		let referrer: T::AccountId = account("referrer", 0, 0);
		let referred: T::AccountId = account("referred", 0, 1);

		// Ensure the `referred` account has not been referred before
		PendingReferrals::<T>::remove(&referred);
		Referrals::<T>::remove(&referred);

		#[extrinsic_call]
		initiate_referral(RawOrigin::Signed(referrer.clone()), referred.clone());

		assert_eq!(PendingReferrals::<T>::get(&referred), Some(referrer));
	}

	#[benchmark]
	fn force_confirm_referral() {
		let referrer: T::AccountId = account("referrer", 0, 0);
		let referred: T::AccountId = account("referred", 0, 1);

		// Ensure clean state
		PendingReferrals::<T>::remove(&referred);
		Referrals::<T>::remove(&referred);
		ReferralCount::<T>::remove(&referrer);

		#[extrinsic_call]
		force_confirm_referral(RawOrigin::Root, referrer.clone(), referred.clone());

		assert!(Referrals::<T>::contains_key(&referred));
		assert_eq!(ReferralCount::<T>::get(&referrer), 1);
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
