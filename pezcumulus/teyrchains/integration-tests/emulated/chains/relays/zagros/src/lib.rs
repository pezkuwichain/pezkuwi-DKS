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
pub use zagros_runtime;

pub mod genesis;

// Pezcumulus
use emulated_integration_tests_common::{
	impl_accounts_helpers_for_relay_chain, impl_assert_events_helpers_for_relay_chain,
	impl_hrmp_channels_helpers_for_relay_chain, impl_send_transact_helpers_for_relay_chain,
	xcm_pez_emulator::decl_test_relay_chains,
};

// Zagros declaration
decl_test_relay_chains! {
	#[api_version(15)]
	pub struct Zagros {
		genesis = genesis::genesis(),
		on_init = (),
		runtime = zagros_runtime,
		core = {
			SovereignAccountOf: zagros_runtime::xcm_config::LocationConverter,
		},
		pallets = {
			XcmPallet: zagros_runtime::XcmPallet,
			Sudo: zagros_runtime::Sudo,
			Balances: zagros_runtime::Balances,
			Treasury: zagros_runtime::Treasury,
			AssetRate: zagros_runtime::AssetRate,
			Hrmp: zagros_runtime::Hrmp,
			Identity: zagros_runtime::Identity,
			IdentityMigrator: zagros_runtime::IdentityMigrator,
		}
	},
}

// Zagros implementation
impl_accounts_helpers_for_relay_chain!(Zagros);
impl_assert_events_helpers_for_relay_chain!(Zagros);
impl_hrmp_channels_helpers_for_relay_chain!(Zagros);
impl_send_transact_helpers_for_relay_chain!(Zagros);
