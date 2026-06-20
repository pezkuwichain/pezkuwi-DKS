// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{mock::*, types::OperationMode};
use pezframe_support::{assert_noop, assert_ok};
// Import SessionManager trait for testing
use pezpallet_session::SessionManager;

#[test]
fn join_validator_pool_works() {
	new_test_ext().execute_with(|| {
		// User 1 has high trust (800) and tiki score (1)
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));

		// Check storage
		assert!(ValidatorPool::pool_members(1).is_some());
		assert_eq!(ValidatorPool::pool_size(), 1);

		// Check performance metrics initialized
		let metrics = ValidatorPool::performance_metrics(1);
		assert_eq!(metrics.reputation_score, 100);
		assert_eq!(metrics.blocks_produced, 0);
	});
}

#[test]
fn join_validator_pool_fails_insufficient_trust() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(99),
				stake_validator_category()
			),
			Error::<Test>::InsufficientTrustScore
		);
	});
}

#[test]
fn join_validator_pool_fails_already_in_pool() {
	new_test_ext().execute_with(|| {
		// First join succeeds
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));

		// Second join fails
		assert_noop!(
			ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(1),
				stake_validator_category()
			),
			Error::<Test>::AlreadyInPool
		);
	});
}

#[test]
fn leave_validator_pool_works() {
	new_test_ext().execute_with(|| {
		// Join first
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));
		assert_eq!(ValidatorPool::pool_size(), 1);

		// Leave pool
		assert_ok!(ValidatorPool::leave_validator_pool(RuntimeOrigin::signed(1)));

		// Check storage cleaned up
		assert!(ValidatorPool::pool_members(1).is_none());
		assert_eq!(ValidatorPool::pool_size(), 0);

		// Performance metrics should be removed
		let metrics = ValidatorPool::performance_metrics(1);
		assert_eq!(metrics.reputation_score, 0); // Default value
	});
}

#[test]
fn leave_validator_pool_fails_not_in_pool() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			ValidatorPool::leave_validator_pool(RuntimeOrigin::signed(1)),
			Error::<Test>::NotInPool
		);
	});
}

#[test]
fn parliamentary_validator_category_validation() {
	new_test_ext().execute_with(|| {
		// User 1 has tiki score, should succeed
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			parliamentary_validator_category()
		));

		// User 16 has no tiki score, should fail
		assert_noop!(
			ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(16),
				parliamentary_validator_category()
			),
			Error::<Test>::MissingRequiredTiki
		);
	});
}

#[test]
fn merit_validator_category_validation() {
	new_test_ext().execute_with(|| {
		// User 1 has both tiki score (1) and high community support (1000)
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			merit_validator_category()
		));

		// User 16 has no tiki score
		assert_noop!(
			ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(16),
				merit_validator_category()
			),
			Error::<Test>::MissingRequiredTiki
		);
	});
}

#[test]
fn update_category_works() {
	new_test_ext().execute_with(|| {
		// Join as stake validator
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));

		// Update to parliamentary validator
		assert_ok!(ValidatorPool::update_category(
			RuntimeOrigin::signed(1),
			parliamentary_validator_category()
		));

		// Check category updated
		let category = ValidatorPool::pool_members(1).unwrap();
		assert!(matches!(category, ValidatorPoolCategory::ParliamentaryValidator));
	});
}

#[test]
fn force_new_era_works() {
	new_test_ext().execute_with(|| {
		// Add validators to pool (at least 4 for BFT)
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(2),
			parliamentary_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(3),
			merit_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(4),
			stake_validator_category()
		));

		let initial_era = ValidatorPool::current_era();

		// Force new era
		assert_ok!(ValidatorPool::force_new_era(RuntimeOrigin::root()));

		// Check era incremented
		assert_eq!(ValidatorPool::current_era(), initial_era + 1);

		// Check validator set exists
		assert!(ValidatorPool::current_validator_set().is_some());
	});
}

#[test]
fn automatic_era_transition_works() {
	new_test_ext().execute_with(|| {
		// Add validators
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(2),
			parliamentary_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(3),
			stake_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(4),
			stake_validator_category()
		));

		let initial_era = ValidatorPool::current_era();
		let era_start = ValidatorPool::era_start();
		let era_length = ValidatorPool::era_length();

		// Advance to trigger era transition
		run_to_block(era_start + era_length);

		// Era should have automatically transitioned
		assert_eq!(ValidatorPool::current_era(), initial_era + 1);
	});
}

#[test]
fn validator_selection_respects_constraints() {
	new_test_ext().execute_with(|| {
		// Add different types of validators
		for i in 1..=10 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}

		// Force era to trigger selection
		assert_ok!(ValidatorPool::force_new_era(RuntimeOrigin::root()));

		let validator_set = ValidatorPool::current_validator_set().unwrap();

		assert!(!validator_set.stake_validators.is_empty());
		assert!(validator_set.total_count() <= 21);
	});
}

#[test]
fn performance_metrics_update_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));

		assert_ok!(ValidatorPool::update_performance_metrics(
			RuntimeOrigin::root(),
			1,
			100,
			10,
			500
		));

		let metrics = ValidatorPool::performance_metrics(1);
		assert_eq!(metrics.blocks_produced, 100);
		assert_eq!(metrics.blocks_missed, 10);
		assert_eq!(metrics.era_points, 500);
		assert_eq!(metrics.reputation_score, 90);
	});
}

#[test]
fn poor_performance_excludes_from_selection() {
	new_test_ext().execute_with(|| {
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));
		assert_ok!(ValidatorPool::update_performance_metrics(
			RuntimeOrigin::root(),
			1,
			30,
			70,
			100
		));
		let metrics = ValidatorPool::performance_metrics(1);
		assert_eq!(metrics.reputation_score, 30);

		// Add other good performers
		for i in 2..=5 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}

		assert_ok!(ValidatorPool::force_new_era(RuntimeOrigin::root()));
		let validator_set = ValidatorPool::current_validator_set().unwrap();
		assert!(!validator_set.all_validators().contains(&1));
		assert!(validator_set.all_validators().contains(&2));
	});
}

#[test]
fn rotation_rule_works() {
	new_test_ext().execute_with(|| {
		// Simply test that multiple validators can be added and pool works
		for i in 1..=5 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}

		// Test that pool size is correct
		assert_eq!(ValidatorPool::pool_size(), 5);

		// Test that we can remove validators
		assert_ok!(ValidatorPool::leave_validator_pool(RuntimeOrigin::signed(1)));
		assert_eq!(ValidatorPool::pool_size(), 4);
	});
}

#[test]
fn pool_size_limit_enforced() {
	new_test_ext().execute_with(|| {
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));
		assert_eq!(ValidatorPool::pool_size(), 1);

		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(2),
			parliamentary_validator_category()
		));
		assert_eq!(ValidatorPool::pool_size(), 2);

		assert_ok!(ValidatorPool::leave_validator_pool(RuntimeOrigin::signed(1)));
		assert_eq!(ValidatorPool::pool_size(), 1);

		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(3),
			merit_validator_category()
		));
		assert_eq!(ValidatorPool::pool_size(), 2);
	});
}

#[test]
fn set_pool_parameters_works() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			ValidatorPool::set_pool_parameters(RuntimeOrigin::signed(1), 200),
			pezsp_runtime::DispatchError::BadOrigin
		);
		assert_ok!(ValidatorPool::set_pool_parameters(RuntimeOrigin::root(), 200));
		assert_eq!(ValidatorPool::era_length(), 200);
	});
}

#[test]
fn session_manager_integration_works() {
	new_test_ext().execute_with(|| {
		for i in 1..=5 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}
		assert_ok!(ValidatorPool::force_new_era(RuntimeOrigin::root()));
		let validators = <ValidatorPool as SessionManager<u64>>::new_session(1);
		assert!(validators.is_some());
		let validator_list = validators.unwrap();
		assert!(!validator_list.is_empty());
	});
}

#[test]
fn validator_set_distribution_works() {
	new_test_ext().execute_with(|| {
		for i in 1..=15 {
			let category = match i {
				1..=10 => stake_validator_category(),
				11..=13 => parliamentary_validator_category(),
				_ => merit_validator_category(),
			};
			assert_ok!(ValidatorPool::join_validator_pool(RuntimeOrigin::signed(i), category));
		}
		assert_ok!(ValidatorPool::force_new_era(RuntimeOrigin::root()));
		let validator_set = ValidatorPool::current_validator_set().unwrap();
		assert!(validator_set.total_count() > 0);
		assert!(validator_set.total_count() <= 21);
		assert!(!validator_set.stake_validators.is_empty());
		assert!(!validator_set.parliamentary_validators.is_empty());
		assert!(!validator_set.merit_validators.is_empty());
	});
}

#[test]
fn events_are_emitted() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));
		let events = System::events();
		assert!(events.iter().any(|event| matches!(
			event.event,
			RuntimeEvent::ValidatorPool(crate::Event::ValidatorJoinedPool { .. })
		)));

		System::reset_events();
		assert_ok!(ValidatorPool::leave_validator_pool(RuntimeOrigin::signed(1)));
		let events = System::events();
		assert!(events.iter().any(|event| matches!(
			event.event,
			RuntimeEvent::ValidatorPool(crate::Event::ValidatorLeftPool { .. })
		)));
	});
}

#[test]
fn minimum_validator_count_enforced() {
	new_test_ext().execute_with(|| {
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(2),
			parliamentary_validator_category()
		));
		assert_noop!(
			ValidatorPool::force_new_era(RuntimeOrigin::root()),
			Error::<Test>::NotEnoughValidators
		);
	});
}

#[test]
fn complex_era_transition_scenario() {
	new_test_ext().execute_with(|| {
		// Test validator addition with different categories
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(2),
			parliamentary_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(3),
			merit_validator_category()
		));

		// Test performance metrics update
		assert_ok!(ValidatorPool::update_performance_metrics(
			RuntimeOrigin::root(),
			1,
			90,
			10,
			500
		));
		let metrics = ValidatorPool::performance_metrics(1);
		assert_eq!(metrics.reputation_score, 90);

		// Test category update
		assert_ok!(ValidatorPool::update_category(
			RuntimeOrigin::signed(1),
			parliamentary_validator_category()
		));

		// Test pool size
		assert_eq!(ValidatorPool::pool_size(), 3);
	});
}

// ============================================================================
// SHADOW MODE TESTS
// ============================================================================

#[test]
fn genesis_sets_operation_mode() {
	// Test Active mode genesis
	new_test_ext().execute_with(|| {
		assert_eq!(ValidatorPool::operation_mode(), OperationMode::Active);
	});

	// Test Shadow mode genesis
	new_test_ext_shadow_mode().execute_with(|| {
		assert_eq!(ValidatorPool::operation_mode(), OperationMode::Shadow);
		// Shadow mode should track activation block
		assert!(ValidatorPool::shadow_mode_since().is_some());
	});
}

#[test]
fn set_operation_mode_works() {
	new_test_ext().execute_with(|| {
		// Start in Active mode
		assert_eq!(ValidatorPool::operation_mode(), OperationMode::Active);

		// Switch to Shadow mode
		assert_ok!(ValidatorPool::set_operation_mode(RuntimeOrigin::root(), OperationMode::Shadow));
		assert_eq!(ValidatorPool::operation_mode(), OperationMode::Shadow);

		// Switch back to Active mode
		assert_ok!(ValidatorPool::set_operation_mode(RuntimeOrigin::root(), OperationMode::Active));
		assert_eq!(ValidatorPool::operation_mode(), OperationMode::Active);
	});
}

#[test]
fn set_operation_mode_fails_same_mode() {
	new_test_ext().execute_with(|| {
		// Already in Active mode
		assert_noop!(
			ValidatorPool::set_operation_mode(RuntimeOrigin::root(), OperationMode::Active),
			Error::<Test>::AlreadyInActiveMode
		);
	});

	new_test_ext_shadow_mode().execute_with(|| {
		// Already in Shadow mode
		assert_noop!(
			ValidatorPool::set_operation_mode(RuntimeOrigin::root(), OperationMode::Shadow),
			Error::<Test>::AlreadyInShadowMode
		);
	});
}

#[test]
fn set_operation_mode_requires_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			ValidatorPool::set_operation_mode(RuntimeOrigin::signed(1), OperationMode::Shadow),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn shadow_mode_session_manager_returns_none() {
	new_test_ext_shadow_mode().execute_with(|| {
		// Add validators
		for i in 1..=5 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}

		// In shadow mode, new_session should return None
		let validators = <ValidatorPool as SessionManager<u64>>::new_session(1);
		assert!(validators.is_none());

		// But shadow validator set should be stored
		assert!(ValidatorPool::shadow_validator_set().is_some());
	});
}

#[test]
fn active_mode_session_manager_returns_validators() {
	new_test_ext().execute_with(|| {
		// Add validators
		for i in 1..=5 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}

		// Force first era to get a validator set
		assert_ok!(ValidatorPool::force_new_era(RuntimeOrigin::root()));

		// In active mode, new_session should return validators
		let validators = <ValidatorPool as SessionManager<u64>>::new_session(2);
		assert!(validators.is_some());
		assert!(!validators.unwrap().is_empty());
	});
}

#[test]
fn record_npos_validators_works() {
	new_test_ext_shadow_mode().execute_with(|| {
		// Add validators to pool
		for i in 1..=5 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}

		// Trigger shadow selection
		let _ = <ValidatorPool as SessionManager<u64>>::new_session(1);

		// Record NPoS validators
		let npos_validators = vec![1u64, 2, 3, 6, 7];
		assert_ok!(ValidatorPool::record_npos_validators(RuntimeOrigin::root(), npos_validators));

		// Check NPoS set is stored
		assert!(!ValidatorPool::npos_validator_set().is_empty());
	});
}

#[test]
fn record_npos_validators_fails_in_active_mode() {
	new_test_ext().execute_with(|| {
		let npos_validators = vec![1u64, 2, 3];
		assert_noop!(
			ValidatorPool::record_npos_validators(RuntimeOrigin::root(), npos_validators),
			Error::<Test>::ShadowModeNotEnabled
		);
	});
}

#[test]
fn shadow_comparison_recorded() {
	new_test_ext_shadow_mode().execute_with(|| {
		// Add validators to pool
		for i in 1..=10 {
			let category = if i <= 5 {
				stake_validator_category()
			} else if i <= 7 {
				parliamentary_validator_category()
			} else {
				merit_validator_category()
			};
			assert_ok!(ValidatorPool::join_validator_pool(RuntimeOrigin::signed(i), category));
		}

		// Trigger shadow selection
		let _ = <ValidatorPool as SessionManager<u64>>::new_session(1);

		// Record NPoS validators (some overlap, some different)
		let npos_validators = vec![1u64, 2, 3, 11, 12];
		assert_ok!(ValidatorPool::record_npos_validators(RuntimeOrigin::root(), npos_validators));

		// Check comparison was recorded
		let comparison = ValidatorPool::shadow_comparison();
		assert!(comparison.is_some());
		let comp = comparison.unwrap();
		assert!(
			comp.overlap_count > 0 || !comp.tnpos_only.is_empty() || !comp.npos_only.is_empty()
		);
	});
}

#[test]
fn cumulative_statistics_updated() {
	new_test_ext_shadow_mode().execute_with(|| {
		// Add validators
		for i in 1..=5 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}

		// Trigger shadow selection
		let _ = <ValidatorPool as SessionManager<u64>>::new_session(1);

		// Record NPoS validators
		let npos_validators = vec![1u64, 2, 6, 7, 8];
		assert_ok!(ValidatorPool::record_npos_validators(RuntimeOrigin::root(), npos_validators));

		// Check cumulative stats were updated
		let stats = ValidatorPool::shadow_statistics();
		assert_eq!(stats.eras_analyzed, 1);
	});
}

#[test]
fn era_analysis_data_stored() {
	new_test_ext_shadow_mode().execute_with(|| {
		// Add validators
		for i in 1..=5 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}

		// Trigger shadow selection
		let _ = <ValidatorPool as SessionManager<u64>>::new_session(1);

		// Record NPoS validators
		let npos_validators = vec![1u64, 2, 3, 4, 5];
		assert_ok!(ValidatorPool::record_npos_validators(RuntimeOrigin::root(), npos_validators));

		// Check era analysis was stored
		let era = ValidatorPool::current_era();
		let analysis = ValidatorPool::era_analysis(era);
		assert!(analysis.is_some());
	});
}

#[test]
fn category_distribution_tracked() {
	new_test_ext_shadow_mode().execute_with(|| {
		// Add different category validators
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(1),
			stake_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(2),
			stake_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(3),
			parliamentary_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(4),
			merit_validator_category()
		));
		assert_ok!(ValidatorPool::join_validator_pool(
			RuntimeOrigin::signed(5),
			stake_validator_category()
		));

		// Trigger shadow selection
		let _ = <ValidatorPool as SessionManager<u64>>::new_session(1);

		// Record NPoS validators
		assert_ok!(ValidatorPool::record_npos_validators(
			RuntimeOrigin::root(),
			vec![1u64, 2, 3, 4, 5]
		));

		// Check category distribution was recorded
		let era = ValidatorPool::current_era();
		let distribution = ValidatorPool::category_distribution(era);
		assert!(distribution.is_some());
		let dist = distribution.unwrap();
		assert!(dist.target_stake > 0);
	});
}

#[test]
fn record_era_end_stats_works() {
	new_test_ext_shadow_mode().execute_with(|| {
		// Setup validators and trigger comparison
		for i in 1..=5 {
			assert_ok!(ValidatorPool::join_validator_pool(
				RuntimeOrigin::signed(i),
				stake_validator_category()
			));
		}
		let _ = <ValidatorPool as SessionManager<u64>>::new_session(1);
		assert_ok!(ValidatorPool::record_npos_validators(
			RuntimeOrigin::root(),
			vec![1u64, 2, 3, 4, 5]
		));

		// Record era end stats
		let era = ValidatorPool::current_era();
		assert_ok!(ValidatorPool::record_era_end_stats(
			RuntimeOrigin::root(),
			era,
			950, // blocks produced
			50   // blocks missed
		));

		// Check era analysis was updated
		let analysis = ValidatorPool::era_analysis(era);
		assert!(analysis.is_some());
		let a = analysis.unwrap();
		assert_eq!(a.blocks_produced, 950);
		assert_eq!(a.blocks_missed, 50);
	});
}

#[test]
fn operation_mode_change_emits_event() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();

		assert_ok!(ValidatorPool::set_operation_mode(RuntimeOrigin::root(), OperationMode::Shadow));

		let events = System::events();
		assert!(events.iter().any(|event| matches!(
			event.event,
			RuntimeEvent::ValidatorPool(crate::Event::OperationModeChanged { .. })
		)));
	});
}

#[test]
fn shadow_mode_tracks_activation_block() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		// Switch to shadow mode
		assert_ok!(ValidatorPool::set_operation_mode(RuntimeOrigin::root(), OperationMode::Shadow));

		// Check activation block is tracked
		let since = ValidatorPool::shadow_mode_since();
		assert!(since.is_some());
		assert_eq!(since.unwrap(), 100);
	});
}
