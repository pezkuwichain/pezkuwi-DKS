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
// along with Pezkuwi. If not, see <http://www.gnu.org/licenses/>.

//! Custom origins for governance interventions.

pub use pezpallet_custom_origins::*;

#[pezframe_support::pezpallet]
pub mod pezpallet_custom_origins {
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[derive(
		PartialEq, Eq, Clone, MaxEncodedLen, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug,
	)]
	#[pezpallet::origin]
	pub enum Origin {
		/// Origin for cancelling slashes.
		#[codec(index = 0)]
		StakingAdmin,
		// Index 2 held `FellowshipAdmin`, and 17 to 29 the Fellowship's own ranks. The Fellowship
		// is retired: it ranked itself and held the whitelist in front of constitutional
		// change. The indices stay empty so a referendum stored under one of them can never
		// decode as a different origin.
		/// Origin for managing the registrar.
		#[codec(index = 3)]
		GeneralAdmin,
		/// Origin for starting auctions.
		#[codec(index = 4)]
		AuctionAdmin,
		/// Origin able to force slot leases.
		#[codec(index = 5)]
		LeaseAdmin,
		/// Origin able to cancel referenda.
		#[codec(index = 6)]
		ReferendumCanceller,
		/// Origin able to kill referenda.
		#[codec(index = 7)]
		ReferendumKiller,
		/// Origin able to dispatch a whitelisted call.
		#[codec(index = 13)]
		WhitelistedCaller,
		// Indices 14, 15 and 16 held `WelatiElection`, `WelatiAdmin` and `CitizenshipAdmin`.
		// They are retired rather than reused: this chain's referenda weigh tokens, and a body
		// that weighs tokens has no business naming who is a person. The register's own
		// head-counted tracks on the People chain carry those three names now, and they are
		// the only ones that do.
	}

	macro_rules! decl_unit_ensures {
		( $name:ident: $success_type:ty = $success:expr ) => {
			pub struct $name;
			impl<O: OriginTrait + From<Origin>> EnsureOrigin<O> for $name
			where
				for <'a> &'a O::PalletsOrigin: TryInto<&'a Origin>,
			{
				type Success = $success_type;
				fn try_origin(o: O) -> Result<Self::Success, O> {
					match o.caller().try_into() {
						Ok(Origin::$name) => return Ok($success),
						_ => (),
					}

					Err(o)
				}
				#[cfg(feature = "runtime-benchmarks")]
				fn try_successful_origin() -> Result<O, ()> {
					Ok(O::from(Origin::$name))
				}
			}
		};
		( $name:ident ) => { decl_unit_ensures! { $name : () = () } };
		( $name:ident: $success_type:ty = $success:expr, $( $rest:tt )* ) => {
			decl_unit_ensures! { $name: $success_type = $success }
			decl_unit_ensures! { $( $rest )* }
		};
		( $name:ident, $( $rest:tt )* ) => {
			decl_unit_ensures! { $name }
			decl_unit_ensures! { $( $rest )* }
		};
		() => {}
	}
	decl_unit_ensures!(
		StakingAdmin,
		GeneralAdmin,
		AuctionAdmin,
		LeaseAdmin,
		ReferendumCanceller,
		ReferendumKiller,
		WhitelistedCaller,
	);
}
