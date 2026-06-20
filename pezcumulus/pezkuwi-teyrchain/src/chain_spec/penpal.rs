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
use pezkuwi_omni_node_lib::chain_spec::{Extensions, GenericChainSpec};
use pezsc_service::ChainType;

pub fn get_penpal_chain_spec(id: ParaId, relay_chain: &str) -> GenericChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "UNIT".into());
	properties.insert("tokenDecimals".into(), 12u32.into());
	properties.insert("ss58Format".into(), 42u32.into());

	GenericChainSpec::builder(
		pez_penpal_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain(relay_chain.into()),
	)
	.with_name("Penpal Teyrchain")
	.with_id(&format!("penpal-{}", relay_chain.replace("-local", "")))
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_genesis_config_patch(serde_json::json!({
		"teyrchainInfo": {
			"teyrchainId": id,
		},
	}))
	.build()
}
