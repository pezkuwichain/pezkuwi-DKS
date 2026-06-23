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

use crate::{imports::*, tests::bridged_roc_at_ah_zagros};
use asset_hub_zagros_runtime::xcm_config::LocationToAccountId;
use emulated_integration_tests_common::{
	snowbridge::{SEPOLIA_ID, WETH},
	PenpalBTeleportableAssetLocation,
};
use hex_literal::hex;
use pezframe_support::traits::fungibles::Mutate;
use pezkuwichain_zagros_system_emulated_network::pez_penpal_emulated_chain::{
	pez_penpal_runtime::xcm_config::{CheckingAccount, TELEPORTABLE_ASSET_ID},
	PenpalAssetOwner,
};
use pezsnowbridge_core::AssetMetadata;
use pezsp_core::H160;
use testnet_teyrchains_constants::zagros::snowbridge::EthereumNetwork;
use xcm_builder::ExternalConsensusLocationsConverterFor;
use xcm_executor::traits::ConvertLocation;

pub const INITIAL_FUND: u128 = 50_000_000_000_000;
pub const ETHEREUM_DESTINATION_ADDRESS: [u8; 20] = hex!("44a57ee2f2FCcb85FDa2B0B18EBD0D8D2333700e");
pub const AGENT_ADDRESS: [u8; 20] = hex!("90A987B944Cb1dCcE5564e5FDeCD7a54D3de27Fe");
pub const TOKEN_AMOUNT: u128 = 10_000_000_000_000;
pub const REMOTE_FEE_AMOUNT_IN_ETHER: u128 = 600_000_000_000;
pub const LOCAL_FEE_AMOUNT_IN_DOT: u128 = 800_000_000_000;

pub const EXECUTION_WEIGHT: u64 = 8_000_000_000;

pub fn beneficiary() -> Location {
	Location::new(0, [AccountKey20 { network: None, key: ETHEREUM_DESTINATION_ADDRESS.into() }])
}

pub fn asset_hub() -> Location {
	Location::new(1, Teyrchain(AssetHubZagros::para_id().into()))
}

pub fn bridge_hub() -> Location {
	Location::new(1, Teyrchain(BridgeHubZagros::para_id().into()))
}

pub fn fund_on_bh() {
	let assethub_sovereign = BridgeHubZagros::sovereign_account_id_of(asset_hub());
	BridgeHubZagros::fund_accounts(vec![(assethub_sovereign.clone(), INITIAL_FUND)]);
}

pub fn register_assets_on_ah() {}
pub fn register_relay_token_on_bh() {
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		type RuntimeOrigin = <BridgeHubZagros as Chain>::RuntimeOrigin;

		// Register ZGR on BH
		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(Location::parent())),
			AssetMetadata {
				name: "wnd".as_bytes().to_vec().try_into().unwrap(),
				symbol: "wnd".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumSystem(pezsnowbridge_pezpallet_system::Event::RegisterToken { .. }) => {},]
		);
	});
}

pub fn register_assets_on_penpal() {
	let ethereum_sovereign: AccountId = snowbridge_sovereign();
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::force_create(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			weth_location().try_into().unwrap(),
			ethereum_sovereign.clone().into(),
			true,
			1,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::force_create(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			ethereum().try_into().unwrap(),
			ethereum_sovereign.into(),
			true,
			1,
		));
	});
}

pub fn register_foreign_asset(token_location: Location) {
	let bridge_owner = snowbridge_sovereign();
	AssetHubZagros::execute_with(|| {
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::force_create(
			RuntimeOrigin::root(),
			token_location.clone().try_into().unwrap(),
			bridge_owner.clone().into(),
			true,
			1000,
		));
		assert!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::asset_exists(
			token_location.clone().try_into().unwrap(),
		));
	});
	AssetHubZagros::set_foreign_asset_reserves(
		token_location,
		bridge_owner,
		vec![(ethereum(), false).into()],
	);
}

pub fn register_pal_on_ah() {
	// Create PAL(i.e. native asset for penpal) on AH.
	AssetHubZagros::execute_with(|| {
		type RuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;
		let penpal_asset_id = Location::new(1, Teyrchain(PenpalB::para_id().into()));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::force_create(
			RuntimeOrigin::root(),
			penpal_asset_id.clone(),
			PenpalAssetOwner::get().into(),
			false,
			1_000_000,
		));

		assert!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::asset_exists(
			penpal_asset_id.clone(),
		));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			penpal_asset_id.clone(),
			&AssetHubZagrosReceiver::get(),
			TOKEN_AMOUNT,
		));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			penpal_asset_id.clone(),
			&AssetHubZagrosSender::get(),
			TOKEN_AMOUNT,
		));
	});
}

pub fn penpal_root_sovereign() -> pezsp_runtime::AccountId32 {
	let penpal_root_sovereign: AccountId = PenpalB::execute_with(|| {
		use pezkuwichain_zagros_system_emulated_network::pez_penpal_emulated_chain::pez_penpal_runtime::xcm_config;
		xcm_config::LocationToAccountId::convert_location(&xcm_config::RootLocation::get())
			.unwrap()
			.into()
	});
	penpal_root_sovereign
}

pub fn fund_on_penpal() {
	let sudo_account = penpal_root_sovereign();
	PenpalB::fund_accounts(vec![
		(PenpalBReceiver::get(), INITIAL_FUND),
		(PenpalBSender::get(), INITIAL_FUND),
		(CheckingAccount::get(), INITIAL_FUND),
		(sudo_account.clone(), INITIAL_FUND),
	]);
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			Location::parent(),
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			Location::parent(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			Location::parent(),
			&sudo_account,
			INITIAL_FUND,
		));
	});
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::Assets::mint_into(
			TELEPORTABLE_ASSET_ID,
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::Assets::mint_into(
			TELEPORTABLE_ASSET_ID,
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::Assets::mint_into(
			TELEPORTABLE_ASSET_ID,
			&sudo_account,
			INITIAL_FUND,
		));
	});
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&sudo_account,
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			ethereum().try_into().unwrap(),
			&PenpalBReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			ethereum().try_into().unwrap(),
			&PenpalBSender::get(),
			INITIAL_FUND,
		));
		assert_ok!(<PenpalB as PenpalBPallet>::ForeignAssets::mint_into(
			ethereum().try_into().unwrap(),
			&sudo_account,
			INITIAL_FUND,
		));
	});
}

pub fn set_trust_reserve_on_penpal() {
	PenpalB::execute_with(|| {
		assert_ok!(<PenpalB as Chain>::System::set_storage(
			<PenpalB as Chain>::RuntimeOrigin::root(),
			vec![(
				PenpalCustomizableAssetFromSystemAssetHub::key().to_vec(),
				Location::new(2, [GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })]).encode(),
			)],
		));
	});
}

pub fn fund_on_ah() {
	AssetHubZagros::fund_accounts(vec![(AssetHubZagrosSender::get(), INITIAL_FUND)]);
	AssetHubZagros::fund_accounts(vec![(AssetHubZagrosReceiver::get(), INITIAL_FUND)]);

	let penpal_sovereign = AssetHubZagros::sovereign_account_id_of(
		AssetHubZagros::sibling_location_of(PenpalB::para_id()),
	);
	let penpal_user_sovereign = LocationToAccountId::convert_location(&Location::new(
		1,
		[
			Teyrchain(PenpalB::para_id().into()),
			AccountId32 {
				network: Some(ByGenesis(ZAGROS_GENESIS_HASH)),
				id: PenpalBSender::get().into(),
			},
		],
	))
	.unwrap();

	AssetHubZagros::execute_with(|| {
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&penpal_sovereign,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&penpal_user_sovereign,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&AssetHubZagrosReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			weth_location().try_into().unwrap(),
			&AssetHubZagrosSender::get(),
			INITIAL_FUND,
		));

		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			ethereum().try_into().unwrap(),
			&penpal_sovereign,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			ethereum().try_into().unwrap(),
			&penpal_user_sovereign,
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			ethereum().try_into().unwrap(),
			&AssetHubZagrosReceiver::get(),
			INITIAL_FUND,
		));
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint_into(
			ethereum().try_into().unwrap(),
			&AssetHubZagrosSender::get(),
			INITIAL_FUND,
		));
	});

	AssetHubZagros::fund_accounts(vec![(snowbridge_sovereign(), INITIAL_FUND)]);
	AssetHubZagros::fund_accounts(vec![(penpal_sovereign.clone(), INITIAL_FUND)]);
	AssetHubZagros::fund_accounts(vec![(penpal_user_sovereign.clone(), INITIAL_FUND)]);
}

pub fn create_pools_on_ah() {
	// We create a pool between ZGR and WETH in AssetHub to support paying for fees with WETH.
	let ethereum_sovereign = snowbridge_sovereign();
	AssetHubZagros::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	PenpalB::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	create_pool_with_native_on!(
		AssetHubZagros,
		weth_location(),
		true,
		ethereum_sovereign.clone(),
		1_000_000_000_000,
		20_000_000_000
	);
	create_pool_with_native_on!(
		AssetHubZagros,
		ethereum(),
		true,
		ethereum_sovereign.clone(),
		1_000_000_000_000,
		20_000_000_000
	);
}

pub(crate) fn set_up_eth_and_hez_pool() {
	// We create a pool between ZGR and WETH in AssetHub to support paying for fees with WETH.
	let ethereum_sovereign = snowbridge_sovereign();
	AssetHubZagros::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	PenpalB::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	create_pool_with_native_on!(AssetHubZagros, eth_location(), true, ethereum_sovereign.clone());
}

pub(crate) fn set_up_eth_and_hez_pool_on_penpal() {
	let ethereum_sovereign = snowbridge_sovereign();
	AssetHubZagros::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	PenpalB::fund_accounts(vec![(ethereum_sovereign.clone(), INITIAL_FUND)]);
	create_pool_with_native_on!(PenpalB, eth_location(), true, ethereum_sovereign.clone());
}

pub(crate) fn set_up_eth_and_hez_pool_on_pezkuwichain() {
	let sa_of_wah_on_rah =
		AssetHubPezkuwichain::sovereign_account_of_teyrchain_on_other_global_consensus(
			ByGenesis(ZAGROS_GENESIS_HASH),
			AssetHubZagros::para_id(),
		);
	AssetHubPezkuwichain::fund_accounts(vec![(sa_of_wah_on_rah.clone(), INITIAL_FUND)]);
	create_pool_with_native_on!(
		AssetHubPezkuwichain,
		eth_location(),
		true,
		sa_of_wah_on_rah.clone()
	);
}

pub fn register_pal_on_bh() {
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		type RuntimeOrigin = <BridgeHubZagros as Chain>::RuntimeOrigin;

		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(PenpalBTeleportableAssetLocation::get())),
			AssetMetadata {
				name: "pal".as_bytes().to_vec().try_into().unwrap(),
				symbol: "pal".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumSystem(pezsnowbridge_pezpallet_system::Event::RegisterToken { .. }) => {},]
		);
	});
}

pub fn snowbridge_sovereign() -> pezsp_runtime::AccountId32 {
	use asset_hub_zagros_runtime::xcm_config::UniversalLocation as AssetHubZagrosUniversalLocation;
	let ethereum_sovereign: AccountId = AssetHubZagros::execute_with(|| {
		ExternalConsensusLocationsConverterFor::<
			AssetHubZagrosUniversalLocation,
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

pub fn weth_location() -> Location {
	erc20_token_location(WETH.into())
}

pub fn eth_location() -> Location {
	Location::new(2, [GlobalConsensus(Ethereum { chain_id: SEPOLIA_ID })])
}

pub fn ethereum() -> Location {
	eth_location()
}

pub fn erc20_token_location(token_id: H160) -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(EthereumNetwork::get().into()),
			AccountKey20 { network: None, key: token_id.into() },
		],
	)
}

// HEZ and wTYR
pub(crate) fn roc_at_ah_pezkuwichain() -> Location {
	Parent.into()
}

// set up pool
pub(crate) fn set_up_pool_with_wnd_on_ah_zagros(
	asset: Location,
	is_foreign: bool,
	initial_fund: u128,
	initial_liquidity: u128,
) {
	let wnd: Location = Parent.into();
	AssetHubZagros::fund_accounts(vec![(AssetHubZagrosSender::get(), initial_fund)]);
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		let owner = AssetHubZagrosSender::get();
		let signed_owner = <AssetHubZagros as Chain>::RuntimeOrigin::signed(owner.clone());

		if is_foreign {
			assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::ForeignAssets::mint(
				signed_owner.clone(),
				asset.clone().into(),
				owner.clone().into(),
				initial_fund,
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
				initial_fund,
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
			initial_liquidity,
			initial_liquidity,
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

pub fn register_roc_on_bh() {
	BridgeHubZagros::execute_with(|| {
		type RuntimeEvent = <BridgeHubZagros as Chain>::RuntimeEvent;
		type RuntimeOrigin = <BridgeHubZagros as Chain>::RuntimeOrigin;

		// Register HEZ on BH
		assert_ok!(<BridgeHubZagros as BridgeHubZagrosPallet>::EthereumSystem::register_token(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::from(bridged_roc_at_ah_zagros())),
			AssetMetadata {
				name: "roc".as_bytes().to_vec().try_into().unwrap(),
				symbol: "roc".as_bytes().to_vec().try_into().unwrap(),
				decimals: 12,
			},
		));
		assert_expected_events!(
			BridgeHubZagros,
			vec![RuntimeEvent::EthereumSystem(pezsnowbridge_pezpallet_system::Event::RegisterToken { .. }) => {},]
		);
	});
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
pub(crate) fn bridge_hub_zagros_location() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
			Teyrchain(BridgeHubZagros::para_id().into()),
		],
	)
}
