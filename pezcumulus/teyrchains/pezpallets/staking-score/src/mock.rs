// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Simplified mock runtime for pezpallet-staking-score.
//! No real staking pallet - all data comes via CachedStakingDetails.

use crate as pezpallet_staking_score;
use pezframe_support::{
	construct_runtime, derive_impl, parameter_types, traits::ConstU32,
	weights::constants::RocksDbWeight,
};
use pezsp_runtime::BuildStorage;

use crate::UNITS;

// --- Type Aliases ---
type Block = pezframe_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

// --- Constants ---
parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const ExistentialDeposit: Balance = 1;
}

// --- Runtime ---
construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		StakingScore: pezpallet_staking_score,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type DbWeight = RocksDbWeight;
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<Balance>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type MaxLocks = ConstU32<1024>;
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
}

/// Mock noter checker for tests.
/// Account 99 is noter, everyone else is not.
pub struct MockNoterChecker;
impl crate::NoterCheck<AccountId> for MockNoterChecker {
	fn is_noter(who: &AccountId) -> bool {
		*who == 99
	}
}

impl crate::Config for Test {
	type Balance = Balance;
	type WeightInfo = ();
	type OnStakingUpdate = ();
	type NoterChecker = MockNoterChecker;
}

// --- ExtBuilder ---
pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		Self
	}
}

impl ExtBuilder {
	pub fn build(self) -> pezsp_io::TestExternalities {
		let mut storage =
			pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		pezpallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 1_000_000 * UNITS),
				(2, 1_000_000 * UNITS),
				(10, 1_000_000 * UNITS),
				(20, 100_000 * UNITS),
				(30, 100_000 * UNITS), // Charlie
				(99, 100_000 * UNITS), // NOTER
			],
			..Default::default()
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		pezsp_io::TestExternalities::new(storage)
	}

	pub fn build_and_execute(self, test: impl FnOnce()) {
		self.build().execute_with(test);
	}
}
