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
use emulated_integration_tests_common::accounts::{ALICE, BOB};
use pezframe_support::{
	dispatch::RawOrigin, pezsp_runtime::traits::Dispatchable, traits::fungible::Inspect,
};
use pezkuwi_runtime_common::impls::VersionedLocatableAsset;
use pezkuwichain_runtime_constants::currency::GRAND;

// Fund Treasury account on Asset Hub from Treasury account on Relay Chain with TYRs.
#[test]
fn spend_roc_on_asset_hub() {
	// initial treasury balance on Asset Hub in TYRs.
	let treasury_balance = 9_000 * GRAND;
	// the balance spend on Asset Hub.
	let treasury_spend_balance = 1_000 * GRAND;

	let init_alice_balance = AssetHubPezkuwichain::execute_with(|| {
		<<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Balances as Inspect<_>>::balance(
			&AssetHubPezkuwichain::account_id_of(ALICE),
		)
	});

	Pezkuwichain::execute_with(|| {
		type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
		type RuntimeCall = <Pezkuwichain as Chain>::RuntimeCall;
		type Runtime = <Pezkuwichain as Chain>::Runtime;
		type Balances = <Pezkuwichain as PezkuwichainPallet>::Balances;
		type Treasury = <Pezkuwichain as PezkuwichainPallet>::Treasury;

		// Fund Treasury account on Asset Hub with TYRs.

		let root = <Pezkuwichain as Chain>::RuntimeOrigin::root();
		let treasury_account = Treasury::account_id();

		// Mint assets to Treasury account on Relay Chain.
		assert_ok!(Balances::force_set_balance(
			root.clone(),
			treasury_account.clone().into(),
			treasury_balance * 2,
		));

		Dmp::make_teyrchain_reachable(1000);
		let native_asset = Location::here();
		let asset_hub_location: Location = [Teyrchain(1000)].into();
		let treasury_location: Location = (Parent, PalletInstance(18)).into();

		let teleport_call = RuntimeCall::Utility(pezpallet_utility::Call::<Runtime>::dispatch_as {
			as_origin: bx!(PezkuwichainOriginCaller::system(RawOrigin::Signed(treasury_account))),
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
			Pezkuwichain,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	Pezkuwichain::execute_with(|| {
		type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
		type RuntimeCall = <Pezkuwichain as Chain>::RuntimeCall;
		type RuntimeOrigin = <Pezkuwichain as Chain>::RuntimeOrigin;
		type Runtime = <Pezkuwichain as Chain>::Runtime;
		type Treasury = <Pezkuwichain as PezkuwichainPallet>::Treasury;

		// Fund Alice account from Pezkuwichain Treasury account on Asset Hub.

		let treasury_origin: RuntimeOrigin =
			pezkuwichain_governance::pezpallet_custom_origins::Origin::Treasurer.into();

		let alice_location: Location = [Junction::AccountId32 {
			network: None,
			id: Pezkuwichain::account_id_of(ALICE).into(),
		}]
		.into();
		let asset_hub_location: Location = [Teyrchain(1000)].into();
		let native_asset = Location::parent();

		let treasury_spend_call =
			RuntimeCall::Treasury(pezpallet_treasury::Call::<Runtime>::spend {
				asset_kind: bx!(VersionedLocatableAsset::from((
					asset_hub_location.clone(),
					native_asset.into()
				))),
				amount: treasury_spend_balance,
				beneficiary: bx!(VersionedLocation::from(alice_location)),
				valid_from: None,
			});

		assert_ok!(treasury_spend_call.dispatch(treasury_origin));

		// Claim the spend.

		let bob_signed = RuntimeOrigin::signed(Pezkuwichain::account_id_of(BOB));
		assert_ok!(Treasury::payout(bob_signed.clone(), 0));

		assert_expected_events!(
			Pezkuwichain,
			vec![
				RuntimeEvent::Treasury(pezpallet_treasury::Event::AssetSpendApproved { .. }) => {},
				RuntimeEvent::Treasury(pezpallet_treasury::Event::Paid { .. }) => {},
			]
		);
	});

	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		type Balances = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Balances;

		// Ensure that the funds deposited to Alice account.

		let alice_account = AssetHubPezkuwichain::account_id_of(ALICE);
		assert_eq!(
			<Balances as Inspect<_>>::balance(&alice_account),
			treasury_spend_balance + init_alice_balance
		);

		// Assert events triggered by xcm pay program:
		// 1. treasury asset transferred to spend beneficiary;
		// 2. response to Relay Chain Treasury pezpallet instance sent back;
		// 3. XCM program completed;
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::Balances(pezpallet_balances::Event::Transfer { .. }) => {},
				RuntimeEvent::TeyrchainSystem(pezcumulus_pezpallet_teyrchain_system::Event::UpwardMessageSent { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true ,.. }) => {},
			]
		);
	});
}

// NOTE: This test is disabled because `AssetRate` pallet is not available in pezkuwichain runtime.
// The test depends on `<Pezkuwichain as PezkuwichainPallet>::AssetRate` which doesn't exist.
#[cfg(any())]
mod disabled_usdt_treasury_test {
	use super::*;
	use emulated_integration_tests_common::USDT_ID;
	use pezframe_support::traits::fungibles::{Inspect as FungiblesInspect, Mutate};
	use teyrchains_common::AccountId;
	use xcm_executor::traits::ConvertLocation;

	#[test]
	fn create_and_claim_treasury_spend_in_usdt() {
		const SPEND_AMOUNT: u128 = 10_000_000;
		// treasury location from a sibling teyrchain.
		let treasury_location: Location = Location::new(1, PalletInstance(18));
		// treasury account on a sibling teyrchain.
		let treasury_account =
			ahr_xcm_config::LocationToAccountId::convert_location(&treasury_location).unwrap();
		let asset_hub_location =
			Location::new(0, Teyrchain(AssetHubPezkuwichain::para_id().into()));
		let root = <Pezkuwichain as Chain>::RuntimeOrigin::root();
		// asset kind to be spent from the treasury.
		let asset_kind: VersionedLocatableAsset = (
			asset_hub_location,
			AssetId((PalletInstance(50), GeneralIndex(USDT_ID.into())).into()),
		)
			.into();
		// treasury spend beneficiary.
		let alice: AccountId = Pezkuwichain::account_id_of(ALICE);
		let bob: AccountId = Pezkuwichain::account_id_of(BOB);
		let bob_signed = <Pezkuwichain as Chain>::RuntimeOrigin::signed(bob.clone());

		AssetHubPezkuwichain::execute_with(|| {
			type Assets = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets;

			// USDT created at genesis, mint some assets to the treasury account.
			assert_ok!(<Assets as Mutate<_>>::mint_into(
				USDT_ID,
				&treasury_account,
				SPEND_AMOUNT * 4
			));
			// beneficiary has zero balance.
			assert_eq!(<Assets as FungiblesInspect<_>>::balance(USDT_ID, &alice,), 0u128,);
		});

		Pezkuwichain::execute_with(|| {
			type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
			type Treasury = <Pezkuwichain as PezkuwichainPallet>::Treasury;
			type AssetRate = <Pezkuwichain as PezkuwichainPallet>::AssetRate;

			// create a conversion rate from `asset_kind` to the native currency.
			assert_ok!(AssetRate::create(root.clone(), Box::new(asset_kind.clone()), 2.into()));

			Dmp::make_teyrchain_reachable(1000);

			// create and approve a treasury spend.
			assert_ok!(Treasury::spend(
				root,
				Box::new(asset_kind),
				SPEND_AMOUNT,
				Box::new(Location::new(0, Into::<[u8; 32]>::into(alice.clone())).into()),
				None,
			));
			// claim the spend.
			assert_ok!(Treasury::payout(bob_signed.clone(), 0));

			assert_expected_events!(
				Pezkuwichain,
				vec![
					RuntimeEvent::Treasury(pezpallet_treasury::Event::Paid { .. }) => {},
				]
			);
		});

		AssetHubPezkuwichain::execute_with(|| {
			type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
			type Assets = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets;

			// assert events triggered by xcm pay program
			// 1. treasury asset transferred to spend beneficiary
			// 2. response to Relay Chain treasury pezpallet instance sent back
			// 3. XCM program completed
			assert_expected_events!(
				AssetHubPezkuwichain,
				vec![
					RuntimeEvent::Assets(pezpallet_assets::Event::Transferred { asset_id: id, from, to, amount }) => {
						id: id == &USDT_ID,
						from: from == &treasury_account,
						to: to == &alice,
						amount: amount == &SPEND_AMOUNT,
					},
					RuntimeEvent::TeyrchainSystem(pezcumulus_pezpallet_teyrchain_system::Event::UpwardMessageSent { .. }) => {},
					RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true ,.. }) => {},
				]
			);
			// beneficiary received the assets from the treasury.
			assert_eq!(<Assets as FungiblesInspect<_>>::balance(USDT_ID, &alice,), SPEND_AMOUNT,);
		});

		Pezkuwichain::execute_with(|| {
			type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
			type Treasury = <Pezkuwichain as PezkuwichainPallet>::Treasury;

			// check the payment status to ensure the response from the AssetHub was received.
			assert_ok!(Treasury::check_status(bob_signed, 0));
			assert_expected_events!(
				Pezkuwichain,
				vec![
					RuntimeEvent::Treasury(pezpallet_treasury::Event::SpendProcessed { .. }) => {},
				]
			);
		});
	}
}
