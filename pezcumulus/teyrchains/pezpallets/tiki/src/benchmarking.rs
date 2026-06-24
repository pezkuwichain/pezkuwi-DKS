// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-tiki
use super::*;

#[allow(unused)]
use crate::Pezpallet as Tiki;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;
// Import the required traits
use pezframe_support::traits::{Currency, Get};
use pezpallet_balances::Pezpallet as Balances;
use pezsp_runtime::traits::StaticLookup;
extern crate alloc;

// Add the required trait bounds to the main benchmarks block.
#[benchmarks(
	where
		T::CollectionId: Copy + Default + PartialOrd,
		T: pezpallet_balances::Config,
)]
mod benchmarks {
	use super::*;

	// This helper function creates the Tiki collection defined in the runtime.
	fn ensure_collection_exists<T: Config + pezpallet_balances::Config>()
	where
		T::CollectionId: Default + PartialOrd,
	{
		let collection_id = T::TikiCollectionId::get();
		// Use the funded `whitelisted_caller` as the collection owner.
		let caller: T::AccountId = whitelisted_caller();

		// Fund the caller account with sufficient balance for NFT deposits
		// Use a very large balance to ensure all deposit requirements can be met
		let funding = Balances::<T>::minimum_balance() * 1_000_000_000u32.into();
		Balances::<T>::make_free_balance_be(&caller, funding);

		// The `while` loop removes the need for the 'Step' trait.
		while pezpallet_nfts::NextCollectionId::<T>::get().unwrap_or_default() <= collection_id {
			let _ = pezpallet_nfts::Pezpallet::<T>::force_create(
				RawOrigin::Root.into(),
				T::Lookup::unlookup(caller.clone()),
				pezpallet_nfts::CollectionConfig {
					settings: Default::default(),
					max_supply: None,
					mint_settings: Default::default(),
				},
			);
		}
	}

	// Helper to ensure user has a citizen NFT
	fn ensure_citizen_nft<T: Config + pezpallet_balances::Config>(
		who: T::AccountId,
	) -> Result<(), DispatchError>
	where
		T::CollectionId: Default + PartialOrd,
	{
		ensure_collection_exists::<T>();

		// Fund the user account with sufficient balance for NFT deposits
		// Use a very large balance to ensure all deposit requirements can be met
		let funding = Balances::<T>::minimum_balance() * 1_000_000_000u32.into();
		Balances::<T>::make_free_balance_be(&who, funding);

		if Tiki::<T>::citizen_nft(&who).is_none() {
			Tiki::<T>::mint_citizen_nft_for_user(&who)?;
		}
		Ok(())
	}

	#[benchmark]
	fn grant_tiki() -> Result<(), BenchmarkError> {
		// Use the funded `whitelisted_caller` as the 'dest' account that will receive the NFT.
		let dest: T::AccountId = whitelisted_caller();
		// Use an appointed role (Wezir instead of Serok)
		let tiki = crate::Tiki::Wezir;

		// Ensure the dest account has a citizen NFT before granting a tiki
		ensure_citizen_nft::<T>(dest.clone())?;

		#[extrinsic_call]
		_(RawOrigin::Root, T::Lookup::unlookup(dest.clone()), tiki);

		// For non-unique roles, check user has the role
		assert!(Tiki::<T>::user_tikis(&dest).contains(&tiki));
		Ok(())
	}

	#[benchmark]
	fn revoke_tiki() -> Result<(), BenchmarkError> {
		// Use the funded `whitelisted_caller` as the 'dest' account that will receive the NFT.
		let dest: T::AccountId = whitelisted_caller();
		let tiki = crate::Tiki::Wezir; // Use appointed role

		// Ensure the dest account has a citizen NFT and the tiki before revoking
		ensure_citizen_nft::<T>(dest.clone())?;
		Tiki::<T>::internal_grant_role(&dest, tiki)?; // Use internal function to grant without origin check

		// Verify the role was granted
		assert!(Tiki::<T>::user_tikis(&dest).contains(&tiki));

		#[extrinsic_call]
		_(RawOrigin::Root, T::Lookup::unlookup(dest.clone()), tiki);

		// User should no longer have this role
		assert!(!Tiki::<T>::user_tikis(&dest).contains(&tiki));
		Ok(())
	}

	#[benchmark]
	fn force_mint_citizen_nft() -> Result<(), BenchmarkError> {
		let dest: T::AccountId = whitelisted_caller();

		// Ensure collection exists first
		ensure_collection_exists::<T>();

		// Should not be a citizen yet
		assert!(Tiki::<T>::citizen_nft(&dest).is_none());

		#[extrinsic_call]
		_(RawOrigin::Root, T::Lookup::unlookup(dest.clone()));

		// Ensure they are now a citizen
		assert!(Tiki::<T>::citizen_nft(&dest).is_some());
		assert!(Tiki::<T>::is_citizen(&dest));

		Ok(())
	}

	#[benchmark]
	fn grant_earned_role() -> Result<(), BenchmarkError> {
		let dest: T::AccountId = whitelisted_caller();
		let tiki = crate::Tiki::Axa; // An earned role

		// Precondition: must be a citizen
		ensure_citizen_nft::<T>(dest.clone())?;

		#[extrinsic_call]
		_(RawOrigin::Root, T::Lookup::unlookup(dest.clone()), tiki);

		// Verify the role was granted
		assert!(Tiki::<T>::has_tiki(&dest, &tiki));

		Ok(())
	}

	#[benchmark]
	fn grant_elected_role() -> Result<(), BenchmarkError> {
		let dest: T::AccountId = whitelisted_caller();
		let tiki = crate::Tiki::Parlementer; // An elected role

		// Precondition: must be a citizen
		ensure_citizen_nft::<T>(dest.clone())?;

		#[extrinsic_call]
		_(RawOrigin::Root, T::Lookup::unlookup(dest.clone()), tiki);

		// Verify the role was granted
		assert!(Tiki::<T>::has_tiki(&dest, &tiki));

		Ok(())
	}

	#[benchmark]
	fn apply_for_citizenship() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();

		// Fund the caller
		let funding = Balances::<T>::minimum_balance() * 1_000_000_000u32.into();
		Balances::<T>::make_free_balance_be(&caller, funding);

		// Ensure collection exists
		ensure_collection_exists::<T>();

		// Set KYC status to Approved directly in storage
		pezpallet_identity_kyc::KycStatuses::<T>::insert(
			&caller,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()));

		// Verify citizenship was granted
		assert!(Tiki::<T>::is_citizen(&caller));
		Ok(())
	}

	#[benchmark]
	fn check_transfer_permission() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let dest: T::AccountId = account("dest", 0, 0);

		// Ensure collections exist past tiki collection so we have a valid non-tiki ID
		ensure_collection_exists::<T>();

		// NextCollectionId is past tiki, so it's a non-tiki collection (call succeeds)
		let non_tiki_id = pezpallet_nfts::NextCollectionId::<T>::get().unwrap_or_default();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), non_tiki_id, 0u32, caller.clone(), dest);

		Ok(())
	}

	impl_benchmark_test_suite!(Tiki, crate::mock::new_test_ext(), crate::mock::Test);
}
