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

//! Benchmarks for the MMR pezpallet.

#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use pezframe::{
	benchmarking::prelude::v1::benchmarks_instance_pallet,
	deps::pezframe_support::traits::OnInitialize,
};

benchmarks_instance_pallet! {
	on_initialize {
		let x in 1 .. 1_000;

		let leaves = x as NodeIndex;

		<<T as pezpallet::Config::<I>>::BenchmarkHelper as BenchmarkHelper>::setup();
		for leaf in 0..(leaves - 1) {
			<Pezpallet::<T, I> as OnInitialize<BlockNumberFor<T>>>::on_initialize((leaf as u32).into());
		}
	}: {
		<Pezpallet::<T, I> as OnInitialize<BlockNumberFor<T>>>::on_initialize((leaves as u32 - 1).into());
	} verify {
		assert_eq!(crate::NumberOfLeaves::<T, I>::get(), leaves);
	}

	impl_benchmark_test_suite!(Pezpallet, crate::tests::new_test_ext(), crate::mock::Test);
}
