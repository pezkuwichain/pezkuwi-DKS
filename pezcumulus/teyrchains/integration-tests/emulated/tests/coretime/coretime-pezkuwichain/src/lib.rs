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
	pub(crate) use xcm::{latest::PEZKUWICHAIN_GENESIS_HASH, prelude::*};

	// Pezcumulus
	pub(crate) use emulated_integration_tests_common::xcm_pez_emulator::{
		assert_expected_events, Chain, TestExt, Teyrchain,
	};
	pub(crate) use pezkuwichain_system_emulated_network::{
		asset_hub_pezkuwichain_emulated_chain::genesis::ED as ASSET_HUB_PEZKUWICHAIN_ED,
		coretime_pezkuwichain_emulated_chain::{
			coretime_pezkuwichain_runtime::ExistentialDeposit as CoretimePezkuwichainExistentialDeposit,
			genesis::ED as CORETIME_PEZKUWICHAIN_ED, CoretimePezkuwichainParaPezpallet,
			CoretimePezkuwichainParaPezpallet as CoretimePezkuwichainPallet,
		},
		pezkuwichain_emulated_chain::{genesis::ED as PEZKUWICHAIN_ED, PezkuwichainRelayPezpallet},
		AssetHubPezkuwichainPara as AssetHubPezkuwichain,
		AssetHubPezkuwichainParaReceiver as AssetHubPezkuwichainReceiver,
		AssetHubPezkuwichainParaSender as AssetHubPezkuwichainSender,
		CoretimePezkuwichainPara as CoretimePezkuwichain,
		CoretimePezkuwichainParaReceiver as CoretimePezkuwichainReceiver,
		CoretimePezkuwichainParaSender as CoretimePezkuwichainSender,
		PezkuwichainRelay as Pezkuwichain, PezkuwichainRelayReceiver as PezkuwichainReceiver,
		PezkuwichainRelaySender as PezkuwichainSender,
	};
}

#[cfg(test)]
mod tests;
