// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-staking-score

use crate::{
	BalanceOf, CachedStakingDetails, Call, Config, NoterBonds, NoterCheck, PendingStakingDetails,
	PendingSubmission, Pezpallet, StakingDetails, StakingSource, StakingStartBlock, UNITS,
};
use pezframe_benchmarking::v2::*;
use pezframe_support::traits::{Currency, EnsureOrigin, Get};
use pezframe_system::RawOrigin;
use pezsp_runtime::traits::Saturating;

/// Fund `who` with double the noter bond and register them as an active
/// (bonded) noter — `T::NoterChecker::make_noter` handles whatever a real
/// runtime's checker needs (e.g. minting the tiki pallet's Noter role NFT),
/// so this pallet's benchmarks stay decoupled from that pallet's internals.
fn fund_and_register_noter<T: Config>(who: &T::AccountId) {
	let bond: BalanceOf<T> = T::NoterBondAmount::get();
	let _ = T::Currency::deposit_creating(who, bond + bond);
	T::NoterChecker::make_noter(who);
	Pezpallet::<T>::register_as_noter(RawOrigin::Signed(who.clone()).into())
		.expect("benchmark setup: register_as_noter must succeed");
}

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

	#[benchmark]
	fn register_as_noter() {
		let caller: T::AccountId = whitelisted_caller();
		let bond: BalanceOf<T> = T::NoterBondAmount::get();
		let _ = T::Currency::deposit_creating(&caller, bond + bond);
		T::NoterChecker::make_noter(&caller);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()));

		assert!(NoterBonds::<T>::get(&caller).is_some());
	}

	#[benchmark]
	fn unregister_as_noter() {
		let caller: T::AccountId = whitelisted_caller();
		fund_and_register_noter::<T>(&caller);
		// No submission on record — the dispute-window gate is a no-op here,
		// which is the cheapest path but still exercises the full call.

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()));

		assert!(NoterBonds::<T>::get(&caller).is_none());
	}

	/// Benchmark worst case: DisputeOrigin freezing a pending submission that
	/// does exist (the existence check + removal, not just the origin check).
	#[benchmark]
	fn dispute_staking_details() {
		let noter: T::AccountId = whitelisted_caller();
		fund_and_register_noter::<T>(&noter);

		let target: T::AccountId = account("target", 0, 0);
		PendingStakingDetails::<T>::insert(
			&target,
			StakingSource::RelayChain,
			PendingSubmission {
				details: StakingDetails {
					staked_amount: (500u128 * UNITS).into(),
					nominations_count: 3,
					unlocking_chunks_count: 0,
				},
				submitted_by: noter,
				submitted_at: 0u32.into(),
			},
		);

		let dispute_origin = T::DisputeOrigin::try_successful_origin()
			.expect("DisputeOrigin must have a benchmark origin");

		#[extrinsic_call]
		_(dispute_origin, target.clone(), StakingSource::RelayChain);

		assert!(PendingStakingDetails::<T>::get(&target, StakingSource::RelayChain).is_none());
	}

	/// Benchmark worst case: same cost profile as the Root path of
	/// `receive_staking_details` (a full `apply_staking_update`), plus the
	/// pending-entry lookup/removal.
	#[benchmark]
	fn finalize_staking_details() {
		let noter: T::AccountId = whitelisted_caller();
		fund_and_register_noter::<T>(&noter);

		let target: T::AccountId = account("target", 0, 0);
		CachedStakingDetails::<T>::insert(
			&target,
			StakingSource::AssetHub,
			StakingDetails {
				staked_amount: (200u128 * UNITS).into(),
				nominations_count: 1,
				unlocking_chunks_count: 0,
			},
		);
		PendingStakingDetails::<T>::insert(
			&target,
			StakingSource::RelayChain,
			PendingSubmission {
				details: StakingDetails {
					staked_amount: (500u128 * UNITS).into(),
					nominations_count: 3,
					unlocking_chunks_count: 0,
				},
				submitted_by: noter,
				submitted_at: 0u32.into(),
			},
		);
		// Advance past the runtime's actual configured DisputeWindow so the
		// submission above has matured — submitted_at alone (block 0) proves
		// nothing if the current block hasn't moved past it too.
		pezframe_system::Pezpallet::<T>::set_block_number(
			T::DisputeWindow::get().saturating_add(1u32.into()),
		);

		let caller: T::AccountId = account("finalizer", 0, 0);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), target.clone(), StakingSource::RelayChain);

		assert!(PendingStakingDetails::<T>::get(&target, StakingSource::RelayChain).is_none());
		assert!(CachedStakingDetails::<T>::get(&target, StakingSource::RelayChain).is_some());
	}

	#[benchmark]
	fn slash_noter() {
		let noter: T::AccountId = whitelisted_caller();
		fund_and_register_noter::<T>(&noter);

		let slash_origin = T::SlashOrigin::try_successful_origin()
			.expect("SlashOrigin must have a benchmark origin");

		#[extrinsic_call]
		_(slash_origin, noter.clone());

		assert!(NoterBonds::<T>::get(&noter).is_none());
	}
}
