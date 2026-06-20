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

//! Test environment for transaction-storage pezpallet.

use crate::{
	self as pezpallet_transaction_storage, TransactionStorageProof, DEFAULT_MAX_BLOCK_TRANSACTIONS,
	DEFAULT_MAX_TRANSACTION_SIZE,
};
use pezframe_support::{derive_impl, traits::ConstU32};
use pezsp_runtime::{traits::IdentityLookup, BuildStorage};

pub type Block = pezframe_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pezpallet.
pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		TransactionStorage: pezpallet_transaction_storage,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u64>;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
}

impl pezpallet_transaction_storage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type FeeDestination = ();
	type WeightInfo = ();
	type MaxBlockTransactions = ConstU32<{ DEFAULT_MAX_BLOCK_TRANSACTIONS }>;
	type MaxTransactionSize = ConstU32<{ DEFAULT_MAX_TRANSACTION_SIZE }>;
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = RuntimeGenesisConfig {
		system: Default::default(),
		balances: pezpallet_balances::GenesisConfig::<Test> {
			balances: vec![(1, 1000000000), (2, 100), (3, 100), (4, 100)],
			..Default::default()
		},
		transaction_storage: pezpallet_transaction_storage::GenesisConfig::<Test> {
			storage_period: 10,
			byte_fee: 2,
			entry_fee: 200,
		},
	}
	.build_storage()
	.unwrap();
	t.into()
}

pub fn run_to_block(n: u64, f: impl Fn() -> Option<TransactionStorageProof> + 'static) {
	System::run_to_block_with::<AllPalletsWithSystem>(
		n,
		pezframe_system::RunToBlockHooks::default().before_finalize(|_| {
			if let Some(proof) = f() {
				TransactionStorage::check_proof(RuntimeOrigin::none(), proof).unwrap();
			}
		}),
	);
}
