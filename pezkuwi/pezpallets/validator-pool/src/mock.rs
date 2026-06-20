// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{self as pezpallet_validator_pool, types::*};
use pezframe_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, Everything},
};
use pezframe_system as system;
use pezsp_core::H256;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

// Configure a mock runtime to test the pezpallet.
// Note: We don't include pezpallet_session here because it requires complex Currency setup.
// We can test SessionManager trait implementation directly.
construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		Balances: pezpallet_balances,
		ValidatorPool: pezpallet_validator_pool,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeTask = ();
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = pezframe_system::mocking::MockBlock<Test>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type ExtensionsWeightInfo = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pezpallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

// Mock Randomness
pub struct MockRandomness;
impl Randomness<H256, BlockNumber> for MockRandomness {
	fn random(subject: &[u8]) -> (H256, BlockNumber) {
		let mut hash = H256::zero();
		// Simple deterministic randomness for testing
		if !subject.is_empty() {
			hash.as_mut()[0] = subject[0];
		}
		(hash, 1)
	}
}

// Test implementations for trait dependencies
pub struct TestTrustProvider;
impl TrustScoreProvider<AccountId> for TestTrustProvider {
	fn trust_score_of(who: &AccountId) -> u128 {
		match who {
			1..=15 => 1000, // Test users with high trust (threshold: 450)
			_ => 100,       // Others have insufficient trust
		}
	}
}

pub struct TestTikiProvider;
impl TikiScoreProvider<AccountId> for TestTikiProvider {
	fn get_tiki_score(who: &AccountId) -> u32 {
		match who {
			1..=15 => 1, // Tüm test user'ları için tiki var
			_ => 0,
		}
	}
}

pub struct TestReferralProvider;
impl ReferralProvider<AccountId> for TestReferralProvider {
	fn get_referral_count(who: &AccountId) -> u32 {
		match who {
			1..=15 => 1000, // Tüm test user'ları için yüksek community support (threshold: 500)
			_ => 600,       // Diğerleri için de yeterli
		}
	}
}

pub struct TestPerwerdeProvider;
impl PerwerdeProvider<AccountId> for TestPerwerdeProvider {
	fn get_perwerde_score(who: &AccountId) -> u32 {
		match who {
			1..=15 => 100, // Tüm test user'ları için perwerde var
			_ => 50,
		}
	}
}

parameter_types! {
	pub const MaxValidators: u32 = 21;
	pub const MaxPoolSize: u32 = 500;
	pub const MinStakeAmount: u128 = 1000;
}

// Mock WeightInfo implementation
pub struct MockWeightInfo;
impl crate::WeightInfo for MockWeightInfo {
	fn join_validator_pool() -> pezframe_support::weights::Weight {
		pezframe_support::weights::Weight::from_parts(10_000, 0)
	}

	fn leave_validator_pool() -> pezframe_support::weights::Weight {
		pezframe_support::weights::Weight::from_parts(10_000, 0)
	}

	fn update_performance_metrics() -> pezframe_support::weights::Weight {
		pezframe_support::weights::Weight::from_parts(10_000, 0)
	}

	fn force_new_era(_p: u32) -> pezframe_support::weights::Weight {
		pezframe_support::weights::Weight::from_parts(50_000, 0)
	}

	fn update_category() -> pezframe_support::weights::Weight {
		pezframe_support::weights::Weight::from_parts(10_000, 0)
	}

	fn set_pool_parameters() -> pezframe_support::weights::Weight {
		pezframe_support::weights::Weight::from_parts(10_000, 0)
	}
}

impl Config for Test {
	type WeightInfo = MockWeightInfo;
	type Randomness = MockRandomness;
	type TrustSource = TestTrustProvider;
	type TikiSource = TestTikiProvider;
	type ReferralSource = TestReferralProvider;
	type PerwerdeSource = TestPerwerdeProvider;
	type PoolManagerOrigin = pezframe_system::EnsureRoot<AccountId>;
	type MaxValidators = MaxValidators;
	type MaxPoolSize = MaxPoolSize;
	type MinStakeAmount = MinStakeAmount;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	new_test_ext_with_mode(OperationMode::Active)
}

// Build genesis storage with specific operation mode
pub fn new_test_ext_with_mode(mode: OperationMode) -> pezsp_io::TestExternalities {
	let mut storage = system::GenesisConfig::<Test>::default().build_storage().unwrap();

	// Initialize balances - Fixed genesis config with correct type
	pezpallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(1, 10000),
			(2, 8000),
			(3, 6000),
			(4, 4000),
			(5, 2000),
			(6, 5000),
			(7, 5000),
			(8, 5000),
			(9, 5000),
			(10, 5000),
		],
		dev_accounts: None,
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	// Initialize validator pool with genesis config
	pezpallet_validator_pool::GenesisConfig::<Test> {
		operation_mode: mode,
		era_length: 100,
		initial_pool_members: vec![],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(storage);
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}

// Build genesis storage for shadow mode testing
pub fn new_test_ext_shadow_mode() -> pezsp_io::TestExternalities {
	new_test_ext_with_mode(OperationMode::Shadow)
}

// Helper functions for tests
pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 1 {
			ValidatorPool::on_finalize(System::block_number());
			System::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		ValidatorPool::on_initialize(System::block_number());
	}
}

#[allow(dead_code)]
pub fn advance_era() {
	let current_era_start = ValidatorPool::era_start();
	let era_length = ValidatorPool::era_length();
	run_to_block(current_era_start + era_length + 1);
}

// Create test categories
pub fn stake_validator_category() -> ValidatorPoolCategory {
	ValidatorPoolCategory::StakeValidator { min_stake: 1000, trust_threshold: 450 }
}

pub fn parliamentary_validator_category() -> ValidatorPoolCategory {
	ValidatorPoolCategory::ParliamentaryValidator
}

pub fn merit_validator_category() -> ValidatorPoolCategory {
	ValidatorPoolCategory::MeritValidator {
		special_tikis: vec![1u8].try_into().unwrap(), // Mock tiki type
		community_threshold: 500,
	}
}
