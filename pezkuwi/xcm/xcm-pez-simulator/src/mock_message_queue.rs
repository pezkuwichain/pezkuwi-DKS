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

//! Simple mock message queue.

use codec::{Decode, Encode};

use pezkuwi_primitives::BlockNumber as RelayBlockNumber;
use pezkuwi_teyrchain_primitives::primitives::{
	DmpMessageHandler, Id as ParaId, XcmpMessageFormat, XcmpMessageHandler,
};
use pezsp_runtime::traits::{Get, Hash};

use xcm::{latest::prelude::*, VersionedXcm};

pub use pezpallet::*;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
		type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {}

	#[pezpallet::pezpallet]
	#[pezpallet::without_storage_info]
	pub struct Pezpallet<T>(_);

	#[pezpallet::storage]
	pub type TeyrchainId<T: Config> = StorageValue<_, ParaId, ValueQuery>;

	#[pezpallet::storage]
	/// A queue of received DMP messages
	pub type ReceivedDmp<T: Config> = StorageValue<_, Vec<Xcm<T::RuntimeCall>>, ValueQuery>;

	impl<T: Config> Get<ParaId> for Pezpallet<T> {
		fn get() -> ParaId {
			TeyrchainId::<T>::get()
		}
	}

	pub type MessageId = [u8; 32];

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// XCMP
		/// Some XCM was executed OK.
		Success { message_id: Option<T::Hash> },
		/// Some XCM failed.
		Fail { message_id: Option<T::Hash>, error: XcmError },
		/// Bad XCM version used.
		BadVersion { message_id: Option<T::Hash> },
		/// Bad XCM format used.
		BadFormat { message_id: Option<T::Hash> },

		// DMP
		/// Downward message is invalid XCM.
		InvalidFormat { message_id: MessageId },
		/// Downward message is unsupported version of XCM.
		UnsupportedVersion { message_id: MessageId },
		/// Downward message executed with the given outcome.
		ExecutedDownward { message_id: MessageId, outcome: Outcome },
	}

	impl<T: Config> Pezpallet<T> {
		pub fn set_para_id(para_id: ParaId) {
			TeyrchainId::<T>::put(para_id);
		}

		fn handle_xcmp_message(
			sender: ParaId,
			_sent_at: RelayBlockNumber,
			xcm: VersionedXcm<T::RuntimeCall>,
			max_weight: xcm::latest::Weight,
		) -> Result<xcm::latest::Weight, XcmError> {
			let hash = Encode::using_encoded(&xcm, T::Hashing::hash);
			let mut message_hash = Encode::using_encoded(&xcm, pezsp_io::hashing::blake2_256);
			let (result, event) = match Xcm::<T::RuntimeCall>::try_from(xcm) {
				Ok(xcm) => {
					let location = (Parent, Teyrchain(sender.into()));
					match T::XcmExecutor::prepare_and_execute(
						location,
						xcm,
						&mut message_hash,
						max_weight,
						Weight::zero(),
					) {
						Outcome::Error(InstructionError { error, .. }) => {
							(Err(error), Event::Fail { message_id: Some(hash), error })
						},
						Outcome::Complete { used } => {
							(Ok(used), Event::Success { message_id: Some(hash) })
						},
						// As far as the caller is concerned, this was dispatched without error, so
						// we just report the weight used.
						Outcome::Incomplete {
							used, error: InstructionError { error, .. }, ..
						} => (Ok(used), Event::Fail { message_id: Some(hash), error }),
					}
				},
				Err(()) => (
					Err(XcmError::UnhandledXcmVersion),
					Event::BadVersion { message_id: Some(hash) },
				),
			};
			Self::deposit_event(event);
			result
		}
	}

	impl<T: Config> XcmpMessageHandler for Pezpallet<T> {
		fn handle_xcmp_messages<'a, I: Iterator<Item = (ParaId, RelayBlockNumber, &'a [u8])>>(
			iter: I,
			max_weight: xcm::latest::Weight,
		) -> xcm::latest::Weight {
			for (sender, sent_at, data) in iter {
				let mut data_ref = data;
				let _ = XcmpMessageFormat::decode(&mut data_ref)
					.expect("Simulator encodes with versioned xcm format; qed");

				let mut remaining_fragments = data_ref;
				while !remaining_fragments.is_empty() {
					if let Ok(xcm) =
						VersionedXcm::<T::RuntimeCall>::decode(&mut remaining_fragments)
					{
						let _ = Self::handle_xcmp_message(sender, sent_at, xcm, max_weight);
					} else {
						debug_assert!(false, "Invalid incoming XCMP message data");
					}
				}
			}
			max_weight
		}
	}

	impl<T: Config> DmpMessageHandler for Pezpallet<T> {
		fn handle_dmp_messages(
			iter: impl Iterator<Item = (RelayBlockNumber, Vec<u8>)>,
			limit: Weight,
		) -> Weight {
			for (_sent_at, data) in iter {
				let mut id = pezsp_io::hashing::blake2_256(&data[..]);
				let maybe_versioned = VersionedXcm::<T::RuntimeCall>::decode(&mut &data[..]);
				match maybe_versioned {
					Err(_) => {
						Self::deposit_event(Event::InvalidFormat { message_id: id });
					},
					Ok(versioned) => match Xcm::try_from(versioned) {
						Err(()) => {
							Self::deposit_event(Event::UnsupportedVersion { message_id: id })
						},
						Ok(x) => {
							let outcome = T::XcmExecutor::prepare_and_execute(
								Parent,
								x.clone(),
								&mut id,
								limit,
								Weight::zero(),
							);
							ReceivedDmp::<T>::append(x);
							Self::deposit_event(Event::ExecutedDownward {
								message_id: id,
								outcome,
							});
						},
					},
				}
			}
			limit
		}
	}
}
