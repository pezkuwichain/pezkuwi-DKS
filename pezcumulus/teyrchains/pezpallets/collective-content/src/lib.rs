// Copyright (C) 2023 Parity Technologies (UK) Ltd.
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

//! Managed Collective Content Pezpallet
//!
//! The pezpallet provides the functionality to store different types of content. This would
//! typically be used by an on-chain collective, such as the Pezkuwi Alliance or Ambassador Program.
//!
//! The pezpallet stores content as an [OpaqueCid], which should correspond to some off-chain
//! hosting service, such as IPFS, and contain any type of data. Each type of content has its own
//! origin from which it can be managed. The origins are configurable in the runtime. Storing
//! content does not require a deposit, as it is expected to be managed by a trusted collective.
//!
//! Content types:
//!
//! - Collective [charter](pezpallet::Charter): A single document (`OpaqueCid`) managed by
//!   [CharterOrigin](pezpallet::Config::CharterOrigin).
//! - Collective [announcements](pezpallet::Announcements): A list of announcements managed by
//!   [AnnouncementOrigin](pezpallet::Config::AnnouncementOrigin).

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

pub use pezpallet::*;
pub use weights::WeightInfo;

use pezframe_support::{traits::schedule::DispatchTime, BoundedVec};
use pezsp_core::ConstU32;

/// IPFS compatible CID.
// Worst case 2 bytes base and codec, 2 bytes hash type and size, 64 bytes hash digest.
pub type OpaqueCid = BoundedVec<u8, ConstU32<68>>;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::{ensure, pezpallet_prelude::*};
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::{traits::BadOrigin, Saturating};

	/// The in-code storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T, I = ()>(PhantomData<(T, I)>);

	/// The module configuration trait.
	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;

		/// Default lifetime for an announcement before it expires.
		type AnnouncementLifetime: Get<BlockNumberFor<Self>>;

		/// The origin to control the collective announcements.
		type AnnouncementOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Maximum number of announcements in the storage.
		#[pezpallet::constant]
		type MaxAnnouncements: Get<u32>;

		/// The origin to control the collective charter.
		type CharterOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information needed for the pezpallet.
		type WeightInfo: WeightInfo;
	}

	#[pezpallet::error]
	pub enum Error<T, I = ()> {
		/// The announcement is not found.
		MissingAnnouncement,
		/// Number of announcements exceeds `MaxAnnouncementsCount`.
		TooManyAnnouncements,
		/// Cannot expire in the past.
		InvalidExpiration,
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// A new charter has been set.
		NewCharterSet { cid: OpaqueCid },
		/// A new announcement has been made.
		AnnouncementAnnounced { cid: OpaqueCid, expire_at: BlockNumberFor<T> },
		/// An on-chain announcement has been removed.
		AnnouncementRemoved { cid: OpaqueCid },
	}

	/// The collective charter.
	#[pezpallet::storage]
	pub type Charter<T: Config<I>, I: 'static = ()> = StorageValue<_, OpaqueCid, OptionQuery>;

	/// The collective announcements.
	#[pezpallet::storage]
	pub type Announcements<T: Config<I>, I: 'static = ()> =
		CountedStorageMap<_, Blake2_128Concat, OpaqueCid, BlockNumberFor<T>, OptionQuery>;

	#[pezpallet::call]
	impl<T: Config<I>, I: 'static> Pezpallet<T, I> {
		/// Set the collective charter.
		///
		/// Parameters:
		/// - `origin`: Must be the [Config::CharterOrigin].
		/// - `cid`: [CID](super::OpaqueCid) of the IPFS document of the collective charter.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::set_charter())]
		pub fn set_charter(origin: OriginFor<T>, cid: OpaqueCid) -> DispatchResult {
			T::CharterOrigin::ensure_origin(origin)?;

			Charter::<T, I>::put(&cid);

			Self::deposit_event(Event::<T, I>::NewCharterSet { cid });
			Ok(())
		}

		/// Publish an announcement.
		///
		/// Parameters:
		/// - `origin`: Must be the [Config::AnnouncementOrigin].
		/// - `cid`: [CID](super::OpaqueCid) of the IPFS document to announce.
		/// - `maybe_expire`: Expiration block of the announcement. If `None`
		///   [`Config::AnnouncementLifetime`]
		/// used as a default.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::announce())]
		pub fn announce(
			origin: OriginFor<T>,
			cid: OpaqueCid,
			maybe_expire: Option<DispatchTime<BlockNumberFor<T>>>,
		) -> DispatchResult {
			T::AnnouncementOrigin::ensure_origin(origin)?;

			let now = pezframe_system::Pezpallet::<T>::block_number();
			let expire_at = maybe_expire
				.map_or(now.saturating_add(T::AnnouncementLifetime::get()), |e| e.evaluate(now));
			ensure!(expire_at > now, Error::<T, I>::InvalidExpiration);
			ensure!(
				T::MaxAnnouncements::get() > <Announcements<T, I>>::count(),
				Error::<T, I>::TooManyAnnouncements
			);

			<Announcements<T, I>>::insert(cid.clone(), expire_at);

			Self::deposit_event(Event::<T, I>::AnnouncementAnnounced { cid, expire_at });
			Ok(())
		}

		/// Remove an announcement.
		///
		/// Transaction fee refunded for expired announcements.
		///
		/// Parameters:
		/// - `origin`: Must be the [Config::AnnouncementOrigin] or signed for expired
		///   announcements.
		/// - `cid`: [CID](super::OpaqueCid) of the IPFS document to remove.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::remove_announcement())]
		pub fn remove_announcement(
			origin: OriginFor<T>,
			cid: OpaqueCid,
		) -> DispatchResultWithPostInfo {
			let maybe_who = match T::AnnouncementOrigin::try_origin(origin) {
				Ok(_) => None,
				Err(origin) => Some(ensure_signed(origin)?),
			};
			let expire_at = <Announcements<T, I>>::get(cid.clone())
				.ok_or(Error::<T, I>::MissingAnnouncement)?;
			let now = pezframe_system::Pezpallet::<T>::block_number();
			ensure!(maybe_who.is_none() || now >= expire_at, BadOrigin);

			<Announcements<T, I>>::remove(cid.clone());

			Self::deposit_event(Event::<T, I>::AnnouncementRemoved { cid });

			if now >= expire_at {
				return Ok(Pays::No.into());
			}
			Ok(Pays::Yes.into())
		}
	}
}
