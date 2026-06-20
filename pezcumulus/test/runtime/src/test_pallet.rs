// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.

// Pezcumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezcumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezcumulus.  If not, see <http://www.gnu.org/licenses/>.

/// A special pezpallet that exposes dispatchables that are only useful for testing.
pub use pezpallet::*;

/// Some key that we set in genesis and only read in [`TestOnRuntimeUpgrade`] to ensure that
/// [`OnRuntimeUpgrade`] works as expected.
pub const TEST_RUNTIME_UPGRADE_KEY: &[u8] = b"+test_runtime_upgrade_key+";

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use crate::test_pallet::TEST_RUNTIME_UPGRADE_KEY;
	use alloc::vec;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config:
		pezframe_system::Config + pezcumulus_pezpallet_teyrchain_system::Config
	{
	}

	/// A simple storage map for testing purposes.
	#[pezpallet::storage]
	pub type TestMap<T: Config> = StorageMap<_, Twox64Concat, u32, (), ValueQuery>;

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// A test dispatchable for setting a custom head data in `validate_block`.
		#[pezpallet::weight(0)]
		pub fn set_custom_validation_head_data(
			_: OriginFor<T>,
			custom_header: alloc::vec::Vec<u8>,
		) -> DispatchResult {
			pezcumulus_pezpallet_teyrchain_system::Pezpallet::<T>::set_custom_validation_head_data(
				custom_header,
			);
			Ok(())
		}

		/// A dispatchable that first reads two values from two different child tries, asserts they
		/// are the expected values (if the values exist in the state) and then writes two different
		/// values to these child tries.
		#[pezpallet::weight(0)]
		pub fn read_and_write_child_tries(_: OriginFor<T>) -> DispatchResult {
			let key = &b"hello"[..];
			let first_trie = &b"first"[..];
			let second_trie = &b"second"[..];
			let first_value = "world1".encode();
			let second_value = "world2".encode();

			if let Some(res) = pezsp_io::default_child_storage::get(first_trie, key) {
				assert_eq!(first_value, res);
			}
			if let Some(res) = pezsp_io::default_child_storage::get(second_trie, key) {
				assert_eq!(second_value, res);
			}

			pezsp_io::default_child_storage::set(first_trie, key, &first_value);
			pezsp_io::default_child_storage::set(second_trie, key, &second_value);

			Ok(())
		}

		/// Reads a key and writes a big value under this key.
		///
		/// At genesis this `key` is empty and thus, will only be set in consequent blocks.
		pub fn read_and_write_big_value(_: OriginFor<T>) -> DispatchResult {
			let key = &b"really_huge_value"[..];
			pezsp_io::storage::get(key);
			pezsp_io::storage::set(key, &vec![0u8; 1024 * 1024 * 5]);

			Ok(())
		}

		/// Stores `()` in `TestMap` for keys from 0 up to `max_key`.
		#[pezpallet::weight(0)]
		pub fn store_values_in_map(_: OriginFor<T>, max_key: u32) -> DispatchResult {
			for i in 0..=max_key {
				TestMap::<T>::insert(i, ());
			}
			Ok(())
		}

		/// Removes the value associated with `key` from `TestMap`.
		#[pezpallet::weight(0)]
		pub fn remove_value_from_map(_: OriginFor<T>, key: u32) -> DispatchResult {
			TestMap::<T>::remove(key);
			Ok(())
		}
	}

	#[derive(pezframe_support::DefaultNoBound)]
	#[pezpallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[serde(skip)]
		pub _config: core::marker::PhantomData<T>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			pezsp_io::storage::set(TEST_RUNTIME_UPGRADE_KEY, &[1, 2, 3, 4]);
		}
	}
}
