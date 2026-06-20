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

pub use asset_hub_pezkuwichain_runtime;

pub mod genesis;

// Bizinikiwi
use pezframe_support::traits::OnInitialize;

// Pezcumulus
use asset_hub_pezkuwichain_runtime::ForeignAssetReserveData;
use emulated_integration_tests_common::{
	impl_accounts_helpers_for_teyrchain, impl_assert_events_helpers_for_teyrchain,
	impl_assets_helpers_for_system_teyrchain, impl_assets_helpers_for_teyrchain,
	impl_bridge_helpers_for_chain, impl_foreign_assets_helpers_for_teyrchain,
	impl_xcm_helpers_for_teyrchain, impls::Teyrchain, xcm_pez_emulator::decl_test_teyrchains,
	AuraDigestProvider,
};
use pezkuwichain_emulated_chain::Pezkuwichain;

// AssetHubPezkuwichain Teyrchain declaration
decl_test_teyrchains! {
	pub struct AssetHubPezkuwichain {
		genesis = genesis::genesis(),
		on_init = {
			asset_hub_pezkuwichain_runtime::AuraExt::on_initialize(1);
		},
		runtime = asset_hub_pezkuwichain_runtime,
		core = {
			XcmpMessageHandler: asset_hub_pezkuwichain_runtime::XcmpQueue,
			LocationToAccountId: asset_hub_pezkuwichain_runtime::xcm_config::LocationToAccountId,
			TeyrchainInfo: asset_hub_pezkuwichain_runtime::TeyrchainInfo,
			MessageOrigin: pezcumulus_primitives_core::AggregateMessageOrigin,
			DigestProvider: AuraDigestProvider,
			AdditionalInherentCode: (),
		},
		pallets = {
			PezkuwiXcm: asset_hub_pezkuwichain_runtime::PezkuwiXcm,
			Assets: asset_hub_pezkuwichain_runtime::Assets,
			ForeignAssets: asset_hub_pezkuwichain_runtime::ForeignAssets,
			PoolAssets: asset_hub_pezkuwichain_runtime::PoolAssets,
			AssetConversion: asset_hub_pezkuwichain_runtime::AssetConversion,
			Balances: asset_hub_pezkuwichain_runtime::Balances,
		}
	},
}

// AssetHubPezkuwichain implementation
impl_accounts_helpers_for_teyrchain!(AssetHubPezkuwichain);
impl_assert_events_helpers_for_teyrchain!(AssetHubPezkuwichain);
impl_assets_helpers_for_system_teyrchain!(AssetHubPezkuwichain, Pezkuwichain);
impl_assets_helpers_for_teyrchain!(AssetHubPezkuwichain);
impl_foreign_assets_helpers_for_teyrchain!(
	AssetHubPezkuwichain,
	xcm::v5::Location,
	ForeignAssetReserveData
);
impl_xcm_helpers_for_teyrchain!(AssetHubPezkuwichain);
impl_bridge_helpers_for_chain!(
	AssetHubPezkuwichain,
	ParaPezpallet,
	PezkuwiXcm,
	pezbp_bridge_hub_pezkuwichain::RuntimeCall::XcmOverBridgeHubZagros
);
