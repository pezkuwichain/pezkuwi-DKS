// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// pezkuwi/pallets/pez-treasury/src/benchmarking.rs

use super::*;
use crate::Pezpallet as PezTreasury;
use pezframe_benchmarking::v2::*;
use pezframe_support::traits::{
	fungibles::{Inspect, Mutate},
	Get, // BUG FIXED: this trait was added for the .get() function
};
use pezframe_system::RawOrigin;
use pezsp_runtime::traits::{Saturating, Zero};

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn initialize_treasury() {
		crate::TreasuryStartBlock::<T>::kill();
		crate::HalvingInfo::<T>::kill();
		crate::NextReleaseMonth::<T>::kill();

		#[extrinsic_call]
		initialize_treasury(RawOrigin::Root);

		assert!(crate::TreasuryStartBlock::<T>::get().is_some());
		let halving_info = crate::HalvingInfo::<T>::get();
		assert_eq!(halving_info.current_period, 0);
		assert!(!halving_info.monthly_amount.is_zero());
	}

	#[benchmark]
	fn force_genesis_distribution() {
		// Clear the flag to allow benchmark run (tests the new storage operation)
		crate::GenesisDistributionDone::<T>::kill();

		#[block]
		{
			PezTreasury::<T>::do_genesis_distribution().unwrap();
		}

		let treasury_account = PezTreasury::<T>::treasury_account_id();
		let presale_account = T::PresaleAccount::get();
		let founder_account = T::FounderAccount::get();

		assert!(!T::Assets::balance(T::PezAssetId::get(), &treasury_account).is_zero());
		assert!(!T::Assets::balance(T::PezAssetId::get(), &presale_account).is_zero());
		assert!(!T::Assets::balance(T::PezAssetId::get(), &founder_account).is_zero());
	}

	#[benchmark]
	fn release_monthly_funds() {
		// Setup
		crate::TreasuryStartBlock::<T>::kill();
		crate::HalvingInfo::<T>::kill();
		crate::NextReleaseMonth::<T>::kill();
		crate::GenesisDistributionDone::<T>::kill();
		// Using `clear` instead of the deprecated `remove_all`.
		let _ = crate::MonthlyReleases::<T>::clear(u32::MAX, None);

		// First do genesis distribution to properly fund the treasury
		PezTreasury::<T>::do_genesis_distribution().unwrap();
		PezTreasury::<T>::do_initialize_treasury().unwrap();

		let treasury_account = PezTreasury::<T>::treasury_account_id();
		let initial_monthly_amount = PezTreasury::<T>::halving_info().monthly_amount;
		let incentive_amount = initial_monthly_amount * 75u32.into() / 100u32.into();
		let government_amount = initial_monthly_amount.saturating_sub(incentive_amount);

		// Ensure treasury has MORE than enough balance for the release
		// Mint additional 10x the monthly amount to ensure sufficient balance
		let _ = T::Assets::mint_into(
			T::PezAssetId::get(),
			&treasury_account,
			initial_monthly_amount * 10u32.into(),
		);

		let current_block = pezframe_system::Pezpallet::<T>::block_number();
		let target_block = current_block + crate::BLOCKS_PER_MONTH.into() + 1u32.into();
		pezframe_system::Pezpallet::<T>::set_block_number(target_block);

		#[extrinsic_call]
		release_monthly_funds(RawOrigin::Root);

		assert_eq!(PezTreasury::<T>::get_incentive_pot_balance(), incentive_amount);
		assert_eq!(PezTreasury::<T>::get_government_pot_balance(), government_amount);
	}

	impl_benchmark_test_suite!(PezTreasury, crate::mock::new_test_ext(), crate::mock::Test);
}
