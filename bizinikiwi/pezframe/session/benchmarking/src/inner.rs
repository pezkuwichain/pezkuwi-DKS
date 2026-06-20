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

//! Benchmarks for the Session Pezpallet.
// This is separated into its own crate due to cyclic dependency issues.

use alloc::{vec, vec::Vec};
use pezsp_runtime::traits::{One, StaticLookup, TrailingZeroInput};

use codec::Decode;
use pezframe_benchmarking::v2::*;
use pezframe_support::{
	assert_ok,
	traits::{Get, KeyOwnerProofSystem, OnInitialize},
};
use pezframe_system::{pezpallet_prelude::BlockNumberFor, RawOrigin};
use pezpallet_session::{historical::Pezpallet as Historical, Pezpallet as Session, *};
use pezpallet_staking::{
	benchmarking::create_validator_with_nominators, testing_utils::create_validators,
	MaxNominationsOf, RewardDestination,
};

const MAX_VALIDATORS: u32 = 1000;

pub struct Pezpallet<T: Config>(pezpallet_session::Pezpallet<T>);
pub trait Config:
	pezpallet_session::Config + pezpallet_session::historical::Config + pezpallet_staking::Config
{
}

impl<T: Config> OnInitialize<BlockNumberFor<T>> for Pezpallet<T> {
	fn on_initialize(n: BlockNumberFor<T>) -> pezframe_support::weights::Weight {
		pezpallet_session::Pezpallet::<T>::on_initialize(n)
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn set_keys() -> Result<(), BenchmarkError> {
		let n = MaxNominationsOf::<T>::get();
		let (v_stash, _) = create_validator_with_nominators::<T>(
			n,
			MaxNominationsOf::<T>::get(),
			false,
			true,
			RewardDestination::Staked,
		)?;
		let v_controller =
			pezpallet_staking::Pezpallet::<T>::bonded(&v_stash).ok_or("not stash")?;

		let keys = T::Keys::decode(&mut TrailingZeroInput::zeroes()).unwrap();
		let proof: Vec<u8> = vec![0, 1, 2, 3];
		// Whitelist controller account from further DB operations.
		let v_controller_key = pezframe_system::Account::<T>::hashed_key_for(&v_controller);
		pezframe_benchmarking::benchmarking::add_to_whitelist(v_controller_key.into());
		assert_ok!(Session::<T>::ensure_can_pay_key_deposit(&v_controller));

		#[extrinsic_call]
		_(RawOrigin::Signed(v_controller), keys, proof);

		Ok(())
	}

	#[benchmark]
	fn purge_keys() -> Result<(), BenchmarkError> {
		let n = MaxNominationsOf::<T>::get();
		let (v_stash, _) = create_validator_with_nominators::<T>(
			n,
			MaxNominationsOf::<T>::get(),
			false,
			true,
			RewardDestination::Staked,
		)?;
		let v_controller =
			pezpallet_staking::Pezpallet::<T>::bonded(&v_stash).ok_or("not stash")?;
		let keys = T::Keys::decode(&mut TrailingZeroInput::zeroes()).unwrap();
		let proof: Vec<u8> = vec![0, 1, 2, 3];
		assert_ok!(Session::<T>::ensure_can_pay_key_deposit(&v_controller));
		Session::<T>::set_keys(RawOrigin::Signed(v_controller.clone()).into(), keys, proof)?;
		// Whitelist controller account from further DB operations.
		let v_controller_key = pezframe_system::Account::<T>::hashed_key_for(&v_controller);
		pezframe_benchmarking::benchmarking::add_to_whitelist(v_controller_key.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(v_controller));

		Ok(())
	}

	#[benchmark(extra)]
	fn check_membership_proof_current_session(n: Linear<2, MAX_VALIDATORS>) {
		let (key, key_owner_proof1) = check_membership_proof_setup::<T>(n);
		let key_owner_proof2 = key_owner_proof1.clone();

		#[block]
		{
			Historical::<T>::check_proof(key, key_owner_proof1);
		}

		assert!(Historical::<T>::check_proof(key, key_owner_proof2).is_some());
	}

	#[benchmark(extra)]
	fn check_membership_proof_historical_session(n: Linear<2, MAX_VALIDATORS>) {
		let (key, key_owner_proof1) = check_membership_proof_setup::<T>(n);

		// skip to the next session so that the session is historical
		// and the membership merkle proof must be checked.
		Session::<T>::rotate_session();

		let key_owner_proof2 = key_owner_proof1.clone();

		#[block]
		{
			Historical::<T>::check_proof(key, key_owner_proof1);
		}

		assert!(Historical::<T>::check_proof(key, key_owner_proof2).is_some());
	}

	impl_benchmark_test_suite!(
		Pezpallet,
		crate::mock::new_test_ext(),
		crate::mock::Test,
		extra = false
	);
}

/// Sets up the benchmark for checking a membership proof. It creates the given
/// number of validators, sets random session keys and then creates a membership
/// proof for the first authority and returns its key and the proof.
fn check_membership_proof_setup<T: Config>(
	n: u32,
) -> ((pezsp_runtime::KeyTypeId, &'static [u8; 32]), pezsp_session::MembershipProof) {
	pezpallet_staking::ValidatorCount::<T>::put(n);

	// create validators and set random session keys
	for (n, who) in create_validators::<T>(n, 1000).unwrap().into_iter().enumerate() {
		use rand::{RngCore, SeedableRng};

		let validator = T::Lookup::lookup(who).unwrap();
		let controller = pezpallet_staking::Pezpallet::<T>::bonded(&validator).unwrap();

		let keys = {
			let mut keys = [0u8; 128];

			// we keep the keys for the first validator as 0x00000...
			if n > 0 {
				let mut rng = rand::rngs::StdRng::seed_from_u64(n as u64);
				rng.fill_bytes(&mut keys);
			}

			keys
		};

		// TODO: this benchmark is broken, session keys cannot be decoded into 128 bytes anymore,
		// but not an issue for CI since it is `extra`.
		let keys: T::Keys = Decode::decode(&mut &keys[..]).unwrap();
		let proof: Vec<u8> = vec![];

		Session::<T>::set_keys(RawOrigin::Signed(controller).into(), keys, proof).unwrap();
	}

	Pezpallet::<T>::on_initialize(pezframe_system::pezpallet_prelude::BlockNumberFor::<T>::one());

	// skip sessions until the new validator set is enacted
	while Validators::<T>::get().len() < n as usize {
		Session::<T>::rotate_session();
	}

	let key = (pezsp_runtime::KeyTypeId(*b"babe"), &[0u8; 32]);

	(key, Historical::<T>::prove(key).unwrap())
}
