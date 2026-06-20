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

//! # bizinikiwi-test pezpallet
//!
//! Provides functionality used in unit-tests of numerous modules across bizinikiwi that require
//! functioning runtime. Some calls are allowed to be submitted as unsigned extrinsics, however most
//! of them requires signing. Refer to `pezpallet::Call` for further details.

use alloc::{vec, vec::Vec};
use pezframe_support::{pezpallet_prelude::*, storage};
use pezsp_core::sr25519::Public;
use pezsp_runtime::{
	traits::Hash,
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
	},
};

pub use self::pezpallet::*;

const LOG_TARGET: &str = "bizinikiwi_test_pallet";

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use super::*;
	use crate::TransferData;
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_core::storage::well_known_keys;
	use pezsp_runtime::{traits::BlakeTwo256, transaction_validity::TransactionPriority, Perbill};

	#[pezpallet::pezpallet]
	#[pezpallet::without_storage_info]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::storage]
	#[pezpallet::getter(fn authorities)]
	pub type Authorities<T> = StorageValue<_, Vec<Public>, ValueQuery>;

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub authorities: Vec<Public>,
		#[serde(skip)]
		pub _config: core::marker::PhantomData<T>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			<Authorities<T>>::put(self.authorities.clone());
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Legacy call used in transaction pool benchmarks.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(100)]
		pub fn bench_call(_origin: OriginFor<T>, _transfer: TransferData) -> DispatchResult {
			Ok(())
		}

		/// Implicitly fill a block body with some data.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(100)]
		pub fn include_data(origin: OriginFor<T>, _data: Vec<u8>) -> DispatchResult {
			pezframe_system::ensure_signed(origin)?;
			Ok(())
		}

		/// Put/delete some data from storage. Intended to use as an unsigned extrinsic.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(100)]
		pub fn storage_change(
			_origin: OriginFor<T>,
			key: Vec<u8>,
			value: Option<Vec<u8>>,
		) -> DispatchResult {
			match value {
				Some(value) => storage::unhashed::put_raw(&key, &value),
				None => storage::unhashed::kill(&key),
			}
			Ok(())
		}

		/// Write a key value pair to the offchain database.
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(100)]
		pub fn offchain_index_set(
			origin: OriginFor<T>,
			key: Vec<u8>,
			value: Vec<u8>,
		) -> DispatchResult {
			pezframe_system::ensure_signed(origin)?;
			pezsp_io::offchain_index::set(&key, &value);
			Ok(())
		}

		/// Remove a key and an associated value from the offchain database.
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(100)]
		pub fn offchain_index_clear(origin: OriginFor<T>, key: Vec<u8>) -> DispatchResult {
			pezframe_system::ensure_signed(origin)?;
			pezsp_io::offchain_index::clear(&key);
			Ok(())
		}

		/// Create an index for this call.
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(100)]
		pub fn indexed_call(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
			pezframe_system::ensure_signed(origin)?;
			let content_hash = pezsp_io::hashing::blake2_256(&data);
			let extrinsic_index: u32 =
				storage::unhashed::get(well_known_keys::EXTRINSIC_INDEX).unwrap();
			pezsp_io::transaction_index::index(extrinsic_index, data.len() as u32, content_hash);
			Ok(())
		}

		/// Deposit given digest items into the system storage. They will be included in a header
		/// during finalization.
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(100)]
		pub fn deposit_log_digest_item(
			_origin: OriginFor<T>,
			log: pezsp_runtime::generic::DigestItem,
		) -> DispatchResult {
			<pezframe_system::Pezpallet<T>>::deposit_log(log);
			Ok(())
		}

		/// This call is validated as `ValidTransaction` with given priority.
		#[pezpallet::call_index(7)]
		#[pezpallet::weight(100)]
		pub fn call_with_priority(
			_origin: OriginFor<T>,
			_priority: TransactionPriority,
		) -> DispatchResult {
			Ok(())
		}

		/// This call is validated as non-propagable `ValidTransaction`.
		#[pezpallet::call_index(8)]
		#[pezpallet::weight(100)]
		pub fn call_do_not_propagate(_origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}

		/// Fill the block weight up to the given ratio.
		#[pezpallet::call_index(9)]
		#[pezpallet::weight(*_ratio * T::BlockWeights::get().max_block)]
		pub fn fill_block(origin: OriginFor<T>, _ratio: Perbill) -> DispatchResult {
			ensure_signed(origin)?;
			Ok(())
		}

		/// Read X times from the state some data.
		///
		/// Panics if it can not read `X` times.
		#[pezpallet::call_index(10)]
		#[pezpallet::weight(100)]
		pub fn read(_origin: OriginFor<T>, count: u32) -> DispatchResult {
			Self::execute_read(count, false)
		}

		/// Read X times from the state some data and then panic!
		///
		/// Returns `Ok` if it didn't read anything.
		#[pezpallet::call_index(11)]
		#[pezpallet::weight(100)]
		pub fn read_and_panic(_origin: OriginFor<T>, count: u32) -> DispatchResult {
			Self::execute_read(count, true)
		}
	}

	impl<T: Config> Pezpallet<T> {
		fn execute_read(read: u32, panic_at_end: bool) -> DispatchResult {
			let mut next_key = vec![];
			for _ in 0..(read as usize) {
				if let Some(next) = pezsp_io::storage::next_key(&next_key) {
					// Read the value
					pezsp_io::storage::get(&next);

					next_key = next;
				} else {
					if panic_at_end {
						return Ok(());
					} else {
						panic!("Could not read {read} times from the state");
					}
				}
			}

			if panic_at_end {
				panic!("BYE")
			} else {
				Ok(())
			}
		}
	}

	#[pezpallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pezpallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			log::trace!(target: LOG_TARGET, "validate_unsigned {call:?}");
			match call {
				// Some tests do not need to be complicated with signer and nonce, some need
				// reproducible block hash (call signature can't be there).
				// Offchain testing requires storage_change.
				Call::deposit_log_digest_item { .. }
				| Call::storage_change { .. }
				| Call::read { .. }
				| Call::read_and_panic { .. } => Ok(ValidTransaction {
					provides: vec![BlakeTwo256::hash_of(&call).encode()],
					..Default::default()
				}),
				_ => Err(TransactionValidityError::Invalid(InvalidTransaction::Call)),
			}
		}
	}
}

pub fn validate_runtime_call<T: pezpallet::Config>(
	call: &pezpallet::Call<T>,
) -> TransactionValidity {
	log::trace!(target: LOG_TARGET, "validate_runtime_call {call:?}");
	match call {
		Call::call_do_not_propagate {} => {
			Ok(ValidTransaction { propagate: false, ..Default::default() })
		},
		Call::call_with_priority { priority } => {
			Ok(ValidTransaction { priority: *priority, ..Default::default() })
		},
		_ => Ok(Default::default()),
	}
}
