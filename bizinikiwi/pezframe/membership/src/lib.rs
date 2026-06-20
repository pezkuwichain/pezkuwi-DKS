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

//! # Membership Module
//!
//! Allows control of membership of a set of `AccountId`s, useful for managing membership of a
//! collective. A prime member may be set

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use pezframe_support::{
	traits::{ChangeMembers, Contains, ContainsLengthBound, Get, InitializeMembers, SortedMembers},
	BoundedVec,
};
use pezsp_runtime::traits::{StaticLookup, UniqueSaturatedInto};

pub mod migrations;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
mod tests;

pub use pezpallet::*;
pub use weights::WeightInfo;

const LOG_TARGET: &str = "runtime::membership";

type AccountIdLookupOf<T> = <<T as pezframe_system::Config>::Lookup as StaticLookup>::Source;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	/// The in-code storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T, I = ()>(PhantomData<(T, I)>);

	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;

		/// Required origin for adding a member (though can always be Root).
		type AddOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Required origin for removing a member (though can always be Root).
		type RemoveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Required origin for adding and removing a member in a single action.
		type SwapOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Required origin for resetting membership.
		type ResetOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Required origin for setting or resetting the prime member.
		type PrimeOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The receiver of the signal for when the membership has been initialized. This happens
		/// pre-genesis and will usually be the same as `MembershipChanged`. If you need to do
		/// something different on initialization, then you can change this accordingly.
		type MembershipInitialized: InitializeMembers<Self::AccountId>;

		/// The receiver of the signal for when the membership has changed.
		type MembershipChanged: ChangeMembers<Self::AccountId>;

		/// The maximum number of members that this membership can have.
		///
		/// This is used for benchmarking. Re-run the benchmarks if this changes.
		///
		/// This is enforced in the code; the membership size can not exceed this limit.
		type MaxMembers: Get<u32>;

		/// Weight information for extrinsics in this pezpallet.
		type WeightInfo: WeightInfo;
	}

	/// The current membership, stored as an ordered Vec.
	#[pezpallet::storage]
	pub type Members<T: Config<I>, I: 'static = ()> =
		StorageValue<_, BoundedVec<T::AccountId, T::MaxMembers>, ValueQuery>;

	/// The current prime member, if one exists.
	#[pezpallet::storage]
	pub type Prime<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub members: BoundedVec<T::AccountId, T::MaxMembers>,
		#[serde(skip)]
		pub phantom: PhantomData<I>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config<I>, I: 'static> BuildGenesisConfig for GenesisConfig<T, I> {
		fn build(&self) {
			use alloc::collections::btree_set::BTreeSet;
			let members_set: BTreeSet<_> = self.members.iter().collect();
			assert_eq!(
				members_set.len(),
				self.members.len(),
				"Members cannot contain duplicate accounts."
			);

			let mut members = self.members.clone();
			members.sort();
			T::MembershipInitialized::initialize_members(&members);
			Members::<T, I>::put(members);
		}
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// The given member was added; see the transaction for who.
		MemberAdded,
		/// The given member was removed; see the transaction for who.
		MemberRemoved,
		/// Two members were swapped; see the transaction for who.
		MembersSwapped,
		/// The membership was reset; see the transaction for who the new set is.
		MembersReset,
		/// One of the members' keys changed.
		KeyChanged,
		/// Phantom member, never used.
		Dummy { _phantom_data: PhantomData<(T::AccountId, <T as Config<I>>::RuntimeEvent)> },
	}

	#[pezpallet::error]
	pub enum Error<T, I = ()> {
		/// Already a member.
		AlreadyMember,
		/// Not a member.
		NotMember,
		/// Too many members.
		TooManyMembers,
	}

	#[pezpallet::call]
	impl<T: Config<I>, I: 'static> Pezpallet<T, I> {
		/// Add a member `who` to the set.
		///
		/// May only be called from `T::AddOrigin`.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::add_member(T::MaxMembers::get()))]
		pub fn add_member(
			origin: OriginFor<T>,
			who: AccountIdLookupOf<T>,
		) -> DispatchResultWithPostInfo {
			T::AddOrigin::ensure_origin(origin)?;
			let who = T::Lookup::lookup(who)?;

			let mut members = Members::<T, I>::get();
			let init_length = members.len();
			let location = members.binary_search(&who).err().ok_or(Error::<T, I>::AlreadyMember)?;
			members
				.try_insert(location, who.clone())
				.map_err(|_| Error::<T, I>::TooManyMembers)?;

			Members::<T, I>::put(&members);

			T::MembershipChanged::change_members_sorted(&[who], &[], &members[..]);

			Self::deposit_event(Event::MemberAdded);

			Ok(Some(T::WeightInfo::add_member(init_length as u32)).into())
		}

		/// Remove a member `who` from the set.
		///
		/// May only be called from `T::RemoveOrigin`.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::remove_member(T::MaxMembers::get()))]
		pub fn remove_member(
			origin: OriginFor<T>,
			who: AccountIdLookupOf<T>,
		) -> DispatchResultWithPostInfo {
			T::RemoveOrigin::ensure_origin(origin)?;
			let who = T::Lookup::lookup(who)?;

			let mut members = Members::<T, I>::get();
			let init_length = members.len();
			let location = members.binary_search(&who).ok().ok_or(Error::<T, I>::NotMember)?;
			members.remove(location);

			Members::<T, I>::put(&members);

			T::MembershipChanged::change_members_sorted(&[], &[who], &members[..]);
			Self::rejig_prime(&members);

			Self::deposit_event(Event::MemberRemoved);
			Ok(Some(T::WeightInfo::remove_member(init_length as u32)).into())
		}

		/// Swap out one member `remove` for another `add`.
		///
		/// May only be called from `T::SwapOrigin`.
		///
		/// Prime membership is *not* passed from `remove` to `add`, if extant.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::swap_member(T::MaxMembers::get()))]
		pub fn swap_member(
			origin: OriginFor<T>,
			remove: AccountIdLookupOf<T>,
			add: AccountIdLookupOf<T>,
		) -> DispatchResultWithPostInfo {
			T::SwapOrigin::ensure_origin(origin)?;
			let remove = T::Lookup::lookup(remove)?;
			let add = T::Lookup::lookup(add)?;

			if remove == add {
				return Ok(().into());
			}

			let mut members = Members::<T, I>::get();
			let location = members.binary_search(&remove).ok().ok_or(Error::<T, I>::NotMember)?;
			members.binary_search(&add).err().ok_or(Error::<T, I>::AlreadyMember)?;
			members[location] = add.clone();
			members.sort();

			Members::<T, I>::put(&members);

			T::MembershipChanged::change_members_sorted(&[add], &[remove], &members[..]);
			Self::rejig_prime(&members);

			Self::deposit_event(Event::MembersSwapped);
			Ok(Some(T::WeightInfo::swap_member(members.len() as u32)).into())
		}

		/// Change the membership to a new set, disregarding the existing membership. Be nice and
		/// pass `members` pre-sorted.
		///
		/// May only be called from `T::ResetOrigin`.
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::reset_members(members.len().unique_saturated_into()))]
		pub fn reset_members(origin: OriginFor<T>, members: Vec<T::AccountId>) -> DispatchResult {
			T::ResetOrigin::ensure_origin(origin)?;

			let mut members: BoundedVec<T::AccountId, T::MaxMembers> =
				BoundedVec::try_from(members).map_err(|_| Error::<T, I>::TooManyMembers)?;
			members.sort();
			Members::<T, I>::mutate(|m| {
				T::MembershipChanged::set_members_sorted(&members[..], m);
				Self::rejig_prime(&members);
				*m = members;
			});

			Self::deposit_event(Event::MembersReset);
			Ok(())
		}

		/// Swap out the sending member for some other key `new`.
		///
		/// May only be called from `Signed` origin of a current member.
		///
		/// Prime membership is passed from the origin account to `new`, if extant.
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::WeightInfo::change_key(T::MaxMembers::get()))]
		pub fn change_key(
			origin: OriginFor<T>,
			new: AccountIdLookupOf<T>,
		) -> DispatchResultWithPostInfo {
			let remove = ensure_signed(origin)?;
			let new = T::Lookup::lookup(new)?;

			if remove == new {
				return Ok(().into());
			}

			let mut members = Members::<T, I>::get();
			let members_length = members.len() as u32;
			let location = members.binary_search(&remove).ok().ok_or(Error::<T, I>::NotMember)?;
			members.binary_search(&new).err().ok_or(Error::<T, I>::AlreadyMember)?;
			members[location] = new.clone();
			members.sort();

			Members::<T, I>::put(&members);

			T::MembershipChanged::change_members_sorted(
				&[new.clone()],
				&[remove.clone()],
				&members[..],
			);

			if Prime::<T, I>::get() == Some(remove) {
				Prime::<T, I>::put(&new);
				T::MembershipChanged::set_prime(Some(new));
			}

			Self::deposit_event(Event::KeyChanged);
			Ok(Some(T::WeightInfo::change_key(members_length)).into())
		}

		/// Set the prime member. Must be a current member.
		///
		/// May only be called from `T::PrimeOrigin`.
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(T::WeightInfo::set_prime(T::MaxMembers::get()))]
		pub fn set_prime(
			origin: OriginFor<T>,
			who: AccountIdLookupOf<T>,
		) -> DispatchResultWithPostInfo {
			T::PrimeOrigin::ensure_origin(origin)?;
			let who = T::Lookup::lookup(who)?;
			let members = Members::<T, I>::get();
			members.binary_search(&who).ok().ok_or(Error::<T, I>::NotMember)?;
			Prime::<T, I>::put(&who);
			T::MembershipChanged::set_prime(Some(who));
			Ok(Some(T::WeightInfo::set_prime(members.len() as u32)).into())
		}

		/// Remove the prime member if it exists.
		///
		/// May only be called from `T::PrimeOrigin`.
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::WeightInfo::clear_prime())]
		pub fn clear_prime(origin: OriginFor<T>) -> DispatchResult {
			T::PrimeOrigin::ensure_origin(origin)?;
			Prime::<T, I>::kill();
			T::MembershipChanged::set_prime(None);
			Ok(())
		}
	}
}

impl<T: Config<I>, I: 'static> Pezpallet<T, I> {
	/// The current membership, stored as an ordered `Vec`.
	pub fn members() -> BoundedVec<T::AccountId, T::MaxMembers> {
		Members::<T, I>::get()
	}

	/// The current prime member, if one exists.
	pub fn prime() -> Option<T::AccountId> {
		Prime::<T, I>::get()
	}

	fn rejig_prime(members: &[T::AccountId]) {
		if let Some(prime) = Prime::<T, I>::get() {
			match members.binary_search(&prime) {
				Ok(_) => T::MembershipChanged::set_prime(Some(prime)),
				Err(_) => Prime::<T, I>::kill(),
			}
		}
	}
}

impl<T: Config<I>, I: 'static> Contains<T::AccountId> for Pezpallet<T, I> {
	fn contains(t: &T::AccountId) -> bool {
		Members::<T, I>::get().binary_search(t).is_ok()
	}
}

impl<T: Config> ContainsLengthBound for Pezpallet<T> {
	fn min_len() -> usize {
		0
	}

	/// Implementation uses a parameter type so calling is cost-free.
	fn max_len() -> usize {
		T::MaxMembers::get() as usize
	}
}

impl<T: Config<I>, I: 'static> SortedMembers<T::AccountId> for Pezpallet<T, I> {
	fn sorted_members() -> Vec<T::AccountId> {
		Members::<T, I>::get().to_vec()
	}

	fn count() -> usize {
		Members::<T, I>::decode_len().unwrap_or(0)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(new_member: &T::AccountId) {
		use pezframe_support::{assert_ok, traits::EnsureOrigin};
		let new_member_lookup = T::Lookup::unlookup(new_member.clone());

		if let Ok(origin) = T::AddOrigin::try_successful_origin() {
			assert_ok!(Pezpallet::<T, I>::add_member(origin, new_member_lookup,));
		} else {
			log::error!(target: LOG_TARGET, "Failed to add `{new_member:?}` in `SortedMembers::add`.")
		}
	}
}
