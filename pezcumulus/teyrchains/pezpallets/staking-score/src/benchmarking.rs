// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-staking-score

use crate::{
	CachedStakingDetails, Call, Config, Pezpallet, StakingDetails, StakingSource,
	StakingStartBlock, UNITS,
};
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn start_score_tracking() {
		let caller: T::AccountId = whitelisted_caller();

		// Ensure no prior tracking exists.
		StakingStartBlock::<T>::remove(&caller);

		// Pre-populate CachedStakingDetails for worst-case OnStakingUpdate callback.
		CachedStakingDetails::<T>::insert(
			&caller,
			StakingSource::RelayChain,
			StakingDetails {
				staked_amount: (1000u128 * UNITS).into(),
				nominations_count: 5,
				unlocking_chunks_count: 2,
			},
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()));

		assert!(StakingStartBlock::<T>::get(&caller).is_some());
	}

	/// Benchmark worst case: root origin, non-zero stake insert.
	#[benchmark]
	fn receive_staking_details() {
		let target: T::AccountId = whitelisted_caller();

		// Pre-populate both sources for worst-case trust callback iteration.
		CachedStakingDetails::<T>::insert(
			&target,
			StakingSource::AssetHub,
			StakingDetails {
				staked_amount: (200u128 * UNITS).into(),
				nominations_count: 1,
				unlocking_chunks_count: 0,
			},
		);

		#[extrinsic_call]
		_(
			RawOrigin::Root,
			target.clone(),
			StakingSource::RelayChain,
			(500u128 * UNITS).into(),
			3u32,
			0u32,
		);

		assert!(CachedStakingDetails::<T>::get(&target, StakingSource::RelayChain).is_some());
	}
}
