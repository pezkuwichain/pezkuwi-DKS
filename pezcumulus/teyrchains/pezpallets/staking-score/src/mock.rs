// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Simplified mock runtime for pezpallet-staking-score.
//! No real staking pallet - all data comes via CachedStakingDetails.

use crate as pezpallet_staking_score;
use pezframe_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU32, SortedMembers},
	weights::constants::RocksDbWeight,
};
use pezframe_system::EnsureSignedBy;
use pezsp_runtime::BuildStorage;

use crate::UNITS;

// --- Type Aliases ---
type Block = pezframe_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

// --- Test-only well-known accounts ---
/// A second, independent registered noter (distinct from account 99, the
/// pre-existing "the" noter in most tests) — used by dispute-window tests
/// that need two noters to exist simultaneously, mirroring how the Noter
/// tiki is designed to support any number of accounts holding it.
pub const NOTER_2: AccountId = 98;
/// A Council-equivalent member for `DisputeOrigin` in tests.
pub const DISPUTE_MEMBER: AccountId = 40;
/// A stronger governance-equivalent member for `SlashOrigin` in tests.
pub const SLASH_MEMBER: AccountId = 41;
/// Where slashed noter bonds land in tests (stands in for `RelayTreasuryAccount`).
pub const TREASURY: AccountId = 999;
/// A short, test-friendly dispute window — real enough to prove the
/// mechanism (tests advance blocks past it) without needing huge block jumps
/// in every test. The runtime uses `HOUR_IN_BLOCKS` (600 blocks); this mock
/// deliberately does not reuse that constant so tests stay fast.
pub const DISPUTE_WINDOW: BlockNumber = 10;

// --- Constants ---
parameter_types! {
	/// Generous enough to cover the dispute window and a bot cycle in the tests.
	pub const OracleGracePeriod: u64 = 100;
	pub const BlockHashCount: BlockNumber = 250;
	pub const ExistentialDeposit: Balance = 1;
	pub const NoterBondAmount: Balance = 1_000 * UNITS;
	pub const DisputeWindowBlocks: BlockNumber = DISPUTE_WINDOW;
	pub const SlashDestinationAccount: AccountId = TREASURY;
}

pub struct DisputeMemberProvider;
impl SortedMembers<AccountId> for DisputeMemberProvider {
	fn sorted_members() -> Vec<AccountId> {
		vec![DISPUTE_MEMBER]
	}
}

pub struct SlashMemberProvider;
impl SortedMembers<AccountId> for SlashMemberProvider {
	fn sorted_members() -> Vec<AccountId> {
		vec![SLASH_MEMBER]
	}
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
/// Accounts 99 and 98 (`NOTER_2`) are noters, everyone else is not.
pub struct MockNoterChecker;
impl crate::NoterCheck<AccountId> for MockNoterChecker {
	fn is_noter(who: &AccountId) -> bool {
		*who == 99 || *who == NOTER_2
	}
}

impl crate::Config for Test {
	type Balance = Balance;
	type WeightInfo = ();
	type OnStakingUpdate = ();
	type NoterChecker = MockNoterChecker;
	type Currency = Balances;
	type NoterBondAmount = NoterBondAmount;
	type DisputeWindow = DisputeWindowBlocks;
	type DisputeOrigin = EnsureSignedBy<DisputeMemberProvider, AccountId>;
	type SlashOrigin = EnsureSignedBy<SlashMemberProvider, AccountId>;
	type OracleGracePeriod = OracleGracePeriod;
	type SlashDestination = SlashDestinationAccount;
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
				(99, 100_000 * UNITS), // NOTER — enough to post NoterBondAmount (1,000 UNITS)
				(NOTER_2, 100_000 * UNITS),
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

// --- Test helpers: bonded-noter registration + dispute-window resolution ---

/// Register `noter` (idempotent — no-op if already registered) as a bonded,
/// active noter. Every noter-signed `receive_staking_details` call in tests
/// now requires this first, matching the real pallet's `register_as_noter`
/// gate.
pub fn ensure_registered(noter: AccountId) {
	if crate::NoterBonds::<Test>::get(noter).is_none() {
		assert!(
			StakingScore::register_as_noter(RuntimeOrigin::signed(noter)).is_ok(),
			"test setup: register_as_noter failed for {noter}"
		);
	}
}

/// Submit staking details as `noter`, then advance the block number past
/// `DisputeWindow` and finalize — i.e. simulate a noter submission that goes
/// unchallenged and takes effect. This is the noter-path equivalent of the
/// old (pre dispute-window) immediate-commit behavior most tests want.
pub fn submit_and_finalize(
	noter: AccountId,
	who: AccountId,
	source: crate::StakingSource,
	staked_amount: Balance,
	nominations_count: u32,
	unlocking_chunks_count: u32,
) {
	ensure_registered(noter);
	assert!(StakingScore::receive_staking_details(
		RuntimeOrigin::signed(noter),
		who,
		source,
		staked_amount,
		nominations_count,
		unlocking_chunks_count,
	)
	.is_ok());

	let matured_at = System::block_number() + DISPUTE_WINDOW;
	System::set_block_number(matured_at);
	assert!(
		StakingScore::finalize_staking_details(RuntimeOrigin::signed(noter), who, source).is_ok(),
		"test setup: finalize_staking_details failed"
	);
}
