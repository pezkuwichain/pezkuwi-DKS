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

const XCM_FEE: u128 = 40_000_000_000;

/// Tests the registering of a Zagros Asset as a bridged asset on Pezkuwichain Asset Hub.
#[test]
fn register_zagros_asset_on_rah_from_wah() {
	// Zagros Asset Hub asset when bridged to Pezkuwichain Asset Hub.
	let bridged_asset_at_rah = Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
			Teyrchain(AssetHubZagros::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(ASSET_ID.into()),
		],
	);
	// Register above asset on Pezkuwichain AH from Zagros AH.
	register_asset_on_rah_from_wah(bridged_asset_at_rah);
}

/// Tests the registering of an Ethereum Asset as a bridged asset on Pezkuwichain Asset Hub.
#[test]
fn register_ethereum_asset_on_rah_from_wah() {
	// Ethereum asset when bridged to Pezkuwichain Asset Hub.
	let token_id = H160::random();
	let bridged_asset_at_rah = Location::new(
		2,
		[
			GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID }),
			AccountKey20 { network: None, key: token_id.into() },
		],
	);
	// Register above asset on Pezkuwichain AH from Zagros AH.
	register_asset_on_rah_from_wah(bridged_asset_at_rah);
}

fn register_asset_on_rah_from_wah(bridged_asset_at_rah: Location) {
	let sa_of_wah_on_rah =
		AssetHubPezkuwichain::sovereign_account_of_teyrchain_on_other_global_consensus(
			ByGenesis(ZAGROS_GENESIS_HASH),
			AssetHubZagros::para_id(),
		);

	// Encoded `create_asset` call to be executed in Pezkuwichain Asset Hub ForeignAssets pezpallet.
	let call = AssetHubPezkuwichain::create_foreign_asset_call(
		bridged_asset_at_rah.clone(),
		ASSET_MIN_BALANCE,
		sa_of_wah_on_rah.clone(),
	);

	let origin_kind = OriginKind::Xcm;
	let fee_amount = XCM_FEE;
	let fees = (Parent, fee_amount).into();

	let xcm = xcm_transact_paid_execution(call, origin_kind, fees, sa_of_wah_on_rah.clone());

	// SA-of-WAH-on-RAH needs to have balance to pay for fees and asset creation deposit
	AssetHubPezkuwichain::fund_accounts(vec![(
		sa_of_wah_on_rah.clone(),
		ASSET_HUB_PEZKUWICHAIN_ED * 10000000000,
	)]);

	let destination = asset_hub_pezkuwichain_location();

	// fund the WAH's SA on WBH for paying bridge delivery fees
	BridgeHubZagros::fund_para_sovereign(AssetHubZagros::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	AssetHubZagros::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubZagros::force_xcm_version(bridge_hub_pezkuwichain_location(), XCM_VERSION);

	let root_origin = <AssetHubZagros as Chain>::RuntimeOrigin::root();
	AssetHubZagros::execute_with(|| {
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::send(
			root_origin,
			bx!(destination.into()),
			bx!(xcm),
		));

		AssetHubZagros::assert_xcm_pallet_sent();
	});

	assert_bridge_hub_zagros_message_accepted(true);
	assert_bridge_hub_pezkuwichain_message_received();
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		AssetHubPezkuwichain::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// Burned the fee
				RuntimeEvent::Balances(pezpallet_balances::Event::Burned { who, amount }) => {
					who: *who == sa_of_wah_on_rah.clone(),
					amount: *amount == fee_amount,
				},
				// Foreign Asset created
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Created { asset_id, creator, owner }) => {
					asset_id: asset_id == &bridged_asset_at_rah,
					creator: *creator == sa_of_wah_on_rah.clone(),
					owner: *owner == sa_of_wah_on_rah,
				},
				// Unspent fee minted to origin
				RuntimeEvent::Balances(pezpallet_balances::Event::Minted { who, .. }) => {
					who: *who == sa_of_wah_on_rah.clone(),
				},
			]
		);
		type ForeignAssets = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::ForeignAssets;
		assert!(ForeignAssets::asset_exists(bridged_asset_at_rah));
	});
}
