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
use collectives_zagros_runtime::{
	ambassador::AmbassadorSalaryPaymaster, secretary::SecretarySalaryPaymaster,
};
use pezframe_support::{
	assert_ok,
	traits::{fungible::Mutate as FungibleMutate, fungibles::Mutate, tokens::Pay},
};
use xcm_executor::traits::ConvertLocation;

const AMBASSADOR_SALARY_PALLET_ID: u8 = 74;
const SECRETARY_SALARY_PALLET_ID: u8 = 91;

#[test]
fn pay_salary_secretary() {
	const USDT_ID: u32 = 1984;
	let secretary_salary = (
		Parent,
		Teyrchain(CollectivesZagros::para_id().into()),
		PalletInstance(SECRETARY_SALARY_PALLET_ID),
	);
	let pay_from = AssetHubLocationToAccountId::convert_location(&secretary_salary.into()).unwrap();
	let pay_to = Zagros::account_id_of(ALICE);
	let pay_amount = 9_000_000_000;

	AssetHubZagros::execute_with(|| {
		type AssetHubAssets = <AssetHubZagros as AssetHubZagrosPallet>::Assets;
		// USDT registered in genesis, now mint some into the payer's account
		assert_ok!(<AssetHubAssets as Mutate<_>>::mint_into(USDT_ID, &pay_from, pay_amount * 2));
	});

	CollectivesZagros::execute_with(|| {
		type RuntimeEvent = <CollectivesZagros as Chain>::RuntimeEvent;

		assert_ok!(SecretarySalaryPaymaster::pay(&pay_to, (), pay_amount));
		assert_expected_events!(
			CollectivesZagros,
			vec![
				RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::Assets(pezpallet_assets::Event::Transferred { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
	});
}

#[test]
fn pay_salary_ambassador() {
	// The Ambassador salary pays in the relay's native token, held by the salary pallet's
	// sovereign account on the Asset Hub. The payout arrives there as unpaid XCM from a pallet
	// on the Collectives chain, so the Asset Hub has to name that pallet in its barrier and in
	// its fee waivers -- the Secretary's salary was once configured in full and unable to pay
	// anyone for exactly this reason.
	let ambassador_salary = (
		Parent,
		Teyrchain(CollectivesZagros::para_id().into()),
		PalletInstance(AMBASSADOR_SALARY_PALLET_ID),
	);
	let pay_from =
		AssetHubLocationToAccountId::convert_location(&ambassador_salary.into()).unwrap();
	let pay_to = Zagros::account_id_of(ALICE);
	let pay_amount = 9_000_000_000;

	AssetHubZagros::execute_with(|| {
		type AssetHubBalances = <AssetHubZagros as AssetHubZagrosPallet>::Balances;
		assert_ok!(<AssetHubBalances as FungibleMutate<_>>::mint_into(&pay_from, pay_amount * 2));
	});

	CollectivesZagros::execute_with(|| {
		type RuntimeEvent = <CollectivesZagros as Chain>::RuntimeEvent;

		assert_ok!(AmbassadorSalaryPaymaster::pay(&pay_to, (), pay_amount));
		assert_expected_events!(
			CollectivesZagros,
			vec![
				RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::Balances(pezpallet_balances::Event::Transfer { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
	});
}
