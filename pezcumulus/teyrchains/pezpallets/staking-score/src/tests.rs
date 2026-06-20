// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Tests for pezpallet-staking-score.
//! All tests use receive_staking_details to populate CachedStakingDetails,
//! mirroring the real People Chain architecture.

use crate::{
	mock::*, CachedStakingDetails, Error, Event, StakingScoreProvider, StakingSource,
	StakingStartBlock, MONTH_IN_BLOCKS, UNITS,
};
use pezframe_support::{assert_noop, assert_ok};

const USER_STASH: AccountId = 10;

// ============================================================================
// Basic Score Calculation
// ============================================================================

#[test]
fn zero_stake_should_return_zero_score() {
	ExtBuilder.build_and_execute(|| {
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 0);
	});
}

#[test]
fn score_is_calculated_correctly_without_time_tracking() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			50 * UNITS,
			0,
			0
		));

		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 20);
	});
}

#[test]
fn start_score_tracking_works_and_enables_duration_multiplier() {
	ExtBuilder.build_and_execute(|| {
		let initial_block = 10u64;
		System::set_block_number(initial_block);

		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			500 * UNITS,
			0,
			0
		));

		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);

		// After 4 months: 40 * 1.4 = 56
		let target_block_4m = initial_block + (4 * MONTH_IN_BLOCKS) as u64;
		System::set_block_number(target_block_4m);

		let (score_4m, duration_4m) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(duration_4m, target_block_4m - initial_block);
		assert_eq!(score_4m, 56);

		// After 13 months: 40 * 2.0 = 80
		let target_block_13m = initial_block + (13 * MONTH_IN_BLOCKS) as u64;
		System::set_block_number(target_block_13m);

		let (score_13m, duration_13m) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(duration_13m, target_block_13m - initial_block);
		assert_eq!(score_13m, 80);
	});
}

#[test]
fn get_staking_score_works_without_explicit_tracking() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			751 * UNITS,
			0,
			0
		));

		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 50);

		// Even after time passes, score stays the same without tracking
		System::set_block_number(1_000_000_000);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 50);
	});
}

// ============================================================================
// Amount-Based Scoring Tiers
// ============================================================================

#[test]
fn amount_score_boundary_100_hez() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			100 * UNITS,
			0,
			0
		));

		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 20);
	});
}

#[test]
fn amount_score_boundary_250_hez() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			250 * UNITS,
			0,
			0
		));

		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 30);
	});
}

#[test]
fn amount_score_boundary_750_hez() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			750 * UNITS,
			0,
			0
		));

		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);
	});
}

#[test]
fn score_capped_at_100() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			1000 * UNITS,
			0,
			0
		));

		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		// After 12+ months: 50 * 2.0 = 100 (capped)
		System::set_block_number((12 * MONTH_IN_BLOCKS + 1) as u64);

		let (score, _) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(score, 100);
	});
}

// ============================================================================
// Duration Multiplier Tests
// ============================================================================

#[test]
fn duration_multiplier_1_month() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			500 * UNITS,
			0,
			0
		));

		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		System::set_block_number((MONTH_IN_BLOCKS + 1) as u64);

		// 40 * 1.2 = 48
		let (score, _) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(score, 48);
	});
}

#[test]
fn duration_multiplier_6_months() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			500 * UNITS,
			0,
			0
		));

		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		System::set_block_number((6 * MONTH_IN_BLOCKS + 1) as u64);

		// 40 * 1.7 = 68
		let (score, _) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(score, 68);
	});
}

#[test]
fn duration_multiplier_progression() {
	ExtBuilder.build_and_execute(|| {
		let base_block = 100u64;
		System::set_block_number(base_block);

		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			100 * UNITS,
			0,
			0
		));

		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		// Start: 20 * 1.0 = 20
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 20);

		// After 3 months: 20 * 1.4 = 28
		System::set_block_number(base_block + (3 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 28);

		// After 12 months: 20 * 2.0 = 40
		System::set_block_number(base_block + (12 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);
	});
}

// ============================================================================
// start_score_tracking Extrinsic Tests
// ============================================================================

#[test]
fn start_tracking_works_without_stake() {
	ExtBuilder.build_and_execute(|| {
		// Opt-in without any cached staking data — should succeed.
		// Bot + noter will submit staking data later.
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		// StakingStartBlock is set, but score is 0 (no cached data yet).
		assert!(StakingStartBlock::<Test>::get(USER_STASH).is_some());
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 0);
	});
}

#[test]
fn start_tracking_fails_if_already_started() {
	ExtBuilder.build_and_execute(|| {
		// First opt-in succeeds (no stake needed).
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		// Second attempt fails.
		assert_noop!(
			StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)),
			Error::<Test>::TrackingAlreadyStarted
		);
	});
}

#[test]
fn start_tracking_emits_event() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		// Opt-in without stake — event should still fire.
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		let events = System::events();
		assert!(events.iter().any(|event| {
			matches!(event.event, RuntimeEvent::StakingScore(Event::ScoreTrackingStarted { .. }))
		}));
	});
}

#[test]
fn start_tracking_works_with_only_asset_hub_stake() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		// Only Asset Hub stake, no Relay Chain stake
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::AssetHub,
			500 * UNITS,
			3,
			0
		));

		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);
	});
}

// ============================================================================
// receive_staking_details Tests
// ============================================================================

#[test]
fn receive_staking_details_rejects_non_noter() {
	ExtBuilder.build_and_execute(|| {
		// Regular user (not noter) cannot submit staking details.
		assert_noop!(
			StakingScore::receive_staking_details(
				RuntimeOrigin::signed(USER_STASH),
				USER_STASH,
				StakingSource::RelayChain,
				100 * UNITS,
				0,
				0
			),
			Error::<Test>::NotAuthorized
		);
	});
}

#[test]
fn receive_staking_details_emits_event() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::AssetHub,
			500 * UNITS,
			2,
			1
		));

		let events = System::events();
		assert!(events.iter().any(|event| {
			matches!(event.event, RuntimeEvent::StakingScore(Event::StakingDetailsReceived { .. }))
		}));
	});
}

#[test]
fn receive_staking_details_overwrites_same_source() {
	ExtBuilder.build_and_execute(|| {
		// First: 100 HEZ from Relay
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			100 * UNITS,
			0,
			0
		));
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 20);

		// Update same source to 300 HEZ
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			300 * UNITS,
			0,
			0
		));
		// 300 HEZ is in 250-750 tier = 40 points
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);
	});
}

// ============================================================================
// Dual-Source Aggregation Tests (NEW)
// ============================================================================

#[test]
fn relay_and_asset_hub_stake_aggregated() {
	ExtBuilder.build_and_execute(|| {
		// Relay Chain: 200 HEZ
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));

		// Asset Hub: 300 HEZ
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::AssetHub,
			300 * UNITS,
			1,
			0
		));

		// Total: 500 HEZ -> 250-750 tier -> 40 points
		let (score, _) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(score, 40);
	});
}

#[test]
fn single_source_update_changes_aggregate() {
	ExtBuilder.build_and_execute(|| {
		// Relay: 100 HEZ -> <=100 tier -> 20 points
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			100 * UNITS,
			0,
			0
		));
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 20);

		// Add Asset Hub: 60 HEZ -> total 160 HEZ -> 101-250 tier -> 30 points
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::AssetHub,
			60 * UNITS,
			0,
			0
		));
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 30);
	});
}

#[test]
fn dual_source_with_duration_multiplier() {
	ExtBuilder.build_and_execute(|| {
		let base_block = 100u64;
		System::set_block_number(base_block);

		// Relay: 200 HEZ + Asset Hub: 300 HEZ = 500 HEZ -> 40 base
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::AssetHub,
			300 * UNITS,
			1,
			0
		));

		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);

		// After 6 months: 40 * 1.7 = 68
		System::set_block_number(base_block + (6 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 68);
	});
}

// ============================================================================
// Multiple Users and Edge Cases
// ============================================================================

#[test]
fn multiple_users_independent_scores() {
	ExtBuilder.build_and_execute(|| {
		let user1 = USER_STASH;
		let user2 = 20;

		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			user1,
			StakingSource::RelayChain,
			100 * UNITS,
			0,
			0
		));

		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			user2,
			StakingSource::AssetHub,
			500 * UNITS,
			2,
			0
		));

		// User2 starts tracking
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(user2)));

		assert_eq!(StakingScore::get_staking_score(&user1).0, 20);
		assert_eq!(StakingScore::get_staking_score(&user2).0, 40);

		// Advance time
		System::set_block_number((3 * MONTH_IN_BLOCKS) as u64);

		// User1 unchanged (no tracking)
		assert_eq!(StakingScore::get_staking_score(&user1).0, 20);

		// User2 increased (40 * 1.4 = 56)
		assert_eq!(StakingScore::get_staking_score(&user2).0, 56);
	});
}

#[test]
fn duration_returned_correctly() {
	ExtBuilder.build_and_execute(|| {
		let start_block = 100u64;
		System::set_block_number(start_block);

		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			100 * UNITS,
			0,
			0
		));

		// Without tracking, duration should be 0
		let (_, duration) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(duration, 0);

		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		// After 5 months
		let target_block = start_block + (5 * MONTH_IN_BLOCKS) as u64;
		System::set_block_number(target_block);

		let (_, duration) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(duration, target_block - start_block);
	});
}

// ============================================================================
// Noter Authorization Tests
// ============================================================================

const NOTER: AccountId = 99; // MockNoterChecker recognizes 99 as noter

#[test]
fn noter_can_submit_staking_details() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));

		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 30);
	});
}

#[test]
fn root_can_still_submit_staking_details() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));

		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 30);
	});
}

// ============================================================================
// Zero-Stake Cleanup Tests
// ============================================================================

#[test]
fn zero_stake_removes_cached_entry() {
	ExtBuilder.build_and_execute(|| {
		// Setup: noter submits 200 HEZ for relay chain
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));
		assert!(CachedStakingDetails::<Test>::get(USER_STASH, StakingSource::RelayChain).is_some());

		// Zero stake removes the entry
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			0u128,
			0,
			0
		));
		assert!(CachedStakingDetails::<Test>::get(USER_STASH, StakingSource::RelayChain).is_none());
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 0);
	});
}

#[test]
fn zero_stake_cleans_up_tracking_when_no_stake_remains() {
	ExtBuilder.build_and_execute(|| {
		// User opts in
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));
		assert!(StakingStartBlock::<Test>::get(USER_STASH).is_some());

		// Noter submits stake
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));

		// Zero out the only source → StakingStartBlock should be cleaned up
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			0u128,
			0,
			0
		));
		assert!(StakingStartBlock::<Test>::get(USER_STASH).is_none());
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 0);
	});
}

#[test]
fn zero_stake_one_source_keeps_tracking_if_other_source_has_stake() {
	ExtBuilder.build_and_execute(|| {
		// User opts in
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		// Noter submits stake for both sources
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			100 * UNITS,
			0,
			0
		));
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::AssetHub,
			150 * UNITS,
			0,
			0
		));
		// Total 250 HEZ → tier 30
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 30);

		// Zero out relay chain only
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			0u128,
			0,
			0
		));

		// Tracking preserved (AssetHub still has stake)
		assert!(StakingStartBlock::<Test>::get(USER_STASH).is_some());
		// 150 HEZ → tier 30
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 30);
	});
}

// ============================================================================
// Full Workflow Simulation (Plan Scenarios)
// ============================================================================

#[test]
fn full_workflow_ali_scenario() {
	ExtBuilder.build_and_execute(|| {
		let ali = USER_STASH;

		// 1. Ali opts in at block 1000
		System::set_block_number(1000);
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(ali)));
		assert_eq!(StakingScore::get_staking_score(&ali).0, 0); // No data yet

		// 2. Bot + noter submit Ali's 200 HEZ relay stake
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			ali,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));
		// 200 HEZ → tier 30, duration < 1 month → x1.0 → 30
		assert_eq!(StakingScore::get_staking_score(&ali).0, 30);

		// 3. After 28 days → still < 1 month → x1.0 → 30
		System::set_block_number(1000 + 28 * 24 * 60 * 10);
		assert_eq!(StakingScore::get_staking_score(&ali).0, 30);
	});
}

#[test]
fn full_workflow_bob_unbond_scenario() {
	ExtBuilder.build_and_execute(|| {
		let bob = 20;

		// Bob opts in and has stake
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(bob)));
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			bob,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));
		assert_eq!(StakingScore::get_staking_score(&bob).0, 30);

		// Bob unbonds → noter reports zero
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			bob,
			StakingSource::RelayChain,
			0u128,
			0,
			0
		));

		// Score = 0, tracking cleaned up
		assert_eq!(StakingScore::get_staking_score(&bob).0, 0);
		assert!(StakingStartBlock::<Test>::get(bob).is_none());
	});
}

#[test]
fn full_workflow_charlie_dual_chain_partial_unbond() {
	ExtBuilder.build_and_execute(|| {
		let charlie = 30;
		System::set_block_number(500);

		// Charlie opts in
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(charlie)));

		// Noter submits both sources: 100 relay + 150 asset hub = 250 HEZ
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			charlie,
			StakingSource::RelayChain,
			100 * UNITS,
			0,
			0
		));
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			charlie,
			StakingSource::AssetHub,
			150 * UNITS,
			0,
			0
		));
		// 250 HEZ → tier 30
		assert_eq!(StakingScore::get_staking_score(&charlie).0, 30);

		// Charlie unbonds from Relay Chain
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			charlie,
			StakingSource::RelayChain,
			0u128,
			0,
			0
		));

		// Relay entry removed, Asset Hub remains
		assert!(CachedStakingDetails::<Test>::get(charlie, StakingSource::RelayChain).is_none());
		assert!(CachedStakingDetails::<Test>::get(charlie, StakingSource::AssetHub).is_some());
		// Tracking preserved
		assert!(StakingStartBlock::<Test>::get(charlie).is_some());
		// 150 HEZ → tier 30 (still in 101-250 range)
		assert_eq!(StakingScore::get_staking_score(&charlie).0, 30);

		// After 3 months: 30 * 1.4 = 42
		System::set_block_number(500 + (3 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&charlie).0, 42);

		// Charlie unbonds from Asset Hub too → fully zeroed
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			charlie,
			StakingSource::AssetHub,
			0u128,
			0,
			0
		));
		assert!(StakingStartBlock::<Test>::get(charlie).is_none());
		assert_eq!(StakingScore::get_staking_score(&charlie).0, 0);
	});
}

// ============================================================================
// E2E: Duration Counts from Opt-in, Not from Data Arrival
// ============================================================================

#[test]
fn duration_counts_from_optin_not_from_data_arrival() {
	ExtBuilder.build_and_execute(|| {
		// Block 100: User opts in (no data yet)
		System::set_block_number(100);
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));
		assert_eq!(StakingScore::get_staking_score(&USER_STASH), (0, 0u64));

		// Block 50_000: Bot + noter submit data (much later)
		System::set_block_number(50_000);
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));

		// Duration is from block 100, NOT from block 50_000.
		// 50_000 - 100 = 49_900 blocks. MONTH_IN_BLOCKS = 432_000
		// 49_900 < 432_000 → x1.0 → 30
		let (score, duration) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(duration, 49_900);
		assert_eq!(score, 30);

		// After reaching 1 month from opt-in: 100 + 432_000 = 432_100
		System::set_block_number(100 + MONTH_IN_BLOCKS as u64);
		let (score, _) = StakingScore::get_staking_score(&USER_STASH);
		// 30 * 1.2 = 36
		assert_eq!(score, 36);
	});
}

// ============================================================================
// E2E: Re-opt-in After Full Unbond
// ============================================================================

#[test]
fn re_optin_after_full_unbond() {
	ExtBuilder.build_and_execute(|| {
		// Phase 1: opt-in + stake + score
		System::set_block_number(100);
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 30);

		// Phase 2: full unbond → cleanup
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			0u128,
			0,
			0
		));
		assert!(StakingStartBlock::<Test>::get(USER_STASH).is_none());

		// Phase 3: re-opt-in at block 1000 (fresh start)
		System::set_block_number(1000);
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));
		assert_eq!(StakingStartBlock::<Test>::get(USER_STASH), Some(1000));

		// Phase 4: new stake data
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			500 * UNITS,
			0,
			0
		));
		// 500 HEZ → tier 40, duration from block 1000
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);

		// After 6 months from re-opt-in: 40 * 1.7 = 68
		System::set_block_number(1000 + (6 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 68);
	});
}

// ============================================================================
// E2E: Noter Batch (Multiple Users in Sequence)
// ============================================================================

#[test]
fn noter_batch_multiple_users() {
	ExtBuilder.build_and_execute(|| {
		let user1: AccountId = 10;
		let user2: AccountId = 20;
		let user3: AccountId = 30;

		// All users opt in
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(user1)));
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(user2)));
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(user3)));

		// Noter submits batch: different amounts for each user
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			user1,
			StakingSource::RelayChain,
			50 * UNITS, // tier 20
			0,
			0
		));
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			user2,
			StakingSource::RelayChain,
			200 * UNITS, // tier 30
			0,
			0
		));
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			user3,
			StakingSource::AssetHub,
			800 * UNITS, // tier 50
			0,
			0
		));

		assert_eq!(StakingScore::get_staking_score(&user1).0, 20);
		assert_eq!(StakingScore::get_staking_score(&user2).0, 30);
		assert_eq!(StakingScore::get_staking_score(&user3).0, 50);
	});
}

// ============================================================================
// E2E: Data Submitted Without Opt-in (No StakingStartBlock)
// ============================================================================

#[test]
fn data_without_optin_still_cached_but_no_duration() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(100);

		// Noter submits data for user who hasn't opted in
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			300 * UNITS,
			0,
			0
		));

		// Data is cached → score is base tier only (no duration)
		assert!(CachedStakingDetails::<Test>::get(USER_STASH, StakingSource::RelayChain).is_some());
		assert!(StakingStartBlock::<Test>::get(USER_STASH).is_none());
		// 300 HEZ → tier 40, no duration multiplier
		let (score, duration) = StakingScore::get_staking_score(&USER_STASH);
		assert_eq!(score, 40);
		assert_eq!(duration, 0);

		// Time passes without opt-in → no duration benefit
		System::set_block_number(100 + (12 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);

		// User finally opts in → duration starts NOW
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));
		assert_eq!(
			StakingStartBlock::<Test>::get(USER_STASH),
			Some(100 + (12 * MONTH_IN_BLOCKS) as u64)
		);
		// Still x1.0 because just started
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);
	});
}

// ============================================================================
// Edge Cases: Exact Tier Boundaries
// ============================================================================

#[test]
fn tier_boundary_101_hez() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			101 * UNITS,
			0,
			0
		));
		// 101 > 100 → tier 30
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 30);
	});
}

#[test]
fn tier_boundary_251_hez() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			251 * UNITS,
			0,
			0
		));
		// 251 > 250 → tier 40
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);
	});
}

#[test]
fn tier_boundary_751_hez() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			751 * UNITS,
			0,
			0
		));
		// 751 > 750 → tier 50
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 50);
	});
}

#[test]
fn sub_unit_stake_rounds_to_zero() {
	ExtBuilder.build_and_execute(|| {
		// Less than 1 HEZ (sub-UNITS)
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			UNITS / 2, // 0.5 HEZ
			0,
			0
		));
		// staked_hez = 0.5 / 1 = 0 (integer division) → score 0
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 0);
	});
}

#[test]
fn exactly_one_hez_returns_tier_20() {
	ExtBuilder.build_and_execute(|| {
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			UNITS, // 1 HEZ
			0,
			0
		));
		// 1 HEZ ≤ 100 → tier 20
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 20);
	});
}

// ============================================================================
// Edge Cases: Unsigned Origin Rejection
// ============================================================================

#[test]
fn unsigned_origin_rejected_for_start_tracking() {
	ExtBuilder.build_and_execute(|| {
		assert_noop!(
			StakingScore::start_score_tracking(RuntimeOrigin::none()),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn unsigned_origin_rejected_for_receive_details() {
	ExtBuilder.build_and_execute(|| {
		assert_noop!(
			StakingScore::receive_staking_details(
				RuntimeOrigin::none(),
				USER_STASH,
				StakingSource::RelayChain,
				100 * UNITS,
				0,
				0
			),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

// ============================================================================
// Edge Cases: Zero-Stake Events
// ============================================================================

#[test]
fn zero_stake_emits_event_with_zero_amount() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		// Setup
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			200 * UNITS,
			0,
			0
		));

		// Zero-stake
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			0u128,
			0,
			0
		));

		let events = System::events();
		// Last event should be StakingDetailsReceived with amount 0
		let last_staking_event = events
			.iter()
			.rev()
			.find(|e| {
				matches!(e.event, RuntimeEvent::StakingScore(Event::StakingDetailsReceived { .. }))
			})
			.expect("should have StakingDetailsReceived event");

		match &last_staking_event.event {
			RuntimeEvent::StakingScore(Event::StakingDetailsReceived { staked_amount, .. }) => {
				assert_eq!(*staked_amount, 0u128)
			},
			_ => panic!("wrong event type"),
		}
	});
}

// ============================================================================
// Edge Cases: Noter Overwrites Previous Noter Submission
// ============================================================================

#[test]
fn noter_overwrites_previous_submission() {
	ExtBuilder.build_and_execute(|| {
		// First submission: 100 HEZ
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			100 * UNITS,
			5,
			2
		));
		let details =
			CachedStakingDetails::<Test>::get(USER_STASH, StakingSource::RelayChain).unwrap();
		assert_eq!(details.staked_amount, 100 * UNITS);
		assert_eq!(details.nominations_count, 5);
		assert_eq!(details.unlocking_chunks_count, 2);

		// Second submission: updated values
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			300 * UNITS,
			10,
			1
		));
		let details =
			CachedStakingDetails::<Test>::get(USER_STASH, StakingSource::RelayChain).unwrap();
		assert_eq!(details.staked_amount, 300 * UNITS);
		assert_eq!(details.nominations_count, 10);
		assert_eq!(details.unlocking_chunks_count, 1);
	});
}

// ============================================================================
// Storage Integrity: Zero-stake Does Not Create Ghost Entries
// ============================================================================

#[test]
fn zero_stake_for_nonexistent_source_is_noop() {
	ExtBuilder.build_and_execute(|| {
		// Zero-stake for a source that was never set — should be a no-op
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			0u128,
			0,
			0
		));

		// No ghost entries
		assert!(CachedStakingDetails::<Test>::get(USER_STASH, StakingSource::RelayChain).is_none());
		assert!(StakingStartBlock::<Test>::get(USER_STASH).is_none());
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 0);
	});
}

// ============================================================================
// Max Score Scenario: Highest Tier + Max Duration
// ============================================================================

#[test]
fn max_score_scenario() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(1);

		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));

		// Dual-chain max: 500 relay + 500 asset hub = 1000 HEZ → tier 50
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::RelayChain,
			500 * UNITS,
			0,
			0
		));
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::signed(NOTER),
			USER_STASH,
			StakingSource::AssetHub,
			500 * UNITS,
			0,
			0
		));
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 50);

		// 12+ months → 50 * 2.0 = 100 (capped)
		System::set_block_number(1 + (12 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 100);

		// 24 months → still 100 (cap)
		System::set_block_number(1 + (24 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 100);
	});
}

// ============================================================================
// Duration Boundary: Exact Month Boundaries
// ============================================================================

#[test]
fn duration_exact_month_boundaries() {
	ExtBuilder.build_and_execute(|| {
		System::set_block_number(0);
		assert_ok!(StakingScore::start_score_tracking(RuntimeOrigin::signed(USER_STASH)));
		assert_ok!(StakingScore::receive_staking_details(
			RuntimeOrigin::root(),
			USER_STASH,
			StakingSource::RelayChain,
			100 * UNITS, // tier 20
			0,
			0
		));

		// Exactly 1 month - 1 block: still x1.0
		System::set_block_number(MONTH_IN_BLOCKS as u64 - 1);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 20);

		// Exactly 1 month: x1.2 → 20 * 1.2 = 24
		System::set_block_number(MONTH_IN_BLOCKS as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 24);

		// Exactly 3 months - 1: still x1.2
		System::set_block_number((3 * MONTH_IN_BLOCKS) as u64 - 1);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 24);

		// Exactly 3 months: x1.4 → 20 * 1.4 = 28
		System::set_block_number((3 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 28);

		// Exactly 6 months: x1.7 → 20 * 1.7 = 34
		System::set_block_number((6 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 34);

		// Exactly 12 months: x2.0 → 20 * 2.0 = 40
		System::set_block_number((12 * MONTH_IN_BLOCKS) as u64);
		assert_eq!(StakingScore::get_staking_score(&USER_STASH).0, 40);
	});
}
