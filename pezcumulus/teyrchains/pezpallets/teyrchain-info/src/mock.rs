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

//! Mock runtime for teyrchain-info pallet tests.

use crate as pezstaging_teyrchain_info;
use pezframe_support::{derive_impl, traits::ConstU32};
use pezsp_runtime::BuildStorage;

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		TeyrchainInfo: pezstaging_teyrchain_info,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountId = u64;
	type MaxConsumers = ConstU32<16>;
}

impl pezstaging_teyrchain_info::Config for Test {}

/// Build test externalities with default genesis (teyrchain_id = 100).
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = RuntimeGenesisConfig::default().build_storage().unwrap();
	pezsp_io::TestExternalities::new(t)
}

/// Build test externalities with a custom ParaId.
pub fn new_test_ext_with_id(para_id: u32) -> pezsp_io::TestExternalities {
	let genesis = RuntimeGenesisConfig {
		teyrchain_info: pezstaging_teyrchain_info::GenesisConfig {
			teyrchain_id: para_id.into(),
			_config: Default::default(),
		},
		..Default::default()
	};
	let t = genesis.build_storage().unwrap();
	pezsp_io::TestExternalities::new(t)
}
