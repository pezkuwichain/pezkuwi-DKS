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

//! Remark storage pezpallet. Indexes remarks and stores them off chain.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

extern crate alloc;

use alloc::vec::Vec;

// Re-export pezpallet items so that they can be accessed from the crate namespace.
pub use pezpallet::*;
pub use weights::WeightInfo;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
		/// Weight information for extrinsics in this pezpallet.
		type WeightInfo: WeightInfo;
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// Attempting to store empty data.
		Empty,
		/// Attempted to call `store` outside of block execution.
		BadContext,
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Index and store data off chain.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::store(remark.len() as u32))]
		pub fn store(origin: OriginFor<T>, remark: Vec<u8>) -> DispatchResultWithPostInfo {
			ensure!(!remark.is_empty(), Error::<T>::Empty);
			let sender = ensure_signed(origin)?;
			let content_hash = pezsp_io::hashing::blake2_256(&remark);
			let extrinsic_index = <pezframe_system::Pezpallet<T>>::extrinsic_index()
				.ok_or_else(|| Error::<T>::BadContext)?;
			pezsp_io::transaction_index::index(extrinsic_index, remark.len() as u32, content_hash);
			Self::deposit_event(Event::Stored { sender, content_hash: content_hash.into() });
			Ok(().into())
		}
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Stored data off chain.
		Stored { sender: T::AccountId, content_hash: pezsp_core::H256 },
	}
}
