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

// Mock for Whitelist Pezpallet

#![cfg(test)]

use crate as pezpallet_whitelist;

use pezframe::testing_prelude::*;
type Block = MockBlock<Test>;

construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		Whitelist: pezpallet_whitelist,
		Preimage: pezpallet_preimage,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u64>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
}

impl pezpallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<Self::AccountId>;
	type Consideration = ();
	type WeightInfo = ();
}

impl pezpallet_whitelist::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WhitelistOrigin = EnsureRoot<Self::AccountId>;
	type DispatchWhitelistedOrigin = EnsureRoot<Self::AccountId>;
	type Preimages = Preimage;
	type WeightInfo = ();
}

pub fn new_test_ext() -> TestExternalities {
	let t = RuntimeGenesisConfig::default().build_storage().unwrap();
	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
