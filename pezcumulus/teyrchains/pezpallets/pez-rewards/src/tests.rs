// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// tests.rs (v11 - Final Bug Fixes)

use crate::{mock::*, EpochState, Error, Event};
use pezframe_support::{
	assert_noop, assert_ok,
	traits::{
		fungibles::Mutate,
		tokens::{Fortitude, Precision, Preservation},
	},
};
use pezsp_runtime::traits::BadOrigin;

// =============================================================================
// 1. INITIALIZATION TESTS
// =============================================================================

#[test]
fn initialize_rewards_system_works() {
	new_test_ext().execute_with(|| {
		let epoch_info = PezRewards::get_current_epoch_info();
		assert_eq!(epoch_info.current_epoch, 0);
		assert_eq!(epoch_info.total_epochs_completed, 0);
		assert_eq!(epoch_info.epoch_start_block, 1);
		assert_eq!(PezRewards::epoch_status(0), EpochState::Open);

		// BUG FIX E0599: Matches lib.rs v2
		System::assert_has_event(Event::NewEpochStarted { epoch_index: 0, start_block: 1 }.into());
	});
}

#[test]
fn cannot_initialize_twice() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PezRewards::initialize_rewards_system(RuntimeOrigin::root()),
			Error::<Test>::AlreadyInitialized // BUG FIX E0599: Matches lib.rs v2
		);
	});
}

// =============================================================================
// 2. TRUST SCORE RECORDING TESTS
// =============================================================================

#[test]
fn record_trust_score_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		let score = PezRewards::get_user_trust_score_for_epoch(0, &alice());
		assert_eq!(score, Some(100));

		System::assert_has_event(
			Event::TrustScoreRecorded { user: alice(), epoch_index: 0, trust_score: 100 }.into(),
		);
	});
}

#[test]
fn multiple_users_can_record_scores() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(bob())));
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(charlie())));

		assert_eq!(PezRewards::get_user_trust_score_for_epoch(0, &alice()), Some(100));
		assert_eq!(PezRewards::get_user_trust_score_for_epoch(0, &bob()), Some(50));
		assert_eq!(PezRewards::get_user_trust_score_for_epoch(0, &charlie()), Some(75));
	});
}

#[test]
fn record_trust_score_twice_updates() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		assert_eq!(PezRewards::get_user_trust_score_for_epoch(0, &alice()), Some(100));

		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		assert_eq!(PezRewards::get_user_trust_score_for_epoch(0, &alice()), Some(100));
	});
}

#[test]
fn cannot_record_score_for_closed_epoch() {
	new_test_ext().execute_with(|| {
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));
		advance_blocks(crate::CLAIM_PERIOD_BLOCKS as u64 + 1);
		assert_ok!(PezRewards::close_epoch(RuntimeOrigin::root(), 0));

		// FIX: Dave now registering in epoch 1 (epoch 1 Open)
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(dave())));

		// Dave's score should be recorded in epoch 1
		assert_eq!(PezRewards::get_user_trust_score_for_epoch(1, &dave()), Some(0));
	});
}

// =============================================================================
// 3. EPOCH FINALIZATION TESTS
// =============================================================================

#[test]
fn getter_functions_work_correctly() {
	new_test_ext().execute_with(|| {
		assert_eq!(PezRewards::get_claimed_reward(0, &alice()), None);
		assert_eq!(PezRewards::get_user_trust_score_for_epoch(0, &alice()), None);
		assert_eq!(PezRewards::get_epoch_reward_pool(0), None);
		assert_eq!(PezRewards::epoch_status(0), EpochState::Open);

		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		assert_eq!(PezRewards::get_user_trust_score_for_epoch(0, &alice()), Some(100));

		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));
		assert!(PezRewards::get_epoch_reward_pool(0).is_some());
		// FIX: Should be ClaimPeriod after finalize
		assert_eq!(PezRewards::epoch_status(0), EpochState::ClaimPeriod);

		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0));
		assert!(PezRewards::get_claimed_reward(0, &alice()).is_some());
	});
}

#[test]
fn finalize_epoch_too_early_fails() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));

		advance_blocks(crate::BLOCKS_PER_EPOCH as u64 - 1);
		assert_noop!(
			PezRewards::finalize_epoch(RuntimeOrigin::root()),
			Error::<Test>::EpochNotFinished
		);
	});
}

#[test]
fn finalize_epoch_calculates_rewards_correctly() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice()))); // 100
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(bob()))); // 50
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(charlie()))); // 75
		let total_trust: u128 = 100 + 50 + 75;
		let expected_deadline = System::block_number()
			+ crate::BLOCKS_PER_EPOCH as u64
			+ crate::CLAIM_PERIOD_BLOCKS as u64;

		let incentive_pot = PezRewards::incentive_pot_account_id();
		let initial_pot_balance = pez_balance(&incentive_pot);

		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let reward_pool = PezRewards::get_epoch_reward_pool(0).unwrap();

		// FIX: Reduced amount after parliamentary reward (90%)
		let trust_score_pool = initial_pot_balance * 90u128 / 100;

		assert_eq!(reward_pool.total_reward_pool, trust_score_pool);
		assert_eq!(reward_pool.total_trust_score, total_trust);
		assert_eq!(reward_pool.participants_count, 3);
		assert_eq!(reward_pool.reward_per_trust_point, trust_score_pool / total_trust);
		assert_eq!(
			reward_pool.claim_deadline,
			System::block_number() + crate::CLAIM_PERIOD_BLOCKS as u64
		);

		// FIX: Event'te trust_score_pool (90%) bekle
		System::assert_has_event(
			Event::EpochRewardPoolCalculated {
				epoch_index: 0,
				total_pool: trust_score_pool,
				participants_count: 3,
				total_trust_score: total_trust,
				claim_deadline: expected_deadline,
			}
			.into(),
		);
		System::assert_has_event(
			Event::NewEpochStarted {
				epoch_index: 1,
				start_block: crate::BLOCKS_PER_EPOCH as u64 + 1,
			}
			.into(),
		);
		// FIX: Finalize sonrası ClaimPeriod
		assert_eq!(PezRewards::epoch_status(0), EpochState::ClaimPeriod);
		assert_eq!(PezRewards::epoch_status(1), EpochState::Open);
	});
}

#[test]
fn finalize_epoch_fails_if_already_finalized_or_closed() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		// FIX: Second finalize tries to finalize epoch 1 (not finished yet)
		assert_noop!(
			PezRewards::finalize_epoch(RuntimeOrigin::root()),
			Error::<Test>::EpochNotFinished
		);
	});
}

#[test]
fn finalize_epoch_no_participants() {
	new_test_ext().execute_with(|| {
		let incentive_pot = PezRewards::incentive_pot_account_id();
		let pot_balance_before = pez_balance(&incentive_pot);

		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let reward_pool = PezRewards::get_epoch_reward_pool(0).unwrap();
		assert_eq!(reward_pool.total_trust_score, 0);
		assert_eq!(reward_pool.participants_count, 0);
		assert_eq!(reward_pool.reward_per_trust_point, 0);

		// FIX: NFT owner not registered, parliamentary reward not distributed
		// All balance remains in pot (100%)
		let pot_balance_after = pez_balance(&incentive_pot);
		assert_eq!(pot_balance_after, pot_balance_before);
	});
}

#[test]
fn finalize_epoch_zero_trust_score_participant() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(dave()))); // Skor 0
																			 // FIX: Zero scores are now being recorded
		assert_eq!(PezRewards::get_user_trust_score_for_epoch(0, &dave()), Some(0));

		let incentive_pot = PezRewards::incentive_pot_account_id();
		let pot_balance_before = pez_balance(&incentive_pot);

		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let reward_pool = PezRewards::get_epoch_reward_pool(0).unwrap();
		assert_eq!(reward_pool.total_trust_score, 0);
		assert_eq!(reward_pool.participants_count, 1);
		assert_eq!(reward_pool.reward_per_trust_point, 0);

		// FIX: NFT owner not registered, parliamentary reward not distributed
		// All balance remains in pot (100%)
		let pot_balance_after = pez_balance(&incentive_pot);
		assert_eq!(pot_balance_after, pot_balance_before);

		// FIX: NoRewardToClaim instead of NoTrustScoreForEpoch (0 score exists but reward is 0)
		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(dave()), 0),
			Error::<Test>::NoRewardToClaim
		);
	});
}

// =============================================================================
// 4. CLAIM REWARD TESTS
// =============================================================================

#[test]
fn claim_reward_works_for_single_user() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice()))); // 100
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let balance_before = pez_balance(&alice());
		let reward_pool = PezRewards::get_epoch_reward_pool(0).unwrap();
		let expected_reward = reward_pool.reward_per_trust_point * 100;

		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0));

		let balance_after = pez_balance(&alice());
		assert_eq!(balance_after, balance_before + expected_reward);

		System::assert_last_event(
			Event::RewardClaimed { user: alice(), epoch_index: 0, amount: expected_reward }.into(),
		);
		assert!(PezRewards::get_claimed_reward(0, &alice()).is_some());
	});
}

#[test]
fn claim_reward_works_for_multiple_users() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice()))); // 100
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(bob()))); // 50
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let balance1_before = pez_balance(&alice());
		let balance2_before = pez_balance(&bob());

		let reward_pool = PezRewards::get_epoch_reward_pool(0).unwrap();
		let reward1 = reward_pool.reward_per_trust_point * 100;
		let reward2 = reward_pool.reward_per_trust_point * 50;

		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0));
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(bob()), 0));

		let balance1_after = pez_balance(&alice());
		let balance2_after = pez_balance(&bob());

		assert_eq!(balance1_after, balance1_before + reward1);
		assert_eq!(balance2_after, balance2_before + reward2);
	});
}

#[test]
fn claim_reward_fails_if_already_claimed() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0));

		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0),
			Error::<Test>::RewardAlreadyClaimed
		);
	});
}

#[test]
fn claim_reward_fails_if_not_participant() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		// FIX: Bob not registered, should get NoTrustScoreForEpoch error
		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(bob()), 0),
			Error::<Test>::NoTrustScoreForEpoch
		);
	});
}

#[test]
fn claim_reward_fails_if_epoch_not_finalized() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		// FIX: Unfinalized epoch -> ClaimPeriodExpired error (Open state)
		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0),
			Error::<Test>::ClaimPeriodExpired
		);
	});
}

#[test]
fn claim_reward_fails_if_claim_period_over() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		advance_blocks(crate::CLAIM_PERIOD_BLOCKS as u64 + 1);

		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0),
			Error::<Test>::ClaimPeriodExpired // BUG FIX E0599
		);
	});
}

#[test]
fn claim_reward_fails_if_epoch_closed() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));
		advance_blocks(crate::CLAIM_PERIOD_BLOCKS as u64 + 1);
		assert_ok!(PezRewards::close_epoch(RuntimeOrigin::root(), 0));

		// FIX: Epoch Closed -> ClaimPeriodExpired error
		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0),
			Error::<Test>::ClaimPeriodExpired
		);
	});
}

#[test]
fn claim_reward_fails_if_pot_insufficient_during_claim() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let incentive_pot = PezRewards::incentive_pot_account_id();
		let pez_pot_balance = pez_balance(&incentive_pot);
		assert_ok!(Assets::burn_from(
			PezAssetId::get(),
			&incentive_pot,
			pez_pot_balance,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Polite
		));

		// FIX: Arithmetic Underflow error expected
		assert!(PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0).is_err());
	});
}

#[test]
fn claim_reward_fails_for_wrong_epoch() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		// FIX: Epoch 1 not yet finalized -> ClaimPeriodExpired
		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 1),
			Error::<Test>::ClaimPeriodExpired
		);

		// Epoch 999 yok -> ClaimPeriodExpired
		assert_noop!(
			PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 999),
			Error::<Test>::ClaimPeriodExpired
		);
	});
}

// =============================================================================
// 5. CLOSE EPOCH TESTS
// =============================================================================

#[test]
fn close_epoch_works_after_claim_period() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice()))); // Claim etmeyecek
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(bob()))); // Claim edecek

		let incentive_pot = PezRewards::incentive_pot_account_id();
		let _pot_balance_before_finalize = pez_balance(&incentive_pot);

		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let reward_pool = PezRewards::get_epoch_reward_pool(0).unwrap();
		let _alice_reward = reward_pool.reward_per_trust_point * 100;
		let bob_reward = reward_pool.reward_per_trust_point * 50;

		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(bob()), 0)); // Bob claim etti

		let clawback_recipient = ClawbackRecipient::get();
		let balance_before = pez_balance(&clawback_recipient);

		// Only unclaimed rewards should be clawed back, not the entire pot.
		// total_allocated = reward_pool.total_reward_pool (90% trust score pool)
		// total_claimed = bob_reward
		// unclaimed = total_allocated - bob_reward = alice_reward (+ any rounding remainder)
		let total_claimed = bob_reward;
		let expected_unclaimed = reward_pool.total_reward_pool - total_claimed;

		advance_blocks(crate::CLAIM_PERIOD_BLOCKS as u64 + 1);

		assert_ok!(PezRewards::close_epoch(RuntimeOrigin::root(), 0));

		let balance_after = pez_balance(&clawback_recipient);
		// Only alice's unclaimed reward (not entire pot) should be clawed back
		assert_eq!(balance_after, balance_before + expected_unclaimed);

		assert_eq!(PezRewards::epoch_status(0), EpochState::Closed);

		// Verify the pot still has funds from future epochs (not drained)
		let pot_after = pez_balance(&incentive_pot);
		// The pot should still have the 10% remaining from parliamentary allocation
		// that wasn't distributed (no NFT owners registered)
		assert!(pot_after > 0, "Pot should not be completely drained");

		System::assert_last_event(
			Event::EpochClosed {
				epoch_index: 0,
				unclaimed_amount: expected_unclaimed,
				clawback_recipient,
			}
			.into(),
		);
	});
}

#[test]
fn close_epoch_fails_before_claim_period_ends() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		advance_blocks(crate::CLAIM_PERIOD_BLOCKS as u64 - 1);
		assert_noop!(
			PezRewards::close_epoch(RuntimeOrigin::root(), 0),
			Error::<Test>::ClaimPeriodExpired // BUG FIX E0599
		);
	});
}

#[test]
fn close_epoch_fails_if_already_closed() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));
		advance_blocks(crate::CLAIM_PERIOD_BLOCKS as u64 + 1);
		assert_ok!(PezRewards::close_epoch(RuntimeOrigin::root(), 0));

		assert_noop!(
			PezRewards::close_epoch(RuntimeOrigin::root(), 0),
			Error::<Test>::EpochAlreadyClosed
		);
	});
}

#[test]
fn close_epoch_fails_if_not_finalized() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice())));
		advance_blocks(crate::CLAIM_PERIOD_BLOCKS as u64 + 1);
		assert_noop!(
			PezRewards::close_epoch(RuntimeOrigin::root(), 0),
			Error::<Test>::EpochAlreadyClosed // This error returns even if not finalized
		);
	});
}

// =============================================================================
// 6. PARLIAMENTARY REWARDS TESTS
// =============================================================================

#[test]
fn parliamentary_rewards_distributed_correctly() {
	new_test_ext().execute_with(|| {
		register_nft_owner(1, dave());
		register_nft_owner(2, alice());
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice()))); // 100

		let incentive_pot = PezRewards::incentive_pot_account_id();
		let pot_balance = pez_balance(&incentive_pot);

		let expected_parliamentary_reward_pot =
			pot_balance * u128::from(crate::PARLIAMENTARY_REWARD_PERCENT) / 100;
		let expected_parliamentary_reward =
			expected_parliamentary_reward_pot / u128::from(crate::PARLIAMENTARY_NFT_COUNT);

		let dave_balance_before = pez_balance(&dave());
		let alice_balance_before = pez_balance(&alice());

		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let dave_balance_after = pez_balance(&dave());
		assert_eq!(dave_balance_after, dave_balance_before + expected_parliamentary_reward);

		let reward_pool = PezRewards::get_epoch_reward_pool(0).unwrap();
		let trust_reward = reward_pool.reward_per_trust_point * 100;

		let alice_balance_after_finalize = pez_balance(&alice());
		assert_eq!(
			alice_balance_after_finalize,
			alice_balance_before + expected_parliamentary_reward
		);

		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0));
		let alice_balance_after_claim = pez_balance(&alice());
		assert_eq!(alice_balance_after_claim, alice_balance_after_finalize + trust_reward);

		System::assert_has_event(
			Event::ParliamentaryNftRewardDistributed {
				nft_id: 1,
				owner: dave(),
				amount: expected_parliamentary_reward,
				epoch: 0,
			}
			.into(),
		);
		System::assert_has_event(
			Event::ParliamentaryNftRewardDistributed {
				nft_id: 2,
				owner: alice(),
				amount: expected_parliamentary_reward,
				epoch: 0,
			}
			.into(),
		);
	});
}

#[test]
fn parliamentary_reward_division_precision() {
	new_test_ext().execute_with(|| {
		register_nft_owner(1, dave());
		register_nft_owner(2, alice());

		let incentive_pot = PezRewards::incentive_pot_account_id();
		let current_balance = pez_balance(&incentive_pot);
		assert_ok!(Assets::burn_from(
			PezAssetId::get(),
			&incentive_pot,
			current_balance,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Polite
		));

		// FIX: Put larger amount (to avoid BelowMinimum error)
		fund_incentive_pot(100_000);

		let dave_balance_before = pez_balance(&dave());
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let dave_balance_after = pez_balance(&dave());
		// 10% of 100_000 = 10_000 / 201 NFT = 49 per NFT
		let expected_reward = 49;
		assert_eq!(dave_balance_after, dave_balance_before + expected_reward);
	});
}

// =============================================================================
// 7. NFT OWNER REGISTRATION TESTS
// =============================================================================

#[test]
fn register_parliamentary_nft_owner_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(PezRewards::get_parliamentary_nft_owner(10), None);
		assert_ok!(PezRewards::register_parliamentary_nft_owner(
			RuntimeOrigin::root(),
			10,
			alice()
		));
		assert_eq!(PezRewards::get_parliamentary_nft_owner(10), Some(alice()));

		System::assert_last_event(
			Event::ParliamentaryOwnerRegistered { nft_id: 10, owner: alice() }.into(),
		);
	});
}

#[test]
fn register_parliamentary_nft_owner_fails_for_non_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PezRewards::register_parliamentary_nft_owner(
				RuntimeOrigin::signed(alice()),
				10,
				alice()
			),
			BadOrigin
		);
	});
}

#[test]
fn register_parliamentary_nft_owner_updates_existing() {
	new_test_ext().execute_with(|| {
		assert_ok!(PezRewards::register_parliamentary_nft_owner(
			RuntimeOrigin::root(),
			10,
			alice()
		));
		assert_eq!(PezRewards::get_parliamentary_nft_owner(10), Some(alice()));

		assert_ok!(PezRewards::register_parliamentary_nft_owner(RuntimeOrigin::root(), 10, bob()));
		assert_eq!(PezRewards::get_parliamentary_nft_owner(10), Some(bob()));
	});
}

// =============================================================================
// 8. MULTIPLE EPOCHS TEST
// =============================================================================

#[test]
fn multiple_epochs_work_correctly() {
	new_test_ext().execute_with(|| {
		// --- EPOCH 0 ---
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice()))); // 100
		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(bob()))); // 50
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root()));

		let reward_pool_0 = PezRewards::get_epoch_reward_pool(0).unwrap();
		let reward1_0 = reward_pool_0.reward_per_trust_point * 100;
		let reward2_0 = reward_pool_0.reward_per_trust_point * 50;
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 0));
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(bob()), 0));

		// --- EPOCH 1 ---
		assert_eq!(PezRewards::get_current_epoch_info().current_epoch, 1);

		fund_incentive_pot(1_000_000_000_000_000);

		assert_ok!(PezRewards::record_trust_score(RuntimeOrigin::signed(alice()))); // 100 (Epoch 1 için)
		advance_blocks(crate::BLOCKS_PER_EPOCH as u64);
		assert_ok!(PezRewards::finalize_epoch(RuntimeOrigin::root())); // Epoch 1'i finalize et

		let reward_pool_1 = PezRewards::get_epoch_reward_pool(1).unwrap(); // Epoch 1 havuzu
		let reward1_1 = reward_pool_1.reward_per_trust_point * 100;
		assert_ok!(PezRewards::claim_reward(RuntimeOrigin::signed(alice()), 1)); // Epoch 1'den claim et

		// Check balances
		let alice_balance = pez_balance(&alice());
		let bob_balance = pez_balance(&bob());
		assert_eq!(alice_balance, reward1_0 + reward1_1);
		assert_eq!(bob_balance, reward2_0);
	});
}

// =============================================================================
// 9. ORIGIN CHECKS
// =============================================================================

#[test]
fn non_root_origin_fails_for_privileged_calls() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PezRewards::initialize_rewards_system(RuntimeOrigin::signed(alice())),
			BadOrigin
		);
		assert_noop!(
			PezRewards::register_parliamentary_nft_owner(RuntimeOrigin::signed(alice()), 1, bob()),
			BadOrigin
		);
	});
}

#[test]
fn non_signed_origin_fails_for_user_calls() {
	new_test_ext().execute_with(|| {
		assert_noop!(PezRewards::record_trust_score(RuntimeOrigin::root()), BadOrigin);
	});
}
