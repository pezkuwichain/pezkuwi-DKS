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

//! Benchmarking setup for pezpallet-session.
#![cfg(feature = "runtime-benchmarks")]

use alloc::vec::Vec;

use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;
use pezpallet_session::*;
pub struct Pezpallet<T: Config>(pezpallet_session::Pezpallet<T>);
pub trait Config: pezpallet_session::Config {
	/// Generate a session key and a proof of ownership.
	///
	/// The given `owner` is the account that will call `set_keys` using the returned session keys
	/// and proof. This means that the proof should prove the ownership of `owner` over the private
	/// keys associated to the session keys.
	fn generate_session_keys_and_proof(owner: Self::AccountId) -> (Self::Keys, Vec<u8>);
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn set_keys() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		pezframe_system::Pezpallet::<T>::inc_providers(&caller);
		let (keys, proof) = T::generate_session_keys_and_proof(caller.clone());

		<pezpallet_session::Pezpallet<T>>::ensure_can_pay_key_deposit(&caller).unwrap();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), keys, proof);

		Ok(())
	}

	#[benchmark]
	fn purge_keys() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		pezframe_system::Pezpallet::<T>::inc_providers(&caller);
		let (keys, proof) = T::generate_session_keys_and_proof(caller.clone());
		<pezpallet_session::Pezpallet<T>>::ensure_can_pay_key_deposit(&caller).unwrap();

		let _t = pezpallet_session::Pezpallet::<T>::set_keys(
			RawOrigin::Signed(caller.clone()).into(),
			keys,
			proof,
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller));

		Ok(())
	}
}
