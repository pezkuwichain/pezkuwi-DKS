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
use people_zagros_runtime::people::IdentityInfo;
use pezframe_support::{
	assert_err, pezsp_runtime::traits::Dispatchable, pezsp_runtime::DispatchError,
};
use teyrchains_common::AccountId;
use zagros_runtime::Dmp;
use zagros_system_emulated_network::people_zagros_emulated_chain::people_zagros_runtime;

use pezpallet_identity::Data;

use emulated_integration_tests_common::accounts::{ALICE, BOB};

#[test]
fn relay_commands_add_registrar() {
	let (origin_kind, origin) = (OriginKind::Superuser, <Zagros as Chain>::RuntimeOrigin::root());

	let registrar: AccountId = [1; 32].into();
	Zagros::execute_with(|| {
		type Runtime = <Zagros as Chain>::Runtime;
		type RuntimeCall = <Zagros as Chain>::RuntimeCall;
		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		type PeopleCall = <PeopleZagros as Chain>::RuntimeCall;
		type PeopleRuntime = <PeopleZagros as Chain>::Runtime;

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
			Zagros,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeopleZagros::execute_with(|| {
		type RuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleZagros,
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
	PeopleZagros::execute_with(|| {
		type PeopleRuntime = <PeopleZagros as Chain>::Runtime;
		type PeopleRuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;

		let people_zagros_alice =
			<PeopleZagros as Chain>::RuntimeOrigin::signed(PeopleZagros::account_id_of(ALICE));

		let identity_info = IdentityInfo {
			email: Data::Raw(b"test@test.io".to_vec().try_into().unwrap()),
			..Default::default()
		};
		let identity: Box<<PeopleRuntime as pezpallet_identity::Config>::IdentityInformation> =
			Box::new(identity_info);

		assert_ok!(<PeopleZagros as PeopleZagrosPallet>::Identity::set_identity(
			people_zagros_alice,
			identity
		));

		assert_expected_events!(
			PeopleZagros,
			vec![
				PeopleRuntimeEvent::Identity(pezpallet_identity::Event::IdentitySet { .. }) => {},
			]
		);
	});

	let (origin_kind, origin) = (OriginKind::Superuser, <Zagros as Chain>::RuntimeOrigin::root());

	Zagros::execute_with(|| {
		type Runtime = <Zagros as Chain>::Runtime;
		type RuntimeCall = <Zagros as Chain>::RuntimeCall;
		type PeopleCall = <PeopleZagros as Chain>::RuntimeCall;
		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		type PeopleRuntime = <PeopleZagros as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let kill_identity_call =
			PeopleCall::Identity(pezpallet_identity::Call::<PeopleRuntime>::kill_identity {
				target: people_zagros_runtime::MultiAddress::Id(PeopleZagros::account_id_of(ALICE)),
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
			Zagros,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeopleZagros::execute_with(|| {
		type RuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleZagros,
			vec![
				RuntimeEvent::Identity(pezpallet_identity::Event::IdentityKilled { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn relay_commands_add_remove_username_authority() {
	let people_zagros_alice = PeopleZagros::account_id_of(ALICE);
	let people_zagros_bob = PeopleZagros::account_id_of(BOB);

	let (origin_kind, origin, usr) =
		(OriginKind::Superuser, <Zagros as Chain>::RuntimeOrigin::root(), "rootusername");

	// First, add a username authority.
	Zagros::execute_with(|| {
		type Runtime = <Zagros as Chain>::Runtime;
		type RuntimeCall = <Zagros as Chain>::RuntimeCall;
		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		type PeopleCall = <PeopleZagros as Chain>::RuntimeCall;
		type PeopleRuntime = <PeopleZagros as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let add_username_authority = PeopleCall::Identity(pezpallet_identity::Call::<
			PeopleRuntime,
		>::add_username_authority {
			authority: people_zagros_runtime::MultiAddress::Id(people_zagros_alice.clone()),
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
			Zagros,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	// Check events system-teyrchain-side
	PeopleZagros::execute_with(|| {
		type RuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleZagros,
			vec![
				RuntimeEvent::Identity(pezpallet_identity::Event::AuthorityAdded { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});

	// Now, use the previously added username authority to concede a username to an account.
	PeopleZagros::execute_with(|| {
		type PeopleRuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;
		let full_username = [usr.to_owned(), ".suffix1".to_owned()].concat().into_bytes();

		assert_ok!(<PeopleZagros as PeopleZagrosPallet>::Identity::set_username_for(
			<PeopleZagros as Chain>::RuntimeOrigin::signed(people_zagros_alice.clone()),
			people_zagros_runtime::MultiAddress::Id(people_zagros_bob.clone()),
			full_username,
			None,
			true
		));

		assert_expected_events!(
			PeopleZagros,
			vec![
				PeopleRuntimeEvent::Identity(pezpallet_identity::Event::UsernameQueued { .. }) => {},
			]
		);
	});

	// Accept the given username
	PeopleZagros::execute_with(|| {
		type PeopleRuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;
		let full_username = [usr.to_owned(), ".suffix1".to_owned()].concat().into_bytes();

		assert_ok!(<PeopleZagros as PeopleZagrosPallet>::Identity::accept_username(
			<PeopleZagros as Chain>::RuntimeOrigin::signed(people_zagros_bob.clone()),
			full_username.try_into().unwrap(),
		));

		assert_expected_events!(
			PeopleZagros,
			vec![
				PeopleRuntimeEvent::Identity(pezpallet_identity::Event::UsernameSet { .. }) => {},
			]
		);
	});

	// Now, remove the username authority with another privileged XCM call.
	Zagros::execute_with(|| {
		type Runtime = <Zagros as Chain>::Runtime;
		type RuntimeCall = <Zagros as Chain>::RuntimeCall;
		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		type PeopleCall = <PeopleZagros as Chain>::RuntimeCall;
		type PeopleRuntime = <PeopleZagros as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let remove_username_authority = PeopleCall::Identity(pezpallet_identity::Call::<
			PeopleRuntime,
		>::remove_username_authority {
			authority: people_zagros_runtime::MultiAddress::Id(people_zagros_alice.clone()),
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
			Zagros,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	// Final event check.
	PeopleZagros::execute_with(|| {
		type RuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleZagros,
			vec![
				RuntimeEvent::Identity(pezpallet_identity::Event::AuthorityRemoved { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

/// A signed account on the relay cannot command this chain at all.
///
/// Three tests used to assert this, one per payload, and each of them also carried a case for
/// the relay's `WelatiAdmin` origin -- which no longer exists. The payload was never what the
/// fact depended on: the relay's `SendXcmOrigin` refuses a signed account before any message
/// leaves, so nothing about People is reached to be tested three times.
#[test]
fn a_signed_account_cannot_send_this_chain_a_message() {
	let alice = PeopleZagros::account_id_of(ALICE);

	Zagros::execute_with(|| {
		type Runtime = <Zagros as Chain>::Runtime;
		type RuntimeCall = <Zagros as Chain>::RuntimeCall;
		type PeopleCall = <PeopleZagros as Chain>::RuntimeCall;
		type PeopleRuntime = <PeopleZagros as Chain>::Runtime;

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
			xcm_message.dispatch(<Zagros as Chain>::RuntimeOrigin::signed(alice)),
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
	Zagros::execute_with(|| {
		type Runtime = <Zagros as Chain>::Runtime;
		type RuntimeCall = <Zagros as Chain>::RuntimeCall;
		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		type PeopleCall = <PeopleZagros as Chain>::RuntimeCall;
		type PeopleRuntime = <PeopleZagros as Chain>::Runtime;

		Dmp::make_teyrchain_reachable(1004);

		let revoke = PeopleCall::IdentityKyc(
			pezpallet_identity_kyc::Call::<PeopleRuntime>::revoke_citizenship {
				who: PeopleZagros::account_id_of(BOB),
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

		assert_ok!(xcm_message.dispatch(<Zagros as Chain>::RuntimeOrigin::root()));

		assert_expected_events!(
			Zagros,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeopleZagros::execute_with(|| {
		type RuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleZagros,
			vec![
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed {
					success: false,
					..
				}) => {},
			]
		);
	});
}
