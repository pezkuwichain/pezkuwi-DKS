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

/// Collectives Zagros Development Config.
pub fn collectives_zagros_development_config() -> GenericChainSpec {
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 42.into());
	properties.insert("tokenSymbol".into(), "ZGR".into());
	properties.insert("tokenDecimals".into(), 12.into());

	GenericChainSpec::builder(
		collectives_zagros_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain("zagros-dev".into()),
	)
	.with_name("Zagros Collectives Development")
	.with_id("collectives_zagros_dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_preset_name(pezsp_genesis_builder::DEV_RUNTIME_PRESET)
	.with_boot_nodes(Vec::new())
	.with_properties(properties)
	.build()
}

/// Collectives Zagros Local Config.
pub fn collectives_zagros_local_config() -> GenericChainSpec {
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 42.into());
	properties.insert("tokenSymbol".into(), "ZGR".into());
	properties.insert("tokenDecimals".into(), 12.into());

	GenericChainSpec::builder(
		collectives_zagros_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain("zagros-local".into()),
	)
	.with_name("Zagros Collectives Local")
	.with_id("collectives_zagros_local")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_boot_nodes(Vec::new())
	.with_properties(properties)
	.build()
}
