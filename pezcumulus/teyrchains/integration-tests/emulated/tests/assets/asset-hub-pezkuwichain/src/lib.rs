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
	pub(crate) use codec::Encode;

	// Bizinikiwi
	pub(crate) use pezframe_support::{
		assert_err, assert_ok,
		pezpallet_prelude::Weight,
		pezsp_runtime::{DispatchError, DispatchResult, ModuleError},
		traits::fungibles::Inspect,
	};

	// Pezkuwi
	pub(crate) use xcm::{
		latest::{PEZKUWICHAIN_GENESIS_HASH, ZAGROS_GENESIS_HASH},
		prelude::{AccountId32 as AccountId32Junction, *},
	};
	pub(crate) use xcm_executor::traits::TransferType;

	// Pezcumulus
	pub(crate) use asset_test_pezutils::xcm_helpers;
	pub(crate) use emulated_integration_tests_common::{
		accounts::DUMMY_EMPTY,
		test_relay_is_trusted_teleporter, test_teyrchain_is_trusted_teleporter,
		test_teyrchain_is_trusted_teleporter_for_relay,
		test_xcm_fee_querying_apis_work_for_asset_hub,
		xcm_helpers::{
			fee_asset, get_amount_from_versioned_assets, non_fee_asset, xcm_transact_paid_execution,
		},
		xcm_pez_emulator::{
			assert_expected_events, bx, Chain, RelayChain as Relay, Test, TestArgs, TestContext,
			TestExt, Teyrchain as Para,
		},
		PenpalATeleportableAssetLocation, ASSETS_PALLET_ID, RESERVABLE_ASSET_ID, XCM_V3,
	};
	pub(crate) use pezkuwichain_system_emulated_network::{
		asset_hub_pezkuwichain_emulated_chain::{
			asset_hub_pezkuwichain_runtime::{
				self,
				xcm_config::{
					self as ahr_xcm_config, TokenLocation as RelayLocation, TreasuryAccount,
					XcmConfig as AssetHubPezkuwichainXcmConfig,
				},
				AssetConversionOrigin as AssetHubPezkuwichainAssetConversionOrigin,
				ExistentialDeposit as AssetHubPezkuwichainExistentialDeposit,
			},
			genesis::{AssetHubPezkuwichainAssetOwner, ED as ASSET_HUB_PEZKUWICHAIN_ED},
			AssetHubPezkuwichainParaPezpallet,
			AssetHubPezkuwichainParaPezpallet as AssetHubPezkuwichainPallet,
		},
		pez_penpal_emulated_chain::{
			pez_penpal_runtime::xcm_config::{
				CustomizableAssetFromSystemAssetHub as PenpalCustomizableAssetFromSystemAssetHub,
				LocalReservableFromAssetHub as PenpalLocalReservableFromAssetHub,
				LocalTeleportableToAssetHub as PenpalLocalTeleportableToAssetHub,
				UsdtFromAssetHub as PenpalUsdtFromAssetHub,
			},
			PenpalAParaPezpallet, PenpalAParaPezpallet as PenpalAPallet, PenpalAssetOwner,
			PenpalBParaPezpallet, PenpalBParaPezpallet as PenpalBPallet, ED as PENPAL_ED,
		},
		pezkuwichain_emulated_chain::{
			genesis::ED as PEZKUWICHAIN_ED,
			pezkuwichain_runtime::{
				governance as pezkuwichain_governance,
				governance::pezpallet_custom_origins::Origin::Treasurer,
				xcm_config::UniversalLocation as PezkuwichainUniversalLocation, Dmp,
				OriginCaller as PezkuwichainOriginCaller,
			},
			PezkuwichainRelayPezpallet, PezkuwichainRelayPezpallet as PezkuwichainPallet,
		},
		AssetHubPezkuwichainPara as AssetHubPezkuwichain,
		AssetHubPezkuwichainParaReceiver as AssetHubPezkuwichainReceiver,
		AssetHubPezkuwichainParaSender as AssetHubPezkuwichainSender,
		BridgeHubPezkuwichainPara as BridgeHubPezkuwichain,
		BridgeHubPezkuwichainParaReceiver as BridgeHubPezkuwichainReceiver, PenpalAPara as PenpalA,
		PenpalAParaReceiver as PenpalAReceiver, PenpalAParaSender as PenpalASender,
		PenpalBPara as PenpalB, PenpalBParaReceiver as PenpalBReceiver,
		PezkuwichainRelay as Pezkuwichain, PezkuwichainRelayReceiver as PezkuwichainReceiver,
		PezkuwichainRelaySender as PezkuwichainSender,
	};
	pub(crate) use teyrchains_common::Balance;

	pub(crate) const ASSET_ID: u32 = 3;
	pub(crate) const ASSET_MIN_BALANCE: u128 = 1000;

	pub(crate) type RelayToParaTest = Test<Pezkuwichain, PenpalA>;
	pub(crate) type ParaToRelayTest = Test<PenpalA, Pezkuwichain>;
	pub(crate) type SystemParaToRelayTest = Test<AssetHubPezkuwichain, Pezkuwichain>;
	pub(crate) type SystemParaToParaTest = Test<AssetHubPezkuwichain, PenpalA>;
	pub(crate) type ParaToSystemParaTest = Test<PenpalA, AssetHubPezkuwichain>;
	pub(crate) type ParaToParaThroughRelayTest = Test<PenpalA, PenpalB, Pezkuwichain>;
	pub(crate) type ParaToParaThroughAHTest = Test<PenpalA, PenpalB, AssetHubPezkuwichain>;
	pub(crate) type RelayToParaThroughAHTest = Test<Pezkuwichain, PenpalA, AssetHubPezkuwichain>;
}

#[cfg(test)]
mod tests;
