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

use pezframe_support::derive_impl;
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezsp_core::sr25519;
use pezsp_runtime::{
	generic,
	traits::{BlakeTwo256, Verify},
};

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {}

	#[pezpallet::storage]
	#[pezpallet::unbounded]
	pub type AppendableDM<T: Config> =
		StorageDoubleMap<_, Identity, u32, Identity, BlockNumberFor<T>, Vec<u32>>;

	#[pezpallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub t: Vec<(u32, BlockNumberFor<T>, Vec<u32>)>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { t: Default::default() }
		}
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (k1, k2, v) in &self.t {
				<AppendableDM<T>>::insert(k1, k2, v);
			}
		}
	}
}

pub type BlockNumber = u32;
pub type Signature = sr25519::Signature;
pub type AccountId = <Signature as Verify>::Signer;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<u32, RuntimeCall, Signature, ()>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

pezframe_support::construct_runtime!(
	pub enum Test

	{
		System: pezframe_system,
		MyPallet: pezpallet,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type Block = Block;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletInfo = PalletInfo;
	type OnSetCode = ();
}

impl pezpallet::Config for Test {}

#[test]
fn init_genesis_config() {
	pezpallet::GenesisConfig::<Test>::default();
}
