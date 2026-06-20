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

//! # Scheduler test environment.

use super::*;

use crate as scheduler;
use pezframe_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{ConstU32, Contains, EitherOfDiverse, EqualPrivilegeOnly},
};
use pezframe_system::{EnsureRoot, EnsureSignedBy};
use pezsp_runtime::{BuildStorage, Perbill};
use pezsp_weights::constants::WEIGHT_REF_TIME_PER_SECOND;

// Logger module to track execution.
#[pezframe_support::pezpallet]
pub mod logger {
	use super::{OriginCaller, OriginTrait};
	use pezframe_support::{parameter_types, pezpallet_prelude::*};
	use pezframe_system::pezpallet_prelude::*;

	parameter_types! {
		static Log: Vec<(OriginCaller, u32)> = Vec::new();
	}
	pub fn log() -> Vec<(OriginCaller, u32)> {
		Log::get().clone()
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::storage]
	pub type Threshold<T: Config> = StorageValue<_, (BlockNumberFor<T>, BlockNumberFor<T>)>;

	#[pezpallet::error]
	pub enum Error<T> {
		/// Under the threshold.
		TooEarly,
		/// Over the threshold.
		TooLate,
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {}

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Logged(u32, Weight),
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T>
	where
		<T as pezframe_system::Config>::RuntimeOrigin: OriginTrait<PalletsOrigin = OriginCaller>,
	{
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(*weight)]
		pub fn log(origin: OriginFor<T>, i: u32, weight: Weight) -> DispatchResult {
			Self::deposit_event(Event::Logged(i, weight));
			Log::mutate(|log| {
				log.push((origin.caller().clone(), i));
			});
			Ok(())
		}

		#[pezpallet::call_index(1)]
		#[pezpallet::weight(*weight)]
		pub fn log_without_filter(origin: OriginFor<T>, i: u32, weight: Weight) -> DispatchResult {
			Self::deposit_event(Event::Logged(i, weight));
			Log::mutate(|log| {
				log.push((origin.caller().clone(), i));
			});
			Ok(())
		}

		#[pezpallet::call_index(2)]
		#[pezpallet::weight(*weight)]
		pub fn timed_log(origin: OriginFor<T>, i: u32, weight: Weight) -> DispatchResult {
			let now = pezframe_system::Pezpallet::<T>::block_number();
			let (start, end) = Threshold::<T>::get().unwrap_or((0u32.into(), u32::MAX.into()));
			ensure!(now >= start, Error::<T>::TooEarly);
			ensure!(now <= end, Error::<T>::TooLate);
			Self::deposit_event(Event::Logged(i, weight));
			Log::mutate(|log| {
				log.push((origin.caller().clone(), i));
			});
			Ok(())
		}
	}
}

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Logger: logger,
		Scheduler: scheduler,
		Preimage: pezpallet_preimage,
	}
);

// Scheduler must dispatch with root and no filter, this tests base filter is indeed not used.
pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
	fn contains(call: &RuntimeCall) -> bool {
		!matches!(call, RuntimeCall::Logger(LoggerCall::log { .. }))
	}
}

parameter_types! {
	pub BlockWeights: pezframe_system::limits::BlockWeights =
		pezframe_system::limits::BlockWeights::simple_max(
			Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND * 2, u64::MAX),
		);
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
	type BaseCallFilter = BaseFilter;
	type Block = Block;
	type BlockWeights = BlockWeights;
}
impl logger::Config for Test {
	type RuntimeEvent = RuntimeEvent;
}
ord_parameter_types! {
	pub const One: u64 = 1;
}

impl pezpallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = ();
	type ManagerOrigin = EnsureRoot<u64>;
	type Consideration = ();
}

pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
	fn service_agendas_base() -> Weight {
		Weight::from_parts(0b0000_0001, 0)
	}
	fn service_agenda_base(i: u32) -> Weight {
		Weight::from_parts((i << 8) as u64 + 0b0000_0010, 0)
	}
	fn service_task_base() -> Weight {
		Weight::from_parts(0b0000_0100, 0)
	}
	fn service_task_periodic() -> Weight {
		Weight::from_parts(0b0000_1100, 0)
	}
	fn service_task_named() -> Weight {
		Weight::from_parts(0b0001_0100, 0)
	}
	fn service_task_fetched(s: u32) -> Weight {
		Weight::from_parts((s << 8) as u64 + 0b0010_0100, 0)
	}
	fn execute_dispatch_signed() -> Weight {
		Weight::from_parts(0b0100_0000, 0)
	}
	fn execute_dispatch_unsigned() -> Weight {
		Weight::from_parts(0b1000_0000, 0)
	}
	fn schedule(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn cancel(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn schedule_named(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn cancel_named(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn schedule_retry(_s: u32) -> Weight {
		Weight::from_parts(100000, 0)
	}
	fn set_retry() -> Weight {
		Weight::from_parts(50, 0)
	}
	fn set_retry_named() -> Weight {
		Weight::from_parts(50, 0)
	}
	fn cancel_retry() -> Weight {
		Weight::from_parts(50, 0)
	}
	fn cancel_retry_named() -> Weight {
		Weight::from_parts(50, 0)
	}
}
parameter_types! {
	pub storage MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		BlockWeights::get().max_block;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EitherOfDiverse<EnsureRoot<u64>, EnsureSignedBy<One, u64>>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaxScheduledPerBlock = ConstU32<10>;
	type WeightInfo = TestWeightInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = pezframe_system::Pezpallet<Self>;
}

pub type LoggerCall = logger::Call<Test>;

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = system::GenesisConfig::<Test>::default().build_storage().unwrap();
	t.into()
}

pub fn root() -> OriginCaller {
	system::RawOrigin::Root.into()
}
