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

use crate::{
	unsigned::{miner::OffchainWorkerMiner, Call, Config, Pezpallet},
	verifier::Verifier,
	CurrentPhase, Phase,
};
use pezframe_benchmarking::v2::*;
use pezframe_election_provider_support::ElectionProvider;
use pezframe_support::{assert_ok, pezpallet_prelude::*};
use pezframe_system::RawOrigin;
use pezsp_std::boxed::Box;

#[benchmarks(where T: crate::Config + crate::signed::Config + crate::verifier::Config)]
mod benchmarks {
	use super::*;

	#[benchmark(pov_mode = Measured)]
	fn validate_unsigned() -> Result<(), BenchmarkError> {
		#[cfg(test)]
		crate::mock::ElectionStart::set(pezsp_runtime::traits::Bounded::max_value());
		crate::Pezpallet::<T>::start().unwrap();

		crate::Pezpallet::<T>::roll_until_matches(|| {
			matches!(CurrentPhase::<T>::get(), Phase::Unsigned(_))
		});
		let call: Call<T> = OffchainWorkerMiner::<T>::mine_solution(T::MinerPages::get(), false)
			.map(|solution| Call::submit_unsigned { paged_solution: Box::new(solution) })
			.unwrap();

		#[block]
		{
			assert_ok!(Pezpallet::<T>::validate_unsigned(TransactionSource::Local, &call));
		}

		Ok(())
	}

	#[benchmark(pov_mode = Measured)]
	fn submit_unsigned() -> Result<(), BenchmarkError> {
		#[cfg(test)]
		crate::mock::ElectionStart::set(pezsp_runtime::traits::Bounded::max_value());
		crate::Pezpallet::<T>::start().unwrap();

		// roll to unsigned phase open
		crate::Pezpallet::<T>::roll_until_matches(|| {
			matches!(CurrentPhase::<T>::get(), Phase::Unsigned(_))
		});
		// TODO: we need to better ensure that this is actually worst case
		let solution =
			OffchainWorkerMiner::<T>::mine_solution(T::MinerPages::get(), false).unwrap();

		// nothing is queued
		assert!(T::Verifier::queued_score().is_none());
		#[block]
		{
			assert_ok!(Pezpallet::<T>::submit_unsigned(RawOrigin::None.into(), Box::new(solution)));
		}

		// something is queued
		assert!(T::Verifier::queued_score().is_some());
		Ok(())
	}

	#[benchmark(extra, pov_mode = Measured)]
	fn mine_solution(p: Linear<1, { T::Pages::get() }>) -> Result<(), BenchmarkError> {
		#[cfg(test)]
		crate::mock::ElectionStart::set(pezsp_runtime::traits::Bounded::max_value());
		crate::Pezpallet::<T>::start().unwrap();

		// roll to unsigned phase open
		crate::Pezpallet::<T>::roll_until_matches(|| {
			matches!(CurrentPhase::<T>::get(), Phase::Unsigned(_))
		});

		#[block]
		{
			OffchainWorkerMiner::<T>::mine_solution(p, true).unwrap();
		}

		Ok(())
	}

	impl_benchmark_test_suite!(
		Pezpallet,
		crate::mock::ExtBuilder::full().build_unchecked(),
		crate::mock::Runtime
	);
}
