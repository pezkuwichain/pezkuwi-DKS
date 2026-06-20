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

pub use pezpallet::*;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use pezframe_support::pezpallet_prelude::*;

	// The struct on which we build all of our Pezpallet logic.
	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	// Your Pezpallet's configuration trait, representing custom external types and interfaces.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::storage]
	type MyStorage<T: Config> = StorageValue<_, Vec<u8>>;

	#[pezpallet::storage]
	type MyStorageMap<T: Config> = StorageMap<Key = u32, Value = u64>;

	#[pezpallet::storage]
	type MyStorageDoubleMap<T: Config> = StorageDoubleMap<Key1 = u32, Key2 = u64, Value = u64>;

	#[pezpallet::storage]
	type MyCountedStorageMap<T: Config> = CountedStorageMap<Key = u32, Value = u64>;

	// Your Pezpallet's internal functions.
	impl<T: Config> Pezpallet<T> {}
}

fn main() {}
