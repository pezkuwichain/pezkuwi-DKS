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
		assert_expected_events, bx, Chain, TestExt, Teyrchain as Para,
	};
	pub(crate) use zagros_system_emulated_network::{
		self,
		asset_hub_zagros_emulated_chain::{
			AssetHubZagrosParaPezpallet, AssetHubZagrosParaPezpallet as AssetHubZagrosPallet,
		},
		collectives_zagros_emulated_chain::CollectivesZagrosParaPezpallet,
		coretime_zagros_emulated_chain::CoretimeZagrosParaPezpallet,
		people_zagros_emulated_chain::{
			people_zagros_runtime::{
				self, xcm_config::XcmConfig as PeopleZagrosXcmConfig,
				ExistentialDeposit as PeopleZagrosExistentialDeposit,
			},
			PeopleZagrosParaPezpallet, PeopleZagrosParaPezpallet as PeopleZagrosPallet,
		},
		pez_penpal_emulated_chain::{PenpalAssetOwner, PenpalBParaPezpallet},
		pezbridge_hub_zagros_emulated_chain::BridgeHubZagrosParaPezpallet,
		zagros_emulated_chain::{genesis::ED as ZAGROS_ED, ZagrosRelayPezpallet},
		AssetHubZagrosPara as AssetHubZagros, AssetHubZagrosParaReceiver as AssetHubZagrosReceiver,
		BridgeHubZagrosPara as BridgeHubZagros, CollectivesZagrosPara as CollectivesZagros,
		CoretimeZagrosPara as CoretimeZagros, PenpalBPara as PenpalB,
		PeopleZagrosPara as PeopleZagros, PeopleZagrosParaReceiver as PeopleZagrosReceiver,
		PeopleZagrosParaSender as PeopleZagrosSender, ZagrosRelay as Zagros,
		ZagrosRelayReceiver as ZagrosReceiver, ZagrosRelaySender as ZagrosSender,
	};
}

#[cfg(test)]
mod tests;
