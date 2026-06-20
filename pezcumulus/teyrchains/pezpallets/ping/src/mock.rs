// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

//! Mock runtime for testing the ping pezpallet.

pub use crate as pezcumulus_ping;
use crate::weights::WeightInfo;
use pezframe_support::{derive_impl, traits::ConstU32, weights::Weight};
use pezsp_runtime::{traits::IdentityLookup, BuildStorage};
use xcm::latest::prelude::*;

type AccountId = u64;
type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		CumulusXcm: pezcumulus_pezpallet_xcm,
		Ping: pezcumulus_ping,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Block = Block;
	type Hash = pezsp_core::H256;
	type Hashing = pezsp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pezcumulus_pezpallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	/// Use the blanket `()` implementation of `ExecuteXcm` which always returns Unimplemented.
	/// This is sufficient for the ping pallet tests since we never actually execute incoming XCM.
	type XcmExecutor = ();
}

/// A mock XCM sender that always succeeds.
pub struct MockXcmSender;
impl SendXcm for MockXcmSender {
	type Ticket = ();

	fn validate(
		_dest: &mut Option<Location>,
		_msg: &mut Option<Xcm<()>>,
	) -> SendResult<Self::Ticket> {
		Ok(((), Assets::new()))
	}

	fn deliver(_ticket: Self::Ticket) -> Result<XcmHash, SendError> {
		Ok([0u8; 32])
	}
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type XcmSender = MockXcmSender;
	type WeightInfo = PingWeightInfo;
}

/// Zero-weight implementation for tests.
pub struct PingWeightInfo;
impl WeightInfo for PingWeightInfo {
	fn start(_s: u32) -> Weight {
		Weight::zero()
	}
	fn start_many(_n: u32) -> Weight {
		Weight::zero()
	}
	fn stop() -> Weight {
		Weight::zero()
	}
	fn stop_all() -> Weight {
		Weight::zero()
	}
	fn ping(_s: u32) -> Weight {
		Weight::zero()
	}
	fn pong(_s: u32) -> Weight {
		Weight::zero()
	}
}

/// Build test externalities.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = RuntimeGenesisConfig::default().build_storage().unwrap().into();
	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Build bench externalities (no block number set).
#[cfg(feature = "runtime-benchmarks")]
pub fn new_bench_ext() -> pezsp_io::TestExternalities {
	RuntimeGenesisConfig::default().build_storage().unwrap().into()
}
