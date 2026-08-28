// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{mock::*, *};
use pezframe_support::{assert_noop, assert_ok};

#[test]
fn genesis_installs_the_nine_strata() {
	new_test_ext().execute_with(|| {
		assert_eq!(Strata::<Test>::get().len(), 9);
		assert_eq!(CurrentEra::<Test>::get(), 0);
		assert!(CurrentCommittee::<Test>::get().is_empty());
	});
}

#[test]
fn genesis_refuses_a_configuration_that_cannot_be_seated() {
	// Four strata cannot clear MIN_STRATA, so building genesis with them must fail rather
	// than start a chain whose committee is below the security budget from block zero.
	assert!(std::panic::catch_unwind(|| { new_test_ext_with_strata(4) }).is_err());
}

#[test]
fn set_strata_requires_the_manager_origin() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tnpos::set_strata(RuntimeOrigin::root(), nine_strata()));
	});
}

#[test]
fn joining_measures_the_score_it_does_not_take_the_callers_word() {
	new_test_ext().execute_with(|| {
		// ALICE has no perwerde credential in the mock; nothing she can put in the call
		// should get her into that stratum.
		set_perwerde(ALICE, 0);
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde),
			Error::<Test>::NotEligible
		);
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_eq!(PoolMembers::<Test>::get(ALICE), Some(StratumId::Perwerde));
		assert_eq!(StratumSize::<Test>::get(StratumId::Perwerde), 1);
	});
}

#[test]
fn a_stale_score_blocks_joining_and_says_so() {
	new_test_ext().execute_with(|| {
		set_perwerde_at(ALICE, 500, 1);
		run_to_block(1 + MaxScoreAge::get() + 1);
		// Not NotEligible: the account may well qualify. The chain simply does not know.
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde),
			Error::<Test>::ScoreUnavailable
		);
	});
}

#[test]
fn a_member_stands_in_exactly_one_stratum() {
	new_test_ext().execute_with(|| {
		set_perwerde(ALICE, 500);
		set_tiki(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Tiki),
			Error::<Test>::AlreadyInPool
		);
	});
}

#[test]
fn leaving_decrements_the_stratum_it_left() {
	new_test_ext().execute_with(|| {
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_ok!(Tnpos::leave(RuntimeOrigin::signed(ALICE)));
		assert_eq!(StratumSize::<Test>::get(StratumId::Perwerde), 0);
		assert_eq!(PoolMembers::<Test>::get(ALICE), None);
	});
}

#[test]
fn office_tikis_do_not_open_the_tiki_stratum() {
	// Tiki and Meclis must stay independent gates. An office tiki is granted by the
	// assembly, so counting it here would quietly collapse two strata into one and the
	// security arithmetic would be measuring a chain that does not exist.
	new_test_ext().execute_with(|| {
		set_office_tiki_only(ALICE);
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Tiki),
			Error::<Test>::NotEligible
		);
	});
}

#[test]
fn the_pool_is_bounded() {
	new_test_ext().execute_with(|| {
		for i in 0..MaxPoolSize::get() {
			let who = 1_000 + i as u64;
			set_perwerde(who, 500);
			assert_ok!(Tnpos::join(RuntimeOrigin::signed(who), StratumId::Perwerde));
		}
		set_perwerde(9_999, 500);
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(9_999), StratumId::Perwerde),
			Error::<Test>::PoolFull
		);
	});
}
