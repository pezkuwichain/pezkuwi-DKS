// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Bizinikiwi.

// Bizinikiwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bizinikiwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bizinikiwi.  If not, see <http://www.gnu.org/licenses/>.

pub mod mocks;
pub mod primitives;
pub mod relay_chain;
pub mod teyrchain;

#[cfg(test)]
mod tests;

use crate::primitives::{AccountId, UNITS};
pub use pezpallet_contracts::test_utils::{ALICE, BOB};
use pezsp_runtime::BuildStorage;
use xcm::latest::prelude::*;
use xcm_executor::traits::ConvertLocation;
pub use xcm_pez_simulator::TestExt;
use xcm_pez_simulator::{decl_test_network, decl_test_relay_chain, decl_test_teyrchain};

// Accounts
pub const ADMIN: pezsp_runtime::AccountId32 = pezsp_runtime::AccountId32::new([0u8; 32]);

// Balances
pub const INITIAL_BALANCE: u128 = 1_000_000_000 * UNITS;

decl_test_teyrchain! {
	pub struct ParaA {
		Runtime = teyrchain::Runtime,
		XcmpMessageHandler = teyrchain::MsgQueue,
		DmpMessageHandler = teyrchain::MsgQueue,
		new_ext = para_ext(1),
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
			(1, ParaA),
		],
	}
}

pub fn relay_sovereign_account_id() -> AccountId {
	let location: Location = (Parent,).into();
	teyrchain::SovereignAccountOf::convert_location(&location).unwrap()
}

pub fn teyrchain_sovereign_account_id(para: u32) -> AccountId {
	let location: Location = (Teyrchain(para),).into();
	relay_chain::SovereignAccountOf::convert_location(&location).unwrap()
}

pub fn teyrchain_account_sovereign_account_id(
	para: u32,
	who: pezsp_runtime::AccountId32,
) -> AccountId {
	let location: Location = (
		Teyrchain(para),
		AccountId32 { network: Some(relay_chain::RelayNetwork::get()), id: who.into() },
	)
		.into();
	relay_chain::SovereignAccountOf::convert_location(&location).unwrap()
}

pub fn para_ext(para_id: u32) -> pezsp_io::TestExternalities {
	use teyrchain::{MsgQueue, Runtime, System};

	let mut t = pezframe_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(ALICE, INITIAL_BALANCE),
			(relay_sovereign_account_id(), INITIAL_BALANCE),
			(BOB, INITIAL_BALANCE),
		],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	pezpallet_assets::GenesisConfig::<Runtime> {
		assets: vec![
			(0u128, ADMIN, false, 1u128), // Create derivative asset for relay's native token
		],
		metadata: Default::default(),
		accounts: vec![
			(0u128, ALICE, INITIAL_BALANCE),
			(0u128, relay_sovereign_account_id(), INITIAL_BALANCE),
		],
		next_asset_id: None,
		reserves: vec![],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		pezsp_tracing::try_init_simple();
		System::set_block_number(1);
		MsgQueue::set_para_id(para_id.into());
	});
	ext
}

pub fn relay_ext() -> pezsp_io::TestExternalities {
	use relay_chain::{Runtime, System};

	let mut t = pezframe_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(ALICE, INITIAL_BALANCE),
			(teyrchain_sovereign_account_id(1), INITIAL_BALANCE),
			(teyrchain_account_sovereign_account_id(1, ALICE), INITIAL_BALANCE),
		],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}

pub type TeyrchainPalletXcm = pezpallet_xcm::Pezpallet<teyrchain::Runtime>;
pub type TeyrchainBalances = pezpallet_balances::Pezpallet<teyrchain::Runtime>;
