// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Storage migrations for pezpallet-tiki

use super::*;
use pezframe_support::{
	traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};
use pezsp_std::marker::PhantomData;

/// Current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

/// Migration from v0 to v1
/// This is a template migration that can be customized based on actual storage changes
pub mod v1 {
	use super::*;

	pub struct MigrateToV1<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			let current = Pezpallet::<T>::on_chain_storage_version();

			log::info!(
				"🔄 Running migration for pezpallet-tiki from {current:?} to {STORAGE_VERSION:?}"
			);

			if current == StorageVersion::new(0) {
				let migrated = 0u64;
				let mut weight = Weight::zero();

				// Update storage version to v1
				StorageVersion::new(1).put::<Pezpallet<T>>();

				log::info!("✅ Migrated {migrated} entries in pezpallet-tiki");

				// Return weight used
				// Reads: migrated items + version read
				// Writes: migrated items + version write
				weight = weight
					.saturating_add(T::DbWeight::get().reads_writes(migrated + 1, migrated + 1));

				weight
			} else {
				log::info!(
					"👌 pezpallet-tiki migration not needed, current version is {current:?}"
				);
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<pezsp_std::vec::Vec<u8>, pezsp_runtime::TryRuntimeError> {
			use codec::Encode;

			let current = Pezpallet::<T>::on_chain_storage_version();

			log::info!("🔍 Pre-upgrade check for pezpallet-tiki");
			log::info!("   Current version: {current:?}");

			// Encode current storage counts for verification
			let citizen_count = CitizenNft::<T>::iter().count() as u32;
			let user_tikis_count = UserTikis::<T>::iter().count() as u32;
			let tiki_holder_count = TikiHolder::<T>::iter().count() as u32;

			log::info!("   CitizenNft entries: {citizen_count}");
			log::info!("   UserTikis entries: {user_tikis_count}");
			log::info!("   TikiHolder entries: {tiki_holder_count}");

			Ok((citizen_count, user_tikis_count, tiki_holder_count).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			state: pezsp_std::vec::Vec<u8>,
		) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use codec::Decode;

			let (pre_citizen_count, pre_user_tikis_count, pre_tiki_holder_count): (u32, u32, u32) =
				Decode::decode(&mut &state[..]).map_err(|_| "Failed to decode pre-upgrade state")?;

			log::info!("🔍 Post-upgrade check for pezpallet-tiki");

			// Verify storage version was updated
			let current_version = Pezpallet::<T>::on_chain_storage_version();
			assert_eq!(current_version, STORAGE_VERSION, "Storage version not updated correctly");
			log::info!("✅ Storage version updated to {current_version:?}");

			// Verify storage counts (should be same or more, never less)
			let post_citizen_count = CitizenNft::<T>::iter().count() as u32;
			let post_user_tikis_count = UserTikis::<T>::iter().count() as u32;
			let post_tiki_holder_count = TikiHolder::<T>::iter().count() as u32;

			log::info!("   CitizenNft entries: {pre_citizen_count} -> {post_citizen_count}");
			log::info!("   UserTikis entries: {pre_user_tikis_count} -> {post_user_tikis_count}");
			log::info!(
				"   TikiHolder entries: {pre_tiki_holder_count} -> {post_tiki_holder_count}"
			);

			assert!(
				post_citizen_count >= pre_citizen_count,
				"CitizenNft entries decreased during migration"
			);
			assert!(
				post_user_tikis_count >= pre_user_tikis_count,
				"UserTikis entries decreased during migration"
			);
			assert!(
				post_tiki_holder_count >= pre_tiki_holder_count,
				"TikiHolder entries decreased during migration"
			);

			log::info!("✅ Post-upgrade checks passed for pezpallet-tiki");
			Ok(())
		}
	}
}

/// Migration v1 -> v2: Populate TikiHolder from UserTikis
/// Fixes: TikiHolder was empty despite unique roles (Serok) being assigned.
/// This scans all UserTikis entries and populates TikiHolder for unique roles.
pub mod v2 {
	use super::*;

	pub struct MigrateToV2<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
		fn on_runtime_upgrade() -> Weight {
			let current = Pezpallet::<T>::on_chain_storage_version();

			if current < StorageVersion::new(2) {
				log::info!("Running migration for pezpallet-tiki v1 -> v2");

				let mut reads = 1u64; // version read
				let mut writes = 1u64; // version write
				let mut holders_fixed = 0u32;

				// Scan all UserTikis to find unique role holders
				for (account, tikis) in UserTikis::<T>::iter() {
					reads += 1;
					for tiki in tikis.iter() {
						if Pezpallet::<T>::is_unique_role(tiki) {
							// Only set if not already populated
							if TikiHolder::<T>::get(tiki).is_none() {
								TikiHolder::<T>::insert(tiki, account.clone());
								holders_fixed += 1;
								writes += 1;
							}
							reads += 1;
						}
					}
				}

				STORAGE_VERSION.put::<Pezpallet<T>>();

				log::info!(
					"Completed pezpallet-tiki v2 migration: fixed {holders_fixed} TikiHolder entries"
				);

				T::DbWeight::get().reads_writes(reads, writes)
			} else {
				log::info!("pezpallet-tiki v2 migration not needed, current: {current:?}");
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<pezsp_std::vec::Vec<u8>, pezsp_runtime::TryRuntimeError> {
			use codec::Encode;

			let tiki_holder_count = TikiHolder::<T>::iter().count() as u32;
			log::info!("Pre-upgrade: TikiHolder entries = {tiki_holder_count}");

			Ok(tiki_holder_count.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			state: pezsp_std::vec::Vec<u8>,
		) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use codec::Decode;

			let pre_count: u32 = Decode::decode(&mut &state[..])
				.map_err(|_| "Failed to decode pre-upgrade state")?;
			let post_count = TikiHolder::<T>::iter().count() as u32;

			log::info!("Post-upgrade: TikiHolder {pre_count} -> {post_count}");

			// Should have at least as many entries as before
			assert!(post_count >= pre_count, "TikiHolder entries decreased during migration");

			// Verify consistency: every unique role in UserTikis has a TikiHolder entry
			for (account, tikis) in UserTikis::<T>::iter() {
				for tiki in tikis.iter() {
					if Pezpallet::<T>::is_unique_role(tiki) {
						let holder = TikiHolder::<T>::get(tiki);
						assert!(
							holder.is_some(),
							"Unique role missing from TikiHolder after migration"
						);
						assert_eq!(holder.unwrap(), account, "TikiHolder mismatch for unique role");
					}
				}
			}

			assert_eq!(
				Pezpallet::<T>::on_chain_storage_version(),
				STORAGE_VERSION,
				"Storage version not updated"
			);

			log::info!("Post-upgrade checks passed for pezpallet-tiki v2");
			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		mock::{new_test_ext, Test},
		pezpallet::{CitizenNft, Tiki, TikiHolder, UserTikis},
	};
	use pezframe_support::traits::OnRuntimeUpgrade;

	#[test]
	fn test_migration_v1() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(0).put::<Pezpallet<Test>>();

			let weight = v1::MigrateToV1::<Test>::on_runtime_upgrade();

			assert_eq!(Pezpallet::<Test>::on_chain_storage_version(), StorageVersion::new(1));
			assert!(weight != Weight::zero());
		});
	}

	#[test]
	fn test_migration_v2_populates_tiki_holder() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(1).put::<Pezpallet<Test>>();

			// Simulate on-chain state: account 1 has Serok in UserTikis but TikiHolder is empty
			let account: u64 = 1;
			CitizenNft::<Test>::insert(account, 0u32);
			UserTikis::<Test>::mutate(account, |tikis| {
				let _ = tikis.try_push(Tiki::Welati);
				let _ = tikis.try_push(Tiki::Serok);
			});

			// TikiHolder should be empty before migration
			assert!(TikiHolder::<Test>::get(Tiki::Serok).is_none());

			// Run migration
			let weight = v2::MigrateToV2::<Test>::on_runtime_upgrade();

			// TikiHolder should now have Serok -> account 1
			assert_eq!(TikiHolder::<Test>::get(Tiki::Serok), Some(account));
			assert_eq!(Pezpallet::<Test>::on_chain_storage_version(), STORAGE_VERSION);
			assert!(weight != Weight::zero());
		});
	}

	#[test]
	fn test_migration_v2_idempotent() {
		new_test_ext().execute_with(|| {
			STORAGE_VERSION.put::<Pezpallet<Test>>();

			let weight = v2::MigrateToV2::<Test>::on_runtime_upgrade();

			// Should be a no-op
			assert_eq!(weight, pezframe_support::weights::constants::RocksDbWeight::get().reads(1));
		});
	}

	#[test]
	fn test_migration_v2_multiple_unique_roles() {
		new_test_ext().execute_with(|| {
			StorageVersion::new(1).put::<Pezpallet<Test>>();

			// Account 1: Serok
			CitizenNft::<Test>::insert(1u64, 0u32);
			UserTikis::<Test>::mutate(1u64, |tikis| {
				let _ = tikis.try_push(Tiki::Welati);
				let _ = tikis.try_push(Tiki::Serok);
			});

			// Account 2: SerokiMeclise
			CitizenNft::<Test>::insert(2u64, 1u32);
			UserTikis::<Test>::mutate(2u64, |tikis| {
				let _ = tikis.try_push(Tiki::Welati);
				let _ = tikis.try_push(Tiki::SerokiMeclise);
			});

			// Account 3: just Welati (no unique role)
			CitizenNft::<Test>::insert(3u64, 2u32);
			UserTikis::<Test>::mutate(3u64, |tikis| {
				let _ = tikis.try_push(Tiki::Welati);
			});

			v2::MigrateToV2::<Test>::on_runtime_upgrade();

			assert_eq!(TikiHolder::<Test>::get(Tiki::Serok), Some(1u64));
			assert_eq!(TikiHolder::<Test>::get(Tiki::SerokiMeclise), Some(2u64));
			assert!(TikiHolder::<Test>::get(Tiki::Xezinedar).is_none());
			assert!(TikiHolder::<Test>::get(Tiki::Balyoz).is_none());
		});
	}
}
