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

//! This pezpallet demonstrates the use of the `pezpallet::task` api for service work.
#![cfg_attr(not(feature = "std"), no_std)]

use pezframe_support::dispatch::DispatchResult;
use pezframe_system::offchain::CreateBare;
#[cfg(feature = "experimental")]
use pezframe_system::offchain::SubmitTransaction;
// Re-export pezpallet items so that they can be accessed from the crate namespace.
pub use pezpallet::*;

pub mod mock;
pub mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[cfg(feature = "experimental")]
const LOG_TARGET: &str = "pezpallet-example-tasks";

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::error]
	pub enum Error<T> {
		/// The referenced task was not found.
		NotFound,
	}

	#[pezpallet::tasks_experimental]
	impl<T: Config> Pezpallet<T> {
		/// Add a pair of numbers into the totals and remove them.
		#[pezpallet::task_list(Numbers::<T>::iter_keys())]
		#[pezpallet::task_condition(|i| Numbers::<T>::contains_key(i))]
		#[pezpallet::task_weight(T::WeightInfo::add_number_into_total())]
		#[pezpallet::task_index(0)]
		pub fn add_number_into_total(_i: u32) -> DispatchResult {
			let i = _i;
			let v = Numbers::<T>::take(i).ok_or(Error::<T>::NotFound)?;
			Total::<T>::mutate(|(total_keys, total_values)| {
				*total_keys += i;
				*total_values += v;
			});
			Ok(())
		}
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		#[cfg(feature = "experimental")]
		fn offchain_worker(_block_number: BlockNumberFor<T>) {
			if let Some(key) = Numbers::<T>::iter_keys().next() {
				// Create a valid task
				let task = Task::<T>::AddNumberIntoTotal { _i: key };
				let runtime_task = <T as Config>::RuntimeTask::from(task);
				let call = pezframe_system::Call::<T>::do_task { task: runtime_task.into() };

				// Submit the task as an unsigned transaction
				let xt = <T as CreateBare<pezframe_system::Call<T>>>::create_bare(call.into());
				let res = SubmitTransaction::<T, pezframe_system::Call<T>>::submit_transaction(xt);
				match res {
					Ok(_) => log::info!(target: LOG_TARGET, "Submitted the task."),
					Err(e) => log::error!(target: LOG_TARGET, "Error submitting task: {:?}", e),
				}
			}
		}

		#[cfg(not(feature = "experimental"))]
		fn offchain_worker(_block_number: BlockNumberFor<T>) {}
	}

	#[pezpallet::config]
	pub trait Config: CreateBare<pezframe_system::Call<Self>> + pezframe_system::Config {
		type RuntimeTask: pezframe_support::traits::Task
			+ IsType<<Self as pezframe_system::Config>::RuntimeTask>
			+ From<Task<Self>>;
		type WeightInfo: WeightInfo;
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	/// Some running total.
	#[pezpallet::storage]
	pub type Total<T: Config> = StorageValue<_, (u32, u32), ValueQuery>;

	/// Numbers to be added into the total.
	#[pezpallet::storage]
	pub type Numbers<T: Config> = StorageMap<_, Twox64Concat, u32, u32, OptionQuery>;
}
