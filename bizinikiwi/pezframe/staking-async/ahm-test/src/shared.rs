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

use crate::*;
use pezframe::testing_prelude::*;
use std::cell::UnsafeCell;

thread_local! {
	pub static RC_STATE: UnsafeCell<TestState> = UnsafeCell::new(Default::default());
	pub static AH_STATE: UnsafeCell<TestState> = UnsafeCell::new(Default::default());
}

parameter_types! {
	// counts how many times a new offence message is sent from RC -> AH.
	pub static CounterRCAHNewOffence: u32 = 0;
	// counts how many times a new session report is sent from RC -> AH.
	pub static CounterRCAHSessionReport: u32 = 0;
	// counts how many times a validator set is sent to RC.
	pub static CounterAHRCValidatorSet: u32 = 0;
}

pub fn put_ah_state(ah: TestState) {
	AH_STATE.with(|state| unsafe {
		let ptr = state.get();
		*ptr = ah;
	})
}

pub fn in_ah(f: impl FnMut() -> ()) {
	AH_STATE.with(|state| unsafe {
		let ptr = state.get();
		(*ptr).execute_with(f)
	})
}

pub fn put_rc_state(rc: TestState) {
	RC_STATE.with(|state| unsafe {
		let ptr = state.get();
		*ptr = rc;
	})
}

pub fn in_rc(f: impl FnMut() -> ()) {
	RC_STATE.with(|state| unsafe {
		let ptr = state.get();
		(*ptr).execute_with(f)
	})
}

pub fn migrate_state() {
	// NOTE: this is not exhaustive, only migrates the state that is needed for the tests.
	shared::in_rc(|| {
		let current_era = pezpallet_staking::CurrentEra::<rc::Runtime>::take();
		let active_era = pezpallet_staking::ActiveEra::<rc::Runtime>::take().unwrap();
		shared::in_ah(|| {
			pezpallet_staking_async::CurrentEra::<ah::Runtime>::set(current_era);
			pezpallet_staking_async::ActiveEra::<ah::Runtime>::set(Some(
				pezpallet_staking_async::ActiveEraInfo {
					index: active_era.index,
					start: active_era.start,
				},
			));
		});

		for (era, reward_points) in pezpallet_staking::ErasRewardPoints::<rc::Runtime>::drain() {
			shared::in_ah(|| {
				pezpallet_staking_async::ErasRewardPoints::<ah::Runtime>::insert(
					era,
					pezpallet_staking_async::EraRewardPoints {
						total: reward_points.total,
						individual: reward_points.individual.clone().try_into().unwrap(),
					},
				)
			});
		}

		// exposure
		for (era, account, overview) in
			pezpallet_staking::ErasStakersOverview::<rc::Runtime>::drain()
		{
			shared::in_ah(|| {
				pezpallet_staking_async::ErasStakersOverview::<ah::Runtime>::insert(
					era, account, overview,
				)
			});
		}

		for ((era, account, page), exposure_page) in
			pezpallet_staking::ErasStakersPaged::<rc::Runtime>::drain()
		{
			shared::in_ah(|| {
				pezpallet_staking_async::ErasStakersPaged::<ah::Runtime>::insert(
					(era, account, page),
					exposure_page.clone(),
				)
			});
		}

		shared::in_ah(|| {
			pezpallet_staking_async::BondedEras::<ah::Runtime>::kill();
		});

		for (era, session) in pezpallet_staking::BondedEras::<rc::Runtime>::get() {
			shared::in_ah(|| {
				pezpallet_staking_async::BondedEras::<ah::Runtime>::mutate(|bonded| {
					bonded.try_push((era, session)).unwrap()
				})
			});
		}
	})
}
