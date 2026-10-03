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

use pezsc_chain_spec::{ChainSpec, ChainType};
use std::str::FromStr;

/// Collects all supported BridgeHub configurations
#[derive(Debug, PartialEq)]
pub enum BridgeHubRuntimeType {
	// The live ids stay even though no live spec is embedded: `LegacyRuntime::from_id` parses
	// a running node's chain id into this type, so a launched hub started from its own spec
	// file would not start if its id were missing here.
	Pezkuwichain,
	PezkuwichainLocal,
	// used by benchmarks
	PezkuwichainDevelopment,

	Zagros,
	ZagrosLocal,
	// used by benchmarks
	ZagrosDevelopment,
}

impl FromStr for BridgeHubRuntimeType {
	type Err = String;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			zagros::BRIDGE_HUB_ZAGROS => Ok(BridgeHubRuntimeType::Zagros),
			zagros::BRIDGE_HUB_ZAGROS_LOCAL => Ok(BridgeHubRuntimeType::ZagrosLocal),
			zagros::BRIDGE_HUB_ZAGROS_DEVELOPMENT => Ok(BridgeHubRuntimeType::ZagrosDevelopment),
			pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN => Ok(BridgeHubRuntimeType::Pezkuwichain),
			pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_LOCAL => {
				Ok(BridgeHubRuntimeType::PezkuwichainLocal)
			},
			pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_DEVELOPMENT => {
				Ok(BridgeHubRuntimeType::PezkuwichainDevelopment)
			},
			_ => Err(format!("Value '{}' is not configured yet", value)),
		}
	}
}

impl BridgeHubRuntimeType {
	pub const ID_PREFIX: &'static str = "bridge-hub";

	pub fn load_config(&self) -> Result<Box<dyn ChainSpec>, String> {
		match self {
			BridgeHubRuntimeType::Zagros | BridgeHubRuntimeType::Pezkuwichain => {
				Err(std::format!("{self:?}: not launched: neither network runs this chain yet, so no spec of it is embedded. Zagros takes it first and Pezkuwichain after it; pass the launched chain's spec as a file"))
			},
			BridgeHubRuntimeType::ZagrosLocal => Ok(Box::new(zagros::local_config(
				zagros::BRIDGE_HUB_ZAGROS_LOCAL,
				"Zagros BridgeHub Local",
				"zagros-local",
				ChainType::Local,
			))),
			BridgeHubRuntimeType::ZagrosDevelopment => Ok(Box::new(zagros::local_config(
				zagros::BRIDGE_HUB_ZAGROS_DEVELOPMENT,
				"Zagros BridgeHub Development",
				"zagros-dev",
				ChainType::Development,
			))),
			BridgeHubRuntimeType::PezkuwichainLocal => Ok(Box::new(pezkuwichain::local_config(
				pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_LOCAL,
				"Pezkuwichain BridgeHub Local",
				"pezkuwichain-local",
				|_| (),
				ChainType::Local,
			))),
			BridgeHubRuntimeType::PezkuwichainDevelopment => {
				Ok(Box::new(pezkuwichain::local_config(
					pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_DEVELOPMENT,
					"Pezkuwichain BridgeHub Development",
					"pezkuwichain-dev",
					|_| (),
					ChainType::Development,
				)))
			},
		}
	}
}

/// Check if 'id' satisfy BridgeHub-like format
fn ensure_id(id: &str) -> Result<&str, String> {
	if id.starts_with(BridgeHubRuntimeType::ID_PREFIX) {
		Ok(id)
	} else {
		Err(format!(
			"Invalid 'id' attribute ({}), should start with prefix: {}",
			id,
			BridgeHubRuntimeType::ID_PREFIX
		))
	}
}

/// Sub-module for Pezkuwichain setup
pub mod pezkuwichain {
	use super::ChainType;
	use pezkuwi_omni_node_lib::chain_spec::{Extensions, GenericChainSpec};

	pub(crate) const BRIDGE_HUB_PEZKUWICHAIN: &str = "bridge-hub-pezkuwichain";
	pub(crate) const BRIDGE_HUB_PEZKUWICHAIN_LOCAL: &str = "bridge-hub-pezkuwichain-local";
	pub(crate) const BRIDGE_HUB_PEZKUWICHAIN_DEVELOPMENT: &str = "bridge-hub-pezkuwichain-dev";

	pub fn local_config<ModifyProperties: Fn(&mut pezsc_chain_spec::Properties)>(
		id: &str,
		chain_name: &str,
		relay_chain: &str,
		modify_props: ModifyProperties,
		chain_type: ChainType,
	) -> GenericChainSpec {
		// Pezkuwichain defaults
		let mut properties = pezsc_chain_spec::Properties::new();
		properties.insert("ss58Format".into(), 42.into());
		properties.insert("tokenSymbol".into(), "HEZ".into());
		properties.insert("tokenDecimals".into(), 12.into());
		modify_props(&mut properties);

		GenericChainSpec::builder(
			pezbridge_hub_pezkuwichain_runtime::WASM_BINARY
				.expect("WASM binary was not built, please build it!"),
			Extensions::new_with_relay_chain(relay_chain.to_string()),
		)
		.with_name(chain_name)
		.with_id(super::ensure_id(id).expect("invalid id"))
		.with_chain_type(chain_type.clone())
		.with_genesis_config_preset_name(match chain_type {
			ChainType::Development => pezsp_genesis_builder::DEV_RUNTIME_PRESET,
			ChainType::Local => pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET,
			_ => panic!("chain_type: {chain_type:?} not supported here!"),
		})
		.with_properties(properties)
		.build()
	}
}

/// Sub-module for Zagros setup.
pub mod zagros {
	use super::ChainType;
	use pezkuwi_omni_node_lib::chain_spec::{Extensions, GenericChainSpec};

	pub(crate) const BRIDGE_HUB_ZAGROS: &str = "bridge-hub-zagros";
	pub(crate) const BRIDGE_HUB_ZAGROS_LOCAL: &str = "bridge-hub-zagros-local";
	pub(crate) const BRIDGE_HUB_ZAGROS_DEVELOPMENT: &str = "bridge-hub-zagros-dev";

	pub fn local_config(
		id: &str,
		chain_name: &str,
		relay_chain: &str,
		chain_type: ChainType,
	) -> GenericChainSpec {
		let mut properties = pezsc_chain_spec::Properties::new();
		properties.insert("tokenSymbol".into(), "HEZ".into());
		properties.insert("tokenDecimals".into(), 12.into());

		GenericChainSpec::builder(
			pezbridge_hub_zagros_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!"),
			Extensions::new_with_relay_chain(relay_chain.to_string()),
		)
		.with_name(chain_name)
		.with_id(super::ensure_id(id).expect("invalid id"))
		.with_chain_type(chain_type.clone())
		.with_genesis_config_preset_name(match chain_type {
			ChainType::Development => pezsp_genesis_builder::DEV_RUNTIME_PRESET,
			ChainType::Local => pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET,
			_ => panic!("chain_type: {chain_type:?} not supported here!"),
		})
		.with_properties(properties)
		.build()
	}
}
