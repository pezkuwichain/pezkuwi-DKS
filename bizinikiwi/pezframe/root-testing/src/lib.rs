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

//! # Root Testing Pezpallet
//!
//! Pezpallet that contains extrinsics that can be useful in testing.
//!
//! NOTE: This pezpallet should only be used for testing purposes and should not be used in
//! production runtimes!

#![cfg_attr(not(feature = "std"), no_std)]

use pezframe_support::{dispatch::DispatchResult, pezsp_runtime::Perbill};

pub use pezpallet::*;

#[pezframe_support::pezpallet(dev_mode)]
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
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event dispatched when the trigger_defensive extrinsic is called.
		DefensiveTestCall,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// A dispatch that will fill the block weight up to the given ratio.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(*_ratio * T::BlockWeights::get().max_block)]
		pub fn fill_block(origin: OriginFor<T>, _ratio: Perbill) -> DispatchResult {
			ensure_root(origin)?;
			Ok(())
		}

		#[pezpallet::call_index(1)]
		#[pezpallet::weight(0)]
		pub fn trigger_defensive(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;
			pezframe_support::defensive!("root_testing::trigger_defensive was called.");
			Self::deposit_event(Event::DefensiveTestCall);
			Ok(())
		}
	}
}
