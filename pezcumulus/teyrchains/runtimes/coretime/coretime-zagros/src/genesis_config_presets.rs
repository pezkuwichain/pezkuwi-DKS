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

//! # Coretime Zagros Runtime genesis config presets

use crate::*;
use alloc::{vec, vec::Vec};
use hex_literal::hex;
use pezcumulus_primitives_core::ParaId;
use pezframe_support::build_struct_json_patch;
use pezsp_core::crypto::UncheckedInto;
use pezsp_genesis_builder::PresetId;
use pezsp_keyring::Sr25519Keyring;
use testnet_teyrchains_constants::zagros::{currency::UNITS as ZGR, xcm_version::SAFE_XCM_VERSION};
use teyrchains_common::{AccountId, AuraId};

const CORETIME_ZAGROS_ED: Balance = ExistentialDeposit::get();
pub const CORETIME_PARA_ID: ParaId = ParaId::new(1005);

fn coretime_zagros_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	endowment: Balance,
	id: ParaId,
) -> serde_json::Value {
	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, endowment)).collect(),
		},
		teyrchain_info: TeyrchainInfoConfig { teyrchain_id: id },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: CORETIME_ZAGROS_ED * 16,
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
	})
}

/// Provides the JSON representation of predefined genesis config for given `id`.
mod preset_names {
	pub const PRESET_GENESIS: &str = "genesis";
}

pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_ref() {
		// The preset a real Zagros launch uses. Endows nobody, which is what upstream's own live
		// system-parachain specs do -- measured from the raw genesis in `chain-specs/`: bridge
		// hub, coretime and people all carry zero balance, and collectives carries four HEZ
		// across nine accounts. The `dev` and `local` presets below hand `well_known()` large
		// sums because a test network needs spendable keys; shipping that to a launch is how
		// the live Pezkuwichain bridge hub ended up with 1,152,921 HEZ -- `1u128 << 60`, held
		// by a migration account inherited from the fork base rather than chosen here.
		PRESET_GENESIS => coretime_zagros_genesis(
			// The two collators from the Zagros key set, derived from its master phrase at the
			// paths shown (res/genesis/zagros/zagros-wallets.json). Not Alice and Bob: the live
			// Zagros is keyed from its own phrase -- sudo included -- and so are its Asset Hub
			// and People collators.
			vec![
				// Zagros coretime collator 1 (5DF7RXz16xETjNwpDq6cfXWH7parrjdoA8VA5ZS2yQshcdKo), `//zagros//collator//coretime//1`
				(
					hex!("342c051b10dd5d4f49c31cf2fe24b240e2ed5280242e1eceffff067c3432e025").into(),
					hex!("342c051b10dd5d4f49c31cf2fe24b240e2ed5280242e1eceffff067c3432e025")
						.unchecked_into(),
				),
				// Zagros coretime collator 2 (5EZwCmb7H1tJFePkjJEMrQ3xF1D4HbL8i6sHhq2gGQ4VM8tx), `//zagros//collator//coretime//2`
				(
					hex!("6ec3831e1f8d4d928f9d0c9464191aada4db21e5ff0efedeb1f871e945050958").into(),
					hex!("6ec3831e1f8d4d928f9d0c9464191aada4db21e5ff0efedeb1f871e945050958")
						.unchecked_into(),
				),
			],
			// No endowed accounts: a launched chain funds nobody here. Test accounts are
			// funded after launch by teleport, which is the path mainnet uses.
			Vec::new(),
			ZGR * 1_000_000,
			CORETIME_PARA_ID,
		),
		pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => coretime_zagros_genesis(
			// initial collators.
			vec![
				(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
				(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
			],
			Sr25519Keyring::well_known().map(|k| k.to_account_id()).collect(),
			ZGR * 1_000_000,
			CORETIME_PARA_ID,
		),
		pezsp_genesis_builder::DEV_RUNTIME_PRESET => coretime_zagros_genesis(
			// initial collators.
			vec![(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into())],
			vec![
				Sr25519Keyring::Alice.to_account_id(),
				Sr25519Keyring::Bob.to_account_id(),
				Sr25519Keyring::AliceStash.to_account_id(),
				Sr25519Keyring::BobStash.to_account_id(),
			],
			ZGR * 1_000_000,
			CORETIME_PARA_ID,
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
