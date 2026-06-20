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

//! Test utilities

use super::*;
use crate as pezpallet_lottery;

use pezframe_support::{derive_impl, parameter_types, traits::ConstU32};
use pezframe_support_test::TestRandomness;
use pezframe_system::EnsureRoot;
use pezsp_runtime::{BuildStorage, Perbill};

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		Lottery: pezpallet_lottery,
	}
);

parameter_types! {
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u64>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub const LotteryPalletId: PalletId = PalletId(*b"py/lotto");
}

impl Config for Test {
	type PalletId = LotteryPalletId;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type Randomness = TestRandomness<Self>;
	type RuntimeEvent = RuntimeEvent;
	type ManagerOrigin = EnsureRoot<u64>;
	type MaxCalls = ConstU32<2>;
	type ValidateCall = Lottery;
	type MaxGenerateRandom = ConstU32<10>;
	type WeightInfo = ();
}

pub type SystemCall = pezframe_system::Call<Test>;
pub type BalancesCall = pezpallet_balances::Call<Test>;

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pezpallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 100), (2, 100), (3, 100), (4, 100), (5, 100)],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}
