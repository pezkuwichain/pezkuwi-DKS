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

// ! A basic pezpallet to test it compiles along with a runtime using it when `pezframe_system` and
// `pezframe_support` are reexported by a `frame` crate.

use pezframe::deps::{pezframe_support, pezframe_system};

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	// The only valid syntax here is the following or
	// ```
	// pub trait Config: pezframe::deps::pezframe_system::Config {}
	// ```
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		#[serde(skip)]
		_config: core::marker::PhantomData<T>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {}
	}
}

#[cfg(test)]
// Dummy test to make sure a runtime would compile.
mod tests {
	use super::{
		pezframe_support::{construct_runtime, derive_impl},
		pezframe_system, pezpallet,
	};

	type Block = pezframe_system::mocking::MockBlock<Runtime>;

	impl crate::pezpallet::Config for Runtime {}

	#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
	impl pezframe_system::Config for Runtime {
		type Block = Block;
	}

	construct_runtime! {
		pub struct Runtime
		{
			System: pezframe_system,
			Pezpallet: pezpallet,
		}
	}
}
