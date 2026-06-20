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

//! A simple wrapper allowing `Sudo` to call into `paras` routines.

use alloc::boxed::Box;
use codec::Encode;
use pezframe_support::pezpallet_prelude::*;
use pezframe_system::pezpallet_prelude::*;
use pezkuwi_primitives::Id as ParaId;
use pezkuwi_runtime_teyrchains::{
	configuration, dmp, hrmp,
	paras::{self, AssignCoretime, ParaGenesisArgs, ParaKind},
	ParaLifecycle,
};
pub use pezpallet::*;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	#[pezpallet::disable_pezframe_system_supertrait_check]
	pub trait Config: configuration::Config + paras::Config + dmp::Config + hrmp::Config {}

	#[pezpallet::error]
	pub enum Error<T> {
		/// The specified teyrchain is not registered.
		ParaDoesntExist,
		/// The specified teyrchain is already registered.
		ParaAlreadyExists,
		/// A DMP message couldn't be sent because it exceeds the maximum size allowed for a
		/// downward message.
		ExceedsMaxMessageSize,
		/// A DMP message couldn't be sent because the destination is unreachable.
		Unroutable,
		/// Could not schedule para cleanup.
		CouldntCleanup,
		/// Not a parathread (on-demand teyrchain).
		NotParathread,
		/// Not a lease holding teyrchain.
		NotTeyrchain,
		/// Cannot upgrade on-demand teyrchain to lease holding teyrchain.
		CannotUpgrade,
		/// Cannot downgrade lease holding teyrchain to on-demand.
		CannotDowngrade,
		/// There are more cores than supported by the runtime.
		TooManyCores,
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Schedule a para to be initialized at the start of the next session.
		///
		/// This should only be used for TESTING and not on PRODUCTION chains. It automatically
		/// assigns Coretime to the chain and increases the number of cores. Thus, there is no
		/// running coretime chain required.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_schedule_para_initialize(
			origin: OriginFor<T>,
			id: ParaId,
			genesis: ParaGenesisArgs,
		) -> DispatchResult {
			ensure_root(origin)?;

			let assign_coretime = genesis.para_kind == ParaKind::Teyrchain;

			pezkuwi_runtime_teyrchains::schedule_para_initialize::<T>(id, genesis)
				.map_err(|_| Error::<T>::ParaAlreadyExists)?;

			if assign_coretime {
				T::AssignCoretime::assign_coretime(id)?;
			}

			Ok(())
		}

		/// Schedule a para to be cleaned up at the start of the next session.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_schedule_para_cleanup(origin: OriginFor<T>, id: ParaId) -> DispatchResult {
			ensure_root(origin)?;
			pezkuwi_runtime_teyrchains::schedule_para_cleanup::<T>(id)
				.map_err(|_| Error::<T>::CouldntCleanup)?;
			Ok(())
		}

		/// Upgrade a parathread (on-demand teyrchain) to a lease holding teyrchain
		#[pezpallet::call_index(2)]
		#[pezpallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_schedule_parathread_upgrade(
			origin: OriginFor<T>,
			id: ParaId,
		) -> DispatchResult {
			ensure_root(origin)?;
			// Para backend should think this is a parathread (on-demand teyrchain)...
			ensure!(
				paras::Pezpallet::<T>::lifecycle(id) == Some(ParaLifecycle::Parathread),
				Error::<T>::NotParathread,
			);
			pezkuwi_runtime_teyrchains::schedule_parathread_upgrade::<T>(id)
				.map_err(|_| Error::<T>::CannotUpgrade)?;
			Ok(())
		}

		/// Downgrade a lease holding teyrchain to an on-demand teyrchain
		#[pezpallet::call_index(3)]
		#[pezpallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_schedule_teyrchain_downgrade(
			origin: OriginFor<T>,
			id: ParaId,
		) -> DispatchResult {
			ensure_root(origin)?;
			// Para backend should think this is a teyrchain...
			ensure!(
				paras::Pezpallet::<T>::lifecycle(id) == Some(ParaLifecycle::Teyrchain),
				Error::<T>::NotTeyrchain,
			);
			pezkuwi_runtime_teyrchains::schedule_teyrchain_downgrade::<T>(id)
				.map_err(|_| Error::<T>::CannotDowngrade)?;
			Ok(())
		}

		/// Send a downward XCM to the given para.
		///
		/// The given teyrchain should exist and the payload should not exceed the preconfigured
		/// size `config.max_downward_message_size`.
		#[pezpallet::call_index(4)]
		#[pezpallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_queue_downward_xcm(
			origin: OriginFor<T>,
			id: ParaId,
			xcm: Box<xcm::opaque::VersionedXcm>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(paras::Pezpallet::<T>::is_valid_para(id), Error::<T>::ParaDoesntExist);
			let config = configuration::ActiveConfig::<T>::get();
			dmp::Pezpallet::<T>::queue_downward_message(&config, id, xcm.encode()).map_err(|e| {
				match e {
					dmp::QueueDownwardMessageError::ExceedsMaxMessageSize => {
						Error::<T>::ExceedsMaxMessageSize.into()
					},
					dmp::QueueDownwardMessageError::Unroutable => Error::<T>::Unroutable.into(),
				}
			})
		}

		/// Forcefully establish a channel from the sender to the recipient.
		///
		/// This is equivalent to sending an `Hrmp::hrmp_init_open_channel` extrinsic followed by
		/// `Hrmp::hrmp_accept_open_channel`.
		#[pezpallet::call_index(5)]
		#[pezpallet::weight((1_000, DispatchClass::Operational))]
		pub fn sudo_establish_hrmp_channel(
			origin: OriginFor<T>,
			sender: ParaId,
			recipient: ParaId,
			max_capacity: u32,
			max_message_size: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			hrmp::Pezpallet::<T>::init_open_channel(
				sender,
				recipient,
				max_capacity,
				max_message_size,
			)?;
			hrmp::Pezpallet::<T>::accept_open_channel(recipient, sender)?;
			Ok(())
		}
	}
}
