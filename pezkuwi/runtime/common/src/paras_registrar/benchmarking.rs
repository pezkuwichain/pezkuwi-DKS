// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! Benchmarking for paras_registrar pezpallet

#[cfg(feature = "runtime-benchmarks")]
use super::{Pezpallet as Registrar, *};
use crate::traits::Registrar as RegistrarT;
use pezframe_support::assert_ok;
use pezframe_system::RawOrigin;
use pezkuwi_primitives::{MAX_CODE_SIZE, MAX_HEAD_DATA_SIZE, MIN_CODE_SIZE};
use pezkuwi_runtime_teyrchains::{paras, shared, Origin as ParaOrigin};
use pezsp_runtime::traits::Bounded;

use pezframe_benchmarking::v2::*;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	let events = pezframe_system::Pezpallet::<T>::events();
	let system_event: <T as pezframe_system::Config>::RuntimeEvent = generic_event.into();
	// compare to the last event record
	let pezframe_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

fn register_para<T: Config>(id: u32) -> ParaId {
	let para = ParaId::from(id);
	let genesis_head = Registrar::<T>::worst_head_data();
	let validation_code = Registrar::<T>::worst_validation_code();
	let caller: T::AccountId = whitelisted_caller();
	T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
	assert_ok!(Registrar::<T>::reserve(RawOrigin::Signed(caller.clone()).into()));
	assert_ok!(Registrar::<T>::register(
		RawOrigin::Signed(caller).into(),
		para,
		genesis_head,
		validation_code.clone()
	));
	assert_ok!(pezkuwi_runtime_teyrchains::paras::Pezpallet::<T>::add_trusted_validation_code(
		pezframe_system::Origin::<T>::Root.into(),
		validation_code,
	));
	return para;
}

fn para_origin(id: u32) -> ParaOrigin {
	ParaOrigin::Teyrchain(id.into())
}

// This function moves forward to the next scheduled session for teyrchain lifecycle upgrades.
fn next_scheduled_session<T: Config>() {
	shared::Pezpallet::<T>::set_session_index(shared::Pezpallet::<T>::scheduled_session());
	paras::Pezpallet::<T>::test_on_new_session();
}

#[benchmarks(
		where ParaOrigin: Into<<T as pezframe_system::Config>::RuntimeOrigin>,
	)]
mod benchmarks {
	use super::*;
	use alloc::vec;

	#[benchmark]
	fn reserve() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()));

		assert_last_event::<T>(
			Event::<T>::Reserved { para_id: LOWEST_PUBLIC_ID, who: caller }.into(),
		);
		assert!(Paras::<T>::get(LOWEST_PUBLIC_ID).is_some());
		assert_eq!(paras::Pezpallet::<T>::lifecycle(LOWEST_PUBLIC_ID), None);

		Ok(())
	}

	#[benchmark]
	fn register() -> Result<(), BenchmarkError> {
		let para = LOWEST_PUBLIC_ID;
		let genesis_head = Registrar::<T>::worst_head_data();
		let validation_code = Registrar::<T>::worst_validation_code();
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		assert_ok!(Registrar::<T>::reserve(RawOrigin::Signed(caller.clone()).into()));

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), para, genesis_head, validation_code.clone());

		assert_last_event::<T>(Event::<T>::Registered { para_id: para, manager: caller }.into());
		assert_eq!(paras::Pezpallet::<T>::lifecycle(para), Some(ParaLifecycle::Onboarding));
		assert_ok!(pezkuwi_runtime_teyrchains::paras::Pezpallet::<T>::add_trusted_validation_code(
			pezframe_system::Origin::<T>::Root.into(),
			validation_code,
		));
		next_scheduled_session::<T>();
		assert_eq!(paras::Pezpallet::<T>::lifecycle(para), Some(ParaLifecycle::Parathread));

		Ok(())
	}

	#[benchmark]
	fn force_register() -> Result<(), BenchmarkError> {
		let manager: T::AccountId = account("manager", 0, 0);
		let deposit = 0u32.into();
		let para = ParaId::from(69);
		let genesis_head = Registrar::<T>::worst_head_data();
		let validation_code = Registrar::<T>::worst_validation_code();

		#[extrinsic_call]
		_(RawOrigin::Root, manager.clone(), deposit, para, genesis_head, validation_code.clone());

		assert_last_event::<T>(Event::<T>::Registered { para_id: para, manager }.into());
		assert_eq!(paras::Pezpallet::<T>::lifecycle(para), Some(ParaLifecycle::Onboarding));
		assert_ok!(pezkuwi_runtime_teyrchains::paras::Pezpallet::<T>::add_trusted_validation_code(
			pezframe_system::Origin::<T>::Root.into(),
			validation_code,
		));
		next_scheduled_session::<T>();
		assert_eq!(paras::Pezpallet::<T>::lifecycle(para), Some(ParaLifecycle::Parathread));

		Ok(())
	}

	#[benchmark]
	fn deregister() -> Result<(), BenchmarkError> {
		let para = register_para::<T>(LOWEST_PUBLIC_ID.into());
		next_scheduled_session::<T>();
		let caller: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), para);

		assert_last_event::<T>(Event::<T>::Deregistered { para_id: para }.into());

		Ok(())
	}

	#[benchmark]
	fn swap() -> Result<(), BenchmarkError> {
		// On demand teyrchain
		let parathread = register_para::<T>(LOWEST_PUBLIC_ID.into());
		let teyrchain = register_para::<T>((LOWEST_PUBLIC_ID + 1).into());

		let teyrchain_origin = para_origin(teyrchain.into());

		// Actually finish registration process
		next_scheduled_session::<T>();

		// Upgrade the teyrchain
		Registrar::<T>::make_teyrchain(teyrchain)?;
		next_scheduled_session::<T>();

		assert_eq!(paras::Pezpallet::<T>::lifecycle(teyrchain), Some(ParaLifecycle::Teyrchain));
		assert_eq!(paras::Pezpallet::<T>::lifecycle(parathread), Some(ParaLifecycle::Parathread));

		let caller: T::AccountId = whitelisted_caller();
		Registrar::<T>::swap(teyrchain_origin.into(), teyrchain, parathread)?;

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), parathread, teyrchain);

		next_scheduled_session::<T>();
		// Swapped!
		assert_eq!(paras::Pezpallet::<T>::lifecycle(teyrchain), Some(ParaLifecycle::Parathread));
		assert_eq!(paras::Pezpallet::<T>::lifecycle(parathread), Some(ParaLifecycle::Teyrchain));

		Ok(())
	}

	#[benchmark]
	fn schedule_code_upgrade(
		b: Linear<MIN_CODE_SIZE, MAX_CODE_SIZE>,
	) -> Result<(), BenchmarkError> {
		let new_code = ValidationCode(vec![0; b as usize]);
		let para_id = ParaId::from(1000);

		#[extrinsic_call]
		_(RawOrigin::Root, para_id, new_code);

		Ok(())
	}

	#[benchmark]
	fn set_current_head(b: Linear<1, MAX_HEAD_DATA_SIZE>) -> Result<(), BenchmarkError> {
		let new_head = HeadData(vec![0; b as usize]);
		let para_id = ParaId::from(1000);

		#[extrinsic_call]
		_(RawOrigin::Root, para_id, new_head);

		Ok(())
	}

	impl_benchmark_test_suite!(
		Registrar,
		crate::integration_tests::new_test_ext(),
		crate::integration_tests::Test,
	);
}
