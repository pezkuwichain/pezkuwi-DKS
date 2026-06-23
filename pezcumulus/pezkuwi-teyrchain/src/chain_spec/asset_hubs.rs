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

use pezkuwi_omni_node_lib::chain_spec::{Extensions, GenericChainSpec};
use pezsc_service::ChainType;

pub fn asset_hub_zagros_development_config() -> GenericChainSpec {
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "ZGR".into());
	properties.insert("tokenDecimals".into(), 12.into());

	GenericChainSpec::builder(
		asset_hub_zagros_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain("zagros".into()),
	)
	.with_name("Zagros Asset Hub Development")
	.with_id("asset-hub-zagros-dev")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::DEV_RUNTIME_PRESET)
	.with_properties(properties)
	.build()
}

pub fn asset_hub_zagros_local_config() -> GenericChainSpec {
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "ZGR".into());
	properties.insert("tokenDecimals".into(), 12.into());

	GenericChainSpec::builder(
		asset_hub_zagros_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain("zagros-local".into()),
	)
	.with_name("Zagros Asset Hub Local")
	.with_id("asset-hub-zagros-local")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_properties(properties)
	.build()
}

pub fn asset_hub_zagros_config() -> GenericChainSpec {
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "ZGR".into());
	properties.insert("tokenDecimals".into(), 12.into());

	GenericChainSpec::builder(
		asset_hub_zagros_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain("zagros".into()),
	)
	.with_name("Zagros Asset Hub")
	.with_id("asset-hub-zagros")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_preset_name("genesis")
	.with_properties(properties)
	.build()
}

pub fn asset_hub_pezkuwichain_development_config() -> GenericChainSpec {
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 42.into());
	properties.insert("tokenSymbol".into(), "HEZ".into());
	properties.insert("tokenDecimals".into(), 12.into());
	asset_hub_pezkuwichain_like_development_config(
		properties,
		"Pezkuwichain Asset Hub Development",
		"asset-hub-pezkuwichain-dev",
	)
}

fn asset_hub_pezkuwichain_like_development_config(
	properties: pezsc_chain_spec::Properties,
	name: &str,
	chain_id: &str,
) -> GenericChainSpec {
	GenericChainSpec::builder(
		asset_hub_pezkuwichain_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain("pezkuwichain-dev".into()),
	)
	.with_name(name)
	.with_id(chain_id)
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::DEV_RUNTIME_PRESET)
	.with_properties(properties)
	.build()
}

pub fn asset_hub_pezkuwichain_local_config() -> GenericChainSpec {
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 42.into());
	properties.insert("tokenSymbol".into(), "HEZ".into());
	properties.insert("tokenDecimals".into(), 12.into());
	asset_hub_pezkuwichain_like_local_config(
		properties,
		"Pezkuwichain Asset Hub Local",
		"asset-hub-pezkuwichain-local",
	)
}

fn asset_hub_pezkuwichain_like_local_config(
	properties: pezsc_chain_spec::Properties,
	name: &str,
	chain_id: &str,
) -> GenericChainSpec {
	GenericChainSpec::builder(
		asset_hub_pezkuwichain_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain("pezkuwichain-local".into()),
	)
	.with_name(name)
	.with_id(chain_id)
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_properties(properties)
	.build()
}

pub fn asset_hub_pezkuwichain_genesis_config() -> GenericChainSpec {
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "HEZ".into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties.insert("ss58Format".into(), 42.into());
	GenericChainSpec::builder(
		asset_hub_pezkuwichain_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions::new("pezkuwichain-mainnet".into(), 1000),
	)
	.with_name("Pezkuwichain Asset Hub")
	.with_id("asset-hub-pezkuwichain")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_preset_name("genesis")
	.with_properties(properties)
	.build()
}
