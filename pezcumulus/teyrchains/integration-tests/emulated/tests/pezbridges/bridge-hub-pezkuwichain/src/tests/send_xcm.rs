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

use pezkuwichain_system_emulated_network::pezkuwichain_emulated_chain::pezkuwichain_runtime::Dmp;

use crate::tests::*;

#[test]
fn send_xcm_from_pezkuwichain_relay_to_zagros_asset_hub_should_fail_on_not_applicable() {
	// Init tests variables
	// XcmPallet send arguments
	let sudo_origin = <Pezkuwichain as Chain>::RuntimeOrigin::root();
	let destination = Pezkuwichain::child_location_of(BridgeHubPezkuwichain::para_id()).into();
	let weight_limit = WeightLimit::Unlimited;
	let check_origin = None;

	let remote_xcm = Xcm(vec![ClearOrigin]);

	let xcm = VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit, check_origin },
		ExportMessage {
			network: ByGenesis(ZAGROS_GENESIS_HASH),
			destination: [Teyrchain(AssetHubZagros::para_id().into())].into(),
			xcm: remote_xcm,
		},
	]));

	// Pezkuwichain Global Consensus
	// Send XCM message from Relay Chain to Bridge Hub source Teyrchain
	Pezkuwichain::execute_with(|| {
		Dmp::make_teyrchain_reachable(BridgeHubPezkuwichain::para_id());

		assert_ok!(<Pezkuwichain as PezkuwichainPallet>::XcmPallet::send(
			sudo_origin,
			bx!(destination),
			bx!(xcm),
		));

		type RuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;

		assert_expected_events!(
			Pezkuwichain,
			vec![
				RuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});
	// Receive XCM message in Bridge Hub source Teyrchain, it should fail, because we don't have
	// opened bridge/lane.
	assert_bridge_hub_pezkuwichain_message_accepted(false);
}

#[test]
fn send_xcm_through_opened_lane_with_different_xcm_version_on_hops_works() {
	// prepare data
	let destination = asset_hub_zagros_location();
	let native_token = Location::parent();
	let amount = ASSET_HUB_PEZKUWICHAIN_ED * 1_000;

	// fund the AHR's SA on BHR for paying bridge delivery fees
	BridgeHubPezkuwichain::fund_para_sovereign(
		AssetHubPezkuwichain::para_id(),
		10_000_000_000_000u128,
	);
	// fund sender
	AssetHubPezkuwichain::fund_accounts(vec![(
		AssetHubPezkuwichainSender::get().into(),
		amount * 10,
	)]);

	// Initially set only default version on all runtimes
	let newer_xcm_version = xcm::prelude::XCM_VERSION;
	let older_xcm_version = newer_xcm_version - 1;
	AssetHubPezkuwichain::force_default_xcm_version(Some(older_xcm_version));
	BridgeHubPezkuwichain::force_default_xcm_version(Some(older_xcm_version));
	BridgeHubZagros::force_default_xcm_version(Some(older_xcm_version));
	AssetHubZagros::force_default_xcm_version(Some(older_xcm_version));

	// send XCM from AssetHubPezkuwichain - fails - destination version not known
	assert_err!(
		send_assets_from_asset_hub_pezkuwichain(
			destination.clone(),
			(native_token.clone(), amount).into(),
			0,
			TransferType::LocalReserve
		),
		DispatchError::Module(pezsp_runtime::ModuleError {
			index: 31,
			error: [1, 0, 0, 0],
			message: Some("SendFailure")
		})
	);

	// set destination version
	AssetHubPezkuwichain::force_xcm_version(destination.clone(), newer_xcm_version);

	// set version with `ExportMessage` for BridgeHubPezkuwichain
	AssetHubPezkuwichain::force_xcm_version(
		ParentThen(Teyrchain(BridgeHubPezkuwichain::para_id().into()).into()).into(),
		newer_xcm_version,
	);
	// send XCM from AssetHubPezkuwichain - ok
	assert_ok!(send_assets_from_asset_hub_pezkuwichain(
		destination.clone(),
		(native_token.clone(), amount).into(),
		0,
		TransferType::LocalReserve
	));

	// `ExportMessage` on local BridgeHub - fails - remote BridgeHub version not known
	assert_bridge_hub_pezkuwichain_message_accepted(false);

	// set version for remote BridgeHub on BridgeHubPezkuwichain
	BridgeHubPezkuwichain::force_xcm_version(bridge_hub_zagros_location(), newer_xcm_version);
	// set version for AssetHubZagros on BridgeHubZagros
	BridgeHubZagros::force_xcm_version(
		ParentThen(Teyrchain(AssetHubZagros::para_id().into()).into()).into(),
		newer_xcm_version,
	);

	// send XCM from AssetHubPezkuwichain - ok
	assert_ok!(send_assets_from_asset_hub_pezkuwichain(
		destination.clone(),
		(native_token.clone(), amount).into(),
		0,
		TransferType::LocalReserve
	));
	assert_bridge_hub_pezkuwichain_message_accepted(true);
	assert_bridge_hub_zagros_message_received();
	// message delivered and processed at destination
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				// message processed with failure, but for this scenario it is ok, important is that was delivered
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: false, .. }
				) => {},
			]
		);
	});
}
