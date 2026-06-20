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

//! Pezpallet for stuff specific to teyrchains' usage of XCM. Right now that's just the origin
//! used by teyrchains when receiving `Transact` messages from other teyrchains or the Relay chain
//! which must be natively represented.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use pezcumulus_primitives_core::ParaId;
pub use pezpallet::*;
use pezsp_runtime::{traits::BadOrigin, RuntimeDebug};
use scale_info::TypeInfo;
use xcm::latest::{ExecuteXcm, Outcome};

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	/// The module configuration trait.
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;

		type XcmExecutor: ExecuteXcm<Self::RuntimeCall>;
	}

	#[pezpallet::event]
	pub enum Event<T: Config> {
		/// Downward message is invalid XCM.
		/// \[ id \]
		InvalidFormat([u8; 32]),
		/// Downward message is unsupported version of XCM.
		/// \[ id \]
		UnsupportedVersion([u8; 32]),
		/// Downward message executed with the given outcome.
		/// \[ id, outcome \]
		ExecutedDownward([u8; 32], Outcome),
	}

	/// Origin for the teyrchains module.
	#[derive(
		PartialEq,
		Eq,
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		TypeInfo,
		RuntimeDebug,
		MaxEncodedLen,
	)]
	#[pezpallet::origin]
	pub enum Origin {
		/// It comes from the (parent) relay chain.
		Relay,
		/// It comes from a (sibling) teyrchain.
		SiblingTeyrchain(ParaId),
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {}

	impl From<ParaId> for Origin {
		fn from(id: ParaId) -> Origin {
			Origin::SiblingTeyrchain(id)
		}
	}
	impl From<u32> for Origin {
		fn from(id: u32) -> Origin {
			Origin::SiblingTeyrchain(id.into())
		}
	}
}

/// Ensure that the origin `o` represents a sibling teyrchain.
/// Returns `Ok` with the teyrchain ID of the sibling or an `Err` otherwise.
pub fn ensure_sibling_para<OuterOrigin>(o: OuterOrigin) -> Result<ParaId, BadOrigin>
where
	OuterOrigin: Into<Result<Origin, OuterOrigin>>,
{
	match o.into() {
		Ok(Origin::SiblingTeyrchain(id)) => Ok(id),
		_ => Err(BadOrigin),
	}
}

/// Ensure that the origin `o` represents is the relay chain.
/// Returns `Ok` if it does or an `Err` otherwise.
pub fn ensure_relay<OuterOrigin>(o: OuterOrigin) -> Result<(), BadOrigin>
where
	OuterOrigin: Into<Result<Origin, OuterOrigin>>,
{
	match o.into() {
		Ok(Origin::Relay) => Ok(()),
		_ => Err(BadOrigin),
	}
}
