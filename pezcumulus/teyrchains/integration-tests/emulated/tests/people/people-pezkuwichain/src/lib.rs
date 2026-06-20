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
	pub(crate) use pezframe_support::pezsp_runtime::DispatchResult;

	// Pezkuwi
	pub(crate) use xcm::{latest::PEZKUWICHAIN_GENESIS_HASH, prelude::*};

	// Pezcumulus
	pub(crate) use asset_test_pezutils::xcm_helpers;
	pub(crate) use emulated_integration_tests_common::xcm_pez_emulator::{
		assert_expected_events, bx, Chain, Test, TestArgs, TestContext, TestExt, Teyrchain as Para,
	};
	pub(crate) use pezkuwichain_system_emulated_network::{
		people_pezkuwichain_emulated_chain::{
			people_pezkuwichain_runtime::{
				xcm_config::XcmConfig as PeoplePezkuwichainXcmConfig,
				ExistentialDeposit as PeoplePezkuwichainExistentialDeposit,
			},
			PeoplePezkuwichainParaPezpallet,
			PeoplePezkuwichainParaPezpallet as PeoplePezkuwichainPallet,
		},
		pezkuwichain_emulated_chain::{genesis::ED as PEZKUWICHAIN_ED, PezkuwichainRelayPezpallet},
		AssetHubPezkuwichainPara as AssetHubPezkuwichain,
		AssetHubPezkuwichainParaReceiver as AssetHubPezkuwichainReceiver,
		PeoplePezkuwichainPara as PeoplePezkuwichain,
		PeoplePezkuwichainParaReceiver as PeoplePezkuwichainReceiver,
		PeoplePezkuwichainParaSender as PeoplePezkuwichainSender,
		PezkuwichainRelay as Pezkuwichain, PezkuwichainRelayReceiver as PezkuwichainReceiver,
		PezkuwichainRelaySender as PezkuwichainSender,
	};
	pub(crate) use teyrchains_common::Balance;

	pub(crate) type SystemParaToRelayTest = Test<PeoplePezkuwichain, Pezkuwichain>;
}

#[cfg(test)]
mod tests;
