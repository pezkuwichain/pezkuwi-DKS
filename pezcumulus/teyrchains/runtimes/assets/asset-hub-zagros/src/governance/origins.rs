// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Origins the economic franchise dispatches with.
//!
//! These are the economy's matters and only the economy's: what the treasury spends and how
//! much of it, and the hygiene of this chain's own ballot. Everything the state decides --
//! who holds office, who is a citizen, how an election runs -- belongs to the other franchise,
//! on the chain that holds the register, and is deliberately absent here.
//!
//! That absence is the safety property, not an omission. The two franchises count different
//! electorates: this one counts holdings, and the other counts citizens, one each. If an
//! origin appeared in both, a holding could reach a state power and the register would be for
//! sale. `state_and_economic_origins_do_not_overlap` holds the two lists apart.

pub use pezpallet_custom_origins::*;

#[pezframe_support::pezpallet]
pub mod pezpallet_custom_origins {
	use crate::Balance;
	use pezframe_support::pezpallet_prelude::*;
	use testnet_teyrchains_constants::zagros::currency::UNITS;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[derive(
		PartialEq, Eq, Clone, MaxEncodedLen, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug,
	)]
	#[pezpallet::origin]
	pub enum Origin {
		/// The treasury as a whole: rates, budgets, and what the other spenders may reach.
		#[codec(index = 0)]
		Treasurer,
		/// Small tips, decided quickly because the amount cannot do much harm.
		#[codec(index = 1)]
		SmallTipper,
		/// Larger tips.
		#[codec(index = 2)]
		BigTipper,
		/// Ordinary spending.
		#[codec(index = 3)]
		SmallSpender,
		#[codec(index = 4)]
		MediumSpender,
		/// The largest single amounts this chain will move without Root.
		#[codec(index = 5)]
		BigSpender,
		/// Cancel a referendum on this chain, refunding its deposit.
		#[codec(index = 6)]
		ReferendumCanceller,
		/// Kill a referendum on this chain and slash its deposit.
		#[codec(index = 7)]
		ReferendumKiller,
		/// The knobs of HEZ's own economy: what it emits, and how that emission is split.
		///
		/// Those who bear a decision decide it. Dilution falls on holders in proportion to
		/// what they hold, and this is the franchise that counts holdings -- so this is where
		/// the rate belongs. It is deliberately not `Treasurer`: the treasury's share of
		/// emission is the treasury's own budget, and no organ writes its own input.
		#[codec(index = 8)]
		EconomicAdmin,
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

	decl_unit_ensures!(Treasurer, ReferendumCanceller, ReferendumKiller, EconomicAdmin,);

	macro_rules! decl_ensure {
		(
			$vis:vis type $name:ident: EnsureOrigin<Success = $success_type:ty> {
				$( $item:ident = $success:expr, )*
			}
		) => {
			$vis struct $name;
			impl<O: OriginTrait + From<Origin>> EnsureOrigin<O> for $name
			where
				for<'a> &'a O::PalletsOrigin: TryInto<&'a Origin>,
			{
				type Success = $success_type;
				fn try_origin(o: O) -> Result<Self::Success, O> {
					match o.caller().try_into() {
						$(
							Ok(Origin::$item) => return Ok($success),
						)*
						_ => (),
					}

					Err(o)
				}
				#[cfg(feature = "runtime-benchmarks")]
				fn try_successful_origin() -> Result<O, ()> {
					// By convention the more privileged origins go later, so the last one
					// should be the most powerful.
					let _result: Result<O, ()> = Err(());
					$(
						let _result: Result<O, ()> = Ok(O::from(Origin::$item));
					)*
					_result
				}
			}
		}
	}

	decl_ensure! {
		pub type Spender: EnsureOrigin<Success = Balance> {
			SmallTipper = 250 * UNITS,
			BigTipper = 1_000 * UNITS,
			SmallSpender = 10_000 * UNITS,
			MediumSpender = 100_000 * UNITS,
			BigSpender = 1_000_000 * UNITS,
		}
	}
}
