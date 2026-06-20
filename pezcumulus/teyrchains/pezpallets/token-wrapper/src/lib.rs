// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # Token Wrapper Pezpallet
//!
//! A pezpallet for wrapping native tokens (HEZ) into fungible assets (wHEZ)
//! to enable DEX operations between native and asset tokens.
//!
//! ## Overview
//!
//! This pezpallet provides:
//! - `wrap`: Convert native HEZ to wHEZ (Asset ID 0)
//! - `unwrap`: Convert wHEZ back to native HEZ
//!
//! The pezpallet maintains a 1:1 backing between HEZ and wHEZ.

pub use pezpallet::*;
pub use weights::WeightInfo;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use pezframe_support::{
	dispatch::DispatchResult,
	pezpallet_prelude::*,
	traits::{
		fungibles::{Create, Inspect, Mutate},
		Currency, ExistenceRequirement,
	},
	PalletId,
};
use pezframe_system::pezpallet_prelude::*;
use pezsp_runtime::traits::{AccountIdConversion, Saturating, Zero};

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as pezframe_system::Config>::AccountId>>::Balance;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		/// Weight information for extrinsics in this pezpallet.
		type WeightInfo: crate::WeightInfo;

		/// Native currency (HEZ)
		type Currency: Currency<Self::AccountId>;

		/// Asset ID type
		type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize + MaxEncodedLen;

		/// Fungible assets (for wHEZ)
		type Assets: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = BalanceOf<Self>>
			+ Mutate<Self::AccountId>
			+ Create<Self::AccountId>;

		/// Pezpallet ID for the wrapper account
		#[pezpallet::constant]
		type PalletId: Get<PalletId>;

		/// Asset ID for wrapped token (wHEZ)
		#[pezpallet::constant]
		type WrapperAssetId: Get<Self::AssetId>;
	}

	// ============================================================================
	// STORAGE ITEMS
	// ============================================================================

	/// Total amount of native tokens locked in wrapper
	#[pezpallet::storage]
	#[pezpallet::getter(fn total_locked)]
	pub type TotalLocked<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	// ============================================================================
	// EVENTS
	// ============================================================================

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Native token wrapped into asset token. [who, amount]
		Wrapped { who: T::AccountId, amount: BalanceOf<T> },
		/// Asset token unwrapped back to native. [who, amount]
		Unwrapped { who: T::AccountId, amount: BalanceOf<T> },
	}

	// ============================================================================
	// ERRORS
	// ============================================================================

	#[pezpallet::error]
	pub enum Error<T> {
		/// Insufficient balance for wrapping
		InsufficientBalance,
		/// Insufficient wrapped tokens for unwrapping
		InsufficientWrappedBalance,
		/// Transfer failed
		TransferFailed,
		/// Mint failed
		MintFailed,
		/// Burn failed
		BurnFailed,
		/// Amount is zero
		ZeroAmount,
	}

	// ============================================================================
	// DISPATCHABLE FUNCTIONS
	// ============================================================================

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Wrap native tokens (HEZ) into wrapped asset tokens (wHEZ)
		///
		/// - `amount`: The amount of native tokens to wrap
		///
		/// This will:
		/// 1. Transfer native tokens from user to pezpallet account (lock)
		/// 2. Mint equivalent amount of wrapped tokens to user
		///
		/// Emits `Wrapped` event.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::wrap())]
		pub fn wrap(
			origin: OriginFor<T>,
			#[pezpallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Ensure amount is not zero
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			// Check balance
			ensure!(T::Currency::free_balance(&who) >= amount, Error::<T>::InsufficientBalance);

			// Transfer native tokens to pezpallet account (lock them)
			T::Currency::transfer(
				&who,
				&Self::account_id(),
				amount,
				ExistenceRequirement::KeepAlive,
			)
			.map_err(|_| Error::<T>::TransferFailed)?;

			// Mint wrapped tokens to user BEFORE updating TotalLocked
			// If mint fails, the extrinsic reverts (including the transfer above)
			T::Assets::mint_into(T::WrapperAssetId::get(), &who, amount)
				.map_err(|_| Error::<T>::MintFailed)?;

			// Update total locked only after both transfer and mint succeeded
			TotalLocked::<T>::mutate(|total| {
				*total = total.saturating_add(amount);
			});

			Self::deposit_event(Event::Wrapped { who, amount });
			Ok(())
		}

		/// Unwrap wrapped asset tokens (wHEZ) back to native tokens (HEZ)
		///
		/// - `amount`: The amount of wrapped tokens to unwrap
		///
		/// This will:
		/// 1. Burn wrapped tokens from user
		/// 2. Transfer equivalent native tokens back to user (unlock)
		///
		/// Emits `Unwrapped` event.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::unwrap())]
		pub fn unwrap(
			origin: OriginFor<T>,
			#[pezpallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Ensure amount is not zero
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

			// Check wrapped token balance
			let wrapped_balance = T::Assets::balance(T::WrapperAssetId::get(), &who);
			ensure!(wrapped_balance >= amount, Error::<T>::InsufficientWrappedBalance);

			// Verify pallet has sufficient backing before any state changes
			let pallet_balance = T::Currency::free_balance(&Self::account_id());
			ensure!(pallet_balance >= amount, Error::<T>::TransferFailed);

			// Burn wrapped tokens from user
			T::Assets::burn_from(
				T::WrapperAssetId::get(),
				&who,
				amount,
				pezframe_support::traits::tokens::Preservation::Expendable,
				pezframe_support::traits::tokens::Precision::Exact,
				pezframe_support::traits::tokens::Fortitude::Force,
			)
			.map_err(|_| Error::<T>::BurnFailed)?;

			// Transfer native tokens back to user (unlock)
			// If this fails, the extrinsic reverts (including the burn above)
			T::Currency::transfer(
				&Self::account_id(),
				&who,
				amount,
				ExistenceRequirement::AllowDeath,
			)
			.map_err(|_| Error::<T>::TransferFailed)?;

			// Update total locked only after both burn and transfer succeeded
			TotalLocked::<T>::mutate(|total| {
				*total = total.saturating_sub(amount);
			});

			Self::deposit_event(Event::Unwrapped { who, amount });
			Ok(())
		}
	}

	// ============================================================================
	// HELPER FUNCTIONS
	// ============================================================================

	impl<T: Config> Pezpallet<T> {
		/// Get the account ID of the pezpallet
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		/// Get the total supply of wrapped tokens
		pub fn total_wrapped() -> BalanceOf<T> {
			T::Assets::total_issuance(T::WrapperAssetId::get())
		}
	}
}
