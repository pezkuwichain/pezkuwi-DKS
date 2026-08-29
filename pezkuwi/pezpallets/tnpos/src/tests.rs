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
		// should get her into that stratum. Keys are granted up front so this failure is
		// the score gate, not the session-keys gate underneath it.
		ensure_has_keys(ALICE);
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
		ensure_has_keys(ALICE);
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
		ensure_has_keys(ALICE);
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
		ensure_has_keys(ALICE);
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
		ensure_has_keys(ALICE);
		set_office_tiki_only(ALICE);
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Tiki),
			Error::<Test>::NotEligible
		);
	});
}

#[test]
fn joining_without_session_keys_is_refused() {
	// Session silently drops a keyless validator on rotation, so the join gate has to
	// catch this itself rather than let it surface later as an unexplained absence.
	// Nothing has keys by default, so ALICE is already in exactly the state this checks.
	new_test_ext().execute_with(|| {
		set_perwerde(ALICE, 500);
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde),
			Error::<Test>::NoSessionKeys
		);
	});
}

#[test]
fn an_account_that_loses_its_keys_after_joining_is_not_drawn() {
	// `join` only catches a keyless account at the door; an account that deregisters its
	// keys afterwards stays in `PoolMembers`, so the draw itself has to filter it too.
	// Stripping all but three of Perwerde's sixty members down to no keys makes this
	// deterministic: with exactly three candidates left, all three must be seated and
	// none of the other fifty-seven can be, regardless of the seed.
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		let perwerde: Vec<AccountId> = PoolMembers::<Test>::iter()
			.filter_map(|(w, s)| (s == StratumId::Perwerde).then_some(w))
			.collect();
		let (keep, drop) = perwerde.split_at(3);
		for &who in drop {
			remove_keys(who);
		}
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let committee = CurrentCommittee::<Test>::get();
		for &who in keep {
			assert!(committee.contains(&who), "a keyed account must be seated");
		}
		for &who in drop {
			assert!(!committee.contains(&who), "a keyless account must not be seated");
		}
	});
}

#[test]
fn a_stratum_short_on_keyed_candidates_refuses_the_era_rather_than_seating_short() {
	// `seat` judges a stratum seatable from `StratumSize`, which counts pool membership;
	// the draw filters that same pool by session keys. Leaving Perwerde only two keyed
	// candidates for its three seats keeps it counted as seatable while its real candidate
	// pool cannot fill it -- the whole era must be refused, not seated with a stratum short
	// of its announced size.
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		let perwerde: Vec<AccountId> = PoolMembers::<Test>::iter()
			.filter_map(|(w, s)| (s == StratumId::Perwerde).then_some(w))
			.collect();
		for &who in &perwerde[2..] {
			remove_keys(who);
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
fn the_pool_is_bounded() {
	new_test_ext().execute_with(|| {
		for i in 0..MaxPoolSize::get() {
			let who = 1_000 + i as u64;
			ensure_has_keys(who);
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

#[test]
fn an_offence_bans_the_member_from_the_pool() {
	new_test_ext().execute_with(|| {
		ensure_has_keys(ALICE);
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), ALICE, Offence::Equivocation));
		assert_eq!(PoolMembers::<Test>::get(ALICE), None);
		assert_eq!(StratumSize::<Test>::get(StratumId::Perwerde), 0);
		assert_eq!(Banned::<Test>::get(ALICE), Some(360));
	});
}

#[test]
fn a_banned_member_cannot_rejoin_until_the_ban_expires() {
	new_test_ext().execute_with(|| {
		ensure_has_keys(ALICE);
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), ALICE, Offence::Unavailable));
		assert_noop!(
			Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde),
			Error::<Test>::Banned
		);
		advance_eras(24);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
	});
}

#[test]
fn being_unavailable_costs_less_than_equivocating() {
	// Going offline is a failure; signing two conflicting blocks is an attack. The ladder
	// has to say so, or the deterrent for the thing that can fork the chain is the same as
	// the deterrent for a bad connection.
	new_test_ext().execute_with(|| {
		ensure_has_keys(ALICE);
		ensure_has_keys(BOB);
		set_perwerde(ALICE, 500);
		set_perwerde(BOB, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(BOB), StratumId::Perwerde));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), ALICE, Offence::Unavailable));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), BOB, Offence::Equivocation));
		assert!(Banned::<Test>::get(BOB) > Banned::<Test>::get(ALICE));
	});
}

#[test]
fn reporting_requires_the_manager_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tnpos::report_offence(RuntimeOrigin::signed(ALICE), BOB, Offence::Unavailable),
			pezsp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn an_offence_removes_the_member_from_the_seated_committee() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let victim = CurrentCommittee::<Test>::get()[0];
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), victim, Offence::Equivocation));
		assert!(!CurrentCommittee::<Test>::get().contains(&victim));
	});
}

#[test]
fn a_lesser_offence_cannot_shorten_a_longer_ban() {
	// Reports arrive from outside and in no guaranteed order. Recomputed from whichever
	// came last, a trivial report following an equivocation would cut the penalty from
	// three hundred and sixty eras to twenty-four.
	new_test_ext().execute_with(|| {
		ensure_has_keys(ALICE);
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), ALICE, Offence::Equivocation));
		let heavy = Banned::<Test>::get(ALICE);
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), ALICE, Offence::Unavailable));
		assert_eq!(Banned::<Test>::get(ALICE), heavy, "a ban may only ever lengthen");
	});
}

#[test]
fn an_offender_who_already_left_the_pool_still_leaves_the_committee() {
	// Leaving the pool does not unseat a member from the era they are already serving.
	// If punishment only reached pool members, an offender could call `leave` first and
	// keep counting towards quorum for the rest of the era.
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let victim = CurrentCommittee::<Test>::get()[0];
		assert_ok!(Tnpos::leave(RuntimeOrigin::signed(victim)));
		assert!(CurrentCommittee::<Test>::get().contains(&victim));
		assert_ok!(Tnpos::report_offence(RuntimeOrigin::root(), victim, Offence::Equivocation));
		assert!(!CurrentCommittee::<Test>::get().contains(&victim));
	});
}

#[test]
fn a_new_session_hands_over_the_seated_committee() {
	use pezpallet_session::SessionManager;

	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let handed = <Tnpos as SessionManager<u64>>::new_session(1).expect("a committee exists");
		assert_eq!(handed, CurrentCommittee::<Test>::get().to_vec());
		assert_eq!(handed.len(), 27);
	});
}

#[test]
fn no_committee_hands_over_nothing_rather_than_an_empty_set() {
	// Returning Some(vec![]) would tell session to install an empty authority set and stop
	// the chain. None means "keep the current one", which is the recoverable answer.
	use pezpallet_session::SessionManager;

	new_test_ext().execute_with(|| {
		assert!(<Tnpos as SessionManager<u64>>::new_session(1).is_none());
	});
}

#[test]
fn a_banned_member_is_never_handed_over() {
	use pezpallet_session::SessionManager;

	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));
		let victim = CurrentCommittee::<Test>::get()[0].clone();
		assert_ok!(Tnpos::report_offence(
			RuntimeOrigin::root(),
			victim.clone(),
			Offence::Equivocation
		));
		let handed = <Tnpos as SessionManager<u64>>::new_session(2).unwrap();
		assert!(!handed.contains(&victim));
		assert_eq!(handed.len(), 26);
	});
}

// ===== RELAY KEY REGISTER =====
//
// The pallet keeps the register of who holds relay session keys, and the relay keeps the keys
// consensus reads. Those are two writes and they must not come apart -- a local record of a key
// the relay never received would let a keyless account into the pool, and session drops a
// keyless validator silently on rotation.

/// Keys are recorded only if the relay accepted the message.
#[test]
fn a_key_the_relay_did_not_receive_is_not_recorded() {
	new_test_ext().execute_with(|| {
		let keys = mock_keys(ALICE);
		let proof = mock_proof(ALICE);

		SEND_FAILS.with(|f| *f.borrow_mut() = true);
		assert_noop!(
			Tnpos::set_relay_keys(RuntimeOrigin::signed(ALICE), keys.clone(), proof.clone()),
			Error::<Test>::CouldNotReachRelay
		);
		assert!(!RelayKeys::<Test>::contains_key(ALICE), "recorded a key that never arrived");

		SEND_FAILS.with(|f| *f.borrow_mut() = false);
		assert_ok!(Tnpos::set_relay_keys(RuntimeOrigin::signed(ALICE), keys, proof));
		assert!(RelayKeys::<Test>::contains_key(ALICE));
	});
}

/// Bytes that are not the relay's keys are refused before they are recorded.
#[test]
fn keys_that_would_not_decode_on_the_relay_are_refused_here() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tnpos::set_relay_keys(RuntimeOrigin::signed(ALICE), vec![0xff], vec![]),
			Error::<Test>::InvalidRelayKeys
		);
		assert!(!RelayKeys::<Test>::contains_key(ALICE));
	});
}

/// A proof for somebody else's account does not register keys for this one.
#[test]
fn the_proof_has_to_belong_to_the_account_offering_it() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tnpos::set_relay_keys(RuntimeOrigin::signed(ALICE), mock_keys(ALICE), mock_proof(BOB)),
			Error::<Test>::InvalidKeyOwnershipProof
		);
		assert!(!RelayKeys::<Test>::contains_key(ALICE));
	});
}

/// Withdrawing keys takes the member out of the pool in the same call.
///
/// A member without keys is a seat the session would silently drop, which is the reason `join`
/// refuses one. Leaving them in the pool would let the stratum counts claim a seat that cannot
/// be filled -- the same arithmetic error, arrived at from the other direction.
#[test]
fn purging_keys_also_leaves_the_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tnpos::set_relay_keys(
			RuntimeOrigin::signed(ALICE),
			mock_keys(ALICE),
			mock_proof(ALICE)
		));
		set_perwerde(ALICE, 500);
		assert_ok!(Tnpos::join(RuntimeOrigin::signed(ALICE), StratumId::Perwerde));
		assert_eq!(StratumSize::<Test>::get(StratumId::Perwerde), 1);

		assert_ok!(Tnpos::purge_relay_keys(RuntimeOrigin::signed(ALICE)));
		assert!(!RelayKeys::<Test>::contains_key(ALICE));
		assert!(!PoolMembers::<Test>::contains_key(ALICE));
		assert_eq!(StratumSize::<Test>::get(StratumId::Perwerde), 0, "a seat was left behind");
	});
}

// ===== EXPORT =====
//
// A committee is a list in storage until the chain that validates with it receives it. These
// hold the delivery, because a pallet that draws a committee nobody gets is a mechanism that
// was designed and never wired.

/// Seating a committee hands it to the relay, with the era it belongs to.
#[test]
fn a_seated_committee_is_handed_to_the_relay() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));

		let seated = CurrentCommittee::<Test>::get().to_vec();
		assert!(!seated.is_empty(), "nothing was seated, so the test proves nothing");

		let exported = EXPORTED.with(|e| e.borrow().clone());
		assert_eq!(exported.len(), 1, "the committee was seated but never offered to the relay");
		let (era, sent) = &exported[0];
		assert_eq!(*era, CurrentEra::<Test>::get());
		assert_eq!(sent, &seated, "the relay was sent a different set than the one seated");
	});
}

/// A delivery that fails is reported, and the era still moves.
///
/// The alternative -- refusing to seat because the message could not go out -- would let an
/// unreachable relay freeze this chain's era clock. What must not happen is silence: the
/// difference between "the relay has an old committee because delivery failed" and "because
/// nothing was drawn" has to be visible without reading the relay.
#[test]
fn a_committee_that_could_not_be_delivered_says_so() {
	new_test_ext().execute_with(|| {
		fill_every_stratum(60);
		SEND_FAILS.with(|f| *f.borrow_mut() = true);

		let era_before = CurrentEra::<Test>::get();
		assert_ok!(Tnpos::force_new_era(RuntimeOrigin::root()));

		assert!(CurrentEra::<Test>::get() > era_before, "a failed send froze the era clock");
		assert!(
			EXPORTED.with(|e| !e.borrow().is_empty()),
			"the committee was never offered, which is a different fault"
		);
		assert!(
			System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::Tnpos(Event::CommitteeCouldNotBeSent { .. })
			)),
			"a failed delivery must be visible on chain, not only in a log line"
		);
	});
}
