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
	pub(crate) use xcm::{latest::ZAGROS_GENESIS_HASH, prelude::*};

	pub(crate) use pezframe_support::assert_ok;

	pub(crate) use emulated_integration_tests_common::{
		accounts::ALICE,
		test_teyrchain_is_trusted_teleporter,
		xcm_pez_emulator::{assert_expected_events, bx, Chain, TestExt, Teyrchain},
	};
	pub(crate) use zagros_system_emulated_network::{
		asset_hub_zagros_emulated_chain::{
			asset_hub_zagros_runtime::xcm_config::LocationToAccountId as AssetHubLocationToAccountId,
			genesis::ED as ASSET_HUB_ZAGROS_ED, AssetHubZagrosParaPezpallet,
			AssetHubZagrosParaPezpallet as AssetHubZagrosPallet,
		},
		collectives_zagros_emulated_chain::{
			collectives_zagros_runtime::{
				fellowship as collectives_fellowship,
				xcm_config::XcmConfig as CollectivesZagrosXcmConfig,
			},
			genesis::ED as COLLECTIVES_ZAGROS_ED,
			CollectivesZagrosParaPezpallet,
			CollectivesZagrosParaPezpallet as CollectivesZagrosPallet,
		},
		coretime_zagros_emulated_chain::CoretimeZagrosParaPezpallet,
		people_zagros_emulated_chain::PeopleZagrosParaPezpallet,
		pez_penpal_emulated_chain::{PenpalAssetOwner, PenpalBParaPezpallet},
		pezbridge_hub_zagros_emulated_chain::BridgeHubZagrosParaPezpallet,
		zagros_emulated_chain::{
			genesis::ED as ZAGROS_ED,
			zagros_runtime::{governance as zagros_governance, OriginCaller as ZagrosOriginCaller},
			ZagrosRelayPezpallet, ZagrosRelayPezpallet as ZagrosPallet,
		},
		AssetHubZagrosPara as AssetHubZagros, AssetHubZagrosParaReceiver as AssetHubZagrosReceiver,
		AssetHubZagrosParaSender as AssetHubZagrosSender, BridgeHubZagrosPara as BridgeHubZagros,
		CollectivesZagrosPara as CollectivesZagros,
		CollectivesZagrosParaReceiver as CollectivesZagrosReceiver,
		CollectivesZagrosParaSender as CollectivesZagrosSender,
		CoretimeZagrosPara as CoretimeZagros, PenpalBPara as PenpalB,
		PeopleZagrosPara as PeopleZagros, ZagrosRelay as Zagros,
		ZagrosRelayReceiver as ZagrosReceiver, ZagrosRelaySender as ZagrosSender,
	};
}

#[cfg(test)]
mod tests;
