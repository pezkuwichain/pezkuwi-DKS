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
pub use coretime_pezkuwichain_emulated_chain;
pub use people_pezkuwichain_emulated_chain;
pub use pez_penpal_emulated_chain;
pub use pezbridge_hub_pezkuwichain_emulated_chain;
pub use pezkuwichain_emulated_chain;

use asset_hub_pezkuwichain_emulated_chain::AssetHubPezkuwichain;
use coretime_pezkuwichain_emulated_chain::CoretimePezkuwichain;
use people_pezkuwichain_emulated_chain::PeoplePezkuwichain;
use pez_penpal_emulated_chain::{PenpalA, PenpalB};
use pezbridge_hub_pezkuwichain_emulated_chain::BridgeHubPezkuwichain;
use pezkuwichain_emulated_chain::Pezkuwichain;

// Pezcumulus
use emulated_integration_tests_common::{
	accounts::{ALICE, BOB},
	xcm_pez_emulator::{decl_test_networks, decl_test_sender_receiver_accounts_parameter_types},
};

decl_test_networks! {
	pub struct PezkuwichainMockNet {
		relay_chain = Pezkuwichain,
		teyrchains = vec![
			AssetHubPezkuwichain,
			BridgeHubPezkuwichain,
			CoretimePezkuwichain,
			PenpalA,
			PenpalB,
			PeoplePezkuwichain,
		],
		bridge = ()
	},
}

decl_test_sender_receiver_accounts_parameter_types! {
	PezkuwichainRelay { sender: ALICE, receiver: BOB },
	AssetHubPezkuwichainPara { sender: ALICE, receiver: BOB },
	BridgeHubPezkuwichainPara { sender: ALICE, receiver: BOB },
	CoretimePezkuwichainPara { sender: ALICE, receiver: BOB },
	PenpalAPara { sender: ALICE, receiver: BOB },
	PenpalBPara { sender: ALICE, receiver: BOB },
	PeoplePezkuwichainPara { sender: ALICE, receiver: BOB }
}
