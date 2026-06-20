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

#![cfg_attr(not(feature = "std"), no_std)]

use pezframe_support::{derive_impl, traits::ConstU32};

pub use pezpallet::*;

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	// The struct on which we build all of our Pezpallet logic.
	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	// Your Pezpallet's configuration trait, representing custom external types and interfaces.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	// The MEL requirement for bounded pallets is skipped by `dev_mode`.
	#[pezpallet::storage]
	type MyStorage<T: Config> = StorageValue<_, Vec<u8>>;

	// The Hasher requirement skipped by `dev_mode`.
	#[pezpallet::storage]
	pub type MyStorageMap<T: Config> = StorageMap<_, _, u32, u64>;

	#[pezpallet::storage]
	type MyStorageDoubleMap<T: Config> = StorageDoubleMap<_, _, u32, _, u64, u64>;

	#[pezpallet::storage]
	type MyCountedStorageMap<T: Config> = CountedStorageMap<_, _, u32, u64>;

	#[pezpallet::storage]
	pub type MyStorageMap2<T: Config> = StorageMap<Key = u32, Value = u64>;

	#[pezpallet::storage]
	type MyStorageDoubleMap2<T: Config> = StorageDoubleMap<Key1 = u32, Key2 = u64, Value = u64>;

	#[pezpallet::storage]
	type MyCountedStorageMap2<T: Config> = CountedStorageMap<Key = u32, Value = u64>;

	// Your Pezpallet's callable functions.
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		// No need to define a `weight` attribute here because of `dev_mode`.
		pub fn my_call(_origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}

	// Your Pezpallet's internal functions.
	impl<T: Config> Pezpallet<T> {}
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = pezsp_runtime::testing::H256;
	type Hashing = pezsp_runtime::traits::BlakeTwo256;
	type AccountId = u64;
	type Lookup = pezsp_runtime::traits::IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
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

pub type Header = pezsp_runtime::generic::Header<u32, pezsp_runtime::traits::BlakeTwo256>;
pub type Block = pezsp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = pezsp_runtime::generic::UncheckedExtrinsic<u32, RuntimeCall, (), ()>;

pezframe_support::construct_runtime!(
	pub struct Runtime
	{
		// Exclude part `Storage` in order not to check its metadata in tests.
		System: pezframe_system exclude_parts { Pezpallet, Storage },
		Example: pezpallet,
	}
);

impl pezpallet::Config for Runtime {}

fn main() {
	use pezframe_support::pezpallet_prelude::*;
	use pezsp_io::{
		hashing::{blake2_128, twox_128},
		TestExternalities,
	};
	use storage::unhashed;

	fn blake2_128_concat(d: &[u8]) -> Vec<u8> {
		let mut v = blake2_128(d).to_vec();
		v.extend_from_slice(d);
		v
	}

	TestExternalities::default().execute_with(|| {
		pezpallet::MyStorageMap::<Runtime>::insert(1, 2);
		let mut k = [twox_128(b"Example"), twox_128(b"MyStorageMap")].concat();
		k.extend(1u32.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u64>(&k), Some(2u64));
	});
}
