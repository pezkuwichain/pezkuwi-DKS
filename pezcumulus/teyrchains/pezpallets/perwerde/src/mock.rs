// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate as pezpallet_perwerde;
use pezframe_support::{
	construct_runtime, parameter_types,
	traits::{ConstU128, ConstU16, ConstU32, ConstU64, Everything, SortedMembers},
};
use pezframe_system::EnsureRoot;
use pezsp_core::H256;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

// Define the basic types.
pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type Block = pezframe_system::mocking::MockBlock<Test>;

// Set up our test runtime.
construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		Perwerde: pezpallet_perwerde,
		Council: pezpallet_collective::<Instance1>,
	}
);

// Implementation for pezframe_system.
impl pezframe_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type RuntimeTask = ();
	type ExtensionsWeightInfo = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

// Implementation for pezpallet_balances.
impl pezpallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<1>;
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const MaxCourseNameLength: u32 = 100;
	pub const MaxCourseDescLength: u32 = 500;
	pub const MaxCourseLinkLength: u32 = 200;
	pub const MaxStudentsPerCourse: u32 = 100; // Reduced for test performance
	pub const MaxCoursesPerStudent: u32 = 50;  // Max courses a student can enroll in
	pub const MaxPointsPerCourse: u32 = 1000;  // Max points per course completion
}

// --- THE DEFINITIVE SOLUTION STARTS HERE ---

// We create our own custom authority provider to test AdminOrigin.
use pezframe_system::EnsureSignedBy;

// This struct manually implements the `SortedMembers` trait that the compiler requires.
// This eliminates the need for external, version-dependent tools.
pub struct TestAdminProvider;
impl SortedMembers<AccountId> for TestAdminProvider {
	fn sorted_members() -> Vec<AccountId> {
		// For tests, we authorize only the account with ID 0 as admin.
		vec![0]
	}
}

impl pezpallet_perwerde::Config for Test {
	// We bind AdminOrigin to our own provider, which accepts only 0 as admin.
	type AdminOrigin = EnsureSignedBy<TestAdminProvider, AccountId>;
	type WeightInfo = ();
	type MaxCourseNameLength = MaxCourseNameLength;
	type MaxCourseDescLength = MaxCourseDescLength;
	type MaxCourseLinkLength = MaxCourseLinkLength;
	type MaxStudentsPerCourse = MaxStudentsPerCourse;
	type MaxCoursesPerStudent = MaxCoursesPerStudent;
	type MaxPointsPerCourse = MaxPointsPerCourse;
	type TrustScoreUpdater = ();
}

// Mock setup for the Council pallet (kept because it is required in construct_runtime)
use pezpallet_collective::Instance1;
parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 5 * 60; // 5 minutes
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
	pub MaxProposalWeight: pezframe_support::weights::Weight = pezframe_support::weights::Weight::from_parts(1_000_000_000, 0);
}
impl pezpallet_collective::Config<Instance1> for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pezpallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
	type DisapproveOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Consideration = ();
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	// We no longer need to set up the genesis of `pezpallet-collective` because our test is no
	// longer dependent on it.
	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
