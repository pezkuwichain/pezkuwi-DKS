// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

//! Pezpallet to spam the XCM/UMP.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

use alloc::{vec, vec::Vec};
use pezcumulus_pezpallet_xcm::{ensure_sibling_para, Origin as CumulusOrigin};
use pezcumulus_primitives_core::ParaId;
use pezframe_support::{parameter_types, BoundedVec};
use pezframe_system::Config as SystemConfig;
use pezsp_runtime::traits::Saturating;
use xcm::latest::prelude::*;

pub use pezpallet::*;
pub use weights::WeightInfo;

parameter_types! {
	const MaxTeyrchains: u32 = 100;
	const MaxPayloadSize: u32 = 1024;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	/// The module configuration trait.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;

		type RuntimeOrigin: From<<Self as SystemConfig>::RuntimeOrigin>
			+ Into<Result<CumulusOrigin, <Self as Config>::RuntimeOrigin>>;

		/// The overarching call type; we assume sibling chains use the same type.
		type RuntimeCall: From<Call<Self>> + Encode;

		type XcmSender: SendXcm;

		/// Weight information for extrinsics in this pezpallet.
		type WeightInfo: WeightInfo;
	}

	/// The target teyrchains to ping.
	#[pezpallet::storage]
	pub(super) type Targets<T: Config> = StorageValue<
		_,
		BoundedVec<(ParaId, BoundedVec<u8, MaxPayloadSize>), MaxTeyrchains>,
		ValueQuery,
	>;

	/// The total number of pings sent.
	#[pezpallet::storage]
	pub(super) type PingCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// The sent pings.
	#[pezpallet::storage]
	pub(super) type Pings<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, BlockNumberFor<T>, OptionQuery>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PingSent(ParaId, u32, Vec<u8>, XcmHash, Assets),
		Pinged(ParaId, u32, Vec<u8>),
		PongSent(ParaId, u32, Vec<u8>, XcmHash, Assets),
		Ponged(ParaId, u32, Vec<u8>, BlockNumberFor<T>),
		ErrorSendingPing(SendError, ParaId, u32, Vec<u8>),
		ErrorSendingPong(SendError, ParaId, u32, Vec<u8>),
		UnknownPong(ParaId, u32, Vec<u8>),
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// Too many teyrchains have been added as a target.
		TooManyTargets,
		/// The payload provided is too large, limit is 1024 bytes.
		PayloadTooLarge,
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			for (para, payload) in Targets::<T>::get().into_iter() {
				let seq = PingCount::<T>::mutate(|seq| {
					*seq = seq.saturating_add(1);
					*seq
				});
				match send_xcm::<T::XcmSender>(
					(Parent, Junction::Teyrchain(para.into())).into(),
					Xcm(vec![Transact {
						origin_kind: OriginKind::Native,
						call: <T as Config>::RuntimeCall::from(Call::<T>::ping {
							seq,
							payload: payload.clone().to_vec(),
						})
						.encode()
						.into(),
						fallback_max_weight: None,
					}]),
				) {
					Ok((hash, cost)) => {
						Pings::<T>::insert(seq, n);
						Self::deposit_event(Event::PingSent(
							para,
							seq,
							payload.to_vec(),
							hash,
							cost,
						));
					},
					Err(e) => {
						Self::deposit_event(Event::ErrorSendingPing(
							e,
							para,
							seq,
							payload.to_vec(),
						));
					},
				}
			}
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::start(payload.len() as u32))]
		pub fn start(origin: OriginFor<T>, para: ParaId, payload: Vec<u8>) -> DispatchResult {
			ensure_root(origin)?;
			let payload = BoundedVec::<u8, MaxPayloadSize>::try_from(payload)
				.map_err(|_| Error::<T>::PayloadTooLarge)?;
			Targets::<T>::try_mutate(|t| {
				t.try_push((para, payload)).map_err(|_| Error::<T>::TooManyTargets)
			})?;
			Ok(())
		}

		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::start_many(*count))]
		pub fn start_many(
			origin: OriginFor<T>,
			para: ParaId,
			count: u32,
			payload: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let bounded_payload = BoundedVec::<u8, MaxPayloadSize>::try_from(payload)
				.map_err(|_| Error::<T>::PayloadTooLarge)?;
			for _ in 0..count {
				Targets::<T>::try_mutate(|t| {
					t.try_push((para, bounded_payload.clone()))
						.map_err(|_| Error::<T>::TooManyTargets)
				})?;
			}
			Ok(())
		}

		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::stop())]
		pub fn stop(origin: OriginFor<T>, para: ParaId) -> DispatchResult {
			ensure_root(origin)?;
			Targets::<T>::mutate(|t| {
				if let Some(p) = t.iter().position(|(p, _)| p == &para) {
					t.swap_remove(p);
				}
			});
			Ok(())
		}

		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::stop_all())]
		pub fn stop_all(origin: OriginFor<T>, maybe_para: Option<ParaId>) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(para) = maybe_para {
				Targets::<T>::mutate(|t| t.retain(|&(x, _)| x != para));
			} else {
				Targets::<T>::kill();
			}
			Ok(())
		}

		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::WeightInfo::ping(payload.len() as u32))]
		pub fn ping(origin: OriginFor<T>, seq: u32, payload: Vec<u8>) -> DispatchResult {
			// Only accept pings from other chains.
			let para = ensure_sibling_para(<T as Config>::RuntimeOrigin::from(origin))?;

			Self::deposit_event(Event::Pinged(para, seq, payload.clone()));
			match send_xcm::<T::XcmSender>(
				(Parent, Junction::Teyrchain(para.into())).into(),
				Xcm(vec![Transact {
					origin_kind: OriginKind::Native,
					call: <T as Config>::RuntimeCall::from(Call::<T>::pong {
						seq,
						payload: payload.clone(),
					})
					.encode()
					.into(),
					fallback_max_weight: None,
				}]),
			) {
				Ok((hash, cost)) => {
					Self::deposit_event(Event::PongSent(para, seq, payload, hash, cost))
				},
				Err(e) => Self::deposit_event(Event::ErrorSendingPong(e, para, seq, payload)),
			}
			Ok(())
		}

		#[pezpallet::call_index(5)]
		#[pezpallet::weight(T::WeightInfo::pong(payload.len() as u32))]
		pub fn pong(origin: OriginFor<T>, seq: u32, payload: Vec<u8>) -> DispatchResult {
			// Only accept pings from other chains.
			let para = ensure_sibling_para(<T as Config>::RuntimeOrigin::from(origin))?;

			if let Some(sent_at) = Pings::<T>::take(seq) {
				Self::deposit_event(Event::Ponged(
					para,
					seq,
					payload,
					pezframe_system::Pezpallet::<T>::block_number().saturating_sub(sent_at),
				));
			} else {
				// Pong received for a ping we apparently didn't send?!
				Self::deposit_event(Event::UnknownPong(para, seq, payload));
			}
			Ok(())
		}
	}
}
