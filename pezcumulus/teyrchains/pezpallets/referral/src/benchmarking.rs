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
		let inviter: T::AccountId = account("inviter", 0, 0);
		let referred: T::AccountId = account("referred", 0, 1);

		// The call refuses an account that already has an inviter, so start from none.
		InvitedBy::<T>::remove(&referred);

		#[extrinsic_call]
		initiate_referral(RawOrigin::Signed(inviter.clone()), referred.clone());

		assert!(Invitations::<T>::contains_key(&referred, &inviter));
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
