// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Storage migrations for pezpallet-staking-score.

use crate::*;
use core::marker::PhantomData;
use pezframe_support::{
	pezpallet_prelude::*,
	traits::{GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
};

/// Version 2: tier crossings are written down when a clock starts.
///
/// A clock started under version 1 has no crossings written, so its next tier would only reach
/// a derived score when a noter next confirmed the stake. This writes them for every account
/// whose clock is running. One pass over the opt-ins, which on Zagros is a hundred; a chain
/// founded on this code writes version 2 at genesis and never runs it.
pub mod v2 {
	use super::*;

	pub struct ScheduleTierCrossings<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for ScheduleTierCrossings<T> {
		fn on_runtime_upgrade() -> Weight {
			let current = Pezpallet::<T>::on_chain_storage_version();
			if current != StorageVersion::new(1) {
				log::info!(
					"👌 pezpallet-staking-score v2 not needed, current version is {current:?}"
				);
				return T::DbWeight::get().reads(1);
			}

			let mut clocks = 0u64;
			for who in StakingStartBlock::<T>::iter_keys() {
				clocks += 1;
				Pezpallet::<T>::schedule_tier_crossings(&who);
			}
			// Where the announcements start: here, so none of the blocks before the upgrade
			// are looked through.
			NextCrossingBlock::<T>::put(pezframe_system::Pezpallet::<T>::block_number());

			StorageVersion::new(2).put::<Pezpallet<T>>();
			log::info!("pezpallet-staking-score v2: tier crossings written for {clocks} clocks");
			// Per clock: the opt-in, the first sighting, the schedule mark; the mark and up to
			// four crossings.
			T::DbWeight::get().reads_writes(1 + 3 * clocks, 2 + 5 * clocks)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<pezsp_std::vec::Vec<u8>, pezsp_runtime::TryRuntimeError> {
			use codec::Encode;
			Ok((StakingStartBlock::<T>::iter_keys().count() as u32).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			state: pezsp_std::vec::Vec<u8>,
		) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use codec::Decode;
			use pezframe_support::ensure;

			let clocks_before =
				u32::decode(&mut &state[..]).map_err(|_| "Failed to decode pre-upgrade state")?;
			ensure!(
				StakingStartBlock::<T>::iter_keys().count() as u32 == clocks_before,
				"the migration changed whose clock is running"
			);
			for who in StakingStartBlock::<T>::iter_keys() {
				if let Some(origin) = Pezpallet::<T>::duration_origin(&who) {
					ensure!(
						CrossingsScheduledFrom::<T>::get(&who) == Some(origin),
						"a running clock has no crossings written"
					);
				}
			}
			ensure!(
				Pezpallet::<T>::on_chain_storage_version() == StorageVersion::new(2),
				"pezpallet-staking-score is not at version 2"
			);
			Ok(())
		}
	}
}
