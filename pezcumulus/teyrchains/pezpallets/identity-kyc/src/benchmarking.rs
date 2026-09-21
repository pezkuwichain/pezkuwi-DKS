// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-identity-kyc

use super::*;
use pezframe_benchmarking::v2::*;
use pezframe_support::traits::Currency;
use pezframe_system::RawOrigin;
use pezsp_core::H256;

/// Helper function to create a funded account
fn funded_account<T: Config>(name: &'static str, index: u32) -> T::AccountId {
	let caller: T::AccountId = account(name, index, 0);
	let amount = T::KycApplicationDeposit::get() * 10u32.into();
	T::Currency::make_free_balance_be(&caller, amount);
	caller
}

/// Helper function to setup a citizen (for referrer)
fn setup_citizen<T: Config>(who: &T::AccountId) {
	KycStatuses::<T>::insert(who, KycLevel::Approved);

	// An approved KYC status is not a citizen, and the difference is what two of this pallet's
	// benchmarks measured instead of their calls. Measured against the People runtimes on
	// 2026-09-20: `approve_referral` came back `NotEligibleToVouch` because `CitizenSince` was
	// never written, and `renounce_citizenship` came back `CitizenNftNotFound` because nothing
	// had minted the NFT. Neither showed up in this crate's own tests -- the mock binds
	// `CitizenNftProvider` and `VouchingCapacity` to stubs that accept anything, so the gap only
	// exists where the real pallets are wired in, which is exactly where weights are taken.
	//
	// One rather than zero: zero is the genesis exemption, and the path every referral after
	// the founding generation takes is the one that waits.
	let since: pezframe_system::pezpallet_prelude::BlockNumberFor<T> = 1u32.into();
	CitizenSince::<T>::insert(who, since);
	pezframe_system::Pezpallet::<T>::set_block_number(
		T::VouchingWaitingPeriod::get().saturating_add(2u32.into()),
	);
	T::CitizenNftProvider::mint_citizen_nft_confirmed(who)
		.expect("a citizen this pallet just approved must be able to hold the NFT; qed");
}

/// Helper function to setup an applicant in PendingReferral state
fn setup_pending_referral<T: Config>(applicant: &T::AccountId, referrer: &T::AccountId) {
	let identity_hash = H256::repeat_byte(0x01);
	let application = CitizenshipApplication {
		identity_hash,
		referrer: referrer.clone(),
		inviter: None,
		applied_at: pezframe_system::Pezpallet::<T>::block_number(),
	};
	Applications::<T>::insert(applicant, application);
	KycStatuses::<T>::insert(applicant, KycLevel::PendingReferral);

	// Reserve deposit
	let deposit = T::KycApplicationDeposit::get();
	let _ = T::Currency::reserve(applicant, deposit);
}

/// Helper function to setup an applicant in ReferrerApproved state
fn setup_referrer_approved<T: Config>(applicant: &T::AccountId, referrer: &T::AccountId) {
	let identity_hash = H256::repeat_byte(0x01);
	let application = CitizenshipApplication {
		identity_hash,
		referrer: referrer.clone(),
		inviter: None,
		applied_at: pezframe_system::Pezpallet::<T>::block_number(),
	};
	Applications::<T>::insert(applicant, application);
	KycStatuses::<T>::insert(applicant, KycLevel::ReferrerApproved);

	// Reserve deposit
	let deposit = T::KycApplicationDeposit::get();
	let _ = T::Currency::reserve(applicant, deposit);
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn apply_for_citizenship() {
		let referrer: T::AccountId = funded_account::<T>("referrer", 0);
		setup_citizen::<T>(&referrer);

		let applicant: T::AccountId = funded_account::<T>("applicant", 1);
		let identity_hash = H256::repeat_byte(0x42);

		#[extrinsic_call]
		apply_for_citizenship(
			RawOrigin::Signed(applicant.clone()),
			identity_hash,
			Some(referrer.clone()),
			None,
		);

		assert_eq!(KycStatuses::<T>::get(&applicant), KycLevel::PendingReferral);
	}

	#[benchmark]
	fn approve_referral() {
		let referrer: T::AccountId = funded_account::<T>("referrer", 0);
		setup_citizen::<T>(&referrer);

		let applicant: T::AccountId = funded_account::<T>("applicant", 1);
		setup_pending_referral::<T>(&applicant, &referrer);

		#[extrinsic_call]
		approve_referral(RawOrigin::Signed(referrer.clone()), applicant.clone());

		assert_eq!(KycStatuses::<T>::get(&applicant), KycLevel::ReferrerApproved);
	}

	#[benchmark]
	fn confirm_citizenship() {
		let referrer: T::AccountId = funded_account::<T>("referrer", 0);
		setup_citizen::<T>(&referrer);

		let applicant: T::AccountId = funded_account::<T>("applicant", 1);
		setup_referrer_approved::<T>(&applicant, &referrer);

		#[extrinsic_call]
		confirm_citizenship(RawOrigin::Signed(applicant.clone()));

		assert_eq!(KycStatuses::<T>::get(&applicant), KycLevel::Approved);
	}

	#[benchmark]
	fn revoke_citizenship() {
		let citizen: T::AccountId = funded_account::<T>("citizen", 0);
		setup_citizen::<T>(&citizen);

		#[extrinsic_call]
		revoke_citizenship(RawOrigin::Root, citizen.clone());

		assert_eq!(KycStatuses::<T>::get(&citizen), KycLevel::Revoked);
	}

	#[benchmark]
	fn renounce_citizenship() {
		let citizen: T::AccountId = funded_account::<T>("citizen", 0);
		setup_citizen::<T>(&citizen);

		#[extrinsic_call]
		renounce_citizenship(RawOrigin::Signed(citizen.clone()));

		assert_eq!(KycStatuses::<T>::get(&citizen), KycLevel::NotStarted);
	}

	#[benchmark]
	fn cancel_application() {
		let referrer: T::AccountId = funded_account::<T>("referrer", 0);
		setup_citizen::<T>(&referrer);

		let applicant: T::AccountId = funded_account::<T>("applicant", 1);
		setup_pending_referral::<T>(&applicant, &referrer);

		#[extrinsic_call]
		cancel_application(RawOrigin::Signed(applicant.clone()));

		assert_eq!(KycStatuses::<T>::get(&applicant), KycLevel::NotStarted);
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
