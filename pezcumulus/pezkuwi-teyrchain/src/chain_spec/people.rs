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
use std::str::FromStr;

/// Collects all supported People configurations.
#[derive(Debug, PartialEq)]
pub enum PeopleRuntimeType {
	Dicle,
	DicleLocal,
	Pezkuwi,
	PezkuwiLocal,
	Pezkuwichain,
	PezkuwichainGenesis,
	PezkuwichainLocal,
	PezkuwichainDevelopment,
	Zagros,
	ZagrosLocal,
	ZagrosDevelopment,
}

impl FromStr for PeopleRuntimeType {
	type Err = String;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			dicle::PEOPLE_DICLE => Ok(PeopleRuntimeType::Dicle),
			dicle::PEOPLE_DICLE_LOCAL => Ok(PeopleRuntimeType::DicleLocal),
			pezkuwi::PEOPLE_PEZKUWI => Ok(PeopleRuntimeType::Pezkuwi),
			pezkuwi::PEOPLE_PEZKUWI_LOCAL => Ok(PeopleRuntimeType::PezkuwiLocal),
			pezkuwichain::PEOPLE_PEZKUWICHAIN => Ok(PeopleRuntimeType::Pezkuwichain),
			pezkuwichain::PEOPLE_PEZKUWICHAIN_GENESIS => Ok(PeopleRuntimeType::PezkuwichainGenesis),
			pezkuwichain::PEOPLE_PEZKUWICHAIN_LOCAL => Ok(PeopleRuntimeType::PezkuwichainLocal),
			pezkuwichain::PEOPLE_PEZKUWICHAIN_DEVELOPMENT => {
				Ok(PeopleRuntimeType::PezkuwichainDevelopment)
			},
			zagros::PEOPLE_ZAGROS => Ok(PeopleRuntimeType::Zagros),
			zagros::PEOPLE_ZAGROS_LOCAL => Ok(PeopleRuntimeType::ZagrosLocal),
			zagros::PEOPLE_ZAGROS_DEVELOPMENT => Ok(PeopleRuntimeType::ZagrosDevelopment),
			_ => Err(format!("Value '{}' is not configured yet", value)),
		}
	}
}

impl PeopleRuntimeType {
	pub const ID_PREFIX: &'static str = "people";

	pub fn load_config(&self) -> Result<Box<dyn ChainSpec>, String> {
		match self {
			PeopleRuntimeType::Dicle => Ok(Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/people-dicle.json")[..],
			)?)),
			PeopleRuntimeType::Pezkuwi => Ok(Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/people-pezkuwi.json")[..],
			)?)),
			PeopleRuntimeType::Pezkuwichain => Ok(Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/people-pezkuwichain.json")[..],
			)?)),
			PeopleRuntimeType::PezkuwichainGenesis => Ok(Box::new(pezkuwichain::genesis_config())),
			PeopleRuntimeType::PezkuwichainLocal => Ok(Box::new(pezkuwichain::local_config(
				pezkuwichain::PEOPLE_PEZKUWICHAIN_LOCAL,
				"Pezkuwichain People Local",
				"pezkuwichain-local",
				ChainType::Local,
			))),
			PeopleRuntimeType::PezkuwichainDevelopment => Ok(Box::new(pezkuwichain::local_config(
				pezkuwichain::PEOPLE_PEZKUWICHAIN_DEVELOPMENT,
				"Pezkuwichain People Development",
				"pezkuwichain-development",
				ChainType::Development,
			))),
			PeopleRuntimeType::Zagros => Ok(Box::new(GenericChainSpec::from_json_bytes(
				&include_bytes!("../../chain-specs/people-zagros.json")[..],
			)?)),
			PeopleRuntimeType::ZagrosLocal => Ok(Box::new(zagros::local_config(
				zagros::PEOPLE_ZAGROS_LOCAL,
				"Zagros People Local",
				"zagros-local",
				ChainType::Local,
			))),
			PeopleRuntimeType::ZagrosDevelopment => Ok(Box::new(zagros::local_config(
				zagros::PEOPLE_ZAGROS_DEVELOPMENT,
				"Zagros People Development",
				"zagros-development",
				ChainType::Development,
			))),
			other => Err(std::format!(
				"No default config present for {:?}, you should provide a chain-spec as json file!",
				other
			)),
		}
	}
}

/// Check if `id` satisfies People-like format.
fn ensure_id(id: &str) -> Result<&str, String> {
	if id.starts_with(PeopleRuntimeType::ID_PREFIX) {
		Ok(id)
	} else {
		Err(format!(
			"Invalid 'id' attribute ({}), should start with prefix: {}",
			id,
			PeopleRuntimeType::ID_PREFIX
		))
	}
}

/// Sub-module for Pezkuwichain setup.
pub mod pezkuwichain {
	use pezkuwi_omni_node_lib::chain_spec::{Extensions, GenericChainSpec};
	use pezsc_chain_spec::ChainType;

	pub(crate) const PEOPLE_PEZKUWICHAIN: &str = "people-pezkuwichain";
	pub(crate) const PEOPLE_PEZKUWICHAIN_GENESIS: &str = "people-pezkuwichain-genesis";
	pub(crate) const PEOPLE_PEZKUWICHAIN_LOCAL: &str = "people-pezkuwichain-local";
	pub(crate) const PEOPLE_PEZKUWICHAIN_DEVELOPMENT: &str = "people-pezkuwichain-dev";

	/// Genesis config for People Pezkuwichain mainnet
	pub fn genesis_config() -> GenericChainSpec {
		let mut properties = pezsc_chain_spec::Properties::new();
		properties.insert("ss58Format".into(), 42.into());
		properties.insert("tokenSymbol".into(), "HEZ".into());
		properties.insert("tokenDecimals".into(), 12.into());

		GenericChainSpec::builder(
			people_pezkuwichain_runtime::WASM_BINARY
				.expect("WASM binary was not built, please build it!"),
			Extensions::new("pezkuwichain-mainnet".to_string(), 1004),
		)
		.with_name("Pezkuwichain People")
		.with_id("people-pezkuwichain")
		.with_chain_type(ChainType::Live)
		.with_genesis_config_preset_name("genesis")
		.with_properties(properties)
		.build()
	}

	pub fn local_config(
		spec_id: &str,
		chain_name: &str,
		relay_chain: &str,
		chain_type: ChainType,
	) -> GenericChainSpec {
		let mut properties = pezsc_chain_spec::Properties::new();
		properties.insert("ss58Format".into(), 42.into());
		properties.insert("tokenSymbol".into(), "HEZ".into());
		properties.insert("tokenDecimals".into(), 12.into());

		GenericChainSpec::builder(
			people_pezkuwichain_runtime::WASM_BINARY
				.expect("WASM binary was not built, please build it!"),
			Extensions::new_with_relay_chain(relay_chain.to_string()),
		)
		.with_name(chain_name)
		.with_id(super::ensure_id(spec_id).expect("invalid id"))
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
	use pezkuwi_omni_node_lib::chain_spec::{Extensions, GenericChainSpec};
	use pezsc_chain_spec::ChainType;

	pub(crate) const PEOPLE_ZAGROS: &str = "people-zagros";
	pub(crate) const PEOPLE_ZAGROS_LOCAL: &str = "people-zagros-local";
	pub(crate) const PEOPLE_ZAGROS_DEVELOPMENT: &str = "people-zagros-dev";

	pub fn local_config(
		spec_id: &str,
		chain_name: &str,
		relay_chain: &str,
		chain_type: ChainType,
	) -> GenericChainSpec {
		let mut properties = pezsc_chain_spec::Properties::new();
		properties.insert("ss58Format".into(), 42.into());
		properties.insert("tokenSymbol".into(), "ZGR".into());
		properties.insert("tokenDecimals".into(), 12.into());

		GenericChainSpec::builder(
			people_zagros_runtime::WASM_BINARY
				.expect("WASM binary was not built, please build it!"),
			Extensions::new_with_relay_chain(relay_chain.to_string()),
		)
		.with_name(chain_name)
		.with_id(super::ensure_id(spec_id).expect("invalid id"))
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
	pub(crate) const PEOPLE_DICLE: &str = "people-dicle";
	pub(crate) const PEOPLE_DICLE_LOCAL: &str = "people-dicle-local";
}

pub mod pezkuwi {
	pub(crate) const PEOPLE_PEZKUWI: &str = "people-pezkuwi";
	pub(crate) const PEOPLE_PEZKUWI_LOCAL: &str = "people-pezkuwi-local";
}
