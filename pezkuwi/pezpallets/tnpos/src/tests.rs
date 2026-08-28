// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{mock::*, *};
use pezframe_support::{assert_noop, assert_ok};
use pezsp_io::hashing::blake2_256;

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

#[test]
fn a_healthy_pool_seats_twenty_seven_across_nine_strata() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let c = CurrentCommittee::<Test>::get();
		assert_eq!(c.len(), 27);
		let mut per = std::collections::BTreeMap::new();
		for who in c.iter() {
			*per.entry(PoolMembers::<Test>::get(who).unwrap()).or_insert(0) += 1;
		}
		assert_eq!(per.len(), 9);
		assert!(per.values().all(|&v| v == 3), "each stratum seats exactly three");
	});
}

#[test]
fn a_short_stratum_shrinks_the_committee_it_does_not_hand_its_seats_away() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		empty_stratum(StratumId::Tiki);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let c = CurrentCommittee::<Test>::get();
		assert_eq!(c.len(), 24, "three seats are lost, not moved");
		assert!(!c.iter().any(|w| PoolMembers::<Test>::get(w) == Some(StratumId::Tiki)));
	});
}

#[test]
fn seating_is_refused_rather_than_run_below_the_budget() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		for s in [
			StratumId::Tiki,
			StratumId::Divan,
			StratumId::Geography,
			StratumId::Tenure,
			StratumId::Infrastructure,
		] {
			empty_stratum(s);
		}
		let before = CurrentCommittee::<Test>::get();
		assert_noop!(
			Tnpos::force_new_era(RuntimeOrigin::root()),
			Error::<Test>::UnseatableConfiguration
		);
		assert_eq!(CurrentCommittee::<Test>::get(), before, "the old committee stays");
	});
}

#[test]
fn nobody_is_seated_twice() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let mut c = CurrentCommittee::<Test>::get().to_vec();
		let n = c.len();
		c.sort();
		c.dedup();
		assert_eq!(c.len(), n);
	});
}

#[test]
fn a_new_era_draws_a_different_committee() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(200);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let first = CurrentCommittee::<Test>::get();
		seed_the_era();
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		assert_ne!(CurrentCommittee::<Test>::get(), first);
	});
}

#[test]
fn the_era_advances_on_schedule() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		run_to_block(EraLength::get());
		assert_eq!(CurrentEra::<Test>::get(), 1);
	});
}

#[test]
fn a_refused_seating_does_not_retry_every_block() {
	// The pallet this replaces swallowed the error and left EraStart untouched, so it
	// re-ran the whole selection on every single block and paid full weight for it.
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		for s in [
			StratumId::Tiki,
			StratumId::Divan,
			StratumId::Geography,
			StratumId::Tenure,
			StratumId::Infrastructure,
		] {
			empty_stratum(s);
		}
		run_to_block(EraLength::get());
		// The window must have been written even though seating failed. Asserting only that
		// it is unchanged between two blocks would pass while it sat at genesis zero, which
		// is the exact bug this test exists for.
		assert_eq!(
			EraStart::<Test>::get(),
			EraLength::get(),
			"a failed seating must still move the era window"
		);
		run_to_block(EraLength::get() + 1);
		assert_eq!(
			EraStart::<Test>::get(),
			EraLength::get(),
			"and must not fire again on the very next block"
		);
	});
}

#[test]
fn a_revealed_seed_matches_its_commitment() {
	new_test_ext().execute_with(|| {
		join_pool(ALICE);
		let pre = [3u8; 32];
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&pre)));
		advance_to_reveal_window();
		assert_ok!(Tnpos::reveal_seed(RuntimeOrigin::signed(ALICE), pre));
		assert!(NextSeed::<Test>::get().is_some());
	});
}

#[test]
fn a_reveal_that_does_not_match_is_rejected() {
	new_test_ext().execute_with(|| {
		join_pool(ALICE);
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&[3u8; 32])));
		advance_to_reveal_window();
		assert_noop!(
			Tnpos::reveal_seed(RuntimeOrigin::signed(ALICE), [9u8; 32]),
			Error::<Test>::BadReveal
		);
	});
}

#[test]
fn one_honest_contributor_changes_the_seed() {
	// The property the whole scheme rests on: an adversary who reveals last still cannot
	// choose the result, because every contribution is mixed in. Both commits land inside
	// the commit half (the window is one shared deadline, not one per account), then both
	// reveals land after it closes.
	new_test_ext().execute_with(|| {
		join_pool(ALICE);
		join_pool(BOB);
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&[1u8; 32])));
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(BOB), blake2_256(&[2u8; 32])));
		advance_to_reveal_window();
		assert_ok!(Tnpos::reveal_seed(RuntimeOrigin::signed(ALICE), [1u8; 32]));
		let only_alice = NextSeed::<Test>::get().unwrap();
		assert_ok!(Tnpos::reveal_seed(RuntimeOrigin::signed(BOB), [2u8; 32]));
		assert_ne!(NextSeed::<Test>::get().unwrap(), only_alice);
	});
}

#[test]
fn seating_is_refused_when_no_seed_was_contributed() {
	// Falling back to a predictable seed would hand an adversary the draw. Refusing keeps
	// the previous committee, which is the safe direction.
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		clear_seed();
		assert_noop!(
			Tnpos::force_new_era(RuntimeOrigin::root()),
			Error::<Test>::UnseatableConfiguration
		);
	});
}

#[test]
fn only_pool_members_may_contribute() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tnpos::commit_seed(RuntimeOrigin::signed(9_999), blake2_256(&[1u8; 32])),
			Error::<Test>::NotInPool
		);
	});
}

#[test]
fn a_drawn_seed_is_spent_and_the_next_draw_needs_its_own_round() {
	// Every preimage is public the instant it is revealed, so carrying a spent seed into
	// another era would let anyone compute that era's committee in advance.
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		assert!(NextSeed::<Test>::get().is_none(), "the seed is spent once its era is drawn");
		assert_noop!(
			Tnpos::force_new_era(RuntimeOrigin::root()),
			Error::<Test>::UnseatableConfiguration
		);
	});
}

#[test]
fn an_account_may_commit_only_once_per_round() {
	// An unscoped commitment pot would let an account commit again after seeing what
	// others revealed -- not withholding, but steering.
	new_test_ext().execute_with(|| {
		join_pool(ALICE);
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&[1u8; 32])));
		assert_noop!(
			Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&[2u8; 32])),
			Error::<Test>::AlreadyCommitted
		);
	});
}

#[test]
fn committing_after_the_window_closes_is_rejected() {
	new_test_ext().execute_with(|| {
		join_pool(ALICE);
		advance_to_reveal_window();
		assert_noop!(
			Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&[1u8; 32])),
			Error::<Test>::CommitWindowClosed
		);
	});
}

#[test]
fn revealing_before_the_window_opens_is_rejected() {
	// Without this deadline a member could wait for everyone else to reveal, then commit
	// and reveal in the same block with a preimage chosen to land the seed where they want.
	new_test_ext().execute_with(|| {
		join_pool(ALICE);
		let pre = [1u8; 32];
		assert_ok!(Tnpos::commit_seed(RuntimeOrigin::signed(ALICE), blake2_256(&pre)));
		assert_noop!(
			Tnpos::reveal_seed(RuntimeOrigin::signed(ALICE), pre),
			Error::<Test>::RevealWindowNotOpen
		);
	});
}
