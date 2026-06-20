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

// Bizinikiwi
use pezframe_support::parameter_types;
use pezsp_core::storage::Storage;
use pezsp_keyring::Sr25519Keyring as Keyring;

// Pezcumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, collators,
	snowbridge::{ETHER_MIN_BALANCE, WETH},
	xcm_pez_emulator::ConvertLocation,
	PenpalALocation, PenpalASiblingSovereignAccount, PenpalATeleportableAssetLocation,
	PenpalBLocation, PenpalBSiblingSovereignAccount, PenpalBTeleportableAssetLocation,
	RESERVABLE_ASSET_ID, SAFE_XCM_VERSION, USDT_ID,
};
use testnet_teyrchains_constants::zagros::snowbridge::EthereumNetwork;
use teyrchains_common::{AccountId, Balance};
use xcm::{latest::prelude::*, opaque::latest::ZAGROS_GENESIS_HASH};
use xcm_builder::ExternalConsensusLocationsConverterFor;

pub const PARA_ID: u32 = 1000;
pub const ED: Balance = testnet_teyrchains_constants::zagros::currency::EXISTENTIAL_DEPOSIT;
pub const USDT_ED: Balance = 70_000;

parameter_types! {
	pub AssetHubZagrosAssetOwner: AccountId = Keyring::Alice.to_account_id();
	pub ZagrosGlobalConsensusNetwork: NetworkId = NetworkId::ByGenesis(ZAGROS_GENESIS_HASH);
	pub AssetHubZagrosUniversalLocation: InteriorLocation = [GlobalConsensus(ZagrosGlobalConsensusNetwork::get()), Teyrchain(PARA_ID)].into();
	pub EthereumLocation: Location = Location::new(2, [GlobalConsensus(EthereumNetwork::get())]);
	pub EthereumSovereignAccount: AccountId = ExternalConsensusLocationsConverterFor::<
			AssetHubZagrosUniversalLocation,
			AccountId,
		>::convert_location(&EthereumLocation::get()).unwrap();
}

pub fn genesis() -> Storage {
	let genesis_config = asset_hub_zagros_runtime::RuntimeGenesisConfig {
		system: asset_hub_zagros_runtime::SystemConfig::default(),
		balances: asset_hub_zagros_runtime::BalancesConfig {
			balances: accounts::init_balances()
				.iter()
				.cloned()
				.map(|k| (k, ED * 4096))
				// pre-fund checking account to avoid pre-funding for every test scenario
				// teleporting funds to asset hub
				.chain(std::iter::once((
					asset_hub_zagros_runtime::xcm_config::CheckingAccount::get(),
					ED * 1000,
				)))
				.collect(),
			..Default::default()
		},
		teyrchain_info: asset_hub_zagros_runtime::TeyrchainInfoConfig {
			teyrchain_id: PARA_ID.into(),
			..Default::default()
		},
		collator_selection: asset_hub_zagros_runtime::CollatorSelectionConfig {
			invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: asset_hub_zagros_runtime::SessionConfig {
			keys: collators::invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                                    // account id
						acc,                                            // validator id
						asset_hub_zagros_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
			..Default::default()
		},
		pezkuwi_xcm: asset_hub_zagros_runtime::PezkuwiXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		assets: asset_hub_zagros_runtime::AssetsConfig {
			assets: vec![
				(RESERVABLE_ASSET_ID, AssetHubZagrosAssetOwner::get(), false, ED),
				(USDT_ID, AssetHubZagrosAssetOwner::get(), true, USDT_ED),
			],
			..Default::default()
		},
		foreign_assets: asset_hub_zagros_runtime::ForeignAssetsConfig {
			assets: vec![
				// PenpalA's teleportable asset representation
				(
					PenpalATeleportableAssetLocation::get(),
					PenpalASiblingSovereignAccount::get(),
					false,
					ED,
				),
				// PenpalB's teleportable asset representation
				(
					PenpalBTeleportableAssetLocation::get(),
					PenpalBSiblingSovereignAccount::get(),
					false,
					ED,
				),
				// Ether
				(
					Location::new(2, [GlobalConsensus(EthereumNetwork::get())]),
					EthereumSovereignAccount::get(),
					true,
					ETHER_MIN_BALANCE,
				),
				// Weth
				(
					Location::new(
						2,
						[
							GlobalConsensus(EthereumNetwork::get()),
							AccountKey20 { network: None, key: WETH.into() },
						],
					),
					EthereumSovereignAccount::get(),
					true,
					ETHER_MIN_BALANCE,
				),
			],
			reserves: vec![
				(
					PenpalATeleportableAssetLocation::get(),
					vec![(PenpalALocation::get(), true).into()],
				),
				(
					PenpalBTeleportableAssetLocation::get(),
					vec![(PenpalBLocation::get(), true).into()],
				),
				(EthereumLocation::get(), vec![(EthereumLocation::get(), false).into()]),
				(
					Location::new(
						2,
						[
							GlobalConsensus(EthereumNetwork::get()),
							AccountKey20 { network: None, key: WETH.into() },
						],
					),
					vec![(EthereumLocation::get(), false).into()],
				),
			],
			..Default::default()
		},
		..Default::default()
	};

	build_genesis_storage(
		&genesis_config,
		asset_hub_zagros_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
	)
}
