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
	let amount = PEZKUWICHAIN_ED * 10;

	test_relay_is_trusted_teleporter!(
		Pezkuwichain,               // Origin
		vec![CoretimePezkuwichain], // Destinations
		amount,
		limited_teleport_assets
	);

	test_teyrchain_is_trusted_teleporter_for_relay!(
		CoretimePezkuwichain, // Origin
		Pezkuwichain,         // Destination
		amount,
		limited_teleport_assets
	);
}

#[test]
fn teleport_via_transfer_assets_from_and_to_relay() {
	let amount = PEZKUWICHAIN_ED * 10;

	test_relay_is_trusted_teleporter!(
		Pezkuwichain,               // Origin
		vec![CoretimePezkuwichain], // Destinations
		amount,
		transfer_assets
	);

	test_teyrchain_is_trusted_teleporter_for_relay!(
		CoretimePezkuwichain, // Origin
		Pezkuwichain,         // Destination
		amount,
		transfer_assets
	);
}

#[test]
fn teleport_via_limited_teleport_assets_from_coretime_to_asset_hub() {
	let amount = ASSET_HUB_PEZKUWICHAIN_ED * 100;
	let native_asset: Assets = (Parent, amount).into();

	let fee_asset_id: AssetId = Parent.into();
	test_teyrchain_is_trusted_teleporter!(
		CoretimePezkuwichain,       // Origin
		vec![AssetHubPezkuwichain], // Destinations
		(native_asset, amount),
		fee_asset_id,
		limited_teleport_assets
	);
}

#[test]
fn teleport_via_transfer_assets_from_coretime_to_asset_hub() {
	let amount = ASSET_HUB_PEZKUWICHAIN_ED * 100;
	let native_asset: Assets = (Parent, amount).into();

	let fee_asset_id: AssetId = Parent.into();
	test_teyrchain_is_trusted_teleporter!(
		CoretimePezkuwichain,       // Origin
		vec![AssetHubPezkuwichain], // Destinations
		(native_asset, amount),
		fee_asset_id,
		transfer_assets
	);
}

#[test]
fn teleport_via_limited_teleport_assets_from_asset_hub_to_coretime() {
	let amount = CORETIME_PEZKUWICHAIN_ED * 100;
	let native_asset: Assets = (Parent, amount).into();

	let fee_asset_id: AssetId = Parent.into();
	test_teyrchain_is_trusted_teleporter!(
		AssetHubPezkuwichain,       // Origin
		vec![CoretimePezkuwichain], // Destinations
		(native_asset, amount),
		fee_asset_id,
		limited_teleport_assets
	);
}

#[test]
fn teleport_via_transfer_assets_from_asset_hub_to_coretime() {
	let amount = CORETIME_PEZKUWICHAIN_ED * 100;
	let native_asset: Assets = (Parent, amount).into();

	let fee_asset_id: AssetId = Parent.into();
	test_teyrchain_is_trusted_teleporter!(
		AssetHubPezkuwichain,       // Origin
		vec![CoretimePezkuwichain], // Destinations
		(native_asset, amount),
		fee_asset_id,
		transfer_assets
	);
}
