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

const COLLECTIVES_ZAGROS_ED: Balance = ExistentialDeposit::get();

fn collectives_zagros_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, COLLECTIVES_ZAGROS_ED * 4096))
				.collect::<Vec<_>>(),
		},
		teyrchain_info: TeyrchainInfoConfig { teyrchain_id: id },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: COLLECTIVES_ZAGROS_ED * 16,
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
		PRESET_GENESIS => collectives_zagros_genesis(
			// initial collators.
			vec![
				(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
				(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
			],
			// No endowed accounts: a launched chain funds nobody here.
			Vec::new(),
			1001.into(),
		),
		pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => collectives_zagros_genesis(
			// initial collators.
			vec![
				(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
				(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
			],
			Sr25519Keyring::well_known().map(|k| k.to_account_id()).collect(),
			1001.into(),
		),
		pezsp_genesis_builder::DEV_RUNTIME_PRESET => collectives_zagros_genesis(
			// initial collators.
			vec![(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into())],
			vec![
				Sr25519Keyring::Alice.to_account_id(),
				Sr25519Keyring::Bob.to_account_id(),
				Sr25519Keyring::AliceStash.to_account_id(),
				Sr25519Keyring::BobStash.to_account_id(),
			],
			1001.into(),
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
