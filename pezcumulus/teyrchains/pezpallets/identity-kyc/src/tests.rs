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
			Some(CITIZEN_1),
			None,
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
			Some(CITIZEN_2), // Same as caller → filtered → DefaultReferrer
			None,
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
				Some(CITIZEN_1), // Not a citizen, falls back to FOUNDER who is also not citizen
				None,
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
			Some(CITIZEN_1),
			None,
		));

		// Second application fails
		assert_noop!(
			IdentityKycPallet::apply_for_citizenship(
				RuntimeOrigin::signed(APPLICANT),
				H256::from_low_u64_be(99999),
				Some(CITIZEN_1),
				None,
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
				Some(CITIZEN_1),
				None,
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
			Some(CITIZEN_1),
			None,
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
			Some(CITIZEN_1),
			None,
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
			Some(CITIZEN_1),
			None,
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
			Some(CITIZEN_1),
			None,
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
			Some(CITIZEN_1),
			None,
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
			Some(CITIZEN_1),
			None,
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
			Some(CITIZEN_1),
			None,
		));

		// Cancel
		assert_ok!(IdentityKycPallet::cancel_application(RuntimeOrigin::signed(APPLICANT)));

		// Can apply again with different referrer
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(99999),
			Some(FOUNDER), // Different referrer this time
			None,
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
			Some(CITIZEN_1),
			None,
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
			Some(CITIZEN_1),
			None,
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
			Some(APPLICANT), // APPLICANT is now the referrer
			None,
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
			Some(CITIZEN_1),
			None,
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
			Some(FOUNDER),                // Different referrer
			None,
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
			Some(CITIZEN_1),
			None,
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

// ============================================================================
// WAITING ON A REFERRER
// ============================================================================
//
// Citizenship needs a person to vouch for it -- that is what keeps the register from filling
// with accounts nobody has met, and it is the whole of the sybil defence. But a referrer who
// never gets round to it would otherwise leave the applicant waiting for ever, and the answer
// cannot be to drop the application: that punishes the applicant for somebody else's silence.

mod referral_fallback {
	use super::*;

	fn apply_with(referrer: Option<u64>) {
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::repeat_byte(7),
			referrer,
			None,
		));
	}

	#[test]
	fn an_application_waits_rather_than_lapsing() {
		new_test_ext().execute_with(|| {
			apply_with(Some(CITIZEN_1));

			// Long past any deadline anyone might have imagined.
			System::set_block_number(1_000_000);

			assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::PendingReferral);
			assert!(IdentityKycPallet::applications(APPLICANT).is_some());
		});
	}

	#[test]
	fn the_referrer_can_still_approve_however_late() {
		new_test_ext().execute_with(|| {
			apply_with(Some(CITIZEN_1));
			System::set_block_number(1_000_000);

			assert_ok!(IdentityKycPallet::approve_referral(
				RuntimeOrigin::signed(CITIZEN_1),
				APPLICANT
			));
			assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::ReferrerApproved);
		});
	}

	#[test]
	fn the_founder_cannot_step_in_before_the_period_is_up() {
		new_test_ext().execute_with(|| {
			apply_with(Some(CITIZEN_1));

			assert_noop!(
				IdentityKycPallet::approve_referral(RuntimeOrigin::signed(FOUNDER), APPLICANT),
				Error::<Test>::NotTheReferrer
			);
		});
	}

	#[test]
	fn the_founder_may_approve_once_the_referrer_has_had_long_enough() {
		new_test_ext().execute_with(|| {
			apply_with(Some(CITIZEN_1));
			System::set_block_number(System::block_number() + ReferralFallbackPeriod::get() + 1);

			assert_ok!(IdentityKycPallet::approve_referral(
				RuntimeOrigin::signed(FOUNDER),
				APPLICANT
			));
			assert_eq!(IdentityKycPallet::kyc_status_of(APPLICANT), KycLevel::ReferrerApproved);
		});
	}

	#[test]
	fn whoever_actually_vouched_becomes_the_referrer_of_record() {
		// The accountability that follows a referral has to follow the person who vouched,
		// not the one who stayed silent: if this citizen is later revoked, the penalty lands
		// on the founder, who approved them.
		new_test_ext().execute_with(|| {
			apply_with(Some(CITIZEN_1));
			System::set_block_number(System::block_number() + ReferralFallbackPeriod::get() + 1);
			assert_ok!(IdentityKycPallet::approve_referral(
				RuntimeOrigin::signed(FOUNDER),
				APPLICANT
			));
			assert_ok!(IdentityKycPallet::confirm_citizenship(RuntimeOrigin::signed(APPLICANT)));

			assert_eq!(IdentityKycPallet::citizen_referrer(APPLICANT), Some(FOUNDER));
		});
	}

	#[test]
	fn an_application_with_no_referrer_falls_to_the_founder() {
		// Nobody becomes a citizen without a human saying so. Someone with no connections is
		// not turned away -- the founder is the referrer of last resort.
		new_test_ext().execute_with(|| {
			apply_with(None);

			let application = IdentityKycPallet::applications(APPLICANT).unwrap();
			assert_eq!(application.referrer, FOUNDER);

			assert_ok!(IdentityKycPallet::approve_referral(
				RuntimeOrigin::signed(FOUNDER),
				APPLICANT
			));
		});
	}
}

// ============================================================================
// TAKING CITIZENSHIP AND GIVING IT BACK
// ============================================================================

mod revocation {
	use super::*;

	#[test]
	fn a_revoked_citizen_can_be_restored() {
		// `Revoked` was terminal: `apply_for_citizenship` requires `NotStarted`, so no path
		// existed to undo a revocation, right or wrong. A court that can find a revocation
		// unjustified and not put it right is only half a court.
		new_test_ext().execute_with(|| {
			assert_ok!(IdentityKycPallet::revoke_citizenship(RuntimeOrigin::root(), CITIZEN_1));
			assert_eq!(IdentityKycPallet::kyc_status_of(CITIZEN_1), KycLevel::Revoked);
			let after_revoke = IdentityKycPallet::approved_citizen_count();

			assert_ok!(IdentityKycPallet::restore_citizenship(RuntimeOrigin::root(), CITIZEN_1));

			assert_eq!(IdentityKycPallet::kyc_status_of(CITIZEN_1), KycLevel::Approved);
			assert_eq!(IdentityKycPallet::approved_citizen_count(), after_revoke + 1);
		});
	}

	#[test]
	fn restoration_puts_them_back_as_a_citizen_not_at_the_start() {
		// Making somebody re-apply, find a referrer and wait again would be a second penalty
		// for a revocation that has just been found unjustified.
		new_test_ext().execute_with(|| {
			assert_ok!(IdentityKycPallet::revoke_citizenship(RuntimeOrigin::root(), CITIZEN_1));
			assert_ok!(IdentityKycPallet::restore_citizenship(RuntimeOrigin::root(), CITIZEN_1));

			assert_ne!(IdentityKycPallet::kyc_status_of(CITIZEN_1), KycLevel::NotStarted);
			assert!(IdentityKycPallet::is_citizen(&CITIZEN_1));
		});
	}

	#[test]
	fn only_a_revoked_citizenship_can_be_restored() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				IdentityKycPallet::restore_citizenship(RuntimeOrigin::root(), CITIZEN_1),
				Error::<Test>::NotRevoked
			);
		});
	}

	#[test]
	fn restoring_needs_the_governance_origin() {
		new_test_ext().execute_with(|| {
			assert_ok!(IdentityKycPallet::revoke_citizenship(RuntimeOrigin::root(), CITIZEN_1));
			assert_noop!(
				IdentityKycPallet::restore_citizenship(RuntimeOrigin::signed(CITIZEN_2), CITIZEN_1),
				DispatchError::BadOrigin
			);
		});
	}
}

// ============================================================================
// HONORARY CITIZENSHIP
// ============================================================================
//
// A citizen the state named rather than one who applied. The same status, the same rights,
// counted in the same population. The distinction is recorded so the chain can be asked how
// many came each way, and for nothing else.

mod honorary {
	use super::*;

	const GUEST: u64 = 42;

	#[test]
	fn an_honorary_citizen_is_a_citizen() {
		new_test_ext().execute_with(|| {
			let before = IdentityKycPallet::approved_citizen_count();

			assert_ok!(IdentityKycPallet::register_honorary_citizen(&GUEST));

			assert_eq!(IdentityKycPallet::kyc_status_of(GUEST), KycLevel::Approved);
			assert!(IdentityKycPallet::is_citizen(&GUEST));
			assert_eq!(IdentityKycPallet::approved_citizen_count(), before + 1);
		});
	}

	#[test]
	fn how_they_came_in_is_readable_from_the_chain() {
		new_test_ext().execute_with(|| {
			assert!(!IdentityKycPallet::is_honorary_citizen(GUEST).is_some());
			assert_ok!(IdentityKycPallet::register_honorary_citizen(&GUEST));

			assert!(IdentityKycPallet::is_honorary_citizen(GUEST).is_some());
			assert_eq!(IdentityKycPallet::honorary_citizen_count(), 1);
			// And the ones who applied are not in the register.
			assert!(IdentityKycPallet::is_honorary_citizen(CITIZEN_1).is_none());
		});
	}

	#[test]
	fn nobody_is_named_a_citizen_twice() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				IdentityKycPallet::register_honorary_citizen(&CITIZEN_1),
				Error::<Test>::AlreadyACitizen
			);
			assert_eq!(IdentityKycPallet::honorary_citizen_count(), 0);
		});
	}
}

// ============================================================================
// THE INVARIANT CAN FAIL
// ============================================================================

#[cfg(feature = "try-runtime")]
mod invariant {
	use super::*;
	use crate::{
		CitizenCount, HonoraryCitizenCount, HonoraryCitizens, IdentityHashToAccount,
		IdentityHashes, KycStatuses,
	};
	use pezframe_support::traits::Hooks;

	fn check() -> Result<(), pezsp_runtime::TryRuntimeError> {
		<IdentityKycPallet as Hooks<u64>>::try_state(System::block_number())
	}

	fn assert_rejected(what: &str) {
		assert!(check().is_err(), "try_state accepted a state where {what}");
	}

	#[test]
	fn an_ordinary_state_passes() {
		new_test_ext().execute_with(|| {
			assert_ok!(IdentityKycPallet::register_honorary_citizen(&42));
			assert_ok!(check());
		});
	}

	#[test]
	fn a_miscounted_population_is_caught() {
		// The number the treasury reads to decide whether the state has enough citizens to
		// start paying them.
		new_test_ext().execute_with(|| {
			CitizenCount::<Test>::mutate(|n| *n += 1);
			assert_rejected("the population count did not match the register");
		});
	}

	#[test]
	fn a_hash_that_does_not_point_back_is_caught() {
		// One person, one citizenship, enforced by the hash being unique. If the two maps
		// disagree, two accounts can hold the same hash with only one visible from either
		// side -- and that uniqueness is the whole of the sybil defence.
		new_test_ext().execute_with(|| {
			let hash = IdentityHashes::<Test>::get(CITIZEN_1).unwrap();
			IdentityHashToAccount::<Test>::insert(hash, CITIZEN_2);
			assert_rejected("an identity hash pointed at the wrong account");
		});
	}

	#[test]
	fn an_honorary_entry_for_a_non_citizen_is_caught() {
		new_test_ext().execute_with(|| {
			HonoraryCitizens::<Test>::insert(99, ());
			HonoraryCitizenCount::<Test>::mutate(|n| *n += 1);
			assert_rejected("somebody was in the honorary register without being a citizen");
		});
	}

	#[test]
	fn a_miscounted_honorary_register_is_caught() {
		new_test_ext().execute_with(|| {
			assert_ok!(IdentityKycPallet::register_honorary_citizen(&42));
			HonoraryCitizenCount::<Test>::mutate(|n| *n += 1);
			assert_rejected("the honorary count did not match the honorary register");
		});
	}

	#[test]
	fn more_named_citizens_than_citizens_is_caught() {
		new_test_ext().execute_with(|| {
			for who in 40..60u64 {
				HonoraryCitizens::<Test>::insert(who, ());
				KycStatuses::<Test>::insert(who, KycLevel::Approved);
			}
			HonoraryCitizenCount::<Test>::put(20);
			assert_rejected("more citizens were named than there are citizens");
		});
	}
}

/// A citizen waits before vouching; the founding generation does not.
///
/// The waiting period exists to slow a chain of vouching -- one forged citizen admitting the
/// next within minutes. That attack needs a citizen to have been admitted, so the rule binds
/// whoever was admitted. It does not bind the founding citizens: they were written at genesis
/// rather than vouched in, their number is fixed, and an attacker cannot add to that set. If
/// they waited too, the register would simply be shut for a month and no attack prevented.
#[test]
fn a_new_citizen_waits_before_vouching_and_the_founders_do_not() {
	new_test_ext().execute_with(|| {
		// Genesis citizen, first block, vouches immediately.
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(APPLICANT),
			H256::from_low_u64_be(42),
			Some(CITIZEN_1),
			None,
		));
		assert_ok!(IdentityKycPallet::approve_referral(
			RuntimeOrigin::signed(CITIZEN_1),
			APPLICANT
		));
		assert_ok!(IdentityKycPallet::confirm_citizenship(RuntimeOrigin::signed(APPLICANT)));

		// The one just admitted may not pass it on yet.
		// A funded account: the application reserves a deposit.
		let newcomer = CITIZEN_2;
		assert_ok!(IdentityKycPallet::apply_for_citizenship(
			RuntimeOrigin::signed(newcomer),
			H256::from_low_u64_be(43),
			Some(APPLICANT),
			None,
		));
		assert_noop!(
			IdentityKycPallet::approve_referral(RuntimeOrigin::signed(APPLICANT), newcomer),
			Error::<Test>::VouchingTooSoon
		);

		// And may once the period has run.
		System::set_block_number(System::block_number() + VouchingWaitingPeriod::get());
		assert_ok!(IdentityKycPallet::approve_referral(RuntimeOrigin::signed(APPLICANT), newcomer));
	});
}
