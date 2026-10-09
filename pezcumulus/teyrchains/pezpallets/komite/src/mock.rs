// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate as pezpallet_komite;
use pezframe_support::{construct_runtime, derive_impl, parameter_types};
use pezframe_system::EnsureRoot;
use pezsp_runtime::BuildStorage;

type Block = pezframe_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Komite: pezpallet_komite,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
}

parameter_types! {
	pub const MaxMembers: u32 = 100;
}

impl pezpallet_komite::Config for Test {
	type CommitteeOrigin = EnsureRoot<u64>;
	type MaxMembers = MaxMembers;
	type OnCommittee = ();
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = RootIsTheCommitteeOrigin;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct RootIsTheCommitteeOrigin;
#[cfg(feature = "runtime-benchmarks")]
impl crate::BenchmarkSetup<RuntimeOrigin> for RootIsTheCommitteeOrigin {
	fn committee_origin() -> RuntimeOrigin {
		RuntimeOrigin::root()
	}
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
