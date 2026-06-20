// This file is part of Bizinikiwi.

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

//! Tests mock for `pezpallet-assets-freezer`.

pub use super::*;
use pezframe_support::{derive_impl, traits::AsEnsureOriginWithArg};
use pezsp_runtime::BuildStorage;

type Block = pezframe_system::mocking::MockBlock<Test>;

#[pezframe_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeTask,
		RuntimeHoldReason,
		RuntimeFreezeReason
	)]
	pub struct Test;

	#[runtime::pezpallet_index(0)]
	pub type System = pezframe_system;
	#[runtime::pezpallet_index(10)]
	pub type Balances = pezpallet_balances;
	#[runtime::pezpallet_index(20)]
	pub type Assets = pezpallet_assets;
	#[runtime::pezpallet_index(21)]
	pub type Revive = pezpallet_revive;
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u64>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig as pezpallet_balances::DefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
}

#[derive_impl(pezpallet_assets::config_preludes::TestDefaultConfig as pezpallet_assets::DefaultConfig)]
impl pezpallet_assets::Config for Test {
	type CreateOrigin = AsEnsureOriginWithArg<pezframe_system::EnsureSigned<u64>>;
	type ForceOrigin = pezframe_system::EnsureRoot<u64>;
	type Currency = Balances;
}

#[derive_impl(pezpallet_revive::config_preludes::TestDefaultConfig)]
impl pezpallet_revive::Config for Test {
	type AddressMapper = pezpallet_revive::TestAccountMapper<Self>;
	type Balance = u64;
	type Currency = Balances;
	type Precompiles = (ERC20<Self, InlineIdConfig<0x0120>>,);
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = RuntimeGenesisConfig {
		assets: pezpallet_assets::GenesisConfig {
			assets: vec![(1, 0, true, 1)],
			metadata: vec![],
			accounts: vec![(1, 1, 100)],
			next_asset_id: None,
			reserves: vec![],
		},
		system: Default::default(),
		balances: Default::default(),
		revive: Default::default(),
	}
	.build_storage()
	.unwrap();
	let mut ext: pezsp_io::TestExternalities = t.into();
	ext.execute_with(|| {
		System::set_block_number(1);
	});

	ext
}
