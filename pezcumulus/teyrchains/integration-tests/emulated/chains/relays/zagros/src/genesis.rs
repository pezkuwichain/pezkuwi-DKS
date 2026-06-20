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

// Bizinikiwi
use pezsc_consensus_grandpa::AuthorityId as GrandpaId;
use pezsp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use pezsp_consensus_babe::AuthorityId as BabeId;
use pezsp_consensus_beefy::ecdsa_crypto::AuthorityId as BeefyId;
use pezsp_core::storage::Storage;

// Pezkuwi
use pezkuwi_primitives::{AssignmentId, ValidatorId};

// Pezcumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, get_host_config, validators,
};
use teyrchains_common::Balance;
use zagros_runtime_constants::currency::UNITS as ZGR;

pub const ED: Balance = zagros_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
const ENDOWMENT: u128 = 1_000_000 * ZGR;

fn session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	para_validator: ValidatorId,
	para_assignment: AssignmentId,
	authority_discovery: AuthorityDiscoveryId,
	beefy: BeefyId,
) -> zagros_runtime::SessionKeys {
	zagros_runtime::SessionKeys {
		babe,
		grandpa,
		para_validator,
		para_assignment,
		authority_discovery,
		beefy,
	}
}

pub fn genesis() -> Storage {
	let genesis_config = zagros_runtime::RuntimeGenesisConfig {
		system: zagros_runtime::SystemConfig::default(),
		balances: zagros_runtime::BalancesConfig {
			balances: accounts::init_balances().iter().cloned().map(|k| (k, ENDOWMENT)).collect(),
			..Default::default()
		},
		session: zagros_runtime::SessionConfig {
			keys: validators::initial_authorities()
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
							x.6.clone(),
							x.7.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
			..Default::default()
		},
		babe: zagros_runtime::BabeConfig {
			authorities: Default::default(),
			epoch_config: zagros_runtime::BABE_GENESIS_EPOCH_CONFIG,
			..Default::default()
		},
		configuration: zagros_runtime::ConfigurationConfig { config: get_host_config() },
		..Default::default()
	};

	build_genesis_storage(&genesis_config, zagros_runtime::WASM_BINARY.unwrap())
}
