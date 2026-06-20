// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate as pezpallet_messaging;
use pezframe_support::{
	derive_impl,
	pezpallet_prelude::ConstU32,
	traits::{ConstU128, ConstU64},
};
use pezsp_runtime::BuildStorage;

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		Balances: pezpallet_balances,
		Messaging: pezpallet_messaging,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u128>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = u128;
	type ExistentialDeposit = ConstU128<1>;
}

/// Mock citizenship checker — accounts 1-10 are citizens
pub struct MockCitizenshipChecker;
impl crate::types::CitizenshipChecker<u64> for MockCitizenshipChecker {
	fn is_citizen(who: &u64) -> bool {
		*who >= 1 && *who <= 10
	}
}

/// Mock trust score checker — accounts 1-10 have trust score 50, others 0
pub struct MockTrustScoreChecker;
impl crate::types::TrustScoreChecker<u64> for MockTrustScoreChecker {
	fn trust_score_of(who: &u64) -> u32 {
		if *who >= 1 && *who <= 10 {
			50
		} else {
			0
		}
	}
}

impl pezpallet_messaging::Config for Test {
	type WeightInfo = ();
	type CitizenshipChecker = MockCitizenshipChecker;
	type TrustScoreChecker = MockTrustScoreChecker;
	type MinTrustScore = ConstU32<20>;
	type MaxMessageSize = ConstU32<512>;
	type MaxInboxSize = ConstU32<50>;
	type MaxMessagesPerEra = ConstU32<5>;
	type EraLength = ConstU64<100>; // 100 blocks per era in tests
}

/// Build test externalities
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Test> {
		balances: (1..=10).map(|i| (i, 100_000_000_000_000)).collect(),
		dev_accounts: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}
