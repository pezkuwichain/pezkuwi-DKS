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
	pub(crate) use pezframe_support::assert_ok;

	// Pezkuwi
	pub(crate) use xcm::{latest::ZAGROS_GENESIS_HASH, prelude::*};

	// Pezcumulus
	pub(crate) use emulated_integration_tests_common::xcm_pez_emulator::{
		assert_expected_events, Chain, TestExt, Teyrchain,
	};
	pub(crate) use zagros_system_emulated_network::{
		asset_hub_zagros_emulated_chain::{
			genesis::ED as ASSET_HUB_ZAGROS_ED, AssetHubZagrosParaPezpallet,
		},
		collectives_zagros_emulated_chain::CollectivesZagrosParaPezpallet,
		coretime_zagros_emulated_chain::{
			self, coretime_zagros_runtime::ExistentialDeposit as CoretimeZagrosExistentialDeposit,
			genesis::ED as CORETIME_ZAGROS_ED, CoretimeZagrosParaPezpallet,
			CoretimeZagrosParaPezpallet as CoretimeZagrosPallet,
		},
		people_zagros_emulated_chain::PeopleZagrosParaPezpallet,
		pez_penpal_emulated_chain::{PenpalAssetOwner, PenpalBParaPezpallet},
		pezbridge_hub_zagros_emulated_chain::BridgeHubZagrosParaPezpallet,
		zagros_emulated_chain::{genesis::ED as ZAGROS_ED, ZagrosRelayPezpallet},
		AssetHubZagrosPara as AssetHubZagros, AssetHubZagrosParaReceiver as AssetHubZagrosReceiver,
		AssetHubZagrosParaSender as AssetHubZagrosSender, BridgeHubZagrosPara as BridgeHubZagros,
		CollectivesZagrosPara as CollectivesZagros, CoretimeZagrosPara as CoretimeZagros,
		CoretimeZagrosParaReceiver as CoretimeZagrosReceiver,
		CoretimeZagrosParaSender as CoretimeZagrosSender, PenpalBPara as PenpalB,
		PeopleZagrosPara as PeopleZagros, ZagrosRelay as Zagros,
		ZagrosRelayReceiver as ZagrosReceiver, ZagrosRelaySender as ZagrosSender,
	};
}

#[cfg(test)]
mod tests;
