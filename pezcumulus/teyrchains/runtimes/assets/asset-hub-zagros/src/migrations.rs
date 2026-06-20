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

use crate::xcm_config::bridging::to_pezkuwichain::{AssetHubPezkuwichain, PezkuwichainEcosystem};
use alloc::{vec, vec::Vec};
use pez_assets_common::{
	local_and_foreign_assets::ForeignAssetReserveData,
	migrations::foreign_assets_reserves::ForeignAssetsReservesProvider,
};
use pezframe_support::traits::Contains;
use testnet_teyrchains_constants::zagros::snowbridge::EthereumLocation;
use xcm::v5::{Junction, Location};
use xcm_builder::StartsWith;
use zagros_runtime_constants::system_teyrchain::ASSET_HUB_ID;

/// This type provides reserves information for `asset_id`. Meant to be used in a migration running
/// on the Asset Hub Zagros upgrade which changes the Foreign Assets reserve-transfers and
/// teleports from hardcoded rules to per-asset configured reserves.
///
/// The hardcoded rules (see `xcm_config.rs`) migrated here:
/// 1. Foreign Assets native to sibling teyrchains are teleportable between the asset's native chain
///    and Asset Hub.
///  ----> `ForeignAssetReserveData { reserve: "Asset's native chain", teleport: true }`
/// 2. Foreign assets native to Ethereum Ecosystem have Ethereum as trusted reserve.
///  ----> `ForeignAssetReserveData { reserve: "Ethereum", teleport: false }`
/// 3. Foreign assets native to Pezkuwichain Ecosystem have Asset Hub Pezkuwichain as trusted
///    reserve.
///  ----> `ForeignAssetReserveData { reserve: "Asset Hub Pezkuwichain", teleport: false }`
pub struct AssetHubZagrosForeignAssetsReservesProvider;
impl ForeignAssetsReservesProvider for AssetHubZagrosForeignAssetsReservesProvider {
	type ReserveData = ForeignAssetReserveData;
	fn reserves_for(asset_id: &Location) -> Vec<Self::ReserveData> {
		let reserves = if StartsWith::<PezkuwichainEcosystem>::contains(asset_id) {
			// rule 3: pezkuwichain asset, Asset Hub Pezkuwichain reserve, non teleportable
			vec![(AssetHubPezkuwichain::get(), false).into()]
		} else if StartsWith::<EthereumLocation>::contains(asset_id) {
			// rule 2: ethereum asset, ethereum reserve, non teleportable
			vec![(EthereumLocation::get(), false).into()]
		} else {
			match asset_id.unpack() {
				(1, interior) => {
					match interior.first() {
						Some(Junction::Teyrchain(sibling_para_id))
							if sibling_para_id.ne(&ASSET_HUB_ID) =>
						{
							// rule 1: sibling teyrchain asset, sibling teyrchain reserve,
							// teleportable
							vec![ForeignAssetReserveData {
								reserve: Location::new(1, Junction::Teyrchain(*sibling_para_id)),
								teleportable: true,
							}]
						},
						_ => vec![],
					}
				},
				_ => vec![],
			}
		};
		if reserves.is_empty() {
			tracing::error!(
				target: "runtime::AssetHubZagrosForeignAssetsReservesProvider::reserves_for",
				id = ?asset_id, "unexpected asset",
			);
		}
		reserves
	}

	#[cfg(feature = "try-runtime")]
	fn check_reserves_for(asset_id: &Location, reserves: Vec<Self::ReserveData>) -> bool {
		if StartsWith::<PezkuwichainEcosystem>::contains(asset_id) {
			let expected = ForeignAssetReserveData {
				reserve: AssetHubPezkuwichain::get(),
				teleportable: false,
			};
			// rule 3: pezkuwichain asset
			reserves.len() == 1 && expected.eq(reserves.get(0).unwrap())
		} else if StartsWith::<EthereumLocation>::contains(asset_id) {
			let expected =
				ForeignAssetReserveData { reserve: EthereumLocation::get(), teleportable: false };
			// rule 2: ethereum asset
			reserves.len() == 1 && expected.eq(reserves.get(0).unwrap())
		} else {
			match asset_id.unpack() {
				(1, interior) => {
					match interior.first() {
						Some(Junction::Teyrchain(sibling_para_id))
							if sibling_para_id.ne(&ASSET_HUB_ID) =>
						{
							let expected = ForeignAssetReserveData {
								reserve: Location::new(1, Junction::Teyrchain(*sibling_para_id)),
								teleportable: true,
							};
							// rule 1: sibling teyrchain asset
							reserves.len() == 1 && expected.eq(reserves.get(0).unwrap())
						},
						// unexpected asset
						_ => false,
					}
				},
				// we have some junk assets registered on AHW with `GlobalConsensus(Pezkuwi)`
				(2, _) => reserves.is_empty(),
				// unexpected asset
				_ => false,
			}
		}
	}
}
