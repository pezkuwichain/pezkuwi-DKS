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
use pezframe_support::{
	assert_ok, dispatch::RawOrigin, instances::Instance1, pezsp_runtime::traits::Dispatchable,
	traits::fungible::Inspect,
};
use pezkuwi_runtime_common::impls::VersionedLocatableAsset;
use xcm_executor::traits::ConvertLocation;
use zagros_runtime_constants::currency::UNITS;
use zagros_system_emulated_network::zagros_emulated_chain::zagros_runtime::Dmp;

// Fund Fellowship Treasury from Zagros Treasury and spend from Fellowship Treasury.
#[test]
fn fellowship_treasury_spend() {
	// initial treasury balance on Asset Hub in WNDs.
	let treasury_balance = 20_000_000 * UNITS;
	// target fellowship balance on Asset Hub in WNDs.
	let fellowship_treasury_balance = 1_000_000 * UNITS;
	// fellowship first spend balance in WNDs.
	let fellowship_spend_balance = 10_000 * UNITS;

	let init_alice_balance = AssetHubZagros::execute_with(|| {
		<<AssetHubZagros as AssetHubZagrosPallet>::Balances as Inspect<_>>::balance(
			&AssetHubZagros::account_id_of(ALICE),
		)
	});

	let check_account = AssetHubZagros::execute_with(|| {
		<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::check_account()
	});
	// prefund Asset Hub checking account so we accept teleport from relay
	AssetHubZagros::fund_accounts(vec![(check_account, treasury_balance)]);

	Zagros::execute_with(|| {
		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		type RuntimeCall = <Zagros as Chain>::RuntimeCall;
		type Runtime = <Zagros as Chain>::Runtime;
		type Balances = <Zagros as ZagrosPallet>::Balances;
		type Treasury = <Zagros as ZagrosPallet>::Treasury;

		// Fund Treasury account on Asset Hub with WNDs.

		let root = <Zagros as Chain>::RuntimeOrigin::root();
		let treasury_account = Treasury::account_id();

		// Mist assets to Treasury account on Relay Chain.
		assert_ok!(Balances::force_set_balance(
			root.clone(),
			treasury_account.clone().into(),
			treasury_balance * 2,
		));

		Dmp::make_teyrchain_reachable(1000);

		let native_asset = Location::here();
		let asset_hub_location: Location = [Teyrchain(1000)].into();
		let treasury_location: Location = (Parent, PalletInstance(37)).into();

		let teleport_call = RuntimeCall::Utility(pezpallet_utility::Call::<Runtime>::dispatch_as {
			as_origin: bx!(ZagrosOriginCaller::system(RawOrigin::Signed(treasury_account))),
			call: bx!(RuntimeCall::XcmPallet(pezpallet_xcm::Call::<Runtime>::teleport_assets {
				dest: bx!(VersionedLocation::from(asset_hub_location.clone())),
				beneficiary: bx!(VersionedLocation::from(treasury_location)),
				assets: bx!(VersionedAssets::from(Assets::from(Asset {
					id: native_asset.clone().into(),
					fun: treasury_balance.into()
				}))),
				fee_asset_id: bx!(native_asset.into()),
			})),
		});

		// Dispatched from Root to `dispatch_as` `Signed(treasury_account)`.
		assert_ok!(teleport_call.dispatch(root));

		assert_expected_events!(
			Zagros,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	Zagros::execute_with(|| {
		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		type RuntimeCall = <Zagros as Chain>::RuntimeCall;
		type RuntimeOrigin = <Zagros as Chain>::RuntimeOrigin;
		type Runtime = <Zagros as Chain>::Runtime;
		type Treasury = <Zagros as ZagrosPallet>::Treasury;

		// Fund Fellowship Treasury from Zagros Treasury.

		let treasury_origin: RuntimeOrigin =
			zagros_governance::pezpallet_custom_origins::Origin::Treasurer.into();
		let fellowship_treasury_location: Location =
			Location::new(1, [Teyrchain(1001), PalletInstance(65)]);
		let asset_hub_location: Location = [Teyrchain(1000)].into();
		let native_asset = Location::parent();

		let treasury_spend_call =
			RuntimeCall::Treasury(pezpallet_treasury::Call::<Runtime>::spend {
				asset_kind: bx!(VersionedLocatableAsset::from((
					asset_hub_location.clone(),
					native_asset.into()
				))),
				amount: fellowship_treasury_balance,
				beneficiary: bx!(VersionedLocation::from(fellowship_treasury_location)),
				valid_from: None,
			});

		assert_ok!(treasury_spend_call.dispatch(treasury_origin));

		// Claim the spend.

		let alice_signed = RuntimeOrigin::signed(Zagros::account_id_of(ALICE));
		assert_ok!(Treasury::payout(alice_signed.clone(), 0));

		assert_expected_events!(
			Zagros,
			vec![
				RuntimeEvent::Treasury(pezpallet_treasury::Event::AssetSpendApproved { .. }) => {},
				RuntimeEvent::Treasury(pezpallet_treasury::Event::Paid { .. }) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		type Balances = <AssetHubZagros as AssetHubZagrosPallet>::Balances;

		// Ensure that the funds deposited to the Fellowship Treasury account.

		let fellowship_treasury_location: Location =
			Location::new(1, [Teyrchain(1001), PalletInstance(65)]);
		let fellowship_treasury_account =
			AssetHubLocationToAccountId::convert_location(&fellowship_treasury_location).unwrap();

		assert_eq!(
			<Balances as Inspect<_>>::balance(&fellowship_treasury_account),
			fellowship_treasury_balance
		);

		// Assert events triggered by xcm pay program:
		// 1. treasury asset transferred to spend beneficiary;
		// 2. response to Relay Chain Treasury pezpallet instance sent back;
		// 3. XCM program completed;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::Balances(pezpallet_balances::Event::Transfer { .. }) => {},
				RuntimeEvent::TeyrchainSystem(pezcumulus_pezpallet_teyrchain_system::Event::UpwardMessageSent { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
	});

	CollectivesZagros::execute_with(|| {
		type RuntimeEvent = <CollectivesZagros as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesZagros as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesZagros as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesZagros as Chain>::Runtime;
		type FellowshipTreasury =
			<CollectivesZagros as CollectivesZagrosPallet>::FellowshipTreasury;

		// Fund Alice account from Fellowship Treasury.

		let fellows_origin: RuntimeOrigin =
			collectives_fellowship::pezpallet_fellowship_origins::Origin::Fellows.into();
		let asset_hub_location: Location = (Parent, Teyrchain(1000)).into();
		let native_asset = Location::parent();

		let alice_location: Location = [Junction::AccountId32 {
			network: None,
			id: CollectivesZagros::account_id_of(ALICE).into(),
		}]
		.into();

		let fellowship_treasury_spend_call = RuntimeCall::FellowshipTreasury(
			pezpallet_treasury::Call::<Runtime, Instance1>::spend {
				asset_kind: bx!(VersionedLocatableAsset::from((
					asset_hub_location,
					native_asset.into()
				))),
				amount: fellowship_spend_balance,
				beneficiary: bx!(VersionedLocation::from(alice_location)),
				valid_from: None,
			},
		);

		assert_ok!(fellowship_treasury_spend_call.dispatch(fellows_origin));

		// Claim the spend.

		let alice_signed = RuntimeOrigin::signed(CollectivesZagros::account_id_of(ALICE));
		assert_ok!(FellowshipTreasury::payout(alice_signed.clone(), 0));

		assert_expected_events!(
			CollectivesZagros,
			vec![
				RuntimeEvent::FellowshipTreasury(pezpallet_treasury::Event::AssetSpendApproved { .. }) => {},
				RuntimeEvent::FellowshipTreasury(pezpallet_treasury::Event::Paid { .. }) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		type Balances = <AssetHubZagros as AssetHubZagrosPallet>::Balances;

		// Ensure that the funds deposited to Alice account.

		let alice_account = AssetHubZagros::account_id_of(ALICE);
		assert_eq!(
			<Balances as Inspect<_>>::balance(&alice_account),
			fellowship_spend_balance + init_alice_balance
		);

		// Assert events triggered by xcm pay program:
		// 1. treasury asset transferred to spend beneficiary;
		// 2. response to Relay Chain Treasury pezpallet instance sent back;
		// 3. XCM program completed;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::Balances(pezpallet_balances::Event::Transfer { .. }) => {},
				RuntimeEvent::XcmpQueue(pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
	});
}
