// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Custom Origin verification mechanisms based on Tiki ownership.
//!
//! This module provides `EnsureOrigin` implementations that verify
//! the caller holds a specific Tiki role (Serok, Wezir, or Parlementer).

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

/// Marker struct representing the `Serok` (President/Leader) role.
///
/// Use with `EnsureTiki` to require the caller holds the Serok Tiki:
/// ```ignore
/// type SerokOrigin = EnsureTiki<Runtime, SerokRole>;
/// ```
pub struct SerokRole;

impl GetTiki for SerokRole {
	fn tiki() -> crate::Tiki {
		crate::Tiki::Serok
	}
}

/// Marker struct representing the `Wezir` (Minister/Advisor) role.
///
/// Use with `EnsureTiki` to require the caller holds the Wezir Tiki:
/// ```ignore
/// type WezirOrigin = EnsureTiki<Runtime, WezirRole>;
/// ```
pub struct WezirRole;

impl GetTiki for WezirRole {
	fn tiki() -> crate::Tiki {
		crate::Tiki::Wezir
	}
}

/// Marker struct representing the `Parlementer` (Parliamentarian) role.
///
/// Use with `EnsureTiki` to require the caller holds the Parlementer Tiki:
/// ```ignore
/// type ParlementerOrigin = EnsureTiki<Runtime, ParlementerRole>;
/// ```
pub struct ParlementerRole;

impl GetTiki for ParlementerRole {
	fn tiki() -> crate::Tiki {
		crate::Tiki::Parlementer
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
/// // Require the caller to hold the Serok Tiki
/// type SerokOrigin = EnsureTiki<Runtime, SerokRole>;
///
/// // Use in a pezpallet's dispatchable
/// #[pezpallet::call]
/// impl<T: Config> Pezpallet<T> {
///     pub fn privileged_action(origin: OriginFor<T>) -> DispatchResult {
///         let who = T::SerokOrigin::ensure_origin(origin)?;
///         // ... action requiring Serok authority
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

		// For unique roles, check TikiHolder (fast O(1) lookup)
		if TikiPallet::<T>::is_unique_role(&required_tiki) {
			match TikiPallet::<T>::tiki_holder(required_tiki) {
				Some(holder) if holder == who => Ok(who),
				_ => Err(o),
			}
		} else {
			// For non-unique roles (Wezir, Parlementer, etc.), check UserTikis storage
			if TikiPallet::<T>::user_tikis(&who).contains(&required_tiki) {
				Ok(who)
			} else {
				Err(o)
			}
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
pub type EnsureSerok<T> = EnsureTiki<T, SerokRole>;
pub type EnsureWezir<T> = EnsureTiki<T, WezirRole>;
pub type EnsureParlementer<T> = EnsureTiki<T, ParlementerRole>;
