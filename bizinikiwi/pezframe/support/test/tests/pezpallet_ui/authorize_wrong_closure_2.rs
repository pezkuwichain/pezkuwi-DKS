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
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::config(with_default)]
	pub trait Config: pezframe_system::Config {
		type WeightInfo: WeightInfo;
	}

	pub trait WeightInfo {
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::call(weight = T::WeightInfo)]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::authorize(|_, _: u8| -> bool { true })]
		#[pezpallet::weight_of_authorize(Weight::zero())]
		#[pezpallet::weight(Weight::zero())]
		#[pezpallet::call_index(0)]
		pub fn call1(origin: OriginFor<T>, a: u32) -> DispatchResult {
			let _ = origin;
			let _ = a;
			Ok(())
		}
	}
}

fn main() {}
