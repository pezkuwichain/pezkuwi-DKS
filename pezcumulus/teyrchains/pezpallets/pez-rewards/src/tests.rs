// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Tests for `pezpallet-pez-rewards`.
//!
//! The pallet has one job -- work out what the state owes and instruct the payment -- and
//! three rules behind it. It cannot pay out more than the pot was given; it must measure
//! every claimant against the same roll at the same instant; and a seat is paid because it is
//! held, not because a list still names its holder. Most of what follows is there to keep
//! those three honest.

use crate::{
	mock::*, EpochState, Error, Event, BLOCKS_PER_EPOCH, CLAIM_PERIOD_BLOCKS, PARLIAMENT_SIZE,
};
use pezframe_support::{assert_noop, assert_ok};

const EPOCH: u64 = BLOCKS_PER_EPOCH as u64;
const WINDOW: u64 = CLAIM_PERIOD_BLOCKS as u64;

/// Start the clock at block 1, which is where `new_test_ext` leaves it.
fn start() {
	assert_ok!(PezRewards::initialize_rewards_system(RuntimeOrigin::root()));
}

/// Report `total` as everything the pot has been given.
fn fund(total: u128) {
	assert_ok!(PezRewards::note_incentive_funding(RuntimeOrigin::root(), total));
}

/// Land on the block the current epoch falls due, so `on_initialize` finalises it.
fn finalize_current_epoch() -> u64 {
	let due = PezRewards::epoch_info().epoch_start_block + EPOCH;
	jump_to_block(due);
	due
}

// =============================================================================
// 1. THE CLOCK
// =============================================================================

#[test]
fn the_clock_starts_with_one_open_epoch() {
	new_test_ext().execute_with(|| {
		start();
		assert_eq!(PezRewards::epoch_info().current_epoch, 0);
		assert_eq!(PezRewards::epoch_status(0), EpochState::Open);
		assert!(PezRewards::epoch_in_claim().is_none());
	});
}

#[test]
fn the_clock_cannot_be_started_twice() {
	new_test_ext().execute_with(|| {
		start();
		assert_noop!(
			PezRewards::initialize_rewards_system(RuntimeOrigin::root()),
			Error::<Test>::AlreadyInitialized
		);
	});
}

#[test]
fn an_epoch_finalises_itself_when_its_month_is_up() {
	// There is no `finalize_epoch` extrinsic any more. There does not need to be: the work is
	// constant now, so the block can do it -- and a month's rewards no longer depend on
	// somebody remembering to send a transaction.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);

		jump_to_block(EPOCH); // one block short
		assert_eq!(PezRewards::epoch_status(0), EpochState::Open, "not due yet");

		let at = finalize_current_epoch();
		assert_eq!(PezRewards::epoch_status(0), EpochState::ClaimPeriod);
		assert_eq!(PezRewards::epoch_in_claim(), Some(0));
		assert_eq!(PezRewards::epoch_status(1), EpochState::Open, "and the next one is running");
		assert_eq!(PezRewards::epoch_info().current_epoch, 1);

		let pool = PezRewards::epoch_reward_pools(0).unwrap();
		assert_eq!(pool.finalized_at, at);
		assert_eq!(pool.claim_deadline, at + WINDOW);
	});
}

#[test]
fn an_epoch_with_nothing_behind_it_still_ends() {
	// A month where the pot was empty pays nothing, and that is all it does. If it stopped
	// the clock instead, every later month would be lost with it.
	new_test_ext().execute_with(|| {
		start();
		finalize_current_epoch();

		let pool = PezRewards::epoch_reward_pools(0).unwrap();
		assert_eq!(pool.reward_per_trust_point, 0);
		assert_eq!(pool.seat_share, 0);
		assert_eq!(PezRewards::epoch_info().current_epoch, 1, "the clock moved on regardless");
	});
}

#[test]
fn the_claim_window_closes_itself() {
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		let at = finalize_current_epoch();

		jump_to_block(at + WINDOW);
		assert_eq!(PezRewards::epoch_status(0), EpochState::ClaimPeriod, "still open on the day");

		jump_to_block(at + WINDOW + 1);
		assert_eq!(PezRewards::epoch_status(0), EpochState::Closed);
		assert!(PezRewards::epoch_in_claim().is_none());
	});
}

// =============================================================================
// 2. FUNDING
// =============================================================================

#[test]
fn funding_is_a_running_total_and_cannot_go_backwards() {
	// The treasury reports the sum, not the month, so a message that never arrives is
	// repaired by the next one. That only works if the number is monotonic -- a report that
	// went backwards would be a lost message read as a refund.
	new_test_ext().execute_with(|| {
		fund(500);
		assert_eq!(PezRewards::reported_incentive_total(), 500);

		fund(1_200);
		assert_eq!(PezRewards::reported_incentive_total(), 1_200);

		assert_noop!(
			PezRewards::note_incentive_funding(RuntimeOrigin::root(), 900),
			Error::<Test>::FundingReportWentBackwards
		);
		assert_eq!(PezRewards::reported_incentive_total(), 1_200);
	});
}

#[test]
fn only_the_funding_origin_may_report() {
	// A signed account that could report funding could conjure a payroll out of a pot with
	// no money in it -- and the failure would land on the far side of a bridge.
	new_test_ext().execute_with(|| {
		assert_noop!(
			PezRewards::note_incentive_funding(RuntimeOrigin::signed(1), 1_000),
			pezsp_runtime::DispatchError::BadOrigin
		);
		assert_eq!(PezRewards::reported_incentive_total(), 0);
	});
}

#[test]
fn what_is_available_is_what_was_reported_minus_what_was_paid() {
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		assert_eq!(PezRewards::available_funds(), 1_000_000);

		finalize_current_epoch();
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));

		let paid = PezRewards::claimed_rewards(0, 1).unwrap();
		assert_eq!(PezRewards::paid_out_total(), paid);
		assert_eq!(PezRewards::available_funds(), 1_000_000 - paid);
	});
}

// =============================================================================
// 3. THE RATE
// =============================================================================

#[test]
fn the_rate_is_the_citizens_share_spread_over_the_whole_roll() {
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 100);
		set_trust(2, 200);
		set_trust(3, 700);

		finalize_current_epoch();
		let pool = PezRewards::epoch_reward_pools(0).unwrap();

		// Ten per cent to the house, divided by its size and not by its membership.
		assert_eq!(pool.seat_share, 100_000 / PARLIAMENT_SIZE as u128);
		// The rest over the whole roll.
		assert_eq!(pool.reward_per_trust_point, 900_000 / 1_000);
	});
}

#[test]
fn the_shares_add_up_to_the_pool() {
	// The property that makes the denominator safe to take from the trust pallet: if the sum
	// of every share came to more than the pool, the last claimant would find it empty.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 100);
		set_trust(2, 200);
		set_trust(3, 700);

		finalize_current_epoch();
		for who in 1..=3u64 {
			assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(who), 0));
		}

		assert_eq!(PezRewards::paid_out_total(), 900_000, "exactly the citizens' share");
	});
}

#[test]
fn finalising_freezes_the_trust_roll_for_the_claim_window() {
	// The whole of the cheap design rests on this. The rate is computed against the roll at
	// one instant and each share is read from the roll when it is claimed; if the roll could
	// move in between, the shares would not add up and claiming late would be worth doing.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);

		let at = finalize_current_epoch();
		assert_eq!(
			freezes(),
			vec![at + WINDOW],
			"the roll is held still for exactly as long as it can be drawn against"
		);
	});
}

// =============================================================================
// 4. CLAIMS
// =============================================================================

#[test]
fn a_citizen_claims_their_share_and_the_payment_is_instructed() {
	// Recording the claim is not paying it: the money is on another chain. A test that only
	// checked storage would pass on a pallet that never sent anything.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 100);
		set_trust(2, 900);
		finalize_current_epoch();
		clear_sent_xcm();

		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));

		let expected = 900 * 100;
		assert_eq!(PezRewards::claimed_rewards(0, 1), Some(expected));
		assert_eq!(sent_xcm().len(), 1, "the payment was actually instructed");
		System::assert_has_event(
			Event::RewardClaimed { who: 1, epoch_index: 0, amount: expected }.into(),
		);
	});
}

#[test]
fn nobody_is_paid_twice() {
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		finalize_current_epoch();

		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));
		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(1), 0),
			Error::<Test>::RewardAlreadyClaimed
		);
	});
}

#[test]
fn a_claim_after_the_deadline_is_refused() {
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		let at = finalize_current_epoch();

		jump_to_block(at + WINDOW + 1);
		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(1), 0),
			Error::<Test>::NotInClaimPeriod
		);
	});
}

#[test]
fn nothing_owed_is_nothing_claimed() {
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		finalize_current_epoch();

		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(99), 0),
			Error::<Test>::NoRewardToClaim
		);
	});
}

#[test]
fn a_revoked_citizen_is_paid_nothing_and_no_citizenship_check_says_so() {
	// There is no `is_citizen` here on purpose. Revoking a citizenship takes the trust score
	// away, so the share is zero by arithmetic -- and the rule lives in one place instead of
	// two that can disagree.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 500);
		set_trust(2, 500);
		finalize_current_epoch();

		set_trust(1, 0); // what `OnCitizenshipRevoked` does to the roll

		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(1), 0),
			Error::<Test>::NoRewardToClaim
		);
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(2), 0));
	});
}

#[test]
fn an_unreachable_treasury_leaves_no_record_of_a_payment_that_did_not_happen() {
	// The record and the money must not be able to disagree. If the instruction cannot be
	// sent, the claim has to unwind -- otherwise the claimant is marked paid and never was.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		finalize_current_epoch();

		fail_sending(true);
		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(1), 0),
			Error::<Test>::CouldNotReachTreasury
		);
		assert!(PezRewards::claimed_rewards(0, 1).is_none());
		assert_eq!(PezRewards::paid_out_total(), 0);

		fail_sending(false);
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));
	});
}

#[test]
fn the_payment_is_addressed_to_the_treasury_pallet_and_its_call() {
	// The two chains do not share a runtime type, so the call is addressed by index. This
	// pins both ends of that agreement; the treasury pallet has the mirror of this test.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		finalize_current_epoch();
		clear_sent_xcm();

		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));
		let (destination, message) = sent_xcm().pop().expect("a payment was instructed");
		assert_eq!(destination, TreasuryChain::get());

		let transact = message
			.inner()
			.iter()
			.find_map(|i| match i {
				xcm::latest::Instruction::Transact { call, .. } => Some(call.clone()),
				_ => None,
			})
			.expect("a Transact");
		let encoded = transact.into_encoded();
		assert_eq!(encoded[0], TreasuryPalletIndex::get(), "the treasury pallet's index");
		assert_eq!(encoded[1], 3, "pay_from_incentive_pot");

		assert!(message
			.inner()
			.iter()
			.any(|i| matches!(i, xcm::latest::Instruction::UnpaidExecution { .. })));
	});
}

// =============================================================================
// 5. PARLIAMENTARY SEATS
// =============================================================================

#[test]
fn a_seat_is_paid_by_the_seat_and_never_by_the_member() {
	// Dividing by the number of people sitting would make removing a member profitable for
	// everyone left. The divisor is the size of the house, always.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		set_seat(1, 1, true);
		finalize_current_epoch();

		let pool = PezRewards::epoch_reward_pools(0).unwrap();
		assert_eq!(pool.seat_share, 100_000 / PARLIAMENT_SIZE as u128);

		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));
		assert_eq!(
			PezRewards::claimed_rewards(0, 1),
			Some(pool.reward_per_trust_point * 1_000 + pool.seat_share),
			"one seat's worth, not a two-hundred-and-first of the house between one member"
		);
	});
}

#[test]
fn a_member_the_diwan_removed_is_paid_nothing_for_the_seat() {
	// The roll still names them -- that is the design, `welati::ParliamentMembers` is not
	// rewritten when a seat is forfeited. The tiki is what is asked, and it says no.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		set_seat(1, 1, false); // on the roll, no longer holding the seat
		finalize_current_epoch();

		let pool = PezRewards::epoch_reward_pools(0).unwrap();
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));
		assert_eq!(
			PezRewards::claimed_rewards(0, 1),
			Some(pool.reward_per_trust_point * 1_000),
			"the citizen's share and not a penny of the seat's"
		);
	});
}

#[test]
fn a_seat_taken_during_the_claim_window_is_not_paid_for_the_month_before_it() {
	// An election counted inside the claim window must not pay the new house for the old
	// house's month. That is what `finalized_at` is for.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		let at = finalize_current_epoch();

		set_seat(1, at + 1, true); // seated after the roll was measured

		let pool = PezRewards::epoch_reward_pools(0).unwrap();
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));
		assert_eq!(PezRewards::claimed_rewards(0, 1), Some(pool.reward_per_trust_point * 1_000));
	});
}

#[test]
fn an_unclaimed_seat_stays_in_the_pot_for_the_following_month() {
	// Two hundred seats go unclaimed here. Nothing is clawed back, because nothing left: the
	// money is still on the other chain, and next month's rate is computed over more of it.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		finalize_current_epoch();
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));

		let paid = PezRewards::paid_out_total();
		assert!(paid < 1_000_000);

		let second = finalize_current_epoch();
		let pool = PezRewards::epoch_reward_pools(1).unwrap();
		assert_eq!(pool.finalized_at, second);
		assert_eq!(
			pool.reward_per_trust_point,
			(1_000_000 - paid) * 90 / 100 / 1_000,
			"what nobody claimed is simply next month's pot"
		);
	});
}

// =============================================================================
// 6. THE CALL SURFACE
// =============================================================================

#[test]
fn the_call_surface_is_three_calls_and_none_of_them_moves_money_directly() {
	// A compile-time statement of the surface. Adding a call stops this matching and the
	// test fails to build, which is the point. None of the three touches a balance: this
	// chain has no PEZ on it at all, and the only way money moves is an instruction to the
	// chain that does.
	use crate::pezpallet::Call;
	fn assert_exhaustive(call: Call<Test>) {
		match call {
			Call::initialize_rewards_system {} => {},
			Call::claim_reward { .. } => {},
			Call::note_incentive_funding { .. } => {},
			Call::__Ignore(_, _) => {},
		}
	}
	let _ = assert_exhaustive;
}

// =============================================================================
// 7. THE INVARIANTS, PROVED BREAKABLE
// =============================================================================

#[cfg(feature = "try-runtime")]
#[test]
#[should_panic(expected = "more has been paid out than the pot was ever given")]
fn paying_out_more_than_the_pot_was_given_is_caught() {
	new_test_ext().execute_with(|| {
		start();
		fund(100);
		crate::PaidOutTotal::<Test>::put(101u128);
		check_invariants();
	});
}

#[cfg(feature = "try-runtime")]
#[test]
#[should_panic(expected = "the claims recorded do not add up to what was paid out")]
fn a_claim_the_ledger_does_not_know_about_is_caught() {
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		set_trust(1, 1_000);
		finalize_current_epoch();
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(1), 0));

		crate::PaidOutTotal::<Test>::put(1u128);
		check_invariants();
	});
}

#[cfg(feature = "try-runtime")]
#[test]
#[should_panic(expected = "two epochs are open to claims at once")]
fn two_claim_windows_at_once_is_caught() {
	// The same pot promised twice over. Nothing in the pallet can produce this today, which
	// is exactly why it is worth a check: it is the shape the bug would take.
	new_test_ext().execute_with(|| {
		start();
		fund(1_000_000);
		finalize_current_epoch();
		crate::EpochStatus::<Test>::insert(1, EpochState::ClaimPeriod);
		check_invariants();
	});
}

#[cfg(feature = "try-runtime")]
#[test]
#[should_panic(expected = "a reward was paid for an epoch that is still collecting")]
fn a_reward_paid_against_an_open_epoch_is_caught() {
	new_test_ext().execute_with(|| {
		start();
		crate::ClaimedRewards::<Test>::insert(0, 1u64, 5u128);
		crate::PaidOutTotal::<Test>::put(5u128);
		crate::ReportedIncentiveTotal::<Test>::put(5u128);
		check_invariants();
	});
}
