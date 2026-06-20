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
use emulated_integration_tests_common::snowbridge::{SEPOLIA_ID, WETH};

mod aliases;
mod asset_transfers;
mod claim_assets;
mod register_bridged_assets;
mod send_xcm;
mod snowbridge;
mod snowbridge_common;
// mod snowbridge_v2_inbound;
mod snowbridge_edge_case;
mod snowbridge_v2_inbound;
mod snowbridge_v2_inbound_to_pezkuwichain;
mod snowbridge_v2_outbound;
mod snowbridge_v2_outbound_edge_case;
mod snowbridge_v2_outbound_from_pezkuwichain;
mod snowbridge_v2_rewards;
mod teleport;
mod transact;

pub(crate) fn asset_hub_pezkuwichain_location() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
			Teyrchain(AssetHubPezkuwichain::para_id().into()),
		],
	)
}

pub(crate) fn asset_hub_zagros_global_location() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
			Teyrchain(AssetHubZagros::para_id().into()),
		],
	)
}

pub(crate) fn bridge_hub_pezkuwichain_location() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
			Teyrchain(BridgeHubPezkuwichain::para_id().into()),
		],
	)
}

// ZGR and wWND
pub(crate) fn wnd_at_ah_zagros() -> Location {
	Parent.into()
}
pub(crate) fn bridged_wnd_at_ah_pezkuwichain() -> Location {
	Location::new(2, [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH))])
}

// TYR and wTYR
pub(crate) fn bridged_roc_at_ah_zagros() -> Location {
	Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH))])
}

// USDT and wUSDT
pub(crate) fn usdt_at_ah_zagros() -> Location {
	Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())])
}
pub(crate) fn bridged_usdt_at_ah_pezkuwichain() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
			Teyrchain(AssetHubZagros::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(USDT_ID.into()),
		],
	)
}

// wETH has same relative location on both Pezkuwichain and Zagros AssetHubs
pub(crate) fn weth_at_asset_hubs() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID }),
			AccountKey20 { network: None, key: WETH },
		],
	)
}

pub(crate) fn create_foreign_on_ah_pezkuwichain(
	id: v5::Location,
	sufficient: bool,
	reserves: Vec<ForeignAssetReserveData>,
) {
	let owner = AssetHubPezkuwichain::account_id_of(ALICE);
	AssetHubPezkuwichain::force_create_foreign_asset(
		id.clone(),
		owner.clone(),
		sufficient,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubPezkuwichain::set_foreign_asset_reserves(id, owner, reserves);
}

pub(crate) fn create_foreign_on_ah_zagros(
	id: v5::Location,
	sufficient: bool,
	reserves: Vec<ForeignAssetReserveData>,
	prefund_accounts: Vec<(AccountId, u128)>,
) {
	let owner = AssetHubZagros::account_id_of(ALICE);
	let min = ASSET_MIN_BALANCE;
	AssetHubZagros::force_create_foreign_asset(
		id.clone(),
		owner.clone(),
		sufficient,
		min,
		prefund_accounts,
	);
	AssetHubZagros::set_foreign_asset_reserves(id, owner, reserves);
}

pub(crate) fn foreign_balance_on_ah_pezkuwichain(id: v5::Location, who: &AccountId) -> u128 {
	AssetHubPezkuwichain::execute_with(|| {
		type Assets = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(id, who)
	})
}
pub(crate) fn foreign_balance_on_ah_zagros(id: v5::Location, who: &AccountId) -> u128 {
	AssetHubZagros::execute_with(|| {
		type Assets = <AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(id, who)
	})
}

pub(crate) fn send_assets_from_asset_hub_zagros(
	destination: Location,
	assets: Assets,
	fee_idx: u32,
	// For knowing what reserve to pick.
	// We only allow using the same transfer type for assets and fees right now.
	// And only `LocalReserve` or `DestinationReserve`.
	transfer_type: TransferType,
) -> DispatchResult {
	let signed_origin =
		<AssetHubZagros as Chain>::RuntimeOrigin::signed(AssetHubZagrosSender::get().into());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: AssetHubPezkuwichainReceiver::get().into() }
			.into();

	type Runtime = <AssetHubZagros as Chain>::Runtime;
	let remote_fee_id: AssetId = assets
		.clone()
		.into_inner()
		.get(fee_idx as usize)
		.ok_or(pezpallet_xcm::Error::<Runtime>::Empty)?
		.clone()
		.id;

	AssetHubZagros::execute_with(|| {
		<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
			signed_origin,
			bx!(destination.into()),
			bx!(assets.into()),
			bx!(transfer_type.clone()),
			bx!(remote_fee_id.into()),
			bx!(transfer_type),
			bx!(VersionedXcm::from(
				Xcm::<()>::builder_unsafe().deposit_asset(AllCounted(1), beneficiary).build()
			)),
			WeightLimit::Unlimited,
		)
	})
}

pub(crate) fn assert_bridge_hub_zagros_message_accepted(expected_processed: bool) {
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;

		if expected_processed {
			assert_expected_events!(
				BridgeHubZagros,
				vec![
					// pay for bridge fees
					RuntimeEvent::Balances(pezpallet_balances::Event::Burned { .. }) => {},
					// message exported
					RuntimeEvent::BridgePezkuwichainMessages(
						pezpallet_bridge_messages::Event::MessageAccepted { .. }
					) => {},
					// message processed successfully
					RuntimeEvent::MessageQueue(
						pezpallet_message_queue::Event::Processed { success: true, .. }
					) => {},
				]
			);
		} else {
			assert_expected_events!(
				BridgeHubZagros,
				vec![
					RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed {
						success: false,
						..
					}) => {},
				]
			);
		}
	})
}

pub(crate) fn assert_bridge_hub_pezkuwichain_message_received() {
	BridgeHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <BridgeHubPezkuwichain as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubPezkuwichain,
			vec![
				// message sent to destination
				RuntimeEvent::XcmpQueue(
					pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	})
}
