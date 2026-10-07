// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use super::*;
use alloc::vec::Vec;
use pezframe_benchmarking::v2::*;
use pezframe_support::{traits::Get, BoundedVec};

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn set_committee(m: Linear<1, { T::MaxMembers::get() }>) -> Result<(), BenchmarkError> {
		let origin = T::BenchmarkHelper::committee_origin();
		let members: BoundedVec<_, T::MaxMembers> = (0..m)
			.map(|i| (account::<T::AccountId>("member", i, 0), i as u128 + 1))
			.collect::<Vec<_>>()
			.try_into()
			.map_err(|_| BenchmarkError::Stop("too many members"))?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, 1, members);

		assert_eq!(Pezpallet::<T>::member_count(), m);
		Ok(())
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
