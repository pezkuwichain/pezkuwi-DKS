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

// Cumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, collators, PEN2_TELEPORTABLE_ASSET_ID, SAFE_XCM_VERSION,
};
use pez_penpal_runtime::xcm_config::{
	LocalReservableFromAssetHub, PenpalNativeCurrency, RelayLocation, UsdtFromAssetHub,
};
use teyrchains_common::{AccountId, Balance};
// Penpal
pub const PARA_ID_A: u32 = 2000;
pub const PARA_ID_B: u32 = 2001;
pub const ED: Balance = pez_penpal_runtime::EXISTENTIAL_DEPOSIT;
pub const USDT_ED: Balance = 70_000;

parameter_types! {
	pub PenpalSudoAccount: AccountId = Keyring::Alice.to_account_id();
	pub PenpalAssetOwner: AccountId = PenpalSudoAccount::get();
}

pub fn genesis(para_id: u32) -> Storage {
	let genesis_config = pez_penpal_runtime::RuntimeGenesisConfig {
		system: pez_penpal_runtime::SystemConfig::default(),
		balances: pez_penpal_runtime::BalancesConfig {
			// Two parts, because two different things are funded from this one list.
			//
			// `ED * 4096` is what the transfers in these suites move: they are written in
			// deposits, so they scaled with the deposit when it was rederived from `CENTS`
			// and the relation still holds. Left at that on purpose — a test that fails for
			// want of funds says something, while one that passes only because every account
			// was handed far more than its case needs says nothing, and the tests asserting a
			// transfer is refused for insufficient balance are the first to go quiet.
			//
			// The four units are for the genesis asset-conversion pool below, which seeds one
			// whole unit of native liquidity from the owner in this same list. That amount is
			// absolute, so it did not move when the deposit did, and `ED * 4096` alone leaves
			// it seventy times short — genesis then fails with `Arithmetic(Underflow)` before
			// a single test runs. Measured, not padded: one unit for the pool, the rest headroom.
			balances: accounts::init_balances()
				.iter()
				.cloned()
				.map(|k| (k, pez_penpal_runtime::UNIT * 4 + ED * 4096))
				.collect(),
			..Default::default()
		},
		teyrchain_info: pez_penpal_runtime::TeyrchainInfoConfig {
			teyrchain_id: para_id.into(),
			..Default::default()
		},
		collator_selection: pez_penpal_runtime::CollatorSelectionConfig {
			invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: pez_penpal_runtime::SessionConfig {
			keys: collators::invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                              // account id
						acc,                                      // validator id
						pez_penpal_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
			..Default::default()
		},
		pezkuwi_xcm: pez_penpal_runtime::PezkuwiXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		sudo: pez_penpal_runtime::SudoConfig { key: Some(PenpalSudoAccount::get()) },
		// Upstream seeds all four of these into one pallet, because its penpal carries one. Ours
		// carries two, so each asset is seeded into the instance that `LocalAndForeignAssets`
		// routes its location to — otherwise the asset is registered where nothing will look for
		// it and every use of it fails with `UnknownAsset`.
		//
		// Pen2 is the one that lands on this side: its location is
		// `PalletInstance(50)/GeneralIndex(2)`, which is this chain's own address for asset 2 in
		// the index-keyed instance, so it is seeded by index rather than by location.
		assets: pez_penpal_runtime::AssetsConfig {
			assets: vec![(
				PEN2_TELEPORTABLE_ASSET_ID,
				PenpalAssetOwner::get(),
				false,
				ED,
			)],
			..Default::default()
		},
		foreign_assets: pez_penpal_runtime::ForeignAssetsConfig {
			assets: vec![
				// Relay Native asset representation
				(RelayLocation::get(), PenpalAssetOwner::get(), true, ED),
				// Sufficient AssetHub asset representation
				(LocalReservableFromAssetHub::get(), PenpalAssetOwner::get(), true, ED),
				// USDT from AssetHub
				(UsdtFromAssetHub::get(), PenpalAssetOwner::get(), true, USDT_ED),
			],
			accounts: vec![
				// Relay tokens for the pool liquidity provider.
				(RelayLocation::get(), PenpalAssetOwner::get(), 10_000_000_000_000),
			],
			..Default::default()
		},
		asset_conversion: pez_penpal_runtime::AssetConversionConfig {
			pools: vec![
				// Relay token pool (native PEN <-> relay HEZ) for XCM fee payment.
				(
					PenpalNativeCurrency::get(),
					RelayLocation::get(),
					PenpalAssetOwner::get(),
					1_000_000_000_000,
					2_000_000_000_000,
				),
			],
		},
		..Default::default()
	};

	build_genesis_storage(
		&genesis_config,
		pez_penpal_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
	)
}
