// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// pezkuwi/pallets/pez-treasury/src/mock.rs
// VERSION 3: AccountId type changed to H256 (instead of u64)

use crate::{self as pezpallet_pez_treasury, weights};
use pezframe_support::{
	assert_ok, construct_runtime, parameter_types,
	traits::{fungibles::Mutate, ConstU128, ConstU32, OnFinalize, OnInitialize},
	PalletId,
};
use pezsp_core::H256;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = pezframe_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		Assets: pezpallet_assets,
		PezTreasury: pezpallet_pez_treasury,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl pezframe_system::Config for Test {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = pezframe_support::weights::constants::RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = H256; // V3: changed u64 -> H256
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
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

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
}

impl pezpallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = u128;
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

parameter_types! {
	pub const AssetDeposit: u128 = 100;
	pub const ApprovalDeposit: u128 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u128 = 10;
	pub const MetadataDepositPerByte: u128 = 1;
}

impl pezpallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin = pezframe_support::traits::AsEnsureOriginWithArg<
		pezframe_system::EnsureSigned<Self::AccountId>,
	>;
	type ForceOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<0>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	type RemoveItemsLimit = ConstU32<1000>;
	type Holder = ();
	type ReserveData = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

// CRITICAL: These three PalletIds MUST be DIFFERENT
parameter_types! {
	pub const PezTreasuryPalletId: PalletId = PalletId(*b"py/pztrs");
	pub const PezIncentivePotId: PalletId = PalletId(*b"py/pzinc");
	pub const PezGovernmentPotId: PalletId = PalletId(*b"py/pzgov");
	pub const PezAssetId: u32 = 1;
}

// V3: Test accounts - in H256 format
use pezsp_runtime::traits::AccountIdConversion;

pub fn alice() -> H256 {
	H256::from_low_u64_be(1)
}

pub fn bob() -> H256 {
	H256::from_low_u64_be(2)
}

pub fn charlie() -> H256 {
	H256::from_low_u64_be(3)
}

// Two ordinary funded accounts. The pallet no longer knows about a presale or a founder --
// their allocations are genesis balances now -- but the tests still use them to check that
// releases move money between the pots and touch nothing else.
pub fn presale() -> H256 {
	H256::from_low_u64_be(10)
}

pub fn founder() -> H256 {
	H256::from_low_u64_be(11)
}

parameter_types! {
	pub RewardsChain: xcm::latest::Location =
		xcm::latest::Location::new(1, [xcm::latest::Junction::Teyrchain(1004)]);
	pub const RewardsPalletIndex: u8 = 91;
}

/// Records what the pallet tried to send instead of sending it.
///
/// A test that only checked storage could not tell "decided not to send" from "sent and it
/// vanished". Keeping the messages makes the difference visible.
pub struct RecordingXcmSender;

thread_local! {
	pub static SENT_XCM: core::cell::RefCell<Vec<(xcm::latest::Location, xcm::latest::Xcm<()>)>> =
		const { core::cell::RefCell::new(Vec::new()) };
	pub static SENDING_FAILS: core::cell::RefCell<bool> = const { core::cell::RefCell::new(false) };
}

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

/// Everything the pallet has tried to send so far.
pub fn sent_xcm() -> Vec<(xcm::latest::Location, xcm::latest::Xcm<()>)> {
	SENT_XCM.with(|q| q.borrow().clone())
}

/// Forget what was sent, so a test can assert about one stretch of blocks.
pub fn clear_sent_xcm() {
	SENT_XCM.with(|q| q.borrow_mut().clear());
}

/// Make every send fail, so a test can prove the release survives a lost report.
pub fn fail_sending(on: bool) {
	SENDING_FAILS.with(|f| *f.borrow_mut() = on);
}

impl pezpallet_pez_treasury::Config for Test {
	type Assets = Assets;
	type WeightInfo = weights::BizinikiwiWeight<Test>;
	type PezAssetId = PezAssetId;
	type TreasuryPalletId = PezTreasuryPalletId;
	type IncentivePotId = PezIncentivePotId;
	type GovernmentPotId = PezGovernmentPotId;
	// On the real chains this is the People chain's XCM origin. Root stands in here only
	// because the mock has no sibling chain to speak for the citizen register.
	type ActivationOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	// Likewise the People chain on the real runtimes; root stands in here.
	type GovernmentSpendOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	// The rewards chain on the real runtimes; root stands in here for the same reason.
	type IncentiveSpendOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type XcmSender = RecordingXcmSender;
	type RewardsChainLocation = RewardsChain;
	type RewardsPalletIndex = RewardsPalletIndex;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(alice(), 1_000_000_000_000_000),
			(bob(), 1_000_000_000_000_000),
			(charlie(), 1_000_000_000_000_000),
			(presale(), 1_000_000_000_000_000),
			(founder(), 1_000_000_000_000_000),
		],
		dev_accounts: None,
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);

		// Create PEZ asset
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			PezAssetId::get(),
			alice(),
			true,
			1
		));
	});
	ext
}

// Helper function to run to specific block
pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 1 {
			AllPalletsWithSystem::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		AllPalletsWithSystem::on_initialize(System::block_number());
		check_invariants();
	}
}

/// Assert the pallet's `try_state` invariant.
///
/// Called after every block the tests run, so the invariant is checked against real histories
/// -- backlogs, halvings, failed releases -- rather than in a test of its own that would only
/// ever see the states someone thought to write down. Without the feature this compiles away,
/// which is why the suite is run both ways.
pub fn check_invariants() {
	#[cfg(feature = "try-runtime")]
	{
		use pezframe_support::traits::TryState;
		AllPalletsWithSystem::try_state(
			System::block_number(),
			pezframe_support::traits::TryStateSelect::All,
		)
		.expect("try_state failed");
	}
}

/// Advance the block number without running any hooks.
///
/// `run_to_block` executes every block in between, which is right when the point is what the
/// hooks do, and unusable when the point is what happens forty-eight months from now -- that
/// is twenty million blocks. Tests that need to be somewhere far away jump, then run the few
/// blocks they actually care about.
pub fn jump_to_block(n: u64) {
	assert!(n >= System::block_number(), "cannot jump backwards");
	System::set_block_number(n);
}

/// Run `count` blocks, hooks and all.
pub fn run_blocks(count: u64) {
	run_to_block(System::block_number() + count);
}

/// Put the genesis treasury allocation where genesis would put it.
///
/// The pallet has no way to mint, which is the point of it; on a real chain the chain spec
/// credits this account before the first block. The tests have to stand in for that.
pub fn fund_treasury() {
	assert_ok!(Assets::mint_into(
		PezAssetId::get(),
		&treasury_account(),
		pezpallet_pez_treasury::TREASURY_ALLOCATION,
	));
}

/// PEZ held by one account.
pub fn pez_balance(account: H256) -> u128 {
	Assets::balance(PezAssetId::get(), account)
}

/// Total PEZ in existence. Nothing the pallet does may ever change this.
pub fn pez_total_supply() -> u128 {
	Assets::total_supply(PezAssetId::get())
}

// V3: Helper to assert balance - H256 account ile
pub fn assert_pez_balance(account: H256, expected: u128) {
	assert_eq!(
		Assets::balance(PezAssetId::get(), account),
		expected,
		"PEZ balance mismatch for account {:?}. Expected: {}, Got: {}",
		account,
		expected,
		Assets::balance(PezAssetId::get(), account)
	);
}

// V3: Helper functions - return H256
#[allow(dead_code)]
pub fn treasury_account() -> H256 {
	PezTreasuryPalletId::get().into_account_truncating()
}

#[allow(dead_code)]
pub fn incentive_pot_account() -> H256 {
	PezIncentivePotId::get().into_account_truncating()
}

#[allow(dead_code)]
pub fn government_pot_account() -> H256 {
	PezGovernmentPotId::get().into_account_truncating()
}

// V3: Debug helper
#[allow(dead_code)]
pub fn debug_pot_accounts() {
	println!("\n=== PalletId Debug ===");
	println!("Treasury bytes: {:?}", PezTreasuryPalletId::get().0);
	println!("Incentive bytes: {:?}", PezIncentivePotId::get().0);
	println!("Government bytes: {:?}", PezGovernmentPotId::get().0);
	println!("======================\n");
}
