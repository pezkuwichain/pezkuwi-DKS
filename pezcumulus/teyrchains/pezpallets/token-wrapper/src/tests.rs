// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::mock::*;
use pezframe_support::{assert_noop, assert_ok};

#[test]
fn wrap_works() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let amount = 1000;

		assert_eq!(Balances::free_balance(user), 10000);
		assert_eq!(Assets::balance(0, user), 0);

		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user), amount));

		assert_eq!(Balances::free_balance(user), 10000 - amount);
		assert_eq!(Assets::balance(0, user), amount);
		assert_eq!(TokenWrapper::total_locked(), amount);
	});
}

#[test]
fn unwrap_works() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let amount = 1000;

		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user), amount));
		let native_balance = Balances::free_balance(user);

		assert_ok!(TokenWrapper::unwrap(RuntimeOrigin::signed(user), amount));

		assert_eq!(Balances::free_balance(user), native_balance + amount);
		assert_eq!(Assets::balance(0, user), 0);
		assert_eq!(TokenWrapper::total_locked(), 0);
	});
}

#[test]
fn wrap_fails_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let amount = 20000;

		assert_noop!(
			TokenWrapper::wrap(RuntimeOrigin::signed(user), amount),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn unwrap_fails_insufficient_wrapped_balance() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let amount = 1000;

		assert_noop!(
			TokenWrapper::unwrap(RuntimeOrigin::signed(user), amount),
			Error::<Test>::InsufficientWrappedBalance
		);
	});
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[test]
fn wrap_fails_zero_amount() {
	new_test_ext().execute_with(|| {
		let user = 1;

		assert_noop!(TokenWrapper::wrap(RuntimeOrigin::signed(user), 0), Error::<Test>::ZeroAmount);
	});
}

#[test]
fn unwrap_fails_zero_amount() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let amount = 1000;

		// First wrap some tokens
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user), amount));

		// Try to unwrap zero
		assert_noop!(
			TokenWrapper::unwrap(RuntimeOrigin::signed(user), 0),
			Error::<Test>::ZeroAmount
		);
	});
}

#[test]
fn multi_user_concurrent_wrap_unwrap() {
	new_test_ext().execute_with(|| {
		let user1 = 1;
		let user2 = 2;
		let user3 = 3;

		let amount1 = 1000;
		let amount2 = 2000;
		let amount3 = 1500;

		// All users wrap
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user1), amount1));
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user2), amount2));
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user3), amount3));

		// Verify balances
		assert_eq!(Assets::balance(0, user1), amount1);
		assert_eq!(Assets::balance(0, user2), amount2);
		assert_eq!(Assets::balance(0, user3), amount3);

		// Verify total locked
		assert_eq!(TokenWrapper::total_locked(), amount1 + amount2 + amount3);

		// User 2 unwraps
		assert_ok!(TokenWrapper::unwrap(RuntimeOrigin::signed(user2), amount2));
		assert_eq!(Assets::balance(0, user2), 0);
		assert_eq!(TokenWrapper::total_locked(), amount1 + amount3);

		// User 1 and 3 still have their wrapped tokens
		assert_eq!(Assets::balance(0, user1), amount1);
		assert_eq!(Assets::balance(0, user3), amount3);
	});
}

#[test]
fn multiple_wrap_operations_same_user() {
	new_test_ext().execute_with(|| {
		let user = 1;

		// Multiple wraps
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user), 100));
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user), 200));
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user), 300));

		// Verify accumulated balance
		assert_eq!(Assets::balance(0, user), 600);
		assert_eq!(TokenWrapper::total_locked(), 600);

		// Partial unwrap
		assert_ok!(TokenWrapper::unwrap(RuntimeOrigin::signed(user), 250));
		assert_eq!(Assets::balance(0, user), 350);
		assert_eq!(TokenWrapper::total_locked(), 350);
	});
}

#[test]
fn events_emitted_correctly() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let amount = 1000;

		// Wrap and check event
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user), amount));
		System::assert_has_event(Event::Wrapped { who: user, amount }.into());

		// Unwrap and check event
		assert_ok!(TokenWrapper::unwrap(RuntimeOrigin::signed(user), amount));
		System::assert_has_event(Event::Unwrapped { who: user, amount }.into());
	});
}

#[test]
fn total_locked_tracking_accuracy() {
	new_test_ext().execute_with(|| {
		assert_eq!(TokenWrapper::total_locked(), 0);

		let user1 = 1;
		let user2 = 2;

		// User 1 wraps
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user1), 1000));
		assert_eq!(TokenWrapper::total_locked(), 1000);

		// User 2 wraps
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user2), 500));
		assert_eq!(TokenWrapper::total_locked(), 1500);

		// User 1 unwraps partially
		assert_ok!(TokenWrapper::unwrap(RuntimeOrigin::signed(user1), 300));
		assert_eq!(TokenWrapper::total_locked(), 1200);

		// User 2 unwraps all
		assert_ok!(TokenWrapper::unwrap(RuntimeOrigin::signed(user2), 500));
		assert_eq!(TokenWrapper::total_locked(), 700);

		// User 1 unwraps remaining
		assert_ok!(TokenWrapper::unwrap(RuntimeOrigin::signed(user1), 700));
		assert_eq!(TokenWrapper::total_locked(), 0);
	});
}

#[test]
fn large_amount_wrap_unwrap() {
	new_test_ext().execute_with(|| {
		let user = 1;
		// User has 10000 initial balance
		let large_amount = 9000; // Leave some for existential deposit

		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user), large_amount));
		assert_eq!(Assets::balance(0, user), large_amount);
		assert_eq!(TokenWrapper::total_locked(), large_amount);

		assert_ok!(TokenWrapper::unwrap(RuntimeOrigin::signed(user), large_amount));
		assert_eq!(Assets::balance(0, user), 0);
		assert_eq!(TokenWrapper::total_locked(), 0);
	});
}

#[test]
fn pezpallet_account_balance_consistency() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let amount = 1000;
		let pezpallet_account = TokenWrapper::account_id();

		let initial_pallet_balance = Balances::free_balance(pezpallet_account);

		// Wrap - pezpallet account should receive native tokens
		assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(user), amount));
		assert_eq!(Balances::free_balance(pezpallet_account), initial_pallet_balance + amount);

		// Unwrap - pezpallet account should release native tokens
		assert_ok!(TokenWrapper::unwrap(RuntimeOrigin::signed(user), amount));
		assert_eq!(Balances::free_balance(pezpallet_account), initial_pallet_balance);
	});
}

#[test]
fn wrap_unwrap_maintains_1_to_1_backing() {
	new_test_ext().execute_with(|| {
		let users = [1, 2, 3];
		let amounts = [1000, 2000, 1500];

		// All users wrap
		for (user, amount) in users.iter().zip(amounts.iter()) {
			assert_ok!(TokenWrapper::wrap(RuntimeOrigin::signed(*user), *amount));
		}

		let total_wrapped = amounts.iter().sum::<u128>();
		let pezpallet_account = TokenWrapper::account_id();
		let pezpallet_balance = Balances::free_balance(pezpallet_account);

		// Pezpallet should hold exactly the amount of wrapped tokens
		// (Note: may include existential deposit, so check >= total_wrapped)
		assert!(pezpallet_balance >= total_wrapped);
		assert_eq!(TokenWrapper::total_locked(), total_wrapped);

		// Verify total supply matches
		assert_eq!(Assets::total_issuance(0), total_wrapped);
	});
}
