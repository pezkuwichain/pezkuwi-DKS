// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

use pezcumulus_primitives_core::ParaId;
use pezkuwi_omni_node_lib::{
	chain_spec::{GenericChainSpec, LoadSpec},
	runtime::{
		AuraConsensusId, BlockNumber, Consensus, Runtime, RuntimeResolver as RuntimeResolverT,
	},
};
use pezsc_chain_spec::{ChainSpec, ChainType};

pub mod asset_hubs;
pub mod bridge_hubs;
pub mod collectives;
pub mod coretime;
pub mod glutton;
pub mod penpal;
pub mod people;

/// Extracts the normalized chain id and teyrchain id from the input chain id.
/// (H/T to Phala for the idea)
/// E.g. "penpal-dicle-2004" yields ("penpal-dicle", Some(2004))
fn extract_teyrchain_id<'a>(
	id: &'a str,
	para_prefixes: &[&str],
) -> (&'a str, &'a str, Option<ParaId>) {
	for para_prefix in para_prefixes {
		if let Some(suffix) = id.strip_prefix(para_prefix) {
			let para_id: u32 = suffix.parse().expect("Invalid teyrchain-id suffix");
			return (&id[..para_prefix.len() - 1], id, Some(para_id.into()));
		}
	}

	(id, id, None)
}

#[derive(Debug)]
pub(crate) struct ChainSpecLoader;

impl LoadSpec for ChainSpecLoader {
	fn load_spec(&self, id: &str) -> Result<Box<dyn ChainSpec>, String> {
		Ok(match id {
			"tick" => Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/tick.json")[..],
			)?),
			"trick" => Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/trick.json")[..],
			)?),
			"track" => Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/track.json")[..],
			)?),

			// -- Asset Hub Pezkuwi
			"asset-hub-pezkuwi" => Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/asset-hub-pezkuwi.json")[..],
			)?),

			// -- Asset Hub Dicle
			"asset-hub-dicle" => Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/asset-hub-dicle.json")[..],
			)?),

			// -- Asset Hub Pezkuwichain
			"asset-hub-pezkuwichain-dev" => {
				Box::new(asset_hubs::asset_hub_pezkuwichain_development_config())
			},
			"asset-hub-pezkuwichain-local" => {
				Box::new(asset_hubs::asset_hub_pezkuwichain_local_config())
			},
			// the chain spec as used for generating the upgrade genesis values
			"asset-hub-pezkuwichain-genesis" => {
				Box::new(asset_hubs::asset_hub_pezkuwichain_genesis_config())
			},
			"asset-hub-pezkuwichain" => Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/asset-hub-pezkuwichain.json")[..],
			)?),

			// -- Asset Hub Zagros
			"asset-hub-zagros-dev" => Box::new(asset_hubs::asset_hub_zagros_development_config()),
			"asset-hub-zagros-local" => Box::new(asset_hubs::asset_hub_zagros_local_config()),
			// the chain spec as used for generating the upgrade genesis values
			"asset-hub-zagros-genesis" => Box::new(asset_hubs::asset_hub_zagros_config()),
			// the shell-based chain spec as used for syncing
			"asset-hub-zagros" => Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/asset-hub-zagros.json")[..],
			)?),

			// -- Pezkuwi Collectives
			"collectives-pezkuwi" => Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/collectives-pezkuwi.json")[..],
			)?),

			// -- Zagros Collectives
			"collectives-zagros-dev" => {
				Box::new(collectives::collectives_zagros_development_config())
			},
			"collectives-zagros-local" => Box::new(collectives::collectives_zagros_local_config()),
			"collectives-zagros" => Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/collectives-zagros.json")[..],
			)?),

			// -- BridgeHub
			bridge_like_id
				if bridge_like_id.starts_with(bridge_hubs::BridgeHubRuntimeType::ID_PREFIX) =>
			{
				bridge_like_id
					.parse::<bridge_hubs::BridgeHubRuntimeType>()
					.expect("invalid value")
					.load_config()?
			},

			// -- Coretime
			coretime_like_id
				if coretime_like_id.starts_with(coretime::CoretimeRuntimeType::ID_PREFIX) =>
			{
				coretime_like_id
					.parse::<coretime::CoretimeRuntimeType>()
					.expect("invalid value")
					.load_config()?
			},

			// -- Penpal
			id if id.starts_with("penpal-pezkuwichain") => {
				let (_, _, para_id) = extract_teyrchain_id(&id, &["penpal-pezkuwichain-"]);
				Box::new(penpal::get_penpal_chain_spec(
					para_id.expect("Must specify teyrchain id"),
					"pezkuwichain-local",
				))
			},
			id if id.starts_with("penpal-zagros") => {
				let (_, _, para_id) = extract_teyrchain_id(&id, &["penpal-zagros-"]);
				Box::new(penpal::get_penpal_chain_spec(
					para_id.expect("Must specify teyrchain id"),
					"zagros-local",
				))
			},

			// -- Glutton Zagros
			id if id.starts_with("glutton-zagros-dev") => {
				let (_, _, para_id) = extract_teyrchain_id(&id, &["glutton-zagros-dev-"]);
				Box::new(glutton::glutton_zagros_config(
					para_id.expect("Must specify teyrchain id"),
					ChainType::Development,
					"zagros-dev",
				))
			},
			id if id.starts_with("glutton-zagros-local") => {
				let (_, _, para_id) = extract_teyrchain_id(&id, &["glutton-zagros-local-"]);
				Box::new(glutton::glutton_zagros_config(
					para_id.expect("Must specify teyrchain id"),
					ChainType::Local,
					"zagros-local",
				))
			},
			// the chain spec as used for generating the upgrade genesis values
			id if id.starts_with("glutton-zagros-genesis") => {
				let (_, _, para_id) = extract_teyrchain_id(&id, &["glutton-zagros-genesis-"]);
				Box::new(glutton::glutton_zagros_config(
					para_id.expect("Must specify teyrchain id"),
					ChainType::Live,
					"zagros",
				))
			},

			// -- People
			people_like_id if people_like_id.starts_with(people::PeopleRuntimeType::ID_PREFIX) => {
				people_like_id
					.parse::<people::PeopleRuntimeType>()
					.expect("invalid value")
					.load_config()?
			},

			// -- Fallback (generic chainspec)
			"" => {
				log::warn!(
					"No ChainSpec.id specified, so using default one, based on Penpal runtime \
					 against a local Zagros relay"
				);
				Box::new(penpal::get_penpal_chain_spec(2000.into(), "zagros-local"))
			},

			// -- Loading a specific spec from disk
			path => Box::new(GenericChainSpec::from_json_file(path.into())?),
		})
	}
}

/// Helper enum that is used for better distinction of different teyrchain/runtime configuration
/// (it is based/calculated on ChainSpec's ID attribute)
#[derive(Debug, PartialEq)]
enum LegacyRuntime {
	Omni,
	AssetHubPezkuwi,
	AssetHub,
	Penpal,
	Collectives,
	Glutton,
	BridgeHub(bridge_hubs::BridgeHubRuntimeType),
	Coretime(coretime::CoretimeRuntimeType),
	People(people::PeopleRuntimeType),
}

impl LegacyRuntime {
	fn from_id(id: &str) -> LegacyRuntime {
		let id = id.replace('_', "-");

		// NOTE: Check longer prefixes FIRST to avoid false matches.
		// "asset-hub-pezkuwichain" starts with "asset-hub-pezkuwi", so we must check
		// pezkuwichain before pezkuwi.
		if id.starts_with("asset-hub-pezkuwichain")
			| id.starts_with("asset-hub-dicle")
			| id.starts_with("asset-hub-zagros")
		{
			LegacyRuntime::AssetHub
		} else if id.starts_with("asset-hub-pezkuwi") {
			LegacyRuntime::AssetHubPezkuwi
		} else if id.starts_with("penpal") {
			LegacyRuntime::Penpal
		} else if id.starts_with("collectives-pezkuwi") || id.starts_with("collectives-zagros") {
			LegacyRuntime::Collectives
		} else if id.starts_with(bridge_hubs::BridgeHubRuntimeType::ID_PREFIX) {
			LegacyRuntime::BridgeHub(
				id.parse::<bridge_hubs::BridgeHubRuntimeType>().expect("Invalid value"),
			)
		} else if id.starts_with(coretime::CoretimeRuntimeType::ID_PREFIX) {
			LegacyRuntime::Coretime(
				id.parse::<coretime::CoretimeRuntimeType>().expect("Invalid value"),
			)
		} else if id.starts_with("glutton") {
			LegacyRuntime::Glutton
		} else if id.starts_with(people::PeopleRuntimeType::ID_PREFIX) {
			LegacyRuntime::People(id.parse::<people::PeopleRuntimeType>().expect("Invalid value"))
		} else {
			log::warn!(
				"No specific runtime was recognized for ChainSpec's id: '{}', \
				so Runtime::Omni(Consensus::Aura) will be used",
				id
			);
			LegacyRuntime::Omni
		}
	}
}

#[derive(Debug)]
pub(crate) struct RuntimeResolver;

impl RuntimeResolverT for RuntimeResolver {
	fn runtime(&self, chain_spec: &dyn ChainSpec) -> pezsc_cli::Result<Runtime> {
		let legacy_runtime = LegacyRuntime::from_id(chain_spec.id());
		Ok(match legacy_runtime {
			LegacyRuntime::AssetHubPezkuwi => {
				Runtime::Omni(BlockNumber::U32, Consensus::Aura(AuraConsensusId::Ed25519))
			},
			LegacyRuntime::AssetHub
			| LegacyRuntime::BridgeHub(_)
			| LegacyRuntime::Collectives
			| LegacyRuntime::Coretime(_)
			| LegacyRuntime::People(_)
			| LegacyRuntime::Glutton
			| LegacyRuntime::Penpal
			| LegacyRuntime::Omni => {
				Runtime::Omni(BlockNumber::U32, Consensus::Aura(AuraConsensusId::Sr25519))
			},
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use pezsc_chain_spec::{ChainSpecExtension, ChainSpecGroup, ChainType, Extension};
	use serde::{Deserialize, Serialize};

	#[derive(
		Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension, Default,
	)]
	#[serde(deny_unknown_fields)]
	pub struct Extensions1 {
		pub attribute1: String,
		pub attribute2: u32,
	}

	#[derive(
		Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension, Default,
	)]
	#[serde(deny_unknown_fields)]
	pub struct Extensions2 {
		pub attribute_x: String,
		pub attribute_y: String,
		pub attribute_z: u32,
	}

	pub type DummyChainSpec<E> = pezsc_service::GenericChainSpec<E>;

	pub fn create_default_with_extensions<E: Extension>(
		id: &str,
		extension: E,
	) -> DummyChainSpec<E> {
		DummyChainSpec::builder(
			pezkuwichain_teyrchain_runtime::WASM_BINARY
				.expect("WASM binary was not built, please build it!"),
			extension,
		)
		.with_name("Dummy local testnet")
		.with_id(id)
		.with_chain_type(ChainType::Local)
		.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
		.build()
	}

	#[test]
	fn test_legacy_runtime_for_different_chain_specs() {
		let chain_spec =
			create_default_with_extensions("penpal-pezkuwichain-1000", Extensions2::default());
		assert_eq!(LegacyRuntime::Penpal, LegacyRuntime::from_id(chain_spec.id()));

		let chain_spec =
			crate::chain_spec::pezkuwichain_teyrchain::pezkuwichain_teyrchain_local_config();
		assert_eq!(LegacyRuntime::Omni, LegacyRuntime::from_id(chain_spec.id()));
	}
}
