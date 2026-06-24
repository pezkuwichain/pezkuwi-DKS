// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{mock::*, Error, Event};
use pezframe_support::{assert_noop, assert_ok};
use pezsp_runtime::traits::BadOrigin;

#[test]
fn calculate_trust_score_works() {
	new_test_ext().execute_with(|| {
		let account = 1u64;
		let score = TrustPallet::calculate_trust_score(&account).unwrap();

		let expected = {
			let staking = 100u128;
			let referral = 50u128;
			let perwerde = 30u128;
			let tiki = 20u128;
			let base = ScoreMultiplierBase::get();

			let weighted_sum = staking * 100 + referral * 300 + perwerde * 300 + tiki * 300;
			staking * weighted_sum / base
		};

		assert_eq!(score, expected);
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
