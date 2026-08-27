// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{mock::*, Error, Event, Invitations, ReferralCount, Referrals, ReferrerStatsStorage};
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
		assert!(ReferralPallet::invitation_claim(REFERRED, REFERRER).is_some());

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
fn several_people_may_claim_the_same_newcomer() {
	// This used to refuse the second claim, which meant whoever called first held the only
	// slot for any address in existence -- a stranger could take the place of the person who
	// had actually done the inviting, and nothing could clear it. A claim decides nothing on
	// its own, so letting several stand costs nothing and takes the race away. The newcomer
	// settles it by naming one of them.
	new_test_ext().execute_with(|| {
		assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(REFERRER), REFERRED));
		assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(USER_3), REFERRED));

		assert!(ReferralPallet::invitation_claim(REFERRED, REFERRER).is_some());
		assert!(ReferralPallet::invitation_claim(REFERRED, USER_3).is_some());
	});
}

#[test]
fn nobody_claims_a_newcomer_whose_invitation_is_already_settled() {
	new_test_ext().execute_with(|| {
		assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(REFERRER), REFERRED));
		crate::InvitedBy::<Test>::insert(REFERRED, REFERRER);

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
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);

		// Verification
		// 1. Pending referral record is deleted
		assert!(Invitations::<Test>::iter_prefix(REFERRED).next().is_none());
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
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);

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
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);

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
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);
		assert_eq!(ReferralCount::<Test>::get(REFERRER), 1);

		// Second approval attempt should be ignored (already processed)
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);
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
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);

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
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);

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

// The four tests that were here exercised `force_confirm_referral`, the root call that could
// invent a confirmed referral between two accounts with no KYC, no citizenship and no
// application. It existed to repair historical data; both chains start again from genesis, so
// there is none, and a call that writes a referral out of nothing writes a trust score out of
// nothing.

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
			ReferralPallet::on_kyc_approved(&referred, &REFERRER, None);
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
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);

		// Verify stored block number
		let info = Referrals::<Test>::get(REFERRED).unwrap();
		assert_eq!(info.created_at, block_number);
		assert_eq!(info.referrer, REFERRER);
	});
}

// ============================================================================
// WHO BROUGHT SOMEBODY HERE, AND WHO STOOD FOR THEM
// ============================================================================
//
// Two different facts about the same citizen, and frequently two different people. You can be
// brought to the state by one person and ask another -- a parent, a friend -- to stand for
// you. The guarantor takes the credit and the consequences; the one who brought you gets a
// record of having done it and nothing else.

mod invitations {
	use super::*;
	use crate::{InvitationCount, InvitedBy};
	use pezpallet_identity_kyc::types::OnKycApproved;

	const INVITER: u64 = 7;

	/// Mark the applicant approved, the way `identity-kyc` does before it calls the hook.
	/// The hook re-checks it on-chain rather than trusting the caller.
	fn approved(who: u64) {
		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			who,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);
	}

	#[test]
	fn an_invitation_needs_both_sides_to_say_so() {
		new_test_ext().execute_with(|| {
			assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(INVITER), REFERRED));
			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, Some(&INVITER));

			assert_eq!(InvitedBy::<Test>::get(REFERRED), Some(INVITER));
			assert_eq!(InvitationCount::<Test>::get(INVITER), 1);
		});
	}

	#[test]
	fn a_claim_nobody_confirms_counts_for_nothing() {
		// Otherwise a bot could watch for new addresses, claim every one of them, and collect
		// a record of having built the community it never touched.
		new_test_ext().execute_with(|| {
			assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(INVITER), REFERRED));
			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);

			assert_eq!(InvitedBy::<Test>::get(REFERRED), None);
			assert_eq!(InvitationCount::<Test>::get(INVITER), 0);
		});
	}

	#[test]
	fn naming_somebody_who_never_claimed_counts_for_nothing() {
		// The other direction: a citizen cannot hand the credit for their arrival to whoever
		// they like. Both people have to have said it.
		new_test_ext().execute_with(|| {
			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, Some(&INVITER));

			assert_eq!(InvitedBy::<Test>::get(REFERRED), None);
			assert_eq!(InvitationCount::<Test>::get(INVITER), 0);
		});
	}

	#[test]
	fn only_the_claim_the_newcomer_named_survives() {
		new_test_ext().execute_with(|| {
			assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(INVITER), REFERRED));
			assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(USER_3), REFERRED));

			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, Some(&INVITER));

			assert_eq!(InvitedBy::<Test>::get(REFERRED), Some(INVITER));
			assert_eq!(InvitationCount::<Test>::get(USER_3), 0);
			// The losing claims are cleared; they cannot be reused.
			assert!(crate::Invitations::<Test>::iter_prefix(REFERRED).next().is_none());
		});
	}

	#[test]
	fn bringing_somebody_in_is_not_standing_for_them() {
		// The record the Serok would read: somebody who has grown the country and taken
		// nothing from it shows up as a high invitation count beside a referral count of
		// zero. One number could never show that.
		new_test_ext().execute_with(|| {
			assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(INVITER), REFERRED));
			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, Some(&INVITER));

			assert_eq!(InvitationCount::<Test>::get(INVITER), 1);
			assert_eq!(ReferralCount::<Test>::get(INVITER), 0);
			// And the guarantor has the reverse.
			assert_eq!(ReferralCount::<Test>::get(REFERRER), 1);
			assert_eq!(InvitationCount::<Test>::get(REFERRER), 0);
		});
	}

	#[test]
	fn the_record_outlives_the_claim_that_made_it() {
		new_test_ext().execute_with(|| {
			assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(INVITER), REFERRED));
			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, Some(&INVITER));

			// The claim is gone, the fact it settled is not -- the old code deleted the
			// invitation at exactly the moment it started to mean something.
			assert!(crate::Invitations::<Test>::iter_prefix(REFERRED).next().is_none());
			assert_eq!(InvitedBy::<Test>::get(REFERRED), Some(INVITER));
		});
	}
}

// ============================================================================
// WHO PAYS WHEN A CITIZEN IS REVOKED
// ============================================================================

mod accountability {
	use super::*;
	use crate::{InvitedBy, ReferrerStatsStorage};
	use pezpallet_identity_kyc::types::{OnCitizenshipRevoked, OnKycApproved};

	const INVITER: u64 = 7;

	/// Mark the applicant approved, the way `identity-kyc` does before it calls the hook.
	/// The hook re-checks it on-chain rather than trusting the caller.
	fn approved(who: u64) {
		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			who,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);
	}

	#[test]
	fn the_guarantor_pays() {
		// Standing for somebody is saying you believe in them. The cost of a guarantee
		// belongs to the guarantor.
		new_test_ext().execute_with(|| {
			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);
			ReferralPallet::on_citizenship_revoked(&REFERRED);

			assert_eq!(ReferrerStatsStorage::<Test>::get(REFERRER).revoked_referrals, 1);
		});
	}

	#[test]
	fn the_founder_standing_in_does_not_pay() {
		// The founder approves applications nobody else answered. That is a structural role,
		// and charging it to them would be a certainty rather than a risk: their referral
		// score is capped at five hundred and reached almost at once, while the penalty has
		// no ceiling. It falls instead to whoever actually brought the person here.
		new_test_ext().execute_with(|| {
			assert_ok!(ReferralPallet::initiate_referral(RuntimeOrigin::signed(INVITER), REFERRED));
			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &FOUNDER, Some(&INVITER));
			pezpallet_identity_kyc::ApprovedByFallback::<Test>::insert(REFERRED, ());

			ReferralPallet::on_citizenship_revoked(&REFERRED);

			assert_eq!(ReferrerStatsStorage::<Test>::get(FOUNDER).revoked_referrals, 0);
			assert_eq!(ReferrerStatsStorage::<Test>::get(INVITER).revoked_referrals, 1);
		});
	}

	#[test]
	fn the_founder_pays_for_the_people_they_chose_to_stand_for() {
		// Being the guarantor of last resort does not make the founder exempt everywhere. If
		// they stood for somebody in the ordinary way, they carry it in the ordinary way.
		new_test_ext().execute_with(|| {
			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &FOUNDER, None);
			ReferralPallet::on_citizenship_revoked(&REFERRED);

			assert_eq!(ReferrerStatsStorage::<Test>::get(FOUNDER).revoked_referrals, 1);
		});
	}

	#[test]
	fn nobody_pays_when_nobody_vouched_in_any_sense() {
		new_test_ext().execute_with(|| {
			approved(REFERRED);
			ReferralPallet::on_kyc_approved(&REFERRED, &FOUNDER, None);
			pezpallet_identity_kyc::ApprovedByFallback::<Test>::insert(REFERRED, ());
			assert_eq!(InvitedBy::<Test>::get(REFERRED), None);

			ReferralPallet::on_citizenship_revoked(&REFERRED);

			assert_eq!(ReferrerStatsStorage::<Test>::get(FOUNDER).revoked_referrals, 0);
		});
	}
}

// ===== VOUCHING CAPACITY =====

/// Bringing people into the register costs room, and the room is earned.
///
/// The register has no authority in its path: a citizen says "I know this person" and that is
/// the whole of it. So the word has to cost something, or the population can be manufactured
/// as cheaply as accounts can be made. It costs two things. Waiting -- a citizen admitted
/// today vouches for nobody -- and having vouched well before: capacity starts small, grows
/// with settled referrals, and shrinks when one of them is revoked.
///
/// The arithmetic here uses the mock's numbers: two places to begin with, one more for every
/// two who stayed, never more than six.
#[test]
fn vouching_capacity_is_earned_and_bounded() {
	new_test_ext().execute_with(|| {
		let who = REFERRER;

		// Nothing vouched for yet: the opening allowance and no more.
		assert_eq!(ReferralPallet::vouching_capacity(&who), 2);
		assert_eq!(ReferralPallet::vouching_remaining(&who), 2);

		// Two who stayed buys a third place.
		ReferrerStatsStorage::<Test>::insert(
			who,
			crate::types::ReferrerStats {
				total_referrals: 2,
				revoked_referrals: 0,
				penalty_score: 0,
			},
		);
		assert_eq!(ReferralPallet::vouching_capacity(&who), 3);

		// One of them revoked and the place goes with it. Vouching carelessly costs the room
		// to do it again, which is the part a penalty on the score alone does not reach.
		ReferrerStatsStorage::<Test>::insert(
			who,
			crate::types::ReferrerStats {
				total_referrals: 2,
				revoked_referrals: 1,
				penalty_score: 3,
			},
		);
		assert_eq!(ReferralPallet::vouching_capacity(&who), 2);

		// However good the record, there is a ceiling.
		ReferrerStatsStorage::<Test>::insert(
			who,
			crate::types::ReferrerStats {
				total_referrals: 1_000,
				revoked_referrals: 0,
				penalty_score: 0,
			},
		);
		assert_eq!(ReferralPallet::vouching_capacity(&who), 6);
	});
}

/// What is left counts those already brought in, not only what was allowed.
#[test]
fn settled_invitations_use_up_the_allowance() {
	new_test_ext().execute_with(|| {
		let who = REFERRER;
		assert_eq!(ReferralPallet::vouching_remaining(&who), 2);

		crate::InvitationCount::<Test>::insert(who, 2);
		assert_eq!(ReferralPallet::vouching_remaining(&who), 0);

		// And the register asks this exact question before letting anyone vouch.
		use pezpallet_identity_kyc::types::VouchingCapacity;
		assert_eq!(<ReferralPallet as VouchingCapacity<AccountId>>::remaining(&who), Some(0));
	});
}

/// A record bad enough stops the account vouching, and the court undoing it lifts the stop.
///
/// The tree is public and a manufactured cluster shows as a subtree with an anomalous share of
/// revocations. But a revocation only happens once somebody has noticed, so this is a lagging
/// signal, and the right answer to a lagging signal is to stop the bleeding rather than to
/// punish -- the penalty to standing already does the punishing.
///
/// The second half matters as much as the first. A revocation can be wrong. If restoring the
/// citizenship left the voucher's record marked, the court would correct its own mistake and
/// the person who vouched honestly would carry it for ever -- and now not only in standing but
/// in the capacity to vouch again at all.
#[test]
fn a_bad_record_suspends_vouching_and_the_court_can_lift_it() {
	new_test_ext().execute_with(|| {
		let who = REFERRER;

		// Ten brought in, two of them revoked: a fifth, which is at the line but under the
		// floor of three. Still vouching.
		ReferrerStatsStorage::<Test>::insert(
			who,
			crate::types::ReferrerStats {
				total_referrals: 10,
				revoked_referrals: 2,
				penalty_score: 6,
			},
		);
		assert!(ReferralPallet::vouching_capacity(&who) > 0);

		// A third: floor reached and three in ten is above a fifth. Stopped.
		ReferrerStatsStorage::<Test>::insert(
			who,
			crate::types::ReferrerStats {
				total_referrals: 10,
				revoked_referrals: 3,
				penalty_score: 9,
			},
		);
		assert_eq!(ReferralPallet::vouching_capacity(&who), 0);

		// Three in a hundred is not a pattern, and a prolific voucher is not stopped by it.
		ReferrerStatsStorage::<Test>::insert(
			who,
			crate::types::ReferrerStats {
				total_referrals: 100,
				revoked_referrals: 3,
				penalty_score: 9,
			},
		);
		assert!(ReferralPallet::vouching_capacity(&who) > 0);
	});
}

/// Undoing a revocation undoes what it charged the voucher.
///
/// A guarantee costs the guarantor, and a wrongful finding must not. Before this, the court
/// could restore a citizenship it had taken and the person who vouched for them kept the mark
/// -- in standing, and now also in the capacity to vouch again, since capacity is computed
/// from that same record.
#[test]
fn restoring_a_citizenship_refunds_the_voucher() {
	use pezpallet_identity_kyc::types::OnCitizenshipRestored;

	new_test_ext().execute_with(|| {
		pezpallet_identity_kyc::KycStatuses::<Test>::insert(
			REFERRED,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);
		ReferralPallet::on_kyc_approved(&REFERRED, &REFERRER, None);
		let before = ReferrerStatsStorage::<Test>::get(REFERRER);

		ReferralPallet::on_citizenship_revoked(&REFERRED);
		let charged = ReferrerStatsStorage::<Test>::get(REFERRER);
		assert_eq!(charged.revoked_referrals, before.revoked_referrals + 1);
		assert!(charged.penalty_score > before.penalty_score);

		<ReferralPallet as OnCitizenshipRestored<AccountId>>::on_citizenship_restored(&REFERRED);
		let refunded = ReferrerStatsStorage::<Test>::get(REFERRER);
		assert_eq!(refunded.revoked_referrals, before.revoked_referrals);
		assert_eq!(refunded.penalty_score, before.penalty_score);
	});
}
