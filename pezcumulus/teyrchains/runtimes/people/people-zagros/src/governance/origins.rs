// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Origins the state franchise dispatches with.
//!
//! These are the state's matters and only the state's: who holds office, who is a citizen,
//! how an election runs, and the hygiene of the ballot itself. Everything an economy decides
//! -- what a token costs, what the treasury spends, who is slashed for a bad nomination --
//! belongs to the other franchise, on the chain that holds the money, and is deliberately
//! absent here.
//!
//! That absence is the safety property, not an omission. The two franchises count different
//! electorates: this one counts citizens, one each, and the other counts holdings. If an
//! origin appeared in both, a holding could reach a state power and the register would be
//! for sale. `state_and_economic_origins_do_not_overlap` in the runtime's tests holds the
//! two lists apart.
//!
//! The three Welati origins used to live on the relay, where their own documentation said
//! they act "on People Chain" -- every use was a message across a chain boundary to reach a
//! register sitting here. They are local now.

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
		/// Origin for Welati administrative actions: tiki grants and appointments.
		#[codec(index = 0)]
		WelatiAdmin,
		/// Origin for citizenship management: revocation and restoration.
		#[codec(index = 1)]
		CitizenshipAdmin,
		/// Origin for election management: initiating and finalizing an election.
		#[codec(index = 2)]
		WelatiElection,
		/// Origin able to cancel a referendum, refunding its deposit.
		#[codec(index = 3)]
		ReferendumCanceller,
		/// Origin able to kill a referendum, slashing its deposit.
		#[codec(index = 4)]
		ReferendumKiller,
	}

	macro_rules! decl_unit_ensures {
		( $name:ident: $success_type:ty = $success:expr ) => {
			pub struct $name;
			impl<O: OriginTrait + From<Origin>> EnsureOrigin<O> for $name
			where
				for<'a> &'a O::PalletsOrigin: TryInto<&'a Origin>,
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
		WelatiAdmin,
		CitizenshipAdmin,
		WelatiElection,
		ReferendumCanceller,
		ReferendumKiller,
	);
}
