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

//! A pezpallet for managing validators on Pezkuwichain.

use alloc::vec::Vec;
use pezsp_runtime::traits::Convert;
use pezsp_staking::SessionIndex;

pub use pezpallet::*;

type Session<T> = pezpallet_session::Pezpallet<T>;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::{dispatch::DispatchResult, pezpallet_prelude::*, traits::EnsureOrigin};
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	#[pezpallet::without_storage_info]
	pub struct Pezpallet<T>(_);

	/// Configuration for the teyrchain proposer.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config + pezpallet_session::Config {
		/// The overreaching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;

		/// Privileged origin that can add or remove validators.
		type PrivilegedOrigin: EnsureOrigin<<Self as pezframe_system::Config>::RuntimeOrigin>;

		/// Staking pallet for forwarding session events
		type Staking: pezpallet_session::SessionManager<Self::ValidatorId>;
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New validators were added to the set.
		ValidatorsRegistered(Vec<T::ValidatorId>),
		/// Validators were removed from the set.
		ValidatorsDeregistered(Vec<T::ValidatorId>),
	}

	/// Validators that should be retired, because their Teyrchain was deregistered.
	#[pezpallet::storage]
	pub(crate) type ValidatorsToRetire<T: Config> =
		StorageValue<_, Vec<T::ValidatorId>, ValueQuery>;

	/// Validators that should be added.
	#[pezpallet::storage]
	pub(crate) type ValidatorsToAdd<T: Config> = StorageValue<_, Vec<T::ValidatorId>, ValueQuery>;

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Add new validators to the set.
		///
		/// The new validators will be active from current session + 2.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight({100_000})]
		pub fn register_validators(
			origin: OriginFor<T>,
			validators: Vec<T::ValidatorId>,
		) -> DispatchResult {
			T::PrivilegedOrigin::ensure_origin(origin)?;

			validators.clone().into_iter().for_each(|v| ValidatorsToAdd::<T>::append(v));

			Self::deposit_event(Event::ValidatorsRegistered(validators));
			Ok(())
		}

		/// Remove validators from the set.
		///
		/// The removed validators will be deactivated from current session + 2.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight({100_000})]
		pub fn deregister_validators(
			origin: OriginFor<T>,
			validators: Vec<T::ValidatorId>,
		) -> DispatchResult {
			T::PrivilegedOrigin::ensure_origin(origin)?;

			validators.clone().into_iter().for_each(|v| ValidatorsToRetire::<T>::append(v));

			Self::deposit_event(Event::ValidatorsDeregistered(validators));
			Ok(())
		}
	}
}

impl<T: Config> pezpallet_session::SessionManager<T::ValidatorId> for Pezpallet<T> {
	fn new_session(new_index: SessionIndex) -> Option<Vec<T::ValidatorId>> {
		// Forward to Staking pallet for era management
		let _ = T::Staking::new_session(new_index);

		if new_index <= 1 {
			return None;
		}

		let mut validators = Session::<T>::validators();

		ValidatorsToRetire::<T>::take().iter().for_each(|v| {
			if let Some(pos) = validators.iter().position(|r| r == v) {
				validators.swap_remove(pos);
			}
		});

		ValidatorsToAdd::<T>::take().into_iter().for_each(|v| {
			if !validators.contains(&v) {
				validators.push(v);
			}
		});

		Some(validators)
	}

	fn end_session(end_index: SessionIndex) {
		// Forward to Staking pallet
		T::Staking::end_session(end_index);
	}

	fn start_session(start_index: SessionIndex) {
		// Forward to Staking pallet
		T::Staking::start_session(start_index);
	}
}

impl<T: Config + pezpallet_session::historical::Config>
	pezpallet_session::historical::SessionManager<T::ValidatorId, T::FullIdentification>
	for Pezpallet<T>
{
	fn new_session(
		new_index: SessionIndex,
	) -> Option<Vec<(T::ValidatorId, T::FullIdentification)>> {
		<Self as pezpallet_session::SessionManager<_>>::new_session(new_index).map(|r| {
			r.into_iter()
				.filter_map(|v| {
					let full_id = T::FullIdentificationOf::convert(v.clone());
					full_id.map(|id| (v, id))
				})
				.collect()
		})
	}

	fn start_session(start_index: SessionIndex) {
		<Self as pezpallet_session::SessionManager<_>>::start_session(start_index)
	}

	fn end_session(end_index: SessionIndex) {
		<Self as pezpallet_session::SessionManager<_>>::end_session(end_index)
	}
}
