// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Custom Origin verification mechanisms based on Tiki ownership.
//!
//! `EnsureTiki` admits a signed caller who currently holds a given Tiki, read through the
//! expiry-aware readers. A role is named by a marker type implementing `GetTiki`; runtimes
//! declare their own markers for the offices they bind.
//!
//! Only single-holder offices belong behind it. For an office many people hold at once -- a
//! member of parliament, a minister -- `EnsureTiki` admits any one of them acting alone, which
//! is not the body deciding. Markers for `Parlementer` and `Wezir` stood here once, with
//! aliases that invited exactly that reading, and no runtime used them; they are removed so the
//! next binding has to be written, and reasoned about, deliberately.

use crate::{Config, Pezpallet as TikiPallet};
use pezframe_support::traits::EnsureOrigin;
use pezframe_system::ensure_signed;
use pezsp_std::marker::PhantomData;

// --- Marker Trait for Tiki Roles ---

/// A trait to return a specific `Tiki` enum variant.
///
/// This trait is implemented by marker structs to identify which
/// Tiki role is required for origin verification.
pub trait GetTiki {
	/// Returns the specific Tiki variant this marker represents.
	fn tiki() -> crate::Tiki;
}

// --- Marker Structs for Each Role ---

/// Marker for the `SerokWeziran` (head of government) office, which one person holds at a time.
///
/// ```ignore
/// type HeadOfGovernment = EnsureTiki<Runtime, SerokWeziranRole>;
/// ```
pub struct SerokWeziranRole;

impl GetTiki for SerokWeziranRole {
	fn tiki() -> crate::Tiki {
		crate::Tiki::SerokWeziran
	}
}

// --- EnsureOrigin Implementation ---

/// An `EnsureOrigin` implementation that requires ownership of a specific Tiki.
///
/// This struct verifies that the origin is a signed account that currently
/// holds the Tiki role specified by the `I: GetTiki` type parameter.
///
/// # Type Parameters
///
/// * `T` - The runtime configuration type implementing `Config`
/// * `I` - A marker type implementing `GetTiki` to specify which Tiki role is required
///
/// # Example
///
/// ```ignore
/// // Require the caller to hold the head-of-government Tiki
/// type HeadOfGovernment = EnsureTiki<Runtime, SerokWeziranRole>;
///
/// // Use in a pezpallet's dispatchable
/// #[pezpallet::call]
/// impl<T: Config> Pezpallet<T> {
///     pub fn privileged_action(origin: OriginFor<T>) -> DispatchResult {
///         let who = T::HeadOfGovernment::ensure_origin(origin)?;
///         // ... action requiring that office
///     }
/// }
/// ```
pub struct EnsureTiki<T, I>(PhantomData<(T, I)>);

impl<T, I> EnsureOrigin<T::RuntimeOrigin> for EnsureTiki<T, I>
where
	T: Config,
	I: GetTiki,
{
	type Success = T::AccountId;

	fn try_origin(o: T::RuntimeOrigin) -> Result<Self::Success, T::RuntimeOrigin> {
		// First, verify the origin is a signed account
		let who = match ensure_signed(o.clone()) {
			Ok(account) => account,
			Err(_) => return Err(o),
		};

		// Get the required Tiki role from the marker type
		let required_tiki = I::tiki();

		// Both paths go through the expiry-aware readers. Asking `TikiHolder` directly would
		// let an officeholder whose term ran out keep authorising things for as long as
		// nobody removed them -- which is the one failure the term exists to prevent, and it
		// would fail open, in the direction of more authority rather than less.
		if TikiPallet::<T>::is_unique_role(&required_tiki) {
			match TikiPallet::<T>::current_holder(&required_tiki) {
				Some(holder) if holder == who => Ok(who),
				_ => Err(o),
			}
		} else if TikiPallet::<T>::has_tiki(&who, &required_tiki) {
			Ok(who)
		} else {
			Err(o)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<T::RuntimeOrigin, ()> {
		use codec::Decode;
		use pezsp_runtime::traits::TrailingZeroInput;

		// Generate a deterministic zero-filled account for benchmarking
		let zero_account = T::AccountId::decode(&mut TrailingZeroInput::zeroes())
			.expect("infinite length input; no invalid inputs for type; qed");

		Ok(T::RuntimeOrigin::from(pezframe_system::RawOrigin::Signed(zero_account)))
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl<T, I> pezframe_support::traits::EnsureOriginWithArg<T::RuntimeOrigin, ()> for EnsureTiki<T, I>
where
	T: Config,
	I: GetTiki,
{
	type Success = T::AccountId;

	fn try_origin(o: T::RuntimeOrigin, _: &()) -> Result<Self::Success, T::RuntimeOrigin> {
		<Self as EnsureOrigin<T::RuntimeOrigin>>::try_origin(o)
	}

	fn try_successful_origin(_: &()) -> Result<T::RuntimeOrigin, ()> {
		use codec::Decode;
		use pezsp_runtime::traits::TrailingZeroInput;

		// Generate a deterministic zero-filled account for benchmarking
		let zero_account = T::AccountId::decode(&mut TrailingZeroInput::zeroes())
			.expect("infinite length input; no invalid inputs for type; qed");

		Ok(T::RuntimeOrigin::from(pezframe_system::RawOrigin::Signed(zero_account)))
	}
}

// Convenience type aliases
pub type EnsureSerokWeziran<T> = EnsureTiki<T, SerokWeziranRole>;
