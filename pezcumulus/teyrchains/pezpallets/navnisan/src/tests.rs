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
