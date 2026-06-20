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

pub use pezpallet::*;

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use pezframe_support::{
		dispatch::{Pays, PostDispatchInfo},
		ensure,
		pezpallet_prelude::DispatchResultWithPostInfo,
		weights::Weight,
	};
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Dummy function that overcharges the predispatch weight, allowing us to test the correct
		/// values of [`ContractResult::gas_consumed`] and [`ContractResult::gas_required`] in
		/// tests.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(*pre_charge)]
		pub fn overestimate_pre_charge(
			origin: OriginFor<T>,
			pre_charge: Weight,
			actual_weight: Weight,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			ensure!(pre_charge.any_gt(actual_weight), "pre_charge must be > actual_weight");
			Ok(PostDispatchInfo { actual_weight: Some(actual_weight), pays_fee: Pays::Yes })
		}
	}
}
