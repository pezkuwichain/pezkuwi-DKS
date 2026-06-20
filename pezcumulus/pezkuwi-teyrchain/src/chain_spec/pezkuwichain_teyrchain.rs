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

//! ChainSpecs dedicated to Pezkuwichain teyrchain setups (for testing and example purposes)

use hex_literal::hex;
use pezcumulus_primitives_core::ParaId;
use pezkuwi_omni_node_lib::chain_spec::{Extensions, GenericChainSpec};
use pezkuwichain_teyrchain_runtime::AuraId;
use pezsc_chain_spec::ChainType;
use pezsp_core::crypto::UncheckedInto;
use teyrchains_common::AccountId;

pub fn pezkuwichain_teyrchain_local_config() -> GenericChainSpec {
	GenericChainSpec::builder(
		pezkuwichain_teyrchain_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain("pezkuwichain-local".into()),
	)
	.with_name("Pezkuwichain Teyrchain Local")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.build()
}

pub fn pezstaging_pezkuwichain_teyrchain_local_config() -> GenericChainSpec {
	GenericChainSpec::builder(
		pezkuwichain_teyrchain_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions::new_with_relay_chain("pezkuwichain-local".into()),
	)
	.with_name("Staging Pezkuwichain Teyrchain Local")
	.with_id("pezstaging_testnet")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_genesis_config_patch(testnet_genesis_patch(
		hex!["9ed7705e3c7da027ba0583a22a3212042f7e715d3c168ba14f1424e2bc111d00"].into(),
		vec![
			// $secret//one
			hex!["aad9fa2249f87a210a0f93400b7f90e47b810c6d65caa0ca3f5af982904c2a33"]
				.unchecked_into(),
			// $secret//two
			hex!["d47753f0cca9dd8da00c70e82ec4fc5501a69c49a5952a643d18802837c88212"]
				.unchecked_into(),
		],
		vec![hex!["9ed7705e3c7da027ba0583a22a3212042f7e715d3c168ba14f1424e2bc111d00"].into()],
		1000.into(),
	))
	.build()
}

pub(crate) fn testnet_genesis_patch(
	root_key: AccountId,
	initial_authorities: Vec<AuraId>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 1u64 << 60)).collect::<Vec<_>>(),
		},
		"sudo": { "key": Some(root_key) },
		"teyrchainInfo": {
			"teyrchainId": id,
		},
		"aura": { "authorities": initial_authorities },
	})
}
