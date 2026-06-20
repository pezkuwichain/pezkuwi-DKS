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

//! Benchmarks for the ping pezpallet.

use super::*;
use alloc::vec;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	/// Benchmark `start` with variable payload size.
	#[benchmark]
	fn start(s: Linear<1, 1024>) {
		let para = ParaId::from(2000u32);
		let payload = vec![0u8; s as usize];

		#[extrinsic_call]
		_(RawOrigin::Root, para, payload.clone());

		assert_eq!(Targets::<T>::get().len(), 1);
		let (stored_para, stored_payload) = &Targets::<T>::get()[0];
		assert_eq!(*stored_para, para);
		assert_eq!(stored_payload.to_vec(), payload);
	}

	/// Benchmark `start_many` with variable count.
	#[benchmark]
	fn start_many(n: Linear<1, 100>) {
		let para = ParaId::from(2000u32);
		let payload = vec![0u8; 32];

		#[extrinsic_call]
		_(RawOrigin::Root, para, n, payload);

		assert_eq!(Targets::<T>::get().len(), n as usize);
	}

	/// Benchmark `stop` — pre-fill Targets with one entry, then remove it.
	#[benchmark]
	fn stop() {
		let para = ParaId::from(2000u32);
		let payload: BoundedVec<u8, MaxPayloadSize> = vec![0u8; 32].try_into().unwrap();
		let targets: BoundedVec<(ParaId, BoundedVec<u8, MaxPayloadSize>), MaxTeyrchains> =
			vec![(para, payload)].try_into().unwrap();
		Targets::<T>::put(targets);
		assert_eq!(Targets::<T>::get().len(), 1);

		#[extrinsic_call]
		_(RawOrigin::Root, para);

		assert_eq!(Targets::<T>::get().len(), 0);
	}

	/// Benchmark `stop_all` — pre-fill Targets, then clear everything.
	#[benchmark]
	fn stop_all() {
		let payload: BoundedVec<u8, MaxPayloadSize> = vec![0u8; 32].try_into().unwrap();
		let mut entries = alloc::vec::Vec::new();
		for i in 0..10u32 {
			entries.push((ParaId::from(2000 + i), payload.clone()));
		}
		let targets: BoundedVec<(ParaId, BoundedVec<u8, MaxPayloadSize>), MaxTeyrchains> =
			entries.try_into().unwrap();
		Targets::<T>::put(targets);
		assert_eq!(Targets::<T>::get().len(), 10);

		#[extrinsic_call]
		_(RawOrigin::Root, None::<ParaId>);

		assert_eq!(Targets::<T>::get().len(), 0);
	}

	// NOTE: `ping` and `pong` extrinsics require a sibling teyrchain origin
	// (via `pezcumulus_pezpallet_xcm::ensure_sibling_para`), which is an XCM-derived
	// origin that cannot be constructed in the standard frame-benchmarking environment.
	// These extrinsics also perform XCM sends as part of their execution.
	//
	// To properly benchmark these, a parachain-aware benchmarking harness with
	// cumulus origin support would be needed. For now, their weights are estimated
	// based on the storage access patterns and XCM encoding costs.
	//
	// The weight functions `ping(s)` and `pong(s)` in weights.rs account for the
	// payload size variable and provide conservative estimates.

	impl_benchmark_test_suite!(
		Pezpallet,
		super::super::mock::new_bench_ext(),
		super::super::mock::Test
	);
}
