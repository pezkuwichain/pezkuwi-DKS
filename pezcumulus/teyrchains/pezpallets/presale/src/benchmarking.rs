// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-presale
//!
//! Complete benchmarks for all presale operations including:
//! - create_presale, cancel_presale, add_to_whitelist
//! - contribute, refund, claim_vested
//! - finalize_presale (with O(N) contributor loop)
//! - refund_cancelled_presale, batch_refund_failed_presale

use super::*;
#[allow(unused)]
use crate::Pezpallet as Presale;
use pezframe_benchmarking::v2::*;
use pezframe_support::traits::fungibles::Mutate;
use pezframe_system::RawOrigin;

#[benchmarks(
	where
		T::AssetId: From<u32>,
		T::Assets: Create<T::AccountId> + Mutate<T::AccountId>,
)]
mod benchmarks {
	use super::*;
	use pezframe_support::traits::{fungibles::Create, Get};

	fn get_asset_id<T: Config>(seed: u32) -> T::AssetId
	where
		T::AssetId: From<u32>,
	{
		seed.into()
	}

	/// Setup assets for presale benchmarking
	/// Creates payment and reward assets, mints to necessary accounts
	fn setup_benchmark_assets<T: Config>(
		caller: &T::AccountId,
		presale_treasury: &T::AccountId,
	) -> (T::AssetId, T::AssetId)
	where
		T::AssetId: From<u32>,
		T::Assets: Create<T::AccountId> + Mutate<T::AccountId>,
	{
		let payment_asset = get_asset_id::<T>(1);
		let reward_asset = get_asset_id::<T>(2);

		// Create assets if they don't exist (ignore errors if already created)
		let min_balance: T::Balance = 1u128.into();
		let _ = T::Assets::create(payment_asset, caller.clone(), true, min_balance);
		let _ = T::Assets::create(reward_asset, caller.clone(), true, min_balance);

		// Mint payment tokens to caller for contributions
		let payment_amount: T::Balance = 100_000_000u128.into();
		let _ = T::Assets::mint_into(payment_asset, caller, payment_amount);

		// Mint payment tokens to platform accounts for fee distribution
		let _ = T::Assets::mint_into(payment_asset, &T::PlatformTreasury::get(), payment_amount);
		let _ = T::Assets::mint_into(payment_asset, &T::StakingRewardPool::get(), payment_amount);

		// Mint reward tokens to presale treasury for distribution
		let reward_amount: T::Balance = 10_000_000_000u128.into();
		let _ = T::Assets::mint_into(reward_asset, presale_treasury, reward_amount);

		(payment_asset, reward_asset)
	}

	/// Create a presale with standard parameters
	fn create_test_presale<T: Config>(
		caller: &T::AccountId,
		payment_asset: T::AssetId,
		reward_asset: T::AssetId,
		is_whitelist: bool,
		enable_vesting: bool,
	) -> PresaleId
	where
		T::AssetId: From<u32>,
	{
		let presale_id = NextPresaleId::<T>::get();

		let vesting = if enable_vesting {
			Some(crate::VestingSchedule {
				immediate_release_percent: 20u8,
				vesting_duration_blocks: 100u32.into(),
				cliff_blocks: 10u32.into(),
			})
		} else {
			None
		};

		let params = crate::PresaleCreationParams {
			tokens_for_sale: 10_000_000_000u128,
			duration: 1000u32.into(),
			is_whitelist,
			limits: crate::ContributionLimits {
				min_contribution: 100u128,
				max_contribution: 10_000_000u128,
				soft_cap: 1_000_000u128,
				hard_cap: 100_000_000u128,
			},
			vesting,
			refund_config: crate::RefundConfig {
				grace_period_blocks: 10u32.into(),
				refund_fee_percent: 5u8,
				grace_refund_fee_percent: 2u8,
			},
		};

		let _ = Presale::<T>::create_presale(
			RawOrigin::Signed(caller.clone()).into(),
			payment_asset,
			reward_asset,
			params,
		);

		presale_id
	}

	#[benchmark]
	fn create_presale() {
		let caller: T::AccountId = whitelisted_caller();
		let payment_asset = get_asset_id::<T>(1);
		let reward_asset = get_asset_id::<T>(2);

		let params = crate::PresaleCreationParams {
			tokens_for_sale: 1_000_000u128,
			duration: 100u32.into(),
			is_whitelist: false,
			limits: crate::ContributionLimits {
				min_contribution: 100u128,
				max_contribution: 10_000u128,
				soft_cap: 500_000u128,
				hard_cap: 1_000_000u128,
			},
			vesting: None,
			refund_config: crate::RefundConfig {
				grace_period_blocks: 10u32.into(),
				refund_fee_percent: 5u8,
				grace_refund_fee_percent: 10u8,
			},
		};

		#[extrinsic_call]
		create_presale(RawOrigin::Signed(caller), payment_asset, reward_asset, params);

		// Verify presale was created
		assert!(crate::Presales::<T>::contains_key(0));
	}

	#[benchmark]
	fn cancel_presale() {
		let caller: T::AccountId = whitelisted_caller();
		let payment_asset = get_asset_id::<T>(1);
		let reward_asset = get_asset_id::<T>(2);

		// Create a presale first
		let presale_id =
			create_test_presale::<T>(&caller, payment_asset, reward_asset, false, false);

		#[extrinsic_call]
		cancel_presale(RawOrigin::Root, presale_id);

		// Verify presale was cancelled
		let presale = crate::Presales::<T>::get(presale_id).unwrap();
		assert_eq!(presale.status, PresaleStatus::Cancelled);
	}

	#[benchmark]
	fn add_to_whitelist() {
		let owner: T::AccountId = whitelisted_caller();
		let user: T::AccountId = account("user", 0, 0);
		let payment_asset = get_asset_id::<T>(1);
		let reward_asset = get_asset_id::<T>(2);

		// Create a whitelist presale
		let presale_id = create_test_presale::<T>(&owner, payment_asset, reward_asset, true, false);

		#[extrinsic_call]
		add_to_whitelist(RawOrigin::Signed(owner), presale_id, user.clone());

		// Verify user was whitelisted
		assert!(crate::WhitelistedAccounts::<T>::get(presale_id, &user));
	}

	#[benchmark]
	fn contribute() {
		let caller: T::AccountId = whitelisted_caller();
		// Get next presale ID before creating
		let presale_id = NextPresaleId::<T>::get();
		let presale_treasury = Presale::<T>::presale_account_id(presale_id);

		// Setup assets
		let (payment_asset, reward_asset) = setup_benchmark_assets::<T>(&caller, &presale_treasury);

		// Create presale (will get the presale_id we calculated)
		let _ = create_test_presale::<T>(&caller, payment_asset, reward_asset, false, false);

		let amount: u128 = 10_000u128;

		#[extrinsic_call]
		contribute(RawOrigin::Signed(caller.clone()), presale_id, amount);

		// Verify contribution was recorded
		assert!(crate::Contributions::<T>::get(presale_id, &caller).is_some());
		assert!(crate::TotalRaised::<T>::get(presale_id) > 0);
	}

	#[benchmark]
	fn refund() {
		let caller: T::AccountId = whitelisted_caller();
		// Get next presale ID before creating
		let presale_id = NextPresaleId::<T>::get();
		let presale_treasury = Presale::<T>::presale_account_id(presale_id);

		// Setup assets
		let (payment_asset, reward_asset) = setup_benchmark_assets::<T>(&caller, &presale_treasury);

		// Create presale (will get the presale_id we calculated)
		let _ = create_test_presale::<T>(&caller, payment_asset, reward_asset, false, false);

		// Make a contribution first
		let amount: u128 = 10_000u128;
		let _ =
			Presale::<T>::contribute(RawOrigin::Signed(caller.clone()).into(), presale_id, amount);

		// Verify contribution exists
		assert!(crate::Contributions::<T>::get(presale_id, &caller).is_some());

		#[extrinsic_call]
		refund(RawOrigin::Signed(caller.clone()), presale_id);

		// Verify refund was processed
		let contribution = crate::Contributions::<T>::get(presale_id, &caller).unwrap();
		assert!(contribution.refunded);
	}

	#[benchmark]
	fn claim_vested() {
		let caller: T::AccountId = whitelisted_caller();
		// Get next presale ID before creating
		let presale_id = NextPresaleId::<T>::get();
		let presale_treasury = Presale::<T>::presale_account_id(presale_id);

		// Setup assets
		let (payment_asset, reward_asset) = setup_benchmark_assets::<T>(&caller, &presale_treasury);

		// Mint EXTRA reward tokens to presale treasury to prevent account death
		let extra_reward: T::Balance = 100_000_000_000u128.into();
		let _ = T::Assets::mint_into(reward_asset, &presale_treasury, extra_reward);

		// Create presale WITH vesting (will get the presale_id we calculated)
		let _ = create_test_presale::<T>(&caller, payment_asset, reward_asset, false, true);

		// Make a contribution
		let amount: u128 = 1_000_000u128; // Large enough to reach soft cap
		let _ =
			Presale::<T>::contribute(RawOrigin::Signed(caller.clone()).into(), presale_id, amount);

		// Advance blocks past presale end
		pezframe_system::Pezpallet::<T>::set_block_number(2000u32.into());

		// Finalize presale (requires root)
		let _ = Presale::<T>::finalize_presale(RawOrigin::Root.into(), presale_id);

		// Advance past cliff period
		pezframe_system::Pezpallet::<T>::set_block_number(3000u32.into());

		#[extrinsic_call]
		claim_vested(RawOrigin::Signed(caller.clone()), presale_id);

		// Verify claim was recorded
		let claimed = crate::VestingClaimed::<T>::get(presale_id, &caller);
		assert!(claimed > 0);
	}

	#[benchmark]
	fn refund_cancelled_presale() {
		let caller: T::AccountId = whitelisted_caller();
		// Get next presale ID before creating
		let presale_id = NextPresaleId::<T>::get();
		let presale_treasury = Presale::<T>::presale_account_id(presale_id);

		// Setup assets
		let (payment_asset, reward_asset) = setup_benchmark_assets::<T>(&caller, &presale_treasury);

		// Create presale (will get the presale_id we calculated)
		let _ = create_test_presale::<T>(&caller, payment_asset, reward_asset, false, false);

		// Make a contribution
		let amount: u128 = 10_000u128;
		let _ =
			Presale::<T>::contribute(RawOrigin::Signed(caller.clone()).into(), presale_id, amount);

		// Mint payment tokens to presale treasury for refund
		let refund_amount: T::Balance = 100_000u128.into();
		let _ = T::Assets::mint_into(payment_asset, &presale_treasury, refund_amount);

		// Cancel the presale
		let _ = Presale::<T>::cancel_presale(RawOrigin::Root.into(), presale_id);

		#[extrinsic_call]
		refund_cancelled_presale(RawOrigin::Signed(caller.clone()), presale_id, 0, 100);

		// Verify refund was processed
		let contribution = crate::Contributions::<T>::get(presale_id, &caller).unwrap();
		assert!(contribution.refunded);
	}

	/// Benchmark finalize_presale with variable number of contributors
	/// This is O(N) complexity - critical for proper weight calculation
	#[benchmark]
	fn finalize_presale(n: Linear<1, 100>) {
		let caller: T::AccountId = whitelisted_caller();
		// Get next presale ID before creating
		let presale_id = NextPresaleId::<T>::get();
		let presale_treasury = Presale::<T>::presale_account_id(presale_id);

		// Setup assets with enough for many contributors
		let (payment_asset, reward_asset) = setup_benchmark_assets::<T>(&caller, &presale_treasury);

		// Create presale (will get the presale_id we calculated)
		let _ = create_test_presale::<T>(&caller, payment_asset, reward_asset, false, false);

		// Add n contributors
		for i in 0..n {
			let contributor: T::AccountId = account("contributor", i, 0);

			// Mint payment tokens to contributor
			let contribution_amount: T::Balance = 50_000u128.into();
			let _ = T::Assets::mint_into(payment_asset, &contributor, contribution_amount);

			// Make contribution
			let _ = Presale::<T>::contribute(
				RawOrigin::Signed(contributor).into(),
				presale_id,
				10_000u128,
			);
		}

		// Advance blocks past presale end
		pezframe_system::Pezpallet::<T>::set_block_number(2000u32.into());

		#[extrinsic_call]
		finalize_presale(RawOrigin::Root, presale_id);

		// Verify presale was finalized
		let presale = crate::Presales::<T>::get(presale_id).unwrap();
		assert!(
			presale.status == PresaleStatus::Finalized || presale.status == PresaleStatus::Failed
		);
	}

	/// Benchmark batch_refund_failed_presale with variable batch size
	/// This is also O(N) complexity
	#[benchmark]
	fn batch_refund_failed_presale(n: Linear<1, 100>) {
		let caller: T::AccountId = whitelisted_caller();
		// Get next presale ID before creating
		let presale_id = NextPresaleId::<T>::get();
		let presale_treasury = Presale::<T>::presale_account_id(presale_id);

		// Setup assets
		let (payment_asset, reward_asset) = setup_benchmark_assets::<T>(&caller, &presale_treasury);

		// Create presale with HIGH soft cap (will fail)
		let params = crate::PresaleCreationParams {
			tokens_for_sale: 10_000_000_000u128,
			duration: 1000u32.into(),
			is_whitelist: false,
			limits: crate::ContributionLimits {
				min_contribution: 100u128,
				max_contribution: 10_000_000u128,
				soft_cap: 1_000_000_000_000u128, // very high - will fail
				hard_cap: 2_000_000_000_000u128,
			},
			vesting: None,
			refund_config: crate::RefundConfig {
				grace_period_blocks: 10u32.into(),
				refund_fee_percent: 5u8,
				grace_refund_fee_percent: 2u8,
			},
		};
		let _ = Presale::<T>::create_presale(
			RawOrigin::Signed(caller.clone()).into(),
			payment_asset,
			reward_asset,
			params,
		);

		// Add n contributors (small amounts that won't reach soft cap)
		for i in 0..n {
			let contributor: T::AccountId = account("contributor", i, 0);

			// Mint payment tokens to contributor
			let contribution_amount: T::Balance = 50_000u128.into();
			let _ = T::Assets::mint_into(payment_asset, &contributor, contribution_amount);

			// Make small contribution
			let _ = Presale::<T>::contribute(
				RawOrigin::Signed(contributor).into(),
				presale_id,
				1_000u128,
			);
		}

		// Mint payment tokens to presale treasury for refunds
		let refund_pool: T::Balance = (n as u128 * 10_000u128).into();
		let _ = T::Assets::mint_into(payment_asset, &presale_treasury, refund_pool);

		// Advance blocks past presale end
		pezframe_system::Pezpallet::<T>::set_block_number(2000u32.into());

		// Finalize presale (will mark as Failed due to soft cap not reached)
		let _ = Presale::<T>::finalize_presale(RawOrigin::Root.into(), presale_id);

		// Verify presale failed
		let presale = crate::Presales::<T>::get(presale_id).unwrap();
		assert_eq!(presale.status, PresaleStatus::Failed);

		#[extrinsic_call]
		batch_refund_failed_presale(RawOrigin::Signed(caller), presale_id, 0, n);

		// Verify refunds were processed
		let first_contributor: T::AccountId = account("contributor", 0, 0);
		let contribution = crate::Contributions::<T>::get(presale_id, &first_contributor);
		if let Some(c) = contribution {
			assert!(c.refunded);
		}
	}

	impl_benchmark_test_suite!(Presale, crate::mock::new_test_ext(), crate::mock::Test);
}
