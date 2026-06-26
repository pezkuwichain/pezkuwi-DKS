// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
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

// We do not declare all features used by `construct_runtime`
#[allow(unexpected_cfgs)]
mod teyrchain;

// We do not declare all features used by `construct_runtime`
#[allow(unexpected_cfgs)]
mod relay_chain;

#[cfg(test)]
mod tests;

use pezsp_runtime::BuildStorage;
use pezsp_tracing;
use xcm::prelude::*;
use xcm_executor::traits::ConvertLocation;
use xcm_pez_simulator::{decl_test_network, decl_test_relay_chain, decl_test_teyrchain, TestExt};

pub const ALICE: pezsp_runtime::AccountId32 = pezsp_runtime::AccountId32::new([1u8; 32]);
pub const INITIAL_BALANCE: u128 = 1_000_000_000;

decl_test_teyrchain! {
	pub struct ParaA {
		Runtime = teyrchain::Runtime,
		XcmpMessageHandler = teyrchain::MsgQueue,
		DmpMessageHandler = teyrchain::MsgQueue,
		new_ext = para_ext(1),
	}
}

decl_test_teyrchain! {
	pub struct ParaB {
		Runtime = teyrchain::Runtime,
		XcmpMessageHandler = teyrchain::MsgQueue,
		DmpMessageHandler = teyrchain::MsgQueue,
		new_ext = para_ext(2),
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
			(2, ParaB),
		],
	}
}

pub fn parent_account_id() -> teyrchain::AccountId {
	let location = (Parent,);
	teyrchain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn child_account_id(para: u32) -> relay_chain::AccountId {
	let location = (Teyrchain(para),);
	relay_chain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn child_account_account_id(
	para: u32,
	who: pezsp_runtime::AccountId32,
) -> relay_chain::AccountId {
	let location = (Teyrchain(para), AccountId32 { network: None, id: who.into() });
	relay_chain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn sibling_account_account_id(
	para: u32,
	who: pezsp_runtime::AccountId32,
) -> teyrchain::AccountId {
	let location = (Parent, Teyrchain(para), AccountId32 { network: None, id: who.into() });
	teyrchain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn parent_account_account_id(who: pezsp_runtime::AccountId32) -> teyrchain::AccountId {
	let location = (Parent, AccountId32 { network: None, id: who.into() });
	teyrchain::location_converter::LocationConverter::convert_location(&location.into()).unwrap()
}

pub fn para_ext(para_id: u32) -> pezsp_io::TestExternalities {
	use teyrchain::{MsgQueue, Runtime, System};

	let mut t = pezframe_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE), (parent_account_id(), INITIAL_BALANCE)],
		..Default::default()
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
	use relay_chain::{Runtime, RuntimeOrigin, System, Uniques};

	let mut t = pezframe_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(ALICE, INITIAL_BALANCE),
			(child_account_id(1), INITIAL_BALANCE),
			(child_account_id(2), INITIAL_BALANCE),
		],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		assert_eq!(Uniques::force_create(RuntimeOrigin::root(), 1, ALICE, true), Ok(()));
		assert_eq!(Uniques::mint(RuntimeOrigin::signed(ALICE), 1, 42, child_account_id(1)), Ok(()));
	});
	ext
}

pub type RelayChainPalletXcm = pezpallet_xcm::Pezpallet<relay_chain::Runtime>;
pub type TeyrchainPalletXcm = pezpallet_xcm::Pezpallet<teyrchain::Runtime>;
