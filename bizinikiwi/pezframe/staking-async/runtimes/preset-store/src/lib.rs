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

//! Simple pezpallet that stores the preset that was used to generate the genesis state in the
//! state.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pezpallet::*;

#[pezframe::pezpallet]
pub mod pezpallet {
	extern crate alloc;
	use pezframe::prelude::*;

	#[pezpallet::storage]
	#[pezpallet::getter(fn preset)]
	#[pezpallet::unbounded]
	pub type Preset<T: Config> = StorageValue<_, alloc::string::String, OptionQuery>;

	#[pezpallet::genesis_config]
	#[derive(DefaultNoBound, DebugNoBound, CloneNoBound, PartialEqNoBound, EqNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub preset: alloc::string::String,
		pub _marker: core::marker::PhantomData<T>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Preset::<T>::put(self.preset.clone());
		}
	}

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);
}
