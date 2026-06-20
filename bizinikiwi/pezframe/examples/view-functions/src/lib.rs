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

//! This pezpallet demonstrates the use of the `pezpallet::view_functions` api for service
//! work.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod tests;

use pezframe_support::Parameter;
use scale_info::TypeInfo;

pub struct SomeType1;
impl From<SomeType1> for u64 {
	fn from(_t: SomeType1) -> Self {
		0u64
	}
}

pub trait SomeAssociation1 {
	type _1: Parameter + codec::MaxEncodedLen + TypeInfo;
}
impl SomeAssociation1 for u64 {
	type _1 = u64;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::error]
	pub enum Error<T> {}

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::storage]
	pub type SomeValue<T: Config> = StorageValue<_, u32>;

	#[pezpallet::storage]
	pub type SomeMap<T: Config> = StorageMap<_, Twox64Concat, u32, u32, OptionQuery>;

	#[pezpallet::view_functions]
	impl<T: Config> Pezpallet<T>
	where
		T::AccountId: From<SomeType1> + SomeAssociation1,
	{
		/// Query value with no input args.
		pub fn get_value() -> Option<u32> {
			SomeValue::<T>::get()
		}

		/// Query value with input args.
		pub fn get_value_with_arg(key: u32) -> Option<u32> {
			SomeMap::<T>::get(key)
		}
	}
}

#[pezframe_support::pezpallet]
pub mod pallet2 {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::error]
	pub enum Error<T, I = ()> {}

	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(PhantomData<(T, I)>);

	#[pezpallet::storage]
	pub type SomeValue<T: Config<I>, I: 'static = ()> = StorageValue<_, u32>;

	#[pezpallet::storage]
	pub type SomeMap<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Twox64Concat, u32, u32, OptionQuery>;

	#[pezpallet::view_functions]
	impl<T: Config<I>, I: 'static> Pezpallet<T, I>
	where
		T::AccountId: From<SomeType1> + SomeAssociation1,
	{
		/// Query value with no input args.
		pub fn get_value() -> Option<u32> {
			SomeValue::<T, I>::get()
		}

		/// Query value with input args.
		pub fn get_value_with_arg(key: u32) -> Option<u32> {
			SomeMap::<T, I>::get(key)
		}
	}
}
