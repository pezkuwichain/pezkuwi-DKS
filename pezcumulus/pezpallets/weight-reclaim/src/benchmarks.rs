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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use pezframe_support::pezpallet_prelude::{DispatchClass, Pays};
use pezframe_system::RawOrigin;
use pezsp_runtime::traits::{AsTransactionAuthorizedOrigin, DispatchTransaction};

#[pezframe_benchmarking::v2::benchmarks(
	where T: Send + Sync,
		<T as pezframe_system::Config>::RuntimeCall:
			Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
		<T as pezframe_system::Config>::RuntimeOrigin: AsTransactionAuthorizedOrigin,
)]
mod bench {
	use super::*;
	use pezframe_benchmarking::impl_test_function;

	#[benchmark]
	fn storage_weight_reclaim() {
		let ext = StorageWeightReclaim::<T, ()>::new(());

		let origin = RawOrigin::Root.into();
		let call = T::RuntimeCall::from(pezframe_system::Call::remark { remark: alloc::vec![] });

		let overestimate = 10_000;
		let info = DispatchInfo {
			call_weight: Weight::zero().add_proof_size(overestimate),
			extension_weight: Weight::zero(),
			class: DispatchClass::Normal,
			pays_fee: Pays::No,
		};

		let post_info = PostDispatchInfo { actual_weight: None, pays_fee: Pays::No };

		let mut block_weight = pezframe_system::ConsumedWeight::default();
		block_weight.accrue(Weight::from_parts(0, overestimate), info.class);

		pezframe_system::BlockWeight::<T>::put(block_weight);

		#[block]
		{
			assert!(ext.test_run(origin, &call, &info, 0, 0, |_| Ok(post_info)).unwrap().is_ok());
		}

		let final_block_proof_size =
			pezframe_system::BlockWeight::<T>::get().get(info.class).proof_size();

		assert!(
			final_block_proof_size < overestimate,
			"The proof size measured should be less than {overestimate}"
		);
	}

	impl_benchmark_test_suite!(
		Pezpallet,
		crate::tests::setup_test_ext_default(),
		crate::tests::Test
	);
}
