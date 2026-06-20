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

#[cfg(test)]
mod imports {
	// Bizinikiwi
	pub(crate) use codec::Encode;
	pub(crate) use pezframe_support::{assert_err, assert_ok, pezpallet_prelude::DispatchResult};
	pub(crate) use pezsp_runtime::DispatchError;

	// Pezkuwi
	pub(crate) use xcm::{
		latest::{ParentThen, PEZKUWICHAIN_GENESIS_HASH, ZAGROS_GENESIS_HASH},
		prelude::{AccountId32 as AccountId32Junction, *},
	};
	pub(crate) use xcm_builder::ExternalConsensusLocationsConverterFor;
	pub(crate) use xcm_executor::traits::TransferType;

	// Pezcumulus
	pub(crate) use emulated_integration_tests_common::{
		accounts::ALICE,
		impls::Inspect,
		test_dry_run_transfer_across_pk_bridge, test_relay_is_trusted_teleporter,
		test_teyrchain_is_trusted_teleporter, test_teyrchain_is_trusted_teleporter_for_relay,
		xcm_helpers::xcm_transact_paid_execution,
		xcm_pez_emulator::{
			assert_expected_events, bx, Chain, RelayChain as Relay, TestExt, Teyrchain as Para,
		},
		ASSETS_PALLET_ID, USDT_ID,
	};
	pub(crate) use pezkuwichain_zagros_system_emulated_network::{
		asset_hub_pezkuwichain_emulated_chain::{
			asset_hub_pezkuwichain_runtime::{
				xcm_config::TreasuryAccount, ForeignAssetReserveData,
			},
			genesis::ED as ASSET_HUB_PEZKUWICHAIN_ED,
			AssetHubPezkuwichainParaPezpallet,
			AssetHubPezkuwichainParaPezpallet as AssetHubPezkuwichainPallet,
		},
		asset_hub_zagros_emulated_chain::{
			genesis::{AssetHubZagrosAssetOwner, ED as ASSET_HUB_ZAGROS_ED},
			AssetHubZagrosParaPezpallet as AssetHubZagrosPallet,
		},
		pez_penpal_emulated_chain::{
			pez_penpal_runtime::xcm_config::{
				CustomizableAssetFromSystemAssetHub as PenpalCustomizableAssetFromSystemAssetHub,
				UniversalLocation as PenpalUniversalLocation,
			},
			PenpalAParaPezpallet as PenpalAPallet, PenpalAssetOwner,
		},
		pezbridge_hub_pezkuwichain_emulated_chain::{
			genesis::ED as BRIDGE_HUB_PEZKUWICHAIN_ED, BridgeHubPezkuwichainExistentialDeposit,
			BridgeHubPezkuwichainParaPezpallet,
		},
		pezkuwichain_emulated_chain::{
			genesis::ED as PEZKUWICHAIN_ED, PezkuwichainRelayPezpallet,
			PezkuwichainRelayPezpallet as PezkuwichainPallet,
		},
		AssetHubPezkuwichainPara as AssetHubPezkuwichain,
		AssetHubPezkuwichainParaReceiver as AssetHubPezkuwichainReceiver,
		AssetHubPezkuwichainParaSender as AssetHubPezkuwichainSender,
		AssetHubZagrosPara as AssetHubZagros, AssetHubZagrosParaReceiver as AssetHubZagrosReceiver,
		AssetHubZagrosParaSender as AssetHubZagrosSender,
		BridgeHubPezkuwichainPara as BridgeHubPezkuwichain,
		BridgeHubPezkuwichainParaReceiver as BridgeHubPezkuwichainReceiver,
		BridgeHubPezkuwichainParaSender as BridgeHubPezkuwichainSender,
		BridgeHubZagrosPara as BridgeHubZagros, PenpalAPara as PenpalA,
		PenpalAParaSender as PenpalASender, PezkuwichainRelay as Pezkuwichain,
		PezkuwichainRelayReceiver as PezkuwichainReceiver,
		PezkuwichainRelaySender as PezkuwichainSender,
	};
	pub(crate) use teyrchains_common::AccountId;

	pub(crate) const ASSET_ID: u32 = 1;
	pub(crate) const ASSET_MIN_BALANCE: u128 = 1000;
}

#[cfg(test)]
mod tests;
