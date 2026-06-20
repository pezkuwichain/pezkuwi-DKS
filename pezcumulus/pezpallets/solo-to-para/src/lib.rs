// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

extern crate alloc;

use alloc::vec::Vec;
use pezcumulus_pezpallet_teyrchain_system as teyrchain_system;
use pezframe_support::pezpallet_prelude::*;
use pezframe_system::pezpallet_prelude::*;
use pezkuwi_primitives::PersistedValidationData;
pub use pezpallet::*;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	#[pezpallet::config]
	pub trait Config:
		pezframe_system::Config + teyrchain_system::Config + pezpallet_sudo::Config
	{
		#[allow(deprecated)]
		type RuntimeEvent: From<Event> + IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
	}

	#[pezpallet::pezpallet]
	#[pezpallet::without_storage_info]
	pub struct Pezpallet<T>(_);

	/// In case of a scheduled migration, this storage field contains the custom head data to be
	/// applied.
	#[pezpallet::storage]
	pub(super) type PendingCustomValidationHeadData<T: Config> =
		StorageValue<_, Vec<u8>, OptionQuery>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event {
		/// The custom validation head data has been scheduled to apply.
		CustomValidationHeadDataStored,
		/// The custom validation head data was applied as of the contained relay chain block
		/// number.
		CustomValidationHeadDataApplied,
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// CustomHeadData is not stored in storage.
		NoCustomHeadData,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(0)]
		#[pezpallet::weight({0})]
		pub fn schedule_migration(
			origin: OriginFor<T>,
			code: Vec<u8>,
			head_data: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;

			teyrchain_system::Pezpallet::<T>::schedule_code_upgrade(code)?;
			Self::store_pending_custom_validation_head_data(head_data);
			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// Set a custom head data that should only be applied when upgradeGoAheadSignal from
		/// the Relay Chain is GoAhead
		fn store_pending_custom_validation_head_data(head_data: Vec<u8>) {
			PendingCustomValidationHeadData::<T>::put(head_data);
			Self::deposit_event(Event::CustomValidationHeadDataStored);
		}

		/// Set pending custom head data as head data that will be returned by `validate_block`. on
		/// the relay chain.
		fn set_pending_custom_validation_head_data() {
			if let Some(head_data) = <PendingCustomValidationHeadData<T>>::take() {
				teyrchain_system::Pezpallet::<T>::set_custom_validation_head_data(head_data);
				Self::deposit_event(Event::CustomValidationHeadDataApplied);
			}
		}
	}

	impl<T: Config> teyrchain_system::OnSystemEvent for Pezpallet<T> {
		fn on_validation_data(_data: &PersistedValidationData) {}
		fn on_validation_code_applied() {
			crate::Pezpallet::<T>::set_pending_custom_validation_head_data();
		}
	}
}
