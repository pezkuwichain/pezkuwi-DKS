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
use hex_literal::hex;
use pezcumulus_primitives_core::ParaId;
use pezframe_support::build_struct_json_patch;
use pezsp_core::crypto::UncheckedInto;
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
		// the live Pezkuwichain bridge hub ended up with 1,152,921 HEZ -- `1u128 << 60`, held
		// by a migration account inherited from the fork base rather than chosen here.
		PRESET_GENESIS => bridge_hub_zagros_genesis(
			// The two collators from the Zagros key set, derived from its master phrase at the
			// paths shown (res/genesis/zagros/zagros-wallets.json). Not Alice and Bob: the live
			// Zagros is keyed from its own phrase -- sudo included -- and so are its Asset Hub
			// and People collators.
			vec![
				// Zagros bridge-hub collator 1 (5E4opQwvJvBnbwcXDQkETzZ11S8YgoCGF8a6ma2xPEuPfPUU), `//zagros//collator//bridge-hub//1`
				(
					hex!("588c88a8aa5f77895f53b80aa4e7596705e4ebf6cb495c48f38b598d5a935c27").into(),
					hex!("588c88a8aa5f77895f53b80aa4e7596705e4ebf6cb495c48f38b598d5a935c27")
						.unchecked_into(),
				),
				// Zagros bridge-hub collator 2 (5HpaGQTzLRTUq9bCt2kEfgEPfTmtMBtUAY5g2u6xrCbR8LRj), `//zagros//collator//bridge-hub//2`
				(
					hex!("fea2698df97ca5a28dd04c43ef62b16e12c159e452de005a5e8294ca3ba6f72a").into(),
					hex!("fea2698df97ca5a28dd04c43ef62b16e12c159e452de005a5e8294ca3ba6f72a")
						.unchecked_into(),
				),
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
			// No bridge opened at genesis. Pezkuwichain launches with its relay, Asset Hub and
			// People only and takes a bridge hub once Zagros has run one long enough (Serok,
			// 2026-10-03), so a lane named here would point at a chain that does not exist yet,
			// keyed by a genesis hash that changes when mainnet is born. The lane is opened by
			// governance when the far side is live.
			vec![],
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

/// The launch preset names its own collators, and no development key anywhere. Zagros is
/// keyed from its own master phrase; a keyring account in a launched genesis is a key every
/// developer holds, sitting in a seat this chain gave it.
#[test]
fn the_launch_preset_names_no_keyring_account() {
	let raw = get_preset(&PresetId::from(preset_names::PRESET_GENESIS))
		.expect("the genesis preset exists");
	let text = core::str::from_utf8(&raw).expect("the preset is utf-8 json");
	for key in Sr25519Keyring::iter() {
		let ss58 = key.to_account_id().to_string();
		assert!(!text.contains(&ss58), "the launch preset names {key:?} ({ss58})");
	}

	let g: serde_json::Value = serde_json::from_str(text).expect("valid json");
	let invulnerables = g["collatorSelection"]["invulnerables"]
		.as_array()
		.expect("the preset sets the invulnerables");
	assert_eq!(invulnerables.len(), 2, "two collators, one per box");
}
