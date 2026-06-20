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

use crate::{imports::*, tests::*};

const XCM_FEE: u128 = 4_000_000_000_000;

/// Tests the registering of a Pezkuwichain Asset as a bridged asset on Zagros Asset Hub.
#[test]
fn register_pezkuwichain_asset_on_wah_from_rah() {
	let sa_of_rah_on_wah = AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
		ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
		AssetHubPezkuwichain::para_id(),
	);

	// Pezkuwichain Asset Hub asset when bridged to Zagros Asset Hub.
	let bridged_asset_at_wah = Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
			Teyrchain(AssetHubPezkuwichain::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(ASSET_ID.into()),
		],
	);

	// Encoded `create_asset` call to be executed in Zagros Asset Hub ForeignAssets pezpallet.
	let call = AssetHubZagros::create_foreign_asset_call(
		bridged_asset_at_wah.clone(),
		ASSET_MIN_BALANCE,
		sa_of_rah_on_wah.clone(),
	);

	let origin_kind = OriginKind::Xcm;
	let fee_amount = XCM_FEE;
	let fees = (Parent, fee_amount).into();

	let xcm = xcm_transact_paid_execution(call, origin_kind, fees, sa_of_rah_on_wah.clone());

	// SA-of-RAH-on-WAH needs to have balance to pay for fees and asset creation deposit
	AssetHubZagros::fund_accounts(vec![(
		sa_of_rah_on_wah.clone(),
		ASSET_HUB_ZAGROS_ED * 10000000000,
	)]);

	let destination = asset_hub_zagros_location();

	// fund the RAH's SA on RBH for paying bridge delivery fees
	BridgeHubPezkuwichain::fund_para_sovereign(
		AssetHubPezkuwichain::para_id(),
		10_000_000_000_000u128,
	);

	// set XCM versions
	AssetHubPezkuwichain::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubPezkuwichain::force_xcm_version(bridge_hub_zagros_location(), XCM_VERSION);

	let root_origin = <AssetHubPezkuwichain as Chain>::RuntimeOrigin::root();
	AssetHubPezkuwichain::execute_with(|| {
		assert_ok!(<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::PezkuwiXcm::send(
			root_origin,
			bx!(destination.into()),
			bx!(xcm),
		));

		AssetHubPezkuwichain::assert_xcm_pallet_sent();
	});

	assert_bridge_hub_pezkuwichain_message_accepted(true);
	assert_bridge_hub_zagros_message_received();
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		AssetHubZagros::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubZagros,
			vec![
				// Burned the fee
				RuntimeEvent::Balances(pezpallet_balances::Event::Burned { who, amount }) => {
					who: *who == sa_of_rah_on_wah.clone(),
					amount: *amount == fee_amount,
				},
				// Foreign Asset created
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Created { asset_id, creator, owner }) => {
					asset_id: asset_id == &bridged_asset_at_wah,
					creator: *creator == sa_of_rah_on_wah.clone(),
					owner: *owner == sa_of_rah_on_wah,
				},
				// Unspent fee minted to origin
				RuntimeEvent::Balances(pezpallet_balances::Event::Minted { who, .. }) => {
					who: *who == sa_of_rah_on_wah.clone(),
				},
			]
		);
		type ForeignAssets = <AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets;
		assert!(ForeignAssets::asset_exists(bridged_asset_at_wah));
	});
}
