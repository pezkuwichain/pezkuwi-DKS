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
use pezframe_support::{parameter_types, pezsp_runtime::traits::AccountIdConversion};
use pezsp_core::storage::Storage;
use pezsp_keyring::Sr25519Keyring as Keyring;

// Pezcumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, collators,
	snowbridge::{ETHER_MIN_BALANCE, WETH},
	xcm_pez_emulator::ConvertLocation,
	PenpalALocation, PenpalAPen2TeleportableAssetLocation, PenpalASiblingSovereignAccount,
	PenpalBLocation, PenpalBPen2TeleportableAssetLocation, PenpalBSiblingSovereignAccount,
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
				// `AssetDeposit` is a tenth of a unit, which is roughly 30_000 existential
				// deposits on this chain; `ED * 4096` leaves every account short of it, so
				// `Assets::create` — the first line of most asset tests — fails with
				// `InsufficientBalance`. The mainnet Asset Hub's emulated chain already
				// funds accounts at this level.
				.map(|k| (k, ED * 4096 * 4096))
				// Pre-fund the checking account so tests teleporting funds in don't each have
				// to. With `MintLocation::Local`, assets teleported away are minted into this
				// account and assets arriving are burned from it, so it has to hold at least
				// as much as any inbound teleport or the transfer fails with
				// `NotWithdrawable`.
				//
				// `ED * 1000` was not enough: teleport amounts are denominated in the relay's
				// existential deposit, which is ten times this chain's, so the largest single
				// teleport in these tests (`ZAGROS_ED * 100`) came to 3_333_333_300 against a
				// balance of 3_333_333_000 — three hundred plancks short, with nothing left
				// over for the account's own existential deposit or for a second teleport.
				//
				// Two orders of magnitude of headroom over that largest teleport, since
				// several tests teleport in more than once.
				.chain(std::iter::once((
					asset_hub_zagros_runtime::xcm_config::CheckingAccount::get(),
					ED * 100_000,
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
				// PEZ, exactly as the launch preset creates it: keyless team, sufficient,
				// minimum balance of one. The runtime reads `PezAssetId = 1`, so this entry is
				// not an addition to the fixture -- it is the entry that was missing, and
				// without it every call that reaches for PEZ found a keyring-owned test asset
				// in its place. Metadata is left off: it carries no behaviour.
				(
					asset_hub_zagros_runtime::genesis_config_presets::PEZ_ASSET_ID,
					asset_hub_zagros_runtime::PezAssetTeamId::get().into_account_truncating(),
					true,
					1,
				),
				(RESERVABLE_ASSET_ID, AssetHubZagrosAssetOwner::get(), false, ED),
				(USDT_ID, AssetHubZagrosAssetOwner::get(), true, USDT_ED),
			],
			..Default::default()
		},
		foreign_assets: asset_hub_zagros_runtime::ForeignAssetsConfig {
			assets: vec![
				// PenpalA's teleportable asset representation
				(
					PenpalAPen2TeleportableAssetLocation::get(),
					PenpalASiblingSovereignAccount::get(),
					false,
					ED,
				),
				// PenpalB's teleportable asset representation
				(
					PenpalBPen2TeleportableAssetLocation::get(),
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
					PenpalAPen2TeleportableAssetLocation::get(),
					vec![(PenpalALocation::get(), true).into()],
				),
				(
					PenpalBPen2TeleportableAssetLocation::get(),
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
