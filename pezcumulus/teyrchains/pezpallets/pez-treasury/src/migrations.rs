// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Storage migrations for pezpallet-pez-treasury

use super::*;
use pezframe_support::{
	traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};
use pezsp_std::marker::PhantomData;

/// Current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

/// Migration from v0 to v1
/// This migration handles the initial version setup for pezpallet-pez-treasury
pub mod v1 {
	use super::*;

	pub struct MigrateToV1<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			let current = Pezpallet::<T>::on_chain_storage_version();

			log::info!(
				"🔄 Running migration for pezpallet-pez-treasury from {current:?} to {STORAGE_VERSION:?}"
			);

			if current == StorageVersion::new(0) {
				let mut weight = Weight::zero();

				// Example migration logic for treasury storage
				// If storage format changes in the future, implement transformation here

				// Count existing storage items for logging
				let monthly_releases_count = MonthlyReleases::<T>::iter().count() as u64;
				let has_halving_info = if HalvingInfo::<T>::exists() { 1u64 } else { 0u64 };
				let has_treasury_start =
					if TreasuryStartBlock::<T>::get().is_some() { 1u64 } else { 0u64 };
				let has_genesis_done =
					if GenesisDistributionDone::<T>::get() { 1u64 } else { 0u64 };

				let migrated = monthly_releases_count
					+ has_halving_info
					+ has_treasury_start
					+ has_genesis_done;

				// Update storage version
				STORAGE_VERSION.put::<Pezpallet<T>>();

				log::info!("✅ Migrated {migrated} entries in pezpallet-pez-treasury");
				log::info!("   MonthlyReleases: {monthly_releases_count}, HalvingInfo: {has_halving_info}, TreasuryStartBlock: {has_treasury_start}, GenesisDistributionDone: {has_genesis_done}");

				// Return weight used
				// Reads: all storage items + version read
				// Writes: version write
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(migrated + 1, 1));

				weight
			} else {
				log::info!(
					"👌 pezpallet-pez-treasury migration not needed, current version is {current:?}"
				);
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<pezsp_std::vec::Vec<u8>, pezsp_runtime::TryRuntimeError> {
			use codec::Encode;
			let current = Pezpallet::<T>::on_chain_storage_version();

			log::info!("🔍 Pre-upgrade check for pezpallet-pez-treasury");
			log::info!("   Current version: {current:?}");

			// Encode current storage counts for verification
			let monthly_releases_count = MonthlyReleases::<T>::iter().count() as u32;
			let next_release_month = NextReleaseMonth::<T>::get();
			let has_treasury_start = TreasuryStartBlock::<T>::get().is_some();
			let genesis_done = GenesisDistributionDone::<T>::get();

			log::info!("   MonthlyReleases entries: {monthly_releases_count}");
			log::info!("   NextReleaseMonth: {next_release_month}");
			log::info!("   TreasuryStartBlock exists: {has_treasury_start}");
			log::info!("   GenesisDistributionDone: {genesis_done}");

			Ok((monthly_releases_count, next_release_month, has_treasury_start, genesis_done)
				.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			state: pezsp_std::vec::Vec<u8>,
		) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use codec::Decode;

			let (
				pre_monthly_releases_count,
				pre_next_release_month,
				pre_has_treasury_start,
				pre_genesis_done,
			): (u32, u32, bool, bool) = Decode::decode(&mut &state[..])
				.map_err(|_| "Failed to decode pre-upgrade state")?;

			log::info!("🔍 Post-upgrade check for pezpallet-pez-treasury");

			// Verify storage version was updated
			let current_version = Pezpallet::<T>::on_chain_storage_version();
			assert_eq!(current_version, STORAGE_VERSION, "Storage version not updated correctly");
			log::info!("✅ Storage version updated to {current_version:?}");

			// Verify storage counts (should be same or more, never less)
			let post_monthly_releases_count = MonthlyReleases::<T>::iter().count() as u32;
			let post_next_release_month = NextReleaseMonth::<T>::get();
			let post_has_treasury_start = TreasuryStartBlock::<T>::get().is_some();
			let post_genesis_done = GenesisDistributionDone::<T>::get();

			log::info!(
				"   MonthlyReleases entries: {pre_monthly_releases_count} -> {post_monthly_releases_count}"
			);
			log::info!(
				"   NextReleaseMonth: {pre_next_release_month} -> {post_next_release_month}"
			);
			log::info!(
				"   TreasuryStartBlock exists: {pre_has_treasury_start} -> {post_has_treasury_start}"
			);
			log::info!("   GenesisDistributionDone: {pre_genesis_done} -> {post_genesis_done}");

			// Verify no data was lost
			assert!(
				post_monthly_releases_count >= pre_monthly_releases_count,
				"MonthlyReleases entries decreased during migration"
			);

			// NextReleaseMonth should not decrease
			assert!(
				post_next_release_month >= pre_next_release_month,
				"NextReleaseMonth decreased during migration"
			);

			// Treasury start block should not be removed if it existed
			if pre_has_treasury_start {
				assert!(post_has_treasury_start, "TreasuryStartBlock was removed during migration");
			}

			// Genesis done flag should not change from true to false
			if pre_genesis_done {
				assert!(post_genesis_done, "GenesisDistributionDone was reset during migration");
			}

			log::info!("✅ Post-upgrade checks passed for pezpallet-pez-treasury");
			Ok(())
		}
	}
}

/// Example migration for future version changes
/// This demonstrates how to handle storage format changes in treasury data
pub mod v2 {
	use super::*;

	/// Example: Migration when halving data or release format changes
	pub struct MigrateToV2<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV2<T> {
		fn on_runtime_upgrade() -> Weight {
			let current = Pezpallet::<T>::on_chain_storage_version();

			if current < StorageVersion::new(2) {
				log::info!("🔄 Running migration for pezpallet-pez-treasury to v2");

				// Example migration logic
				// 1. Transform halving data if format changed
				// 2. Migrate monthly release records if needed
				// 3. Update version

				// For now, this is just a template
				StorageVersion::new(2).put::<Pezpallet<T>>();

				log::info!("✅ Completed migration to pezpallet-pez-treasury v2");

				T::DbWeight::get().reads_writes(1, 1)
			} else {
				log::info!("👌 pezpallet-pez-treasury v2 migration not needed");
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<pezsp_std::vec::Vec<u8>, pezsp_runtime::TryRuntimeError> {
			log::info!("🔍 Pre-upgrade check for pezpallet-pez-treasury v2");
			Ok(pezsp_std::vec::Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			_state: pezsp_std::vec::Vec<u8>,
		) -> Result<(), pezsp_runtime::TryRuntimeError> {
			log::info!("✅ Post-upgrade check passed for pezpallet-pez-treasury v2");
			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use pezframe_support::traits::OnRuntimeUpgrade;

	#[test]
	fn test_migration_v1() {
		new_test_ext().execute_with(|| {
			// Set initial storage version to 0
			StorageVersion::new(0).put::<Pezpallet<Test>>();

			// Run migration
			let weight = v1::MigrateToV1::<Test>::on_runtime_upgrade();

			// Verify version was updated
			assert_eq!(Pezpallet::<Test>::on_chain_storage_version(), STORAGE_VERSION);

			// Verify weight is non-zero
			assert!(weight != Weight::zero());
		});
	}

	#[test]
	fn test_migration_idempotent() {
		new_test_ext().execute_with(|| {
			// Set current version
			STORAGE_VERSION.put::<Pezpallet<Test>>();

			// Run migration again
			let weight = v1::MigrateToV1::<Test>::on_runtime_upgrade();

			// Should be a no-op
			assert_eq!(weight, pezframe_support::weights::constants::RocksDbWeight::get().reads(1));
		});
	}
}
