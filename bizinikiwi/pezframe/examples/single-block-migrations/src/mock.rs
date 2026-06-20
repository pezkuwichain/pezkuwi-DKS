// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: MIT-0

// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is furnished to do
// so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#![cfg(any(all(feature = "try-runtime", test), doc))]

use crate::*;
use pezframe_support::{derive_impl, weights::constants::ParityDbWeight};

// Re-export crate as its pezpallet name for construct_runtime.
use crate as pezpallet_example_storage_migration;

type Block = pezframe_system::mocking::MockBlock<MockRuntime>;

// For testing the pezpallet, we construct a mock runtime.
pezframe_support::construct_runtime!(
	pub struct MockRuntime {
		System: pezframe_system::{Pezpallet, Call, Config<T>, Storage, Event<T>},
		Balances: pezpallet_balances::{Pezpallet, Call, Storage, Config<T>, Event<T>},
		Example: pezpallet_example_storage_migration::{Pezpallet, Call, Storage},
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for MockRuntime {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u64>;
	type DbWeight = ParityDbWeight;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for MockRuntime {
	type AccountStore = System;
}

impl Config for MockRuntime {}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	use pezsp_runtime::BuildStorage;

	let t = RuntimeGenesisConfig { system: Default::default(), balances: Default::default() }
		.build_storage()
		.unwrap();
	t.into()
}
