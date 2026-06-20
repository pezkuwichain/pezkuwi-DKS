// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-token-wrapper

use super::*;
#[allow(unused)]
use crate::Pezpallet as TokenWrapper;
use pezframe_benchmarking::v2::*;
use pezframe_support::traits::Currency;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn wrap() {
		let caller: T::AccountId = whitelisted_caller();
		let pezpallet_account = Pezpallet::<T>::account_id();
		let amount = 10_000u32.into();

		// Fund both caller and pezpallet account
		let funding = <T::Currency as Currency<T::AccountId>>::minimum_balance()
			.saturating_mul(1000u32.into());

		T::Currency::make_free_balance_be(&caller, funding);
		T::Currency::make_free_balance_be(&pezpallet_account, funding);

		// Create asset
		let _ = T::Assets::create(
			T::WrapperAssetId::get(),
			pezpallet_account.clone(),
			true,
			1u32.into(),
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), amount);

		// Verify
		assert!(T::Assets::balance(T::WrapperAssetId::get(), &caller) >= amount);
	}

	#[benchmark]
	fn unwrap() {
		let caller: T::AccountId = whitelisted_caller();
		let pezpallet_account = Pezpallet::<T>::account_id();
		let amount = 10_000u32.into();

		// Fund both accounts
		let funding = <T::Currency as Currency<T::AccountId>>::minimum_balance()
			.saturating_mul(1000u32.into());

		T::Currency::make_free_balance_be(&caller, funding);
		T::Currency::make_free_balance_be(&pezpallet_account, funding);

		// Create asset
		let _ = T::Assets::create(
			T::WrapperAssetId::get(),
			pezpallet_account.clone(),
			true,
			1u32.into(),
		);

		// Wrap first
		let _ = Pezpallet::<T>::wrap(RawOrigin::Signed(caller.clone()).into(), amount);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), amount);

		// Verify
		assert_eq!(T::Assets::balance(T::WrapperAssetId::get(), &caller), 0u32.into());
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
