// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! Mock network

use pezframe::deps::{
	pezframe_system,
	pezsp_io::TestExternalities,
	pezsp_runtime::{AccountId32, BuildStorage},
};
use xcm_pez_simulator::{decl_test_network, decl_test_relay_chain, decl_test_teyrchain, TestExt};

use super::{relay_chain, teyrchain};

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const UNITS: u64 = 10_000_000_000;
pub const CENTS: u64 = 100_000_000;
pub const INITIAL_BALANCE: u64 = UNITS;

decl_test_teyrchain! {
	pub struct ParaA {
		Runtime = teyrchain::Runtime,
		XcmpMessageHandler = teyrchain::MessageQueue,
		DmpMessageHandler = teyrchain::MessageQueue,
		new_ext = para_ext(),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay_chain::Runtime,
		RuntimeCall = relay_chain::RuntimeCall,
		RuntimeEvent = relay_chain::RuntimeEvent,
		XcmConfig = relay_chain::XcmConfig,
		MessageQueue = relay_chain::MessageQueue,
		System = relay_chain::System,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct MockNet {
		relay_chain = Relay,
		teyrchains = vec![
			(2222, ParaA),
		],
	}
}

pub fn para_ext() -> TestExternalities {
	use teyrchain::{MessageQueue, Runtime, System};

	let t = pezframe_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	let mut ext = pezframe::deps::pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		MessageQueue::set_para_id(2222.into());
	});
	ext
}

pub fn relay_ext() -> TestExternalities {
	use relay_chain::{Runtime, System};

	let mut t = pezframe_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE)],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}
