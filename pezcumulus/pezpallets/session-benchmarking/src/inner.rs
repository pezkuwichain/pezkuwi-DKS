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

use alloc::{vec, vec::Vec};

use codec::Decode;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;
use pezpallet_session::*;
pub struct Pezpallet<T: Config>(pezpallet_session::Pezpallet<T>);
pub trait Config: pezpallet_session::Config {}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn set_keys() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		pezframe_system::Pezpallet::<T>::inc_providers(&caller);
		let keys =
			T::Keys::decode(&mut pezsp_runtime::traits::TrailingZeroInput::zeroes()).unwrap();
		let proof: Vec<u8> = vec![0, 1, 2, 3];
		<pezpallet_session::Pezpallet<T>>::ensure_can_pay_key_deposit(&caller).unwrap();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), keys, proof);

		Ok(())
	}

	#[benchmark]
	fn purge_keys() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		pezframe_system::Pezpallet::<T>::inc_providers(&caller);
		let keys =
			T::Keys::decode(&mut pezsp_runtime::traits::TrailingZeroInput::zeroes()).unwrap();
		let proof: Vec<u8> = vec![0, 1, 2, 3];
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
