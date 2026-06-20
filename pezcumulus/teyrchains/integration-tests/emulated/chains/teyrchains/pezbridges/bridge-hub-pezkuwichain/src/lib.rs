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

pub mod genesis;

pub use pezbridge_hub_pezkuwichain_runtime::{
	self as pezbridge_hub_pezkuwichain_runtime,
	xcm_config::XcmConfig as BridgeHubPezkuwichainXcmConfig, EthereumBeaconClient,
	EthereumInboundQueue, ExistentialDeposit as BridgeHubPezkuwichainExistentialDeposit,
	RuntimeOrigin as BridgeHubPezkuwichainRuntimeOrigin,
};

// Bizinikiwi
use pezframe_support::traits::OnInitialize;

// Pezcumulus
use emulated_integration_tests_common::{
	impl_accounts_helpers_for_teyrchain, impl_assert_events_helpers_for_teyrchain,
	impl_xcm_helpers_for_teyrchain, impls::Teyrchain, xcm_pez_emulator::decl_test_teyrchains,
	AuraDigestProvider,
};

// BridgeHubPezkuwichain Teyrchain declaration
decl_test_teyrchains! {
	pub struct BridgeHubPezkuwichain {
		genesis = genesis::genesis(),
		on_init = {
			pezbridge_hub_pezkuwichain_runtime::AuraExt::on_initialize(1);
		},
		runtime = pezbridge_hub_pezkuwichain_runtime,
		core = {
			XcmpMessageHandler: pezbridge_hub_pezkuwichain_runtime::XcmpQueue,
			LocationToAccountId: pezbridge_hub_pezkuwichain_runtime::xcm_config::LocationToAccountId,
			TeyrchainInfo: pezbridge_hub_pezkuwichain_runtime::TeyrchainInfo,
			MessageOrigin: pezbridge_hub_common::AggregateMessageOrigin,
			DigestProvider: AuraDigestProvider,
		},
		pallets = {
			PezkuwiXcm: pezbridge_hub_pezkuwichain_runtime::PezkuwiXcm,
			Balances: pezbridge_hub_pezkuwichain_runtime::Balances,
			EthereumSystem: pezbridge_hub_pezkuwichain_runtime::EthereumSystem,
			EthereumInboundQueue: pezbridge_hub_pezkuwichain_runtime::EthereumInboundQueue,
			EthereumOutboundQueue: pezbridge_hub_pezkuwichain_runtime::EthereumOutboundQueue,
		}
	},
}

// BridgeHubPezkuwichain implementation
impl_accounts_helpers_for_teyrchain!(BridgeHubPezkuwichain);
impl_assert_events_helpers_for_teyrchain!(BridgeHubPezkuwichain);
impl_xcm_helpers_for_teyrchain!(BridgeHubPezkuwichain);
