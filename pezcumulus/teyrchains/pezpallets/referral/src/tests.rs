// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{
	mock::*, Error, Event, PendingReferrals, ReferralCount, Referrals, ReferrerStatsStorage,
};
use pezframe_support::{assert_noop, assert_ok};
use pezpallet_identity_kyc::types::{OnCitizenshipRevoked, OnKycApproved};
use pezsp_runtime::DispatchError;

type ReferralPallet = crate::Pezpallet<Test>;

// ============================================================================
// initiate_referral Tests
// ============================================================================

#[test]
fn initiate_referral_works() {
	new_test_ext().execute_with(|| {
		// REFERRER (citizen) invites REFERRED
		assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(REFERRER), REFERRED));

		// Verification: Correct record is added to pending referrals list.
		assert_eq!(ReferralPallet::pending_referrals(REFERRED), Some(REFERRER));

		// Correct event is emitted.
		System::assert_last_event(
			Event::ReferralInitiated { referrer: REFERRER, referred: REFERRED }.into(),
		);
	});
}

#[test]
fn initiate_referral_fails_for_self_referral() {
	new_test_ext().execute_with(|| {
		// User cannot invite themselves.
		assert_noop!(
			ReferralPallet::initiate_referral(RuntimeOrigin::signed(REFERRER), REFERRER),
			Error::<Test>::SelfReferral
		);
	});
}

#[test]
fn initiate_referral_fails_if_already_referred() {
	new_test_ext().execute_with(|| {
		// First referral succeeds
		assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(REFERRER), REFERRED));

		// Second referral attempt by USER_3 fails
		assert_noop!(
			ReferralPallet::initiate_referral(RuntimeOrigin::signed(USER_3), REFERRED),
			Error::<Test>::AlreadyReferred
		);
	});
}

// ============================================================================
// on_kyc_approved Hook Tests (Updated for new trait signature)
// ============================================================================

#[test]
fn on_kyc_approved_hook_works() {
	new_test_ext().execute_with(|| {
		// Setup: REFERRER invites REFERRED via PendingReferrals
		assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(REFERRER), REFERRED));

		// Set user's KYC as approved
		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			REFERRED,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);

		// Action: Call on_kyc_approved with referrer parameter
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER);

		// Verification
		// 1. Pending referral record is deleted
		assert_eq!(PendingReferrals::<Test>::get(REFERRED), None);
		// 2. Referrer's referral count increases by 1
		assert_eq!(ReferralCount::<Test>::get(REFERRER), 1);
		// 3. Permanent referral information is created
		assert!(Referrals::<Test>::contains_key(REFERRED));
		let referral_info = Referrals::<Test>::get(REFERRED).unwrap();
		assert_eq!(referral_info.referrer, REFERRER);
		// 4. ReferrerStats updated
		let stats = ReferrerStatsStorage::<Test>::get(REFERRER);
		assert_eq!(stats.total_referrals, 1);
		assert_eq!(stats.revoked_referrals, 0);
		// 5. Correct event is emitted
		System::assert_last_event(
			Event::ReferralConfirmed {
				referrer: REFERRER,
				referred: REFERRED,
				new_referrer_count: 1,
			}
			.into(),
		);
	});
}

#[test]
fn on_kyc_approved_uses_referrer_parameter() {
	new_test_ext().execute_with(|| {
		// No pending referral - but referrer is passed as parameter
		// This tests the new model where identity-kyc passes referrer directly

		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			REFERRED,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);

		// Call with explicit referrer parameter
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER);

		// Should use the passed referrer, not look up from PendingReferrals
		let referral_info = Referrals::<Test>::get(REFERRED).unwrap();
		assert_eq!(referral_info.referrer, REFERRER);
		assert_eq!(ReferralCount::<Test>::get(REFERRER), 1);
	});
}

#[test]
fn on_kyc_approved_does_nothing_if_not_approved_status() {
	new_test_ext().execute_with(|| {
		// User's KYC is NOT approved - status is still NotStarted
		// on_kyc_approved should do nothing

		let initial_count = ReferralCount::<Test>::get(REFERRER);
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER);

		// No changes should have occurred
		assert_eq!(ReferralCount::<Test>::get(REFERRER), initial_count);
		assert!(Referrals::<Test>::get(REFERRED).is_none());
	});
}

#[test]
fn on_kyc_approved_prevents_double_counting() {
	new_test_ext().execute_with(|| {
		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			REFERRED,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);

		// First approval
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER);
		assert_eq!(ReferralCount::<Test>::get(REFERRER), 1);

		// Second approval attempt should be ignored (already processed)
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER);
		assert_eq!(ReferralCount::<Test>::get(REFERRER), 1); // Still 1
	});
}

// ============================================================================
// on_citizenship_revoked Tests (Direct Responsibility Penalty)
// ============================================================================

#[test]
fn on_citizenship_revoked_penalizes_referrer() {
	new_test_ext().execute_with(|| {
		// Setup: Complete referral first
		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			REFERRED,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER);

		// Verify initial stats
		let stats = ReferrerStatsStorage::<Test>::get(REFERRER);
		assert_eq!(stats.total_referrals, 1);
		assert_eq!(stats.revoked_referrals, 0);
		assert_eq!(stats.penalty_score, 0);

		// Action: Citizenship revoked (malicious actor identified)
		ReferralPallet::on_citizenship_revoked(&REFERRED);

		// Verify penalty applied
		let stats = ReferrerStatsStorage::<Test>::get(REFERRER);
		assert_eq!(stats.total_referrals, 1);
		assert_eq!(stats.revoked_referrals, 1);
		assert_eq!(stats.penalty_score, PenaltyPerRevocationAmount::get());

		// Verify event
		System::assert_last_event(
			Event::ReferralPenalized {
				referrer: REFERRER,
				revoked_citizen: REFERRED,
				new_penalty_score: PenaltyPerRevocationAmount::get(),
				total_revoked: 1,
			}
			.into(),
		);
	});
}

#[test]
fn on_citizenship_revoked_does_nothing_if_no_referral() {
	new_test_ext().execute_with(|| {
		// Try to revoke someone who was never referred
		let unknown_user = 999;
		ReferralPallet::on_citizenship_revoked(&unknown_user);

		// No penalty events should be emitted
		// (this is safe - just a no-op)
	});
}

// ============================================================================
// Referral Score Calculation Tests (with balanced penalty)
// ============================================================================

#[test]
fn referral_score_tier_0_to_10() {
	use crate::types::ReferralScoreProvider;

	new_test_ext().execute_with(|| {
		// Update stats directly for testing
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 0;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 0);

		// 1 referral = 10 points
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 1;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 10);

		// 5 referrals = 50 points
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 5;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 50);

		// 10 referrals = 100 points
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 10;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 100);
	});
}

#[test]
fn referral_score_tier_11_to_50() {
	use crate::types::ReferralScoreProvider;

	new_test_ext().execute_with(|| {
		// 11 referrals: 100 + (1 * 5) = 105
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 11;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 105);

		// 20 referrals: 100 + (10 * 5) = 150
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 20;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 150);

		// 50 referrals: 100 + (40 * 5) = 300
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 50;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 300);
	});
}

#[test]
fn referral_score_tier_51_to_100() {
	use crate::types::ReferralScoreProvider;

	new_test_ext().execute_with(|| {
		// 51 referrals: 300 + (1 * 4) = 304
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 51;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 304);

		// 75 referrals: 300 + (25 * 4) = 400
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 75;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 400);

		// 100 referrals: 300 + (50 * 4) = 500
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 100;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 500);
	});
}

#[test]
fn referral_score_capped_at_500() {
	use crate::types::ReferralScoreProvider;

	new_test_ext().execute_with(|| {
		// 101+ referrals capped at 500
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 101;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 500);

		// Even 1000 referrals = 500
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 1000;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 500);
	});
}

#[test]
fn referral_score_with_balanced_penalty() {
	use crate::types::ReferralScoreProvider;

	new_test_ext().execute_with(|| {
		// 10 good referrals = 100 points
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 10;
			stats.revoked_referrals = 0;
			stats.penalty_score = 0;
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 100);

		// 10 total, 4 revoked = 6 good
		// penalty_score: 4 * PenaltyPerRevocation(3) = 12
		// Base score: 6 * 10 = 60
		// Final: 60 - 12 = 48
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 10;
			stats.revoked_referrals = 4;
			stats.penalty_score = 4 * PenaltyPerRevocationAmount::get();
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 48);

		// 20 total, 8 revoked = 12 good (tier 2)
		// penalty_score: 8 * PenaltyPerRevocation(3) = 24
		// Base score: 100 + (2 * 5) = 110
		// Final: 110 - 24 = 86
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 20;
			stats.revoked_referrals = 8;
			stats.penalty_score = 8 * PenaltyPerRevocationAmount::get();
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 86);
	});
}

#[test]
fn referral_score_cannot_go_negative() {
	use crate::types::ReferralScoreProvider;

	new_test_ext().execute_with(|| {
		// Extreme case: All referrals revoked
		// 5 total, 5 revoked = 0 good
		// penalty_score: 5 * PenaltyPerRevocation(3) = 15
		// Base score: 0
		// Final: 0 - 15 = 0 (saturating_sub)
		ReferrerStatsStorage::<Test>::mutate(REFERRER, |stats| {
			stats.total_referrals = 5;
			stats.revoked_referrals = 5;
			stats.penalty_score = 5 * PenaltyPerRevocationAmount::get();
		});
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 0);
	});
}

// ============================================================================
// InviterProvider Trait Tests
// ============================================================================

#[test]
fn get_inviter_returns_correct_referrer() {
	use crate::types::InviterProvider;

	new_test_ext().execute_with(|| {
		// Complete referral
		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			REFERRED,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER);

		// Verify InviterProvider trait
		assert_eq!(ReferralPallet::get_inviter(&REFERRED), Some(REFERRER));
	});
}

#[test]
fn get_inviter_returns_none_for_non_referred() {
	use crate::types::InviterProvider;

	new_test_ext().execute_with(|| {
		// User was not referred
		assert_eq!(ReferralPallet::get_inviter(&999), None);
	});
}

// ============================================================================
// Force Confirm Referral Tests (Sudo-only)
// ============================================================================

#[test]
fn force_confirm_referral_works() {
	use crate::types::InviterProvider;

	new_test_ext().execute_with(|| {
		// Force confirm referral (sudo-only)
		assert_ok!(ReferralPallet::force_confirm_referral(
			RuntimeOrigin::root(),
			REFERRER,
			REFERRED
		));

		// Verify storage updates
		assert_eq!(ReferralCount::<Test>::get(REFERRER), 1);
		assert!(Referrals::<Test>::contains_key(REFERRED));
		assert_eq!(Referrals::<Test>::get(REFERRED).unwrap().referrer, REFERRER);

		// Verify ReferrerStats is updated (was missing before fix)
		let stats = ReferrerStatsStorage::<Test>::get(REFERRER);
		assert_eq!(stats.total_referrals, 1);
		assert_eq!(stats.revoked_referrals, 0);
		assert_eq!(stats.penalty_score, 0);

		// Verify trait implementations
		assert_eq!(ReferralPallet::get_inviter(&REFERRED), Some(REFERRER));
	});
}

#[test]
fn force_confirm_referral_requires_root() {
	new_test_ext().execute_with(|| {
		// Non-root origin should fail
		assert_noop!(
			ReferralPallet::force_confirm_referral(
				RuntimeOrigin::signed(REFERRER),
				REFERRER,
				REFERRED
			),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn force_confirm_referral_prevents_self_referral() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			ReferralPallet::force_confirm_referral(RuntimeOrigin::root(), REFERRER, REFERRER),
			Error::<Test>::SelfReferral
		);
	});
}

#[test]
fn force_confirm_referral_prevents_duplicate() {
	new_test_ext().execute_with(|| {
		// First force confirm succeeds
		assert_ok!(ReferralPallet::force_confirm_referral(
			RuntimeOrigin::root(),
			REFERRER,
			REFERRED
		));

		// Second attempt fails
		assert_noop!(
			ReferralPallet::force_confirm_referral(RuntimeOrigin::root(), REFERRER, REFERRED),
			Error::<Test>::AlreadyReferred
		);
	});
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn complete_referral_flow_integration() {
	use crate::types::{InviterProvider, ReferralScoreProvider};

	new_test_ext().execute_with(|| {
		// Step 1: Initiate referral (legacy way via PendingReferrals)
		assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(REFERRER), REFERRED));
		assert_eq!(PendingReferrals::<Test>::get(REFERRED), Some(REFERRER));

		// Step 2: KYC approval triggers confirmation
		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			REFERRED,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER);

		// Step 3: Verify all storage updates
		assert_eq!(PendingReferrals::<Test>::get(REFERRED), None);
		assert_eq!(ReferralCount::<Test>::get(REFERRER), 1);
		assert!(Referrals::<Test>::contains_key(REFERRED));

		// Step 4: Verify trait implementations
		assert_eq!(ReferralPallet::get_inviter(&REFERRED), Some(REFERRER));
		assert_eq!(ReferralPallet::get_referral_score(&REFERRER), 10);
	});
}

#[test]
fn multiple_referrals_for_same_referrer() {
	new_test_ext().execute_with(|| {
		// REFERRER refers 3 people
		let referred1 = 10;
		let referred2 = 11;
		let referred3 = 12;

		// Approve all via direct calls
		for &referred in &[referred1, referred2, referred3] {
			pezpallet_identity_kyc::KycStatuses::<Test>::insert(
				referred,
				pezpallet_identity_kyc::types::KycLevel::Approved,
			);
			ReferralPallet::on_kyc_approved(&referred, &REFERRER);
		}

		// Verify count
		assert_eq!(ReferralCount::<Test>::get(REFERRER), 3);

		// Verify stats
		let stats = ReferrerStatsStorage::<Test>::get(REFERRER);
		assert_eq!(stats.total_referrals, 3);
	});
}

#[test]
fn referral_info_stores_block_number() {
	new_test_ext().execute_with(|| {
		let block_number = 42u64;
		System::set_block_number(block_number);

		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			REFERRED,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER);

		// Verify stored block number
		let info = Referrals::<Test>::get(REFERRED).unwrap();
		assert_eq!(info.created_at, block_number);
		assert_eq!(info.referrer, REFERRER);
	});
}
