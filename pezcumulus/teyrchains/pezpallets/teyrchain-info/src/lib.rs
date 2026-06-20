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

//! Minimal Pezpallet that injects a TeyrchainId into Runtime storage from

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pezpallet::*;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use pezcumulus_primitives_core::ParaId;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {}

	#[pezpallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[serde(skip)]
		pub _config: core::marker::PhantomData<T>,
		pub teyrchain_id: ParaId,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { teyrchain_id: 100.into(), _config: Default::default() }
		}
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			TeyrchainId::<T>::put(self.teyrchain_id);
		}
	}

	#[pezpallet::type_value]
	pub(super) fn DefaultForTeyrchainId() -> ParaId {
		100.into()
	}

	#[pezpallet::storage]
	pub(super) type TeyrchainId<T: Config> =
		StorageValue<_, ParaId, ValueQuery, DefaultForTeyrchainId>;

	impl<T: Config> Get<ParaId> for Pezpallet<T> {
		fn get() -> ParaId {
			TeyrchainId::<T>::get()
		}
	}

	impl<T: Config> Pezpallet<T> {
		pub fn teyrchain_id() -> ParaId {
			TeyrchainId::<T>::get()
		}
	}
}
