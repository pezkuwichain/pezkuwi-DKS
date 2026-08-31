// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Test runtime for `pezpallet-pez-rewards`.
//!
//! Deliberately small. The trust roll and the parliamentary roll reach the pallet through
//! traits, and the money is on another chain, so none of the trust pallet, welati, tiki or an
//! assets instance has to be stood up here. What the mocks give back is exactly what the
//! pallet reads, and nothing else.

use crate as pezpallet_pez_rewards;
use pezframe_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, OnFinalize, OnInitialize},
};
use pezframe_system::EnsureRoot;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

pub type AccountId = u64;
pub type BlockNumber = u64;
type Block = pezframe_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		PezRewards: pezpallet_pez_rewards,
	}
);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const SS58Prefix: u8 = 42;
}

impl pezframe_system::Config for Test {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = pezframe_support::weights::constants::RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = pezsp_core::H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
	type RuntimeTask = ();
	type ExtensionsWeightInfo = ();
}

// ---------------------------------------------------------------------------
// The trust roll
// ---------------------------------------------------------------------------

pezframe_support::parameter_types! {
	pub TreasuryChain: xcm::latest::Location =
		xcm::latest::Location::new(1, [xcm::latest::Junction::Teyrchain(1000)]);
}

thread_local! {
	pub static SCORES: core::cell::RefCell<alloc::collections::BTreeMap<AccountId, u128>> =
		const { core::cell::RefCell::new(alloc::collections::BTreeMap::new()) };
	/// Every block the payroll has asked the roll to be held until.
	pub static FREEZES: core::cell::RefCell<Vec<BlockNumber>> =
		const { core::cell::RefCell::new(Vec::new()) };
	pub static SEATS: core::cell::RefCell<alloc::collections::BTreeMap<AccountId, (BlockNumber, bool)>> =
		const { core::cell::RefCell::new(alloc::collections::BTreeMap::new()) };
	pub static SENT_XCM: core::cell::RefCell<Vec<(xcm::latest::Location, xcm::latest::Xcm<()>)>> =
		const { core::cell::RefCell::new(Vec::new()) };
	pub static SENDING_FAILS: core::cell::RefCell<bool> = const { core::cell::RefCell::new(false) };
}

extern crate alloc;

pub struct MockTrustRoll;
impl pezpallet_pez_rewards::TrustRoll<AccountId, BlockNumber> for MockTrustRoll {
	fn score_of(who: &AccountId) -> u128 {
		SCORES.with(|s| s.borrow().get(who).copied().unwrap_or(0))
	}
	fn total_score() -> u128 {
		SCORES.with(|s| s.borrow().values().sum())
	}
	fn freeze_until(until: BlockNumber) {
		FREEZES.with(|f| f.borrow_mut().push(until));
	}
}

/// Put `who` on the trust roll with `score`.
pub fn set_trust(who: AccountId, score: u128) {
	SCORES.with(|s| {
		if score == 0 {
			s.borrow_mut().remove(&who);
		} else {
			s.borrow_mut().insert(who, score);
		}
	});
}

/// Every block the payroll has asked the trust roll to be held until.
pub fn freezes() -> Vec<BlockNumber> {
	FREEZES.with(|f| f.borrow().clone())
}

// ---------------------------------------------------------------------------
// The parliamentary roll
// ---------------------------------------------------------------------------

pub struct MockParliamentRoll;
impl pezpallet_pez_rewards::ParliamentRoll<AccountId, BlockNumber> for MockParliamentRoll {
	fn seated_at(who: &AccountId) -> Option<BlockNumber> {
		SEATS.with(|s| s.borrow().get(who).map(|(at, _)| *at))
	}
	fn holds_seat(who: &AccountId) -> bool {
		SEATS.with(|s| s.borrow().get(who).map(|(_, holds)| *holds).unwrap_or(false))
	}
}

/// Put `who` on the parliamentary roll, seated at `at`, holding the seat or not.
///
/// The two are separate on purpose: a member the Diwan has removed is still on the roll and
/// no longer holds the seat, and that is the case the pallet has to get right.
pub fn set_seat(who: AccountId, seated_at: BlockNumber, holds: bool) {
	SEATS.with(|s| {
		s.borrow_mut().insert(who, (seated_at, holds));
	});
}

pub fn clear_seat(who: AccountId) {
	SEATS.with(|s| {
		s.borrow_mut().remove(&who);
	});
}

// ---------------------------------------------------------------------------
// The payment channel
// ---------------------------------------------------------------------------

/// Records what the pallet tried to send instead of sending it.
///
/// A test that only looked at storage could not tell "recorded a claim" from "recorded a
/// claim and actually instructed the payment", and on this pallet those are the whole point:
/// the money is on the other side of a bridge.
pub struct RecordingXcmSender;

impl xcm::latest::SendXcm for RecordingXcmSender {
	type Ticket = (xcm::latest::Location, xcm::latest::Xcm<()>);

	fn validate(
		dest: &mut Option<xcm::latest::Location>,
		msg: &mut Option<xcm::latest::Xcm<()>>,
	) -> xcm::latest::SendResult<Self::Ticket> {
		if SENDING_FAILS.with(|f| *f.borrow()) {
			return Err(xcm::latest::SendError::Transport("mock"));
		}
		let pair = (dest.take().unwrap(), msg.take().unwrap());
		Ok((pair, xcm::latest::Assets::new()))
	}

	fn deliver(ticket: Self::Ticket) -> Result<xcm::latest::XcmHash, xcm::latest::SendError> {
		SENT_XCM.with(|q| q.borrow_mut().push(ticket));
		Ok([0u8; 32])
	}
}

pub fn sent_xcm() -> Vec<(xcm::latest::Location, xcm::latest::Xcm<()>)> {
	SENT_XCM.with(|q| q.borrow().clone())
}

pub fn clear_sent_xcm() {
	SENT_XCM.with(|q| q.borrow_mut().clear());
}

pub fn fail_sending(on: bool) {
	SENDING_FAILS.with(|f| *f.borrow_mut() = on);
}

parameter_types! {
	pub const TreasuryPalletIndex: u8 = 70;
}

impl pezpallet_pez_rewards::Config for Test {
	type WeightInfo = ();
	type TrustSource = MockTrustRoll;
	type ParliamentSource = MockParliamentRoll;
	// The Asset Hub's XCM origin on the real runtimes; root stands in here.
	type FundingOrigin = EnsureRoot<AccountId>;
	type XcmSender = RecordingXcmSender;
	type TreasuryChainLocation = TreasuryChain;
	type TreasuryPalletIndex = TreasuryPalletIndex;
	type ForceOrigin = EnsureRoot<AccountId>;
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	// Every thread-local is per-test, but a test binary reuses threads, so they are cleared
	// here rather than left to whatever the previous test on this thread put in them.
	SCORES.with(|s| s.borrow_mut().clear());
	FREEZES.with(|f| f.borrow_mut().clear());
	SEATS.with(|s| s.borrow_mut().clear());
	SENT_XCM.with(|q| q.borrow_mut().clear());
	SENDING_FAILS.with(|f| *f.borrow_mut() = false);

	let t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}

pub fn run_to_block(n: BlockNumber) {
	while System::block_number() < n {
		PezRewards::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		PezRewards::on_initialize(System::block_number());
		check_invariants();
	}
}

/// Jump the clock without running the blocks in between.
///
/// The epoch is 432,000 blocks; running every one of them would make the suite unusable. The
/// hooks are run on the block that is landed on, which is the block that matters.
pub fn jump_to_block(n: BlockNumber) {
	assert!(n >= System::block_number(), "cannot jump backwards");
	System::set_block_number(n);
	PezRewards::on_initialize(n);
	check_invariants();
}

pub fn check_invariants() {
	#[cfg(feature = "try-runtime")]
	{
		use pezframe_support::traits::Hooks;
		<PezRewards as Hooks<BlockNumber>>::try_state(System::block_number())
			.expect("try_state failed");
	}
}
