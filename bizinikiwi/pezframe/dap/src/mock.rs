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

//! Test mock for the DAP pezpallet.

use crate::{self as pezpallet_dap, Config};
use pezframe_support::{derive_impl, parameter_types, PalletId};
use pezsp_core::crypto::AccountId32;
use pezsp_runtime::{traits::IdentityLookup, BuildStorage};

type Block = pezframe_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;

/// Derive a 32-byte test account from a small integer.
pub fn account_id(n: u64) -> AccountId {
	let mut bytes = [0u8; 32];
	bytes[..8].copy_from_slice(&n.to_le_bytes());
	AccountId::from(bytes)
}

pezframe_support::construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		Balances: pezpallet_balances,
		Dap: pezpallet_dap,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pezpallet_balances::AccountData<u64>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
	type ExistentialDeposit = ExistentialDeposit;
}

parameter_types! {
	pub const DapPalletId: PalletId = crate::DAP_PALLET_ID;
	pub const ExistentialDeposit: u64 = 10;
	pub const IssuanceCadence: u64 = 60_000; // 60 seconds
	pub const MaxElapsedPerDrip: u64 = 600_000; // 10 minutes
}

/// Returns 100 per 60_000ms elapsed (proportional).
pub struct TestIssuanceCurve;
impl pezsp_staking::budget::IssuanceCurve<u64> for TestIssuanceCurve {
	fn issue(_total_issuance: u64, elapsed_millis: u64) -> u64 {
		// 100 per minute (60_000ms)
		(100u128 * elapsed_millis as u128 / 60_000u128) as u64
	}
}

parameter_types! {
	pub static MockTime: u64 = 0;
}

impl pezframe_support::traits::Time for MockTime {
	type Moment = u64;
	fn now() -> u64 {
		Self::get()
	}
}

/// Test budget recipient: staker rewards pot.
pub struct TestStakerRecipient;
impl pezsp_staking::budget::BudgetRecipient<AccountId> for TestStakerRecipient {
	fn budget_key() -> pezsp_staking::budget::BudgetKey {
		pezsp_staking::budget::BudgetKey::truncate_from(b"staker_rewards".to_vec())
	}
	fn pot_account() -> AccountId {
		account_id(500)
	}
}

/// Test budget recipient: validator incentive pot.
pub struct TestValidatorIncentiveRecipient;
impl pezsp_staking::budget::BudgetRecipient<AccountId> for TestValidatorIncentiveRecipient {
	fn budget_key() -> pezsp_staking::budget::BudgetKey {
		pezsp_staking::budget::BudgetKey::truncate_from(b"validator_incentive".to_vec())
	}
	fn pot_account() -> AccountId {
		account_id(501)
	}
}

impl Config for Test {
	type Currency = Balances;
	type PalletId = DapPalletId;
	type IssuanceCurve = TestIssuanceCurve;
	type BudgetRecipients = (Dap, TestStakerRecipient, TestValidatorIncentiveRecipient);
	type Time = MockTime;
	type IssuanceCadence = IssuanceCadence;
	type MaxElapsedPerDrip = MaxElapsedPerDrip;
	type BudgetOrigin = pezframe_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
}

/// Sets a default budget allocation mimicking what the migration would do.
pub fn set_default_budget_allocation() {
	use pezsp_runtime::{BoundedBTreeMap, Perbill};
	use pezsp_staking::budget::BudgetRecipient;

	let mut map = BoundedBTreeMap::new();
	map.try_insert(Dap::budget_key(), Perbill::from_percent(15)).unwrap();
	map.try_insert(TestStakerRecipient::budget_key(), Perbill::from_percent(85))
		.unwrap();
	map.try_insert(TestValidatorIncentiveRecipient::budget_key(), Perbill::from_percent(0))
		.unwrap();
	crate::BudgetAllocation::<Test>::put(map);
}

fn new_test_ext_inner(fund_buffer: bool) -> pezsp_io::TestExternalities {
	let mut balances =
		vec![(account_id(1), 100u64), (account_id(2), 200u64), (account_id(3), 300u64)];

	if fund_buffer {
		let buffer: AccountId = Dap::buffer_account();
		balances.push((buffer, ExistentialDeposit::get()));
		// Also pre-fund staging account so tests can deposit without hitting the ED requirement.
		let staging: AccountId = Dap::staging_account();
		balances.push((staging, ExistentialDeposit::get()));
	}

	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pezpallet_balances::GenesisConfig::<Test> { balances, ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext: pezsp_io::TestExternalities = t.into();

	ext.execute_with(|| {
		// Initialize time to simulate "genesis already happened".
		MockTime::set(1_000_000);
		// Initialize LastIssuanceTimestamp so drip doesn't skip first call.
		crate::LastIssuanceTimestamp::<Test>::put(1_000_000);
	});

	ext
}

pub fn build_and_execute(fund_buffer: bool, test: impl FnOnce()) {
	let mut ext = new_test_ext_inner(fund_buffer);
	ext.execute_with(test);
	ext.execute_with(|| {
		Dap::do_try_state().unwrap();
	});
}

/// Asserts that `do_try_state` fails. Use after intentionally corrupting storage.
pub fn assert_try_state_invalid() {
	assert!(Dap::do_try_state().is_err());
}
