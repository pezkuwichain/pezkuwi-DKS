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

//! Pezcumulus extension pezpallet for AuRa
//!
//! This pezpallet extends the Bizinikiwi AuRa pezpallet to make it compatible with teyrchains. It
//! provides the [`Pezpallet`], the [`Config`] and the [`GenesisConfig`].
//!
//! It is also required that the teyrchain runtime uses the provided [`BlockExecutor`] to properly
//! check the constructed block on the relay chain.
//!
//! ```
//! # struct Runtime;
//! # struct Executive;
//! pezcumulus_pezpallet_teyrchain_system::register_validate_block! {
//!     Runtime = Runtime,
//!     BlockExecutor = pezcumulus_pezpallet_aura_ext::BlockExecutor::<Runtime, Executive>,
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

use pezframe_support::traits::{ExecuteBlock, FindAuthor};
use pezsp_application_crypto::RuntimeAppPublic;
use pezsp_consensus_aura::{digests::CompatibleDigestItem, Slot};
use pezsp_runtime::traits::{Block as BlockT, Header as HeaderT, LazyBlock};

pub mod consensus_hook;
pub mod migration;
#[cfg(test)]
mod test;

pub use consensus_hook::FixedVelocityConsensusHook;

type Aura<T> = pezpallet_aura::Pezpallet<T>;

pub use pezpallet::*;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	/// The configuration trait.
	#[pezpallet::config]
	pub trait Config: pezpallet_aura::Config + pezframe_system::Config {}

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(migration::STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		fn on_finalize(_: BlockNumberFor<T>) {
			// Update to the latest AuRa authorities.
			Authorities::<T>::put(pezpallet_aura::Authorities::<T>::get());
		}

		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			// Fetch the authorities once to get them into the storage proof of the PoV.
			Authorities::<T>::get();

			T::DbWeight::get().reads_writes(1, 0)
		}
	}

	/// Serves as cache for the authorities.
	///
	/// The authorities in AuRa are overwritten in `on_initialize` when we switch to a new session,
	/// but we require the old authorities to verify the seal when validating a PoV. This will
	/// always be updated to the latest AuRa authorities in `on_finalize`.
	#[pezpallet::storage]
	pub(crate) type Authorities<T: Config> = StorageValue<
		_,
		BoundedVec<T::AuthorityId, <T as pezpallet_aura::Config>::MaxAuthorities>,
		ValueQuery,
	>;

	/// Current relay chain slot paired with a number of authored blocks.
	///
	/// This is updated in [`FixedVelocityConsensusHook::on_state_proof`] with the current relay
	/// chain slot as provided by the relay chain state proof.
	#[pezpallet::storage]
	pub(crate) type RelaySlotInfo<T: Config> = StorageValue<_, (Slot, u32), OptionQuery>;

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		#[serde(skip)]
		pub _config: core::marker::PhantomData<T>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let authorities = pezpallet_aura::Authorities::<T>::get();
			Authorities::<T>::put(authorities);
		}
	}
}

/// The block executor used when validating a PoV at the relay chain.
///
/// When executing the block it will verify the block seal to ensure that the correct author created
/// the block.
pub struct BlockExecutor<T, I>(core::marker::PhantomData<(T, I)>);

impl<Block, T, I> ExecuteBlock<Block> for BlockExecutor<T, I>
where
	Block: BlockT,
	T: Config,
	I: ExecuteBlock<Block>,
{
	fn verify_and_remove_seal(block: &mut <Block as BlockT>::LazyBlock) {
		let header = block.header_mut();
		// We need to fetch the authorities before we execute the block, to get the authorities
		// before any potential update.
		let authorities = Authorities::<T>::get();

		let mut seal = None;
		header.digest_mut().logs.retain(|s| {
			let s =
				CompatibleDigestItem::<<T::AuthorityId as RuntimeAppPublic>::Signature>::as_aura_seal(s);
			match (s, seal.is_some()) {
				(Some(_), true) => panic!("Found multiple AuRa seal digests"),
				(None, _) => true,
				(Some(s), false) => {
					seal = Some(s);
					false
				},
			}
		});

		let seal = seal.expect("Could not find an AuRa seal digest!");

		let author = Aura::<T>::find_author(
			header.digest().logs().iter().filter_map(|d| d.as_pre_runtime()),
		)
		.expect("Could not find AuRa author index!");

		let pre_hash = header.hash();

		if !authorities
			.get(author as usize)
			.unwrap_or_else(|| {
				panic!("Invalid AuRa author index {} for authorities: {:?}", author, authorities)
			})
			.verify(&pre_hash, &seal)
		{
			panic!("Invalid AuRa seal");
		}
	}

	fn execute_verified_block(block: Block::LazyBlock) {
		I::execute_verified_block(block);
	}
}
