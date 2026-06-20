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

//! This pezpallet used to implement a message queue for downward messages from the relay-chain.
//!
//! It is now deprecated and has been refactored to simply drain any remaining messages into
//! something implementing `HandleMessage`. It proceeds in the state of
//! [`MigrationState`] one by one by their listing in the source code. The pezpallet can be removed
//! from the runtime once `Completed` was emitted.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated)] // The pezpallet itself is deprecated.

extern crate alloc;

use migration::*;
pub use pezpallet::*;

mod benchmarking;
mod migration;
mod mock;
mod tests;
pub mod weights;

pub use weights::WeightInfo;

/// The maximal length of a DMP message.
pub type MaxDmpMessageLenOf<T> =
	<<T as Config>::DmpSink as pezframe_support::traits::HandleMessage>::MaxMessageLen;

#[pezframe_support::pezpallet]
#[deprecated(
	note = "`pezcumulus-pezpallet-dmp-queue` will be removed after November 2024. It can be removed once its lazy migration completed. See <https://github.com/pezkuwichain/pezkuwi-sdk/issues/247>."
)]
pub mod pezpallet {
	use super::*;
	use pezframe_support::{pezpallet_prelude::*, traits::HandleMessage, weights::WeightMeter};
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_io::hashing::twox_128;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		/// The overarching event type of the runtime.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;

		/// The sink for all DMP messages that the lazy migration will use.
		type DmpSink: HandleMessage;

		/// Weight info for this pezpallet (only needed for the lazy migration).
		type WeightInfo: WeightInfo;
	}

	/// The migration state of this pezpallet.
	#[pezpallet::storage]
	pub type MigrationStatus<T> = StorageValue<_, MigrationState, ValueQuery>;

	/// The lazy-migration state of the pezpallet.
	#[derive(
		codec::Encode, codec::Decode, Debug, PartialEq, Eq, Clone, MaxEncodedLen, TypeInfo,
	)]
	pub enum MigrationState {
		/// Migration has not started yet.
		NotStarted,
		/// The export of pages started.
		StartedExport {
			/// The next page that should be exported.
			next_begin_used: PageCounter,
		},
		/// The page export completed.
		CompletedExport,
		/// The export of overweight messages started.
		StartedOverweightExport {
			/// The next overweight index that should be exported.
			next_overweight_index: u64,
		},
		/// The export of overweight messages completed.
		CompletedOverweightExport,
		/// The storage cleanup started.
		StartedCleanup { cursor: Option<BoundedVec<u8, ConstU32<1024>>> },
		/// The migration finished. The pezpallet can now be removed from the runtime.
		Completed,
	}

	impl Default for MigrationState {
		fn default() -> Self {
			Self::NotStarted
		}
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The export of pages started.
		StartedExport,

		/// The export of a page completed.
		Exported { page: PageCounter },

		/// The export of a page failed.
		///
		/// This should never be emitted.
		ExportFailed { page: PageCounter },

		/// The export of pages completed.
		CompletedExport,

		/// The export of overweight messages started.
		StartedOverweightExport,

		/// The export of an overweight message completed.
		ExportedOverweight { index: OverweightIndex },

		/// The export of an overweight message failed.
		///
		/// This should never be emitted.
		ExportOverweightFailed { index: OverweightIndex },

		/// The export of overweight messages completed.
		CompletedOverweightExport,

		/// The cleanup of remaining pezpallet storage started.
		StartedCleanup,

		/// Some debris was cleaned up.
		CleanedSome { keys_removed: u32 },

		/// The cleanup of remaining pezpallet storage completed.
		Completed { error: bool },
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		fn integrity_test() {
			let w = Self::on_idle_weight();
			assert!(w != Weight::zero());
			assert!(w.all_lte(T::BlockWeights::get().max_block));
		}

		fn on_idle(now: BlockNumberFor<T>, limit: Weight) -> Weight {
			let mut meter = WeightMeter::with_limit(limit);

			if meter.try_consume(Self::on_idle_weight()).is_err() {
				log::debug!(target: LOG, "Not enough weight for on_idle. {limit:?} < {:?}", Self::on_idle_weight());
				return meter.consumed();
			}

			let state = MigrationStatus::<T>::get();
			let index = PageIndex::<T>::get();
			log::debug!(target: LOG, "on_idle: block={:?}, state={:?}, index={:?}", now, state, index);

			match state {
				MigrationState::NotStarted => {
					log::debug!(target: LOG, "Init export at page {}", index.begin_used);

					MigrationStatus::<T>::put(MigrationState::StartedExport {
						next_begin_used: index.begin_used,
					});
					Self::deposit_event(Event::StartedExport);
				},
				MigrationState::StartedExport { next_begin_used } => {
					log::debug!(target: LOG, "Exporting page {next_begin_used}");

					if next_begin_used == index.end_used {
						MigrationStatus::<T>::put(MigrationState::CompletedExport);
						log::debug!(target: LOG, "CompletedExport");
						Self::deposit_event(Event::CompletedExport);
					} else {
						let res = migration::migrate_page::<T>(next_begin_used);

						MigrationStatus::<T>::put(MigrationState::StartedExport {
							next_begin_used: next_begin_used.saturating_add(1),
						});

						if let Ok(()) = res {
							log::debug!(target: LOG, "Exported page {next_begin_used}");
							Self::deposit_event(Event::Exported { page: next_begin_used });
						} else {
							Self::deposit_event(Event::ExportFailed { page: next_begin_used });
						}
					}
				},
				MigrationState::CompletedExport => {
					log::debug!(target: LOG, "Init export overweight at index 0");

					MigrationStatus::<T>::put(MigrationState::StartedOverweightExport {
						next_overweight_index: 0,
					});
					Self::deposit_event(Event::StartedOverweightExport);
				},
				MigrationState::StartedOverweightExport { next_overweight_index } => {
					log::debug!(target: LOG, "Exporting overweight index {next_overweight_index}");

					if next_overweight_index == index.overweight_count {
						MigrationStatus::<T>::put(MigrationState::CompletedOverweightExport);
						log::debug!(target: LOG, "CompletedOverweightExport");
						Self::deposit_event(Event::CompletedOverweightExport);
					} else {
						let res = migration::migrate_overweight::<T>(next_overweight_index);

						MigrationStatus::<T>::put(MigrationState::StartedOverweightExport {
							next_overweight_index: next_overweight_index.saturating_add(1),
						});

						if let Ok(()) = res {
							log::debug!(target: LOG, "Exported overweight index {next_overweight_index}");
							Self::deposit_event(Event::ExportedOverweight {
								index: next_overweight_index,
							});
						} else {
							Self::deposit_event(Event::ExportOverweightFailed {
								index: next_overweight_index,
							});
						}
					}
				},
				MigrationState::CompletedOverweightExport => {
					log::debug!(target: LOG, "Init cleanup");

					MigrationStatus::<T>::put(MigrationState::StartedCleanup { cursor: None });
					Self::deposit_event(Event::StartedCleanup);
				},
				MigrationState::StartedCleanup { cursor } => {
					log::debug!(target: LOG, "Cleaning up");
					let hashed_prefix =
						twox_128(<Pezpallet<T> as PalletInfoAccess>::name().as_bytes());

					let result = pezframe_support::storage::unhashed::clear_prefix(
						&hashed_prefix,
						Some(2), // Somehow it does nothing when set to 1, so we set it to 2.
						cursor.as_ref().map(|c| c.as_ref()),
					);
					Self::deposit_event(Event::CleanedSome { keys_removed: result.backend });

					// GOTCHA! We deleted *all* pezpallet storage; hence we also our own
					// `MigrationState`. BUT we insert it back:
					if let Some(unbound_cursor) = result.maybe_cursor {
						if let Ok(cursor) = unbound_cursor.try_into() {
							log::debug!(target: LOG, "Next cursor: {:?}", &cursor);
							MigrationStatus::<T>::put(MigrationState::StartedCleanup {
								cursor: Some(cursor),
							});
						} else {
							MigrationStatus::<T>::put(MigrationState::Completed);
							log::error!(target: LOG, "Completed with error: could not bound cursor");
							Self::deposit_event(Event::Completed { error: true });
						}
					} else {
						MigrationStatus::<T>::put(MigrationState::Completed);
						log::debug!(target: LOG, "Completed");
						Self::deposit_event(Event::Completed { error: false });
					}
				},
				MigrationState::Completed => {
					log::debug!(target: LOG, "Idle; you can remove this pezpallet");
				},
			}

			meter.consumed()
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// The worst-case weight of [`Self::on_idle`].
		pub fn on_idle_weight() -> Weight {
			<T as crate::Config>::WeightInfo::on_idle_good_msg()
				.max(<T as crate::Config>::WeightInfo::on_idle_large_msg())
		}
	}
}
