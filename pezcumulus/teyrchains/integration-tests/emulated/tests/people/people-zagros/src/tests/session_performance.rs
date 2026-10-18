// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The relay tells the register who authored and who did not.
//!
//! The relay builds `Tnpos::note_session_performance` by hand -- People's pallet index and call
//! index, encoded on this side -- and sends it down at every session end, swallowing a failure
//! so the Asset Hub's copy still goes out. An encoding test holds the bytes; this holds the
//! behaviour: the message arrives, executes as root on People, and the register counts the
//! session for every validator the relay seated, scored or not. A wrong index or origin fails
//! quietly in production, because the relay never learns the message was refused.

use crate::imports::*;

use pezpallet_staking_async_rc_client::{SendToAssetHub, SessionReport};
use teyrchains_common::AccountId;
use zagros_system_emulated_network::zagros_emulated_chain::zagros_runtime::{
	Dmp, Session, StakingXcmToAssetHub,
};

#[test]
fn the_relay_tells_the_register_who_authored() {
	let (scored, failed): (Vec<AccountId>, Vec<AccountId>) = Zagros::execute_with(|| {
		Dmp::make_teyrchain_reachable(1004);
		let seated = Session::validators();
		// The emulated relay may seat a single validator; then nobody is absent, and the
		// register must still count the session for the one who sat it.
		let (author, absent) = seated.split_first().expect("the emulated relay seats a validator");
		let report = SessionReport {
			end_index: 1,
			validator_points: vec![(author.clone(), 20)],
			activation_timestamp: None,
			leftover: false,
		};
		// The Asset Hub half may or may not be deliverable here; the register's half is what
		// this asks about, and it is sent first and independently.
		let _ = StakingXcmToAssetHub::relay_session_report(report);
		(vec![author.clone()], absent.to_vec())
	});

	PeopleZagros::execute_with(|| {
		type RuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;
		type Runtime = <PeopleZagros as Chain>::Runtime;

		assert_expected_events!(
			PeopleZagros,
			vec![
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
		assert_eq!(pezpallet_tnpos::SessionsObserved::<Runtime>::get(), 1);
		for who in scored.iter().chain(failed.iter()) {
			assert_eq!(
				pezpallet_tnpos::SeatedSessions::<Runtime>::get(who),
				1,
				"{who:?} sat the session and the register did not count it"
			);
		}
	});
}
