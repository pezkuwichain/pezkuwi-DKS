// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # Trust Score Pezpallet
//!
//! A pezpallet for calculating and managing composite trust scores based on multiple ecosystem
//! metrics.
//!
//! ## Overview
//!
//! The Trust Score pezpallet aggregates multiple reputation and activity metrics to produce
//! a unified trust score for each citizen. This score is used throughout the ecosystem for:
//!
//! - Validator pool eligibility (trust-based validators)
//! - Reward distribution weighting (pez-rewards)
//! - Governance participation rights
//! - Social reputation tracking
//!
//! ## Trust Score Components
//!
//! The trust score is calculated from four primary sources:
//!
//! 1. **Staking Score**: Economic security through token staking
//! 2. **Referral Score**: Network growth contribution via referrals
//! 3. **Perwerde Score**: Educational achievement and verification
//! 4. **Tiki Score**: Social engagement and platform activity
//!
//! ## Score Calculation
//!
//! ```text
//! trust_score = (staking_score + referral_score + perwerde_score + tiki_score) * multiplier
//! ```
//!
//! Where:
//! - Each component score is normalized and weighted
//! - The multiplier is configurable via `ScoreMultiplierBase`
//! - Citizenship status is required (KYC approved)
//!
//! ## Update Mechanisms
//!
//! ### Automatic Updates
//! - Periodic batch updates scheduled at `UpdateInterval` (e.g., daily)
//! - Processes all citizens in batches to manage computational load
//! - Maintains update progress across blocks for large user bases
//!
//! ### Manual Updates
//! - Individual score recalculation via privileged call
//! - Full batch update trigger (root only)
//! - Component change hooks from other pallets
//!
//! ## Storage
//!
//! - `TrustScores` - Per-account trust score mapping
//! - `TotalActiveTrustScore` - Aggregate trust score across all citizens
//! - `BatchUpdateInProgress` - Flag for ongoing batch update process
//! - `LastProcessedAccount` - Checkpoint for resumable batch updates
//!
//! ## Interface
//!
//! ### Extrinsics
//!
//! - `force_recalculate_trust_score(who)` - Manually recalculate specific user's score (root)
//! - `update_all_trust_scores()` - Trigger batch update of all citizens (root)
//!
//! ### Trait Implementations
//!
//! - `TrustScoreProvider` - Query trust scores from other pallets
//! - `TrustScoreUpdater` - Receive notifications of component changes
//!
//! ## Dependencies
//!
//! This pezpallet requires integration with:
//! - `pezpallet-identity-kyc` - Citizenship status verification
//! - `pezpallet-staking-score` - Staking metrics provider
//! - `pezpallet-referral` - Referral score provider
//! - `pezpallet-perwerde` - Education score provider
//! - `pezpallet-tiki` - Social engagement provider
//!
//! ## Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_trust::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type WeightInfo = pezpallet_trust::weights::BizinikiwiWeight<Runtime>;
//!     type Score = u128;
//!     type ScoreMultiplierBase = ConstU128<10_000>;
//!     type UpdateInterval = ConstU32<14400>; // ~1 day in blocks
//!     type StakingScoreSource = StakingScore;
//!     type ReferralScoreSource = Referral;
//!     type PerwerdeScoreSource = Perwerde;
//!     type TikiScoreSource = Tiki;
//!     type CitizenshipSource = IdentityKyc;
//! }
//! ```

pub use pezpallet::*;

pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pezpallet_staking_score::{
	OnStakingDataUpdate, RawScore as StakingRawScore, StakingScoreProvider,
};
/* use pezkuwi_primitives::traits::{
	CitizenshipStatusProvider, PerwerdeScoreProvider, ReferralScoreProvider, RawScore,
	StakingDetails, StakingScoreProvider, TikiScoreProvider, TrustScoreUpdater, TrustScoreProvider
}; */

use core::convert::TryFrom;
use pezframe_system::pezpallet_prelude::BlockNumberFor;

use pezframe_support::pezpallet_prelude::{
	Get, MaxEncodedLen, Member, OptionQuery, Parameter, ValueQuery,
};

pub trait ReferralScoreProvider<AccountId> {
	fn get_referral_score(who: &AccountId) -> u32;
}

// Re-export from identity-kyc pezpallet
pub use pezpallet_identity_kyc::CitizenshipStatusProvider;

pub trait TrustScoreUpdater<AccountId> {
	fn on_score_component_changed(who: &AccountId);
}

/// Noop implementation of TrustScoreUpdater for use in mock environments
/// and pallets that don't need to trigger trust score updates.
impl<AccountId> TrustScoreUpdater<AccountId> for () {
	fn on_score_component_changed(_who: &AccountId) {}
}

pub trait PerwerdeScoreProvider<AccountId> {
	fn get_perwerde_score(who: &AccountId) -> u32;
}

pub trait TrustScoreProvider<AccountId> {
	fn trust_score_of(who: &AccountId) -> u128;
}

pub trait TikiScoreProvider<AccountId> {
	fn get_tiki_score(who: &AccountId) -> u32;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::{weights::WeightInfo, *};
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::traits::{Saturating, Zero};

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config:
		pezframe_system::Config<RuntimeEvent: From<Event<Self>>> + pezpallet_identity_kyc::Config
	{
		type WeightInfo: WeightInfo;

		type Score: Member
			+ Parameter
			+ MaxEncodedLen
			+ Copy
			+ Default
			+ PartialOrd
			+ Saturating
			+ Zero
			+ From<StakingRawScore>
			+ Into<u128>
			+ TryFrom<u128>;

		#[pezpallet::constant]
		type ScoreMultiplierBase: Get<u128>;

		/// Block interval for Trust score updates (e.g. daily)
		#[pezpallet::constant]
		type UpdateInterval: Get<BlockNumberFor<Self>>;

		/// Maximum number of accounts to process per batch update
		/// Prevents DoS by limiting computation per extrinsic call
		#[pezpallet::constant]
		type MaxBatchSize: Get<u32>;

		type StakingScoreSource: StakingScoreProvider<Self::AccountId, BlockNumberFor<Self>>;
		type ReferralScoreSource: ReferralScoreProvider<Self::AccountId>;
		type PerwerdeScoreSource: PerwerdeScoreProvider<Self::AccountId>;
		type TikiScoreSource: TikiScoreProvider<Self::AccountId>;
		type CitizenshipSource: CitizenshipStatusProvider<Self::AccountId>;
	}

	#[pezpallet::storage]
	#[pezpallet::getter(fn trust_score_of)]
	pub type TrustScores<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::Score, ValueQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn total_active_trust_score)]
	pub type TotalActiveTrustScore<T: Config> = StorageValue<_, T::Score, ValueQuery>;

	#[pezpallet::storage]
	pub type LastProcessedAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pezpallet::storage]
	pub type BatchUpdateInProgress<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A user's Trust Score was successfully updated.
		TrustScoreUpdated { who: T::AccountId, old_score: T::Score, new_score: T::Score },
		/// Total active Trust Score on chain updated.
		TotalTrustScoreUpdated { new_total: T::Score },
		/// A batch Trust Score update completed.
		BulkTrustScoreUpdate { count: u32 },
		/// All Trust Scores update completed.
		AllTrustScoresUpdated { total_updated: u32 },
		/// Periodic Trust Score update scheduled for next time.
		PeriodicUpdateScheduled { next_block: BlockNumberFor<T> },
	}

	#[pezpallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		CalculationOverflow,
		NotACitizen,
		UpdateInProgress,
	}

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub start_periodic_updates: bool,
		#[serde(skip)]
		pub _phantom: core::marker::PhantomData<T>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if self.start_periodic_updates {
				// Schedule first periodic update for 1 day later
				let _first_update_block =
					pezframe_system::Pezpallet::<T>::block_number() + T::UpdateInterval::get();

				// Note: Scheduler may not be available during Genesis build
				// In this case, manual start required or scheduled in runtime
				// For now, we are just marking the flag
			}
		}
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let batch_in_progress = BatchUpdateInProgress::<T>::get();
			let interval = T::UpdateInterval::get();

			// Continue in-progress batch update
			if batch_in_progress {
				return Self::do_batch_update();
			}

			// Start new batch at periodic interval
			if !interval.is_zero() && !n.is_zero() && (n % interval).is_zero() {
				return Self::do_batch_update();
			}

			// Fast path: just reading BatchUpdateInProgress
			T::DbWeight::get().reads(1)
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// To manually recalculate a specific user's Trust Score.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(<T as Config>::WeightInfo::force_recalculate_trust_score())]
		pub fn force_recalculate_trust_score(
			origin: OriginFor<T>,
			who: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::update_score_for_account(&who)?;
			Ok(())
		}

		/// Updates Trust Scores of all citizens in bulk
		/// Works in batches for large user base using efficient pagination
		/// UPDATED (Gemini suggestion): Uses iter_from for true O(1) resume
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(<T as Config>::WeightInfo::update_all_trust_scores())]
		pub fn update_all_trust_scores(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			let batch_size = Self::calculate_optimal_batch_size();
			let mut updated_count = 0u32;
			let mut all_processed = true;
			let mut last_account: Option<T::AccountId> = None;

			// Use iter_from for efficient pagination - O(1) resume instead of O(n) scan
			// This is critical for large user bases to prevent chain stalling
			let iterator = match LastProcessedAccount::<T>::get() {
				Some(start_key) => {
					// Resume from last processed account using iter_from
					pezpallet_identity_kyc::KycStatuses::<T>::iter_from(
						pezpallet_identity_kyc::KycStatuses::<T>::hashed_key_for(&start_key),
					)
				},
				None => {
					// Start from beginning
					pezpallet_identity_kyc::KycStatuses::<T>::iter()
				},
			};

			// Process accounts in batch
			for (account, kyc_level) in iterator {
				// Is batch limit full?
				if updated_count >= batch_size {
					// Save last processed account for next batch
					last_account = Some(account);
					all_processed = false;
					break;
				}

				// Only process accounts with Approved KYC (citizens)
				if kyc_level == pezpallet_identity_kyc::types::KycLevel::Approved {
					let _ = Self::update_score_for_account(&account);
					updated_count += 1;
				}

				// Track last processed for checkpoint
				last_account = Some(account);
			}

			// Update state based on completion
			if all_processed {
				LastProcessedAccount::<T>::kill();
				BatchUpdateInProgress::<T>::put(false);
				Self::deposit_event(Event::AllTrustScoresUpdated { total_updated: updated_count });
			} else {
				if let Some(ref account) = last_account {
					LastProcessedAccount::<T>::put(account.clone());
				}
				BatchUpdateInProgress::<T>::put(true);
				Self::deposit_event(Event::BulkTrustScoreUpdate { count: updated_count });
			}

			Ok(())
		}

		/// Function that starts the periodic update
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(<T as Config>::WeightInfo::periodic_trust_score_update())]
		pub fn periodic_trust_score_update(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			// If a previous update is still in progress, wait
			ensure!(!BatchUpdateInProgress::<T>::get(), Error::<T>::UpdateInProgress);

			// Start the new periodic update
			Self::update_all_trust_scores(OriginFor::<T>::root())?;

			// Schedule the next periodic update
			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			let next_update_block = current_block + T::UpdateInterval::get();

			Self::deposit_event(Event::PeriodicUpdateScheduled { next_block: next_update_block });

			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		pub fn calculate_trust_score(who: &T::AccountId) -> Result<T::Score, Error<T>> {
			ensure!(T::CitizenshipSource::is_citizen(who), Error::<T>::NotACitizen);

			let (staking_score_raw, _) = T::StakingScoreSource::get_staking_score(who);
			if staking_score_raw.is_zero() {
				return Ok(T::Score::zero());
			}

			let staking_u128: u128 = staking_score_raw.into();
			let referral_u128: u128 = T::ReferralScoreSource::get_referral_score(who).into();
			let perwerde_u128: u128 = T::PerwerdeScoreSource::get_perwerde_score(who).into();
			let tiki_u128: u128 = T::TikiScoreSource::get_tiki_score(who).into();

			let base = T::ScoreMultiplierBase::get();

			let weighted_sum = staking_u128
				.saturating_mul(100)
				.saturating_add(referral_u128.saturating_mul(300))
				.saturating_add(perwerde_u128.saturating_mul(300))
				.saturating_add(tiki_u128.saturating_mul(300));

			// Safe: both operands are derived from u32 scores, product fits in u128
			let final_score_u128 = staking_u128
				.saturating_mul(weighted_sum)
				.checked_div(base)
				.ok_or(Error::<T>::CalculationOverflow)?;

			let new_trust_score = T::Score::try_from(final_score_u128)
				.map_err(|_| Error::<T>::CalculationOverflow)?;

			Ok(new_trust_score)
		}

		pub fn update_score_for_account(who: &T::AccountId) -> Result<T::Score, Error<T>> {
			let old_score = Self::trust_score_of(who);
			let new_score = Self::calculate_trust_score(who)?;

			if old_score != new_score {
				<TrustScores<T>>::insert(who, new_score);
				let old_total = Self::total_active_trust_score();
				let new_total = old_total.saturating_sub(old_score).saturating_add(new_score);
				<TotalActiveTrustScore<T>>::put(new_total);
				Self::deposit_event(Event::TrustScoreUpdated {
					who: who.clone(),
					old_score,
					new_score,
				});
				Self::deposit_event(Event::TotalTrustScoreUpdated { new_total });
			}
			Ok(new_score)
		}

		/// Returns the configured batch size for trust score updates
		/// Configurable via MaxBatchSize to allow governance control
		fn calculate_optimal_batch_size() -> u32 {
			T::MaxBatchSize::get()
		}

		/// Internal batch update logic used by both on_initialize and extrinsics.
		/// Returns consumed weight.
		fn do_batch_update() -> Weight {
			let batch_size = Self::calculate_optimal_batch_size();
			let mut updated_count = 0u32;
			let mut all_processed = true;
			let mut last_account: Option<T::AccountId> = None;

			let iterator = match LastProcessedAccount::<T>::get() {
				Some(start_key) => pezpallet_identity_kyc::KycStatuses::<T>::iter_from(
					pezpallet_identity_kyc::KycStatuses::<T>::hashed_key_for(&start_key),
				),
				None => pezpallet_identity_kyc::KycStatuses::<T>::iter(),
			};

			for (account, kyc_level) in iterator {
				if updated_count >= batch_size {
					last_account = Some(account);
					all_processed = false;
					break;
				}

				if kyc_level == pezpallet_identity_kyc::types::KycLevel::Approved {
					let _ = Self::update_score_for_account(&account);
					updated_count += 1;
				}

				last_account = Some(account);
			}

			if all_processed {
				LastProcessedAccount::<T>::kill();
				BatchUpdateInProgress::<T>::put(false);
				Self::deposit_event(Event::AllTrustScoresUpdated { total_updated: updated_count });
			} else {
				if let Some(ref account) = last_account {
					LastProcessedAccount::<T>::put(account.clone());
				}
				BatchUpdateInProgress::<T>::put(true);
				Self::deposit_event(Event::BulkTrustScoreUpdate { count: updated_count });
			}

			// Approximate weight
			let base_weight = T::DbWeight::get().reads_writes(2, 2);
			let per_account = T::DbWeight::get().reads_writes(3, 2);
			base_weight.saturating_add(per_account.saturating_mul(updated_count as u64))
		}
	}

	impl<T: Config> TrustScoreProvider<T::AccountId> for Pezpallet<T> {
		fn trust_score_of(who: &T::AccountId) -> u128 {
			Self::trust_score_of(who).into()
		}
	}

	impl<T: Config> TrustScoreUpdater<T::AccountId> for Pezpallet<T> {
		fn on_score_component_changed(who: &T::AccountId) {
			if let Err(e) = Self::update_score_for_account(who) {
				log::error!("Failed to update trust score for {who:?}: {e:?}");
			}
		}
	}

	impl<T: Config> OnStakingDataUpdate<T::AccountId> for Pezpallet<T> {
		fn on_staking_data_changed(who: &T::AccountId) {
			if let Err(e) = Self::update_score_for_account(who) {
				log::error!("Failed to update trust score on staking change for {who:?}: {e:?}");
			}
		}
	}
}
