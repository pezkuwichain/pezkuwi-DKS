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

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use pezframe_support::{ensure, pezpallet_prelude::DispatchResult};

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(core::marker::PhantomData<T>);

    #[pezpallet::tasks_experimental]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::task_index(0)]
		#[pezpallet::task_condition(|i, j| i == 0u32 && j == 2u64)]
		#[pezpallet::task_list(vec![(0u32, 2u64), (2u32, 4u64)].iter())]
		#[pezpallet::task_weight(0.into())]
		fn foo(i: u32, j: u64) -> DispatchResult {
			ensure!(i == 0, "i must be 0");
			ensure!(j == 2, "j must be 2");
			Ok(())
		}
	}
}

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet_with_instance {
	use pezframe_support::pezpallet_prelude::{ValueQuery, StorageValue};

	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(_);

	#[pezpallet::storage]
	pub type SomeStorage<T, I = ()> = StorageValue<_, u32, ValueQuery>;

    #[pezpallet::tasks_experimental]
	impl<T: Config<I>, I> Pezpallet<T, I> {
		#[pezpallet::task_index(0)]
		#[pezpallet::task_condition(|i, j| i == 0u32 && j == 2u64)]
		#[pezpallet::task_list(vec![(0u32, 2u64), (2u32, 4u64)].iter())]
		#[pezpallet::task_weight(0.into())]
		fn foo(_i: u32, _j: u64) -> pezframe_support::pezpallet_prelude::DispatchResult {
			<SomeStorage<T, I>>::get();
			Ok(())
		}
	}
}

fn main() {
}
