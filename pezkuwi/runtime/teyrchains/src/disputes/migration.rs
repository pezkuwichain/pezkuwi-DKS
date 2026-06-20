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

//! Storage migration(s) related to disputes pezpallet

use pezframe_support::traits::StorageVersion;

pub mod v1 {
	use super::*;
	use crate::disputes::{Config, Pezpallet};
	use alloc::vec::Vec;
	use pezframe_support::{
		pezpallet_prelude::*, storage_alias, traits::OnRuntimeUpgrade, weights::Weight,
	};
	use pezkuwi_primitives::SessionIndex;

	#[storage_alias]
	type SpamSlots<T: Config> = StorageMap<Pezpallet<T>, Twox64Concat, SessionIndex, Vec<u32>>;

	pub struct MigrateToV1<T>(core::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			let mut weight: Weight = Weight::zero();

			if StorageVersion::get::<Pezpallet<T>>() < 1 {
				log::info!(target: crate::disputes::LOG_TARGET, "Migrating disputes storage to v1");
				weight += migrate_to_v1::<T>();
				StorageVersion::new(1).put::<Pezpallet<T>>();
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			} else {
				log::info!(
					target: crate::disputes::LOG_TARGET,
					"Disputes storage up to date - no need for migration"
				);
			}

			weight
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, pezsp_runtime::TryRuntimeError> {
			log::trace!(
				target: crate::disputes::LOG_TARGET,
				"SpamSlots before migration: {}",
				SpamSlots::<T>::iter().count()
			);
			ensure!(
				StorageVersion::get::<Pezpallet<T>>() == 0,
				"Storage version should be less than `1` before the migration",
			);
			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_state: Vec<u8>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			log::trace!(target: crate::disputes::LOG_TARGET, "Running post_upgrade()");
			ensure!(
				StorageVersion::get::<Pezpallet<T>>() >= 1,
				"Storage version should be `1` after the migration"
			);
			ensure!(
				SpamSlots::<T>::iter().count() == 0,
				"SpamSlots should be empty after the migration"
			);
			Ok(())
		}
	}

	/// Migrates the pezpallet storage to the most recent version, checking and setting the
	/// `StorageVersion`.
	pub fn migrate_to_v1<T: Config>() -> Weight {
		let mut weight: Weight = Weight::zero();

		// SpamSlots should not contain too many keys so removing everything at once should be safe
		let res = SpamSlots::<T>::clear(u32::MAX, None);
		// `loops` is the number of iterations => used to calculate read weights
		// `backend` is the number of keys removed from the backend => used to calculate write
		// weights
		weight = weight
			.saturating_add(T::DbWeight::get().reads_writes(res.loops as u64, res.backend as u64));

		weight
	}
}
