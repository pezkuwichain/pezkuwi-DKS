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

//! End-to-end testing pezpallet for PoV benchmarking. Should only be deployed in a  testing
//! runtime.

#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
mod tests;
mod weights;

extern crate alloc;

pub use pezpallet::*;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use alloc::vec::Vec;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
	}

	#[pezpallet::storage]
	pub(crate) type Value<T: Config> = StorageValue<Value = u32, QueryKind = OptionQuery>;

	#[pezpallet::storage]
	pub(crate) type Value2<T: Config> = StorageValue<Value = u32, QueryKind = OptionQuery>;

	/// A value without a MEL bound.
	#[pezpallet::storage]
	#[pezpallet::unbounded]
	pub(crate) type UnboundedValue<T: Config> =
		StorageValue<Value = Vec<u8>, QueryKind = OptionQuery>;

	/// A value with a MEL bound of 32 byte.
	#[pezpallet::storage]
	pub(crate) type BoundedValue<T: Config> =
		StorageValue<Value = BoundedVec<u8, ConstU32<32>>, QueryKind = OptionQuery>;

	/// 4MiB value.
	#[pezpallet::storage]
	pub(crate) type LargeValue<T: Config> =
		StorageValue<Value = BoundedVec<u8, ConstU32<{ 1 << 22 }>>, QueryKind = OptionQuery>;

	#[pezpallet::storage]
	pub(crate) type LargeValue2<T: Config> =
		StorageValue<Value = BoundedVec<u8, ConstU32<{ 1 << 22 }>>, QueryKind = OptionQuery>;

	/// A map with a maximum of 1M entries.
	#[pezpallet::storage]
	pub(crate) type Map1M<T: Config> = StorageMap<
		Hasher = Blake2_256,
		Key = u32,
		Value = u32,
		QueryKind = OptionQuery,
		MaxValues = ConstU32<1_000_000>,
	>;

	/// A map with a maximum of 16M entries.
	#[pezpallet::storage]
	pub(crate) type Map16M<T: Config> = StorageMap<
		Hasher = Blake2_256,
		Key = u32,
		Value = u32,
		QueryKind = OptionQuery,
		MaxValues = ConstU32<16_000_000>,
	>;

	#[pezpallet::storage]
	pub(crate) type DoubleMap1M<T: Config> = StorageDoubleMap<
		Hasher1 = Blake2_256,
		Hasher2 = Blake2_256,
		Key1 = u32,
		Key2 = u32,
		Value = u32,
		QueryKind = OptionQuery,
		MaxValues = ConstU32<1_000_000>,
	>;

	#[pezpallet::storage]
	#[pezpallet::unbounded]
	pub(crate) type UnboundedMap<T: Config> =
		StorageMap<Hasher = Blake2_256, Key = u32, Value = Vec<u32>, QueryKind = OptionQuery>;

	#[pezpallet::storage]
	#[pezpallet::unbounded]
	pub(crate) type UnboundedMap2<T: Config> =
		StorageMap<Hasher = Blake2_256, Key = u32, Value = Vec<u32>, QueryKind = OptionQuery>;

	#[pezpallet::storage]
	#[pezpallet::unbounded]
	pub(crate) type UnboundedMapTwox<T: Config> =
		StorageMap<Hasher = Twox64Concat, Key = u32, Value = Vec<u32>, QueryKind = OptionQuery>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		TestEvent,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(0)]
		#[pezpallet::weight({0})]
		pub fn emit_event(_origin: OriginFor<T>) -> DispatchResult {
			Self::deposit_event(Event::TestEvent);
			Ok(())
		}

		#[pezpallet::call_index(1)]
		#[pezpallet::weight({0})]
		pub fn noop(_origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}
}
