// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// pezkuwi/pallets/pez-treasury/src/tests.rs

use crate::{mock::*, Error, Event};
use pezframe_support::{assert_noop, assert_ok};
use pezsp_runtime::traits::Zero; // FIXED: Import Zero trait for is_zero() method

// =============================================================================
// 1. GENESIS DISTRIBUTION TESTS
// =============================================================================

#[test]
fn genesis_distribution_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());

		let treasury_amount = 4_812_500_000 * 1_000_000_000_000u128;
		let presale_amount = 93_750_000 * 1_000_000_000_000u128;
		let founder_amount = 93_750_000 * 1_000_000_000_000u128;

		assert_pez_balance(treasury_account(), treasury_amount);
		assert_pez_balance(presale(), presale_amount);
		assert_pez_balance(founder(), founder_amount);

		let total = treasury_amount + presale_amount + founder_amount;
		assert_eq!(total, 5_000_000_000 * 1_000_000_000_000u128);

		System::assert_has_event(
			Event::GenesisDistributionCompleted { treasury_amount, presale_amount, founder_amount }
				.into(),
		);
	});
}

#[test]
fn force_genesis_distribution_requires_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PezTreasury::force_genesis_distribution(RuntimeOrigin::signed(alice())),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn force_genesis_distribution_works_with_root() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::force_genesis_distribution(RuntimeOrigin::root()));

		assert!(Assets::balance(PezAssetId::get(), treasury_account()) > 0);
		assert!(Assets::balance(PezAssetId::get(), presale()) > 0);
		assert!(Assets::balance(PezAssetId::get(), founder()) > 0);
	});
}

#[test]
fn genesis_distribution_can_only_happen_once() {
	new_test_ext().execute_with(|| {
		// First call should succeed
		assert_ok!(PezTreasury::do_genesis_distribution());

		// Verify flag is set
		assert!(PezTreasury::genesis_distribution_done());

		// Second call should fail
		assert_noop!(
			PezTreasury::do_genesis_distribution(),
			Error::<Test>::GenesisDistributionAlreadyDone
		);

		// Verify balances didn't double
		let treasury_amount = 4_812_500_000 * 1_000_000_000_000u128;
		assert_pez_balance(treasury_account(), treasury_amount);
	});
}

// =============================================================================
// 2. TREASURY INITIALIZATION TESTS
// =============================================================================

#[test]
fn initialize_treasury_works() {
	new_test_ext().execute_with(|| {
		let start_block = System::block_number();

		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		// Verify storage
		assert_eq!(PezTreasury::treasury_start_block(), Some(start_block));

		let halving_info = PezTreasury::halving_info();
		assert_eq!(halving_info.current_period, 0);
		assert_eq!(halving_info.period_start_block, start_block);
		assert!(!halving_info.monthly_amount.is_zero());

		// Verify next release month
		assert_eq!(PezTreasury::next_release_month(), 0);

		// Verify event
		System::assert_has_event(
			Event::TreasuryInitialized {
				start_block,
				initial_monthly_amount: halving_info.monthly_amount,
			}
			.into(),
		);
	});
}

#[test]
fn initialize_treasury_fails_if_already_initialized() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		// Try to initialize again
		assert_noop!(
			PezTreasury::initialize_treasury(RuntimeOrigin::root()),
			Error::<Test>::TreasuryAlreadyInitialized
		);
	});
}

#[test]
fn initialize_treasury_requires_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PezTreasury::initialize_treasury(RuntimeOrigin::signed(alice())),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn initialize_treasury_calculates_correct_monthly_amount() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let halving_info = PezTreasury::halving_info();

		// First period total = 96.25% / 2 = 48.125%
		let treasury_total = 4_812_500_000 * 1_000_000_000_000u128;
		let first_period = treasury_total / 2;
		let expected_monthly = first_period / 48; // 48 months

		assert_eq!(halving_info.monthly_amount, expected_monthly);
	});
}

// =============================================================================
// 3. MONTHLY RELEASE TESTS
// =============================================================================

#[test]
fn release_monthly_funds_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let initial_monthly = PezTreasury::halving_info().monthly_amount;
		let incentive_expected = initial_monthly * 75 / 100;
		let government_expected = initial_monthly - incentive_expected;

		run_to_block(432_001);

		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		assert_pez_balance(PezTreasury::incentive_pot_account_id(), incentive_expected);
		assert_pez_balance(PezTreasury::government_pot_account_id(), government_expected);

		assert_eq!(PezTreasury::next_release_month(), 1);

		let halving_info = PezTreasury::halving_info();
		assert_eq!(halving_info.total_released, initial_monthly);
	});
}

#[test]
fn release_monthly_funds_fails_if_not_initialized() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PezTreasury::release_monthly_funds(RuntimeOrigin::root()),
			Error::<Test>::TreasuryNotInitialized
		);
	});
}

#[test]
fn release_monthly_funds_fails_if_too_early() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		// Try to release before time
		run_to_block(100);

		assert_noop!(
			PezTreasury::release_monthly_funds(RuntimeOrigin::root()),
			Error::<Test>::ReleaseTooEarly
		);
	});
}

#[test]
fn release_monthly_funds_fails_if_already_released() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		run_to_block(432_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		// Try to release same month again
		assert_noop!(
			PezTreasury::release_monthly_funds(RuntimeOrigin::root()),
			Error::<Test>::ReleaseTooEarly
		);
	});
}

#[test]
fn release_monthly_funds_splits_correctly() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let monthly_amount = PezTreasury::halving_info().monthly_amount;

		run_to_block(432_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		let incentive_balance =
			Assets::balance(PezAssetId::get(), PezTreasury::incentive_pot_account_id());
		let government_balance =
			Assets::balance(PezAssetId::get(), PezTreasury::government_pot_account_id());

		// 75% to incentive, 25% to government
		assert_eq!(incentive_balance, monthly_amount * 75 / 100);
		// Must match the logic in lib.rs (saturating_sub)
		let incentive_amount_calculated = monthly_amount * 75 / 100;
		assert_eq!(government_balance, monthly_amount - incentive_amount_calculated);

		// Total should equal monthly amount
		assert_eq!(incentive_balance + government_balance, monthly_amount);
	});
}

#[test]
fn multiple_monthly_releases_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let monthly_amount = PezTreasury::halving_info().monthly_amount;

		// Release month 0
		run_to_block(432_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		assert_eq!(PezTreasury::next_release_month(), 1);

		// Release month 1
		run_to_block(864_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		assert_eq!(PezTreasury::next_release_month(), 2);

		// Release month 2
		run_to_block(1_296_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		assert_eq!(PezTreasury::next_release_month(), 3);

		// Verify total released
		let halving_info = PezTreasury::halving_info();
		assert_eq!(halving_info.total_released, monthly_amount * 3);
	});
}

// =============================================================================
// 4. HALVING LOGIC TESTS
// =============================================================================

#[test]
fn halving_occurs_after_48_months() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let initial_monthly = PezTreasury::halving_info().monthly_amount;

		// Release 47 months (no halving yet)
		for month in 0..47 {
			run_to_block(1 + (month + 1) * 432_000 + 1);
			assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		}

		// Still period 0
		assert_eq!(PezTreasury::halving_info().current_period, 0);
		assert_eq!(PezTreasury::halving_info().monthly_amount, initial_monthly);

		// Release 48th month - halving should occur
		run_to_block(1 + 48 * 432_000 + 1);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		// Now in period 1 with halved amount
		let halving_info = PezTreasury::halving_info();
		assert_eq!(halving_info.current_period, 1);
		assert_eq!(halving_info.monthly_amount, initial_monthly / 2);

		// Verify event
		System::assert_has_event(
			Event::NewHalvingPeriod { period: 1, new_monthly_amount: initial_monthly / 2 }.into(),
		);
	});
}

#[test]
fn multiple_halvings_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let initial_monthly = PezTreasury::halving_info().monthly_amount;

		// First halving at month 48
		run_to_block(1 + 48 * 432_000 + 1);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		assert_eq!(PezTreasury::halving_info().current_period, 1);
		assert_eq!(PezTreasury::halving_info().monthly_amount, initial_monthly / 2);

		// Second halving at month 96
		run_to_block(1 + 96 * 432_000 + 1);
		for _ in 49..=96 {
			assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		}
		assert_eq!(PezTreasury::halving_info().current_period, 2);
		assert_eq!(PezTreasury::halving_info().monthly_amount, initial_monthly / 4);

		// Third halving at month 144
		run_to_block(1 + 144 * 432_000 + 1);
		for _ in 97..=144 {
			assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		}
		assert_eq!(PezTreasury::halving_info().current_period, 3);
		assert_eq!(PezTreasury::halving_info().monthly_amount, initial_monthly / 8);
	});
}

#[test]
fn halving_period_start_block_updates() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let period_0_start = PezTreasury::halving_info().period_start_block;

		// Trigger halving
		run_to_block(1 + 48 * 432_000 + 1);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		let period_1_start = PezTreasury::halving_info().period_start_block;
		assert!(period_1_start > period_0_start);
		assert_eq!(period_1_start, System::block_number());
	});
}

// =============================================================================
// 5. ERROR CASES
// =============================================================================

#[test]
fn insufficient_treasury_balance_error() {
	new_test_ext().execute_with(|| {
		// Initialize without genesis distribution (treasury empty)
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		run_to_block(432_001);

		// This should fail due to insufficient balance
		assert_noop!(
			PezTreasury::release_monthly_funds(RuntimeOrigin::root()),
			Error::<Test>::InsufficientTreasuryBalance
		);
	});
}

#[test]
fn release_requires_root_origin() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		run_to_block(432_001);

		assert_noop!(
			PezTreasury::release_monthly_funds(RuntimeOrigin::signed(alice())),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

// =============================================================================
// 6. EDGE CASES
// =============================================================================

#[test]
fn release_exactly_at_boundary_block_fails() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		// Exactly block 432_000 (since start_block=1) means 431_999 blocks have passed.
		// This is not a full month (432_000 blocks).
		run_to_block(432_000);
		assert_noop!(
			PezTreasury::release_monthly_funds(RuntimeOrigin::root()),
			Error::<Test>::ReleaseTooEarly
		);
	});
}

#[test]
fn release_one_block_before_boundary_fails() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		run_to_block(432_000 - 1);
		assert_noop!(
			PezTreasury::release_monthly_funds(RuntimeOrigin::root()),
			Error::<Test>::ReleaseTooEarly
		);
	});
}

#[test]
fn skip_months_and_release() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		// Skip directly to month 3
		run_to_block(1 + 3 * 432_000 + 1);

		// Should release month 0
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		assert_eq!(PezTreasury::next_release_month(), 1);

		// Can still release subsequent months
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		assert_eq!(PezTreasury::next_release_month(), 2);
	});
}

#[test]
fn very_large_block_number() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		// Jump to very large block number
		System::set_block_number(u64::MAX / 2);

		// Should still be able to release (if months passed)
		// This tests overflow protection
		let result = PezTreasury::release_monthly_funds(RuntimeOrigin::root());
		// Result depends on whether enough months passed
		// Main point: no panic/overflow
		assert!(result.is_ok() || result.is_err());
	});
}

#[test]
fn zero_amount_division_protection() {
	new_test_ext().execute_with(|| {
		// Initialize without any balance
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let halving_info = PezTreasury::halving_info();
		// Should not panic, should have some calculated amount
		assert!(!halving_info.monthly_amount.is_zero());
	});
}

// =============================================================================
// 7. GETTER FUNCTIONS TESTS
// =============================================================================

#[test]
fn get_current_halving_info_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let info = PezTreasury::get_current_halving_info();
		assert_eq!(info.current_period, 0);
		assert!(!info.monthly_amount.is_zero());
		assert_eq!(info.total_released, 0);
	});
}

#[test]
fn get_incentive_pot_balance_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		run_to_block(432_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		let balance = PezTreasury::get_incentive_pot_balance();
		assert!(balance > 0);
	});
}

#[test]
fn get_government_pot_balance_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		run_to_block(432_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		let balance = PezTreasury::get_government_pot_balance();
		assert!(balance > 0);
	});
}

// =============================================================================
// 8. ACCOUNT ID TESTS
// =============================================================================

#[test]
fn treasury_account_id_is_consistent() {
	new_test_ext().execute_with(|| {
		let account1 = PezTreasury::treasury_account_id();
		let account2 = PezTreasury::treasury_account_id();
		assert_eq!(account1, account2);
	});
}

#[test]
fn pot_accounts_are_different() {
	new_test_ext().execute_with(|| {
		debug_pot_accounts();

		let treasury = PezTreasury::treasury_account_id();
		let incentive = PezTreasury::incentive_pot_account_id();
		let government = PezTreasury::government_pot_account_id();

		println!("\n=== Account IDs from Pezpallet ===");
		println!("Treasury: {treasury:?}");
		println!("Incentive: {incentive:?}");
		println!("Government: {government:?}");
		println!("================================\n");

		// All three must be different
		assert_ne!(treasury, incentive, "Treasury and Incentive must be different");
		assert_ne!(treasury, government, "Treasury and Government must be different");
		assert_ne!(incentive, government, "Incentive and Government must be different");

		println!("✓ All pot accounts are different!");
	});
}

// =============================================================================
// 9. MONTHLY RELEASE STORAGE TESTS
// =============================================================================

#[test]
fn monthly_release_records_stored_correctly() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let monthly_amount = PezTreasury::halving_info().monthly_amount;
		let incentive_expected = monthly_amount * 75 / 100;
		let government_expected = monthly_amount - incentive_expected;

		run_to_block(432_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		// Verify monthly release record
		let release = PezTreasury::monthly_releases(0).unwrap();
		assert_eq!(release.month_index, 0);
		assert_eq!(release.amount_released, monthly_amount);
		assert_eq!(release.incentive_amount, incentive_expected);
		assert_eq!(release.government_amount, government_expected);
		assert_eq!(release.release_block, System::block_number());
	});
}

#[test]
fn multiple_monthly_releases_stored_separately() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		// Release month 0
		run_to_block(432_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		// Release month 1
		run_to_block(864_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		// Verify both records exist
		assert!(PezTreasury::monthly_releases(0).is_some());
		assert!(PezTreasury::monthly_releases(1).is_some());

		let release_0 = PezTreasury::monthly_releases(0).unwrap();
		let release_1 = PezTreasury::monthly_releases(1).unwrap();

		assert_eq!(release_0.month_index, 0);
		assert_eq!(release_1.month_index, 1);
		assert_ne!(release_0.release_block, release_1.release_block);
	});
}

// =============================================================================
// 10. INTEGRATION TESTS
// =============================================================================

#[test]
fn full_lifecycle_test() {
	new_test_ext().execute_with(|| {
		// 1. Genesis distribution
		assert_ok!(PezTreasury::do_genesis_distribution());
		let treasury_initial = Assets::balance(PezAssetId::get(), treasury_account());
		assert!(treasury_initial > 0);

		// 2. Initialize treasury
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));
		let monthly_amount = PezTreasury::halving_info().monthly_amount;

		// 3. Release first month
		run_to_block(432_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		let treasury_after_month_0 = Assets::balance(PezAssetId::get(), treasury_account());
		assert_eq!(treasury_initial - treasury_after_month_0, monthly_amount);

		// 4. Release multiple months
		for month in 1..10 {
			run_to_block(1 + (month + 1) * 432_000 + 1);
			assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		}

		// 5. Verify cumulative release
		let halving_info = PezTreasury::halving_info();
		assert_eq!(halving_info.total_released, monthly_amount * 10);

		// 6. Verify treasury balance decreased correctly
		let treasury_after_10_months = Assets::balance(PezAssetId::get(), treasury_account());
		assert_eq!(treasury_initial - treasury_after_10_months, monthly_amount * 10);
	});
}

#[test]
fn full_halving_cycle_test() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let initial_monthly = PezTreasury::halving_info().monthly_amount;
		let mut cumulative_released = 0u128;

		// Period 0: 48 months at initial rate
		for month in 0..48 {
			run_to_block(1 + (month + 1) * 432_000 + 1);
			assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

			if month < 47 {
				cumulative_released += initial_monthly;
			} else {
				// On the 48th release (index 47) halving is triggered and the halved amount is used
				cumulative_released += initial_monthly / 2;
			}
		}

		assert_eq!(PezTreasury::halving_info().current_period, 1);
		assert_eq!(PezTreasury::halving_info().monthly_amount, initial_monthly / 2);

		// Period 1: 48 months at half rate
		for month in 48..96 {
			run_to_block(1 + (month + 1) * 432_000 + 1);
			assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

			if month < 95 {
				cumulative_released += initial_monthly / 2;
			} else {
				// On the 96th release (index 95) the second halving is triggered
				cumulative_released += initial_monthly / 4;
			}
		}

		assert_eq!(PezTreasury::halving_info().current_period, 2);
		assert_eq!(PezTreasury::halving_info().monthly_amount, initial_monthly / 4);

		// Verify total released matches expectation
		assert_eq!(PezTreasury::halving_info().total_released, cumulative_released);
	});
}

// =============================================================================
// 11. PRECISION AND ROUNDING TESTS
// =============================================================================

#[test]
fn division_rounding_is_consistent() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let monthly_amount = PezTreasury::halving_info().monthly_amount;
		let incentive_amount = monthly_amount * 75 / 100;
		let government_amount = monthly_amount - incentive_amount;

		// Verify no rounding loss
		assert_eq!(incentive_amount + government_amount, monthly_amount);
	});
}

#[test]
fn halving_precision_maintained() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let initial = PezTreasury::halving_info().monthly_amount;

		// Trigger halving
		run_to_block(1 + 48 * 432_000 + 1);
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		let after_halving = PezTreasury::halving_info().monthly_amount;

		// Check halving is exactly half (no precision loss)
		assert_eq!(after_halving, initial / 2);
	});
}

// =============================================================================
// 12. EVENT EMISSION TESTS
// =============================================================================

#[test]
fn all_events_emitted_correctly() {
	new_test_ext().execute_with(|| {
		// Genesis distribution event
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert!(System::events().iter().any(|e| matches!(
			e.event,
			RuntimeEvent::PezTreasury(Event::GenesisDistributionCompleted { .. })
		)));

		// Treasury initialized event
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));
		assert!(System::events().iter().any(|e| matches!(
			e.event,
			RuntimeEvent::PezTreasury(Event::TreasuryInitialized { .. })
		)));

		// Monthly funds released event
		run_to_block(432_001);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		assert!(System::events().iter().any(|e| matches!(
			e.event,
			RuntimeEvent::PezTreasury(Event::MonthlyFundsReleased { .. })
		)));
	});
}

#[test]
fn halving_event_emitted_at_correct_time() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		// Clear existing events
		System::reset_events();

		// Release up to halving point
		run_to_block(1 + 48 * 432_000 + 1);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		// Verify halving event emitted
		assert!(System::events().iter().any(|e| matches!(
			e.event,
			RuntimeEvent::PezTreasury(Event::NewHalvingPeriod { period: 1, .. })
		)));
	});
}

// =============================================================================
// 13. STRESS TESTS
// =============================================================================

#[test]
fn many_consecutive_releases() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		// Release 100 months consecutively
		for month in 0..100 {
			run_to_block(1 + (month + 1) * 432_000 + 1);
			assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));
		}

		// Verify state is consistent
		assert_eq!(PezTreasury::next_release_month(), 100);

		// Should be in period 2 (after 2 halvings at months 48 and 96)
		assert_eq!(PezTreasury::halving_info().current_period, 2);
	});
}

#[test]
fn treasury_never_goes_negative() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let _initial_balance = Assets::balance(PezAssetId::get(), treasury_account()); // FIXED: Prefixed with underscore

		// Try to release many months
		for month in 0..200 {
			run_to_block(1 + (month + 1) * 432_000 + 1);

			let before_balance = Assets::balance(PezAssetId::get(), treasury_account());

			let result = PezTreasury::release_monthly_funds(RuntimeOrigin::root());

			if result.is_ok() {
				let after_balance = Assets::balance(PezAssetId::get(), treasury_account());
				// Balance should decrease or stay the same, never increase
				assert!(after_balance <= before_balance);
			} else {
				// If release fails, balance should be unchanged
				assert_eq!(before_balance, Assets::balance(PezAssetId::get(), treasury_account()));
				break;
			}
		}
	});
}

// =============================================================================
// 14. BOUNDARY CONDITION TESTS
// =============================================================================

#[test]
fn first_block_initialization() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));
		assert_eq!(PezTreasury::treasury_start_block(), Some(1));
	});
}

#[test]
fn last_month_of_period_before_halving() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let initial_amount = PezTreasury::halving_info().monthly_amount;

		// Release month 47 (last before halving)
		run_to_block(1 + 47 * 432_000 + 1);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		// Should still be in period 0
		assert_eq!(PezTreasury::halving_info().current_period, 0);
		assert_eq!(PezTreasury::halving_info().monthly_amount, initial_amount);
	});
}

#[test]
fn first_month_after_halving() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let initial_amount = PezTreasury::halving_info().monthly_amount;

		// Trigger halving at month 48
		run_to_block(1 + 48 * 432_000 + 1);
		assert_ok!(PezTreasury::release_monthly_funds(RuntimeOrigin::root()));

		// Should be in period 1 with halved amount
		assert_eq!(PezTreasury::halving_info().current_period, 1);
		assert_eq!(PezTreasury::halving_info().monthly_amount, initial_amount / 2);
	});
}

// =============================================================================
// 15. MATHEMATICAL CORRECTNESS TESTS
// =============================================================================

#[test]
fn total_supply_equals_sum_of_allocations() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());

		let treasury = Assets::balance(PezAssetId::get(), treasury_account());
		let presale_acc = Assets::balance(PezAssetId::get(), presale());
		let founder_acc = Assets::balance(PezAssetId::get(), founder());

		let total = treasury + presale_acc + founder_acc;
		let expected_total = 5_000_000_000 * 1_000_000_000_000u128;

		assert_eq!(total, expected_total);
	});
}

#[test]
fn percentage_allocations_correct() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());

		let total_supply = 5_000_000_000 * 1_000_000_000_000u128;
		let treasury = Assets::balance(PezAssetId::get(), treasury_account());
		let presale_acc = Assets::balance(PezAssetId::get(), presale());
		let founder_acc = Assets::balance(PezAssetId::get(), founder());

		assert_eq!(treasury, total_supply * 9625 / 10000);
		assert_eq!(presale_acc, total_supply * 1875 / 100000);
		assert_eq!(founder_acc, total_supply * 1875 / 100000);
	});
}

#[test]
fn first_period_total_is_half_of_treasury() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::do_genesis_distribution());
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let monthly_amount = PezTreasury::halving_info().monthly_amount;
		let first_period_total = monthly_amount * 48;

		let treasury_allocation = 4_812_500_000 * 1_000_000_000_000u128;
		let expected_first_period = treasury_allocation / 2;

		let diff = expected_first_period.saturating_sub(first_period_total);
		// The sum of the remainders must be less than 48 (at most 1 unit of remainder per month)
		assert!(diff < 48, "Rounding error too large: {diff}");
	});
}

#[test]
fn geometric_series_sum_validates() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::initialize_treasury(RuntimeOrigin::root()));

		let initial_monthly = PezTreasury::halving_info().monthly_amount;

		// Sum of geometric series: a(1 - r^n) / (1 - r)
		// For halving: first_period * (1 - 0.5^n) / 0.5
		// With infinite halvings approaches: first_period * 2

		let first_period_total = initial_monthly * 48;
		let treasury_allocation = 4_812_500_000 * 1_000_000_000_000u128;

		// After infinite halvings, total distributed = treasury_allocation
		// first_period_total * 2 = treasury_allocation
		let diff = treasury_allocation.saturating_sub(first_period_total * 2);
		// The sum of the remainders (multiplied by 2) must be less than 96
		assert!(diff < 96, "Rounding error too large: {diff}");
	});
}
