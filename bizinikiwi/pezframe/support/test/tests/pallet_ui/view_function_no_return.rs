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

#[pezframe_support::pezpallet]
mod pezpallet {
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::config(with_default)]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::storage]
	pub type MyStorage<T> = StorageValue<_, u32>;

	#[pezpallet::view_functions]
	impl<T: Config> Pezpallet<T> {
		pub fn get_value() {
			MyStorage::<T>::set(0);
		}
	}
}

fn main() {}
