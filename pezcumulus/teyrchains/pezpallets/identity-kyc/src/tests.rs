// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{mock::*, types::KycLevel, Error, Event};
use pezframe_support::{assert_noop, assert_ok, traits::Currency};
use pezsp_core::H256;
use pezsp_runtime::DispatchError;

// We give our pallet an alias for easy access.
type IdentityKycPallet = crate::Pezpallet<Test>;

// ============================================================================
// Genesis Config Tests
// ============================================================================

#[test]
fn genesis_config_works() {
	new_test_ext().execute_with(|| {
		// FOUNDER and CITIZEN_1 should be pre-approved via genesis
		assert_eq!(IdentityKycPallet::kyc_status_of(FOUNDER), KycLevel::Approved);
		assert_eq!(IdentityKycPallet::kyc_status_of(CITIZEN_1), KycLevel::Approved);

		// Their identity hashes should be stored
		assert!(IdentityKycPallet::identity_hash_of(FOUNDER).is_some());
		assert!(IdentityKycPallet::identity_hash_of(CITIZEN_1).is_some());

		// Non-founding users should be NotStarted
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::NotStarted);
	});
}

// ============================================================================
// apply_for_citizenship Tests
// ============================================================================

#[test]
fn apply_for_citizenship_works() {
	new_test_ext().execute_with(|| {
		let identity_hash = H256::from_low_u64_be(12345);

		// APPLICANT applies with CITIZEN_1 as referrer (who is pre-approved)
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			identity_hash,
			Some(CITIZEN_1)
		));

		// Check status changed to PendingReferral
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::PendingReferral);

		// Check application was stored
		let app = IdentityKycPallet::applications(APPLICANT).expect("Application should exist");
		assert_eq!(app.identity_hash, identity_hash);
		assert_eq!(app.referrer, CITIZEN_1);

		// Check deposit was reserved
		assert_eq!(Balances::reserved_balance(APPLICANT), KycApplicationDepositAmount::get());

		// Check event was emitted
		System::assert_last_event(
			Event::CitizenshipApplied { applicant: APPLICANT, referrer: CITIZEN_1, identity_hash }
				.into(),
		);
	});
}

#[test]
fn apply_for_citizenship_falls_back_on_self_referral() {
	new_test_ext().execute_with(|| {
		// Self-referral with Some(self) is silently filtered,
		// falls back to DefaultReferrer (FOUNDER)
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(CITIZEN_2),
			H256::from_low_u64_be(999),
			Some(CITIZEN_2) // Same as caller → filtered → DefaultReferrer
		));

		// Should use FOUNDER as referrer
		let app = IdentityKycPallet::applications(CITIZEN_2).unwrap();
		assert_eq!(app.referrer, FOUNDER);
	});
}

#[test]
fn apply_for_citizenship_fails_if_referrer_not_citizen() {
	new_test_ext_empty().execute_with(|| {
		// In empty setup, no founding citizens exist
		// Any referrer is invalid, and DefaultReferrer (FOUNDER) is also not a citizen
		assert_noop!(
			IdentityKycPallet::apply_for_citizenship(
				RuntimeOrigin::signed(APPLICANT),
				H256::from_low_u64_be(999),
				Some(CITIZEN_1) // Not a citizen, falls back to FOUNDER who is also not citizen
			),
			Error::<Test>::ReferrerNotCitizen
		);
	});
}

#[test]
fn apply_for_citizenship_fails_if_already_applied() {
	new_test_ext().execute_with(|| {
		let identity_hash = H256::from_low_u64_be(12345);

		// First application succeeds
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			identity_hash,
			Some(CITIZEN_1)
		));

		// Second application fails
		assert_noop!(
			IdentityKycPallet::apply_for_citizenship(
				RuntimeOrigin::signed(APPLICANT),
				H256::from_low_u64_be(99999),
				Some(CITIZEN_1)
			),
			Error::<Test>::ApplicationAlreadyExists
		);
	});
}

#[test]
fn apply_for_citizenship_fails_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let poor_user = 999; // No balance in genesis

		assert_noop!(
			IdentityKycPallet::apply_for_citizenship(
				RuntimeOrigin::signed(poor_user),
				H256::from_low_u64_be(12345),
				Some(CITIZEN_1)
			),
			pezpallet_balances::Error::<Test>::InsufficientBalance
		);
	});
}

// ============================================================================
// approve_referral Tests
// ============================================================================

#[test]
fn approve_referral_works() {
	new_test_ext().execute_with(|| {
		let identity_hash = H256::from_low_u64_be(12345);

		// APPLICANT applies with CITIZEN_1 as referrer
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			identity_hash,
			Some(CITIZEN_1)
		));

		// CITIZEN_1 approves the referral
		assert_ok!(IdentityKycPallet::approve_referral(
			RuntimeOrigin::signed(CITIZEN_1),
			APPLICANT
		));

		// Check status changed to ReferrerApproved
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::ReferrerApproved);

		// Check event
		System::assert_last_event(
			Event::ReferralApproved { referrer: CITIZEN_1, applicant: APPLICANT }.into(),
		);
	});
}

#[test]
fn approve_referral_fails_if_not_referrer() {
	new_test_ext().execute_with(|| {
		// APPLICANT applies with CITIZEN_1 as referrer
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(12345),
			Some(CITIZEN_1)
		));

		// FOUNDER (different citizen) cannot approve
		assert_noop!(
			IdentityKycPallet::approve_referral(RuntimeOrigin::signed(FOUNDER), APPLICANT),
			Error::<Test>::NotTheReferrer
		);
	});
}

#[test]
fn approve_referral_fails_if_not_pending() {
	new_test_ext().execute_with(|| {
		// Try to approve referral for someone who hasn't applied
		assert_noop!(
			IdentityKycPallet::approve_referral(RuntimeOrigin::signed(CITIZEN_1), APPLICANT),
			Error::<Test>::CannotApproveInCurrentState
		);
	});
}

// ============================================================================
// confirm_citizenship Tests (Self-confirmation for Welati NFT)
// ============================================================================

#[test]
fn confirm_citizenship_works() {
	new_test_ext().execute_with(|| {
		let identity_hash = H256::from_low_u64_be(12345);
		let initial_balance = Balances::free_balance(APPLICANT);

		// Apply
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			identity_hash,
			Some(CITIZEN_1)
		));

		// Referrer approves
		assert_ok!(IdentityKycPallet::approve_referral(
			RuntimeOrigin::signed(CITIZEN_1),
			APPLICANT
		));

		// Self-confirm
		assert_ok!(IdentityKycPallet::confirm_citizenship(RuntimeOrigin::signed(APPLICANT)));

		// Check status is Approved
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::Approved);

		// Check identity hash is stored permanently
		assert_eq!(IdentityKycPallet::identity_hash_of(APPLICANT), Some(identity_hash));

		// Check referrer is stored permanently
		assert_eq!(IdentityKycPallet::citizen_referrer(APPLICANT), Some(CITIZEN_1));

		// Check application was removed
		assert!(IdentityKycPallet::applications(APPLICANT).is_none());

		// Check deposit was returned
		assert_eq!(Balances::reserved_balance(APPLICANT), 0);
		assert_eq!(Balances::free_balance(APPLICANT), initial_balance);

		// Check event
		System::assert_last_event(Event::CitizenshipConfirmed { who: APPLICANT }.into());
	});
}

#[test]
fn confirm_citizenship_fails_if_not_referrer_approved() {
	new_test_ext().execute_with(|| {
		// Apply but don't get referrer approval
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(12345),
			Some(CITIZEN_1)
		));

		// Try to self-confirm without referrer approval
		assert_noop!(
			IdentityKycPallet::confirm_citizenship(RuntimeOrigin::signed(APPLICANT)),
			Error::<Test>::CannotConfirmInCurrentState
		);
	});
}

#[test]
fn confirm_citizenship_fails_if_not_applied() {
	new_test_ext().execute_with(|| {
		// Try to confirm without applying
		assert_noop!(
			IdentityKycPallet::confirm_citizenship(RuntimeOrigin::signed(APPLICANT)),
			Error::<Test>::CannotConfirmInCurrentState
		);
	});
}

// ============================================================================
// cancel_application Tests
// ============================================================================

#[test]
fn cancel_application_works() {
	new_test_ext().execute_with(|| {
		let initial_balance = Balances::free_balance(APPLICANT);

		// Apply
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(12345),
			Some(CITIZEN_1)
		));

		// Deposit should be reserved
		assert_eq!(Balances::reserved_balance(APPLICANT), KycApplicationDepositAmount::get());

		// Cancel
		assert_ok!(IdentityKycPallet::cancel_application(RuntimeOrigin::signed(APPLICANT)));

		// Status should be reset to NotStarted
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::NotStarted);

		// Application should be removed
		assert!(IdentityKycPallet::applications(APPLICANT).is_none());

		// Deposit should be returned
		assert_eq!(Balances::reserved_balance(APPLICANT), 0);
		assert_eq!(Balances::free_balance(APPLICANT), initial_balance);

		// Event
		System::assert_last_event(Event::ApplicationCancelled { who: APPLICANT }.into());
	});
}

#[test]
fn cancel_application_fails_if_not_pending_referral() {
	new_test_ext().execute_with(|| {
		// Apply and get referrer approval
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(12345),
			Some(CITIZEN_1)
		));
		assert_ok!(IdentityKycPallet::approve_referral(
			RuntimeOrigin::signed(CITIZEN_1),
			APPLICANT
		));

		// Cannot cancel after referrer approved (status is ReferrerApproved)
		assert_noop!(
			IdentityKycPallet::cancel_application(RuntimeOrigin::signed(APPLICANT)),
			Error::<Test>::CannotCancelInCurrentState
		);
	});
}

#[test]
fn cancel_application_allows_reapplication() {
	new_test_ext().execute_with(|| {
		// First application
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(12345),
			Some(CITIZEN_1)
		));

		// Cancel
		assert_ok!(IdentityKycPallet::cancel_application(RuntimeOrigin::signed(APPLICANT)));

		// Can apply again with different referrer
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(99999),
			Some(FOUNDER) // Different referrer this time
		));

		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::PendingReferral);
	});
}

// ============================================================================
// revoke_citizenship Tests (Governance action)
// ============================================================================

#[test]
fn revoke_citizenship_works() {
	new_test_ext().execute_with(|| {
		// Complete citizenship flow for APPLICANT
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(12345),
			Some(CITIZEN_1)
		));
		assert_ok!(IdentityKycPallet::approve_referral(
			RuntimeOrigin::signed(CITIZEN_1),
			APPLICANT
		));
		assert_ok!(IdentityKycPallet::confirm_citizenship(RuntimeOrigin::signed(APPLICANT)));

		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::Approved);

		// Governance revokes
		assert_ok!(IdentityKycPallet::revoke_citizenship(RuntimeOrigin::root(), APPLICANT));

		// Status should be Revoked
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::Revoked);

		// Event
		System::assert_last_event(Event::CitizenshipRevoked { who: APPLICANT }.into());
	});
}

#[test]
fn revoke_citizenship_fails_for_bad_origin() {
	new_test_ext().execute_with(|| {
		// Non-root cannot revoke
		assert_noop!(
			IdentityKycPallet::revoke_citizenship(RuntimeOrigin::signed(CITIZEN_1), FOUNDER),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn revoke_citizenship_fails_if_not_citizen() {
	new_test_ext().execute_with(|| {
		// APPLICANT is not a citizen
		assert_noop!(
			IdentityKycPallet::revoke_citizenship(RuntimeOrigin::root(), APPLICANT),
			Error::<Test>::CannotRevokeInCurrentState
		);
	});
}

// ============================================================================
// renounce_citizenship Tests (Voluntary exit)
// ============================================================================

#[test]
fn renounce_citizenship_works() {
	new_test_ext().execute_with(|| {
		// CITIZEN_1 is pre-approved, can renounce
		assert_eq!(IdentityKycPallet::kyc_status_of(CITIZEN_1), KycLevel::Approved);

		assert_ok!(IdentityKycPallet::renounce_citizenship(RuntimeOrigin::signed(CITIZEN_1)));

		// Status should be reset to NotStarted
		assert_eq!(IdentityKycPallet::kyc_status_of(CITIZEN_1), KycLevel::NotStarted);

		// Identity hash should be removed
		assert!(IdentityKycPallet::identity_hash_of(CITIZEN_1).is_none());

		// Event
		System::assert_last_event(Event::CitizenshipRenounced { who: CITIZEN_1 }.into());
	});
}

#[test]
fn renounce_citizenship_fails_if_not_citizen() {
	new_test_ext().execute_with(|| {
		// APPLICANT is not a citizen
		assert_noop!(
			IdentityKycPallet::renounce_citizenship(RuntimeOrigin::signed(APPLICANT)),
			Error::<Test>::NotACitizen
		);
	});
}

// ============================================================================
// Full Workflow Tests
// ============================================================================

#[test]
fn full_citizenship_workflow() {
	new_test_ext().execute_with(|| {
		let identity_hash = H256::from_low_u64_be(12345);

		// 1. Apply
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			identity_hash,
			Some(CITIZEN_1)
		));
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::PendingReferral);

		// 2. Referrer approves
		assert_ok!(IdentityKycPallet::approve_referral(
			RuntimeOrigin::signed(CITIZEN_1),
			APPLICANT
		));
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::ReferrerApproved);

		// 3. Self-confirm
		assert_ok!(IdentityKycPallet::confirm_citizenship(RuntimeOrigin::signed(APPLICANT)));
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::Approved);

		// 4. Now APPLICANT is a citizen and can be a referrer for others
		let new_user = 50;
		// First give new_user some balance
		Balances::make_free_balance_be(&new_user, 10_000);

		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(new_user),
			H256::from_low_u64_be(99999),
			Some(APPLICANT) // APPLICANT is now the referrer
		));
		assert_eq!(IdentityKycPallet::kyc_status_of(new_user), KycLevel::PendingReferral);
	});
}

#[test]
fn renounce_and_reapply_workflow() {
	new_test_ext().execute_with(|| {
		// Complete first citizenship
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(12345),
			Some(CITIZEN_1)
		));
		assert_ok!(IdentityKycPallet::approve_referral(
			RuntimeOrigin::signed(CITIZEN_1),
			APPLICANT
		));
		assert_ok!(IdentityKycPallet::confirm_citizenship(RuntimeOrigin::signed(APPLICANT)));
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::Approved);

		// Renounce
		assert_ok!(IdentityKycPallet::renounce_citizenship(RuntimeOrigin::signed(APPLICANT)));
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::NotStarted);

		// Can reapply (free world principle)
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(99999), // Different hash
			Some(FOUNDER)                 // Different referrer
		));
		assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::PendingReferral);
	});
}

// ============================================================================
// Helper Function Tests
// ============================================================================

#[test]
fn is_citizen_works() {
	new_test_ext().execute_with(|| {
		// Founding citizens should return true
		assert!(IdentityKycPallet::is_citizen(&FOUNDER));
		assert!(IdentityKycPallet::is_citizen(&CITIZEN_1));

		// Non-citizens should return false
		assert!(!IdentityKycPallet::is_citizen(&APPLICANT));
	});
}

#[test]
fn get_referrer_works() {
	new_test_ext().execute_with(|| {
		// Complete citizenship for APPLICANT
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(12345),
			Some(CITIZEN_1)
		));
		assert_ok!(IdentityKycPallet::approve_referral(
			RuntimeOrigin::signed(CITIZEN_1),
			APPLICANT
		));
		assert_ok!(IdentityKycPallet::confirm_citizenship(RuntimeOrigin::signed(APPLICANT)));

		// Should return the referrer
		assert_eq!(IdentityKycPallet::get_referrer(&APPLICANT), Some(CITIZEN_1));

		// Founding citizens have no referrer (they were genesis)
		assert_eq!(IdentityKycPallet::get_referrer(&FOUNDER), None);
	});
}
