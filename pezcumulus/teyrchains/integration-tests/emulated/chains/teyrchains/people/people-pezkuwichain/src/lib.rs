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
pub use people_pezkuwichain_runtime;

pub mod genesis;

// Bizinikiwi
use pezframe_support::traits::OnInitialize;

// Pezcumulus
use emulated_integration_tests_common::{
	impl_accounts_helpers_for_teyrchain, impl_assert_events_helpers_for_teyrchain,
	impls::Teyrchain, xcm_pez_emulator::decl_test_teyrchains, AuraDigestProvider,
};

// PeoplePezkuwichain Teyrchain declaration
decl_test_teyrchains! {
	pub struct PeoplePezkuwichain {
		genesis = genesis::genesis(),
		on_init = {
			people_pezkuwichain_runtime::AuraExt::on_initialize(1);
		},
		runtime = people_pezkuwichain_runtime,
		core = {
			XcmpMessageHandler: people_pezkuwichain_runtime::XcmpQueue,
			LocationToAccountId: people_pezkuwichain_runtime::xcm_config::LocationToAccountId,
			TeyrchainInfo: people_pezkuwichain_runtime::TeyrchainInfo,
			MessageOrigin: pezcumulus_primitives_core::AggregateMessageOrigin,
			DigestProvider: AuraDigestProvider,
		},
		pallets = {
			PezkuwiXcm: people_pezkuwichain_runtime::PezkuwiXcm,
			Balances: people_pezkuwichain_runtime::Balances,
			Identity: people_pezkuwichain_runtime::Identity,
			IdentityMigrator: people_pezkuwichain_runtime::IdentityMigrator,
		}
	},
}

// PeoplePezkuwichain implementation
impl_accounts_helpers_for_teyrchain!(PeoplePezkuwichain);
impl_assert_events_helpers_for_teyrchain!(PeoplePezkuwichain);
