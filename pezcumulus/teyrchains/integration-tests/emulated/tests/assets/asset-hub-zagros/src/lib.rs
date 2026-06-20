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
		BoundedVec,
	};

	// Pezkuwi
	pub(crate) use xcm::{
		latest::{AssetTransferFilter, PEZKUWICHAIN_GENESIS_HASH, ZAGROS_GENESIS_HASH},
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
			fee_asset, find_mq_processed_id, find_xcm_sent_message_id,
			get_amount_from_versioned_assets, non_fee_asset, xcm_transact_paid_execution,
		},
		xcm_pez_emulator::{
			assert_expected_events, bx, Chain, RelayChain as Relay, Test, TestArgs, TestContext,
			TestExt, Teyrchain as Para,
		},
		xcm_pez_simulator::helpers::TopicIdTracker,
		PenpalATeleportableAssetLocation, ASSETS_PALLET_ID, RESERVABLE_ASSET_ID, USDT_ID, XCM_V3,
	};
	pub(crate) use teyrchains_common::{AccountId, Balance};
	pub(crate) use zagros_system_emulated_network::{
		asset_hub_zagros_emulated_chain::{
			asset_hub_zagros_runtime::{
				self,
				governance::TreasuryAccount,
				xcm_config::{
					self as ahw_xcm_config, XcmConfig as AssetHubZagrosXcmConfig,
					ZagrosLocation as RelayLocation,
				},
				AssetConversionOrigin as AssetHubZagrosAssetConversionOrigin,
				ExistentialDeposit as AssetHubZagrosExistentialDeposit, ForeignAssetReserveData,
			},
			genesis::{AssetHubZagrosAssetOwner, ED as ASSET_HUB_ZAGROS_ED},
			AssetHubZagrosParaPezpallet, AssetHubZagrosParaPezpallet as AssetHubZagrosPallet,
		},
		collectives_zagros_emulated_chain::{
			CollectivesZagrosParaPezpallet,
			CollectivesZagrosParaPezpallet as CollectivesZagrosPallet,
		},
		coretime_zagros_emulated_chain::CoretimeZagrosParaPezpallet,
		people_zagros_emulated_chain::PeopleZagrosParaPezpallet,
		pez_penpal_emulated_chain::{
			pez_penpal_runtime::xcm_config::{
				CustomizableAssetFromSystemAssetHub as PenpalCustomizableAssetFromSystemAssetHub,
				LocalReservableFromAssetHub as PenpalLocalReservableFromAssetHub,
				LocalTeleportableToAssetHub as PenpalLocalTeleportableToAssetHub,
				UniversalLocation as PenpalUniversalLocation,
				UsdtFromAssetHub as PenpalUsdtFromAssetHub,
			},
			PenpalAParaPezpallet, PenpalAParaPezpallet as PenpalAPallet, PenpalAssetOwner,
			PenpalBParaPezpallet, PenpalBParaPezpallet as PenpalBPallet,
		},
		pezbridge_hub_zagros_emulated_chain::{
			pezbridge_hub_zagros_runtime::xcm_config::{self as bhw_xcm_config},
			BridgeHubZagrosParaPezpallet,
		},
		zagros_emulated_chain::{
			genesis::ED as ZAGROS_ED,
			zagros_runtime::{
				governance::pezpallet_custom_origins::Origin::Treasurer,
				xcm_config::{
					UniversalLocation as ZagrosUniversalLocation, XcmConfig as ZagrosXcmConfig,
				},
				Dmp,
			},
			ZagrosRelayPezpallet, ZagrosRelayPezpallet as ZagrosPallet,
		},
		AssetHubZagrosPara as AssetHubZagros, AssetHubZagrosParaReceiver as AssetHubZagrosReceiver,
		AssetHubZagrosParaSender as AssetHubZagrosSender, BridgeHubZagrosPara as BridgeHubZagros,
		BridgeHubZagrosParaReceiver as BridgeHubZagrosReceiver,
		CollectivesZagrosPara as CollectivesZagros, CoretimeZagrosPara as CoretimeZagros,
		PenpalAPara as PenpalA, PenpalAParaReceiver as PenpalAReceiver,
		PenpalAParaSender as PenpalASender, PenpalBPara as PenpalB,
		PenpalBParaReceiver as PenpalBReceiver, PeopleZagrosPara as PeopleZagros,
		ZagrosRelay as Zagros, ZagrosRelayReceiver as ZagrosReceiver,
		ZagrosRelaySender as ZagrosSender,
	};

	pub(crate) const ASSET_ID: u32 = 3;
	pub(crate) const ASSET_MIN_BALANCE: u128 = 1000;

	pub(crate) type RelayToParaTest = Test<Zagros, PenpalA>;
	pub(crate) type ParaToRelayTest = Test<PenpalA, Zagros>;
	pub(crate) type RelayToSystemParaTest = Test<Zagros, AssetHubZagros>;
	pub(crate) type SystemParaToRelayTest = Test<AssetHubZagros, Zagros>;
	pub(crate) type SystemParaToParaTest = Test<AssetHubZagros, PenpalA>;
	pub(crate) type ParaToSystemParaTest = Test<PenpalA, AssetHubZagros>;
	pub(crate) type ParaToParaThroughRelayTest = Test<PenpalA, PenpalB, Zagros>;
	pub(crate) type ParaToParaThroughAHTest = Test<PenpalA, PenpalB, AssetHubZagros>;
	pub(crate) type RelayToParaThroughAHTest = Test<Zagros, PenpalA, AssetHubZagros>;
	pub(crate) type PenpalToRelayThroughAHTest = Test<PenpalA, Zagros, AssetHubZagros>;
}

#[cfg(test)]
mod tests;
