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

use pezkuwi_omni_node_lib::chain_spec::GenericChainSpec;
use pezsc_chain_spec::{ChainSpec, ChainType};
use std::{borrow::Cow, str::FromStr};

/// Collects all supported Coretime configurations.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CoretimeRuntimeType {
	Dicle,
	DicleLocal,

	Pezkuwi,
	PezkuwiLocal,

	// Live
	Pezkuwichain,
	// Local
	PezkuwichainLocal,
	// Benchmarks
	PezkuwichainDevelopment,

	// Live
	Zagros,
	// Local
	ZagrosLocal,
	// Benchmarks
	ZagrosDevelopment,
}

impl FromStr for CoretimeRuntimeType {
	type Err = String;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			dicle::CORETIME_DICLE => Ok(CoretimeRuntimeType::Dicle),
			dicle::CORETIME_DICLE_LOCAL => Ok(CoretimeRuntimeType::DicleLocal),
			pezkuwi::CORETIME_PEZKUWI => Ok(CoretimeRuntimeType::Pezkuwi),
			pezkuwi::CORETIME_PEZKUWI_LOCAL => Ok(CoretimeRuntimeType::PezkuwiLocal),
			pezkuwichain::CORETIME_PEZKUWICHAIN => Ok(CoretimeRuntimeType::Pezkuwichain),
			pezkuwichain::CORETIME_PEZKUWICHAIN_LOCAL => Ok(CoretimeRuntimeType::PezkuwichainLocal),
			pezkuwichain::CORETIME_PEZKUWICHAIN_DEVELOPMENT => {
				Ok(CoretimeRuntimeType::PezkuwichainDevelopment)
			},
			zagros::CORETIME_ZAGROS => Ok(CoretimeRuntimeType::Zagros),
			zagros::CORETIME_ZAGROS_LOCAL => Ok(CoretimeRuntimeType::ZagrosLocal),
			zagros::CORETIME_ZAGROS_DEVELOPMENT => Ok(CoretimeRuntimeType::ZagrosDevelopment),
			_ => Err(format!("Value '{}' is not configured yet", value)),
		}
	}
}

impl From<CoretimeRuntimeType> for &str {
	fn from(runtime_type: CoretimeRuntimeType) -> Self {
		match runtime_type {
			CoretimeRuntimeType::Dicle => dicle::CORETIME_DICLE,
			CoretimeRuntimeType::DicleLocal => dicle::CORETIME_DICLE_LOCAL,
			CoretimeRuntimeType::Pezkuwi => pezkuwi::CORETIME_PEZKUWI,
			CoretimeRuntimeType::PezkuwiLocal => pezkuwi::CORETIME_PEZKUWI_LOCAL,
			CoretimeRuntimeType::Pezkuwichain => pezkuwichain::CORETIME_PEZKUWICHAIN,
			CoretimeRuntimeType::PezkuwichainLocal => pezkuwichain::CORETIME_PEZKUWICHAIN_LOCAL,
			CoretimeRuntimeType::PezkuwichainDevelopment => {
				pezkuwichain::CORETIME_PEZKUWICHAIN_DEVELOPMENT
			},
			CoretimeRuntimeType::Zagros => zagros::CORETIME_ZAGROS,
			CoretimeRuntimeType::ZagrosLocal => zagros::CORETIME_ZAGROS_LOCAL,
			CoretimeRuntimeType::ZagrosDevelopment => zagros::CORETIME_ZAGROS_DEVELOPMENT,
		}
	}
}

impl From<CoretimeRuntimeType> for ChainType {
	fn from(runtime_type: CoretimeRuntimeType) -> Self {
		match runtime_type {
			CoretimeRuntimeType::Dicle
			| CoretimeRuntimeType::Pezkuwi
			| CoretimeRuntimeType::Pezkuwichain
			| CoretimeRuntimeType::Zagros => ChainType::Live,
			CoretimeRuntimeType::DicleLocal
			| CoretimeRuntimeType::PezkuwiLocal
			| CoretimeRuntimeType::PezkuwichainLocal
			| CoretimeRuntimeType::ZagrosLocal => ChainType::Local,
			CoretimeRuntimeType::PezkuwichainDevelopment
			| CoretimeRuntimeType::ZagrosDevelopment => ChainType::Development,
		}
	}
}

impl CoretimeRuntimeType {
	pub const ID_PREFIX: &'static str = "coretime";

	pub fn load_config(&self) -> Result<Box<dyn ChainSpec>, String> {
		match self {
			CoretimeRuntimeType::Dicle => Ok(Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/coretime-dicle.json")[..],
			)?)),
			CoretimeRuntimeType::Pezkuwi => Ok(Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/coretime-pezkuwi.json")[..],
			)?)),
			CoretimeRuntimeType::Pezkuwichain => Ok(Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/coretime-pezkuwichain.json")[..],
			)?)),
			CoretimeRuntimeType::PezkuwichainLocal => {
				Ok(Box::new(pezkuwichain::local_config(*self, "pezkuwichain-local")))
			},
			CoretimeRuntimeType::PezkuwichainDevelopment => {
				Ok(Box::new(pezkuwichain::local_config(*self, "pezkuwichain-dev")))
			},
			CoretimeRuntimeType::Zagros => Ok(Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../../teyrchains/chain-specs/coretime-zagros.json")[..],
			)?)),
			CoretimeRuntimeType::ZagrosLocal => {
				Ok(Box::new(zagros::local_config(*self, "zagros-local")))
			},
			CoretimeRuntimeType::ZagrosDevelopment => {
				Ok(Box::new(zagros::local_config(*self, "zagros-dev")))
			},
			other => Err(std::format!(
				"No default config present for {:?}, you should provide a chain-spec as json file!",
				other
			)),
		}
	}
}

/// Generate the name directly from the ChainType
pub fn chain_type_name(chain_type: &ChainType) -> Cow<'_, str> {
	match chain_type {
		ChainType::Development => "Development",
		ChainType::Local => "Local",
		ChainType::Live => "Live",
		ChainType::Custom(name) => name,
	}
	.into()
}

/// Sub-module for Pezkuwichain setup.
pub mod pezkuwichain {
	use super::{chain_type_name, CoretimeRuntimeType};
	use pezkuwi_omni_node_lib::chain_spec::{Extensions, GenericChainSpec};
	use pezsc_chain_spec::ChainType;

	pub(crate) const CORETIME_PEZKUWICHAIN: &str = "coretime-pezkuwichain";
	pub(crate) const CORETIME_PEZKUWICHAIN_LOCAL: &str = "coretime-pezkuwichain-local";
	pub(crate) const CORETIME_PEZKUWICHAIN_DEVELOPMENT: &str = "coretime-pezkuwichain-dev";

	pub fn local_config(runtime_type: CoretimeRuntimeType, relay_chain: &str) -> GenericChainSpec {
		// Pezkuwichain defaults
		let mut properties = pezsc_chain_spec::Properties::new();
		properties.insert("ss58Format".into(), 42.into());
		properties.insert("tokenSymbol".into(), "HEZ".into());
		properties.insert("tokenDecimals".into(), 12.into());

		let chain_type = runtime_type.into();
		let chain_name = format!("Coretime Pezkuwichain {}", chain_type_name(&chain_type));

		let wasm_binary = if matches!(chain_type, ChainType::Local | ChainType::Development) {
			coretime_pezkuwichain_runtime::fast_runtime_binary::WASM_BINARY
				.expect("WASM binary was not built, please build it!")
		} else {
			coretime_pezkuwichain_runtime::WASM_BINARY
				.expect("WASM binary was not built, please build it!")
		};

		GenericChainSpec::builder(
			wasm_binary,
			Extensions::new_with_relay_chain(relay_chain.to_string()),
		)
		.with_name(&chain_name)
		.with_id(runtime_type.into())
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
	use super::{chain_type_name, CoretimeRuntimeType, GenericChainSpec};
	use pezkuwi_omni_node_lib::chain_spec::Extensions;
	use pezsc_chain_spec::ChainType;

	pub(crate) const CORETIME_ZAGROS: &str = "coretime-zagros";
	pub(crate) const CORETIME_ZAGROS_LOCAL: &str = "coretime-zagros-local";
	pub(crate) const CORETIME_ZAGROS_DEVELOPMENT: &str = "coretime-zagros-dev";

	pub fn local_config(runtime_type: CoretimeRuntimeType, relay_chain: &str) -> GenericChainSpec {
		// zagros defaults
		let mut properties = pezsc_chain_spec::Properties::new();
		properties.insert("ss58Format".into(), 42.into());
		properties.insert("tokenSymbol".into(), "ZGR".into());
		properties.insert("tokenDecimals".into(), 12.into());

		let chain_type = runtime_type.into();
		let chain_name = format!("Coretime Zagros {}", chain_type_name(&chain_type));

		GenericChainSpec::builder(
			coretime_zagros_runtime::WASM_BINARY
				.expect("WASM binary was not built, please build it!"),
			Extensions::new_with_relay_chain(relay_chain.to_string()),
		)
		.with_name(&chain_name)
		.with_id(runtime_type.into())
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

pub mod dicle {
	pub(crate) const CORETIME_DICLE: &str = "coretime-dicle";
	pub(crate) const CORETIME_DICLE_LOCAL: &str = "coretime-dicle-local";
}

pub mod pezkuwi {
	pub(crate) const CORETIME_PEZKUWI: &str = "coretime-pezkuwi";
	pub(crate) const CORETIME_PEZKUWI_LOCAL: &str = "coretime-pezkuwi-local";
}
