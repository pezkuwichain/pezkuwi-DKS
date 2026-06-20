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

//! On demand assigner pezpallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use super::{Pezpallet, *};
use crate::{
	configuration::{HostConfiguration, Pezpallet as ConfigurationPallet},
	paras::{ParaGenesisArgs, ParaKind, Pezpallet as ParasPallet, TeyrchainsCache},
	shared::Pezpallet as ParasShared,
};

use alloc::vec;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;
use pezsp_runtime::traits::Bounded;

use pezkuwi_primitives::{
	HeadData, Id as ParaId, SessionIndex, ValidationCode, ON_DEMAND_DEFAULT_QUEUE_MAX_SIZE,
};

// Constants for the benchmarking
const SESSION_INDEX: SessionIndex = 1;

// Initialize a parathread for benchmarking.
pub fn init_parathread<T>(para_id: ParaId)
where
	T: Config + crate::paras::Config + crate::shared::Config,
{
	ParasShared::<T>::set_session_index(SESSION_INDEX);
	let mut config = HostConfiguration::default();
	config.scheduler_params.num_cores = 1;
	ConfigurationPallet::<T>::force_set_active_config(config);
	let mut teyrchains = TeyrchainsCache::new();
	ParasPallet::<T>::initialize_para_now(
		&mut teyrchains,
		para_id,
		&ParaGenesisArgs {
			para_kind: ParaKind::Parathread,
			genesis_head: HeadData(vec![1, 2, 3, 4]),
			validation_code: ValidationCode(vec![1, 2, 3, 4]),
		},
	);
}

#[benchmarks]
mod benchmarks {
	/// We want to fill the queue to the maximum, so exactly one more item fits.
	const MAX_FILL_BENCH: u32 = ON_DEMAND_DEFAULT_QUEUE_MAX_SIZE.saturating_sub(1);

	use super::*;
	#[benchmark]
	fn place_order_keep_alive(s: Linear<1, MAX_FILL_BENCH>) {
		// Setup
		let caller = whitelisted_caller();
		let para_id = ParaId::from(111u32);
		init_parathread::<T>(para_id);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
		Pezpallet::<T>::populate_queue(para_id, s);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.into()), BalanceOf::<T>::max_value(), para_id)
	}

	#[benchmark]
	fn place_order_allow_death(s: Linear<1, MAX_FILL_BENCH>) {
		// Setup
		let caller = whitelisted_caller();
		let para_id = ParaId::from(111u32);
		init_parathread::<T>(para_id);
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

		Pezpallet::<T>::populate_queue(para_id, s);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.into()), BalanceOf::<T>::max_value(), para_id)
	}

	#[benchmark]
	fn place_order_with_credits(s: Linear<1, MAX_FILL_BENCH>) {
		// Setup
		let caller: T::AccountId = whitelisted_caller();
		let para_id = ParaId::from(111u32);
		init_parathread::<T>(para_id);
		Credits::<T>::insert(&caller, BalanceOf::<T>::max_value());

		Pezpallet::<T>::populate_queue(para_id, s);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.into()), BalanceOf::<T>::max_value(), para_id)
	}

	impl_benchmark_test_suite!(
		Pezpallet,
		crate::mock::new_test_ext(
			crate::on_demand::mock_helpers::GenesisConfigBuilder::default().build()
		),
		crate::mock::Test
	);
}
