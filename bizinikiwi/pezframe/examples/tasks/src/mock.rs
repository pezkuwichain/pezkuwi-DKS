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

//! Mock runtime for `tasks-example` tests.
#![cfg(test)]

use crate::{self as pezpallet_example_tasks};
use pezframe_support::derive_impl;
use pezsp_runtime::testing::TestXt;

pub type AccountId = u32;
pub type Balance = u32;

type Block = pezframe_system::mocking::MockBlock<Runtime>;
pezframe_support::construct_runtime!(
	pub enum Runtime {
		System: pezframe_system,
		TasksExample: pezpallet_example_tasks,
	}
);

pub type Extrinsic = TestXt<RuntimeCall, ()>;

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type Block = Block;
}

impl<LocalCall> pezframe_system::offchain::CreateTransactionBase<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type RuntimeCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> pezframe_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: Self::RuntimeCall) -> Self::Extrinsic {
		Extrinsic::new_bare(call)
	}
}

impl pezpallet_example_tasks::Config for Runtime {
	type RuntimeTask = RuntimeTask;
	type WeightInfo = ();
}

pub fn advance_to(b: u64) {
	#[cfg(feature = "experimental")]
	use pezframe_support::traits::Hooks;
	while System::block_number() < b {
		System::set_block_number(System::block_number() + 1);
		#[cfg(feature = "experimental")]
		TasksExample::offchain_worker(System::block_number());
	}
}
