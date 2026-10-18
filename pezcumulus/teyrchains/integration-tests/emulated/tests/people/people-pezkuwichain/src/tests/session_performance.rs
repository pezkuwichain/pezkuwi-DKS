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

use pezkuwichain_system_emulated_network::pezkuwichain_emulated_chain::pezkuwichain_runtime::{
	Dmp, Session, StakingXcmToAssetHub,
};
use pezpallet_staking_async_rc_client::{SendToAssetHub, SessionReport};
use teyrchains_common::AccountId;

#[test]
fn the_relay_tells_the_register_who_authored() {
	let (scored, failed): (Vec<AccountId>, Vec<AccountId>) = Pezkuwichain::execute_with(|| {
		Dmp::make_teyrchain_reachable(1004);
		let seated = Session::validators();
		assert!(seated.len() >= 2, "the emulated relay seats at least two validators");
		let (author, absent) = seated.split_first().expect("non-empty");
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

	PeoplePezkuwichain::execute_with(|| {
		type RuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;
		type Runtime = <PeoplePezkuwichain as Chain>::Runtime;

		assert_expected_events!(
			PeoplePezkuwichain,
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
