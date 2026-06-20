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

//! Tests for the validation of `pezpallet_xcm::Pezpallet::<T>::transfer_assets`.
//! See the `pezpallet_xcm::transfer_assets_validation` module for more information.

use crate::imports::*;
use pezframe_support::{assert_err, assert_ok};
use pezsp_runtime::DispatchError;

// ==================================================================================
// ============================== PenpalA <-> Zagros ===============================
// ==================================================================================

/// Test that `transfer_assets` fails when doing reserve transfer of ZGR from PenpalA to Zagros.
/// This fails because PenpalA's IsReserve config considers Zagros as the reserve for ZGR,
/// so transfer_assets automatically chooses reserve transfer, which we block.
#[test]
fn transfer_assets_wnd_reserve_transfer_para_to_relay_fails() {
	let destination = PenpalA::parent_location();
	let beneficiary: Location =
		AccountId32Junction { network: None, id: ZagrosReceiver::get().into() }.into();
	let amount_to_send: Balance = ZAGROS_ED * 1000;
	let assets: Assets = (Parent, amount_to_send).into();

	// Mint ZGR on PenpalA for testing.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		RelayLocation::get(),
		PenpalASender::get(),
		amount_to_send * 2,
	);

	// Fund PenpalA's sovereign account on Zagros with the reserve ZGR.
	let penpal_location_as_seen_by_relay = Zagros::child_location_of(PenpalA::para_id());
	let sov_penpal_on_relay = Zagros::sovereign_account_id_of(penpal_location_as_seen_by_relay);
	Zagros::fund_accounts(vec![(sov_penpal_on_relay.into(), amount_to_send * 2)]);

	let fee_asset_id: AssetId = Parent.into();
	PenpalA::execute_with(|| {
		let result = <PenpalA as PenpalAPallet>::PezkuwiXcm::transfer_assets(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			bx!(fee_asset_id.into()),
			WeightLimit::Unlimited,
		);

		// This should fail because ZGR reserve transfer is blocked.
		assert_err!(
			result,
			DispatchError::Module(pezsp_runtime::ModuleError {
				index: 31,
				error: [21, 0, 0, 0], // InvalidAssetUnknownReserve.
				message: Some("InvalidAssetUnknownReserve")
			})
		);
	});
}

/// Test that `transfer_assets` fails when doing reserve transfer of ZGR from Zagros to PenpalA
/// This fails because Zagros's configuration would make this a reserve transfer, which we block.
#[test]
fn transfer_assets_wnd_reserve_transfer_relay_to_para_fails() {
	let destination = Zagros::child_location_of(PenpalA::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: PenpalAReceiver::get().into() }.into();
	let amount_to_send: Balance = ZAGROS_ED * 1000;
	let assets: Assets = (Here, amount_to_send).into();

	let fee_asset_id: AssetId = Here.into();
	Zagros::execute_with(|| {
		let result = <Zagros as ZagrosPallet>::XcmPallet::transfer_assets(
			<Zagros as Chain>::RuntimeOrigin::signed(ZagrosSender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			bx!(fee_asset_id.into()),
			WeightLimit::Unlimited,
		);

		// This should fail because ZGR reserve transfer is blocked.
		assert_err!(
			result,
			DispatchError::Module(pezsp_runtime::ModuleError {
				index: 99,
				error: [21, 0, 0, 0], // InvalidAssetUnknownReserve.
				message: Some("InvalidAssetUnknownReserve")
			})
		);
	});
}

// ==================================================================================
// ============================== PenpalA <-> PenpalB ===============================
// ==================================================================================

/// Test that `transfer_assets` fails when doing reserve transfer of ZGR from PenpalA to PenpalB
#[test]
fn transfer_assets_wnd_reserve_transfer_para_to_para_fails() {
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: PenpalBReceiver::get().into() }.into();
	let amount_to_send: Balance = ZAGROS_ED * 1000;
	let assets: Assets = (Parent, amount_to_send).into();

	// Mint ZGR on PenpalA for testing
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		RelayLocation::get(),
		PenpalASender::get(),
		amount_to_send * 2,
	);

	let fee_asset_id: AssetId = Parent.into();
	PenpalA::execute_with(|| {
		let result = <PenpalA as PenpalAPallet>::PezkuwiXcm::transfer_assets(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			bx!(fee_asset_id.into()),
			WeightLimit::Unlimited,
		);

		// This should fail because ZGR reserve transfer is blocked
		assert_err!(
			result,
			DispatchError::Module(pezsp_runtime::ModuleError {
				index: 31,
				error: [21, 0, 0, 0], // InvalidAssetUnknownReserve
				message: Some("InvalidAssetUnknownReserve")
			})
		);
	});
}

// ==================================================================================
// ============================== Mixed Assets and Fees =============================
// ==================================================================================

/// Test that `transfer_assets` fails when ZGR is used as fee asset in reserve transfer
#[test]
fn transfer_assets_wnd_as_fee_in_reserve_transfer_fails() {
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: PenpalBReceiver::get().into() }.into();
	let asset_amount: Balance = 1_000_000_000_000; // A million USDT.
	let fee_amount: Balance = ZAGROS_ED * 100;

	// Create a foreign asset location (representing another asset).
	let foreign_asset_location = Location::new(
		1,
		[
			Teyrchain(AssetHubZagros::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(USDT_ID.into()), // USDT.
		],
	);

	// Mint both assets on PenpalA for testing.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		foreign_asset_location.clone(),
		PenpalASender::get(),
		asset_amount * 2,
	);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		RelayLocation::get(),
		PenpalASender::get(),
		fee_amount * 2,
	);

	// Transfer foreign asset, pay fees with ZGR.
	let assets: Assets = vec![
		(foreign_asset_location, asset_amount).into(),
		(Parent, fee_amount).into(), // ZGR as fee.
	]
	.into();
	let fee_asset_id: AssetId = Parent.into(); // ZGR is the fee asset.

	PenpalA::execute_with(|| {
		let result = <PenpalA as PenpalAPallet>::PezkuwiXcm::transfer_assets(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			bx!(fee_asset_id.into()),
			WeightLimit::Unlimited,
		);

		// This should fail because ZGR fee would be reserve transferred.
		assert_err!(
			result,
			DispatchError::Module(pezsp_runtime::ModuleError {
				index: 31,
				error: [21, 0, 0, 0], // InvalidAssetUnknownReserve.
				message: Some("InvalidAssetUnknownReserve")
			})
		);
	});
}

/// Test that `transfer_assets` works when neither asset nor fee is ZGR.
#[test]
fn transfer_assets_non_native_assets_work() {
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: PenpalBReceiver::get().into() }.into();
	let amount: Balance = 1_000_000_000_000; // A million USDT.

	// Create foreign asset locations (both non-native).
	let asset_location = Location::new(
		1,
		[
			Teyrchain(AssetHubZagros::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(USDT_ID.into()), // USDT.
		],
	);

	// Mint both USDT and ZGR on PenpalA, one for sending, the other for paying delivery fees.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		RelayLocation::get(),
		PenpalASender::get(),
		amount * 2,
	);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		asset_location.clone(),
		PenpalASender::get(),
		amount * 2,
	);

	// Transfer non-native assets.
	let assets: Assets = (asset_location.clone(), amount).into();
	let fee_asset_id: AssetId = (asset_location).into();

	PenpalA::execute_with(|| {
		let result = <PenpalA as PenpalAPallet>::PezkuwiXcm::transfer_assets(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			bx!(fee_asset_id.into()),
			WeightLimit::Unlimited,
		);

		// This should succeed because neither asset is ZGR.
		assert_ok!(result);
	});
}
