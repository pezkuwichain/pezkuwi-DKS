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

// Benchmarking for the `AssetTransactor` trait via `Fungible`.

pub use pezpallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
mod mock;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use pezframe_support::pezpallet_prelude::Get;
	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config + crate::Config {
		/// The type of `fungible` that is being used under the hood.
		///
		/// This is useful for testing and checking.
		type TransactAsset: pezframe_support::traits::fungible::Mutate<Self::AccountId>;

		/// The account used to check assets being teleported.
		type CheckedAccount: Get<Option<(Self::AccountId, xcm_builder::MintLocation)>>;

		/// A trusted location which we allow teleports from, and the asset we allow to teleport.
		type TrustedTeleporter: Get<Option<(xcm::latest::Location, xcm::latest::Asset)>>;

		/// A trusted location where reserve assets are stored, and the asset we allow to be
		/// reserves.
		type TrustedReserve: Get<Option<(xcm::latest::Location, xcm::latest::Asset)>>;

		/// Give me a fungible asset that your asset transactor is going to accept.
		fn get_asset() -> xcm::latest::Asset;
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(_);
}
