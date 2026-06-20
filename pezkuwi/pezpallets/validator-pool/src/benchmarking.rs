// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-validator-pool

use super::*;
use pezframe_benchmarking::v2::*;
use pezframe_system::{pezpallet_prelude::BlockNumberFor, RawOrigin};
use pezsp_std::vec;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn join_validator_pool() {
		let caller: T::AccountId = whitelisted_caller();

		let category = ValidatorPoolCategory::StakeValidator {
			min_stake: T::MinStakeAmount::get(),
			trust_threshold: 100u128, // Low threshold for benchmarking
		};

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), category.clone());

		// Verify the validator was added
		assert!(PoolMembers::<T>::contains_key(&caller));
		assert!(PerformanceMetrics::<T>::contains_key(&caller));
	}

	#[benchmark]
	fn leave_validator_pool() {
		let caller: T::AccountId = whitelisted_caller();

		let category = ValidatorPoolCategory::StakeValidator {
			min_stake: T::MinStakeAmount::get(),
			trust_threshold: 100u128,
		};

		PoolMembers::<T>::insert(&caller, &category);
		PoolSize::<T>::put(1u32);
		PerformanceMetrics::<T>::insert(&caller, ValidatorPerformance::default());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()));

		// Verify the validator was removed
		assert!(!PoolMembers::<T>::contains_key(&caller));
		assert!(!PerformanceMetrics::<T>::contains_key(&caller));
	}

	#[benchmark]
	fn update_performance_metrics() {
		let validator: T::AccountId = whitelisted_caller();

		PerformanceMetrics::<T>::insert(&validator, ValidatorPerformance::default());

		#[extrinsic_call]
		_(
			RawOrigin::Root,
			validator.clone(),
			100u32, // blocks_produced
			10u32,  // blocks_missed
			500u32, // era_points
		);

		// Verify metrics were updated
		let metrics = PerformanceMetrics::<T>::get(&validator);
		assert_eq!(metrics.blocks_produced, 100);
	}

	#[benchmark]
	fn force_new_era(p: Linear<4, 100>, // Pool size
	) {
		// Add validators to pool
		for i in 0..p {
			let validator: T::AccountId = account("validator", i, 0);
			let category = match i % 3 {
				0 => ValidatorPoolCategory::StakeValidator {
					min_stake: T::MinStakeAmount::get(),
					trust_threshold: 100u128,
				},
				1 => ValidatorPoolCategory::ParliamentaryValidator,
				_ => ValidatorPoolCategory::MeritValidator {
					special_tikis: vec![1u8].try_into().unwrap(),
					community_threshold: 100u32,
				},
			};

			PoolMembers::<T>::insert(&validator, &category);

			let performance = ValidatorPerformance {
				blocks_produced: 90,
				blocks_missed: 10,
				era_points: 500,
				last_active_era: 0,
				reputation_score: 90,
			};
			PerformanceMetrics::<T>::insert(&validator, performance);
		}

		PoolSize::<T>::put(p);
		EraLength::<T>::put(BlockNumberFor::<T>::from(100u32));

		#[extrinsic_call]
		_(RawOrigin::Root);

		// Verify new era was created
		assert_eq!(CurrentEra::<T>::get(), 1);
		assert!(CurrentValidatorSet::<T>::get().is_some());
	}

	#[benchmark]
	fn update_category() {
		let caller: T::AccountId = whitelisted_caller();

		let initial_category = ValidatorPoolCategory::StakeValidator {
			min_stake: T::MinStakeAmount::get(),
			trust_threshold: 100u128,
		};
		PoolMembers::<T>::insert(&caller, &initial_category);

		let new_category = ValidatorPoolCategory::ParliamentaryValidator;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), new_category.clone());

		// Verify category was updated
		assert_eq!(PoolMembers::<T>::get(&caller).unwrap(), new_category);
	}

	#[benchmark]
	fn set_pool_parameters() {
		let new_era_length = BlockNumberFor::<T>::from(200u32);

		#[extrinsic_call]
		_(RawOrigin::Root, new_era_length);

		// Verify parameters were updated
		assert_eq!(EraLength::<T>::get(), new_era_length);
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
