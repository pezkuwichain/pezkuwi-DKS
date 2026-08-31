// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Tests for pezpallet-pez-treasury.
//!
//! The pallet has one job and two hard rules. The job is to move a fixed allocation out of the
//! treasury account and into the two pots, once a month, halving every forty-eight releases.
//! The rules are that it may never create PEZ, and that nobody -- not root -- may bring a
//! release forward, hold one back, or hand one out by hand. Most of what follows is there to
//! keep those two rules honest.

use crate::{mock::*, Error, Event, BLOCKS_PER_MONTH, HALVING_PERIOD_MONTHS, TREASURY_ALLOCATION};
use pezframe_support::{assert_noop, assert_ok};
use pezsp_runtime::traits::Zero;

/// Blocks per month as a block number.
const MONTH: u64 = BLOCKS_PER_MONTH as u64;

/// The amount release `index` is owed, computed here independently of the pallet.
fn expected_amount(index: u32) -> u128 {
	let initial = (TREASURY_ALLOCATION / 2) / HALVING_PERIOD_MONTHS as u128;
	let period = index / HALVING_PERIOD_MONTHS;
	if period >= 128 {
		0
	} else {
		initial >> period
	}
}

/// Activate at the current block, with the treasury funded as genesis would fund it.
fn activate() {
	fund_treasury();
	assert_ok!(PezTreasury::activate_distribution(RuntimeOrigin::root()));
}

// =============================================================================
// 1. THE PALLET CANNOT CREATE PEZ
// =============================================================================

#[test]
fn activation_mints_nothing() {
	new_test_ext().execute_with(|| {
		fund_treasury();
		let supply_before = pez_total_supply();

		assert_ok!(PezTreasury::activate_distribution(RuntimeOrigin::root()));

		assert_eq!(pez_total_supply(), supply_before);
	});
}

#[test]
fn supply_is_unchanged_across_the_whole_schedule() {
	new_test_ext().execute_with(|| {
		activate();
		let supply = pez_total_supply();
		assert_eq!(supply, TREASURY_ALLOCATION);

		// Two hundred months, which is past four halvings.
		for month in 0..200u64 {
			jump_to_block(1 + month * MONTH);
			run_blocks(1);
			assert_eq!(pez_total_supply(), supply, "supply moved at month {month}");
		}
	});
}

#[test]
fn the_call_surface_is_three_calls_and_none_of_them_mints() {
	// A compile-time statement of the surface. Adding a call makes this stop matching and the
	// test fails to build -- which is the point: a new way into this pallet has to be looked
	// at, not merged. The three that exist move nothing into being: one starts the schedule,
	// the other two move already-released PEZ out of the government and incentive pots.
	use crate::pezpallet::Call;
	fn assert_exhaustive(call: Call<Test>) {
		match call {
			Call::activate_distribution {} => {},
			Call::spend_from_government_pot { .. } => {},
			Call::pay_from_incentive_pot { .. } => {},
			Call::__Ignore(_, _) => {},
		}
	}
	let _ = assert_exhaustive;
}

// =============================================================================
// 2. ACTIVATION
// =============================================================================

#[test]
fn activation_sets_up_the_schedule() {
	new_test_ext().execute_with(|| {
		let start_block = System::block_number();
		activate();

		assert!(PezTreasury::distribution_started());
		assert_eq!(PezTreasury::treasury_start_block(), Some(start_block));
		assert_eq!(PezTreasury::next_release_month(), 0);

		let halving_info = PezTreasury::halving_info();
		assert_eq!(halving_info.current_period, 0);
		assert_eq!(halving_info.period_start_block, start_block);
		assert_eq!(halving_info.monthly_amount, expected_amount(0));
		assert!(halving_info.total_released.is_zero());

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
fn activation_rejects_anyone_but_the_activation_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PezTreasury::activate_distribution(RuntimeOrigin::signed(alice())),
			pezsp_runtime::DispatchError::BadOrigin
		);
		assert!(!PezTreasury::distribution_started());
	});
}

#[test]
fn the_latch_only_turns_once() {
	new_test_ext().execute_with(|| {
		activate();
		let start_block = PezTreasury::treasury_start_block();

		jump_to_block(500);
		assert_noop!(
			PezTreasury::activate_distribution(RuntimeOrigin::root()),
			Error::<Test>::TreasuryAlreadyInitialized
		);

		// A second message must not move the start block; that would reset the whole schedule.
		assert_eq!(PezTreasury::treasury_start_block(), start_block);
	});
}

#[test]
fn nothing_is_released_before_activation() {
	new_test_ext().execute_with(|| {
		fund_treasury();

		jump_to_block(10 * MONTH);
		run_blocks(5);

		assert_eq!(PezTreasury::next_release_month(), 0);
		assert!(PezTreasury::monthly_releases(0).is_none());
		assert!(PezTreasury::get_incentive_pot_balance().is_zero());
		assert!(PezTreasury::get_government_pot_balance().is_zero());
	});
}

#[test]
fn the_first_release_falls_in_the_era_of_activation() {
	new_test_ext().execute_with(|| {
		activate();

		// The block after activation, not a month later.
		run_blocks(1);

		let release = PezTreasury::monthly_releases(0).expect("release 0 was not made");
		assert_eq!(release.month_index, 0);
		assert_eq!(release.amount_released, expected_amount(0));
		assert_eq!(PezTreasury::next_release_month(), 1);
	});
}

// =============================================================================
// 3. THE SCHEDULE
// =============================================================================

#[test]
fn release_one_waits_a_full_month() {
	new_test_ext().execute_with(|| {
		activate();
		run_blocks(1); // release 0
		assert_eq!(PezTreasury::next_release_month(), 1);

		// Release 1 is due at block `1 + MONTH`. Land the hook one block short of it.
		jump_to_block(1 + MONTH - 2);
		run_blocks(1);
		assert_eq!(System::block_number(), 1 + MONTH - 1);
		assert_eq!(PezTreasury::next_release_month(), 1);
		assert!(PezTreasury::monthly_releases(1).is_none());

		// The month is up.
		run_blocks(1);
		assert_eq!(PezTreasury::next_release_month(), 2);
		assert!(PezTreasury::monthly_releases(1).is_some());
	});
}

#[test]
fn each_release_splits_seventy_five_twenty_five() {
	new_test_ext().execute_with(|| {
		activate();
		let monthly = expected_amount(0);
		let incentive = monthly * 75 / 100;
		let government = monthly - incentive;

		run_blocks(1);

		assert_pez_balance(PezTreasury::incentive_pot_account_id(), incentive);
		assert_pez_balance(PezTreasury::government_pot_account_id(), government);
		assert_eq!(incentive + government, monthly, "the split must lose nothing");

		let release = PezTreasury::monthly_releases(0).unwrap();
		assert_eq!(release.incentive_amount, incentive);
		assert_eq!(release.government_amount, government);
	});
}

#[test]
fn releases_are_recorded_one_per_month() {
	new_test_ext().execute_with(|| {
		activate();
		run_blocks(1);
		jump_to_block(1 + MONTH);
		run_blocks(1);

		let zero = PezTreasury::monthly_releases(0).unwrap();
		let one = PezTreasury::monthly_releases(1).unwrap();
		assert_eq!(zero.month_index, 0);
		assert_eq!(one.month_index, 1);
		assert_ne!(zero.release_block, one.release_block);
	});
}

#[test]
fn a_month_only_pays_once() {
	new_test_ext().execute_with(|| {
		activate();
		run_blocks(1);

		let incentive_after_first = PezTreasury::get_incentive_pot_balance();

		// Sit on the same month for a while. The hook runs every block and must decline.
		run_blocks(20);

		assert_eq!(PezTreasury::next_release_month(), 1);
		assert_eq!(PezTreasury::get_incentive_pot_balance(), incentive_after_first);
	});
}

// =============================================================================
// 4. BACKLOG
// =============================================================================

#[test]
fn a_backlog_drains_one_release_per_block() {
	new_test_ext().execute_with(|| {
		activate();

		// Nothing runs for three months, then blocks resume.
		jump_to_block(1 + 3 * MONTH);
		run_blocks(1);
		assert_eq!(PezTreasury::next_release_month(), 1);
		run_blocks(1);
		assert_eq!(PezTreasury::next_release_month(), 2);
		run_blocks(1);
		assert_eq!(PezTreasury::next_release_month(), 3);
		run_blocks(1);
		assert_eq!(PezTreasury::next_release_month(), 4);

		// Month 4 is not due yet, so the drain stops of its own accord.
		run_blocks(5);
		assert_eq!(PezTreasury::next_release_month(), 4);
	});
}

#[test]
fn a_backlog_pays_each_month_what_that_month_was_owed() {
	// This is the defect the derived amount fixes. When the period was advanced once per call
	// rather than read from the release index, a backlog halved on every forty-eighth *call*,
	// so a hundred months paid out in a hundred consecutive blocks settled at the wrong rate.
	new_test_ext().execute_with(|| {
		activate();

		let months = 100u32;
		jump_to_block(1 + (months as u64 - 1) * MONTH);
		run_blocks(months as u64);

		assert_eq!(PezTreasury::next_release_month(), months);

		let expected_total: u128 = (0..months).map(expected_amount).sum();
		assert_eq!(PezTreasury::halving_info().total_released, expected_total);

		for index in 0..months {
			let release = PezTreasury::monthly_releases(index).unwrap();
			assert_eq!(
				release.amount_released,
				expected_amount(index),
				"release {index} paid the wrong amount"
			);
		}
	});
}

// =============================================================================
// 5. HALVING
// =============================================================================

#[test]
fn release_forty_seven_is_paid_in_full() {
	// Forty-eight releases make a period: indices 0 through 47. The one that closes the period
	// is not the one that is halved.
	new_test_ext().execute_with(|| {
		activate();
		let initial = expected_amount(0);

		jump_to_block(1 + 46 * MONTH);
		run_blocks(47); // releases 0..=46
		assert_eq!(PezTreasury::next_release_month(), 47);

		jump_to_block(1 + 47 * MONTH);
		run_blocks(1);

		let release = PezTreasury::monthly_releases(47).unwrap();
		assert_eq!(release.amount_released, initial);
		assert_eq!(PezTreasury::halving_info().current_period, 0);
		assert_eq!(PezTreasury::halving_info().monthly_amount, initial);
	});
}

#[test]
fn release_forty_eight_is_the_first_halved_one() {
	new_test_ext().execute_with(|| {
		activate();
		let initial = expected_amount(0);

		jump_to_block(1 + 48 * MONTH);
		run_blocks(49); // releases 0..=48

		let release = PezTreasury::monthly_releases(48).unwrap();
		assert_eq!(release.amount_released, initial / 2);

		let info = PezTreasury::halving_info();
		assert_eq!(info.current_period, 1);
		assert_eq!(info.monthly_amount, initial / 2);
		assert_eq!(info.period_start_block, System::block_number());

		System::assert_has_event(
			Event::NewHalvingPeriod { period: 1, new_monthly_amount: initial / 2 }.into(),
		);
	});
}

#[test]
fn halvings_keep_coming() {
	new_test_ext().execute_with(|| {
		activate();
		let initial = expected_amount(0);

		jump_to_block(1 + 144 * MONTH);
		run_blocks(145); // releases 0..=144

		let info = PezTreasury::halving_info();
		assert_eq!(info.current_period, 3);
		assert_eq!(info.monthly_amount, initial / 8);
		assert_eq!(PezTreasury::monthly_releases(96).unwrap().amount_released, initial / 4);
		assert_eq!(PezTreasury::monthly_releases(144).unwrap().amount_released, initial / 8);
	});
}

#[test]
fn the_halving_event_fires_once_per_period() {
	new_test_ext().execute_with(|| {
		activate();
		System::reset_events();

		jump_to_block(1 + 96 * MONTH);
		run_blocks(97); // releases 0..=96

		let halvings = System::events()
			.iter()
			.filter(|e| {
				matches!(e.event, RuntimeEvent::PezTreasury(Event::NewHalvingPeriod { .. }))
			})
			.count();
		assert_eq!(halvings, 2, "one for period 1, one for period 2");
	});
}

// =============================================================================
// 6. A RELEASE THAT CANNOT BE MADE
// =============================================================================

#[test]
fn an_unfundable_release_is_retried_not_skipped() {
	new_test_ext().execute_with(|| {
		// Activated with an empty treasury: the schedule is real, the money is not there yet.
		assert_ok!(PezTreasury::activate_distribution(RuntimeOrigin::root()));

		run_blocks(1);

		assert_eq!(PezTreasury::next_release_month(), 0, "a failed release must not advance");
		assert!(PezTreasury::monthly_releases(0).is_none());
		assert!(System::events()
			.iter()
			.any(|e| matches!(e.event, RuntimeEvent::PezTreasury(Event::MonthlyReleaseFailed))));

		// Once the money arrives, the same release is made -- at index 0, not skipped forward.
		fund_treasury();
		run_blocks(1);

		let release = PezTreasury::monthly_releases(0).expect("release 0 was skipped");
		assert_eq!(release.month_index, 0);
		assert_eq!(PezTreasury::next_release_month(), 1);
	});
}

#[test]
fn a_failed_release_moves_no_money() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezTreasury::activate_distribution(RuntimeOrigin::root()));

		run_blocks(3);

		assert!(PezTreasury::get_incentive_pot_balance().is_zero());
		assert!(PezTreasury::get_government_pot_balance().is_zero());
		assert!(PezTreasury::halving_info().total_released.is_zero());
	});
}

#[test]
fn being_too_early_is_quiet() {
	// `ReleaseTooEarly` is the answer on all but one block a month. It must not fill the chain
	// with failure events.
	new_test_ext().execute_with(|| {
		activate();
		run_blocks(1); // release 0
		System::reset_events();

		run_blocks(50);

		assert!(!System::events()
			.iter()
			.any(|e| matches!(e.event, RuntimeEvent::PezTreasury(Event::MonthlyReleaseFailed))));
	});
}

// =============================================================================
// 7. ACCOUNTING
// =============================================================================

#[test]
fn what_leaves_the_treasury_equals_what_reaches_the_pots() {
	new_test_ext().execute_with(|| {
		activate();
		let treasury_before = pez_balance(treasury_account());

		jump_to_block(1 + 9 * MONTH);
		run_blocks(10); // releases 0..=9

		let treasury_after = pez_balance(treasury_account());
		let paid_out = treasury_before - treasury_after;
		let in_pots =
			PezTreasury::get_incentive_pot_balance() + PezTreasury::get_government_pot_balance();

		assert_eq!(paid_out, in_pots);
		assert_eq!(paid_out, PezTreasury::halving_info().total_released);
	});
}

#[test]
fn the_treasury_only_ever_goes_down() {
	new_test_ext().execute_with(|| {
		activate();
		let mut previous = pez_balance(treasury_account());

		for month in 0..120u64 {
			jump_to_block(1 + month * MONTH);
			run_blocks(1);
			let now = pez_balance(treasury_account());
			assert!(now <= previous, "treasury grew at month {month}");
			previous = now;
		}
	});
}

#[test]
fn the_first_period_is_half_the_allocation() {
	new_test_ext().execute_with(|| {
		let first_period: u128 = (0..HALVING_PERIOD_MONTHS).map(expected_amount).sum();
		let target = TREASURY_ALLOCATION / 2;

		// The only difference allowed is integer division dropping at most one unit a month.
		let lost = target - first_period;
		assert!(lost < HALVING_PERIOD_MONTHS as u128, "rounding lost {lost}");
	});
}

#[test]
fn the_schedule_never_exceeds_the_allocation() {
	// A halving series sums to twice its first period, which is the whole allocation. Rounding
	// only ever loses, so the sum must stay under.
	let mut total = 0u128;
	for index in 0..(HALVING_PERIOD_MONTHS * 130) {
		total += expected_amount(index);
	}
	assert!(
		total <= TREASURY_ALLOCATION,
		"schedule pays out {total}, allocation is {TREASURY_ALLOCATION}"
	);
}

#[test]
fn the_schedule_reaches_zero_and_stays_there() {
	// Past roughly 127 halvings the amount is zero. It must not wrap, panic, or come back.
	assert_eq!(expected_amount(HALVING_PERIOD_MONTHS * 200), 0);
	assert_eq!(expected_amount(u32::MAX), 0);
}

// =============================================================================
// 8. POT ACCOUNTS
// =============================================================================

#[test]
fn the_three_accounts_are_distinct() {
	new_test_ext().execute_with(|| {
		let treasury = PezTreasury::treasury_account_id();
		let incentive = PezTreasury::incentive_pot_account_id();
		let government = PezTreasury::government_pot_account_id();

		assert_ne!(treasury, incentive);
		assert_ne!(treasury, government);
		assert_ne!(incentive, government);
	});
}

#[test]
fn the_pot_accounts_have_no_key() {
	// They are derived from PalletIds, so no seed produces them. The test states the property
	// the design depends on: money in a pot can only move by pallet logic.
	new_test_ext().execute_with(|| {
		for account in [
			PezTreasury::treasury_account_id(),
			PezTreasury::incentive_pot_account_id(),
			PezTreasury::government_pot_account_id(),
		] {
			assert_eq!(&account.as_bytes()[0..8], b"modlpy/p", "not a PalletId account");
		}
	});
}

// =============================================================================
// 9. THE INVARIANT CAN FAIL
// =============================================================================
//
// `try_state` runs after every block of every test above. That only means something if it is
// capable of rejecting a bad state -- a check that always passes is worse than none, because
// it reads as coverage. Each test here breaks one thing and insists the invariant sees it.

#[cfg(feature = "try-runtime")]
mod invariant {
	use super::*;
	use crate::{HalvingInfo, MonthlyReleases, NextReleaseMonth, TreasuryStartBlock};
	use pezframe_support::traits::{TryState, TryStateSelect};

	fn try_state_result() -> Result<(), pezsp_runtime::TryRuntimeError> {
		AllPalletsWithSystem::try_state(System::block_number(), TryStateSelect::All)
	}

	fn assert_rejected(what: &str) {
		assert!(try_state_result().is_err(), "try_state accepted a state where {what}");
	}

	#[test]
	fn a_gap_in_the_history_is_caught() {
		new_test_ext().execute_with(|| {
			activate();
			jump_to_block(1 + 4 * MONTH);
			run_blocks(5); // releases 0..=4
			assert_ok!(try_state_result());

			MonthlyReleases::<Test>::remove(2);
			assert_rejected("a month had no release record");
		});
	}

	#[test]
	fn a_release_of_the_wrong_size_is_caught() {
		new_test_ext().execute_with(|| {
			activate();
			run_blocks(1);
			assert_ok!(try_state_result());

			MonthlyReleases::<Test>::mutate(0, |slot| {
				let record = slot.as_mut().unwrap();
				record.amount_released += 1;
			});
			assert_rejected("a release paid more than its month was owed");
		});
	}

	#[test]
	fn a_mismatched_total_is_caught() {
		new_test_ext().execute_with(|| {
			activate();
			run_blocks(1);
			assert_ok!(try_state_result());

			HalvingInfo::<Test>::mutate(|info| info.total_released += 1);
			assert_rejected("total_released did not match the records");
		});
	}

	#[test]
	fn a_period_that_no_release_earned_is_caught() {
		new_test_ext().execute_with(|| {
			activate();
			run_blocks(1);
			assert_ok!(try_state_result());

			HalvingInfo::<Test>::mutate(|info| info.current_period = 1);
			assert_rejected("the halving period ran ahead of the releases");
		});
	}

	#[test]
	fn a_broken_split_is_caught() {
		new_test_ext().execute_with(|| {
			activate();
			run_blocks(1);
			assert_ok!(try_state_result());

			MonthlyReleases::<Test>::mutate(0, |slot| {
				let record = slot.as_mut().unwrap();
				record.incentive_amount += 1;
				record.government_amount -= 1;
			});
			assert_rejected("the pots were credited in the wrong proportion");
		});
	}

	#[test]
	fn a_latch_without_a_start_block_is_caught() {
		new_test_ext().execute_with(|| {
			activate();
			assert_ok!(try_state_result());

			TreasuryStartBlock::<Test>::kill();
			assert_rejected("the schedule had begun with no start block");
		});
	}

	#[test]
	fn releases_before_activation_are_caught() {
		new_test_ext().execute_with(|| {
			assert_ok!(try_state_result());

			NextReleaseMonth::<Test>::put(1);
			assert_rejected("a release was counted before the schedule began");
		});
	}
}

// =============================================================================
// 10. SPENDING FROM THE GOVERNMENT POT
// =============================================================================

/// Release once so the government pot has something in it, and return what it holds.
fn fund_government_pot() -> u128 {
	activate();
	run_blocks(1);
	PezTreasury::get_government_pot_balance()
}

#[test]
fn the_government_can_spend_what_was_released_to_it() {
	new_test_ext().execute_with(|| {
		let pot = fund_government_pot();
		assert!(pot > 0);
		let supply_before = pez_total_supply();

		assert_ok!(PezTreasury::spend_from_government_pot(RuntimeOrigin::root(), alice(), pot / 2));

		assert_pez_balance(alice(), pot / 2);
		assert_eq!(PezTreasury::get_government_pot_balance(), pot - pot / 2);
		assert_eq!(pez_total_supply(), supply_before, "spending must not mint");

		System::assert_has_event(
			Event::GovernmentPotSpent { beneficiary: alice(), amount: pot / 2 }.into(),
		);
	});
}

#[test]
fn spending_rejects_anyone_but_the_spend_origin() {
	new_test_ext().execute_with(|| {
		let pot = fund_government_pot();

		assert_noop!(
			PezTreasury::spend_from_government_pot(RuntimeOrigin::signed(alice()), alice(), 1),
			pezsp_runtime::DispatchError::BadOrigin
		);
		assert_eq!(PezTreasury::get_government_pot_balance(), pot);
	});
}

#[test]
fn the_government_cannot_spend_more_than_its_pot_holds() {
	new_test_ext().execute_with(|| {
		let pot = fund_government_pot();

		assert_noop!(
			PezTreasury::spend_from_government_pot(RuntimeOrigin::root(), alice(), pot + 1),
			Error::<Test>::InsufficientGovernmentPotBalance
		);
		assert_eq!(PezTreasury::get_government_pot_balance(), pot);
	});
}

#[test]
fn spending_cannot_reach_the_incentive_pot_or_the_treasury() {
	// The government may spend its quarter and nothing else. The other two accounts hold what
	// has not been handed to it -- the citizens' three quarters, and every month not yet due.
	new_test_ext().execute_with(|| {
		let pot = fund_government_pot();
		let incentive_before = PezTreasury::get_incentive_pot_balance();
		let treasury_before = pez_balance(treasury_account());

		assert_ok!(PezTreasury::spend_from_government_pot(RuntimeOrigin::root(), alice(), pot - 1));

		assert_eq!(PezTreasury::get_incentive_pot_balance(), incentive_before);
		assert_eq!(pez_balance(treasury_account()), treasury_before);
	});
}

#[test]
fn a_spend_of_nothing_is_refused() {
	new_test_ext().execute_with(|| {
		fund_government_pot();
		assert_noop!(
			PezTreasury::spend_from_government_pot(RuntimeOrigin::root(), alice(), 0),
			Error::<Test>::NothingToSpend
		);
	});
}

#[test]
fn spending_does_not_disturb_the_schedule() {
	new_test_ext().execute_with(|| {
		let pot = fund_government_pot();
		let next_before = PezTreasury::next_release_month();

		assert_ok!(PezTreasury::spend_from_government_pot(RuntimeOrigin::root(), alice(), pot - 1));

		assert_eq!(PezTreasury::next_release_month(), next_before);

		// The following month still arrives, and still pays in full.
		jump_to_block(1 + MONTH);
		run_blocks(1);
		assert_eq!(PezTreasury::next_release_month(), next_before + 1);
		assert_eq!(
			PezTreasury::monthly_releases(next_before).unwrap().amount_released,
			expected_amount(next_before)
		);
	});
}

#[test]
fn the_pot_account_is_never_emptied_to_nothing() {
	// `Preservation::Preserve` on the transfer, stated as a decision rather than left as a
	// surprise. The last unit -- a millionth of a millionth of one PEZ -- stays behind so the
	// pot account is never reaped. What that buys is that the account, its balance and its
	// history are continuously queryable: a pot that blinks out of existence between a
	// spend and the next release reads as "no such account" to anything watching it.
	new_test_ext().execute_with(|| {
		let pot = fund_government_pot();

		assert_noop!(
			PezTreasury::spend_from_government_pot(RuntimeOrigin::root(), alice(), pot),
			Error::<Test>::InsufficientGovernmentPotBalance
		);

		assert_ok!(PezTreasury::spend_from_government_pot(RuntimeOrigin::root(), alice(), pot - 1));
		assert_eq!(PezTreasury::get_government_pot_balance(), 1);
	});
}

// =============================================================================
// 11. PAYING OUT OF THE INCENTIVE POT
// =============================================================================

/// Release once so the incentive pot has something in it, and return what it holds.
fn fund_incentive_pot() -> u128 {
	activate();
	run_blocks(1);
	PezTreasury::get_incentive_pot_balance()
}

#[test]
fn the_rewards_chain_can_pay_out_of_the_incentive_pot() {
	new_test_ext().execute_with(|| {
		let pot = fund_incentive_pot();
		assert!(pot > 0);
		let supply_before = pez_total_supply();

		assert_ok!(PezTreasury::pay_from_incentive_pot(RuntimeOrigin::root(), alice(), pot / 2));

		assert_pez_balance(alice(), pot / 2);
		assert_eq!(PezTreasury::get_incentive_pot_balance(), pot - pot / 2);
		assert_eq!(pez_total_supply(), supply_before, "paying a reward must not mint");

		System::assert_has_event(
			Event::IncentivePotSpent { beneficiary: alice(), amount: pot / 2 }.into(),
		);
	});
}

#[test]
fn paying_rewards_rejects_anyone_but_the_incentive_spend_origin() {
	// The whole point of the split: the money is here, the arithmetic is on the rewards
	// chain. A signed account that could reach this call could pay itself the citizens' share.
	new_test_ext().execute_with(|| {
		let pot = fund_incentive_pot();

		assert_noop!(
			PezTreasury::pay_from_incentive_pot(RuntimeOrigin::signed(alice()), alice(), pot / 2),
			pezsp_runtime::DispatchError::BadOrigin
		);
		assert_eq!(PezTreasury::get_incentive_pot_balance(), pot);
	});
}

#[test]
fn a_reward_payment_of_nothing_is_refused() {
	new_test_ext().execute_with(|| {
		fund_incentive_pot();
		assert_noop!(
			PezTreasury::pay_from_incentive_pot(RuntimeOrigin::root(), alice(), 0),
			Error::<Test>::NothingToSpend
		);
	});
}

#[test]
fn rewards_cannot_reach_the_government_pot_or_the_treasury() {
	// Asking for more than the incentive pot holds must fail rather than quietly reach into
	// the neighbouring pot or the undistributed allocation behind it.
	new_test_ext().execute_with(|| {
		let pot = fund_incentive_pot();
		let government_before = PezTreasury::get_government_pot_balance();
		let treasury_before = pez_balance(PezTreasury::treasury_account_id());

		assert_noop!(
			PezTreasury::pay_from_incentive_pot(RuntimeOrigin::root(), alice(), pot + 1),
			Error::<Test>::InsufficientIncentivePotBalance
		);

		assert_eq!(PezTreasury::get_government_pot_balance(), government_before);
		assert_eq!(pez_balance(PezTreasury::treasury_account_id()), treasury_before);
	});
}

#[test]
fn the_incentive_pot_account_is_never_emptied_to_nothing() {
	// Same `Preservation::Preserve` decision as the government pot, tested separately so
	// that changing one of the two cannot pass on the other one's test.
	new_test_ext().execute_with(|| {
		let pot = fund_incentive_pot();

		assert_noop!(
			PezTreasury::pay_from_incentive_pot(RuntimeOrigin::root(), alice(), pot),
			Error::<Test>::InsufficientIncentivePotBalance
		);

		assert_ok!(PezTreasury::pay_from_incentive_pot(RuntimeOrigin::root(), alice(), pot - 1));
		assert_eq!(PezTreasury::get_incentive_pot_balance(), 1);
	});
}

// =============================================================================
// 12. REPORTING THE INCENTIVE FUNDING TO THE REWARDS CHAIN
// =============================================================================

/// The `Compact<u128>` total carried by the one `Transact` in `message`.
fn reported_total(message: &xcm::latest::Xcm<()>) -> u128 {
	use codec::Decode;
	for instruction in message.inner() {
		if let xcm::latest::Instruction::Transact { call, .. } = instruction {
			let encoded = call.clone().into_encoded();
			// pallet index, call index, then the compact total.
			let mut rest = &encoded[2..];
			return codec::Compact::<u128>::decode(&mut rest).expect("a compact total").0;
		}
	}
	panic!("the message carries no Transact");
}

#[test]
fn a_release_reports_the_running_incentive_total() {
	new_test_ext().execute_with(|| {
		clear_sent_xcm();
		activate();
		run_blocks(1);

		let first = PezTreasury::total_incentive_released();
		assert!(first > 0);
		assert_eq!(first, PezTreasury::monthly_releases(0).unwrap().incentive_amount);

		let sent = sent_xcm();
		assert_eq!(sent.len(), 1, "one report per release");
		assert_eq!(reported_total(&sent[0].1), first);

		// The second release reports the sum, not the month.
		jump_to_block(System::block_number() + MONTH - 1);
		run_blocks(1);
		let second = PezTreasury::total_incentive_released();
		assert!(second > first);

		let sent = sent_xcm();
		assert_eq!(sent.len(), 2);
		assert_eq!(
			reported_total(&sent[1].1),
			second,
			"the report is a running total, so a lost message is repaired by the next one"
		);
	});
}

#[test]
fn a_lost_report_does_not_undo_the_release() {
	// The money has already moved by the time the report is attempted. If a failed send
	// unwound the release, an unreachable sibling chain would stop the state paying itself.
	new_test_ext().execute_with(|| {
		clear_sent_xcm();
		fail_sending(true);
		activate();
		run_blocks(1);

		assert!(PezTreasury::get_incentive_pot_balance() > 0, "the release still happened");
		assert_eq!(PezTreasury::next_release_month(), 1);
		assert!(sent_xcm().is_empty());
		System::assert_has_event(
			Event::IncentiveFundingReportFailed { total: PezTreasury::total_incentive_released() }
				.into(),
		);

		// The next release repairs it: the total sent covers both months.
		fail_sending(false);
		jump_to_block(System::block_number() + MONTH - 1);
		run_blocks(1);
		let sent = sent_xcm();
		assert_eq!(sent.len(), 1);
		assert_eq!(reported_total(&sent[0].1), PezTreasury::total_incentive_released());
	});
}

#[test]
fn the_report_is_addressed_to_the_rewards_chain_and_its_pallet() {
	new_test_ext().execute_with(|| {
		clear_sent_xcm();
		activate();
		run_blocks(1);

		let (destination, message) = sent_xcm().pop().expect("a report was sent");
		assert_eq!(destination, RewardsChain::get());

		let transact = message
			.inner()
			.iter()
			.find_map(|i| match i {
				xcm::latest::Instruction::Transact { call, .. } => Some(call.clone()),
				_ => None,
			})
			.expect("a Transact");
		let encoded = transact.into_encoded();
		assert_eq!(encoded[0], RewardsPalletIndex::get(), "the rewards pallet's index");
		assert_eq!(encoded[1], 2, "note_incentive_funding");

		// Unpaid, for the same reason as every other message between these two chains.
		assert!(message
			.inner()
			.iter()
			.any(|i| matches!(i, xcm::latest::Instruction::UnpaidExecution { .. })));
	});
}
