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

use pezframe_benchmarking::v2::*;

#[pezframe_support::pezpallet]
mod pezpallet {
	use pezframe_system::pezpallet_prelude::*;
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(Weight::default())]
		pub fn call_1(_origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}
}

pub use pezpallet::*;

#[benchmarks]
mod benches {
	use super::*;
	use pezframe_support::traits::OriginTrait;

	#[benchmark]
	fn call_1() {
		let origin = 3u8;
		#[extrinsic_call]
		_(origin);
	}
}

fn main() {}
