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

use crate::tests::{snowbridge_common::snowbridge_sovereign, *};
use emulated_integration_tests_common::{
	macros::Dmp,
	xcm_helpers::{find_all_mq_processed_ids, find_mq_processed_id, find_xcm_sent_message_id},
	xcm_pez_simulator::helpers::TopicIdTracker,
};
use xcm::latest::AssetTransferFilter;

fn send_assets_over_bridge<F: FnOnce()>(send_fn: F) {
	// fund the AHW's SA on BHW for paying bridge delivery fees
	BridgeHubZagros::fund_para_sovereign(AssetHubZagros::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	let local_asset_hub = PenpalB::sibling_location_of(AssetHubZagros::para_id());
	PenpalB::force_xcm_version(local_asset_hub.clone(), XCM_VERSION);
	AssetHubZagros::force_xcm_version(asset_hub_pezkuwichain_location(), XCM_VERSION);
	BridgeHubZagros::force_xcm_version(bridge_hub_pezkuwichain_location(), XCM_VERSION);

	// send message over bridge
	send_fn();

	// process and verify intermediary hops
	assert_bridge_hub_zagros_message_accepted(true);
	assert_bridge_hub_pezkuwichain_message_received();
}

fn set_up_wnds_for_penpal_zagros_through_ahw_to_ahr(
	sender: &AccountId,
	amount: u128,
) -> (Location, v5::Location) {
	let wnd_at_zagros_teyrchains = wnd_at_ah_zagros();
	let wnd_at_asset_hub_pezkuwichain = bridged_wnd_at_ah_pezkuwichain();
	let wnd_reserve = vec![(asset_hub_zagros_global_location(), false).into()];
	create_foreign_on_ah_pezkuwichain(wnd_at_asset_hub_pezkuwichain.clone(), true, wnd_reserve);
	create_pool_with_native_on!(
		AssetHubPezkuwichain,
		wnd_at_asset_hub_pezkuwichain.clone(),
		true,
		AssetHubPezkuwichainSender::get()
	);

	let penpal_location = AssetHubZagros::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_ahw = AssetHubZagros::sovereign_account_id_of(penpal_location);
	// fund Penpal's sovereign account on AssetHub
	AssetHubZagros::fund_accounts(vec![(sov_penpal_on_ahw.into(), amount * 2)]);
	// fund Penpal's sender account
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		wnd_at_zagros_teyrchains.clone(),
		sender.clone(),
		amount * 2,
	);
	(wnd_at_zagros_teyrchains, wnd_at_asset_hub_pezkuwichain)
}

fn send_assets_from_penpal_zagros_through_zagros_ah_to_pezkuwichain_ah(
	destination: Location,
	assets: (Assets, TransferType),
	fees: (AssetId, TransferType),
	custom_xcm_on_dest: Xcm<()>,
) {
	send_assets_over_bridge(|| {
		let sov_penpal_on_ahw = AssetHubZagros::sovereign_account_id_of(
			AssetHubZagros::sibling_location_of(PenpalB::para_id()),
		);
		// send message over bridge
		assert_ok!(PenpalB::execute_with(|| {
			let signed_origin = <PenpalB as Chain>::RuntimeOrigin::signed(PenpalBSender::get());
			<PenpalB as PenpalBPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
				signed_origin,
				bx!(destination.into()),
				bx!(assets.0.into()),
				bx!(assets.1),
				bx!(fees.0.into()),
				bx!(fees.1),
				bx!(VersionedXcm::from(custom_xcm_on_dest)),
				WeightLimit::Unlimited,
			)
		}));
		// verify intermediary AH Zagros hop
		AssetHubZagros::execute_with(|| {
			type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
			assert_expected_events!(
				AssetHubZagros,
				vec![
					// Amount to reserve transfer is withdrawn from Penpal's sovereign account
					RuntimeEvent::Balances(
						pezpallet_balances::Event::Burned { who, .. }
					) => {
						who: *who == sov_penpal_on_ahw.clone().into(),
					},
					// Amount deposited in AHR's sovereign account
					RuntimeEvent::Balances(pezpallet_balances::Event::Minted { who, .. }) => {
						who: *who == TreasuryAccount::get(),
					},
					RuntimeEvent::XcmpQueue(
						pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }
					) => {},
				]
			);
		});
	});
}

#[test]
/// Test transfer of ZGR, USDT and wETH from AssetHub Zagros to AssetHub Pezkuwichain.
///
/// This mix of assets should cover the whole range:
/// - native assets: ZGR,
/// - trust-based assets: USDT (exists only on Zagros, Pezkuwichain gets it from Zagros over
///   bridge),
/// - foreign asset / bridged asset (other bridge / Snowfork): wETH (bridged from Ethereum to Zagros
///   over Snowbridge, then bridged over to Pezkuwichain through this bridge).
fn send_wnds_usdt_and_weth_from_asset_hub_zagros_to_asset_hub_pezkuwichain() {
	let amount = ASSET_HUB_ZAGROS_ED * 1_000;
	let sender = AssetHubZagrosSender::get();
	let receiver = AssetHubPezkuwichainReceiver::get();
	let wnd_at_asset_hub_zagros = wnd_at_ah_zagros();
	let bridged_wnd_at_asset_hub_pezkuwichain = bridged_wnd_at_ah_pezkuwichain();
	let wnd_reserve = vec![(asset_hub_zagros_global_location(), false).into()];
	create_foreign_on_ah_pezkuwichain(
		bridged_wnd_at_asset_hub_pezkuwichain.clone(),
		true,
		wnd_reserve,
	);
	create_pool_with_native_on!(
		AssetHubPezkuwichain,
		bridged_wnd_at_asset_hub_pezkuwichain.clone(),
		true,
		AssetHubPezkuwichainSender::get()
	);

	////////////////////////////////////////////////////////////
	// Let's first send over just some ZGRs as a simple example
	////////////////////////////////////////////////////////////
	let sov_ahr_on_ahw = AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
		ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
		AssetHubPezkuwichain::para_id(),
	);
	let wnds_in_reserve_on_ahw_before =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	let sender_wnds_before = <AssetHubZagros as Chain>::account_data_of(sender.clone()).free;
	let receiver_wnds_before = foreign_balance_on_ah_pezkuwichain(
		bridged_wnd_at_asset_hub_pezkuwichain.clone(),
		&receiver,
	);

	// send WNDs, use them for fees
	send_assets_over_bridge(|| {
		let destination = asset_hub_pezkuwichain_location();
		let assets: Assets = (wnd_at_asset_hub_zagros, amount).into();
		let fee_idx = 0;
		let transfer_type = TransferType::LocalReserve;
		assert_ok!(send_assets_from_asset_hub_zagros(destination, assets, fee_idx, transfer_type));
	});

	// verify expected events on final destination
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// issue ZGRs on AHR
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == bridged_wnd_at_asset_hub_pezkuwichain,
					owner: *owner == receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_wnds_after = <AssetHubZagros as Chain>::account_data_of(sender.clone()).free;
	let receiver_wnds_after =
		foreign_balance_on_ah_pezkuwichain(bridged_wnd_at_asset_hub_pezkuwichain, &receiver);
	let wnds_in_reserve_on_ahw_after =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw).free;

	// Sender's balance is reduced
	assert!(sender_wnds_before > sender_wnds_after);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is increased by sent amount
	assert_eq!(wnds_in_reserve_on_ahw_after, wnds_in_reserve_on_ahw_before + amount);

	/////////////////////////////////////////////////////////////
	// Now let's send over USDTs + wETH (and pay fees with USDT)
	/////////////////////////////////////////////////////////////
	let usdt_at_asset_hub_zagros = usdt_at_ah_zagros();
	let bridged_usdt_at_asset_hub_pezkuwichain = bridged_usdt_at_ah_pezkuwichain();
	// wETH has same relative location on both Zagros and Pezkuwichain AssetHubs
	let bridged_weth_at_ah = weth_at_asset_hubs();

	// mint USDT in sender's account (USDT already created in genesis)
	AssetHubZagros::mint_asset(
		<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosAssetOwner::get()),
		USDT_ID,
		sender.clone(),
		amount * 2,
	);
	// create wETH at src and dest and prefund sender's account
	AssetHubZagros::mint_foreign_asset(
		<AssetHubZagros as Chain>::RuntimeOrigin::signed(snowbridge_sovereign()),
		bridged_weth_at_ah.clone(),
		sender.clone(),
		amount * 2,
	);
	let wnd_reserve = vec![(asset_hub_zagros_global_location(), false).into()];
	create_foreign_on_ah_pezkuwichain(
		bridged_usdt_at_asset_hub_pezkuwichain.clone(),
		true,
		wnd_reserve,
	);
	create_pool_with_native_on!(
		AssetHubPezkuwichain,
		bridged_usdt_at_asset_hub_pezkuwichain.clone(),
		true,
		AssetHubPezkuwichainSender::get()
	);

	let receiver_usdts_before = foreign_balance_on_ah_pezkuwichain(
		bridged_usdt_at_asset_hub_pezkuwichain.clone(),
		&receiver,
	);
	let receiver_weth_before =
		foreign_balance_on_ah_pezkuwichain(bridged_weth_at_ah.clone(), &receiver);

	// send USDTs and wETHs
	let assets: Assets = vec![
		(usdt_at_asset_hub_zagros.clone(), amount).into(),
		(Location::try_from(bridged_weth_at_ah.clone()).unwrap(), amount).into(),
	]
	.into();
	// use USDT for fees
	let fee: AssetId = usdt_at_asset_hub_zagros.into();

	// use the more involved transfer extrinsic
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(assets.len() as u32)),
		beneficiary: AccountId32Junction { network: None, id: receiver.clone().into() }.into(),
	}]);
	assert_ok!(AssetHubZagros::execute_with(|| {
		<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(sender.into()),
			bx!(asset_hub_pezkuwichain_location().into()),
			bx!(assets.into()),
			bx!(TransferType::LocalReserve),
			bx!(fee.into()),
			bx!(TransferType::LocalReserve),
			bx!(VersionedXcm::from(custom_xcm_on_dest)),
			WeightLimit::Unlimited,
		)
	}));
	// verify hops (also advances the message through the hops)
	assert_bridge_hub_zagros_message_accepted(true);
	assert_bridge_hub_pezkuwichain_message_received();
	AssetHubPezkuwichain::execute_with(|| {
		AssetHubPezkuwichain::assert_xcmp_queue_success(None);
	});

	let receiver_usdts_after =
		foreign_balance_on_ah_pezkuwichain(bridged_usdt_at_asset_hub_pezkuwichain, &receiver);
	let receiver_weth_after = foreign_balance_on_ah_pezkuwichain(bridged_weth_at_ah, &receiver);

	// Receiver's USDT balance is increased by almost `amount` (minus fees)
	assert!(receiver_usdts_after > receiver_usdts_before);
	assert!(receiver_usdts_after < receiver_usdts_before + amount);
	// Receiver's wETH balance is increased by sent amount
	assert_eq!(receiver_weth_after, receiver_weth_before + amount);
}

#[test]
/// Send bridged TYRs "back" from AssetHub Zagros to AssetHub Pezkuwichain.
fn send_back_rocs_from_asset_hub_zagros_to_asset_hub_pezkuwichain() {
	let prefund_amount = 10_000_000_000_000u128;
	let amount_to_send = ASSET_HUB_PEZKUWICHAIN_ED * 1_000;
	let sender = AssetHubZagrosSender::get();
	let receiver = AssetHubPezkuwichainReceiver::get();
	let bridged_roc_at_asset_hub_zagros = bridged_roc_at_ah_zagros();
	let reserves = vec![(asset_hub_pezkuwichain_location(), false).into()];
	let prefund_accounts = vec![(sender.clone(), prefund_amount)];
	create_foreign_on_ah_zagros(
		bridged_roc_at_asset_hub_zagros.clone(),
		true,
		reserves,
		prefund_accounts,
	);

	// fund the AHW's SA on AHR with the TYR tokens held in reserve
	let sov_ahw_on_ahr =
		AssetHubPezkuwichain::sovereign_account_of_teyrchain_on_other_global_consensus(
			ByGenesis(ZAGROS_GENESIS_HASH),
			AssetHubZagros::para_id(),
		);
	AssetHubPezkuwichain::fund_accounts(vec![(sov_ahw_on_ahr.clone(), prefund_amount)]);

	let rocs_in_reserve_on_ahr_before =
		<AssetHubPezkuwichain as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	assert_eq!(rocs_in_reserve_on_ahr_before, prefund_amount);

	let sender_rocs_before =
		foreign_balance_on_ah_zagros(bridged_roc_at_asset_hub_zagros.clone(), &sender);
	assert_eq!(sender_rocs_before, prefund_amount);
	let receiver_rocs_before =
		<AssetHubPezkuwichain as Chain>::account_data_of(receiver.clone()).free;

	// send back TYRs, use them for fees
	send_assets_over_bridge(|| {
		let destination = asset_hub_pezkuwichain_location();
		let assets: Assets = (bridged_roc_at_asset_hub_zagros.clone(), amount_to_send).into();
		let fee_idx = 0;
		let transfer_type = TransferType::DestinationReserve;
		assert_ok!(send_assets_from_asset_hub_zagros(destination, assets, fee_idx, transfer_type));
	});

	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// TYR is withdrawn from AHW's SA on AHR
				RuntimeEvent::Balances(
					pezpallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_ahw_on_ahr,
					amount: *amount == amount_to_send,
				},
				// TYRs deposited to beneficiary
				RuntimeEvent::Balances(pezpallet_balances::Event::Minted { who, .. }) => {
					who: *who == receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_rocs_after = foreign_balance_on_ah_zagros(bridged_roc_at_asset_hub_zagros, &sender);
	let receiver_rocs_after =
		<AssetHubPezkuwichain as Chain>::account_data_of(receiver.clone()).free;
	let rocs_in_reserve_on_ahr_after =
		<AssetHubPezkuwichain as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;

	// Sender's balance is reduced
	assert!(sender_rocs_before > sender_rocs_after);
	// Receiver's balance is increased
	assert!(receiver_rocs_after > receiver_rocs_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(rocs_in_reserve_on_ahr_after, rocs_in_reserve_on_ahr_before - amount_to_send);
}

#[test]
fn send_wnds_from_penpal_zagros_through_asset_hub_zagros_to_asset_hub_pezkuwichain() {
	let amount = ASSET_HUB_ZAGROS_ED * 10_000_000;
	let sender = PenpalBSender::get();
	let receiver = AssetHubPezkuwichainReceiver::get();
	let local_asset_hub = PenpalB::sibling_location_of(AssetHubZagros::para_id());
	let (wnd_at_zagros_teyrchains, wnd_at_asset_hub_pezkuwichain) =
		set_up_wnds_for_penpal_zagros_through_ahw_to_ahr(&sender, amount);

	let sov_ahr_on_ahw = AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
		ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
		AssetHubPezkuwichain::para_id(),
	);
	let wnds_in_reserve_on_ahw_before =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	let sender_wnds_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_zagros_teyrchains.clone(), &sender)
	});
	let receiver_wnds_before =
		foreign_balance_on_ah_pezkuwichain(wnd_at_asset_hub_pezkuwichain.clone(), &receiver);

	// Send ZGRs over bridge
	{
		let destination = asset_hub_pezkuwichain_location();
		let assets: Assets = (wnd_at_zagros_teyrchains.clone(), amount).into();
		let asset_transfer_type = TransferType::RemoteReserve(local_asset_hub.clone().into());
		let fees_id: AssetId = wnd_at_zagros_teyrchains.clone().into();
		let fees_transfer_type = TransferType::RemoteReserve(local_asset_hub.into());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
			assets: Wild(AllCounted(assets.len() as u32)),
			beneficiary,
		}]);
		send_assets_from_penpal_zagros_through_zagros_ah_to_pezkuwichain_ah(
			destination,
			(assets, asset_transfer_type),
			(fees_id, fees_transfer_type),
			custom_xcm_on_dest,
		);
	}

	// process AHR incoming message and check events
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// issue ZGRs on AHR
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == wnd_at_asset_hub_pezkuwichain.clone(),
					owner: owner == &receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_wnds_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_zagros_teyrchains, &sender)
	});
	let receiver_wnds_after =
		foreign_balance_on_ah_pezkuwichain(wnd_at_asset_hub_pezkuwichain, &receiver);
	let wnds_in_reserve_on_ahw_after =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;

	// Sender's balance is reduced
	assert!(sender_wnds_after < sender_wnds_before);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is increased by sent amount (less fess)
	assert!(wnds_in_reserve_on_ahw_after > wnds_in_reserve_on_ahw_before);
	assert!(wnds_in_reserve_on_ahw_after <= wnds_in_reserve_on_ahw_before + amount);
}

#[test]
fn send_wnds_from_penpal_zagros_through_asset_hub_zagros_to_asset_hub_pezkuwichain_to_penpal_pezkuwichain(
) {
	let amount = ASSET_HUB_ZAGROS_ED * 10_000_000;
	let sender = PenpalBSender::get();
	let receiver = PenpalAReceiver::get();
	let local_asset_hub = PenpalB::sibling_location_of(AssetHubZagros::para_id());
	// create foreign ZGR on remote paras
	let (wnd_at_zagros_teyrchains, wnd_at_pezkuwichain_teyrchains) =
		set_up_wnds_for_penpal_zagros_through_ahw_to_ahr(&sender, amount);
	let asset_owner: AccountId = AssetHubPezkuwichain::account_id_of(ALICE);
	// create foreign ZGR on remote paras
	PenpalA::force_create_foreign_asset(
		wnd_at_pezkuwichain_teyrchains.clone(),
		asset_owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);
	// Configure destination Penpal chain to trust its sibling AH as reserve of bridged ZGR
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				wnd_at_pezkuwichain_teyrchains.encode(),
			)],
		));
	});
	create_pool_with_native_on!(PenpalA, wnd_at_pezkuwichain_teyrchains.clone(), true, asset_owner);

	let sov_ahr_on_ahw = AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
		ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
		AssetHubPezkuwichain::para_id(),
	);
	let wnds_in_reserve_on_ahw_before =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	let sender_wnds_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_zagros_teyrchains.clone(), &sender)
	});
	let receiver_wnds_before = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(wnd_at_pezkuwichain_teyrchains.clone(), &receiver)
	});

	// Send ZGRs over bridge
	{
		let destination = asset_hub_pezkuwichain_location();
		let assets: Assets = (wnd_at_zagros_teyrchains.clone(), amount).into();
		let asset_transfer_type = TransferType::RemoteReserve(local_asset_hub.clone().into());
		let fees_id: AssetId = wnd_at_zagros_teyrchains.clone().into();
		let fees_transfer_type = TransferType::RemoteReserve(local_asset_hub.into());
		let remote_fees = (bridged_wnd_at_ah_pezkuwichain(), amount / 2).into();
		let beneficiary: Location =
			AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		let custom_xcm_on_penpal_dest = Xcm::<()>(vec![
			BuyExecution { fees: remote_fees, weight_limit: Unlimited },
			DepositAsset { assets: Wild(AllCounted(assets.len() as u32)), beneficiary },
		]);
		let pp_loc_from_ah = AssetHubPezkuwichain::sibling_location_of(PenpalA::para_id());
		let custom_xcm_on_remote_ah = Xcm::<()>(vec![
			// BuyExecution { fees: remote_fees, weight_limit: Unlimited },
			DepositReserveAsset {
				assets: Wild(AllCounted(1)),
				dest: pp_loc_from_ah,
				xcm: custom_xcm_on_penpal_dest,
			},
		]);
		send_assets_from_penpal_zagros_through_zagros_ah_to_pezkuwichain_ah(
			destination,
			(assets, asset_transfer_type),
			(fees_id, fees_transfer_type),
			custom_xcm_on_remote_ah,
		);
	}

	// process AHR incoming message and check events
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// issue ZGRs on AHR
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Issued { .. }) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});
	PenpalA::execute_with(|| {
		PenpalA::assert_xcmp_queue_success(None);
	});

	let sender_wnds_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_zagros_teyrchains, &sender)
	});
	let receiver_wnds_after = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(wnd_at_pezkuwichain_teyrchains, &receiver)
	});
	let wnds_in_reserve_on_ahw_after =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;

	// Sender's balance is reduced
	assert!(sender_wnds_after < sender_wnds_before);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is increased by sent amount (less fess)
	assert!(wnds_in_reserve_on_ahw_after > wnds_in_reserve_on_ahw_before);
	assert!(wnds_in_reserve_on_ahw_after <= wnds_in_reserve_on_ahw_before + amount);
}

#[test]
fn send_wnds_from_zagros_relay_through_asset_hub_zagros_to_asset_hub_pezkuwichain_to_penpal_pezkuwichain(
) {
	let amount = ZAGROS_ED * 100;
	let sender = ZagrosSender::get();
	let receiver = PenpalAReceiver::get();
	let local_asset_hub = Zagros::child_location_of(AssetHubZagros::para_id());

	let wnd_at_zagros_teyrchains = wnd_at_ah_zagros();
	let wnd_at_pezkuwichain_teyrchains = bridged_wnd_at_ah_pezkuwichain();
	let wnd_reserve = vec![(asset_hub_zagros_global_location(), false).into()];

	// create foreign ZGR on AH Pezkuwichain
	create_foreign_on_ah_pezkuwichain(wnd_at_pezkuwichain_teyrchains.clone(), true, wnd_reserve);
	create_pool_with_native_on!(
		AssetHubPezkuwichain,
		wnd_at_pezkuwichain_teyrchains.clone(),
		true,
		AssetHubPezkuwichainSender::get()
	);
	// create foreign ZGR on Penpal Pezkuwichain
	let asset_owner: AccountId = AssetHubPezkuwichain::account_id_of(ALICE);
	PenpalA::force_create_foreign_asset(
		wnd_at_pezkuwichain_teyrchains.clone(),
		asset_owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![],
	);
	// Configure destination Penpal chain to trust its sibling AH as reserve of bridged ZGR
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				wnd_at_pezkuwichain_teyrchains.encode(),
			)],
		));
	});
	create_pool_with_native_on!(PenpalA, wnd_at_pezkuwichain_teyrchains.clone(), true, asset_owner);

	Zagros::execute_with(|| {
		let root_origin = <Zagros as Chain>::RuntimeOrigin::root();
		<Zagros as ZagrosPallet>::XcmPallet::force_xcm_version(
			root_origin,
			bx!(local_asset_hub.clone()),
			XCM_VERSION,
		)
	})
	.unwrap();
	AssetHubPezkuwichain::force_xcm_version(
		AssetHubPezkuwichain::sibling_location_of(PenpalA::para_id()),
		XCM_VERSION,
	);

	let sov_ahr_on_ahw = AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
		ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
		AssetHubPezkuwichain::para_id(),
	);
	let wnds_in_reserve_on_ahw_before =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	let sender_wnds_before = <Zagros as Chain>::account_data_of(sender.clone()).free;
	let receiver_wnds_before = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(wnd_at_pezkuwichain_teyrchains.clone(), &receiver)
	});

	// Send ZGRs from Zagros to AHW over bridge to AHR then onto Penpal teyrchain
	{
		let beneficiary: Location =
			AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		// executes on Zagros Relay
		let dicle_xcm = Xcm::<()>(vec![
			WithdrawAsset((Location::here(), amount).into()),
			SetFeesMode { jit_withdraw: true },
			InitiateTeleport {
				assets: Wild(AllCounted(1)),
				dest: local_asset_hub,
				// executes on Zagros Asset Hub
				xcm: Xcm::<()>(vec![
					BuyExecution {
						fees: (wnd_at_zagros_teyrchains, amount / 2).into(),
						weight_limit: Unlimited,
					},
					DepositReserveAsset {
						assets: Wild(AllCounted(1)),
						dest: asset_hub_pezkuwichain_location(),
						// executes on Pezkuwichain Asset Hub
						xcm: Xcm::<()>(vec![
							BuyExecution {
								fees: (wnd_at_pezkuwichain_teyrchains.clone(), amount / 2).into(),
								weight_limit: Unlimited,
							},
							DepositReserveAsset {
								assets: Wild(AllCounted(1)),
								dest: AssetHubPezkuwichain::sibling_location_of(PenpalA::para_id()),
								// executes on Pezkuwichain Penpal
								xcm: Xcm::<()>(vec![
									BuyExecution {
										fees: (wnd_at_pezkuwichain_teyrchains.clone(), amount / 2)
											.into(),
										weight_limit: Unlimited,
									},
									DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
								]),
							},
						]),
					},
				]),
			},
		]);
		send_assets_over_bridge(|| {
			// send message over bridge
			assert_ok!(Zagros::execute_with(|| {
				Dmp::<<Zagros as Chain>::Runtime>::make_teyrchain_reachable(
					AssetHubZagros::para_id(),
				);
				let signed_origin = <Zagros as Chain>::RuntimeOrigin::signed(ZagrosSender::get());
				<Zagros as ZagrosPallet>::XcmPallet::execute(
					signed_origin,
					bx!(xcm::VersionedXcm::V5(dicle_xcm.into())),
					Weight::MAX,
				)
			}));
			AssetHubZagros::execute_with(|| {
				type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
				assert_expected_events!(
					AssetHubZagros,
					vec![
						// Amount deposited in AHR's sovereign account
						RuntimeEvent::Balances(pezpallet_balances::Event::Minted { who, .. }) => {
							who: *who == sov_ahr_on_ahw.clone().into(),
						},
						RuntimeEvent::XcmpQueue(
							pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }
						) => {},
					]
				);
			});
		});
	}

	// process AHR incoming message and check events
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// issue ZGRs on AHR
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Issued { .. }) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});
	PenpalA::execute_with(|| {
		PenpalA::assert_xcmp_queue_success(None);
	});

	let sender_wnds_after = <Zagros as Chain>::account_data_of(sender.clone()).free;
	let receiver_wnds_after = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(wnd_at_pezkuwichain_teyrchains, &receiver)
	});
	let wnds_in_reserve_on_ahw_after =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;

	// Sender's balance is reduced
	assert!(sender_wnds_after < sender_wnds_before);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is increased by sent amount (less fess)
	assert!(wnds_in_reserve_on_ahw_after > wnds_in_reserve_on_ahw_before);
	assert!(wnds_in_reserve_on_ahw_after <= wnds_in_reserve_on_ahw_before + amount);
}

#[test]
fn send_back_rocs_from_penpal_zagros_through_asset_hub_zagros_to_asset_hub_pezkuwichain() {
	let roc_at_zagros_teyrchains = bridged_roc_at_ah_zagros();
	let amount = ASSET_HUB_ZAGROS_ED * 10_000_000;
	let sender = PenpalBSender::get();
	let receiver = AssetHubPezkuwichainReceiver::get();

	// set up ZGRs for transfer
	let (wnd_at_zagros_teyrchains, _) =
		set_up_wnds_for_penpal_zagros_through_ahw_to_ahr(&sender, amount);

	// set up TYRs for transfer
	let penpal_location = AssetHubZagros::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_ahw = AssetHubZagros::sovereign_account_id_of(penpal_location);
	let reserves = vec![(asset_hub_pezkuwichain_location(), false).into()];
	let prefund_accounts = vec![(sov_penpal_on_ahw, amount * 2)];
	create_foreign_on_ah_zagros(roc_at_zagros_teyrchains.clone(), true, reserves, prefund_accounts);
	let asset_owner: AccountId = AssetHubZagros::account_id_of(ALICE);
	PenpalB::force_create_foreign_asset(
		roc_at_zagros_teyrchains.clone(),
		asset_owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![(sender.clone(), amount * 2)],
	);
	// Configure source Penpal chain to trust local AH as reserve of bridged TYR
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				roc_at_zagros_teyrchains.encode(),
			)],
		));
	});

	// fund the AHW's SA on AHR with the TYR tokens held in reserve
	let sov_ahw_on_ahr =
		AssetHubPezkuwichain::sovereign_account_of_teyrchain_on_other_global_consensus(
			ByGenesis(ZAGROS_GENESIS_HASH),
			AssetHubZagros::para_id(),
		);
	AssetHubPezkuwichain::fund_accounts(vec![(sov_ahw_on_ahr.clone(), amount * 2)]);

	// balances before
	let sender_rocs_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_at_zagros_teyrchains.clone().into(), &sender)
	});
	let receiver_rocs_before =
		<AssetHubPezkuwichain as Chain>::account_data_of(receiver.clone()).free;

	// send TYRs over the bridge, ZGRs only used to pay fees on local AH, pay with TYR on remote AH
	{
		let final_destination = asset_hub_pezkuwichain_location();
		let intermediary_hop = PenpalB::sibling_location_of(AssetHubZagros::para_id());
		let context = PenpalB::execute_with(|| PenpalUniversalLocation::get());

		// what happens at final destination
		let beneficiary = AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		// use TYR as fees on the final destination (AHW)
		let remote_fees: Asset = (roc_at_zagros_teyrchains.clone(), amount).into();
		let remote_fees = remote_fees.reanchored(&final_destination, &context).unwrap();
		// buy execution using TYRs, then deposit all remaining TYRs
		let xcm_on_final_dest = Xcm::<()>(vec![
			BuyExecution { fees: remote_fees, weight_limit: WeightLimit::Unlimited },
			DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
		]);

		// what happens at intermediary hop
		// reanchor final dest (Asset Hub Pezkuwichain) to the view of hop (Asset Hub Zagros)
		let mut final_destination = final_destination.clone();
		final_destination.reanchor(&intermediary_hop, &context).unwrap();
		// reanchor TYRs to the view of hop (Asset Hub Zagros)
		let asset: Asset = (roc_at_zagros_teyrchains.clone(), amount).into();
		let asset = asset.reanchored(&intermediary_hop, &context).unwrap();
		// on Asset Hub Zagros, forward a request to withdraw TYRs from reserve on Asset Hub
		// Pezkuwichain
		let xcm_on_hop = Xcm::<()>(vec![InitiateReserveWithdraw {
			assets: Definite(asset.into()), // TYRs
			reserve: final_destination,     // AHR
			xcm: xcm_on_final_dest,         // XCM to execute on AHR
		}]);
		// assets to send from Penpal and how they reach the intermediary hop
		let assets: Assets = vec![
			(roc_at_zagros_teyrchains.clone(), amount).into(),
			(wnd_at_zagros_teyrchains.clone(), amount).into(),
		]
		.into();
		let asset_transfer_type = TransferType::DestinationReserve;
		let fees_id: AssetId = wnd_at_zagros_teyrchains.into();
		let fees_transfer_type = TransferType::DestinationReserve;

		// initiate the transfer
		send_assets_from_penpal_zagros_through_zagros_ah_to_pezkuwichain_ah(
			intermediary_hop,
			(assets, asset_transfer_type),
			(fees_id, fees_transfer_type),
			xcm_on_hop,
		);
	}

	// process AHR incoming message and check events
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// issue ZGRs on AHR
				RuntimeEvent::Balances(pezpallet_balances::Event::Issued { .. }) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_rocs_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_at_zagros_teyrchains.into(), &sender)
	});
	let receiver_rocs_after = <AssetHubPezkuwichain as Chain>::account_data_of(receiver).free;

	// Sender's balance is reduced by sent "amount"
	assert_eq!(sender_rocs_after, sender_rocs_before - amount);
	// Receiver's balance is increased by no more than "amount"
	assert!(receiver_rocs_after > receiver_rocs_before);
	assert!(receiver_rocs_after <= receiver_rocs_before + amount);
}

#[test]
fn send_back_rocs_from_penpal_zagros_through_asset_hub_zagros_to_asset_hub_pezkuwichain_to_penpal_pezkuwichain(
) {
	let roc_at_zagros_teyrchains = bridged_roc_at_ah_zagros();
	let roc_at_pezkuwichain_teyrchains = Location::parent();
	let amount = ASSET_HUB_ZAGROS_ED * 10_000_000;
	let sender = PenpalBSender::get();
	let receiver = PenpalAReceiver::get();

	// set up TYRs for transfer
	let penpal_location = AssetHubZagros::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_ahw = AssetHubZagros::sovereign_account_id_of(penpal_location);
	let reserves = vec![(asset_hub_pezkuwichain_location(), false).into()];
	let prefund_accounts = vec![(sov_penpal_on_ahw.clone(), amount * 2)];
	create_foreign_on_ah_zagros(roc_at_zagros_teyrchains.clone(), true, reserves, prefund_accounts);
	create_pool_with_native_on!(
		AssetHubZagros,
		roc_at_zagros_teyrchains.clone(),
		true,
		AssetHubPezkuwichainSender::get()
	);
	let asset_owner: AccountId = AssetHubZagros::account_id_of(ALICE);
	// Fund ZGRs on Zagros Penpal
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		Location::parent(),
		sender.clone(),
		amount,
	);
	// Create and fund bridged TYRs on Zagros Penpal
	PenpalB::force_create_foreign_asset(
		roc_at_zagros_teyrchains.clone(),
		asset_owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![(sender.clone(), amount * 2)],
	);
	// Configure source Penpal chain to trust local AH as reserve of bridged TYR
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				roc_at_zagros_teyrchains.encode(),
			)],
		));
	});

	// fund the AHW's SA on AHR with the TYR tokens held in reserve
	let sov_ahw_on_ahr =
		AssetHubPezkuwichain::sovereign_account_of_teyrchain_on_other_global_consensus(
			ByGenesis(ZAGROS_GENESIS_HASH),
			AssetHubZagros::para_id(),
		);
	AssetHubPezkuwichain::fund_accounts(vec![(sov_ahw_on_ahr.clone(), amount * 2)]);

	// balances before
	let sender_rocs_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_at_zagros_teyrchains.clone().into(), &sender)
	});
	let receiver_rocs_before = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(roc_at_pezkuwichain_teyrchains.clone(), &receiver)
	});

	// send TYRs over the bridge, all fees paid with TYR along the way
	{
		let local_asset_hub = PenpalB::sibling_location_of(AssetHubZagros::para_id());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		// executes on Penpal Zagros
		let xcm = Xcm::<()>(vec![
			WithdrawAsset((roc_at_zagros_teyrchains.clone(), amount).into()),
			SetFeesMode { jit_withdraw: true },
			InitiateReserveWithdraw {
				assets: Wild(AllCounted(1)),
				reserve: local_asset_hub,
				// executes on Zagros Asset Hub
				xcm: Xcm::<()>(vec![
					BuyExecution {
						fees: (roc_at_zagros_teyrchains.clone(), amount / 2).into(),
						weight_limit: Unlimited,
					},
					InitiateReserveWithdraw {
						assets: Wild(AllCounted(1)),
						reserve: asset_hub_pezkuwichain_location(),
						// executes on Pezkuwichain Asset Hub
						xcm: Xcm::<()>(vec![
							BuyExecution {
								fees: (roc_at_pezkuwichain_teyrchains.clone(), amount / 2).into(),
								weight_limit: Unlimited,
							},
							DepositReserveAsset {
								assets: Wild(AllCounted(1)),
								dest: AssetHubPezkuwichain::sibling_location_of(PenpalA::para_id()),
								// executes on Pezkuwichain Penpal
								xcm: Xcm::<()>(vec![
									BuyExecution {
										fees: (roc_at_pezkuwichain_teyrchains.clone(), amount / 2)
											.into(),
										weight_limit: Unlimited,
									},
									DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
								]),
							},
						]),
					},
				]),
			},
		]);
		send_assets_over_bridge(|| {
			// send message over bridge
			assert_ok!(PenpalB::execute_with(|| {
				let signed_origin = <PenpalB as Chain>::RuntimeOrigin::signed(sender.clone());
				<PenpalB as PenpalBPallet>::PezkuwiXcm::execute(
					signed_origin,
					bx!(xcm::VersionedXcm::V5(xcm.into())),
					Weight::MAX,
				)
			}));
			AssetHubZagros::execute_with(|| {
				type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
				assert_expected_events!(
					AssetHubZagros,
					vec![
						// Amount to reserve transfer is withdrawn from Penpal's sovereign account
						RuntimeEvent::ForeignAssets(
							pezpallet_assets::Event::Burned { asset_id, owner, .. }
						) => {
							asset_id: asset_id == &roc_at_zagros_teyrchains,
							owner: owner == &sov_penpal_on_ahw,
						},
						RuntimeEvent::XcmpQueue(
							pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }
						) => {},
						// message processed successfully
						RuntimeEvent::MessageQueue(
							pezpallet_message_queue::Event::Processed { success: true, .. }
						) => {},
					]
				);
			});
		});
	}

	// process AHR incoming message and check events
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// burn TYRs from AHW's SA on AHR
				RuntimeEvent::Balances(
					pezpallet_balances::Event::Burned { who, .. }
				) => {
					who: *who == sov_ahw_on_ahr.clone().into(),
				},
				// sent message to sibling Penpal
				RuntimeEvent::XcmpQueue(
					pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});
	PenpalA::execute_with(|| {
		PenpalA::assert_xcmp_queue_success(None);
	});

	let sender_rocs_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_at_zagros_teyrchains.into(), &sender)
	});
	let receiver_rocs_after = PenpalA::execute_with(|| {
		type Assets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(roc_at_pezkuwichain_teyrchains.clone(), &receiver)
	});

	// Sender's balance is reduced by sent "amount"
	assert_eq!(sender_rocs_after, sender_rocs_before - amount);
	// Receiver's balance is increased by no more than "amount"
	assert!(receiver_rocs_after > receiver_rocs_before);
	assert!(receiver_rocs_after <= receiver_rocs_before + amount);
}

#[test]
fn send_back_rocs_from_penpal_zagros_through_asset_hub_zagros_to_asset_hub_pezkuwichain_to_pezkuwichain_relay(
) {
	let roc_at_zagros_teyrchains = bridged_roc_at_ah_zagros();
	let roc_at_pezkuwichain_teyrchains = Location::parent();
	let amount = ASSET_HUB_ZAGROS_ED * 10_000_000;
	let sender = PenpalBSender::get();
	let receiver = PezkuwichainReceiver::get();
	let mut topic_id_tracker = TopicIdTracker::new();

	// set up TYRs for transfer
	let penpal_location = AssetHubZagros::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_ahw = AssetHubZagros::sovereign_account_id_of(penpal_location);
	let reserves = vec![(asset_hub_pezkuwichain_location(), false).into()];
	let prefund_accounts = vec![(sov_penpal_on_ahw.clone(), amount * 2)];
	create_foreign_on_ah_zagros(roc_at_zagros_teyrchains.clone(), true, reserves, prefund_accounts);
	create_pool_with_native_on!(
		AssetHubZagros,
		roc_at_zagros_teyrchains.clone(),
		true,
		AssetHubPezkuwichainSender::get()
	);
	let asset_owner: AccountId = AssetHubZagros::account_id_of(ALICE);
	// Fund ZGRs on Zagros Penpal
	PenpalB::mint_foreign_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		Location::parent(),
		sender.clone(),
		amount,
	);
	// Create and fund bridged TYRs on Zagros Penpal
	PenpalB::force_create_foreign_asset(
		roc_at_zagros_teyrchains.clone(),
		asset_owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![(sender.clone(), amount * 2)],
	);
	// Configure source Penpal chain to trust local AH as reserve of bridged TYR
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				roc_at_zagros_teyrchains.encode(),
			)],
		));
	});

	// fund the AHW's SA on AHR with the TYR tokens held in reserve
	let sov_ahw_on_ahr =
		AssetHubPezkuwichain::sovereign_account_of_teyrchain_on_other_global_consensus(
			ByGenesis(ZAGROS_GENESIS_HASH),
			AssetHubZagros::para_id(),
		);
	AssetHubPezkuwichain::fund_accounts(vec![(sov_ahw_on_ahr.clone(), amount * 2)]);

	// fund Pezkuwichain Relay check account so we can teleport back to it
	Pezkuwichain::fund_accounts(vec![(
		<Pezkuwichain as PezkuwichainPallet>::XcmPallet::check_account(),
		amount,
	)]);

	// balances before
	let sender_rocs_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_at_zagros_teyrchains.clone().into(), &sender)
	});
	let receiver_rocs_before = <Pezkuwichain as Chain>::account_data_of(receiver.clone()).free;

	// send TYRs over the bridge, all fees paid with TYR along the way
	{
		let local_asset_hub = PenpalB::sibling_location_of(AssetHubZagros::para_id());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		// executes on Penpal Zagros
		let xcm = Xcm::<()>(vec![
			WithdrawAsset((roc_at_zagros_teyrchains.clone(), amount).into()),
			SetFeesMode { jit_withdraw: true },
			InitiateReserveWithdraw {
				assets: Wild(AllCounted(1)),
				reserve: local_asset_hub,
				// executes on Zagros Asset Hub
				xcm: Xcm::<()>(vec![
					BuyExecution {
						fees: (roc_at_zagros_teyrchains.clone(), amount / 2).into(),
						weight_limit: Unlimited,
					},
					InitiateReserveWithdraw {
						assets: Wild(AllCounted(1)),
						reserve: asset_hub_pezkuwichain_location(),
						// executes on Pezkuwichain Asset Hub
						xcm: Xcm::<()>(vec![
							BuyExecution {
								fees: (roc_at_pezkuwichain_teyrchains.clone(), amount / 2).into(),
								weight_limit: Unlimited,
							},
							InitiateTeleport {
								assets: Wild(AllCounted(1)),
								dest: Location::parent(),
								// executes on Pezkuwichain Relay
								xcm: Xcm::<()>(vec![
									BuyExecution {
										fees: (Location::here(), amount / 2).into(),
										weight_limit: Unlimited,
									},
									DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
								]),
							},
						]),
					},
				]),
			},
		]);
		send_assets_over_bridge(|| {
			// send message over bridge
			assert_ok!(PenpalB::execute_with(|| {
				let signed_origin = <PenpalB as Chain>::RuntimeOrigin::signed(sender.clone());
				let result = <PenpalB as PenpalBPallet>::PezkuwiXcm::execute(
					signed_origin,
					bx!(xcm::VersionedXcm::V5(xcm.into())),
					Weight::MAX,
				);

				let msg_sent_id =
					find_xcm_sent_message_id::<PenpalB>().expect("Missing Sent Event on PenpalB");
				topic_id_tracker.insert("PenpalB", msg_sent_id.into());

				result
			}));
			AssetHubZagros::execute_with(|| {
				type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
				assert_expected_events!(
					AssetHubZagros,
					vec![
						// Amount to reserve transfer is withdrawn from Penpal's sovereign account
						RuntimeEvent::ForeignAssets(
							pezpallet_assets::Event::Burned { asset_id, owner, .. }
						) => {
							asset_id: asset_id == &roc_at_zagros_teyrchains,
							owner: owner == &sov_penpal_on_ahw,
						},
						RuntimeEvent::XcmpQueue(
							pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }
						) => {},
						// message processed successfully
						RuntimeEvent::MessageQueue(
							pezpallet_message_queue::Event::Processed { success: true, .. }
						) => {},
					]
				);
				let mq_prc_ids = find_all_mq_processed_ids::<AssetHubZagros>();
				assert!(!mq_prc_ids.is_empty(), "Missing Processed Event on AssetHubZagros");
				topic_id_tracker.insert_all("AssetHubZagros", &mq_prc_ids);
			});
		});
	}

	// process AHR incoming message and check events
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// burn TYRs from AHW's SA on AHR
				RuntimeEvent::Balances(
					pezpallet_balances::Event::Burned { who, .. }
				) => {
					who: *who == sov_ahw_on_ahr.clone().into(),
				},
				// sent message to Pezkuwichain Relay
				RuntimeEvent::TeyrchainSystem(
					pezcumulus_pezpallet_teyrchain_system::Event::UpwardMessageSent { .. }
				) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
		let mq_prc_ids = find_all_mq_processed_ids::<AssetHubPezkuwichain>();
		assert!(!mq_prc_ids.is_empty(), "Missing Processed Event on AssetHubPezkuwichain");
		topic_id_tracker.insert_all("AssetHubPezkuwichain", &mq_prc_ids);
	});
	topic_id_tracker.assert_only_id_seen_on_all_chains("PenpalB");

	let sender_rocs_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_at_zagros_teyrchains.into(), &sender)
	});
	let receiver_rocs_after = <Pezkuwichain as Chain>::account_data_of(receiver.clone()).free;

	// Sender's balance is reduced by sent "amount"
	assert_eq!(sender_rocs_after, sender_rocs_before - amount);
	// Receiver's balance is increased by no more than "amount"
	assert!(receiver_rocs_after > receiver_rocs_before);
	assert!(receiver_rocs_after <= receiver_rocs_before + amount);
}

#[test]
fn dry_run_transfer_to_pezkuwichain_sends_xcm_to_bridge_hub() {
	test_dry_run_transfer_across_pk_bridge!(
		AssetHubZagros,
		BridgeHubZagros,
		asset_hub_pezkuwichain_location()
	);
}

fn do_send_pens_and_wnds_from_penpal_zagros_via_ahw_to_asset_hub_pezkuwichain(
	topic_id_tracker: &mut TopicIdTracker,
	wnds: (Location, u128),
	pens: (Location, u128),
) {
	let (wnds_id, wnds_amount) = wnds;
	let (pens_id, pens_amount) = pens;
	send_assets_over_bridge(|| {
		let sov_penpal_on_ahw = AssetHubZagros::sovereign_account_id_of(
			AssetHubZagros::sibling_location_of(PenpalB::para_id()),
		);
		let sov_ahr_on_ahw =
			AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
				ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
				AssetHubPezkuwichain::para_id(),
			);
		let ahw_fee_amount = 120_000_000_000;
		// send message over bridge
		assert_ok!(PenpalB::execute_with(|| {
			let destination = asset_hub_pezkuwichain_location();
			let local_asset_hub = PenpalB::sibling_location_of(AssetHubZagros::para_id());
			let signed_origin = <PenpalB as Chain>::RuntimeOrigin::signed(PenpalBSender::get());
			let beneficiary: Location = AccountId32Junction {
				network: None,
				id: AssetHubPezkuwichainReceiver::get().into(),
			}
			.into();
			let wnds: Asset = (wnds_id.clone(), wnds_amount).into();
			let pens: Asset = (pens_id, pens_amount).into();
			let assets: Assets = vec![wnds.clone(), pens.clone()].into();

			// TODO: dry-run to get exact fees, for now just some static value 100_000_000_000
			let penpal_fees_amount = 100_000_000_000;
			// use 100_000_000_000 ZGRs in fees on AHW
			// (exec fees: 3_593_000_000, transpo fees: 69_021_561_290 = 72_614_561_290)
			// TODO: make this exact once we have bridge dry-running

			// XCM to be executed at dest (Pezkuwichain Asset Hub)
			let xcm_on_dest = Xcm(vec![
				// since this is the last hop, we don't need to further use any assets previously
				// reserved for fees (there are no further hops to cover delivery fees for); we
				// RefundSurplus to get back any unspent fees
				RefundSurplus,
				// deposit everything to final beneficiary
				DepositAsset { assets: Wild(All), beneficiary: beneficiary.clone() },
			]);

			// XCM to be executed at (intermediary) Zagros Asset Hub
			let context = PenpalUniversalLocation::get();
			let reanchored_dest =
				destination.clone().reanchored(&local_asset_hub, &context).unwrap();
			let reanchored_pens = pens.clone().reanchored(&local_asset_hub, &context).unwrap();
			let mut onward_wnds = wnds.clone().reanchored(&local_asset_hub, &context).unwrap();
			onward_wnds.fun = Fungible(wnds_amount - ahw_fee_amount - penpal_fees_amount);
			let xcm_on_ahw = Xcm(vec![
				// both ZGRs and PENs are local-reserve transferred to Pezkuwichain Asset Hub
				// initially, all ZGRs are reserved for fees on destination, but at the end of the
				// program we RefundSurplus to get back any unspent and deposit them to final
				// beneficiary
				InitiateTransfer {
					destination: reanchored_dest,
					remote_fees: Some(AssetTransferFilter::ReserveDeposit(onward_wnds.into())),
					preserve_origin: false,
					assets: BoundedVec::truncate_from(vec![AssetTransferFilter::ReserveDeposit(
						reanchored_pens.into(),
					)]),
					remote_xcm: xcm_on_dest,
				},
			]);

			let penpal_fees = (wnds.id.clone(), Fungible(penpal_fees_amount));
			let ahw_fees: Asset = (wnds.id.clone(), Fungible(ahw_fee_amount)).into();
			let ahw_non_fees_wnds: Asset =
				(wnds.id.clone(), Fungible(wnds_amount - ahw_fee_amount - penpal_fees_amount))
					.into();
			// XCM to be executed locally
			let xcm = Xcm::<()>(vec![
				// Withdraw both ZGRs and PENs from origin account
				WithdrawAsset(assets.into()),
				PayFees { asset: penpal_fees.into() },
				// Execute the transfers while paying remote fees with WNDs
				InitiateTransfer {
					destination: local_asset_hub,
					// ZGRs for fees are reserve-withdrawn at AHW and reserved for fees
					remote_fees: Some(AssetTransferFilter::ReserveWithdraw(ahw_fees.into())),
					preserve_origin: false,
					// PENs are teleported to AHW, rest of non-fee ZGRs are reserve-withdrawn at AHW
					assets: BoundedVec::truncate_from(vec![
						AssetTransferFilter::Teleport(pens.into()),
						AssetTransferFilter::ReserveWithdraw(ahw_non_fees_wnds.into()),
					]),
					remote_xcm: xcm_on_ahw,
				},
			]);

			let result = <PenpalB as PenpalBPallet>::PezkuwiXcm::execute(
				signed_origin,
				bx!(xcm::VersionedXcm::V5(xcm.into())),
				Weight::MAX,
			);

			let msg_sent_id = find_xcm_sent_message_id::<PenpalB>().expect("Missing Sent Event");
			topic_id_tracker.insert_and_assert_unique("PenpalB", msg_sent_id.into());

			result
		}));
		AssetHubZagros::execute_with(|| {
			type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
			let mq_prc_id =
				find_mq_processed_id::<AssetHubZagros>().expect("Missing Processed Event");
			topic_id_tracker.insert_and_assert_unique("AssetHubZagros", mq_prc_id);
			let msg_sent_id =
				find_xcm_sent_message_id::<AssetHubZagros>().expect("Missing Sent Event");
			topic_id_tracker.insert_and_assert_unique("AssetHubZagros", msg_sent_id.into());
			assert_expected_events!(
				AssetHubZagros,
				vec![
					// Amount to reserve transfer is withdrawn from Penpal's sovereign account
					RuntimeEvent::Balances(
						pezpallet_balances::Event::Burned { who, amount }
					) => {
						who: *who == sov_penpal_on_ahw.clone().into(),
						amount: *amount == ahw_fee_amount,
					},
					// Amount deposited in AHR's sovereign account
					RuntimeEvent::Balances(pezpallet_balances::Event::Minted { who, .. }) => {
						who: *who == sov_ahr_on_ahw.clone().into(),
					},
					RuntimeEvent::XcmpQueue(
						pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }
					) => {},
				]
			);
		});

		BridgeHubZagros::ext_wrapper(|| {
			let mq_prc_id =
				find_mq_processed_id::<BridgeHubZagros>().expect("Missing Processed Event");
			topic_id_tracker.insert_and_assert_unique("BridgeHubZagros", mq_prc_id);
		});
	});
}

/// Transfer "PEN"s plus "ZGR"s from PenpalZagros to AssetHubZagros, over bridge to
/// AssetHubPezkuwichain. PENs need to be teleported to AHW, while ZGRs reserve-withdrawn, then both
/// reserve transferred further to AHR. (transfer 2 different assets with different transfer types
/// across 3 different chains)
#[test]
fn send_pens_and_wnds_from_penpal_zagros_via_ahw_to_ahr() {
	let penpal_check_account = <PenpalB as PenpalBPallet>::PezkuwiXcm::check_account();
	let owner: AccountId = AssetHubPezkuwichain::account_id_of(ALICE);
	let sender = PenpalBSender::get();
	let amount = ASSET_HUB_ZAGROS_ED * 10_000_000;

	let (wnd_at_zagros_teyrchains, wnd_at_pezkuwichain_teyrchains) =
		set_up_wnds_for_penpal_zagros_through_ahw_to_ahr(&sender, amount);

	let pens_location_on_penpal = PenpalB::execute_with(|| {
		Location::try_from(PenpalLocalTeleportableToAssetHub::get()).unwrap()
	});
	let pens_id_on_penpal = match pens_location_on_penpal.last() {
		Some(Junction::GeneralIndex(id)) => *id as u32,
		_ => unreachable!(),
	};

	let penpal_teyrchain_junction = Junction::Teyrchain(PenpalB::para_id().into());
	let pens_at_ahw = Location::new(
		1,
		pens_location_on_penpal
			.interior()
			.clone()
			.pushed_front_with(penpal_teyrchain_junction)
			.unwrap(),
	);
	let pens_at_pezkuwichain_teyrchains = Location::new(
		2,
		pens_at_ahw
			.interior()
			.clone()
			.pushed_front_with(Junction::GlobalConsensus(NetworkId::ByGenesis(ZAGROS_GENESIS_HASH)))
			.unwrap(),
	);
	let wnds_to_send = amount;
	let pens_to_send = amount;

	// ---------- Set up Penpal Zagros ----------
	// Fund Penpal's sender account. No need to create the asset (only mint), it exists in genesis.
	PenpalB::mint_asset(
		<PenpalB as Chain>::RuntimeOrigin::signed(owner.clone()),
		pens_id_on_penpal,
		sender.clone(),
		pens_to_send * 2,
	);
	// fund Penpal's check account to be able to teleport
	PenpalB::fund_accounts(vec![(penpal_check_account.clone().into(), pens_to_send * 2)]);

	// ---------- Set up Asset Hub Pezkuwichain ----------
	// create PEN at AHR
	AssetHubPezkuwichain::force_create_foreign_asset(
		pens_at_pezkuwichain_teyrchains.clone(),
		owner.clone(),
		false,
		ASSET_MIN_BALANCE,
		vec![],
	);

	// account balances before
	let sender_wnds_before = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			wnd_at_zagros_teyrchains.clone().into(),
			&PenpalBSender::get(),
		)
	});
	let sender_pens_before = PenpalB::execute_with(|| {
		type Assets = <PenpalB as PenpalBPallet>::Assets;
		<Assets as Inspect<_>>::balance(pens_id_on_penpal, &PenpalBSender::get())
	});
	let sov_ahr_on_ahw = AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
		ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
		AssetHubPezkuwichain::para_id(),
	);
	let wnds_in_reserve_on_ahw_before =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	let pens_in_reserve_on_ahw_before = AssetHubZagros::execute_with(|| {
		type ForeignAssets = <AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(pens_at_ahw.clone(), &sov_ahr_on_ahw)
	});
	let receiver_wnds_before = AssetHubPezkuwichain::execute_with(|| {
		type Assets = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			wnd_at_pezkuwichain_teyrchains.clone(),
			&AssetHubPezkuwichainReceiver::get(),
		)
	});
	let receiver_pens_before = AssetHubPezkuwichain::execute_with(|| {
		type Assets = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			pens_at_pezkuwichain_teyrchains.clone(),
			&AssetHubPezkuwichainReceiver::get(),
		)
	});

	// init topic ID tracker
	let mut topic_id_tracker = TopicIdTracker::new();

	// transfer assets
	do_send_pens_and_wnds_from_penpal_zagros_via_ahw_to_asset_hub_pezkuwichain(
		&mut topic_id_tracker,
		(wnd_at_zagros_teyrchains.clone(), wnds_to_send),
		(pens_location_on_penpal.try_into().unwrap(), pens_to_send),
	);

	let wnd = Location::new(2, [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH))]);
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		let mq_prc_ids = find_all_mq_processed_ids::<AssetHubPezkuwichain>();
		assert!(!mq_prc_ids.is_empty(), "Missing Processed Event on AssetHubPezkuwichain");
		topic_id_tracker.insert_all("AssetHubPezkuwichain", &mq_prc_ids);
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				// issue ZGRs on AHR
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == wnd,
					owner: *owner == AssetHubPezkuwichainReceiver::get(),
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	// assert that the only topic ID on 'PenpalB' exists on all chains
	topic_id_tracker.assert_only_id_seen_on_all_chains("PenpalB");

	// account balances after
	let sender_wnds_after = PenpalB::execute_with(|| {
		type ForeignAssets = <PenpalB as PenpalBPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			wnd_at_zagros_teyrchains.into(),
			&PenpalBSender::get(),
		)
	});
	let sender_pens_after = PenpalB::execute_with(|| {
		type Assets = <PenpalB as PenpalBPallet>::Assets;
		<Assets as Inspect<_>>::balance(pens_id_on_penpal, &PenpalBSender::get())
	});
	let wnds_in_reserve_on_ahw_after =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	let pens_in_reserve_on_ahw_after = AssetHubZagros::execute_with(|| {
		type ForeignAssets = <AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(pens_at_ahw, &sov_ahr_on_ahw)
	});
	let receiver_wnds_after = AssetHubPezkuwichain::execute_with(|| {
		type Assets = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			wnd_at_pezkuwichain_teyrchains.clone(),
			&AssetHubPezkuwichainReceiver::get(),
		)
	});
	let receiver_pens_after = AssetHubPezkuwichain::execute_with(|| {
		type Assets = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(
			pens_at_pezkuwichain_teyrchains,
			&AssetHubPezkuwichainReceiver::get(),
		)
	});

	// Sender's balance is reduced
	assert!(sender_wnds_after < sender_wnds_before);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is increased by sent amount (less fess)
	assert!(wnds_in_reserve_on_ahw_after > wnds_in_reserve_on_ahw_before);
	assert!(wnds_in_reserve_on_ahw_after <= wnds_in_reserve_on_ahw_before + wnds_to_send);

	// Sender's balance is reduced by sent amount
	assert_eq!(sender_pens_after, sender_pens_before - pens_to_send);
	// Reserve balance is increased by sent amount
	assert_eq!(pens_in_reserve_on_ahw_after, pens_in_reserve_on_ahw_before + pens_to_send);
	// Receiver's balance is increased by sent amount
	assert_eq!(receiver_pens_after, receiver_pens_before + pens_to_send);
}
