// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate as pezpallet_token_wrapper;
use pezframe_support::{
	construct_runtime, parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128, ConstU32, Everything},
	PalletId,
};
use pezframe_system as system;
use pezsp_core::H256;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

pub type AccountId = u64;
pub type Balance = u128;
pub type AssetId = u32;

// Configure a mock runtime to test the pezpallet.
construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		Balances: pezpallet_balances,
		Assets: pezpallet_assets,
		TokenWrapper: pezpallet_token_wrapper,
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
}

impl pezpallet_balances::Config for Test {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
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

parameter_types! {
	pub const AssetDeposit: Balance = 100;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pezpallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<pezframe_system::EnsureSigned<AccountId>>;
	type ForceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<1>;
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

parameter_types! {
	pub const TokenWrapperPalletId: PalletId = PalletId(*b"py/wrper");
	pub const WrapperAssetId: u32 = 0;
}

impl pezpallet_token_wrapper::Config for Test {
	type WeightInfo = crate::weights::BizinikiwiWeight<Test>;
	type Currency = Balances;
	type AssetId = AssetId;
	type Assets = Assets;
	type PalletId = TokenWrapperPalletId;
	type WrapperAssetId = WrapperAssetId;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	use pezframe_support::assert_ok;

	let mut storage = system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 10000), (2, 5000), (3, 3000)],
		dev_accounts: None,
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(storage);
	ext.execute_with(|| {
		System::set_block_number(1);

		// Create wHEZ asset (Asset ID 0)
		assert_ok!(Assets::force_create(
			RuntimeOrigin::root(),
			0,                          // Asset ID
			TokenWrapper::account_id(), // Owner = pezpallet account
			true,                       // is_sufficient
			1,                          // min_balance
		));
	});
	ext
}
