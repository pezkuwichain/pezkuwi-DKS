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

//! Test environment for NIS pezpallet.

use pezframe::{runtime::prelude::*, testing_prelude::*, traits::StorageMapShim};

use crate::{self as pezpallet_nis, *};

pub type Balance = u64;

type Block = pezframe_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pezpallet.
#[frame_construct_runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeError,
		RuntimeEvent,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeOrigin,
		RuntimeTask
	)]
	pub struct Test;

	#[runtime::pezpallet_index(0)]
	pub type System = pezframe_system;
	#[runtime::pezpallet_index(1)]
	pub type Balances = pezpallet_balances<Instance1>;
	#[runtime::pezpallet_index(2)]
	pub type NisBalances = pezpallet_balances<Instance2>;
	#[runtime::pezpallet_index(3)]
	pub type Nis = pezpallet_nis;
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<Balance>;
}

impl pezpallet_balances::Config<pezpallet_balances::Instance1> for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU64<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ConstU32<1>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

impl pezpallet_balances::Config<pezpallet_balances::Instance2> for Test {
	type Balance = u128;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = StorageMapShim<
		pezpallet_balances::Account<Test, pezpallet_balances::Instance2>,
		u64,
		pezpallet_balances::AccountData<u128>,
	>;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

parameter_types! {
	pub IgnoredIssuance: Balance = Balances::total_balance(&0); // Account zero is ignored.
	pub const NisPalletId: PalletId = PalletId(*b"py/nis  ");
	pub static Target: Perquintill = Perquintill::zero();
	pub const MinReceipt: Perquintill = Perquintill::from_percent(1);
	pub const ThawThrottle: (Perquintill, u64) = (Perquintill::from_percent(25), 5);
	pub static MaxIntakeWeight: Weight = Weight::from_parts(2_000_000_000_000, 0);
}

ord_parameter_types! {
	pub const One: u64 = 1;
}

impl pezpallet_nis::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type PalletId = NisPalletId;
	type Currency = Balances;
	type CurrencyBalance =
		<Self as pezpallet_balances::Config<pezpallet_balances::Instance1>>::Balance;
	type FundOrigin = pezframe_system::EnsureSigned<Self::AccountId>;
	type Deficit = ();
	type IgnoredIssuance = IgnoredIssuance;
	type Counterpart = NisBalances;
	type CounterpartAmount = WithMaximumOf<ConstU128<21_000_000u128>>;
	type Target = Target;
	type QueueCount = ConstU32<3>;
	type MaxQueueLen = ConstU32<3>;
	type FifoQueueLen = ConstU32<1>;
	type BasePeriod = ConstU64<3>;
	type MinBid = ConstU64<2>;
	type IntakePeriod = ConstU64<2>;
	type MaxIntakeWeight = MaxIntakeWeight;
	type MinReceipt = MinReceipt;
	type ThawThrottle = ThawThrottle;
	type RuntimeHoldReason = RuntimeHoldReason;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = ();
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pezpallet_balances::GenesisConfig::<Test, pezpallet_balances::Instance1> {
		balances: vec![(1, 100), (2, 100), (3, 100), (4, 100)],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup, but without any balances.
#[cfg(feature = "runtime-benchmarks")]
pub fn new_test_ext_empty() -> pezsp_io::TestExternalities {
	pezframe_system::GenesisConfig::<Test>::default()
		.build_storage()
		.unwrap()
		.into()
}
