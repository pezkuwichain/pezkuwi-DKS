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

//! # Glutton Zagros Runtime genesis config presets

use crate::*;
use alloc::vec::Vec;
use pezcumulus_primitives_core::ParaId;
use pezframe_support::build_struct_json_patch;
use pezsp_genesis_builder::PresetId;
use pezsp_keyring::Sr25519Keyring;
use teyrchains_common::AuraId;

/// Default value, unused in a testnet setup currently because
/// we want to supply varying para-ids from the CLI for Glutton.
/// However, the presets does not allow dynamic para-ids currently.
pub const DEFAULT_GLUTTON_PARA_ID: ParaId = ParaId::new(1300);

pub fn glutton_zagros_genesis(
	authorities: Vec<AuraId>,
	sudo: Option<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	build_struct_json_patch!(RuntimeGenesisConfig {
		teyrchain_info: TeyrchainInfoConfig { teyrchain_id: id },
		aura: AuraConfig { authorities },
		sudo: SudoConfig { key: sudo }
	})
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_ref() {
		pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => glutton_zagros_genesis(
			// initial collators.
			vec![Sr25519Keyring::Alice.public().into(), Sr25519Keyring::Bob.public().into()],
			Some(Sr25519Keyring::Alice.to_account_id()),
			DEFAULT_GLUTTON_PARA_ID,
		),
		pezsp_genesis_builder::DEV_RUNTIME_PRESET => glutton_zagros_genesis(
			// initial collators.
			vec![Sr25519Keyring::Alice.public().into()],
			Some(Sr25519Keyring::Alice.to_account_id()),
			DEFAULT_GLUTTON_PARA_ID,
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
	vec![
		PresetId::from(pezsp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}
