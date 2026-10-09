// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{mock::*, Committee, Error, Event};
use pezframe_support::{assert_noop, assert_ok, BoundedVec};

fn members(v: &[(u64, u128)]) -> BoundedVec<(u64, u128), MaxMembers> {
	BoundedVec::try_from(v.to_vec()).unwrap()
}

#[test]
fn only_the_people_chain_may_note_a_committee() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Komite::set_committee(RuntimeOrigin::signed(1), 1, members(&[(10, 5)])),
			pezsp_runtime::DispatchError::BadOrigin
		);
		assert_ok!(Komite::set_committee(RuntimeOrigin::root(), 1, members(&[(10, 5)])));
	});
}

#[test]
fn a_new_snapshot_replaces_the_old_one_and_trust_is_per_mille_of_the_highest() {
	new_test_ext().execute_with(|| {
		assert_ok!(Komite::set_committee(
			RuntimeOrigin::root(),
			1,
			members(&[(10, 50), (11, 100), (12, 0)])
		));
		assert_eq!(Komite::trust_permille(&11), Some(1000));
		assert_eq!(Komite::trust_permille(&10), Some(500));
		assert_eq!(Komite::trust_permille(&12), Some(0));
		assert_eq!(Komite::trust_permille(&99), None);
		assert_ok!(Komite::set_committee(RuntimeOrigin::root(), 2, members(&[(20, 7)])));
		assert!(!Komite::is_member(&10));
		assert!(Komite::is_member(&20));
		assert_eq!(Komite::member_count(), 1);
		System::assert_last_event(Event::CommitteeNoted { era: 2, members: 1 }.into());
	});
}

#[test]
fn an_older_era_does_not_overwrite_a_newer_snapshot() {
	new_test_ext().execute_with(|| {
		assert_ok!(Komite::set_committee(RuntimeOrigin::root(), 5, members(&[(10, 1)])));
		assert_noop!(
			Komite::set_committee(RuntimeOrigin::root(), 4, members(&[(11, 1)])),
			Error::<Test>::StaleSnapshot
		);
		assert_eq!(Committee::<Test>::get().map(|s| s.era), Some(5));
	});
}

#[test]
fn an_all_zero_trust_snapshot_weights_nobody() {
	new_test_ext().execute_with(|| {
		assert_ok!(Komite::set_committee(RuntimeOrigin::root(), 1, members(&[(10, 0), (11, 0)])));
		assert_eq!(Komite::trust_permille(&10), Some(0));
		assert_eq!(Komite::trust_permille(&11), Some(0));
	});
}

#[test]
fn before_any_message_there_is_no_snapshot() {
	new_test_ext().execute_with(|| {
		assert!(!Komite::has_snapshot());
		assert_eq!(Komite::member_count(), 0);
		assert_eq!(Komite::trust_permille(&10), None);
	});
}

#[test]
fn work_is_weighed_by_the_committee_the_active_era_was_elected_under() {
	new_test_ext().execute_with(|| {
		assert_ok!(Komite::set_committee(RuntimeOrigin::root(), 1, members(&[(10, 2), (11, 1)])));
		// Before any era is elected under a committee, the latest one weighs.
		assert_eq!(Komite::weighing_snapshot().map(|s| s.era), Some(1));
		Komite::note_era_planned();
		assert_ok!(Komite::set_committee(RuntimeOrigin::root(), 2, members(&[(12, 1)])));
		Komite::note_era_activated();
		// The era started was planned under committee 1, not the newer 2.
		assert_eq!(Komite::weighing_snapshot().map(|s| s.era), Some(1));
		assert!(crate::PlannedCommittee::<Test>::get().is_none());
		Komite::note_era_planned();
		Komite::note_era_activated();
		assert_eq!(Komite::weighing_snapshot().map(|s| s.era), Some(2));
	});
}
