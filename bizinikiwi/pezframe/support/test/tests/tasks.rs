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

#![cfg(feature = "experimental")]

#[pezframe_support::pezpallet(dev_mode)]
mod my_pezpallet {
	use pezframe_support::pezpallet_prelude::{StorageValue, ValueQuery};

	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(_);

	#[pezpallet::storage]
	pub type SomeStorage<T, I = ()> = StorageValue<_, (u32, u64), ValueQuery>;

	#[pezpallet::tasks_experimental]
	impl<T: Config<I>, I> Pezpallet<T, I> {
		#[pezpallet::task_index(0)]
		#[pezpallet::task_condition(|i, j| i == 0u32 && j == 2u64)]
		#[pezpallet::task_list(vec![(0u32, 2u64), (2u32, 4u64)].iter())]
		#[pezpallet::task_weight(0.into())]
		fn foo(_i: u32, _j: u64) -> pezframe_support::pezpallet_prelude::DispatchResult {
			<SomeStorage<T, I>>::put((_i, _j));
			Ok(())
		}
	}
}

// Another pezpallet for which we won't implement the default instance.
#[pezframe_support::pezpallet(dev_mode)]
mod my_pezpallet_2 {
	use pezframe_support::pezpallet_prelude::{StorageValue, ValueQuery};

	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(_);

	#[pezpallet::storage]
	pub type SomeStorage<T, I = ()> = StorageValue<_, (u32, u64), ValueQuery>;

	#[pezpallet::tasks_experimental]
	impl<T: Config<I>, I> Pezpallet<T, I> {
		#[pezpallet::task_index(0)]
		#[pezpallet::task_condition(|i, j| i == 0u32 && j == 2u64)]
		#[pezpallet::task_list(vec![(0u32, 2u64), (2u32, 4u64)].iter())]
		#[pezpallet::task_weight(0.into())]
		fn foo(_i: u32, _j: u64) -> pezframe_support::pezpallet_prelude::DispatchResult {
			<SomeStorage<T, I>>::put((_i, _j));
			Ok(())
		}
	}
}

type BlockNumber = u32;
type AccountId = u64;
type Header = pezsp_runtime::generic::Header<BlockNumber, pezsp_runtime::traits::BlakeTwo256>;
type UncheckedExtrinsic = pezsp_runtime::generic::UncheckedExtrinsic<u32, RuntimeCall, (), ()>;
type Block = pezsp_runtime::generic::Block<Header, UncheckedExtrinsic>;

pezframe_support::construct_runtime!(
	pub enum Runtime
	{
		System: pezframe_system,
		MyPallet: my_pezpallet,
		MyPallet2: my_pezpallet::<Instance2>,
		#[cfg(feature = "frame-feature-testing")]
		MyPallet3: my_pezpallet::<Instance3>,
		MyPallet4: my_pezpallet_2::<Instance1>,
	}
);

// NOTE: Needed for derive_impl expansion
use pezframe_support::derive_impl;
#[pezframe_support::derive_impl(pezframe_system::config_preludes::TestDefaultConfig as pezframe_system::DefaultConfig)]
impl pezframe_system::Config for Runtime {
	type Block = Block;
	type AccountId = AccountId;
}

impl my_pezpallet::Config for Runtime {}

impl my_pezpallet::Config<pezframe_support::instances::Instance2> for Runtime {}

#[cfg(feature = "frame-feature-testing")]
impl my_pezpallet::Config<pezframe_support::instances::Instance3> for Runtime {}

impl my_pezpallet_2::Config<pezframe_support::instances::Instance1> for Runtime {}

fn new_test_ext() -> pezsp_io::TestExternalities {
	use pezsp_runtime::BuildStorage;

	RuntimeGenesisConfig::default().build_storage().unwrap().into()
}

#[test]
fn tasks_work() {
	new_test_ext().execute_with(|| {
		use pezframe_support::instances::{Instance1, Instance2};

		let task = RuntimeTask::MyPallet(my_pezpallet::Task::<Runtime>::Foo { _i: 0u32, _j: 2u64 });

		pezframe_support::assert_ok!(System::do_task(RuntimeOrigin::signed(1), task.clone(),));
		assert_eq!(my_pezpallet::SomeStorage::<Runtime>::get(), (0, 2));

		let task =
			RuntimeTask::MyPallet2(my_pezpallet::Task::<Runtime, _>::Foo { _i: 0u32, _j: 2u64 });

		pezframe_support::assert_ok!(System::do_task(RuntimeOrigin::signed(1), task.clone(),));
		assert_eq!(my_pezpallet::SomeStorage::<Runtime, Instance2>::get(), (0, 2));

		let task =
			RuntimeTask::MyPallet4(my_pezpallet_2::Task::<Runtime, _>::Foo { _i: 0u32, _j: 2u64 });

		pezframe_support::assert_ok!(System::do_task(RuntimeOrigin::signed(1), task.clone(),));
		assert_eq!(my_pezpallet_2::SomeStorage::<Runtime, Instance1>::get(), (0, 2));
	});
}

#[test]
fn do_task_unsigned_validation_rejects_external_source() {
	new_test_ext().execute_with(|| {
		use pezframe_support::pezpallet_prelude::{
			InvalidTransaction, TransactionSource, TransactionValidityError, ValidateUnsigned,
		};

		let task = RuntimeTask::MyPallet(my_pezpallet::Task::<Runtime>::Foo { _i: 0u32, _j: 2u64 });
		let call = pezframe_system::Call::do_task { task };

		assert!(matches!(
			System::validate_unsigned(TransactionSource::External, &call),
			Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
		));

		assert!(System::validate_unsigned(TransactionSource::InBlock, &call).is_ok());
		assert!(System::validate_unsigned(TransactionSource::Local, &call).is_ok());
	});
}
