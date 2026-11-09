// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{mock::*, *};
use pezframe_support::{assert_noop, assert_ok, traits::fungible::InspectHold};
use pezsp_core::hexdisplay::HexDisplay;
use pezsp_runtime::DispatchError;

type N = Navnisan;

fn top(label: &[u8]) -> NameHash {
	child_node(&kurd_node(), label)
}

fn cid(byte: u8) -> Cid {
	let mut v = vec![0xe3, 0x01, 0x01, 0x70, 0x12, 0x20];
	v.extend([byte; 32]);
	v.try_into().unwrap()
}

/// Commit, wait out the minimum age, register.
fn buy(who: AccountId, label: &[u8], years: u32) -> NameHash {
	let secret = [who as u8; 32];
	assert_ok!(N::commit(RuntimeOrigin::signed(who), commitment(label, &who, &who, &secret)));
	advance(60_000);
	assert_ok!(N::register(RuntimeOrigin::signed(who), label.to_vec(), who, secret, years));
	top(label)
}

fn check() {
	N::do_try_state().unwrap();
}

// --- names and keys -----------------------------------------------------------------------

/// Wallets and the extension compute the same nodes; these vectors are what they test against.
#[test]
fn namehash_vectors_are_pinned() {
	let hex = |h: NameHash| format!("{}", HexDisplay::from(&h.0));
	assert_eq!(
		hex(kurd_node()),
		"011fc4e97a5ef275bd399ccbe8fca90e5f7a907c010663259364a9fb4b1d64d9"
	);
	assert_eq!(
		hex(top(b"ahmet")),
		"a10d2f1372dde3c6b1853b3f14f883fdd6a3ca08944d24e85b56c71249441ed5"
	);
	assert_eq!(
		hex(top(b"gov")),
		"c5c21a8e620900b2c976bc0919a91167b427de09f025bb24bf46f91a39a62907"
	);
	assert_eq!(
		hex(child_node(&top(b"gov"), b"dks")),
		"3569b4cc564de96e00781bee7ad6cda51a65006529208b32e99d3be61ac7708e"
	);
	assert_eq!(N::node_of(b"dks.gov.kurd"), Some(child_node(&top(b"gov"), b"dks")));
	assert_eq!(N::node_of(b"uni-x--edu.kurd"), None);
	assert_eq!(N::node_of(b"dks.gov.com"), None);
}

#[test]
fn genesis_writes_the_state_tree() {
	new_test_ext().execute_with(|| {
		let gov = Names::<Test>::get(top(b"gov")).unwrap();
		assert_eq!(gov.owner, Owner::Governance);
		assert_eq!(gov.kind, Kind::Granted);
		let darayi = Names::<Test>::get(child_node(&top(b"gov"), b"darayi")).unwrap();
		assert_eq!(darayi.owner, Owner::Office(FINANCE));
		assert!(darayi.locked_transfer);
		assert_eq!(darayi.account, Some(MINISTER));
		assert!(Locked::<Test>::contains_key(top(b"serok")));
		assert_eq!(Prices::<Test>::get(), PRICES);
		check();
	});
}

// --- buying -------------------------------------------------------------------------------

#[test]
fn buying_pays_rent_by_length_into_the_treasury() {
	new_test_ext().execute_with(|| {
		let before = Balances::free_balance(TREASURY);
		let node = buy(ALICE, b"ahmet", 2);
		assert_eq!(Balances::free_balance(TREASURY) - before, 20);
		let rec = Names::<Test>::get(node).unwrap();
		assert_eq!(rec.kind, Kind::Leased);
		assert_eq!(rec.account, Some(ALICE));
		assert_eq!(rec.expiry, Some(now() + 2 * YEAR));
		buy(BOB, b"abc", 1);
		assert_eq!(Balances::free_balance(TREASURY) - before, 520);
		// The commitment's deposit came back.
		assert_eq!(Balances::balance_on_hold(&HoldReason::Commitment.into(), &ALICE), 0);
		check();
	});
}

#[test]
fn a_reveal_cannot_come_early_late_or_from_someone_else() {
	new_test_ext().execute_with(|| {
		let secret = [9u8; 32];
		assert_ok!(N::commit(
			RuntimeOrigin::signed(ALICE),
			commitment(b"ahmet", &ALICE, &ALICE, &secret)
		));
		assert_noop!(
			N::register(RuntimeOrigin::signed(ALICE), b"ahmet".to_vec(), ALICE, secret, 1),
			Error::<Test>::CommitmentTooNew
		);
		advance(60_000);
		// Bob saw the reveal: his own account is inside the hash, so it is not his commitment.
		assert_noop!(
			N::register(RuntimeOrigin::signed(BOB), b"ahmet".to_vec(), ALICE, secret, 1),
			Error::<Test>::NoCommitment
		);
		advance(MILLIS_PER_DAY);
		assert_noop!(
			N::register(RuntimeOrigin::signed(ALICE), b"ahmet".to_vec(), ALICE, secret, 1),
			Error::<Test>::CommitmentTooOld
		);
		let c = commitment(b"ahmet", &ALICE, &ALICE, &secret);
		assert_ok!(N::remove_commitment(RuntimeOrigin::signed(CAROL), c));
		assert_eq!(Balances::balance_on_hold(&HoldReason::Commitment.into(), &ALICE), 0);
	});
}

#[test]
fn labels_that_cannot_be_bought() {
	new_test_ext().execute_with(|| {
		let try_buy = |label: &[u8]| {
			let secret = [1u8; 32];
			let c = commitment(label, &ALICE, &ALICE, &secret);
			let _ = N::commit(RuntimeOrigin::signed(ALICE), c);
			advance(60_000);
			N::register(RuntimeOrigin::signed(ALICE), label.to_vec(), ALICE, secret, 1)
		};
		assert_eq!(try_buy(b"ab"), Err(Error::<Test>::TooShortToBuy.into()));
		assert_eq!(try_buy(b"uni-x--edu"), Err(Error::<Test>::BadLabel.into()));
		assert_eq!(try_buy(b"Serok"), Err(Error::<Test>::BadLabel.into()));
		assert_eq!(try_buy(b"serok"), Err(Error::<Test>::LabelLocked.into()));
		assert_eq!(try_buy(b"gov"), Err(Error::<Test>::NameTaken.into()));
	});
}

#[test]
fn years_and_horizon_are_bounded() {
	new_test_ext().execute_with(|| {
		let secret = [1u8; 32];
		assert_ok!(N::commit(
			RuntimeOrigin::signed(ALICE),
			commitment(b"ahmet", &ALICE, &ALICE, &secret)
		));
		advance(60_000);
		assert_noop!(
			N::register(RuntimeOrigin::signed(ALICE), b"ahmet".to_vec(), ALICE, secret, 6),
			Error::<Test>::BadYears
		);
		assert_ok!(N::register(RuntimeOrigin::signed(ALICE), b"ahmet".to_vec(), ALICE, secret, 5));
		assert_ok!(N::renew(RuntimeOrigin::signed(BOB), top(b"ahmet"), 5));
		assert_noop!(
			N::renew(RuntimeOrigin::signed(ALICE), top(b"ahmet"), 1),
			Error::<Test>::BeyondHorizon
		);
	});
}

// --- lifetime -----------------------------------------------------------------------------

#[test]
fn a_lapsed_lease_keeps_resolving_through_grace_then_frees() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"ahmet", 1);
		assert_ok!(N::set_subname(RuntimeOrigin::signed(ALICE), node, b"blog".to_vec(), BOB, None));
		let blog = child_node(&node, b"blog");
		advance(YEAR);
		let rec = Names::<Test>::get(node).unwrap();
		assert_eq!(N::effective_status(&rec, now()), Some(Status::Grace));
		assert!(N::resolves_to(node, &ALICE));
		assert_noop!(N::renew(RuntimeOrigin::signed(BOB), node, 1), Error::<Test>::GraceOwnerOnly);
		assert_noop!(N::reap(RuntimeOrigin::signed(BOB), node), Error::<Test>::NotStale);

		advance(30 * MILLIS_PER_DAY);
		assert!(!N::resolves_to(node, &ALICE));
		// Somebody else may now buy it, and the old owner's subname dies with the old lease.
		buy(BOB, b"ahmet", 1);
		let blog_rec = Names::<Test>::get(blog).unwrap();
		assert_eq!(N::effective_status(&blog_rec, now()), None);
		assert_ok!(N::reap(RuntimeOrigin::signed(CAROL), blog));
		assert_eq!(Balances::balance_on_hold(&HoldReason::Subname.into(), &ALICE), 0);
		check();
	});
}

#[test]
fn the_last_owner_renews_in_grace() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"ahmet", 1);
		let end = now() + YEAR;
		advance(YEAR + MILLIS_PER_DAY);
		assert_ok!(N::renew(RuntimeOrigin::signed(ALICE), node, 1));
		// Renewal counts from now when the lease has already ended.
		assert_eq!(Names::<Test>::get(node).unwrap().expiry, Some(now() + YEAR));
		assert!(now() + YEAR > end + YEAR);
	});
}

// --- transfer -----------------------------------------------------------------------------

#[test]
fn a_bought_name_moves_with_its_account_record() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"ahmet", 1);
		assert_ok!(N::set_primary(RuntimeOrigin::signed(ALICE), node));
		assert_ok!(N::transfer(RuntimeOrigin::signed(ALICE), node, BOB));
		let rec = Names::<Test>::get(node).unwrap();
		assert_eq!(rec.owner, Owner::Account(BOB));
		assert_eq!(rec.account, Some(BOB));
		// Alice's primary name is now stale; anyone may clear it.
		assert_ok!(N::reap_primary(RuntimeOrigin::signed(CAROL), ALICE));
		assert_noop!(
			N::transfer(RuntimeOrigin::signed(ALICE), node, CAROL),
			Error::<Test>::NotManager
		);
	});
}

#[test]
fn nothing_in_a_granted_tree_moves() {
	new_test_ext().execute_with(|| {
		let darayi = child_node(&top(b"gov"), b"darayi");
		assert_noop!(
			N::transfer(RuntimeOrigin::signed(MINISTER), darayi, BOB),
			Error::<Test>::NotTransferable
		);
		assert_ok!(N::grant(RuntimeOrigin::root(), None, b"dks".to_vec(), Owner::Account(CAROL)));
		assert_noop!(
			N::transfer(RuntimeOrigin::signed(CAROL), top(b"dks"), BOB),
			Error::<Test>::NotTransferable
		);
		assert_ok!(N::grant(
			RuntimeOrigin::root(),
			Some(top(b"dks")),
			b"haber".to_vec(),
			Owner::Account(BOB)
		));
		assert_noop!(
			N::transfer(RuntimeOrigin::signed(BOB), child_node(&top(b"dks"), b"haber"), ALICE),
			Error::<Test>::NotTransferable
		);
		check();
	});
}

// --- institutions -------------------------------------------------------------------------

#[test]
fn an_institution_changes_its_site_only_after_the_delay() {
	new_test_ext().execute_with(|| {
		let darayi = child_node(&top(b"gov"), b"darayi");
		assert_ok!(N::set_content(
			RuntimeOrigin::signed(MINISTER),
			darayi,
			Some(Content::Ipfs(cid(1)))
		));
		assert_eq!(Names::<Test>::get(darayi).unwrap().content, None);
		assert_noop!(
			N::enact_change(RuntimeOrigin::signed(CAROL), darayi, 1),
			Error::<Test>::NotDue
		);
		advance(MILLIS_PER_DAY);
		assert_ok!(N::enact_change(RuntimeOrigin::signed(CAROL), darayi, 1));
		assert_eq!(Names::<Test>::get(darayi).unwrap().content, Some(Content::Ipfs(cid(1))));
	});
}

#[test]
fn parliament_stops_a_stolen_ministers_key() {
	new_test_ext().execute_with(|| {
		let darayi = child_node(&top(b"gov"), b"darayi");
		assert_ok!(N::set_content(
			RuntimeOrigin::signed(MINISTER),
			darayi,
			Some(Content::Ipfs(cid(6)))
		));
		assert_ok!(N::cancel_change(RuntimeOrigin::root(), darayi, 1));
		advance(MILLIS_PER_DAY);
		assert_noop!(
			N::enact_change(RuntimeOrigin::signed(CAROL), darayi, 1),
			Error::<Test>::NothingPending
		);
		// A minister cannot add or remove names under the ministry on their own either.
		assert_noop!(
			N::set_subname(RuntimeOrigin::signed(MINISTER), darayi, b"x".to_vec(), BOB, None),
			Error::<Test>::Institutional
		);
		assert_noop!(
			N::cancel_change(RuntimeOrigin::signed(MINISTER), darayi, 1),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn a_ministry_name_follows_the_office() {
	new_test_ext().execute_with(|| {
		let darayi = child_node(&top(b"gov"), b"darayi");
		set_holder(Some(BOB));
		assert_noop!(
			N::set_content(RuntimeOrigin::signed(MINISTER), darayi, Some(Content::Ipfs(cid(1)))),
			Error::<Test>::NotManager
		);
		assert_ok!(N::set_content(RuntimeOrigin::signed(BOB), darayi, Some(Content::Ipfs(cid(1)))));
		set_holder(None);
		assert_noop!(
			N::set_content(RuntimeOrigin::signed(BOB), darayi, None),
			Error::<Test>::NotManager
		);
	});
}

#[test]
fn only_single_seat_offices_own_names() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			N::grant(
				RuntimeOrigin::root(),
				Some(top(b"gov")),
				b"welati".to_vec(),
				Owner::Office(CITIZEN)
			),
			Error::<Test>::BadOffice
		);
		assert_ok!(N::grant(
			RuntimeOrigin::root(),
			Some(top(b"gov")),
			b"dks".to_vec(),
			Owner::Account(CAROL)
		));
		assert_noop!(
			N::grant(
				RuntimeOrigin::signed(ALICE),
				Some(top(b"gov")),
				b"x".to_vec(),
				Owner::Account(ALICE)
			),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn a_feed_moves_forward_by_its_anchorer_without_delay() {
	new_test_ext().execute_with(|| {
		assert_ok!(N::grant(RuntimeOrigin::root(), None, b"dks".to_vec(), Owner::Account(CAROL)));
		let dks = top(b"dks");
		let feed =
			Content::Feed { publisher: [5u8; 32], anchorer: ANCHORER, anchor: cid(1), seq: 1 };
		assert_ok!(N::force_change(RuntimeOrigin::root(), dks, Change::Content(Some(feed))));
		assert_ok!(N::anchor(RuntimeOrigin::signed(ANCHORER), dks, cid(2), 2));
		assert_noop!(
			N::anchor(RuntimeOrigin::signed(ANCHORER), dks, cid(3), 2),
			Error::<Test>::AnchorNotNewer
		);
		assert_noop!(
			N::anchor(RuntimeOrigin::signed(CAROL), dks, cid(3), 3),
			Error::<Test>::NotAnchorer
		);
		match Names::<Test>::get(dks).unwrap().content {
			Some(Content::Feed { anchor, seq, .. }) => {
				assert_eq!(seq, 2);
				assert_eq!(anchor, cid(2));
			},
			other => panic!("{other:?}"),
		}
	});
}

#[test]
fn junk_content_is_refused() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"ahmet", 1);
		let junk: Cid = vec![1, 2, 3].try_into().unwrap();
		assert_noop!(
			N::set_content(RuntimeOrigin::signed(ALICE), node, Some(Content::Ipfs(junk))),
			Error::<Test>::BadContent
		);
		assert_ok!(N::set_content(RuntimeOrigin::signed(ALICE), node, Some(Content::Ipfs(cid(1)))));
		// A bought name is not institutional: its change is immediate.
		assert_eq!(Names::<Test>::get(node).unwrap().content, Some(Content::Ipfs(cid(1))));
	});
}

// --- the court ----------------------------------------------------------------------------

#[test]
fn the_court_takes_back_a_bad_faith_name_after_a_week() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"wezaret", 1);
		let before = Balances::free_balance(TREASURY);
		// Parliament is not the court.
		assert_noop!(
			N::schedule_revocation(RuntimeOrigin::root(), node, H256::repeat_byte(1)),
			DispatchError::BadOrigin
		);
		assert_noop!(
			N::schedule_revocation(RuntimeOrigin::signed(BOB), node, H256::repeat_byte(1)),
			DispatchError::BadOrigin
		);
		assert_ok!(N::schedule_revocation(
			RuntimeOrigin::signed(COURT),
			node,
			H256::repeat_byte(1)
		));
		advance(6 * MILLIS_PER_DAY);
		assert_noop!(
			N::enact_revocation(RuntimeOrigin::signed(CAROL), node),
			Error::<Test>::NotDue
		);
		advance(MILLIS_PER_DAY);
		assert_ok!(N::enact_revocation(RuntimeOrigin::signed(CAROL), node));
		assert!(Names::<Test>::get(node).is_none());
		// No rent comes back.
		assert_eq!(Balances::free_balance(TREASURY), before);
		check();
	});
}

#[test]
fn a_revoked_label_is_locked_until_parliament_frees_it() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"satinal", 1);
		assert_ok!(N::schedule_revocation(RuntimeOrigin::signed(COURT), node, H256::zero()));
		advance(DisputeDelay::get());
		assert_ok!(N::enact_revocation(RuntimeOrigin::signed(CAROL), node));
		assert_eq!(Locked::<Test>::get(node), Some(genesis_set::category::REVOKED));

		// The same hands, under another account, the next minute
		let secret = [2u8; 32];
		assert_ok!(N::commit(
			RuntimeOrigin::signed(BOB),
			commitment(b"satinal", &BOB, &BOB, &secret)
		));
		advance(60_000);
		assert_noop!(
			N::register(RuntimeOrigin::signed(BOB), b"satinal".to_vec(), BOB, secret, 1),
			Error::<Test>::LabelLocked
		);

		assert_ok!(N::unlock(RuntimeOrigin::root(), node));
		assert_ok!(N::register(RuntimeOrigin::signed(BOB), b"satinal".to_vec(), BOB, secret, 1));
		check();
	});
}

// --- suspension ---------------------------------------------------------------------------

fn resolve(name: &[u8]) -> runtime_api::Resolution<AccountId, u8> {
	N::api_resolve(name).unwrap()
}

fn appoint_council() {
	assert_ok!(N::set_emergency_council(RuntimeOrigin::signed(SEROK), COUNCIL.to_vec()));
}

#[test]
fn the_court_suspends_a_name_which_then_points_nowhere_and_cannot_change() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"satinal", 1);
		assert_ok!(N::set_content(RuntimeOrigin::signed(ALICE), node, Some(Content::Ipfs(cid(1)))));
		let why = H256::repeat_byte(7);

		assert_noop!(
			N::suspend(RuntimeOrigin::signed(BOB), node, why, MILLIS_PER_DAY),
			DispatchError::BadOrigin
		);
		assert_noop!(
			N::suspend(RuntimeOrigin::signed(COURT), node, why, 0),
			Error::<Test>::BadDuration
		);
		assert_noop!(
			N::suspend(RuntimeOrigin::signed(COURT), node, why, MaxSuspension::get() + 1),
			Error::<Test>::BadDuration
		);
		assert_ok!(N::suspend(RuntimeOrigin::signed(COURT), node, why, 10 * MILLIS_PER_DAY));

		let r = resolve(b"satinal.kurd");
		assert!(r.suspended);
		assert_eq!((r.content, r.account), (None, None));
		assert!(!N::resolves_to(node, &ALICE));
		assert_noop!(
			N::set_content(RuntimeOrigin::signed(ALICE), node, Some(Content::Ipfs(cid(2)))),
			Error::<Test>::Suspended
		);
		assert_noop!(
			N::transfer(RuntimeOrigin::signed(ALICE), node, BOB),
			Error::<Test>::Suspended
		);
		assert_noop!(
			N::set_subname(RuntimeOrigin::signed(ALICE), node, b"x".to_vec(), BOB, None),
			Error::<Test>::Suspended
		);

		// It ends by itself
		advance(10 * MILLIS_PER_DAY);
		let r = resolve(b"satinal.kurd");
		assert!(!r.suspended);
		assert_eq!(r.content, Some(Content::Ipfs(cid(1))));
		assert_ok!(N::transfer(RuntimeOrigin::signed(ALICE), node, BOB));
		check();
	});
}

#[test]
fn a_suspended_name_takes_its_subnames_with_it() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"satinal", 1);
		assert_ok!(N::set_subname(
			RuntimeOrigin::signed(ALICE),
			node,
			b"dukan".to_vec(),
			BOB,
			None
		));
		let sub = child_node(&node, b"dukan");
		assert_ok!(N::suspend(RuntimeOrigin::signed(COURT), node, H256::zero(), MILLIS_PER_DAY));

		assert!(resolve(b"dukan.satinal.kurd").suspended);
		assert_noop!(
			N::set_content(RuntimeOrigin::signed(BOB), sub, Some(Content::Ipfs(cid(3)))),
			Error::<Test>::Suspended
		);
	});
}

#[test]
fn the_court_lifts_a_suspension() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"satinal", 1);
		assert_ok!(N::suspend(RuntimeOrigin::signed(COURT), node, H256::zero(), MILLIS_PER_DAY));
		assert_noop!(
			N::lift_suspension(RuntimeOrigin::signed(BOB), node),
			DispatchError::BadOrigin
		);
		assert_ok!(N::lift_suspension(RuntimeOrigin::signed(COURT), node));
		assert!(!resolve(b"satinal.kurd").suspended);
		assert_noop!(
			N::lift_suspension(RuntimeOrigin::signed(COURT), node),
			Error::<Test>::NothingPending
		);
	});
}

#[test]
fn institutional_names_answer_to_parliament_not_to_suspension() {
	new_test_ext().execute_with(|| {
		appoint_council();
		let darayi = child_node(&top(b"gov"), b"darayi");
		assert_noop!(
			N::suspend(RuntimeOrigin::signed(COURT), darayi, H256::zero(), MILLIS_PER_DAY),
			Error::<Test>::Institutional
		);
		assert_noop!(
			N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[0]), darayi, H256::zero()),
			Error::<Test>::Institutional
		);
	});
}

#[test]
fn the_serok_appoints_a_full_council_of_different_people() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			N::set_emergency_council(RuntimeOrigin::signed(COURT), COUNCIL.to_vec()),
			DispatchError::BadOrigin
		);
		assert_noop!(
			N::set_emergency_council(RuntimeOrigin::signed(SEROK), COUNCIL[..4].to_vec()),
			Error::<Test>::BadCouncil
		);
		let mut twice = COUNCIL.to_vec();
		twice[4] = twice[0];
		assert_noop!(
			N::set_emergency_council(RuntimeOrigin::signed(SEROK), twice),
			Error::<Test>::BadCouncil
		);
		appoint_council();
		assert_eq!(EmergencyCouncil::<Test>::get().into_inner(), COUNCIL.to_vec());
		// Dissolved
		assert_ok!(N::set_emergency_council(RuntimeOrigin::signed(SEROK), vec![]));
		assert!(EmergencyCouncil::<Test>::get().is_empty());
		check();
	});
}

#[test]
fn three_of_the_five_suspend_at_once_for_seventy_two_hours() {
	new_test_ext().execute_with(|| {
		appoint_council();
		let node = buy(ALICE, b"satinal", 1);
		let why = H256::repeat_byte(9);

		assert_noop!(
			N::emergency_suspend(RuntimeOrigin::signed(BOB), node, why),
			Error::<Test>::NotEmergencyMember
		);
		assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[0]), node, why));
		assert_noop!(
			N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[0]), node, why),
			Error::<Test>::AlreadyApproved
		);
		assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[1]), node, why));
		assert!(!resolve(b"satinal.kurd").suspended);
		assert_noop!(
			N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[2]), node, H256::repeat_byte(1)),
			Error::<Test>::OtherReason
		);
		assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[2]), node, why));

		let s = Suspensions::<Test>::get(node).unwrap();
		assert_eq!(
			(s.by, s.until, s.reason),
			(SuspendedBy::Emergency, now() + EmergencySuspension::get(), why)
		);
		assert!(resolve(b"satinal.kurd").suspended);

		advance(EmergencySuspension::get());
		assert!(!resolve(b"satinal.kurd").suspended);
		check();
	});
}

#[test]
fn the_council_suspends_a_registration_once() {
	new_test_ext().execute_with(|| {
		appoint_council();
		let node = buy(ALICE, b"satinal", 1);
		for m in &COUNCIL[..3] {
			assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(*m), node, H256::zero()));
		}
		assert_noop!(
			N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[3]), node, H256::zero()),
			Error::<Test>::EmergencyUsed
		);
		advance(EmergencySuspension::get());
		assert_noop!(
			N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[3]), node, H256::zero()),
			Error::<Test>::EmergencyUsed
		);
		// The court is not bound by it
		assert_ok!(N::suspend(RuntimeOrigin::signed(COURT), node, H256::zero(), MILLIS_PER_DAY));
	});
}

#[test]
fn the_court_lifts_or_replaces_an_emergency_suspension() {
	new_test_ext().execute_with(|| {
		appoint_council();
		let node = buy(ALICE, b"satinal", 1);
		for m in &COUNCIL[..3] {
			assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(*m), node, H256::zero()));
		}
		assert_ok!(N::suspend(
			RuntimeOrigin::signed(COURT),
			node,
			H256::zero(),
			20 * MILLIS_PER_DAY
		));
		assert_eq!(Suspensions::<Test>::get(node).unwrap().by, SuspendedBy::Court);
		advance(EmergencySuspension::get());
		assert!(resolve(b"satinal.kurd").suspended);

		assert_ok!(N::lift_suspension(RuntimeOrigin::signed(COURT), node));
		assert!(!resolve(b"satinal.kurd").suspended);
	});
}

#[test]
fn only_sitting_members_and_fresh_agreement_count() {
	new_test_ext().execute_with(|| {
		appoint_council();
		let node = buy(ALICE, b"satinal", 1);
		assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[0]), node, H256::zero()));
		assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[1]), node, H256::zero()));

		// The Serok replaces both who agreed; the third agreement is then the first that counts
		let next = vec![COUNCIL[2], COUNCIL[3], COUNCIL[4], 66, 67];
		assert_ok!(N::set_emergency_council(RuntimeOrigin::signed(SEROK), next));
		assert_noop!(
			N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[0]), node, H256::zero()),
			Error::<Test>::NotEmergencyMember
		);
		assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[2]), node, H256::zero()));
		assert_eq!(EmergencyCalls::<Test>::get(node).unwrap().approvals.len(), 1);

		// Agreement older than the suspension it would order has lapsed
		assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[3]), node, H256::zero()));
		advance(EmergencySuspension::get());
		assert_ok!(N::emergency_suspend(RuntimeOrigin::signed(COUNCIL[4]), node, H256::zero()));
		assert!(!resolve(b"satinal.kurd").suspended);
		assert_eq!(EmergencyCalls::<Test>::get(node).unwrap().approvals.len(), 1);
	});
}

#[test]
fn a_suspension_belongs_to_its_registration() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"satinal", 1);
		// Suspended on the last day of its grace period, for a month
		advance(YEAR + GracePeriod::get() - MILLIS_PER_DAY);
		assert_ok!(N::suspend(
			RuntimeOrigin::signed(COURT),
			node,
			H256::zero(),
			MaxSuspension::get()
		));
		// The lease lapses and someone else buys the name: the old suspension is not theirs
		advance(MILLIS_PER_DAY);
		let fresh = buy(BOB, b"satinal", 1);
		assert!(Suspensions::<Test>::get(node).is_some_and(|s| now() < s.until));
		assert!(!resolve(b"satinal.kurd").suspended);
		assert_ok!(N::set_content(RuntimeOrigin::signed(BOB), fresh, Some(Content::Ipfs(cid(4)))));
	});
}

#[test]
fn the_council_parameters_hold_together() {
	use pezframe_support::traits::Hooks;
	<N as Hooks<u64>>::integrity_test();
}

#[test]
fn a_name_under_scheduled_revocation_cannot_be_bought_out_from_under_it() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"wezaret", 1);
		assert_ok!(N::schedule_revocation(RuntimeOrigin::signed(COURT), node, H256::zero()));
		advance(YEAR + 31 * MILLIS_PER_DAY);
		let secret = [2u8; 32];
		assert_ok!(N::commit(
			RuntimeOrigin::signed(BOB),
			commitment(b"wezaret", &BOB, &BOB, &secret)
		));
		advance(60_000);
		assert_noop!(
			N::register(RuntimeOrigin::signed(BOB), b"wezaret".to_vec(), BOB, secret, 1),
			Error::<Test>::NameTaken
		);
	});
}

// --- records ------------------------------------------------------------------------------

#[test]
fn text_records_are_bounded_held_and_cleared_on_reregistration() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"ahmet", 1);
		for i in 0..label::MAX_TEXTS {
			let key: label::TextKey = format!("k{i}").into_bytes().try_into().unwrap();
			assert_ok!(N::set_text(
				RuntimeOrigin::signed(ALICE),
				node,
				key,
				Some(b"v".to_vec().try_into().unwrap())
			));
		}
		let extra: label::TextKey = b"extra".to_vec().try_into().unwrap();
		assert_noop!(
			N::set_text(
				RuntimeOrigin::signed(ALICE),
				node,
				extra,
				Some(b"v".to_vec().try_into().unwrap())
			),
			Error::<Test>::TooManyTexts
		);
		assert!(Balances::balance_on_hold(&HoldReason::Text.into(), &ALICE) > 0);
		advance(YEAR + 31 * MILLIS_PER_DAY);
		buy(BOB, b"ahmet", 1);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Text.into(), &ALICE), 0);
		assert_eq!(TextCount::<Test>::get(Names::<Test>::get(node).unwrap().reg), 0);
		check();
	});
}

#[test]
fn subnames_nest_to_the_depth_limit_and_no_further() {
	new_test_ext().execute_with(|| {
		let a = buy(ALICE, b"ahmet", 1);
		assert_ok!(N::set_subname(RuntimeOrigin::signed(ALICE), a, b"b".to_vec(), ALICE, None));
		let b = child_node(&a, b"b");
		assert_ok!(N::set_subname(RuntimeOrigin::signed(ALICE), b, b"c".to_vec(), ALICE, None));
		let c = child_node(&b, b"c");
		assert_ok!(N::set_subname(RuntimeOrigin::signed(ALICE), c, b"d".to_vec(), ALICE, None));
		let d = child_node(&c, b"d");
		assert_noop!(
			N::set_subname(RuntimeOrigin::signed(ALICE), d, b"e".to_vec(), ALICE, None),
			Error::<Test>::TooDeep
		);
		// The parent takes a subname back by replacing it.
		assert_ok!(N::set_subname(RuntimeOrigin::signed(ALICE), a, b"b".to_vec(), BOB, None));
		assert_eq!(N::effective_status(&Names::<Test>::get(c).unwrap(), now()), None);
		check();
	});
}

#[test]
fn prices_change_only_by_parliament_and_stay_ordered() {
	new_test_ext().execute_with(|| {
		let bad = PriceBands { len3: 1, len4: 100, len5plus: 10 };
		assert_noop!(N::set_prices(RuntimeOrigin::root(), bad), Error::<Test>::BadPrices);
		let good = PriceBands { len3: 900, len4: 90, len5plus: 9 };
		assert_noop!(N::set_prices(RuntimeOrigin::signed(ALICE), good), DispatchError::BadOrigin);
		assert_ok!(N::set_prices(RuntimeOrigin::root(), good));
		assert_eq!(N::price(5, 2), 18);
	});
}

#[test]
fn locking_stops_new_purchases_only() {
	new_test_ext().execute_with(|| {
		let node = buy(ALICE, b"kobani", 1);
		assert_ok!(N::lock(RuntimeOrigin::root(), b"kobani".to_vec(), 5));
		assert!(Names::<Test>::get(node).is_some());
		assert_ok!(N::unlock(RuntimeOrigin::root(), top(b"serok")));
		assert_noop!(N::lock(RuntimeOrigin::root(), b"a--b".to_vec(), 1), Error::<Test>::BadLabel);
	});
}

// --- what wallets ask -----------------------------------------------------------------------

#[test]
fn the_api_resolves_marks_official_and_reverses() {
	new_test_ext().execute_with(|| {
		use runtime_api::Availability as A;
		let node = buy(ALICE, b"ahmet", 1);
		assert_ok!(N::set_content(RuntimeOrigin::signed(ALICE), node, Some(Content::Ipfs(cid(4)))));
		let r = N::api_resolve(b"ahmet.kurd").unwrap();
		assert_eq!((r.account, r.official, r.status), (Some(ALICE), false, Status::Active));
		assert_eq!(r.content, Some(Content::Ipfs(cid(4))));
		let gov = N::api_resolve(b"darayi.gov.kurd").unwrap();
		assert!(gov.official);
		assert_eq!(N::api_resolve(b"nobody.kurd"), None);

		assert_ok!(N::set_primary(RuntimeOrigin::signed(ALICE), node));
		assert_eq!(N::api_reverse(&ALICE), Some(b"ahmet.kurd".to_vec()));
		assert_eq!(N::api_reverse(&BOB), None);
		assert_eq!(
			N::full_name(child_node(&top(b"gov"), b"darayi")),
			Some(b"darayi.gov.kurd".to_vec())
		);

		assert_eq!(N::api_availability(b"ahmet"), A::Taken);
		assert_eq!(N::api_availability(b"serok"), A::Locked);
		assert_eq!(N::api_availability(b"zz"), A::GrantOnly);
		assert_eq!(N::api_availability(b"a--b"), A::Invalid);
		assert_eq!(N::api_availability(b"freename"), A::Free);
		assert_eq!(N::api_price(b"abcd", 2), Some(200));
		advance(YEAR);
		assert_eq!(N::api_availability(b"ahmet"), A::Grace);
		assert_eq!(N::api_resolve(b"ahmet.kurd").unwrap().status, Status::Grace);
	});
}

// --- a chain that gains the pallet by upgrade ---------------------------------------------

struct TestSeed;
impl migrations::v1::SeedSet<Test> for TestSeed {
	fn zones() -> Vec<Vec<u8>> {
		vec![b"gov".to_vec()]
	}
	fn gov_subnames() -> Vec<(Vec<u8>, Option<u8>)> {
		vec![(b"darayi".to_vec(), Some(FINANCE))]
	}
	fn locked() -> Vec<(Vec<u8>, u8)> {
		vec![(b"dks".to_vec(), 2)]
	}
	fn prices() -> PriceBands<Balance> {
		PRICES
	}
}

// The order a live upgrade runs in: Executive stamps a newly added pallet with its in-code
// storage version first, and only then runs the runtime's migrations. A migration keyed on
// this pallet's 0 -> 1 never fires on that path, which is how Zagros People 1020023 came up
// with an empty register.
#[test]
fn the_seed_migration_runs_after_frame_stamps_the_new_pallet() {
	use pezframe_support::traits::{
		BeforeAllRuntimeMigrations, GetStorageVersion, OnRuntimeUpgrade,
	};
	use pezsp_runtime::BuildStorage;
	let mut ext: pezsp_io::TestExternalities = pezframe_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap()
		.into();
	ext.execute_with(|| {
		assert_eq!(N::on_chain_storage_version(), 0);
		AllPalletsWithSystem::before_all_runtime_migrations();
		assert_eq!(N::on_chain_storage_version(), 1);
		migrations::v1::SeedNavnisan::<Test, TestSeed>::on_runtime_upgrade();
		assert!(Names::<Test>::get(child_node(&top(b"gov"), b"darayi")).is_some());
		assert!(Locked::<Test>::contains_key(top(b"dks")));
		check();
	});
}

#[test]
fn the_seed_migration_writes_the_genesis_set_once() {
	use pezframe_support::traits::{GetStorageVersion, OnRuntimeUpgrade, StorageVersion};
	use pezsp_runtime::BuildStorage;
	let mut ext: pezsp_io::TestExternalities = pezframe_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap()
		.into();
	ext.execute_with(|| {
		StorageVersion::new(0).put::<N>();
		migrations::v1::SeedNavnisan::<Test, TestSeed>::on_runtime_upgrade();
		assert_eq!(N::on_chain_storage_version(), 1);
		assert!(Names::<Test>::get(child_node(&top(b"gov"), b"darayi")).is_some());
		assert!(Locked::<Test>::contains_key(top(b"dks")));
		let reg = NextRegistration::<Test>::get();
		// Run again: version 1 now, nothing written.
		migrations::v1::SeedNavnisan::<Test, TestSeed>::on_runtime_upgrade();
		assert_eq!(NextRegistration::<Test>::get(), reg);
		check();
	});
}
