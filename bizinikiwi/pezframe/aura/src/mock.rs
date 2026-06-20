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

//! Test utilities

#![cfg(test)]

use crate as pezpallet_aura;
use pezframe_support::{
	derive_impl, parameter_types,
	traits::{ConstU32, ConstU64, DisabledValidators},
};
use pezsp_consensus_aura::{ed25519::AuthorityId, AuthorityIndex};
use pezsp_runtime::{testing::UintAuthorityId, BuildStorage};

type Block = pezframe_system::mocking::MockBlock<Test>;

const SLOT_DURATION: u64 = 2;

pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Timestamp: pezpallet_timestamp,
		Aura: pezpallet_aura,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
}

impl pezpallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

parameter_types! {
	static DisabledValidatorTestValue: Vec<AuthorityIndex> = Default::default();
	pub static AllowMultipleBlocksPerSlot: bool = false;
}

pub struct MockDisabledValidators;

impl MockDisabledValidators {
	pub fn disable_validator(index: AuthorityIndex) {
		DisabledValidatorTestValue::mutate(|v| {
			if let Err(i) = v.binary_search(&index) {
				v.insert(i, index);
			}
		})
	}
}

impl DisabledValidators for MockDisabledValidators {
	fn is_disabled(index: AuthorityIndex) -> bool {
		DisabledValidatorTestValue::get().binary_search(&index).is_ok()
	}

	fn disabled_validators() -> Vec<u32> {
		DisabledValidatorTestValue::get()
	}
}

impl pezpallet_aura::Config for Test {
	type AuthorityId = AuthorityId;
	type DisabledValidators = MockDisabledValidators;
	type MaxAuthorities = ConstU32<10>;
	type AllowMultipleBlocksPerSlot = AllowMultipleBlocksPerSlot;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

fn build_ext(authorities: Vec<u64>) -> pezsp_io::TestExternalities {
	let mut storage = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pezpallet_aura::GenesisConfig::<Test> {
		authorities: authorities.into_iter().map(|a| UintAuthorityId(a).to_public_key()).collect(),
	}
	.assimilate_storage(&mut storage)
	.unwrap();
	storage.into()
}

pub fn build_ext_and_execute_test(authorities: Vec<u64>, test: impl FnOnce() -> ()) {
	let mut ext = build_ext(authorities);
	ext.execute_with(|| {
		test();
		Aura::do_try_state().expect("Storage invariants should hold")
	});
}
