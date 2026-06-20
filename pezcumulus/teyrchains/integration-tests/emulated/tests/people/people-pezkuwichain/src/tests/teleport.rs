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
use emulated_integration_tests_common::{
	test_relay_is_trusted_teleporter, test_teyrchain_is_trusted_teleporter,
	test_teyrchain_is_trusted_teleporter_for_relay,
};

#[test]
fn teleport_via_limited_teleport_assets_from_and_to_relay() {
	let amount = PEZKUWICHAIN_ED * 100;

	test_relay_is_trusted_teleporter!(
		Pezkuwichain,
		vec![PeoplePezkuwichain],
		amount,
		limited_teleport_assets
	);

	test_teyrchain_is_trusted_teleporter_for_relay!(
		PeoplePezkuwichain,
		Pezkuwichain,
		amount,
		limited_teleport_assets
	);
}

#[test]
fn teleport_via_transfer_assets_from_and_to_relay() {
	let amount = PEZKUWICHAIN_ED * 100;

	test_relay_is_trusted_teleporter!(
		Pezkuwichain,
		vec![PeoplePezkuwichain],
		amount,
		transfer_assets
	);

	test_teyrchain_is_trusted_teleporter_for_relay!(
		PeoplePezkuwichain,
		Pezkuwichain,
		amount,
		transfer_assets
	);
}

#[test]
fn teleport_via_limited_teleport_assets_to_other_system_teyrchains_works() {
	let amount = PEZKUWICHAIN_ED * 100;
	let native_asset: Assets = (Parent, amount).into();

	let fee_asset_id: AssetId = Parent.into();
	test_teyrchain_is_trusted_teleporter!(
		PeoplePezkuwichain,         // Origin
		vec![AssetHubPezkuwichain], // Destinations
		(native_asset, amount),
		fee_asset_id,
		limited_teleport_assets
	);
}

#[test]
fn teleport_via_transfer_assets_to_other_system_teyrchains_works() {
	let amount = PEZKUWICHAIN_ED * 100;
	let native_asset: Assets = (Parent, amount).into();

	let fee_asset_id: AssetId = Parent.into();
	test_teyrchain_is_trusted_teleporter!(
		PeoplePezkuwichain,         // Origin
		vec![AssetHubPezkuwichain], // Destinations
		(native_asset, amount),
		fee_asset_id,
		transfer_assets
	);
}

fn relay_dest_assertions_fail(_t: SystemParaToRelayTest) {
	Pezkuwichain::assert_ump_queue_processed(false, Some(PeoplePezkuwichain::para_id()), None);
}

fn para_origin_assertions(t: SystemParaToRelayTest) {
	type RuntimeEvent = <PeoplePezkuwichain as Chain>::RuntimeEvent;

	PeoplePezkuwichain::assert_xcm_pallet_attempted_complete(None);

	PeoplePezkuwichain::assert_teyrchain_system_ump_sent();

	assert_expected_events!(
		PeoplePezkuwichain,
		vec![
			// Amount is withdrawn from Sender's account
			RuntimeEvent::Balances(pezpallet_balances::Event::Burned { who, amount }) => {
				who: *who == t.sender.account_id,
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn system_para_limited_teleport_assets(t: SystemParaToRelayTest) -> DispatchResult {
	<PeoplePezkuwichain as PeoplePezkuwichainPallet>::PezkuwiXcm::limited_teleport_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		bx!(t.args.fee_asset_id.into()),
		t.args.weight_limit,
	)
}

/// Limited Teleport of native asset from System Teyrchain to Relay Chain
/// shouldn't work when there is not enough balance in Relay Chain's `CheckAccount`
#[test]
fn limited_teleport_native_assets_from_system_para_to_relay_fails() {
	// Init values for Relay Chain
	let amount_to_send: Balance = PEZKUWICHAIN_ED * 1000;
	let destination = PeoplePezkuwichain::parent_location();
	let beneficiary_id = PezkuwichainReceiver::get();
	let assets = (Parent, amount_to_send).into();
	let fee_asset_id: AssetId = Parent.into();

	// Fund a sender
	PeoplePezkuwichain::fund_accounts(vec![(
		PeoplePezkuwichainSender::get(),
		PEZKUWICHAIN_ED * 2_000u128,
	)]);

	let test_args = TestContext {
		sender: PeoplePezkuwichainSender::get(),
		receiver: PezkuwichainReceiver::get(),
		args: TestArgs::new_para(
			destination,
			beneficiary_id,
			amount_to_send,
			assets,
			None,
			fee_asset_id,
		),
	};

	let mut test = SystemParaToRelayTest::new(test_args);

	let sender_balance_before = test.sender.balance;
	let receiver_balance_before = test.receiver.balance;

	test.set_assertion::<PeoplePezkuwichain>(para_origin_assertions);
	test.set_assertion::<Pezkuwichain>(relay_dest_assertions_fail);
	test.set_dispatchable::<PeoplePezkuwichain>(system_para_limited_teleport_assets);
	test.assert();

	let sender_balance_after = test.sender.balance;
	let receiver_balance_after = test.receiver.balance;

	let delivery_fees = PeoplePezkuwichain::execute_with(|| {
		xcm_helpers::teleport_assets_delivery_fees::<
			<PeoplePezkuwichainXcmConfig as xcm_executor::Config>::XcmSender,
		>(
			test.args.assets.clone(),
			test.args.fee_asset_id,
			test.args.weight_limit,
			test.args.beneficiary,
			test.args.dest,
		)
	});

	// Sender's balance is reduced
	assert_eq!(sender_balance_before - amount_to_send - delivery_fees, sender_balance_after);
	// Receiver's balance does not change
	assert_eq!(receiver_balance_after, receiver_balance_before);
}
