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

pub use pez_penpal_runtime::{
	self, xcm_config::RelayNetworkId as PenpalRelayNetworkId, ForeignAssetReserveData,
};

mod genesis;
pub use genesis::{genesis, PenpalAssetOwner, PenpalSudoAccount, ED, PARA_ID_A, PARA_ID_B};

// Bizinikiwi
use pezframe_support::traits::OnInitialize;
use pezsp_core::Encode;

// Pezcumulus
use emulated_integration_tests_common::{
	impl_accounts_helpers_for_teyrchain, impl_assert_events_helpers_for_teyrchain,
	impl_assets_helpers_for_teyrchain, impl_foreign_assets_helpers_for_teyrchain,
	impl_xcm_helpers_for_teyrchain,
	impls::{NetworkId, Teyrchain},
	xcm_pez_emulator::decl_test_teyrchains,
	AuraDigestProvider,
};

// Pezkuwi
use xcm::latest::{PEZKUWICHAIN_GENESIS_HASH, ZAGROS_GENESIS_HASH};

// Penpal Teyrchain declaration
decl_test_teyrchains! {
	pub struct PenpalA {
		genesis = genesis(PARA_ID_A),
		on_init = {
			pez_penpal_runtime::AuraExt::on_initialize(1);
			pezframe_support::assert_ok!(pez_penpal_runtime::System::set_storage(
				pez_penpal_runtime::RuntimeOrigin::root(),
				vec![(PenpalRelayNetworkId::key().to_vec(), NetworkId::ByGenesis(PEZKUWICHAIN_GENESIS_HASH).encode())],
			));
		},
		runtime = pez_penpal_runtime,
		core = {
			XcmpMessageHandler: pez_penpal_runtime::XcmpQueue,
			LocationToAccountId: pez_penpal_runtime::xcm_config::LocationToAccountId,
			TeyrchainInfo: pez_penpal_runtime::TeyrchainInfo,
			MessageOrigin: pezcumulus_primitives_core::AggregateMessageOrigin,
			DigestProvider: AuraDigestProvider,
		},
		pallets = {
			PezkuwiXcm: pez_penpal_runtime::PezkuwiXcm,
			Assets: pez_penpal_runtime::Assets,
			ForeignAssets: pez_penpal_runtime::ForeignAssets,
			AssetConversion: pez_penpal_runtime::AssetConversion,
			Balances: pez_penpal_runtime::Balances,
		}
	},
	pub struct PenpalB {
		genesis = genesis(PARA_ID_B),
		on_init = {
			pez_penpal_runtime::AuraExt::on_initialize(1);
			pezframe_support::assert_ok!(pez_penpal_runtime::System::set_storage(
				pez_penpal_runtime::RuntimeOrigin::root(),
				vec![(PenpalRelayNetworkId::key().to_vec(), NetworkId::ByGenesis(ZAGROS_GENESIS_HASH).encode())],
			));
		},
		runtime = pez_penpal_runtime,
		core = {
			XcmpMessageHandler: pez_penpal_runtime::XcmpQueue,
			LocationToAccountId: pez_penpal_runtime::xcm_config::LocationToAccountId,
			TeyrchainInfo: pez_penpal_runtime::TeyrchainInfo,
			MessageOrigin: pezcumulus_primitives_core::AggregateMessageOrigin,
			DigestProvider: AuraDigestProvider,
		},
		pallets = {
			PezkuwiXcm: pez_penpal_runtime::PezkuwiXcm,
			Assets: pez_penpal_runtime::Assets,
			ForeignAssets: pez_penpal_runtime::ForeignAssets,
			AssetConversion: pez_penpal_runtime::AssetConversion,
			Balances: pez_penpal_runtime::Balances,
		}
	},
}

// Penpal implementation
impl_accounts_helpers_for_teyrchain!(PenpalA);
impl_accounts_helpers_for_teyrchain!(PenpalB);
impl_assert_events_helpers_for_teyrchain!(PenpalA);
impl_assert_events_helpers_for_teyrchain!(PenpalB);
impl_assets_helpers_for_teyrchain!(PenpalA);
impl_foreign_assets_helpers_for_teyrchain!(PenpalA, xcm::latest::Location, ForeignAssetReserveData);
impl_assets_helpers_for_teyrchain!(PenpalB);
impl_foreign_assets_helpers_for_teyrchain!(PenpalB, xcm::latest::Location, ForeignAssetReserveData);
impl_xcm_helpers_for_teyrchain!(PenpalA);
impl_xcm_helpers_for_teyrchain!(PenpalB);
