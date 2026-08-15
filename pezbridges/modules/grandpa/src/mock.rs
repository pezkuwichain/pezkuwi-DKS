// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

// From construct_runtime macro
#![allow(clippy::from_over_into)]

use pezbp_header_pez_chain::ChainWithGrandpa;
use pezbp_runtime::{Chain, ChainId};
use pezframe_support::{
	construct_runtime, derive_impl, parameter_types, pezsp_runtime::StateVersion, traits::Hooks,
	weights::Weight,
};
use pezsp_core::sr25519::Signature;

pub type AccountId = u64;
pub type TestHeader = pezsp_runtime::testing::Header;
pub type TestNumber = u64;

type Block = pezframe_system::mocking::MockBlock<TestRuntime>;

pub const MAX_BRIDGED_AUTHORITIES: u32 = 5;

use crate as grandpa;

construct_runtime! {
	pub enum TestRuntime
	{
		System: pezframe_system::{Pezpallet, Call, Config<T>, Storage, Event<T>},
		Grandpa: grandpa::{Pezpallet, Call, Event<T>},
	}
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for TestRuntime {
	type Block = Block;
}

parameter_types! {
	pub const MaxFreeHeadersPerBlock: u32 = 2;
	pub const FreeHeadersInterval: u32 = 32;
	pub const HeadersToKeep: u32 = 5;
}

impl grandpa::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type BridgedChain = TestBridgedChain;
	type MaxFreeHeadersPerBlock = MaxFreeHeadersPerBlock;
	type FreeHeadersInterval = FreeHeadersInterval;
	type HeadersToKeep = HeadersToKeep;
	type WeightInfo = ();
}

#[derive(Debug)]
pub struct TestBridgedChain;

impl Chain for TestBridgedChain {
	const ID: ChainId = *b"tbch";

	type BlockNumber = pezframe_system::pezpallet_prelude::BlockNumberFor<TestRuntime>;
	type Hash = <TestRuntime as pezframe_system::Config>::Hash;
	type Hasher = <TestRuntime as pezframe_system::Config>::Hashing;
	type Header = TestHeader;

	type AccountId = AccountId;
	type Balance = u64;
	type Nonce = u64;
	type Signature = Signature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		unreachable!()
	}
	fn max_extrinsic_weight() -> Weight {
		unreachable!()
	}
}

impl ChainWithGrandpa for TestBridgedChain {
	const WITH_CHAIN_GRANDPA_PALLET_NAME: &'static str = "";
	const MAX_AUTHORITIES_COUNT: u32 = MAX_BRIDGED_AUTHORITIES;
	const REASONABLE_HEADERS_IN_JUSTIFICATION_ANCESTRY: u32 = 8;
	const MAX_MANDATORY_HEADER_SIZE: u32 = 256;
	const AVERAGE_HEADER_SIZE: u32 = 64;
}

/// Return test externalities to use in tests.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	pezsp_io::TestExternalities::new(Default::default())
}

/// Return test within default test externalities context.
pub fn run_test<T>(test: impl FnOnce() -> T) -> T {
	new_test_ext().execute_with(|| {
		let _ = Grandpa::on_initialize(0);
		test()
	})
}

/// Return test header with given number.
pub fn test_header(num: TestNumber) -> TestHeader {
	// We wrap the call to avoid explicit type annotations in our tests
	pezbp_test_utils::test_header(num)
}
