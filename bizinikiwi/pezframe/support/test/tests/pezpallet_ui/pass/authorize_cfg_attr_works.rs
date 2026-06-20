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

use pezframe_support::pezpallet_prelude::*;
use pezframe_system::pezpallet_prelude::*;

#[pezframe_support::pezpallet]
pub mod my_test {
	use super::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(core::marker::PhantomData<T>);

	#[cfg(all(target_endian = "little", target_endian = "big"))] // Never compiles.
	fn never_compiled() {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		#[cfg(all(target_endian = "little", target_endian = "big"))] // Never compiles.
		#[pezpallet::weight(Weight::zero())]
		#[pezpallet::authorize(|_source| {
			never_compiled(); // This will fail to compile if the authorize function is defined.
			Err(InvalidTransaction::Call.into())
		})]
		#[pezpallet::weight_of_authorize(Weight::zero())]
		#[pezpallet::call_index(0)]
		pub fn call_0(_: OriginFor<T>) -> DispatchResult {
			Ok(())
		}

		#[pezpallet::weight(Weight::zero())]
		#[pezpallet::authorize(|_source| { Err(InvalidTransaction::Call.into()) })]
		#[pezpallet::weight_of_authorize(Weight::zero())]
		#[pezpallet::call_index(1)]
		pub fn call_1(_: OriginFor<T>) -> DispatchResult {
			Ok(())
		}

		#[pezpallet::weight(Weight::zero())]
		#[pezpallet::call_index(2)]
		pub fn call_2(_: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}
}

fn main() {}
