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
use pezsp_keyring::Sr25519Keyring as Keyring;

// Pezkuwi
use pezkuwi_primitives::{AssignmentId, ValidatorId};

// Pezcumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, get_host_config, validators,
};
use pezkuwichain_runtime_constants::currency::UNITS as TYR;
use teyrchains_common::Balance;

pub const ED: Balance = pezkuwichain_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
const ENDOWMENT: u128 = 1_000_000 * TYR;

fn session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	para_validator: ValidatorId,
	para_assignment: AssignmentId,
	authority_discovery: AuthorityDiscoveryId,
	beefy: BeefyId,
) -> pezkuwichain_runtime::SessionKeys {
	pezkuwichain_runtime::SessionKeys {
		babe,
		grandpa,
		para_validator,
		para_assignment,
		authority_discovery,
		beefy,
	}
}

pub fn genesis() -> Storage {
	let genesis_config = pezkuwichain_runtime::RuntimeGenesisConfig {
		system: pezkuwichain_runtime::SystemConfig::default(),
		balances: pezkuwichain_runtime::BalancesConfig {
			balances: accounts::init_balances().iter().map(|k| (k.clone(), ENDOWMENT)).collect(),
			..Default::default()
		},
		session: pezkuwichain_runtime::SessionConfig {
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
		babe: pezkuwichain_runtime::BabeConfig {
			authorities: Default::default(),
			epoch_config: pezkuwichain_runtime::BABE_GENESIS_EPOCH_CONFIG,
			..Default::default()
		},
		sudo: pezkuwichain_runtime::SudoConfig { key: Some(Keyring::Alice.to_account_id()) },
		configuration: pezkuwichain_runtime::ConfigurationConfig { config: get_host_config() },
		registrar: pezkuwichain_runtime::RegistrarConfig {
			next_free_para_id: pezkuwi_primitives::LOWEST_PUBLIC_ID,
			..Default::default()
		},
		..Default::default()
	};

	build_genesis_storage(&genesis_config, pezkuwichain_runtime::WASM_BINARY.unwrap())
}
