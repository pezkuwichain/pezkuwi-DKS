// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use super::*;

use crate::Pezpallet as InboundQueue;
use pezframe_benchmarking::v2::*;
use pezframe_support::assert_ok;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn submit() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();

		let create_message = T::Helper::initialize_storage();

		#[block]
		{
			assert_ok!(InboundQueue::<T>::submit(
				RawOrigin::Signed(caller.clone()).into(),
				Box::new(create_message.event),
			));
		}

		Ok(())
	}

	impl_benchmark_test_suite!(InboundQueue, crate::mock::new_tester(), crate::mock::Test);
}
