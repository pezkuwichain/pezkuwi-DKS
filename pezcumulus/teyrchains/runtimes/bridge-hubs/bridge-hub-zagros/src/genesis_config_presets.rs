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

//! # Bridge Hub Zagros Runtime genesis config presets

use crate::*;
use alloc::{vec, vec::Vec};
use pezcumulus_primitives_core::ParaId;
use pezframe_support::build_struct_json_patch;
use pezsp_genesis_builder::PresetId;
use pezsp_keyring::Sr25519Keyring;
use testnet_teyrchains_constants::zagros::xcm_version::SAFE_XCM_VERSION;
use teyrchains_common::{AccountId, AuraId};
use xcm::latest::PEZKUWICHAIN_GENESIS_HASH;

const BRIDGE_HUB_ZAGROS_ED: Balance = ExistentialDeposit::get();

fn bridge_hub_zagros_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
	bridges_pezpallet_owner: Option<AccountId>,
	asset_hub_para_id: ParaId,
	opened_bridges: Vec<(Location, InteriorLocation, Option<pezbp_messages::LegacyLaneId>)>,
) -> serde_json::Value {
	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1u128 << 60))
				.collect::<Vec<_>>(),
		},
		teyrchain_info: TeyrchainInfoConfig { teyrchain_id: id },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: BRIDGE_HUB_ZAGROS_ED * 16,
		},
		session: SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),          // account id
						acc,                  // validator id
						SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		pezkuwi_xcm: PezkuwiXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },
		bridge_pezkuwichain_grandpa: BridgePezkuwichainGrandpaConfig {
			owner: bridges_pezpallet_owner.clone()
		},
		bridge_pezkuwichain_messages: BridgePezkuwichainMessagesConfig {
			owner: bridges_pezpallet_owner.clone()
		},
		xcm_over_bridge_hub_pezkuwichain: XcmOverBridgeHubPezkuwichainConfig { opened_bridges },
		ethereum_system: EthereumSystemConfig { para_id: id, asset_hub_para_id },
	})
}

/// Provides the JSON representation of predefined genesis config for given `id`.
mod preset_names {
	pub const PRESET_GENESIS: &str = "genesis";
}

pub fn get_preset(id: &pezsp_genesis_builder::PresetId) -> Option<pezsp_std::vec::Vec<u8>> {
	use preset_names::*;
	let patch = match id.as_ref() {
		// The preset a real Zagros launch uses. Endows nobody, which is what upstream's own live
		// system-parachain specs do -- measured from the raw genesis in `chain-specs/`: bridge
		// hub, coretime and people all carry zero balance, and collectives carries four HEZ
		// across nine accounts. The `dev` and `local` presets below hand `well_known()` large
		// sums because a test network needs spendable keys; shipping that to a launch is how
		// the live Pezkuwichain bridge hub ended up with 1,152,921 HEZ -- `1u128 << 60`, to
		// Westend's migration controller, inherited rather than chosen.
		PRESET_GENESIS => bridge_hub_zagros_genesis(
			// Alice and Bob as collators, and on purpose: Zagros is a testnet whose sudo is
			// Alice for the same reason -- every developer already holds these keys and there
			// is nothing here to protect from them. Producing blocks is not a privileged
			// position the way owning the bridge switch is; the account above is the one that
			// mattered. Pezkuwichain names real collators.
			vec![
				(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
				(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
			],
			// No endowed accounts: a launched chain funds nobody here. Relayers and test
			// accounts are funded after launch by teleport -- `TrustedTeleporters` accepts
			// HEZ from the relay and every system chain -- which is the path mainnet will
			// use, so Zagros rehearsing it is the point rather than an inconvenience.
			Vec::new(),
			1002.into(),
			// No pallet owner: halting and resuming a bridge is root's, which here means
			// governance. Copied from the local preset this was `Some(Bob)` -- a keyring
			// account holding a privileged switch on a launched chain. Measured against the
			// live Pezkuwichain bridge hub, whose raw genesis carries no owner at all.
			None,
			zagros_runtime_constants::system_teyrchain::ASSET_HUB_ID.into(),
			vec![(
				Location::new(1, [Teyrchain(1000)]),
				Junctions::from([
					NetworkId::ByGenesis(PEZKUWICHAIN_GENESIS_HASH).into(),
					Teyrchain(1000),
				]),
				Some(pezbp_messages::LegacyLaneId([0, 0, 0, 2])),
			)],
		),
		pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => bridge_hub_zagros_genesis(
			// initial collators.
			vec![
				(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
				(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
			],
			Sr25519Keyring::well_known().map(|k| k.to_account_id()).collect(),
			1002.into(),
			Some(Sr25519Keyring::Bob.to_account_id()),
			zagros_runtime_constants::system_teyrchain::ASSET_HUB_ID.into(),
			vec![(
				Location::new(1, [Teyrchain(1000)]),
				Junctions::from([
					NetworkId::ByGenesis(PEZKUWICHAIN_GENESIS_HASH).into(),
					Teyrchain(1000),
				]),
				Some(pezbp_messages::LegacyLaneId([0, 0, 0, 2])),
			)],
		),
		pezsp_genesis_builder::DEV_RUNTIME_PRESET => bridge_hub_zagros_genesis(
			// initial collators.
			vec![
				(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
				(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
			],
			Sr25519Keyring::well_known().map(|k| k.to_account_id()).collect(),
			1002.into(),
			Some(Sr25519Keyring::Bob.to_account_id()),
			zagros_runtime_constants::system_teyrchain::ASSET_HUB_ID.into(),
			vec![],
		),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
	use preset_names::*;
	vec![
		PresetId::from(PRESET_GENESIS),
		PresetId::from(pezsp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}
