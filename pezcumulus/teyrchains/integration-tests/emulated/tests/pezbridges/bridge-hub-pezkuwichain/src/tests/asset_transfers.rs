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

use crate::tests::*;

fn send_assets_over_bridge<F: FnOnce()>(send_fn: F) {
	// fund the AHR's SA on BHR for paying bridge delivery fees
	BridgeHubPezkuwichain::fund_para_sovereign(
		AssetHubPezkuwichain::para_id(),
		10_000_000_000_000u128,
	);

	// set XCM versions
	let local_asset_hub = PenpalA::sibling_location_of(AssetHubPezkuwichain::para_id());
	PenpalA::force_xcm_version(local_asset_hub.clone(), XCM_VERSION);
	AssetHubPezkuwichain::force_xcm_version(asset_hub_zagros_location(), XCM_VERSION);
	BridgeHubPezkuwichain::force_xcm_version(bridge_hub_zagros_location(), XCM_VERSION);

	// send message over bridge
	send_fn();

	// process and verify intermediary hops
	assert_bridge_hub_pezkuwichain_message_accepted(true);
	assert_bridge_hub_zagros_message_received();
}

fn set_up_rocs_for_penpal_pezkuwichain_through_ahr_to_ahw(
	sender: &AccountId,
	amount: u128,
) -> (Location, v5::Location) {
	let roc_at_pezkuwichain_teyrchains = roc_at_ah_pezkuwichain();
	let roc_at_asset_hub_zagros = bridged_roc_at_ah_zagros();
	let reserves = vec![(asset_hub_pezkuwichain_global_location(), false).into()];
	create_foreign_on_ah_zagros(roc_at_asset_hub_zagros.clone(), true, reserves);

	let penpal_location = AssetHubPezkuwichain::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubPezkuwichain::sovereign_account_id_of(penpal_location);
	// fund Penpal's sovereign account on AssetHub
	AssetHubPezkuwichain::fund_accounts(vec![(sov_penpal_on_ahr.into(), amount * 2)]);
	// fund Penpal's sender account
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		roc_at_pezkuwichain_teyrchains.clone(),
		sender.clone(),
		amount * 2,
	);
	(roc_at_pezkuwichain_teyrchains, roc_at_asset_hub_zagros)
}

fn send_assets_from_penpal_pezkuwichain_through_pezkuwichain_ah_to_zagros_ah(
	destination: Location,
	assets: (Assets, TransferType),
	fees: (AssetId, TransferType),
	custom_xcm_on_dest: Xcm<()>,
) {
	send_assets_over_bridge(|| {
		let sov_penpal_on_ahr = AssetHubPezkuwichain::sovereign_account_id_of(
			AssetHubPezkuwichain::sibling_location_of(PenpalA::para_id()),
		);
		// send message over bridge
		assert_ok!(PenpalA::execute_with(|| {
			let signed_origin = <PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get());
			<PenpalA as PenpalAPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
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
		// verify intermediary AH Pezkuwichain hop
		AssetHubPezkuwichain::execute_with(|| {
			type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
			assert_expected_events!(
				AssetHubPezkuwichain,
				vec![
					// Amount to reserve transfer is withdrawn from Penpal's sovereign account
					RuntimeEvent::Balances(
						pezpallet_balances::Event::Burned { who, .. }
					) => {
						who: *who == sov_penpal_on_ahr.clone().into(),
					},
					// Amount deposited in AHW's sovereign account
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
/// Test transfer of TYR from AssetHub Pezkuwichain to AssetHub Zagros.
fn send_roc_from_asset_hub_pezkuwichain_to_asset_hub_zagros() {
	let amount = ASSET_HUB_PEZKUWICHAIN_ED * 1_000_000;
	let sender = AssetHubPezkuwichainSender::get();
	let receiver = AssetHubZagrosReceiver::get();
	let roc_at_asset_hub_pezkuwichain = roc_at_ah_pezkuwichain();
	let bridged_roc_at_asset_hub_zagros = bridged_roc_at_ah_zagros();
	let reserves = vec![(asset_hub_pezkuwichain_global_location(), false).into()];
	create_foreign_on_ah_zagros(bridged_roc_at_asset_hub_zagros.clone(), true, reserves);
	set_up_pool_with_wnd_on_ah_zagros(bridged_roc_at_asset_hub_zagros.clone(), true);

	let sov_ahw_on_ahr =
		AssetHubPezkuwichain::sovereign_account_of_teyrchain_on_other_global_consensus(
			ByGenesis(ZAGROS_GENESIS_HASH),
			AssetHubZagros::para_id(),
		);
	let rocs_in_reserve_on_ahr_before =
		<AssetHubPezkuwichain as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let sender_rocs_before = <AssetHubPezkuwichain as Chain>::account_data_of(sender.clone()).free;
	let receiver_rocs_before =
		foreign_balance_on_ah_zagros(bridged_roc_at_asset_hub_zagros.clone(), &receiver);

	// send TYRs, use them for fees
	send_assets_over_bridge(|| {
		let destination = asset_hub_zagros_location();
		let assets: Assets = (roc_at_asset_hub_pezkuwichain.clone(), amount).into();
		let fee_idx = 0;
		let transfer_type = TransferType::LocalReserve;
		assert_ok!(send_assets_from_asset_hub_pezkuwichain(
			destination,
			assets,
			fee_idx,
			transfer_type
		));
	});

	// verify expected events on final destination
	let roc = Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH))]);
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				// issue TYRs on AHW
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == roc,
					owner: owner == &receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_rocs_after = <AssetHubPezkuwichain as Chain>::account_data_of(sender.clone()).free;
	let receiver_rocs_after =
		foreign_balance_on_ah_zagros(bridged_roc_at_asset_hub_zagros, &receiver);
	let rocs_in_reserve_on_ahr_after =
		<AssetHubPezkuwichain as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;

	// Sender's TYR balance is reduced
	assert!(sender_rocs_before > sender_rocs_after);
	// Receiver's TYR balance is increased
	assert!(receiver_rocs_after > receiver_rocs_before);
	// Reserve TYR balance is increased by sent amount
	assert_eq!(rocs_in_reserve_on_ahr_after, rocs_in_reserve_on_ahr_before + amount);
}

#[test]
/// Send bridged assets "back" from AssetHub Pezkuwichain to AssetHub Zagros.
///
/// This mix of assets should cover the whole range:
/// - bridged native assets: TYR,
/// - bridged trust-based assets: USDT (exists only on Zagros, Pezkuwichain gets it from Zagros over
///   bridge),
/// - bridged foreign asset / double-bridged asset (other bridge / Snowfork): wETH (bridged from
///   Ethereum to Zagros over Snowbridge, then bridged over to Pezkuwichain through this bridge).
fn send_back_wnds_usdt_and_weth_from_asset_hub_pezkuwichain_to_asset_hub_zagros() {
	let prefund_amount = 10_000_000_000_000u128;
	let amount_to_send = ASSET_HUB_ZAGROS_ED * 1_000;
	let sender = AssetHubPezkuwichainSender::get();
	let receiver = AssetHubZagrosReceiver::get();
	let wnd_at_asset_hub_pezkuwichain = bridged_wnd_at_ah_pezkuwichain();
	let prefund_accounts = vec![(sender.clone(), prefund_amount)];
	let reserves = vec![(asset_hub_zagros_location(), false).into()];
	create_foreign_on_ah_pezkuwichain(
		wnd_at_asset_hub_pezkuwichain.clone(),
		true,
		reserves,
		prefund_accounts,
	);

	////////////////////////////////////////////////////////////
	// Let's first send back just some ZGRs as a simple example
	////////////////////////////////////////////////////////////

	// fund the AHR's SA on AHW with the ZGR tokens held in reserve
	let sov_ahr_on_ahw = AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
		ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
		AssetHubPezkuwichain::para_id(),
	);
	AssetHubZagros::fund_accounts(vec![(sov_ahr_on_ahw.clone(), prefund_amount)]);

	let wnds_in_reserve_on_ahw_before =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	assert_eq!(wnds_in_reserve_on_ahw_before, prefund_amount);

	let sender_wnds_before =
		foreign_balance_on_ah_pezkuwichain(wnd_at_asset_hub_pezkuwichain.clone(), &sender);
	assert_eq!(sender_wnds_before, prefund_amount);
	let receiver_wnds_before = <AssetHubZagros as Chain>::account_data_of(receiver.clone()).free;

	// send back WNDs, use them for fees
	send_assets_over_bridge(|| {
		let destination = asset_hub_zagros_location();
		let assets: Assets = (wnd_at_asset_hub_pezkuwichain.clone(), amount_to_send).into();
		let fee_idx = 0;
		let transfer_type = TransferType::DestinationReserve;
		assert_ok!(send_assets_from_asset_hub_pezkuwichain(
			destination,
			assets,
			fee_idx,
			transfer_type
		));
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				// ZGR is withdrawn from AHR's SA on AHW
				RuntimeEvent::Balances(
					pezpallet_balances::Event::Burned { who, amount }
				) => {
					who: *who == sov_ahr_on_ahw,
					amount: *amount == amount_to_send,
				},
				// ZGRs deposited to beneficiary
				RuntimeEvent::Balances(pezpallet_balances::Event::Minted { who, .. }) => {
					who: who == &receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_wnds_after =
		foreign_balance_on_ah_pezkuwichain(wnd_at_asset_hub_pezkuwichain, &sender);
	let receiver_wnds_after = <AssetHubZagros as Chain>::account_data_of(receiver.clone()).free;
	let wnds_in_reserve_on_ahw_after =
		<AssetHubZagros as Chain>::account_data_of(sov_ahr_on_ahw).free;

	// Sender's balance is reduced
	assert!(sender_wnds_before > sender_wnds_after);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(wnds_in_reserve_on_ahw_after, wnds_in_reserve_on_ahw_before - amount_to_send);

	//////////////////////////////////////////////////////////////////
	// Now let's send back over USDTs + wETH (and pay fees with USDT)
	//////////////////////////////////////////////////////////////////

	// wETH has same relative location on both Zagros and Pezkuwichain AssetHubs
	let bridged_weth_at_ah = weth_at_asset_hubs();
	let bridged_usdt_at_asset_hub_pezkuwichain = bridged_usdt_at_ah_pezkuwichain();

	// set up destination chain AH Zagros:
	// create a ZGR/USDT pool to be able to pay fees with USDT (USDT created in genesis)
	set_up_pool_with_wnd_on_ah_zagros(usdt_at_ah_zagros(), false);
	// prefund AHR's sovereign account on AHW to be able to withdraw USDT and wETH from reserves
	let sov_ahr_on_ahw = AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
		ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
		AssetHubPezkuwichain::para_id(),
	);

	let sov_ahw_on_ahr =
		AssetHubPezkuwichain::sovereign_account_of_teyrchain_on_other_global_consensus(
			ByGenesis(ZAGROS_GENESIS_HASH),
			AssetHubZagros::para_id(),
		);

	AssetHubZagros::mint_asset(
		<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosAssetOwner::get()),
		USDT_ID,
		sov_ahr_on_ahw.clone(),
		amount_to_send * 2,
	);
	AssetHubZagros::mint_foreign_asset(
		<AssetHubZagros as Chain>::RuntimeOrigin::signed(snowbridge_sovereign()),
		bridged_weth_at_ah.clone(),
		sov_ahr_on_ahw.clone(),
		amount_to_send * 2,
	);
	AssetHubPezkuwichain::mint_foreign_asset(
		<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(sov_ahw_on_ahr.clone()),
		bridged_weth_at_ah.clone(),
		sov_ahr_on_ahw,
		prefund_amount,
	);
	AssetHubPezkuwichain::mint_foreign_asset(
		<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(sov_ahw_on_ahr),
		bridged_weth_at_ah.clone(),
		sender.clone(),
		prefund_amount,
	);

	// set up source chain AH Pezkuwichain:
	// create wETH and USDT foreign assets on Pezkuwichain and prefund sender's account
	let prefund_accounts = vec![(sender.clone(), amount_to_send * 2)];
	let reserves = vec![(asset_hub_zagros_location(), false).into()];
	create_foreign_on_ah_pezkuwichain(
		bridged_usdt_at_asset_hub_pezkuwichain.clone(),
		true,
		reserves,
		prefund_accounts,
	);

	// check balances before
	let receiver_usdts_before = AssetHubZagros::execute_with(|| {
		type Assets = <AssetHubZagros as AssetHubZagrosPallet>::Assets;
		<Assets as Inspect<_>>::balance(USDT_ID, &receiver)
	});
	let receiver_weth_before = foreign_balance_on_ah_zagros(bridged_weth_at_ah.clone(), &receiver);

	let usdt_id: AssetId =
		Location::try_from(bridged_usdt_at_asset_hub_pezkuwichain).unwrap().into();
	// send USDTs and wETHs
	let assets: Assets = vec![
		(usdt_id.clone(), amount_to_send).into(),
		(Location::try_from(bridged_weth_at_ah.clone()).unwrap(), amount_to_send).into(),
	]
	.into();
	// use USDT for fees
	let fee = usdt_id;

	// use the more involved transfer extrinsic
	let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
		assets: Wild(AllCounted(assets.len() as u32)),
		beneficiary: AccountId32Junction { network: None, id: receiver.clone().into() }.into(),
	}]);
	assert_ok!(AssetHubPezkuwichain::execute_with(|| {
		<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(sender.into()),
			bx!(asset_hub_zagros_location().into()),
			bx!(assets.into()),
			bx!(TransferType::DestinationReserve),
			bx!(fee.into()),
			bx!(TransferType::DestinationReserve),
			bx!(VersionedXcm::from(custom_xcm_on_dest)),
			WeightLimit::Unlimited,
		)
	}));
	// verify hops (also advances the message through the hops)
	assert_bridge_hub_pezkuwichain_message_accepted(true);
	assert_bridge_hub_zagros_message_received();
	AssetHubZagros::execute_with(|| {
		AssetHubZagros::assert_xcmp_queue_success(None);
	});

	let receiver_usdts_after = AssetHubZagros::execute_with(|| {
		type Assets = <AssetHubZagros as AssetHubZagrosPallet>::Assets;
		<Assets as Inspect<_>>::balance(USDT_ID, &receiver)
	});
	let receiver_weth_after = foreign_balance_on_ah_zagros(bridged_weth_at_ah, &receiver);

	// Receiver's USDT balance is increased by almost `amount_to_send` (minus fees)
	assert!(receiver_usdts_after > receiver_usdts_before);
	assert!(receiver_usdts_after < receiver_usdts_before + amount_to_send);
	// Receiver's wETH balance is increased by `amount_to_send`
	assert_eq!(receiver_weth_after, receiver_weth_before + amount_to_send);
}

#[test]
fn send_rocs_from_penpal_pezkuwichain_through_asset_hub_pezkuwichain_to_asset_hub_zagros() {
	let amount = ASSET_HUB_PEZKUWICHAIN_ED * 10_000_000;
	let sender = PenpalASender::get();
	let receiver = AssetHubZagrosReceiver::get();
	let local_asset_hub = PenpalA::sibling_location_of(AssetHubPezkuwichain::para_id());
	let (roc_at_pezkuwichain_teyrchains, roc_at_asset_hub_zagros) =
		set_up_rocs_for_penpal_pezkuwichain_through_ahr_to_ahw(&sender, amount);
	set_up_pool_with_wnd_on_ah_zagros(roc_at_asset_hub_zagros.clone(), true);

	let sov_ahw_on_ahr =
		AssetHubPezkuwichain::sovereign_account_of_teyrchain_on_other_global_consensus(
			ByGenesis(ZAGROS_GENESIS_HASH),
			AssetHubZagros::para_id(),
		);
	let rocs_in_reserve_on_ahr_before =
		<AssetHubPezkuwichain as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let sender_rocs_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_at_pezkuwichain_teyrchains.clone(), &sender)
	});
	let receiver_rocs_before =
		foreign_balance_on_ah_zagros(roc_at_asset_hub_zagros.clone(), &receiver);

	// Send TYRs over bridge
	{
		let destination = asset_hub_zagros_location();
		let assets: Assets = (roc_at_pezkuwichain_teyrchains.clone(), amount).into();
		let asset_transfer_type = TransferType::RemoteReserve(local_asset_hub.clone().into());
		let fees_id: AssetId = roc_at_pezkuwichain_teyrchains.clone().into();
		let fees_transfer_type = TransferType::RemoteReserve(local_asset_hub.into());
		let beneficiary: Location =
			AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		let custom_xcm_on_dest = Xcm::<()>(vec![DepositAsset {
			assets: Wild(AllCounted(assets.len() as u32)),
			beneficiary,
		}]);
		send_assets_from_penpal_pezkuwichain_through_pezkuwichain_ah_to_zagros_ah(
			destination,
			(assets, asset_transfer_type),
			(fees_id, fees_transfer_type),
			custom_xcm_on_dest,
		);
	}

	// process AHW incoming message and check events
	let roc = Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH))]);
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				// issue TYRs on AHW
				RuntimeEvent::ForeignAssets(pezpallet_assets::Event::Issued { asset_id, owner, .. }) => {
					asset_id: *asset_id == roc,
					owner: owner == &receiver,
				},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_rocs_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(roc_at_pezkuwichain_teyrchains, &sender)
	});
	let receiver_rocs_after = foreign_balance_on_ah_zagros(roc_at_asset_hub_zagros, &receiver);
	let rocs_in_reserve_on_ahr_after =
		<AssetHubPezkuwichain as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;

	// Sender's balance is reduced
	assert!(sender_rocs_after < sender_rocs_before);
	// Receiver's balance is increased
	assert!(receiver_rocs_after > receiver_rocs_before);
	// Reserve balance is increased by sent amount (less fess)
	assert!(rocs_in_reserve_on_ahr_after > rocs_in_reserve_on_ahr_before);
	assert!(rocs_in_reserve_on_ahr_after <= rocs_in_reserve_on_ahr_before + amount);
}

#[test]
fn send_back_wnds_from_penpal_pezkuwichain_through_asset_hub_pezkuwichain_to_asset_hub_zagros() {
	let wnd_at_pezkuwichain_teyrchains = bridged_wnd_at_ah_pezkuwichain();
	let amount = ASSET_HUB_PEZKUWICHAIN_ED * 10_000_000;
	let sender = PenpalASender::get();
	let receiver = AssetHubZagrosReceiver::get();

	// set up TYRs for transfer
	let (roc_at_pezkuwichain_teyrchains, _) =
		set_up_rocs_for_penpal_pezkuwichain_through_ahr_to_ahw(&sender, amount);

	// set up ZGRs for transfer
	let penpal_location = AssetHubPezkuwichain::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahr = AssetHubPezkuwichain::sovereign_account_id_of(penpal_location);
	let prefund_accounts = vec![(sov_penpal_on_ahr, amount * 2)];
	let reserves = vec![(asset_hub_zagros_location(), false).into()];
	create_foreign_on_ah_pezkuwichain(
		wnd_at_pezkuwichain_teyrchains.clone(),
		true,
		reserves,
		prefund_accounts,
	);
	let asset_owner: AccountId = AssetHubPezkuwichain::account_id_of(ALICE);
	PenpalA::force_create_foreign_asset(
		wnd_at_pezkuwichain_teyrchains.clone(),
		asset_owner.clone(),
		true,
		ASSET_MIN_BALANCE,
		vec![(sender.clone(), amount * 2)],
	);
	// Configure source Penpal chain to trust local AH as reserve of bridged ZGR
	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as Chain>::System::set_storage(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				wnd_at_pezkuwichain_teyrchains.encode(),
			)],
		));
	});

	// fund the AHR's SA on AHW with the ZGR tokens held in reserve
	let sov_ahr_on_ahw = AssetHubZagros::sovereign_account_of_teyrchain_on_other_global_consensus(
		NetworkId::ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
		AssetHubPezkuwichain::para_id(),
	);
	AssetHubZagros::fund_accounts(vec![(sov_ahr_on_ahw.clone(), amount * 2)]);

	// balances before
	let sender_wnds_before = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(
			wnd_at_pezkuwichain_teyrchains.clone().into(),
			&sender,
		)
	});
	let receiver_wnds_before = <AssetHubZagros as Chain>::account_data_of(receiver.clone()).free;

	// send ZGRs over the bridge, TYRs only used to pay fees on local AH, pay with ZGR on remote AH
	{
		let final_destination = asset_hub_zagros_location();
		let intermediary_hop = PenpalA::sibling_location_of(AssetHubPezkuwichain::para_id());
		let context = PenpalA::execute_with(|| PenpalUniversalLocation::get());

		// what happens at final destination
		let beneficiary = AccountId32Junction { network: None, id: receiver.clone().into() }.into();
		// use ZGR as fees on the final destination (AHW)
		let remote_fees: Asset = (wnd_at_pezkuwichain_teyrchains.clone(), amount).into();
		let remote_fees = remote_fees.reanchored(&final_destination, &context).unwrap();
		// buy execution using WNDs, then deposit all remaining WNDs
		let xcm_on_final_dest = Xcm::<()>(vec![
			BuyExecution { fees: remote_fees, weight_limit: WeightLimit::Unlimited },
			DepositAsset { assets: Wild(AllCounted(1)), beneficiary },
		]);

		// what happens at intermediary hop
		// reanchor final dest (Asset Hub Zagros) to the view of hop (Asset Hub Pezkuwichain)
		let mut final_destination = final_destination.clone();
		final_destination.reanchor(&intermediary_hop, &context).unwrap();
		// reanchor ZGRs to the view of hop (Asset Hub Pezkuwichain)
		let asset: Asset = (wnd_at_pezkuwichain_teyrchains.clone(), amount).into();
		let asset = asset.reanchored(&intermediary_hop, &context).unwrap();
		// on Asset Hub Pezkuwichain, forward a request to withdraw ZGRs from reserve on Asset Hub
		// Zagros
		let xcm_on_hop = Xcm::<()>(vec![InitiateReserveWithdraw {
			assets: Definite(asset.into()), // WNDs
			reserve: final_destination,     // AHW
			xcm: xcm_on_final_dest,         // XCM to execute on AHW
		}]);
		// assets to send from Penpal and how they reach the intermediary hop
		let assets: Assets = vec![
			(wnd_at_pezkuwichain_teyrchains.clone(), amount).into(),
			(roc_at_pezkuwichain_teyrchains.clone(), amount).into(),
		]
		.into();
		let asset_transfer_type = TransferType::DestinationReserve;
		let fees_id: AssetId = roc_at_pezkuwichain_teyrchains.into();
		let fees_transfer_type = TransferType::DestinationReserve;

		// initiate the transfer
		send_assets_from_penpal_pezkuwichain_through_pezkuwichain_ah_to_zagros_ah(
			intermediary_hop,
			(assets, asset_transfer_type),
			(fees_id, fees_transfer_type),
			xcm_on_hop,
		);
	}

	// process AHW incoming message and check events
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				// issue TYRs on AHW
				RuntimeEvent::Balances(pezpallet_balances::Event::Issued { .. }) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let sender_wnds_after = PenpalA::execute_with(|| {
		type ForeignAssets = <PenpalA as PenpalAPallet>::ForeignAssets;
		<ForeignAssets as Inspect<_>>::balance(wnd_at_pezkuwichain_teyrchains.into(), &sender)
	});
	let receiver_wnds_after = <AssetHubZagros as Chain>::account_data_of(receiver).free;

	// Sender's balance is reduced by sent "amount"
	assert_eq!(sender_wnds_after, sender_wnds_before - amount);
	// Receiver's balance is increased by no more than "amount"
	assert!(receiver_wnds_after > receiver_wnds_before);
	assert!(receiver_wnds_after <= receiver_wnds_before + amount);
}

#[test]
fn dry_run_transfer_to_zagros_sends_xcm_to_bridge_hub() {
	test_dry_run_transfer_across_pk_bridge!(
		AssetHubPezkuwichain,
		BridgeHubPezkuwichain,
		asset_hub_zagros_location()
	);
}
