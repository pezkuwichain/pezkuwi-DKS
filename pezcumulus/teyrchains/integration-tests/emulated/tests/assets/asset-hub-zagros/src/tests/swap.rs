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
	let asset_native =
		Box::new(Location::try_from(RelayLocation::get()).expect("conversion works"));
	let asset_one = Box::new(Location {
		parents: 0,
		interior: [
			Junction::PalletInstance(ASSETS_PALLET_ID),
			Junction::GeneralIndex(ASSET_ID.into()),
		]
		.into(),
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::create(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			ASSET_ID.into(),
			AssetHubZagrosSender::get().into(),
			1000,
		));
		assert!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::mint(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			ASSET_ID.into(),
			AssetHubZagrosSender::get().into(),
			3_000_000_000_000,
		));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::create_pool(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			asset_native.clone(),
			asset_one.clone(),
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::add_liquidity(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			asset_native.clone(),
			asset_one.clone(),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			AssetHubZagrosSender::get().into()
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => { lp_token_minted: *lp_token_minted == 1414213562273, },
			]
		);

		let path = vec![asset_native.clone(), asset_one.clone()];

		assert_ok!(
			<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::swap_exact_tokens_for_tokens(
				<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
				path,
				100,
				1,
				AssetHubZagrosSender::get().into(),
				true
			)
		);

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. }) => {
					amount_in: *amount_in == 100,
					amount_out: *amount_out == 199,
				},
			]
		);

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::remove_liquidity(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			asset_native.clone(),
			asset_one.clone(),
			1414213562273 - 2_000_000_000, // all but the 2 EDs can't be retrieved.
			0,
			0,
			AssetHubZagrosSender::get().into(),
		));
	});
}

#[test]
fn swap_locally_on_chain_using_foreign_assets() {
	let asset_native = Box::new(Location::try_from(RelayLocation::get()).unwrap());
	let asset_location_on_penpal = PenpalA::execute_with(|| {
		Location::try_from(PenpalLocalTeleportableToAssetHub::get()).unwrap()
	});
	let foreign_asset_at_asset_hub_zagros =
		Location::new(1, [Junction::Teyrchain(PenpalA::para_id().into())])
			.appended_with(asset_location_on_penpal)
			.unwrap();

	let penpal_as_seen_by_ah = AssetHubZagros::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubZagros::sovereign_account_id_of(penpal_as_seen_by_ah);
	AssetHubZagros::fund_accounts(vec![
		// An account to swap dot for something else.
		(AssetHubZagrosSender::get().into(), 5_000_000 * ASSET_HUB_ZAGROS_ED),
		// Penpal's sovereign account in AH should have some balance
		(sov_penpal_on_ahr.clone().into(), 100_000_000 * ASSET_HUB_ZAGROS_ED),
	]);

	AssetHubZagros::execute_with(|| {
		// 0: No need to create foreign asset as it exists in genesis.
		//
		// 1: Mint foreign asset on asset_hub_zagros:
		//
		// (While it might be nice to use batch,
		// currently that's disabled due to safe call filters.)

		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		// 1. Mint foreign asset (in reality this should be a teleport or some such)
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahr.clone().into()),
			foreign_asset_at_asset_hub_zagros.clone(),
			sov_penpal_on_ahr.clone().into(),
			ASSET_HUB_ZAGROS_ED * 3_000_000_000_000,
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Issued { .. }) => {},
			]
		);

		// 2. Create pool:
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::create_pool(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			asset_native.clone(),
			Box::new(foreign_asset_at_asset_hub_zagros.clone()),
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		// 3. Add liquidity:
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::add_liquidity(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahr.clone()),
			asset_native.clone(),
			Box::new(foreign_asset_at_asset_hub_zagros.clone()),
			1_000_000_000_000_000,
			2_000_000_000_000_000,
			0,
			0,
			sov_penpal_on_ahr.clone().into()
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => {
					lp_token_minted: *lp_token_minted == 1414213562372995,
				},
			]
		);

		// 4. Swap!
		let path = vec![asset_native.clone(), Box::new(foreign_asset_at_asset_hub_zagros.clone())];

		assert_ok!(
			<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::swap_exact_tokens_for_tokens(
				<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
				path,
				100000 * ASSET_HUB_ZAGROS_ED,
				1000 * ASSET_HUB_ZAGROS_ED,
				AssetHubZagrosSender::get().into(),
				true
			)
		);

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. },) => {
					amount_in: *amount_in == 100000000000000,
					amount_out: *amount_out == 181322178776029,
				},
			]
		);

		// 5. Remove liquidity
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::remove_liquidity(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahr.clone()),
			asset_native.clone(),
			Box::new(foreign_asset_at_asset_hub_zagros),
			1414213562372995 - ASSET_HUB_ZAGROS_ED * 2, // all but the 2 EDs can't be retrieved.
			0,
			0,
			sov_penpal_on_ahr.clone().into(),
		));
	});
}

#[test]
fn cannot_create_pool_from_pool_assets() {
	let asset_native = RelayLocation::get();
	let mut asset_one = ahw_xcm_config::PoolAssetsPalletLocation::get();
	asset_one.append_with(GeneralIndex(ASSET_ID.into())).expect("pool assets");

	AssetHubZagros::execute_with(|| {
		let pool_owner_account_id = AssetHubZagrosAssetConversionOrigin::get();

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PoolAssets::create(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(pool_owner_account_id.clone()),
			ASSET_ID.into(),
			pool_owner_account_id.clone().into(),
			1000,
		));
		assert!(<AssetHubZagros as AssetHubZagrosPallet>::PoolAssets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PoolAssets::mint(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(pool_owner_account_id),
			ASSET_ID.into(),
			AssetHubZagrosSender::get().into(),
			3_000_000_000_000,
		));

		assert_matches::assert_matches!(
			<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::create_pool(
				<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
				Box::new(Location::try_from(asset_native).expect("conversion works")),
				Box::new(Location::try_from(asset_one).expect("conversion works")),
			),
			Err(DispatchError::Module(ModuleError{index: _, error: _, message})) => assert_eq!(message, Some("Unknown"))
		);
	});
}

#[test]
fn pay_xcm_fee_with_some_asset_swapped_for_native() {
	let asset_native = Location::try_from(RelayLocation::get()).expect("conversion works");
	let asset_one = Location {
		parents: 0,
		interior: [
			Junction::PalletInstance(ASSETS_PALLET_ID),
			Junction::GeneralIndex(ASSET_ID.into()),
		]
		.into(),
	};
	let penpal = AssetHubZagros::sovereign_account_id_of(AssetHubZagros::sibling_location_of(
		PenpalA::para_id(),
	));

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		// set up pool with ASSET_ID <> NATIVE pair
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::create(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			ASSET_ID.into(),
			AssetHubZagrosSender::get().into(),
			ASSET_MIN_BALANCE,
		));
		assert!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::mint(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			ASSET_ID.into(),
			AssetHubZagrosSender::get().into(),
			3_000_000_000_000,
		));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::create_pool(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			Box::new(asset_native.clone()),
			Box::new(asset_one.clone()),
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::add_liquidity(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			Box::new(asset_native),
			Box::new(asset_one),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			AssetHubZagrosSender::get().into()
		));

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => { lp_token_minted: *lp_token_minted == 1414213562273, },
			]
		);

		// ensure `penpal` sovereign account has no native tokens and mint some `ASSET_ID`
		assert_eq!(
			<AssetHubZagros as AssetHubZagrosPallet>::Balances::free_balance(penpal.clone()),
			0
		);

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::touch_other(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			ASSET_ID.into(),
			penpal.clone().into(),
		));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::mint(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get()),
			ASSET_ID.into(),
			penpal.clone().into(),
			10_000_000_000_000,
		));
	});

	PenpalA::execute_with(|| {
		// send xcm transact from `penpal` account while paying with `ASSET_ID` tokens on
		// `AssetHubZagros`
		let call = <AssetHubZagros as Chain>::RuntimeCall::System(pezframe_system::Call::<
			<AssetHubZagros as Chain>::Runtime,
		>::remark {
			remark: vec![],
		})
		.encode()
		.into();

		let penpal_root = <PenpalA as Chain>::RuntimeOrigin::root();
		let fee_amount = 4_000_000_000_000u128;
		let asset_one =
			([PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())], fee_amount).into();
		let asset_hub_location = PenpalA::sibling_location_of(AssetHubZagros::para_id()).into();
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

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		AssetHubZagros::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::SwapCreditExecuted { .. },) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true,.. }) => {},
			]
		);
	});
}

#[test]
fn xcm_fee_querying_apis_work() {
	test_xcm_fee_querying_apis_work_for_asset_hub!(AssetHubZagros);
}
