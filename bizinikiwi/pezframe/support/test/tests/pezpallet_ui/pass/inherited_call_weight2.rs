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

pub trait WeightInfo {
	fn foo() -> Weight;
}

impl WeightInfo for () {
	fn foo() -> Weight {
		Weight::zero()
	}
}

#[pezframe_support::pezpallet]
pub mod parentheses {
	use super::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(core::marker::PhantomData<T>);

	// Crazy man just uses `()`, but it still works ;)
	#[pezpallet::call(weight(()))]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(0)]
		pub fn foo(_: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}
}

#[pezframe_support::pezpallet]
pub mod assign {
	use super::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(core::marker::PhantomData<T>);

	// Crazy man just uses `()`, but it still works ;)
	#[pezpallet::call(weight = ())]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(0)]
		pub fn foo(_: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}
}

fn main() {
}
