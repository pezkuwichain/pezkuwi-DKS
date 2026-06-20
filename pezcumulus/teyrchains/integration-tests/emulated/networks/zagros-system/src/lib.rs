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

pub use asset_hub_zagros_emulated_chain;
pub use collectives_zagros_emulated_chain;
pub use coretime_zagros_emulated_chain;
pub use people_zagros_emulated_chain;
pub use pez_penpal_emulated_chain;
pub use pezbridge_hub_zagros_emulated_chain;
pub use zagros_emulated_chain;

use asset_hub_zagros_emulated_chain::AssetHubZagros;
use collectives_zagros_emulated_chain::CollectivesZagros;
use coretime_zagros_emulated_chain::CoretimeZagros;
use people_zagros_emulated_chain::PeopleZagros;
use pez_penpal_emulated_chain::{PenpalA, PenpalB};
use pezbridge_hub_zagros_emulated_chain::BridgeHubZagros;
use zagros_emulated_chain::Zagros;

// Pezcumulus
use emulated_integration_tests_common::{
	accounts::{ALICE, BOB},
	xcm_pez_emulator::{decl_test_networks, decl_test_sender_receiver_accounts_parameter_types},
};

decl_test_networks! {
	pub struct ZagrosMockNet {
		relay_chain = Zagros,
		teyrchains = vec![
			AssetHubZagros,
			BridgeHubZagros,
			CollectivesZagros,
			CoretimeZagros,
			PeopleZagros,
			PenpalA,
			PenpalB,
		],
		bridge = ()
	},
}

decl_test_sender_receiver_accounts_parameter_types! {
	ZagrosRelay { sender: ALICE, receiver: BOB },
	AssetHubZagrosPara { sender: ALICE, receiver: BOB },
	BridgeHubZagrosPara { sender: ALICE, receiver: BOB },
	CollectivesZagrosPara { sender: ALICE, receiver: BOB },
	CoretimeZagrosPara { sender: ALICE, receiver: BOB },
	PeopleZagrosPara { sender: ALICE, receiver: BOB },
	PenpalAPara { sender: ALICE, receiver: BOB },
	PenpalBPara { sender: ALICE, receiver: BOB }
}
