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

use pezframe_support::pezpallet_macros::*;

pub use pezpallet::*;

#[import_section(storages_dev)]
#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		pub fn my_call(_origin: OriginFor<T>) -> DispatchResult {
			MyStorageMap::<T>::insert(1, 2);
			Ok(())
		}
	}
}

fn main() {
}
