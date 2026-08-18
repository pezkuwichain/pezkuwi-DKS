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

use crate::{
	assets_balance_on, create_pool_with_wnd_on, foreign_balance_on, imports::*,
	tests::send::penpal_register_foreign_asset_on_asset_hub,
};

/// What the pool on Penpal is seeded with. These are the amounts the mint-and-pool macros use by
/// default; spelled out here because this file seeds the asset and the pool in two separate steps.
const POOL_NATIVE_LIQUIDITY: u128 = 1_000_000_000_000;
const POOL_ASSET_LIQUIDITY: u128 = 2_000_000_000_000;

// Registers a new asset on Penpal, then registers it over XCM as foreign asset on Asset Hub.
// The foreign asset is set up either as teleportable between Penpal and AH, by making AH a reserve
// for it too. Or it keeps the asset's reserve solely on Penpal resulting in reserve-based transfers
// between Penpal and AH.
pub fn set_up_foreign_asset(
	sender: pezsp_runtime::AccountId32,
	asset_location_on_penpal: Location,
	asset_amount_to_send: u128,
	teleportable: bool,
) -> (Location, Location) {
	let asset_owner = PenpalAssetOwner::get();

	// Give the sender enough native
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(asset_owner.clone()),
		RelayLocation::get(),
		sender.clone(),
		asset_amount_to_send,
	);

	// Create the asset on Penpal.
	//
	// The caller hands in a location, and on this chain a location of the form
	// `PalletInstance(50)/GeneralIndex(n)` is not a foreign asset at all: `AssetsPalletLocation`
	// together with `AsPrefixedGeneralIndex` is exactly how the runtime spells "asset `n` in the
	// index-keyed instance", and it is how `LocalAndForeignAssets` routes the location. So that is
	// where the asset is created, by index. Upstream creates it in the location-keyed instance
	// instead, which for them is the same pallet — their penpal carries only one. Ours carries two,
	// like both of our production Asset Hubs, so the two have to be told apart here.
	let asset_id_on_penpal: u32 = match asset_location_on_penpal.unpack() {
		(0, [Junction::PalletInstance(pallet), Junction::GeneralIndex(index)])
			if *pallet == PENPAL_ASSETS_PALLET_ID =>
			(*index).try_into().expect("asset index fits in u32"),
		_ => panic!(
			"expected an asset held by Penpal's own assets pallet, got {asset_location_on_penpal:?}"
		),
	};
	let to_fund = asset_amount_to_send * 2;
	// The owner is funded alongside the sender because it is the one that seeds the pool below,
	// and the pool takes its liquidity from the owner's own holding of this asset.
	PenpalA::force_create_asset(
		asset_id_on_penpal,
		asset_owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![(sender.clone(), to_fund), (asset_owner.clone(), POOL_ASSET_LIQUIDITY * 2)],
	);
	assert!(asset_exists_on!(PenpalA, asset_id_on_penpal));

	// Setup a pool on Penpal between native asset and newly created asset, so we can pay fees using
	// new asset directly.
	//
	// The pool is keyed by location, not by index — asset conversion on Penpal sits on top of the
	// union — so it is the location that goes in here even though the asset itself was created by
	// index. The mint-and-pool macros bundle those two into one argument, which cannot express a
	// chain where the two differ; the asset is already minted above, so only the pool is left.
	create_pool_with_native_location_on!(
		PenpalA,
		Location::here(),
		asset_location_on_penpal.clone(),
		asset_owner.clone(),
		POOL_NATIVE_LIQUIDITY,
		POOL_ASSET_LIQUIDITY
	);

	// Register asset on Asset Hub using XCM
	let penpal_sovereign_account = AssetHubZagros::sovereign_account_id_of(
		AssetHubZagros::sibling_location_of(PenpalA::para_id()),
	);
	let penpal_location = Location::new(1, [Junction::Teyrchain(PenpalA::para_id().into())]);
	let foreign_asset_at_asset_hub =
		penpal_location.clone().appended_with(asset_location_on_penpal.clone()).unwrap();
	// Do remote registration
	penpal_register_foreign_asset_on_asset_hub(asset_location_on_penpal.clone());

	// Setup a pool on Asset Hub between native asset and newly created asset, so we can pay fees
	// using new asset directly.
	create_pool_with_wnd_on!(
		AssetHubZagros,
		foreign_asset_at_asset_hub.clone(),
		true,
		penpal_sovereign_account.clone()
	);

	if teleportable {
		// Configure Penpal to allow teleports of this asset to AH
		PenpalA::execute_with(|| {
			assert_ok!(<PenpalA as Chain>::System::set_storage(
				<PenpalA as Chain>::RuntimeOrigin::root(),
				vec![(
					PenpalLocalTeleportableToAssetHub::key().to_vec(),
					asset_location_on_penpal.encode(),
				)],
			));
		});
	}
	let reserves_data = vec![(penpal_location, teleportable).into()];
	AssetHubZagros::set_foreign_asset_reserves(
		foreign_asset_at_asset_hub.clone(),
		penpal_sovereign_account.clone(),
		reserves_data,
	);
	(asset_location_on_penpal, foreign_asset_at_asset_hub)
}

// Helper for Penpal root to call ForeignAssets::set_reserves() on Asset Hub.
pub fn penpal_set_foreign_asset_reserves_on_asset_hub(
	asset_id_on_ah: Location,
	reserves: Vec<ForeignAssetReserveData>,
) {
	// Encoded `set_reserves` call to be executed in AssetHub
	let call = <AssetHubZagros as Chain>::RuntimeCall::ForeignAssets(pezpallet_assets::Call::<
		<AssetHubZagros as Chain>::Runtime,
		pezpallet_assets::Instance2,
	>::set_reserves {
		id: asset_id_on_ah.into(),
		reserves: reserves.try_into().expect("reserve list fits the pallet bound"),
	})
	.encode()
	.into();
	let penpal_sovereign = AssetHubZagros::sovereign_account_id_of(
		AssetHubZagros::sibling_location_of(PenpalA::para_id()),
	);
	let origin_kind = OriginKind::Xcm;
	let fee_amount = ASSET_HUB_ZAGROS_ED * 1000000;
	let system_asset = (Parent, fee_amount).into();
	let root_origin = <PenpalA as Chain>::RuntimeOrigin::root();
	let asset_hub_location = PenpalA::sibling_location_of(AssetHubZagros::para_id()).into();
	let xcm =
		xcm_transact_paid_execution(call, origin_kind, system_asset, penpal_sovereign.clone());

	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as PenpalAPallet>::PezkuwiXcm::send(
			root_origin,
			bx!(asset_hub_location),
			bx!(xcm),
		));
		PenpalA::assert_xcm_pezpallet_sent();
	});
}

// ==============================================================================================
// ==== Bidirectional Transfer - Teleportable Foreign Asset - Penpal<->AssetHub ====
// ==============================================================================================
/// Transfers of teleportable foreign asset from Penpal to AssetHub and back.
/// Also verifies that reserve-transferring the asset fails both ways.
#[test]
fn bidirectional_teleport_foreign_asset_between_penpal_and_asset_hub() {
	let sender = PenpalASender::get();
	let receiver = AssetHubZagrosReceiver::get();
	let new_asset_id: u32 = 42;
	let asset_location_on_penpal = local_penpal_asset(new_asset_id);
	let asset_amount_to_send = ASSET_HUB_ZAGROS_ED * 10_000;
	let (asset_location_on_penpal, foreign_asset_location_on_ah) =
		set_up_foreign_asset(sender.clone(), asset_location_on_penpal.clone(), asset_amount_to_send, true);

	////////////////////////////////
	// Teleport it from Penpal to AH
	////////////////////////////////

	let penpal_sender_balance_before = assets_balance_on!(PenpalA, new_asset_id, &sender);
	let ah_receiver_balance_before =
		foreign_balance_on!(AssetHubZagros, foreign_asset_location_on_ah.clone(), &receiver);

	let dest = PenpalA::sibling_location_of(AssetHubZagros::para_id());
	let assets: Assets =
		vec![(asset_location_on_penpal.clone(), asset_amount_to_send).into()].into();
	// execute xcm from penpal to asset hub
	PenpalA::execute_with(|| {
		// xcm to be executed at dest
		let xcm_on_dest = Xcm(vec![
			// since this is the last hop, we don't need to further use any assets previously
			// reserved for fees (there are no further hops to cover delivery fees for); we
			// RefundSurplus to get back any unspent fees
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: receiver.clone().into() },
		]);
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(assets.clone().into()),
			SetFeesMode { jit_withdraw: true },
			InitiateTransfer {
				destination: dest.clone(),
				remote_fees: Some(AssetTransferFilter::Teleport(assets.clone().into())),
				preserve_origin: false,
				assets: BoundedVec::new(),
				remote_xcm: xcm_on_dest.clone(),
			},
		]);
		// teleporting the asset works
		<PenpalA as PenpalAPallet>::PezkuwiXcm::execute(
			<PenpalA as Chain>::RuntimeOrigin::signed(sender.clone()),
			bx!(xcm::VersionedXcm::from(xcm.into())),
			Weight::MAX,
		)
		.unwrap();
	});

	let penpal_sender_balance_after = assets_balance_on!(PenpalA, new_asset_id, &sender);
	let ah_receiver_balance_after =
		foreign_balance_on!(AssetHubZagros, foreign_asset_location_on_ah.clone(), &receiver);

	assert!(penpal_sender_balance_after < penpal_sender_balance_before);
	assert!(ah_receiver_balance_after > ah_receiver_balance_before);

	// reserve-transferring the asset fails
	PenpalA::execute_with(|| {
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(assets.clone().into()),
			SetFeesMode { jit_withdraw: true },
			InitiateTransfer {
				destination: dest,
				remote_fees: Some(AssetTransferFilter::ReserveDeposit(assets.clone().into())),
				preserve_origin: false,
				assets: BoundedVec::new(),
				remote_xcm: Default::default(),
			},
		]);
		<PenpalA as PenpalAPallet>::PezkuwiXcm::execute(
			<PenpalA as Chain>::RuntimeOrigin::signed(sender.clone()),
			bx!(xcm::VersionedXcm::from(xcm.into())),
			Weight::MAX,
		)
		.unwrap();
	});
	// AH is expected to reject the transfer with `UntrustedReserveLocation`
	let expected_origin = AssetHubZagros::sibling_location_of(PenpalA::para_id());
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::PezkuwiXcm(
					pezpallet_xcm::Event::ProcessXcmError { origin, error, .. }
				) => {
					origin: *origin == expected_origin,
					error: *error == xcm::latest::Error::UntrustedReserveLocation,
				},
			]
		);
	});

	/////////////////////////////////////
	// Teleport it back from AH to Penpal
	/////////////////////////////////////

	let asset_amount_to_send = ah_receiver_balance_after;
	let ah_sender_balance_before =
		foreign_balance_on!(AssetHubZagros, foreign_asset_location_on_ah.clone(), &receiver);
	let penpal_receiver_balance_before = assets_balance_on!(PenpalA, new_asset_id, &sender);

	let dest = AssetHubZagros::sibling_location_of(PenpalA::para_id());
	// execute xcm from asset hub to penpal
	AssetHubZagros::execute_with(|| {
		let assets: Assets =
			vec![(foreign_asset_location_on_ah.clone(), asset_amount_to_send).into()].into();
		// xcm to be executed at dest
		let xcm_on_dest = Xcm(vec![
			// since this is the last hop, we don't need to further use any assets previously
			// reserved for fees (there are no further hops to cover delivery fees for); we
			// RefundSurplus to get back any unspent fees
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: sender.clone().into() },
		]);
		// reserve-transferring the asset back to penpal fails
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(assets.clone().into()),
			SetFeesMode { jit_withdraw: true },
			InitiateTransfer {
				destination: dest.clone(),
				remote_fees: Some(AssetTransferFilter::ReserveWithdraw(assets.clone().into())),
				preserve_origin: false,
				assets: BoundedVec::new(),
				remote_xcm: Default::default(),
			},
		]);
		assert!(matches!(
			<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::execute(
				<AssetHubZagros as Chain>::RuntimeOrigin::signed(receiver.clone()),
				bx!(xcm::VersionedXcm::from(xcm.into())),
				Weight::MAX,
			),
			Err(pezsp_runtime::DispatchErrorWithPostInfo { .. }),
		));
		// teleporting it back works
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(assets.clone().into()),
			SetFeesMode { jit_withdraw: true },
			InitiateTransfer {
				destination: dest,
				remote_fees: Some(AssetTransferFilter::Teleport(assets.into())),
				preserve_origin: false,
				assets: BoundedVec::new(),
				remote_xcm: xcm_on_dest,
			},
		]);
		<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::execute(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(receiver.clone()),
			bx!(xcm::VersionedXcm::from(xcm.into())),
			Weight::MAX,
		)
		.unwrap();
	});

	let ah_sender_balance_after =
		foreign_balance_on!(AssetHubZagros, foreign_asset_location_on_ah, &receiver);
	let penpal_receiver_balance_after = assets_balance_on!(PenpalA, new_asset_id, &sender);

	assert!(ah_sender_balance_after < ah_sender_balance_before);
	assert!(penpal_receiver_balance_after > penpal_receiver_balance_before);
}

// ==============================================================================================
// ==== Bidirectional Transfer - Reserve-based Foreign Asset - Penpal<->AssetHub ====
// ==============================================================================================
/// Transfers of foreign asset from Penpal to AssetHub and back. Foreign Asset is not registered
/// with Asset Hub as a trusted reserve, ergo teleports are not available and reserve-transfers are
/// to be used. Also verifies that teleporting the asset fails both ways.
#[test]
fn bidirectional_reserve_transfer_foreign_asset_between_penpal_and_asset_hub() {
	let sender = PenpalASender::get();
	let receiver = AssetHubZagrosReceiver::get();
	let new_asset_id: u32 = 42;
	let asset_location_on_penpal = local_penpal_asset(new_asset_id);
	let asset_amount_to_send = ASSET_HUB_ZAGROS_ED * 10_000;
	let (asset_location_on_penpal, foreign_asset_location_on_ah) =
		set_up_foreign_asset(sender.clone(), asset_location_on_penpal.clone(), asset_amount_to_send, false);

	////////////////////////////////////////
	// Reserve-transfer it from Penpal to AH
	////////////////////////////////////////

	let penpal_sender_balance_before = assets_balance_on!(PenpalA, new_asset_id, &sender);
	let ah_receiver_balance_before =
		foreign_balance_on!(AssetHubZagros, foreign_asset_location_on_ah.clone(), &receiver);

	let dest = PenpalA::sibling_location_of(AssetHubZagros::para_id());
	let assets: Assets =
		vec![(asset_location_on_penpal.clone(), asset_amount_to_send).into()].into();
	// execute xcm from penpal to asset hub
	PenpalA::execute_with(|| {
		// xcm to be executed at dest
		let xcm_on_dest = Xcm(vec![
			// since this is the last hop, we don't need to further use any assets previously
			// reserved for fees (there are no further hops to cover delivery fees for); we
			// RefundSurplus to get back any unspent fees
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: receiver.clone().into() },
		]);
		// teleporting the asset fails
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(assets.clone().into()),
			SetFeesMode { jit_withdraw: true },
			InitiateTransfer {
				destination: dest.clone(),
				remote_fees: Some(AssetTransferFilter::Teleport(assets.clone().into())),
				preserve_origin: false,
				assets: BoundedVec::new(),
				remote_xcm: xcm_on_dest.clone(),
			},
		]);
		assert!(matches!(
			<PenpalA as PenpalAPallet>::PezkuwiXcm::execute(
				<PenpalA as Chain>::RuntimeOrigin::signed(sender.clone()),
				bx!(xcm::VersionedXcm::from(xcm.into())),
				Weight::MAX,
			),
			Err(pezsp_runtime::DispatchErrorWithPostInfo { .. }),
		));
		// reserve-transferring the asset works
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(assets.clone().into()),
			SetFeesMode { jit_withdraw: true },
			InitiateTransfer {
				destination: dest,
				remote_fees: Some(AssetTransferFilter::ReserveDeposit(assets.into())),
				preserve_origin: false,
				assets: BoundedVec::new(),
				remote_xcm: xcm_on_dest,
			},
		]);
		assert_ok!(<PenpalA as PenpalAPallet>::PezkuwiXcm::execute(
			<PenpalA as Chain>::RuntimeOrigin::signed(sender.clone()),
			bx!(xcm::VersionedXcm::from(xcm.into())),
			Weight::MAX,
		));
	});

	let penpal_sender_balance_after = assets_balance_on!(PenpalA, new_asset_id, &sender);
	let ah_receiver_balance_after =
		foreign_balance_on!(AssetHubZagros, foreign_asset_location_on_ah.clone(), &receiver);

	assert!(penpal_sender_balance_after < penpal_sender_balance_before);
	assert!(ah_receiver_balance_after > ah_receiver_balance_before);

	/////////////////////////////////////////////
	// Reserve-transfer it back from AH to Penpal
	/////////////////////////////////////////////

	let asset_amount_to_send = ah_receiver_balance_after;
	let ah_sender_balance_before =
		foreign_balance_on!(AssetHubZagros, foreign_asset_location_on_ah.clone(), &receiver);
	let penpal_receiver_balance_before = assets_balance_on!(PenpalA, new_asset_id, &sender);

	let dest = AssetHubZagros::sibling_location_of(PenpalA::para_id());
	// execute xcm from asset hub to penpal
	AssetHubZagros::execute_with(|| {
		let assets: Assets =
			vec![(foreign_asset_location_on_ah.clone(), asset_amount_to_send).into()].into();
		// xcm to be executed at dest
		let xcm_on_dest = Xcm(vec![
			// since this is the last hop, we don't need to further use any assets previously
			// reserved for fees (there are no further hops to cover delivery fees for); we
			// RefundSurplus to get back any unspent fees
			RefundSurplus,
			DepositAsset { assets: Wild(All), beneficiary: sender.clone().into() },
		]);
		// teleporting the asset back to penpal fails
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(assets.clone().into()),
			SetFeesMode { jit_withdraw: true },
			InitiateTransfer {
				destination: dest.clone(),
				remote_fees: Some(AssetTransferFilter::Teleport(assets.clone().into())),
				preserve_origin: false,
				assets: BoundedVec::new(),
				remote_xcm: xcm_on_dest.clone(),
			},
		]);
		assert!(matches!(
			<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::execute(
				<AssetHubZagros as Chain>::RuntimeOrigin::signed(receiver.clone()),
				bx!(xcm::VersionedXcm::from(xcm.into())),
				Weight::MAX,
			),
			Err(pezsp_runtime::DispatchErrorWithPostInfo { .. }),
		));
		// but reserve-transferring it back works
		let xcm = Xcm::<()>(vec![
			WithdrawAsset(assets.clone().into()),
			SetFeesMode { jit_withdraw: true },
			InitiateTransfer {
				destination: dest,
				remote_fees: Some(AssetTransferFilter::ReserveWithdraw(assets.into())),
				preserve_origin: false,
				assets: BoundedVec::new(),
				remote_xcm: xcm_on_dest,
			},
		]);
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::execute(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(receiver.clone()),
			bx!(xcm::VersionedXcm::from(xcm.into())),
			Weight::MAX,
		));
	});

	let ah_sender_balance_after =
		foreign_balance_on!(AssetHubZagros, foreign_asset_location_on_ah, &receiver);
	let penpal_receiver_balance_after = assets_balance_on!(PenpalA, new_asset_id, &sender);

	assert!(ah_sender_balance_after < ah_sender_balance_before);
	assert!(penpal_receiver_balance_after > penpal_receiver_balance_before);
}

/// Verifies that foreign asset reserves can be only set by signed `Owner` account or through XCM
/// using remote `ManagerOrigin`.
#[test]
fn verify_foreign_asset_origin_checks() {
	let sender = PenpalASender::get();
	let new_asset_id: u32 = 42;
	let asset_location_on_penpal = local_penpal_asset(new_asset_id);
	let asset_amount_to_send = ASSET_HUB_ZAGROS_ED * 10_000;
	let (_, foreign_asset_location_on_ah) =
		set_up_foreign_asset(sender.clone(), asset_location_on_penpal.clone(), asset_amount_to_send, false);

	let penpal_sovereign = AssetHubZagros::sovereign_account_id_of(
		AssetHubZagros::sibling_location_of(PenpalA::para_id()),
	);
	let reserves_data = ForeignAssetReserveData {
		reserve: AssetHubZagros::sibling_location_of(PenpalA::para_id()),
		teleportable: true,
	};
	// Set asset reserves using signed `owner` account.
	let origin = <AssetHubZagros as Chain>::RuntimeOrigin::signed(penpal_sovereign);
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::set_reserves(
			origin,
			foreign_asset_location_on_ah.clone(),
			vec![reserves_data.clone()].try_into().expect("reserve list fits the pallet bound"),
		)
		.unwrap();
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::ReservesUpdated { asset_id, .. }) => {
					asset_id: *asset_id == foreign_asset_location_on_ah,
				},
			]
		);
	});
	// Now set asset reserves using some other signed account. It should fail.
	let origin = <AssetHubZagros as Chain>::RuntimeOrigin::signed(sender.clone());
	AssetHubZagros::execute_with(|| {
		assert!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::set_reserves(
			origin,
			foreign_asset_location_on_ah.clone(),
			vec![reserves_data].try_into().expect("reserve list fits the pallet bound"),
		)
		.is_err());
	});
	// Now set asset reserves using remote XCM from correct origin chain.
	// Use wrong `{origin, asset}` combination.
	let asset_id_on_ah =
		emulated_integration_tests_common::PenpalBPen2TeleportableAssetLocation::get();
	penpal_set_foreign_asset_reserves_on_asset_hub(asset_id_on_ah, vec![]);
	// Verify it failed.
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: false, .. }) => {},
			]
		);
	});
	// Verify it works when using right `{origin, asset}` combination.
	let asset_id_on_ah = foreign_asset_location_on_ah;
	penpal_set_foreign_asset_reserves_on_asset_hub(asset_id_on_ah.clone(), vec![]);
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		AssetHubZagros::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubZagros,
			vec![
				// Foreign Asset created
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::ReservesRemoved { asset_id }) => {
					asset_id: *asset_id == asset_id_on_ah,
				},
			]
		);
	});
}
