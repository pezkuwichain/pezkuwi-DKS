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

pub use asset_hub_pezkuwichain_emulated_chain;
pub use asset_hub_zagros_emulated_chain;
pub use pez_penpal_emulated_chain;
pub use pezbridge_hub_pezkuwichain_emulated_chain;
pub use pezbridge_hub_zagros_emulated_chain;
pub use pezkuwichain_emulated_chain;
pub use zagros_emulated_chain;

use asset_hub_pezkuwichain_emulated_chain::AssetHubPezkuwichain;
use asset_hub_zagros_emulated_chain::AssetHubZagros;
use pez_penpal_emulated_chain::{PenpalA, PenpalB};
use pezbridge_hub_pezkuwichain_emulated_chain::BridgeHubPezkuwichain;
use pezbridge_hub_zagros_emulated_chain::BridgeHubZagros;
use pezkuwichain_emulated_chain::Pezkuwichain;
use zagros_emulated_chain::Zagros;

// Pezcumulus
use emulated_integration_tests_common::{
	accounts::{ALICE, BOB},
	impls::{BridgeHubMessageHandler, BridgeMessagesInstance1, BridgeMessagesInstance3},
	xcm_pez_emulator::{
		decl_test_bridges, decl_test_networks, decl_test_sender_receiver_accounts_parameter_types,
		Chain,
	},
};

decl_test_networks! {
	pub struct PezkuwichainMockNet {
		relay_chain = Pezkuwichain,
		teyrchains = vec![
			AssetHubPezkuwichain,
			BridgeHubPezkuwichain,
			PenpalA,
		],
		bridge = PezkuwichainZagrosMockBridge
	},
	pub struct ZagrosMockNet {
		relay_chain = Zagros,
		teyrchains = vec![
			AssetHubZagros,
			BridgeHubZagros,
			PenpalB,
		],
		bridge = ZagrosPezkuwichainMockBridge
	},
}

decl_test_bridges! {
	pub struct PezkuwichainZagrosMockBridge {
		source = BridgeHubPezkuwichainPara,
		target = BridgeHubZagrosPara,
		handler = PezkuwichainZagrosMessageHandler
	},
	pub struct ZagrosPezkuwichainMockBridge {
		source = BridgeHubZagrosPara,
		target = BridgeHubPezkuwichainPara,
		handler = ZagrosPezkuwichainMessageHandler
	}
}

type BridgeHubPezkuwichainRuntime = <BridgeHubPezkuwichainPara as Chain>::Runtime;
type BridgeHubZagrosRuntime = <BridgeHubZagrosPara as Chain>::Runtime;

pub type PezkuwichainZagrosMessageHandler = BridgeHubMessageHandler<
	BridgeHubPezkuwichainRuntime,
	BridgeMessagesInstance3,
	BridgeHubZagrosRuntime,
	BridgeMessagesInstance1,
>;
pub type ZagrosPezkuwichainMessageHandler = BridgeHubMessageHandler<
	BridgeHubZagrosRuntime,
	BridgeMessagesInstance1,
	BridgeHubPezkuwichainRuntime,
	BridgeMessagesInstance3,
>;

decl_test_sender_receiver_accounts_parameter_types! {
	PezkuwichainRelay { sender: ALICE, receiver: BOB },
	AssetHubPezkuwichainPara { sender: ALICE, receiver: BOB },
	BridgeHubPezkuwichainPara { sender: ALICE, receiver: BOB },
	ZagrosRelay { sender: ALICE, receiver: BOB },
	AssetHubZagrosPara { sender: ALICE, receiver: BOB },
	BridgeHubZagrosPara { sender: ALICE, receiver: BOB },
	PenpalAPara { sender: ALICE, receiver: BOB },
	PenpalBPara { sender: ALICE, receiver: BOB }
}
