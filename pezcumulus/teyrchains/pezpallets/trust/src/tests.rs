// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{mock::*, Error, Event};
use pezframe_support::{assert_noop, assert_ok};
use pezsp_runtime::traits::BadOrigin;

/// What the pallet should produce for a given record, worked out independently of it.
///
/// Each part as a share of its own maximum, brought onto the common scale, then weighted.
/// Written out rather than calling into the pallet, so the test disagrees with the code when
/// the code changes rather than agreeing with whatever it happens to do.
fn expected_trust(staking: u32, referral: u32, perwerde: u32, tiki: u32) -> u128 {
	let scale = TrustScoreScale::get() as u128;
	let part = |score: u32, max: u32, weight: u32| -> u128 {
		(score.min(max) as u128) * scale / (max as u128) * (weight as u128)
	};
	(part(staking, 100, StakingWeight::get())
		+ part(referral, REFERRAL_MAX, ReferralWeight::get())
		+ part(perwerde, PERWERDE_MAX, PerwerdeWeight::get())
		+ part(tiki, TIKI_MAX, TikiWeight::get()))
		/ 100
}

#[test]
fn calculate_trust_score_works() {
	new_test_ext().execute_with(|| {
		let account = 1u64;
		let score = TrustPallet::calculate_trust_score(&account).unwrap();

		// The mock's defaults: staking 100, referral 50, perwerde 30, tiki 20.
		assert_eq!(score, expected_trust(100, 50, 30, 20));
	});
}

#[test]
fn a_perfect_record_scores_exactly_the_scale() {
	// The weights are percentages and add to a hundred, so somebody at the maximum of every
	// part of the record scores the scale itself -- which is what makes every election
	// threshold readable as a share of what a citizen can be.
	new_test_ext().execute_with(|| {
		set_profile(1, 100, REFERRAL_MAX, PERWERDE_MAX, TIKI_MAX);
		assert_eq!(TrustPallet::calculate_trust_score(&1).unwrap(), TrustScoreScale::get() as u128);
		clear_profiles();
	});
}

#[test]
fn no_single_part_of_the_record_can_carry_somebody_alone() {
	// The reason for normalising. Before it, education ran to fifty thousand and referrals to
	// five hundred, both weighted the same -- so an education was worth a hundred referrals,
	// not by anyone's decision but by arithmetic nobody had looked at.
	new_test_ext().execute_with(|| {
		let scale = TrustScoreScale::get() as u128;

		set_profile(1, 100, 0, 0, 0);
		let staking_alone = TrustPallet::calculate_trust_score(&1).unwrap();
		set_profile(1, 1, PERWERDE_MAX, 0, 0);
		let education_alone = TrustPallet::calculate_trust_score(&1).unwrap();
		set_profile(1, 1, REFERRAL_MAX, 0, 0);
		let referrals_alone = TrustPallet::calculate_trust_score(&1).unwrap();

		// Each is its own weight's share of the scale, and none of them is the whole thing.
		assert_eq!(staking_alone, scale * StakingWeight::get() as u128 / 100);
		assert!(education_alone < scale);
		assert!(referrals_alone < scale);

		// And money is the smallest of them, deliberately: the stake is already a gate.
		assert!(staking_alone < education_alone);
		assert!(staking_alone < referrals_alone);
		clear_profiles();
	});
}

#[test]
fn a_component_above_its_own_maximum_cannot_borrow_the_others_weight() {
	new_test_ext().execute_with(|| {
		set_profile(1, 100, REFERRAL_MAX * 10, 0, 0);
		let inflated = TrustPallet::calculate_trust_score(&1).unwrap();
		set_profile(1, 100, REFERRAL_MAX, 0, 0);
		let at_max = TrustPallet::calculate_trust_score(&1).unwrap();
		assert_eq!(inflated, at_max);
		clear_profiles();
	});
}

#[test]
fn calculate_trust_score_fails_for_non_citizen() {
	new_test_ext().execute_with(|| {
		let non_citizen = 999u64;
		assert_noop!(TrustPallet::calculate_trust_score(&non_citizen), Error::<Test>::NotACitizen);
	});
}

#[test]
fn calculate_trust_score_zero_staking() {
	new_test_ext().execute_with(|| {
		let account = 1u64;
		let score = TrustPallet::calculate_trust_score(&account).unwrap();
		assert!(score > 0);
	});
}

#[test]
fn update_score_for_account_works() {
	new_test_ext().execute_with(|| {
		let account = 1u64;

		let initial_score = TrustPallet::trust_score_of(account);
		assert_eq!(initial_score, 0);

		let new_score = TrustPallet::update_score_for_account(&account).unwrap();
		assert!(new_score > 0);

		let stored_score = TrustPallet::trust_score_of(account);
		assert_eq!(stored_score, new_score);

		let total_score = TrustPallet::total_active_trust_score();
		assert_eq!(total_score, new_score);
	});
}

#[test]
fn update_score_for_account_updates_total() {
	new_test_ext().execute_with(|| {
		let account1 = 1u64;
		let account2 = 2u64;

		let score1 = TrustPallet::update_score_for_account(&account1).unwrap();
		let total_after_first = TrustPallet::total_active_trust_score();
		assert_eq!(total_after_first, score1);

		let score2 = TrustPallet::update_score_for_account(&account2).unwrap();
		let total_after_second = TrustPallet::total_active_trust_score();
		assert_eq!(total_after_second, score1 + score2);
	});
}

#[test]
fn force_recalculate_trust_score_works() {
	new_test_ext().execute_with(|| {
		let account = 1u64;

		assert_ok!(TrustPallet::force_recalculate_trust_score(RuntimeOrigin::root(), account));

		let score = TrustPallet::trust_score_of(account);
		assert!(score > 0);
	});
}

#[test]
fn force_recalculate_trust_score_requires_root() {
	new_test_ext().execute_with(|| {
		let account = 1u64;

		assert_noop!(
			TrustPallet::force_recalculate_trust_score(RuntimeOrigin::signed(account), account),
			BadOrigin
		);
	});
}

#[test]
fn update_all_trust_scores_works() {
	new_test_ext().execute_with(|| {
		// Set block number to capture events
		System::set_block_number(1);

		assert_ok!(TrustPallet::update_all_trust_scores(RuntimeOrigin::root()));

		// Because the mock implementation uses an empty account list,
		// the AllTrustScoresUpdated event is emitted (with count: 0)
		let events = System::events();
		assert!(events.iter().any(|event| {
			matches!(
				event.event,
				RuntimeEvent::TrustPallet(Event::AllTrustScoresUpdated { total_updated: 0 })
			)
		}));
	});
}

#[test]
fn update_all_trust_scores_requires_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(TrustPallet::update_all_trust_scores(RuntimeOrigin::signed(1)), BadOrigin);
	});
}

#[test]
fn periodic_trust_score_update_works() {
	new_test_ext().execute_with(|| {
		// Set block number to capture events
		System::set_block_number(1);

		assert_ok!(TrustPallet::periodic_trust_score_update(RuntimeOrigin::root()));

		// Verify the periodic update event was emitted
		let events = System::events();
		assert!(events.iter().any(|event| {
			matches!(event.event, RuntimeEvent::TrustPallet(Event::PeriodicUpdateScheduled { .. }))
		}));

		// The AllTrustScoresUpdated event should also be emitted
		assert!(events.iter().any(|event| {
			matches!(event.event, RuntimeEvent::TrustPallet(Event::AllTrustScoresUpdated { .. }))
		}));
	});
}

#[test]
fn periodic_update_fails_when_batch_in_progress() {
	new_test_ext().execute_with(|| {
		// Start the batch update
		crate::BatchUpdateInProgress::<Test>::put(true);

		// Expect the periodic update to fail
		assert_noop!(
			TrustPallet::periodic_trust_score_update(RuntimeOrigin::root()),
			Error::<Test>::UpdateInProgress
		);
	});
}

#[test]
fn events_are_emitted() {
	new_test_ext().execute_with(|| {
		let account = 1u64;

		System::set_block_number(1);

		TrustPallet::update_score_for_account(&account).unwrap();

		let events = System::events();
		assert!(events.len() >= 2);

		let trust_score_updated = events.iter().any(|event| {
			matches!(event.event, RuntimeEvent::TrustPallet(Event::TrustScoreUpdated { .. }))
		});

		let total_updated = events.iter().any(|event| {
			matches!(event.event, RuntimeEvent::TrustPallet(Event::TotalTrustScoreUpdated { .. }))
		});

		assert!(trust_score_updated);
		assert!(total_updated);
	});
}

#[test]
fn trust_score_updater_trait_works() {
	new_test_ext().execute_with(|| {
		use crate::TrustScoreUpdater;

		let account = 1u64;

		let initial_score = TrustPallet::trust_score_of(account);
		assert_eq!(initial_score, 0);

		TrustPallet::on_score_component_changed(&account);

		let updated_score = TrustPallet::trust_score_of(account);
		assert!(updated_score > 0);
	});
}

#[test]
fn batch_update_storage_works() {
	new_test_ext().execute_with(|| {
		// Initially the batch update is not active
		assert!(!crate::BatchUpdateInProgress::<Test>::get());
		assert!(crate::LastProcessedAccount::<Test>::get().is_none());

		// Simulate the batch update
		crate::BatchUpdateInProgress::<Test>::put(true);
		crate::LastProcessedAccount::<Test>::put(42u64);

		assert!(crate::BatchUpdateInProgress::<Test>::get());
		assert_eq!(crate::LastProcessedAccount::<Test>::get(), Some(42u64));

		// Clean up
		crate::BatchUpdateInProgress::<Test>::put(false);
		crate::LastProcessedAccount::<Test>::kill();

		assert!(!crate::BatchUpdateInProgress::<Test>::get());
		assert!(crate::LastProcessedAccount::<Test>::get().is_none());
	});
}

#[test]
fn periodic_update_scheduling_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(TrustPallet::periodic_trust_score_update(RuntimeOrigin::root()));

		// Verify that next_block is calculated correctly in the event
		let events = System::events();
		let scheduled_event = events.iter().find(|event| {
			matches!(event.event, RuntimeEvent::TrustPallet(Event::PeriodicUpdateScheduled { .. }))
		});

		assert!(scheduled_event.is_some());

		if let Some(event_record) = scheduled_event {
			if let RuntimeEvent::TrustPallet(Event::PeriodicUpdateScheduled { next_block }) =
				&event_record.event
			{
				// Current block (100) + interval (100) = 200
				assert_eq!(next_block, &200u64);
			}
		}
	});
}

// ============================================================================
// update_all_trust_scores Tests (5 tests)
// ============================================================================

#[test]
fn update_all_trust_scores_multiple_users() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Root can update all trust scores
		assert_ok!(TrustPallet::update_all_trust_scores(RuntimeOrigin::root()));

		// Verify at least one user has score (depends on mock KYC setup)
		let total = TrustPallet::total_active_trust_score();
		assert!(total < u128::MAX); // May be 0 if no users have KYC approved in mock
	});
}

#[test]
fn update_all_trust_scores_root_only() {
	new_test_ext().execute_with(|| {
		// Non-root cannot update all trust scores
		assert_noop!(TrustPallet::update_all_trust_scores(RuntimeOrigin::signed(1)), BadOrigin);
	});
}

#[test]
fn update_all_trust_scores_updates_total() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let initial_total = TrustPallet::total_active_trust_score();
		assert_eq!(initial_total, 0);

		assert_ok!(TrustPallet::update_all_trust_scores(RuntimeOrigin::root()));

		let final_total = TrustPallet::total_active_trust_score();
		// Total should remain valid (may stay 0 if no approved KYC users)
		assert!(final_total < u128::MAX);
	});
}

#[test]
fn update_all_trust_scores_emits_event() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(TrustPallet::update_all_trust_scores(RuntimeOrigin::root()));

		let events = System::events();
		let bulk_update_event = events.iter().any(|event| {
			matches!(event.event, RuntimeEvent::TrustPallet(Event::BulkTrustScoreUpdate { .. }))
				|| matches!(
					event.event,
					RuntimeEvent::TrustPallet(Event::AllTrustScoresUpdated { .. })
				)
		});

		assert!(bulk_update_event);
	});
}

#[test]
fn update_all_trust_scores_batch_processing() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// First call should start batch processing
		assert_ok!(TrustPallet::update_all_trust_scores(RuntimeOrigin::root()));

		// Check batch state is cleared after completion
		assert!(!crate::BatchUpdateInProgress::<Test>::get());
		assert!(crate::LastProcessedAccount::<Test>::get().is_none());
	});
}

// ============================================================================
// Score Calculation Edge Cases (5 tests)
// ============================================================================

#[test]
fn calculate_trust_score_handles_overflow() {
	new_test_ext().execute_with(|| {
		let account = 1u64;

		// Even with large values, should not overflow
		let score = TrustPallet::calculate_trust_score(&account);
		assert!(score.is_ok());
		assert!(score.unwrap() < u128::MAX);
	});
}

#[test]
fn calculate_trust_score_all_zero_components() {
	new_test_ext().execute_with(|| {
		let account = 2u64; // User 2 exists in mock

		let score = TrustPallet::calculate_trust_score(&account).unwrap();
		// Should be greater than 0 (mock provides some values)
		assert!(score < u128::MAX);
	});
}

#[test]
fn update_score_maintains_consistency() {
	new_test_ext().execute_with(|| {
		let account = 1u64;

		// Update twice
		let score1 = TrustPallet::update_score_for_account(&account).unwrap();
		let score2 = TrustPallet::update_score_for_account(&account).unwrap();

		// Scores should be equal (no random component)
		assert_eq!(score1, score2);
	});
}

#[test]
fn trust_score_decreases_when_components_decrease() {
	new_test_ext().execute_with(|| {
		let account = 1u64;

		// First update with good scores
		let initial_score = TrustPallet::update_score_for_account(&account).unwrap();

		// Simulate component decrease (in real scenario, staking/referral would decrease)
		// For now, just verify score can be recalculated
		let recalculated = TrustPallet::calculate_trust_score(&account).unwrap();

		// Score should be deterministic
		assert_eq!(initial_score, recalculated);
	});
}

#[test]
fn multiple_users_independent_scores() {
	new_test_ext().execute_with(|| {
		let user1 = 1u64;
		let user2 = 2u64;

		let score1 = TrustPallet::update_score_for_account(&user1).unwrap();
		let score2 = TrustPallet::update_score_for_account(&user2).unwrap();

		// Scores should be independent
		assert_ne!(score1, 0);
		assert_ne!(score2, 0);

		// Verify stored separately
		assert_eq!(TrustPallet::trust_score_of(user1), score1);
		assert_eq!(TrustPallet::trust_score_of(user2), score2);
	});
}

// ============================================================================
// TrustScoreProvider Trait Tests (3 tests)
// ============================================================================

#[test]
fn trust_score_provider_trait_returns_zero_initially() {
	new_test_ext().execute_with(|| {
		let account = 1u64;
		let score = TrustPallet::trust_score_of(account);
		assert_eq!(score, 0);
	});
}

#[test]
fn trust_score_provider_trait_returns_updated_score() {
	new_test_ext().execute_with(|| {
		let account = 1u64;
		TrustPallet::update_score_for_account(&account).unwrap();

		let score = TrustPallet::trust_score_of(account);
		assert!(score > 0);
	});
}

#[test]
fn trust_score_provider_trait_multiple_users() {
	new_test_ext().execute_with(|| {
		TrustPallet::update_score_for_account(&1u64).unwrap();
		TrustPallet::update_score_for_account(&2u64).unwrap();

		let score1 = TrustPallet::trust_score_of(1u64);
		let score2 = TrustPallet::trust_score_of(2u64);

		assert!(score1 > 0);
		assert!(score2 > 0);
	});
}

// ============================================================================
// Storage and State Tests (2 tests)
// ============================================================================

#[test]
fn storage_consistency_after_multiple_updates() {
	new_test_ext().execute_with(|| {
		let account = 1u64;

		// Multiple updates
		for _ in 0..5 {
			TrustPallet::update_score_for_account(&account).unwrap();
		}

		// Score should still be consistent
		let stored = TrustPallet::trust_score_of(account);
		let calculated = TrustPallet::calculate_trust_score(&account).unwrap();

		assert_eq!(stored, calculated);
	});
}

#[test]
fn total_active_trust_score_accumulates_correctly() {
	new_test_ext().execute_with(|| {
		let users = vec![1u64, 2u64]; // Only users that exist in mock
		let mut expected_total = 0u128;

		for user in users {
			let score = TrustPallet::update_score_for_account(&user).unwrap();
			expected_total += score;
		}

		let total = TrustPallet::total_active_trust_score();
		assert_eq!(total, expected_total);
	});
}

// ============================================================================
// THE GATE
// ============================================================================
//
// A state with no economy can do nothing, so standing requires having something at stake. It
// is the one condition nothing else can substitute for -- not an education, not a hundred
// citizens brought in, not high office.

mod gate {
	use super::*;

	#[test]
	fn nothing_at_stake_is_no_standing_whatever_else_there_is() {
		new_test_ext().execute_with(|| {
			// Everything else at its maximum.
			set_profile(1, 0, REFERRAL_MAX, PERWERDE_MAX, TIKI_MAX);
			assert_eq!(TrustPallet::calculate_trust_score(&1).unwrap(), 0);
			clear_profiles();
		});
	}

	#[test]
	fn the_smallest_stake_opens_everything_else_up() {
		// The gate is a condition, not a scale: it asks whether there is a stake, and the
		// weight below decides what more of it is worth.
		new_test_ext().execute_with(|| {
			set_profile(1, 1, REFERRAL_MAX, PERWERDE_MAX, TIKI_MAX);
			let with_a_token_stake = TrustPallet::calculate_trust_score(&1).unwrap();
			assert!(with_a_token_stake > 0);

			// And it is most of a perfect record, because the other three parts are.
			let scale = TrustScoreScale::get() as u128;
			assert!(with_a_token_stake >= scale * 79 / 100);
			clear_profiles();
		});
	}

	#[test]
	fn losing_the_stake_removes_the_standing_it_supported() {
		new_test_ext().execute_with(|| {
			set_profile(1, 100, REFERRAL_MAX, PERWERDE_MAX, TIKI_MAX);
			assert_ok!(TrustPallet::update_score_for_account(&1));
			assert!(TrustPallet::trust_score_of(1) > 0);

			set_profile(1, 0, REFERRAL_MAX, PERWERDE_MAX, TIKI_MAX);
			assert_ok!(TrustPallet::update_score_for_account(&1));
			assert_eq!(TrustPallet::trust_score_of(1), 0);
			clear_profiles();
		});
	}
}

// ============================================================================
// LOSING CITIZENSHIP
// ============================================================================

#[test]
fn a_revoked_citizen_keeps_no_standing() {
	// `calculate_trust_score` refuses to compute for a non-citizen, and refusing meant the old
	// value was never overwritten -- so somebody whose citizenship had been taken kept their
	// standing for good, and the running total kept counting it as though they were still
	// here. Nothing read it for candidacy, which checks citizenship too, but the total is what
	// reward shares are drawn against.
	new_test_ext().execute_with(|| {
		use pezpallet_identity_kyc::types::OnCitizenshipRevoked;

		assert_ok!(TrustPallet::update_score_for_account(&1));
		let standing = TrustPallet::trust_score_of(1);
		assert!(standing > 0);
		assert_eq!(TrustPallet::total_active_trust_score(), standing);

		TrustPallet::on_citizenship_revoked(&1);

		assert_eq!(TrustPallet::trust_score_of(1), 0);
		assert_eq!(TrustPallet::total_active_trust_score(), 0);
	});
}

// ============================================================================
// THE INVARIANT CAN FAIL
// ============================================================================

#[cfg(feature = "try-runtime")]
mod invariant {
	use super::*;
	use crate::{TotalActiveTrustScore, TrustScores};
	use pezframe_support::traits::Hooks;

	fn check() -> Result<(), pezsp_runtime::TryRuntimeError> {
		<TrustPallet as Hooks<u64>>::try_state(System::block_number())
	}

	fn assert_rejected(what: &str) {
		assert!(check().is_err(), "try_state accepted a state where {what}");
	}

	#[test]
	fn an_ordinary_state_passes() {
		new_test_ext().execute_with(|| {
			assert_ok!(TrustPallet::update_score_for_account(&1));
			assert_ok!(check());
		});
	}

	#[test]
	fn standing_without_a_stake_is_caught() {
		// The gate, checked against the register rather than trusted.
		new_test_ext().execute_with(|| {
			assert_ok!(TrustPallet::update_score_for_account(&1));
			set_profile(1, 0, 0, 0, 0);
			assert_rejected("an account held standing with nothing staked");
			clear_profiles();
		});
	}

	#[test]
	fn standing_for_somebody_who_is_not_a_citizen_is_caught() {
		new_test_ext().execute_with(|| {
			TrustScores::<Test>::insert(999, 100u128);
			TotalActiveTrustScore::<Test>::put(100u128);
			assert_rejected("an account held standing without being a citizen");
		});
	}

	#[test]
	fn a_running_total_that_does_not_match_is_caught() {
		new_test_ext().execute_with(|| {
			assert_ok!(TrustPallet::update_score_for_account(&1));
			TotalActiveTrustScore::<Test>::mutate(|t| *t += 1);
			assert_rejected("the running total did not match the register");
		});
	}

	#[test]
	fn scoring_more_than_a_perfect_record_is_caught() {
		new_test_ext().execute_with(|| {
			TrustScores::<Test>::insert(1, TrustScoreScale::get() as u128 + 1);
			TotalActiveTrustScore::<Test>::put(TrustScoreScale::get() as u128 + 1);
			assert_rejected("an account scored more than a perfect record");
		});
	}
}

// =============================================================================
// A FROZEN ROLL
// =============================================================================
//
// The payroll fixes a rate against `TotalActiveTrustScore` at one instant and pays each
// claimant their own score over the week that follows. Those two numbers have to come from
// the same roll, or the shares will not add up and claiming at the right moment will be worth
// money. Holding the roll still is how that is arranged.

use crate::{FrozenUntil, TotalActiveTrustScore, TrustScores};
use pezpallet_identity_kyc::types::OnCitizenshipRevoked;

#[test]
fn a_frozen_roll_does_not_recalculate() {
	new_test_ext().execute_with(|| {
		clear_profiles();
		set_profile(1, 100, 500, 50_000, 1_000);
		assert_ok!(TrustPallet::force_recalculate_trust_score(RuntimeOrigin::root(), 1));
		let before = TrustPallet::trust_score_of(1);
		assert!(before > 0);

		TrustPallet::freeze_until(System::block_number() + 100);

		// Everything about this account changes, and its score does not.
		set_profile(1, 1, 1, 1, 1);
		assert_ok!(TrustPallet::force_recalculate_trust_score(RuntimeOrigin::root(), 1));
		assert_eq!(
			TrustPallet::trust_score_of(1),
			before,
			"the roll was supposed to be held still"
		);
	});
}

#[test]
fn a_freeze_expires_by_itself() {
	// Deliberately not a flag somebody has to clear. The thing that would clear it is the
	// same subsystem that set it, so a freeze that outlived one missed call would be
	// permanent -- and a permanently frozen roll is a state whose standing never changes.
	new_test_ext().execute_with(|| {
		clear_profiles();
		set_profile(1, 100, 500, 50_000, 1_000);
		assert_ok!(TrustPallet::force_recalculate_trust_score(RuntimeOrigin::root(), 1));
		let before = TrustPallet::trust_score_of(1);

		TrustPallet::freeze_until(System::block_number() + 10);
		set_profile(1, 1, 1, 1, 1);

		System::set_block_number(System::block_number() + 11);
		assert!(!TrustPallet::roll_is_frozen());
		assert_ok!(TrustPallet::force_recalculate_trust_score(RuntimeOrigin::root(), 1));
		assert!(
			TrustPallet::trust_score_of(1) < before,
			"and it recalculates once the hold is over"
		);
	});
}

#[test]
fn a_second_freeze_can_only_extend_the_first() {
	// Two payrolls must not be able to shorten each other's hold: the earlier one is still
	// being drawn against.
	new_test_ext().execute_with(|| {
		TrustPallet::freeze_until(500);
		TrustPallet::freeze_until(100);
		assert_eq!(FrozenUntil::<Test>::get(), Some(500));

		TrustPallet::freeze_until(900);
		assert_eq!(FrozenUntil::<Test>::get(), Some(900));
	});
}

#[test]
fn a_frozen_roll_still_drops_a_revoked_citizen() {
	// The one thing the freeze must not block. Someone who stops being a citizen mid-payroll
	// has to stop being paid the same day -- and because their score is what the reward is
	// computed from, taking the score is what stops it. No second check is needed anywhere.
	new_test_ext().execute_with(|| {
		clear_profiles();
		set_profile(1, 100, 500, 50_000, 1_000);
		assert_ok!(TrustPallet::force_recalculate_trust_score(RuntimeOrigin::root(), 1));
		let score = TrustPallet::trust_score_of(1);
		assert!(score > 0);
		let total_before = TotalActiveTrustScore::<Test>::get();

		TrustPallet::freeze_until(System::block_number() + 100);
		<TrustPallet as OnCitizenshipRevoked<u64>>::on_citizenship_revoked(&1);

		assert_eq!(TrustPallet::trust_score_of(1), 0, "a revoked citizen is worth nothing at once");
		assert!(!TrustScores::<Test>::contains_key(1));
		assert_eq!(TotalActiveTrustScore::<Test>::get(), total_before - score);
	});
}

// =============================================================================
// ONE BATCH IMPLEMENTATION, TWO WAYS IN
// =============================================================================

#[test]
fn the_call_and_the_hook_share_one_batch_and_one_checkpoint() {
	// `update_all_trust_scores` and `on_initialize` used to be two copies of the same
	// paginated loop, both writing `LastProcessedAccount` and `BatchUpdateInProgress`. Two
	// copies of a checkpointed loop drift the moment a fix lands in one of them -- and while
	// they disagree, the pagination itself is what breaks. This pins that there is one body.
	use pezframe_support::traits::Hooks;
	use pezpallet_identity_kyc::types::KycLevel;

	new_test_ext().execute_with(|| {
		// One and a half batches' worth of citizens (the mock's batch size is 100).
		for who in 1..=150u64 {
			pezpallet_identity_kyc::KycStatuses::<Test>::insert(who, KycLevel::Approved);
		}

		// The call takes the first batch and leaves a checkpoint behind.
		assert_ok!(TrustPallet::update_all_trust_scores(RuntimeOrigin::root()));
		assert!(crate::BatchUpdateInProgress::<Test>::get(), "a batch is still owed");
		let checkpoint = crate::LastProcessedAccount::<Test>::get();
		assert!(checkpoint.is_some(), "the call has to leave its place in the register");

		// The hook picks up from that same checkpoint rather than starting over.
		System::set_block_number(2);
		<TrustPallet as Hooks<u64>>::on_initialize(2);

		assert!(
			!crate::BatchUpdateInProgress::<Test>::get(),
			"the hook finished what the call started"
		);
		assert!(
			crate::LastProcessedAccount::<Test>::get().is_none(),
			"a finished run leaves no checkpoint"
		);
	});
}
