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
use emulated_integration_tests_common::{snowbridge, snowbridge::WETH};
use testnet_teyrchains_constants::pezkuwichain::snowbridge::EthereumNetwork;
use xcm::opaque::v5;
use xcm_executor::traits::ConvertLocation;

mod asset_transfers;
mod claim_assets;
mod register_bridged_assets;
mod send_xcm;
mod teleport;

pub(crate) fn asset_hub_zagros_location() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
			Teyrchain(AssetHubZagros::para_id().into()),
		],
	)
}
pub(crate) fn asset_hub_pezkuwichain_global_location() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
			Teyrchain(AssetHubPezkuwichain::para_id().into()),
		],
	)
}
pub(crate) fn bridge_hub_zagros_location() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
			Teyrchain(BridgeHubZagros::para_id().into()),
		],
	)
}

// TYR and wTYR
pub(crate) fn roc_at_ah_pezkuwichain() -> Location {
	Parent.into()
}
pub(crate) fn bridged_roc_at_ah_zagros() -> Location {
	Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH))])
}

// ZGR and wWND
pub(crate) fn bridged_wnd_at_ah_pezkuwichain() -> Location {
	Location::new(2, [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH))])
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
			GlobalConsensus(Ethereum { chain_id: snowbridge::SEPOLIA_ID }),
			AccountKey20 { network: None, key: WETH },
		],
	)
}

pub(crate) fn create_foreign_on_ah_pezkuwichain(
	id: v5::Location,
	sufficient: bool,
	reserves: Vec<ForeignAssetReserveData>,
	prefund_accounts: Vec<(AccountId, u128)>,
) {
	let owner = AssetHubPezkuwichain::account_id_of(ALICE);
	let min = ASSET_MIN_BALANCE;
	AssetHubPezkuwichain::force_create_foreign_asset(
		id.clone(),
		owner.clone(),
		sufficient,
		min,
		prefund_accounts,
	);
	AssetHubPezkuwichain::set_foreign_asset_reserves(id, owner, reserves);
}

pub(crate) fn create_foreign_on_ah_zagros(
	id: v5::Location,
	sufficient: bool,
	reserves: Vec<ForeignAssetReserveData>,
) {
	let owner = AssetHubZagros::account_id_of(ALICE);
	AssetHubZagros::force_create_foreign_asset(
		id.clone(),
		owner.clone(),
		sufficient,
		ASSET_MIN_BALANCE,
		vec![],
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

// set up pool
pub(crate) fn set_up_pool_with_wnd_on_ah_zagros(asset: v5::Location, is_foreign: bool) {
	let wnd: v5::Location = v5::Parent.into();
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		let owner = AssetHubZagrosSender::get();
		let signed_owner = <AssetHubZagros as Chain>::RuntimeOrigin::signed(owner.clone());

		if is_foreign {
			assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint(
				signed_owner.clone(),
				asset.clone().into(),
				owner.clone().into(),
				3_000_000_000_000,
			));
		} else {
			let asset_id = match asset.interior.last() {
				Some(GeneralIndex(id)) => *id as u32,
				_ => unreachable!(),
			};
			assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::Assets::mint(
				signed_owner.clone(),
				asset_id.into(),
				owner.clone().into(),
				3_000_000_000_000,
			));
		}
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::create_pool(
			signed_owner.clone(),
			Box::new(wnd.clone()),
			Box::new(asset.clone()),
		));
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::AssetConversion::add_liquidity(
			signed_owner.clone(),
			Box::new(wnd),
			Box::new(asset),
			1_000_000_000_000,
			2_000_000_000_000,
			1,
			1,
			owner.into()
		));
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::AssetConversion(pezpallet_asset_conversion::Event::LiquidityAdded {..}) => {},
			]
		);
	});
}

pub(crate) fn send_assets_from_asset_hub_pezkuwichain(
	destination: Location,
	assets: Assets,
	fee_idx: u32,
	// For knowing what reserve to pick.
	// We only allow using the same transfer type for assets and fees right now.
	// And only `LocalReserve` or `DestinationReserve`.
	transfer_type: TransferType,
) -> DispatchResult {
	let signed_origin =
		<AssetHubPezkuwichain as Chain>::RuntimeOrigin::signed(AssetHubPezkuwichainSender::get());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: AssetHubZagrosReceiver::get().into() }.into();

	type Runtime = <AssetHubPezkuwichain as Chain>::Runtime;
	let remote_fee_id: AssetId = assets
		.clone()
		.into_inner()
		.get(fee_idx as usize)
		.ok_or(pezpallet_xcm::Error::<Runtime>::Empty)?
		.clone()
		.id;

	AssetHubPezkuwichain::execute_with(|| {
		<AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::PezkuwiXcm::transfer_assets_using_type_and_then(
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

pub(crate) fn assert_bridge_hub_pezkuwichain_message_accepted(expected_processed: bool) {
	BridgeHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <BridgeHubPezkuwichain as Chain>::RuntimeEvent;

		if expected_processed {
			assert_expected_events!(
				BridgeHubPezkuwichain,
				vec![
					// pay for bridge fees
					RuntimeEvent::Balances(pezpallet_balances::Event::Burned { .. }) => {},
					// message exported
					RuntimeEvent::BridgeZagrosMessages(
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
				BridgeHubPezkuwichain,
				vec![
					RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed {
						success: false,
						..
					}) => {},
				]
			);
		}
	});
}

pub(crate) fn assert_bridge_hub_zagros_message_received() {
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubZagros,
			vec![
				// message sent to destination
				RuntimeEvent::XcmpQueue(
					pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	})
}

pub fn snowbridge_sovereign() -> pezsp_runtime::AccountId32 {
	use asset_hub_pezkuwichain_runtime::xcm_config::UniversalLocation as AssetHubPezkuwichainUniversalLocation;
	let ethereum_sovereign: AccountId = AssetHubPezkuwichain::execute_with(|| {
		ExternalConsensusLocationsConverterFor::<
			AssetHubPezkuwichainUniversalLocation,
			[u8; 32],
		>::convert_location(&Location::new(
			2,
			[xcm::v5::Junction::GlobalConsensus(EthereumNetwork::get())],
		))
			.unwrap()
			.into()
	});

	ethereum_sovereign
}
