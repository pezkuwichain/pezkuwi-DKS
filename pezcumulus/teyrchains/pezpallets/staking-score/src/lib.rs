#![cfg_attr(not(feature = "std"), no_std)]

//! # Staking Score Pezpallet
//!
//! Calculates time-weighted staking scores from cached staking data received via XCM.
//!
//! ## Overview
//!
//! People Chain does not have direct access to staking data. Staking details are
//! submitted by noter-authorized accounts (or root via XCM Transact) into
//! `CachedStakingDetails`. This pallet aggregates stake from all sources and
//! calculates a score based on amount and duration.
//!
//! ## Noter Delegation
//!
//! The sudo account delegates `receive_staking_details` authority to accounts that
//! hold the `Noter` tiki (role NFT). A bot collects staking data from Relay Chain
//! and Asset Hub, then a noter signs and submits the data to People Chain.
//!
//! ## Dual-Chain Staking
//!
//! Users can stake on both Relay Chain (direct staking) and Asset Hub (nomination pools).
//! `CachedStakingDetails` is a `StorageDoubleMap` keyed by `(AccountId, StakingSource)`
//! to track stake per source. Score calculation aggregates across all sources.
//!
//! ## Workflow
//!
//! 1. User calls `start_score_tracking()` to opt-in to time-based scoring
//! 2. Bot detects the event, collects staking data from Relay Chain / Asset Hub
//! 3. Noter submits `receive_staking_details()` with the staking data
//! 4. `pezpallet-trust` queries staking score via `StakingScoreProvider` trait
//! 5. Score = base_score(amount_tier) * duration_multiplier, capped at 100

pub use pezpallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::weights::WeightInfo;
	use core::ops::Div;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::traits::{Saturating, Zero};

	// --- Constants ---
	pub const MONTH_IN_BLOCKS: u32 = 30 * 24 * 60 * 10;
	pub const UNITS: u128 = 1_000_000_000_000;

	/// The chain from which staking data originates.
	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Copy,
		PartialEq,
		Eq,
		TypeInfo,
		Debug,
		MaxEncodedLen,
	)]
	pub enum StakingSource {
		/// Direct staking on the Relay Chain.
		RelayChain = 0,
		/// Staking via nomination pools on Asset Hub.
		AssetHub = 1,
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	/// Trait for checking if an account has noter authority.
	/// Noter-authorized accounts can submit staking details on behalf of users.
	pub trait NoterCheck<AccountId> {
		fn is_noter(who: &AccountId) -> bool;
	}

	/// Default implementation: nobody is noter (safe default for tests).
	impl<AccountId> NoterCheck<AccountId> for () {
		fn is_noter(_who: &AccountId) -> bool {
			false
		}
	}

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>>
	where
		BlockNumberFor<Self>: From<u32>,
	{
		/// Balance type used for staking amounts.
		type Balance: Member
			+ Parameter
			+ MaxEncodedLen
			+ Copy
			+ Default
			+ PartialOrd
			+ Saturating
			+ Zero
			+ Div<Output = Self::Balance>
			+ From<u128>;

		/// Callback when staking data changes for an account.
		/// Trust pallet implements this to trigger score recalculation.
		type OnStakingUpdate: OnStakingDataUpdate<Self::AccountId>;

		/// Weight information for extrinsics.
		type WeightInfo: WeightInfo;

		/// Checker for noter authority. Accounts with the Noter tiki can submit
		/// staking details without requiring root origin.
		type NoterChecker: NoterCheck<Self::AccountId>;
	}

	// --- Storage ---

	#[pezpallet::storage]
	#[pezpallet::getter(fn staking_start_block)]
	pub type StakingStartBlock<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

	/// Cached staking details received via XCM from various chains.
	/// Keyed by (AccountId, StakingSource) to support stake aggregation across chains.
	#[pezpallet::storage]
	#[pezpallet::getter(fn cached_staking_details)]
	pub type CachedStakingDetails<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		StakingSource,
		StakingDetails<T::Balance>,
		OptionQuery,
	>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A user started time-based scoring.
		ScoreTrackingStarted { who: T::AccountId, start_block: BlockNumberFor<T> },
		/// Staking details received from a chain via XCM.
		StakingDetailsReceived {
			who: T::AccountId,
			source: StakingSource,
			staked_amount: T::Balance,
		},
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// User must have stake to start score tracking.
		NoStakeFound,
		/// Score tracking has already been started for this account.
		TrackingAlreadyStarted,
		/// Caller does not have noter authority.
		NotAuthorized,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Start time-based score accumulation. One-time opt-in call per user.
		///
		/// The user does not need to have cached staking data yet. A bot will
		/// detect the `ScoreTrackingStarted` event and a noter will submit the
		/// staking data via `receive_staking_details`.
		///
		/// Duration tracking begins at the block this is called, regardless of
		/// when the staking data arrives.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::start_score_tracking())]
		pub fn start_score_tracking(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				StakingStartBlock::<T>::get(&who).is_none(),
				Error::<T>::TrackingAlreadyStarted
			);

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			StakingStartBlock::<T>::insert(&who, current_block);

			// Notify trust pallet. Score may be 0 if CachedStakingDetails is empty.
			T::OnStakingUpdate::on_staking_data_changed(&who);

			Self::deposit_event(Event::ScoreTrackingStarted { who, start_block: current_block });
			Ok(())
		}

		/// Receive staking details for an account.
		///
		/// Accepts root origin (XCM Transact) or a signed origin from an account
		/// that holds the Noter tiki. This allows a noter-authorized bot to submit
		/// staking data collected from Relay Chain and Asset Hub.
		///
		/// If `staked_amount` is zero, the cached entry for the given source is
		/// removed. If no stake remains from any source, `StakingStartBlock` is
		/// also cleaned up, effectively resetting the user's staking score to zero.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::receive_staking_details())]
		pub fn receive_staking_details(
			origin: OriginFor<T>,
			who: T::AccountId,
			source: StakingSource,
			staked_amount: T::Balance,
			nominations_count: u32,
			unlocking_chunks_count: u32,
		) -> DispatchResult {
			// Root (XCM Transact) OR noter-authorized signed origin.
			if ensure_root(origin.clone()).is_err() {
				let caller = ensure_signed(origin)?;
				ensure!(T::NoterChecker::is_noter(&caller), Error::<T>::NotAuthorized);
			}

			if staked_amount.is_zero() {
				// Zero stake: remove the cached entry for this source.
				CachedStakingDetails::<T>::remove(&who, source);

				// Check if any stake remains from other sources.
				let remaining = Self::total_cached_stake(&who);
				if remaining.is_zero() {
					// No stake from any source — clean up tracking.
					StakingStartBlock::<T>::remove(&who);
				}
			} else {
				let details =
					StakingDetails { staked_amount, nominations_count, unlocking_chunks_count };
				CachedStakingDetails::<T>::insert(&who, source, details);
			}

			T::OnStakingUpdate::on_staking_data_changed(&who);

			Self::deposit_event(Event::StakingDetailsReceived { who, source, staked_amount });
			Ok(())
		}
	}

	// --- Types ---

	/// Raw score type used in staking score calculations.
	pub type RawScore = u32;

	/// Staking details for a single source chain.
	#[derive(
		Default,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		Debug,
		MaxEncodedLen,
	)]
	pub struct StakingDetails<Balance> {
		pub staked_amount: Balance,
		pub nominations_count: u32,
		pub unlocking_chunks_count: u32,
	}

	// --- Traits ---

	/// Interface for querying staking scores. Used by trust pallet.
	pub trait StakingScoreProvider<AccountId, BlockNumber> {
		/// Returns (score, duration_in_blocks) for the given account.
		fn get_staking_score(who: &AccountId) -> (RawScore, BlockNumber);
	}

	/// Callback trait for when staking data changes.
	/// Trust pallet implements this to recalculate scores on staking updates.
	pub trait OnStakingDataUpdate<AccountId> {
		fn on_staking_data_changed(who: &AccountId);
	}

	impl<AccountId> OnStakingDataUpdate<AccountId> for () {
		fn on_staking_data_changed(_who: &AccountId) {}
	}

	// --- Helpers ---

	impl<T: Config> Pezpallet<T> {
		/// Calculate total cached stake across all sources for a given account.
		pub fn total_cached_stake(who: &T::AccountId) -> T::Balance {
			let mut total = T::Balance::zero();
			for (_, details) in CachedStakingDetails::<T>::iter_prefix(who) {
				total = total.saturating_add(details.staked_amount);
			}
			total
		}
	}

	// --- StakingScoreProvider Implementation ---

	impl<T: Config> StakingScoreProvider<T::AccountId, BlockNumberFor<T>> for Pezpallet<T> {
		fn get_staking_score(who: &T::AccountId) -> (RawScore, BlockNumberFor<T>) {
			// Aggregate stake from all cached sources.
			let total_staked = Self::total_cached_stake(who);
			let staked_hez: T::Balance = total_staked / UNITS.into();

			if staked_hez.is_zero() {
				return (0, Zero::zero());
			}

			// Amount-based tier scoring.
			let amount_score: u32 = if staked_hez <= 100u128.into() {
				20
			} else if staked_hez <= 250u128.into() {
				30
			} else if staked_hez <= 750u128.into() {
				40
			} else {
				50 // 751+ HEZ
			};

			// Duration-based multiplier.
			let (final_score, duration_for_return) = match StakingStartBlock::<T>::get(who) {
				Some(start_block) => {
					let current_block = pezframe_system::Pezpallet::<T>::block_number();
					let duration_in_blocks = current_block.saturating_sub(start_block);

					let score = if duration_in_blocks >= (12 * MONTH_IN_BLOCKS).into() {
						amount_score.saturating_mul(2) // x2.0 (12+ months)
					} else if duration_in_blocks >= (6 * MONTH_IN_BLOCKS).into() {
						amount_score.saturating_mul(17) / 10 // x1.7 (6-11 months)
					} else if duration_in_blocks >= (3 * MONTH_IN_BLOCKS).into() {
						amount_score.saturating_mul(14) / 10 // x1.4 (3-5 months)
					} else if duration_in_blocks >= MONTH_IN_BLOCKS.into() {
						amount_score.saturating_mul(12) / 10 // x1.2 (1-2 months)
					} else {
						amount_score // x1.0 (< 1 month)
					};

					(score, duration_in_blocks)
				},
				None => (amount_score, Zero::zero()),
			};

			(final_score.min(100), duration_for_return)
		}
	}
}
