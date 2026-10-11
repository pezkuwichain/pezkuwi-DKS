// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::imports::*;

use codec::Encode;
use people_pezkuwichain_runtime::people::IdentityInfo;
use pezframe_support::{
	assert_err, pezsp_runtime::traits::Dispatchable, pezsp_runtime::DispatchError,
};
use pezkuwichain_system_emulated_network::pezkuwichain_emulated_chain::pezkuwichain_runtime::Dmp;
use teyrchains_common::AccountId;

use pezpallet_identity::Data;

use emulated_integration_tests_common::accounts::{ALICE, BOB};

#[test]
fn relay_commands_add_registrar() {
	let (origin_kind, origin) =
		(OriginKind::Superuser, <Pezkuwichain as Chain>::RuntimeOrigin::root());

	let registrar: AccountId = [1; 32].into();
	Pezkuwichain::execute_with(|| {
		type Runtime = <Pezkuwichain as Chain>::Runtime;
		type RuntimeCall = <Pezkuwichain as Chain>::RuntimeCall;
		type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
		type PeopleCall = <PeoplePezkuwichain as Chain>::RuntimeCall;
		type PeopleRuntime = <PeoplePezkuwichain as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let add_registrar_call =
			PeopleCall::Identity(pezpallet_identity::Call::<PeopleRuntime>::add_registrar {
				account: registrar.into(),
			});

		let xcm_message = RuntimeCall::XcmPallet(pezpallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Teyrchain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind,
					call: add_registrar_call.encode().into(),
					fallback_max_weight: None
				}
			]))),
		});

		assert_ok!(xcm_message.dispatch(origin));

		assert_expected_events!(
			Pezkuwichain,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeoplePezkuwichain::execute_with(|| {
		type RuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeoplePezkuwichain,
			vec![
				RuntimeEvent::Identity(pezpallet_identity::Event::RegistrarAdded { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn relay_commands_kill_identity() {
	// To kill an identity, first one must be set
	PeoplePezkuwichain::execute_with(|| {
		type PeopleRuntime = <PeoplePezkuwichain as Chain>::Runtime;
		type PeopleRuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;

		let people_pezkuwichain_alice = <PeoplePezkuwichain as Chain>::RuntimeOrigin::signed(
			PeoplePezkuwichain::account_id_of(ALICE),
		);

		let identity_info = IdentityInfo {
			email: Data::Raw(b"test@test.io".to_vec().try_into().unwrap()),
			..Default::default()
		};
		let identity: Box<<PeopleRuntime as pezpallet_identity::Config>::IdentityInformation> =
			Box::new(identity_info);

		assert_ok!(<PeoplePezkuwichain as PeoplePezkuwichainPallet>::Identity::set_identity(
			people_pezkuwichain_alice,
			identity
		));

		assert_expected_events!(
			PeoplePezkuwichain,
			vec![
				PeopleRuntimeEvent::Identity(pezpallet_identity::Event::IdentitySet { .. }) => {},
			]
		);
	});

	let (origin_kind, origin) =
		(OriginKind::Superuser, <Pezkuwichain as Chain>::RuntimeOrigin::root());

	Pezkuwichain::execute_with(|| {
		type Runtime = <Pezkuwichain as Chain>::Runtime;
		type RuntimeCall = <Pezkuwichain as Chain>::RuntimeCall;
		type PeopleCall = <PeoplePezkuwichain as Chain>::RuntimeCall;
		type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
		type PeopleRuntime = <PeoplePezkuwichain as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let kill_identity_call =
			PeopleCall::Identity(pezpallet_identity::Call::<PeopleRuntime>::kill_identity {
				target: people_pezkuwichain_runtime::MultiAddress::Id(
					PeoplePezkuwichain::account_id_of(ALICE),
				),
			});

		let xcm_message = RuntimeCall::XcmPallet(pezpallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Teyrchain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind,
					call: kill_identity_call.encode().into(),
					fallback_max_weight: None
				}
			]))),
		});

		assert_ok!(xcm_message.dispatch(origin));

		assert_expected_events!(
			Pezkuwichain,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeoplePezkuwichain::execute_with(|| {
		type RuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeoplePezkuwichain,
			vec![
				RuntimeEvent::Identity(pezpallet_identity::Event::IdentityKilled { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn relay_commands_add_remove_username_authority() {
	let people_pezkuwichain_alice = PeoplePezkuwichain::account_id_of(ALICE);
	let people_pezkuwichain_bob = PeoplePezkuwichain::account_id_of(BOB);

	let (origin_kind, origin, usr) =
		(OriginKind::Superuser, <Pezkuwichain as Chain>::RuntimeOrigin::root(), "rootusername");

	// First, add a username authority.
	Pezkuwichain::execute_with(|| {
		type Runtime = <Pezkuwichain as Chain>::Runtime;
		type RuntimeCall = <Pezkuwichain as Chain>::RuntimeCall;
		type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
		type PeopleCall = <PeoplePezkuwichain as Chain>::RuntimeCall;
		type PeopleRuntime = <PeoplePezkuwichain as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let add_username_authority = PeopleCall::Identity(pezpallet_identity::Call::<
			PeopleRuntime,
		>::add_username_authority {
			authority: people_pezkuwichain_runtime::MultiAddress::Id(
				people_pezkuwichain_alice.clone(),
			),
			suffix: b"suffix1".into(),
			allocation: 10,
		});

		let add_authority_xcm_msg = RuntimeCall::XcmPallet(pezpallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Teyrchain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind,
					call: add_username_authority.encode().into(),
					fallback_max_weight: None
				}
			]))),
		});

		assert_ok!(add_authority_xcm_msg.dispatch(origin.clone()));

		assert_expected_events!(
			Pezkuwichain,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	// Check events system-teyrchain-side
	PeoplePezkuwichain::execute_with(|| {
		type RuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeoplePezkuwichain,
			vec![
				RuntimeEvent::Identity(pezpallet_identity::Event::AuthorityAdded { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	// Now, use the previously added username authority to concede a username to an account.
	PeoplePezkuwichain::execute_with(|| {
		type PeopleRuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;
		let full_username = [usr.to_owned(), ".suffix1".to_owned()].concat().into_bytes();

		assert_ok!(<PeoplePezkuwichain as PeoplePezkuwichainPallet>::Identity::set_username_for(
			<PeoplePezkuwichain as Chain>::RuntimeOrigin::signed(people_pezkuwichain_alice.clone()),
			people_pezkuwichain_runtime::MultiAddress::Id(people_pezkuwichain_bob.clone()),
			full_username,
			None,
			true
		));

		assert_expected_events!(
			PeoplePezkuwichain,
			vec![
				PeopleRuntimeEvent::Identity(pezpallet_identity::Event::UsernameQueued { .. }) => {},
			]
		);
	});

	// Accept the given username
	PeoplePezkuwichain::execute_with(|| {
		type PeopleRuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;
		let full_username = [usr.to_owned(), ".suffix1".to_owned()].concat().into_bytes();

		assert_ok!(<PeoplePezkuwichain as PeoplePezkuwichainPallet>::Identity::accept_username(
			<PeoplePezkuwichain as Chain>::RuntimeOrigin::signed(people_pezkuwichain_bob.clone()),
			full_username.try_into().unwrap(),
		));

		assert_expected_events!(
			PeoplePezkuwichain,
			vec![
				PeopleRuntimeEvent::Identity(pezpallet_identity::Event::UsernameSet { .. }) => {},
			]
		);
	});

	// Now, remove the username authority with another privileged XCM call.
	Pezkuwichain::execute_with(|| {
		type Runtime = <Pezkuwichain as Chain>::Runtime;
		type RuntimeCall = <Pezkuwichain as Chain>::RuntimeCall;
		type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
		type PeopleCall = <PeoplePezkuwichain as Chain>::RuntimeCall;
		type PeopleRuntime = <PeoplePezkuwichain as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let remove_username_authority = PeopleCall::Identity(pezpallet_identity::Call::<
			PeopleRuntime,
		>::remove_username_authority {
			authority: people_pezkuwichain_runtime::MultiAddress::Id(
				people_pezkuwichain_alice.clone(),
			),
			suffix: b"suffix1".into(),
		});

		let remove_authority_xcm_msg =
			RuntimeCall::XcmPallet(pezpallet_xcm::Call::<Runtime>::send {
				dest: bx!(VersionedLocation::from(Location::new(0, [Teyrchain(1004)]))),
				message: bx!(VersionedXcm::from(Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind,
						call: remove_username_authority.encode().into(),
						fallback_max_weight: None
					}
				]))),
			});

		assert_ok!(remove_authority_xcm_msg.dispatch(origin));

		assert_expected_events!(
			Pezkuwichain,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	// Final event check.
	PeoplePezkuwichain::execute_with(|| {
		type RuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeoplePezkuwichain,
			vec![
				RuntimeEvent::Identity(pezpallet_identity::Event::AuthorityRemoved { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

/// A signed account on the relay cannot command this chain at all.
///
/// The payload is not what this depends on: the relay's `SendXcmOrigin` admits only the
/// `StakingAdmin` plurality and refuses a signed account before any message leaves, so nothing
/// about People is reached. One payload is enough to pin it.
#[test]
fn a_signed_account_cannot_send_this_chain_a_message() {
	let alice = PeoplePezkuwichain::account_id_of(ALICE);

	Pezkuwichain::execute_with(|| {
		type Runtime = <Pezkuwichain as Chain>::Runtime;
		type RuntimeCall = <Pezkuwichain as Chain>::RuntimeCall;
		type PeopleCall = <PeoplePezkuwichain as Chain>::RuntimeCall;
		type PeopleRuntime = <PeoplePezkuwichain as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let call = PeopleCall::Identity(pezpallet_identity::Call::<PeopleRuntime>::add_registrar {
			account: AccountId::from([1; 32]).into(),
		});

		let xcm_message = RuntimeCall::XcmPallet(pezpallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Teyrchain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					call: call.encode().into(),
					fallback_max_weight: None
				}
			]))),
		});

		assert_err!(
			xcm_message.dispatch(<Pezkuwichain as Chain>::RuntimeOrigin::signed(alice)),
			DispatchError::BadOrigin
		);
	});
}

/// The relay cannot write the register, even as Root.
///
/// This is the end-to-end half of the rule. The relay's referenda weigh tokens and this
/// chain's count heads, so a relay message must not be able to decide who is a citizen. Root
/// is what `ParentAsSuperuser` hands it and Root is what every register call asks for, so the
/// refusal cannot live in the origin -- FRAME's Root bypasses origin filters. It lives in
/// `SafeCallFilter`, which runs before the origin is resolved and refuses the *call*.
///
/// The message leaves the relay and arrives; what fails is its execution. Asserting that is
/// the point: a test that only checked the send would pass over a chain that accepted it.
#[test]
fn the_relay_cannot_write_the_register() {
	Pezkuwichain::execute_with(|| {
		type Runtime = <Pezkuwichain as Chain>::Runtime;
		type RuntimeCall = <Pezkuwichain as Chain>::RuntimeCall;
		type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
		type PeopleCall = <PeoplePezkuwichain as Chain>::RuntimeCall;
		type PeopleRuntime = <PeoplePezkuwichain as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let revoke = PeopleCall::IdentityKyc(
			pezpallet_identity_kyc::Call::<PeopleRuntime>::revoke_citizenship {
				who: PeoplePezkuwichain::account_id_of(BOB),
			},
		);

		let xcm_message = RuntimeCall::XcmPallet(pezpallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Teyrchain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Superuser,
					call: revoke.encode().into(),
					fallback_max_weight: None
				}
			]))),
		});

		assert_ok!(xcm_message.dispatch(<Pezkuwichain as Chain>::RuntimeOrigin::root()));

		assert_expected_events!(
			Pezkuwichain,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeoplePezkuwichain::execute_with(|| {
		type RuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeoplePezkuwichain,
			vec![
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed {
					success: false,
					..
				}) => {},
			]
		);
	});
}
