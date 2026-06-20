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

#[test]
fn swap_locally_on_chain_using_local_assets() {
	let asset_native = Box::new(Location::try_from(RelayLocation::get()).unwrap());
	let asset_one = Box::new(Location::new(
		0,
		[Junction::PalletInstance(ASSETS_PALLET_ID), Junction::GeneralIndex(ASSET_ID.into())],
	));

	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;

		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets::create(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
				AssetHubPezkuwichainSender::get()
			),
			ASSET_ID.into(),
			AssetHubPezkuwichainSender::get().into(),
			1000,
		));
		assert!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets::asset_exists(
			ASSET_ID
		));

		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets::mint(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
				AssetHubPezkuwichainSender::get()
			),
			ASSET_ID.into(),
			AssetHubPezkuwichainSender::get().into(),
			100_000_000_000_000,
		));

		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::create_pool(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
					AssetHubPezkuwichainSender::get()
				),
				asset_native.clone(),
				asset_one.clone(),
			)
		);

		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::add_liquidity(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
					AssetHubPezkuwichainSender::get()
				),
				asset_native.clone(),
				asset_one.clone(),
				1_000_000_000_000,
				2_000_000_000_000,
				0,
				0,
				AssetHubPezkuwichainSender::get().into()
			)
		);

		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => { lp_token_minted: *lp_token_minted == 1414213562273, },
			]
		);

		let path = vec![asset_native.clone(), asset_one.clone()];

		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::swap_exact_tokens_for_tokens(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(AssetHubPezkuwichainSender::get()),
				path,
				100,
				1,
				AssetHubPezkuwichainSender::get().into(),
				true
			)
		);

		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. }) => {
					amount_in: *amount_in == 100,
					amount_out: *amount_out == 199,
				},
			]
		);

		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::remove_liquidity(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
					AssetHubPezkuwichainSender::get()
				),
				asset_native,
				asset_one,
				1414213562273 - ASSET_HUB_PEZKUWICHAIN_ED * 2, /* all but the 2 EDs can't be
				                                                * retrieved. */
				0,
				0,
				AssetHubPezkuwichainSender::get().into(),
			)
		);
	});
}

#[test]
fn swap_locally_on_chain_using_foreign_assets() {
	let asset_native = Box::new(Location::try_from(RelayLocation::get()).unwrap());
	let asset_location_on_penpal = PenpalA::execute_with(|| {
		Location::try_from(PenpalLocalTeleportableToAssetHub::get()).unwrap()
	});
	let foreign_asset_at_asset_hub_pezkuwichain =
		Location::new(1, [Junction::Teyrchain(PenpalA::para_id().into())])
			.appended_with(asset_location_on_penpal)
			.unwrap();

	let penpal_as_seen_by_ah = AssetHubPezkuwichain::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubPezkuwichain::sovereign_account_id_of(penpal_as_seen_by_ah);
	AssetHubPezkuwichain::fund_accounts(vec![
		// An account to swap dot for something else.
		(AssetHubPezkuwichainSender::get().into(), 5_000_000 * ASSET_HUB_PEZKUWICHAIN_ED),
		// Penpal's sovereign account in AH should have some balance
		(sov_penpal_on_ahr.clone().into(), 100_000_000 * ASSET_HUB_PEZKUWICHAIN_ED),
	]);

	AssetHubPezkuwichain::execute_with(|| {
		// 0: No need to create foreign asset as it exists in genesis.
		//
		// 1: Mint foreign asset on asset_hub_pezkuwichain:
		//
		// (While it might be nice to use batch,
		// currently that's disabled due to safe call filters.)

		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		// 1. Mint foreign asset (in reality this should be a teleport or some such)
		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::ForeignAssets::mint(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
				sov_penpal_on_ahr.clone().into()
			),
			foreign_asset_at_asset_hub_pezkuwichain.clone(),
			sov_penpal_on_ahr.clone().into(),
			ASSET_HUB_PEZKUWICHAIN_ED * 3_000_000_000_000,
		));

		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Issued { .. }) => {},
			]
		);

		// 2. Create pool:
		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::create_pool(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
					AssetHubPezkuwichainSender::get()
				),
				asset_native.clone(),
				Box::new(foreign_asset_at_asset_hub_pezkuwichain.clone()),
			)
		);

		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		// 3. Add liquidity:
		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::add_liquidity(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahr.clone()),
				asset_native.clone(),
				Box::new(foreign_asset_at_asset_hub_pezkuwichain.clone()),
				1_000_000_000_000,
				2_000_000_000_000,
				0,
				0,
				sov_penpal_on_ahr.clone().into()
			)
		);

		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => {
					lp_token_minted: *lp_token_minted == 1414213562273,
				},
			]
		);

		// 4. Swap!
		let path =
			vec![asset_native.clone(), Box::new(foreign_asset_at_asset_hub_pezkuwichain.clone())];

		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::swap_exact_tokens_for_tokens(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(AssetHubPezkuwichainSender::get()),
				path,
				100000 * ASSET_HUB_PEZKUWICHAIN_ED,
				1000 * ASSET_HUB_PEZKUWICHAIN_ED,
				AssetHubPezkuwichainSender::get().into(),
				true
			)
		);

		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. },) => {
					amount_in: *amount_in == 333333300000,
					amount_out: *amount_out == 498874118173,
				},
			]
		);

		// 5. Remove liquidity
		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::remove_liquidity(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahr.clone()),
				asset_native.clone(),
				Box::new(foreign_asset_at_asset_hub_pezkuwichain.clone()),
				1414213562273 - ASSET_HUB_PEZKUWICHAIN_ED * 2, /* all but the 2 EDs can't be
				                                                * retrieved. */
				0,
				0,
				sov_penpal_on_ahr.clone().into(),
			)
		);
	});
}

#[test]
fn cannot_create_pool_from_pool_assets() {
	let asset_native = RelayLocation::get();
	let mut asset_one = ahr_xcm_config::PoolAssetsPalletLocation::get();
	asset_one.append_with(GeneralIndex(ASSET_ID.into())).expect("pool assets");

	AssetHubPezkuwichain::execute_with(|| {
		let pool_owner_account_id = AssetHubPezkuwichainAssetConversionOrigin::get();

		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::PoolAssets::create(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(pool_owner_account_id.clone()),
			ASSET_ID.into(),
			pool_owner_account_id.clone().into(),
			1000,
		));
		assert!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::PoolAssets::asset_exists(
			ASSET_ID
		));

		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::PoolAssets::mint(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(pool_owner_account_id),
			ASSET_ID.into(),
			AssetHubPezkuwichainSender::get().into(),
			3_000_000_000_000,
		));

		assert_matches::assert_matches!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::create_pool(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(AssetHubPezkuwichainSender::get()),
				Box::new(Location::try_from(asset_native).unwrap()),
				Box::new(Location::try_from(asset_one).unwrap()),
			),
			Err(DispatchError::Module(ModuleError{index: _, error: _, message})) => assert_eq!(message, Some("Unknown"))
		);
	});
}

#[test]
fn pay_xcm_fee_with_some_asset_swapped_for_native() {
	let asset_native = Location::try_from(RelayLocation::get()).unwrap();
	let asset_one = Location {
		parents: 0,
		interior: [
			Junction::PalletInstance(ASSETS_PALLET_ID),
			Junction::GeneralIndex(ASSET_ID.into()),
		]
		.into(),
	};
	let penpal = AssetHubPezkuwichain::sovereign_account_id_of(
		AssetHubPezkuwichain::sibling_location_of(PenpalA::para_id()),
	);

	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;

		// set up pool with ASSET_ID <> NATIVE pair
		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets::create(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
				AssetHubPezkuwichainSender::get()
			),
			ASSET_ID.into(),
			AssetHubPezkuwichainSender::get().into(),
			ASSET_MIN_BALANCE,
		));
		assert!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets::asset_exists(
			ASSET_ID
		));

		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets::mint(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
				AssetHubPezkuwichainSender::get()
			),
			ASSET_ID.into(),
			AssetHubPezkuwichainSender::get().into(),
			3_000_000_000_000,
		));

		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::create_pool(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
					AssetHubPezkuwichainSender::get()
				),
				Box::new(asset_native.clone()),
				Box::new(asset_one.clone()),
			)
		);

		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::AssetConversion::add_liquidity(
				<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
					AssetHubPezkuwichainSender::get()
				),
				Box::new(asset_native),
				Box::new(asset_one),
				1_000_000_000_000,
				2_000_000_000_000,
				0,
				0,
				AssetHubPezkuwichainSender::get().into()
			)
		);

		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => { lp_token_minted: *lp_token_minted == 1414213562273, },
			]
		);

		// ensure `penpal` sovereign account has no native tokens and mint some `ASSET_ID`
		assert_eq!(
			<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Balances::free_balance(
				penpal.clone()
			),
			0
		);

		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets::touch_other(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
				AssetHubPezkuwichainSender::get()
			),
			ASSET_ID.into(),
			penpal.clone().into(),
		));

		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Assets::mint(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(
				AssetHubPezkuwichainSender::get()
			),
			ASSET_ID.into(),
			penpal.clone().into(),
			10_000_000_000_000,
		));
	});

	PenpalA::execute_with(|| {
		// send xcm transact from `penpal` account while paying with `ASSET_ID` tokens on
		// `AssetHubPezkuwichain`
		let call = <AssetHubPezkuwichain as Chain>::RuntimeCall::System(pezframe_system::Call::<
			<AssetHubPezkuwichain as Chain>::Runtime,
		>::remark {
			remark: vec![],
		})
		.encode()
		.into();

		let penpal_root = <PenpalA as Chain>::RuntimeOrigin::root();
		let fee_amount = 4_000_000_000_000u128;
		let asset_one =
			([PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())], fee_amount).into();
		let asset_hub_location =
			PenpalA::sibling_location_of(AssetHubPezkuwichain::para_id()).into();
		let xcm = xcm_transact_paid_execution(
			call,
			OriginKind::SovereignAccount,
			asset_one,
			penpal.clone(),
		);

		assert_ok!(<PenpalA as PenpalAPallet>::PezkuwiXcm::send(
			penpal_root,
			bx!(asset_hub_location),
			bx!(xcm),
		));

		PenpalA::assert_xcm_pallet_sent();
	});

	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;

		AssetHubPezkuwichain::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::SwapCreditExecuted { .. },) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true,.. }) => {},
			]
		);
	});
}

#[test]
fn xcm_fee_querying_apis_work() {
	test_xcm_fee_querying_apis_work_for_asset_hub!(AssetHubPezkuwichain);
}
