// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-pez-treasury.
//!
//! Two paths carry weight: the one-off activation, and the monthly release. The release is no
//! longer an extrinsic -- it happens in `on_initialize` -- so it is benchmarked as a block of
//! work rather than as a call, and the runtime charges that weight from the hook.

use super::*;
use crate::Pezpallet as PezTreasury;
use pezframe_benchmarking::v2::*;
use pezframe_support::traits::{fungibles::Mutate, EnsureOrigin, Get};
use pezsp_runtime::traits::{Saturating, Zero};

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn activate_distribution() -> Result<(), BenchmarkError> {
		crate::TreasuryStartBlock::<T>::kill();
		crate::HalvingInfo::<T>::kill();
		crate::NextReleaseMonth::<T>::kill();
		crate::DistributionStarted::<T>::kill();

		let origin =
			T::ActivationOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin);

		assert!(crate::DistributionStarted::<T>::get());
		assert!(crate::TreasuryStartBlock::<T>::get().is_some());
		let halving_info = crate::HalvingInfo::<T>::get();
		assert_eq!(halving_info.current_period, 0);
		assert!(!halving_info.monthly_amount.is_zero());

		Ok(())
	}

	#[benchmark]
	fn release_monthly_funds() {
		crate::TreasuryStartBlock::<T>::kill();
		crate::HalvingInfo::<T>::kill();
		crate::NextReleaseMonth::<T>::kill();
		let _ = crate::MonthlyReleases::<T>::clear(u32::MAX, None);

		PezTreasury::<T>::do_initialize_treasury().unwrap();

		let treasury_account = PezTreasury::<T>::treasury_account_id();
		let monthly_amount = PezTreasury::<T>::halving_info().monthly_amount;
		let incentive_amount = monthly_amount * 75u32.into() / 100u32.into();
		let government_amount = monthly_amount.saturating_sub(incentive_amount);

		// The pallet cannot mint; on a real chain the treasury account is funded at genesis.
		// The benchmark harness stands in for that here, with room to spare so the transfer
		// itself is what is measured rather than a balance that runs out.
		let _ = T::Assets::mint_into(
			T::PezAssetId::get(),
			&treasury_account,
			monthly_amount * 10u32.into(),
		);

		// Release 0 is due at the activation block, so move past it to measure the ordinary
		// case: a release that has waited a full month.
		let current_block = pezframe_system::Pezpallet::<T>::block_number();
		let target_block = current_block + crate::BLOCKS_PER_MONTH.into() + 1u32.into();
		pezframe_system::Pezpallet::<T>::set_block_number(target_block);

		#[block]
		{
			PezTreasury::<T>::do_monthly_release().unwrap();
		}

		assert_eq!(PezTreasury::<T>::get_incentive_pot_balance(), incentive_amount);
		assert_eq!(PezTreasury::<T>::get_government_pot_balance(), government_amount);
	}

	#[benchmark]
	fn spend_from_government_pot() -> Result<(), BenchmarkError> {
		crate::TreasuryStartBlock::<T>::kill();
		crate::HalvingInfo::<T>::kill();
		crate::NextReleaseMonth::<T>::kill();
		let _ = crate::MonthlyReleases::<T>::clear(u32::MAX, None);

		PezTreasury::<T>::do_initialize_treasury().unwrap();

		// Fund the treasury as genesis would, then make one release so the government pot
		// holds what a real spend would be drawn from.
		let treasury_account = PezTreasury::<T>::treasury_account_id();
		let monthly_amount = PezTreasury::<T>::halving_info().monthly_amount;
		let _ = T::Assets::mint_into(
			T::PezAssetId::get(),
			&treasury_account,
			monthly_amount * 10u32.into(),
		);
		PezTreasury::<T>::do_monthly_release().unwrap();

		let pot = PezTreasury::<T>::get_government_pot_balance();
		let beneficiary: T::AccountId = whitelisted_caller();
		let origin = T::GovernmentSpendOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, beneficiary.clone(), pot / 2u32.into());

		assert!(PezTreasury::<T>::get_government_pot_balance() < pot);

		Ok(())
	}

	impl_benchmark_test_suite!(PezTreasury, crate::mock::new_test_ext(), crate::mock::Test);
}
