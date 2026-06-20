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

use super::*;
use crate as pezpallet_glutton;

use pezframe_support::{assert_ok, derive_impl};
use pezsp_runtime::BuildStorage;

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Glutton: pezpallet_glutton,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type WeightInfo = ();
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Set the `compute`, `storage` and `block_length` limits.
///
/// `1.0` corresponds to `100%`.
pub fn set_limits(compute: f64, storage: f64, block_length: f64) {
	assert_ok!(Glutton::set_compute(RuntimeOrigin::root(), FixedU64::from_float(compute)));
	assert_ok!(Glutton::set_storage(RuntimeOrigin::root(), FixedU64::from_float(storage)));
	assert_ok!(Glutton::set_block_length(
		RuntimeOrigin::root(),
		FixedU64::from_float(block_length)
	));
}
