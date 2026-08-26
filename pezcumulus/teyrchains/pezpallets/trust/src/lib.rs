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
//! if staking_score == 0 { trust = 0 }
//!
//! normalised(x)  = min(x, x_max) * SCALE / x_max            // every part on one scale
//! trust          = Σ normalised(part) * weight(part) / 100  // weights add to 100
//! ```
//!
//! Two things about this are load-bearing.
//!
//! **The gate.** A citizen with nothing staked scores zero, and no amount of education,
//! recruitment or office changes that. A state with no economy can do nothing, so having
//! something at stake is a condition of standing rather than a component of it.
//!
//! **The scale.** Each source reports on a range of its own -- education runs to fifty
//! thousand, referrals to five hundred -- so weights can only mean what they say if the
//! inputs are put on one scale first. They were not: the old formula weighted the raw numbers,
//! which made an education worth a hundred referrals by arithmetic rather than by decision,
//! and made staking's stated weight of 100 amount to seven parts in ten thousand of the total
//! while its real influence was a five-fold multiplier over everything else. Now a perfect
//! record scores exactly `SCALE`, and every election threshold reads as a share of it.
//!
//! Citizenship is required throughout, and losing it removes the score rather than freezing
//! it (`OnCitizenshipRevoked`).
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
//!     type ScoreScale = ConstU32<1_000>;
//!     type StakingWeight = ConstU32<20>;
//!     type ReferralWeight = ConstU32<25>;
//!     type PerwerdeWeight = ConstU32<30>;
//!     type TikiWeight = ConstU32<25>;
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

	/// The most this component can ever report.
	///
	/// Declared by the component rather than assumed here. Trust weights its inputs as
	/// percentages, and a percentage of an unknown range is not a percentage -- without this,
	/// the real weighting is whatever maximum each pallet happens to have, and changing that
	/// maximum silently reweights the whole state.
	fn max_score() -> u32;
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

	/// The most this component can ever report. See `ReferralScoreProvider::max_score`.
	fn max_score() -> u32;
}

pub trait TrustScoreProvider<AccountId> {
	fn trust_score_of(who: &AccountId) -> u128;
}

pub trait TikiScoreProvider<AccountId> {
	fn get_tiki_score(who: &AccountId) -> u32;

	/// The most this component can ever report. See `ReferralScoreProvider::max_score`.
	fn max_score() -> u32;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::{weights::WeightInfo, *};
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::traits::{Saturating, Zero};

	/// The version this pallet's storage layout is at.
	///
	/// Declared so that the first migration has a baseline to compare against. Without it the
	/// in-code and on-chain versions are both an implicit zero, and a migration cannot tell a
	/// chain that has never been migrated from one that has been migrated to zero.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
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

		/// The scale every component is brought onto, and the value a perfect record scores.
		///
		/// Each source reports on a range of its own -- education runs to fifty thousand,
		/// referrals to five hundred -- so the weights below can only mean what they say if
		/// the inputs are first put on one scale. Before this they were not, and the real
		/// weighting was whichever maximum each pallet happened to have: education counted for
		/// a hundred times what bringing people in did, not by anybody's decision but by
		/// arithmetic nobody had looked at.
		#[pezpallet::constant]
		type ScoreScale: Get<u32>;

		/// How much of a citizen's standing each part of their record accounts for.
		///
		/// Percentages, and they are expected to add to a hundred -- `try_state` says so. Set
		/// them and you have set what the state considers a citizen to be made of.
		#[pezpallet::constant]
		type StakingWeight: Get<u32>;
		#[pezpallet::constant]
		type ReferralWeight: Get<u32>;
		#[pezpallet::constant]
		type PerwerdeWeight: Get<u32>;
		#[pezpallet::constant]
		type TikiWeight: Get<u32>;

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

	/// The block until which scores are held still.
	///
	/// The rewards pallet computes a rate against `TotalActiveTrustScore` at one instant and
	/// pays each claimant their own score over the following week. Those two numbers have to
	/// belong to the same roll, or the shares will not add up to the pool -- and a claimant
	/// whose score moved in between would be paid on a different basis from everyone else,
	/// which is exactly the timing game the design is meant to have no room for.
	///
	/// It expires rather than latching. A freeze that had to be lifted by a later call would
	/// leave the roll frozen for good the first time that call did not happen -- and the
	/// thing that would lift it is the same subsystem that set it.
	#[pezpallet::storage]
	#[pezpallet::getter(fn frozen_until)]
	pub type FrozenUntil<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

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
		/// What the trust register claims has to be true of the state it summarises.
		///
		/// This score decides who may stand for office and how rewards are shared, and it is
		/// derived rather than declared -- so it cannot be checked by reading it. What can be
		/// checked is that the arithmetic behind it still means what it is documented to mean:
		/// that the weights are percentages, that the running total matches the register, and
		/// above all that a citizen with nothing staked has nothing.
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use pezframe_support::ensure;

			// Weights are percentages or they are not weights. A set that adds to anything
			// else silently rescales every citizen's standing, in a direction nobody chose.
			let total_weight = T::StakingWeight::get()
				.saturating_add(T::ReferralWeight::get())
				.saturating_add(T::PerwerdeWeight::get())
				.saturating_add(T::TikiWeight::get());
			ensure!(total_weight == 100, "the trust weights do not add up to a hundred");

			let mut summed = T::Score::zero();
			for (who, score) in TrustScores::<T>::iter() {
				summed = summed.saturating_add(score);

				// The gate, checked against the register rather than trusted. Somebody
				// holding standing with nothing staked would be exactly the case the gate
				// exists to make impossible.
				if !score.is_zero() {
					let (staking, _) = T::StakingScoreSource::get_staking_score(&who);
					ensure!(!staking.is_zero(), "an account has standing without anything staked");
					ensure!(
						T::CitizenshipSource::is_citizen(&who),
						"an account has standing without being a citizen"
					);
					ensure!(
						score.into() <= T::ScoreScale::get() as u128,
						"an account scored more than a perfect record"
					);
				}
			}

			ensure!(
				Self::total_active_trust_score() == summed,
				"the running total does not match the register it totals"
			);

			Ok(())
		}

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

		/// Run one batch of the bulk trust-score update.
		///
		/// The same work `on_initialize` does on its own schedule, offered as a call so an
		/// update can be pushed along without waiting for the next interval. It is one
		/// implementation, not a second one: two copies of a paginated loop sharing the same
		/// checkpoint would drift the moment a fix landed in one of them.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(<T as Config>::WeightInfo::update_all_trust_scores())]
		pub fn update_all_trust_scores(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;
			let _ = Self::do_batch_update();
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
		/// Bring one component onto the common scale.
		///
		/// A component that reports no maximum would divide by zero, so it contributes
		/// nothing rather than bringing the block down; and a component reporting above its
		/// own declared maximum is clamped rather than allowed to borrow weight from the
		/// others.
		fn normalise(score: u32, max: u32) -> u128 {
			if max == 0 {
				return 0;
			}
			let scale = T::ScoreScale::get() as u128;
			(score.min(max) as u128).saturating_mul(scale) / (max as u128)
		}

		pub fn calculate_trust_score(who: &T::AccountId) -> Result<T::Score, Error<T>> {
			ensure!(T::CitizenshipSource::is_citizen(who), Error::<T>::NotACitizen);

			// The gate, and it is absolute. A state with no economy can do nothing, so a
			// citizen with nothing staked has no standing here whatever else they have done --
			// no education, no roles and no number of people brought in can substitute for it.
			// Everything below only decides how much standing somebody with a stake has.
			let (staking_score_raw, _) = T::StakingScoreSource::get_staking_score(who);
			if staking_score_raw.is_zero() {
				return Ok(T::Score::zero());
			}

			// Each part on the same scale, then weighted. Staking is one of the four and no
			// longer also a multiplier over the other three: counted twice, it made a large
			// stake worth five times somebody else's identical education, which is money
			// scaling merit through a side door rather than the front one the gate already is.
			let parts = [
				(
					Self::normalise(staking_score_raw, T::StakingScoreSource::max_score()),
					T::StakingWeight::get(),
				),
				(
					Self::normalise(
						T::ReferralScoreSource::get_referral_score(who),
						T::ReferralScoreSource::max_score(),
					),
					T::ReferralWeight::get(),
				),
				(
					Self::normalise(
						T::PerwerdeScoreSource::get_perwerde_score(who),
						T::PerwerdeScoreSource::max_score(),
					),
					T::PerwerdeWeight::get(),
				),
				(
					Self::normalise(
						T::TikiScoreSource::get_tiki_score(who),
						T::TikiScoreSource::max_score(),
					),
					T::TikiWeight::get(),
				),
			];

			let weighted: u128 = parts.iter().fold(0u128, |acc, (normalised, weight)| {
				acc.saturating_add(normalised.saturating_mul(*weight as u128))
			});

			let final_score_u128 = weighted / 100;

			T::Score::try_from(final_score_u128).map_err(|_| Error::<T>::CalculationOverflow)
		}

		pub fn update_score_for_account(who: &T::AccountId) -> Result<T::Score, Error<T>> {
			// While the roll is frozen the stored score is the answer, unchanged. Note what
			// this does *not* block: `on_citizenship_revoked` takes the score by a different
			// path, so somebody who stops being a citizen mid-payroll still stops being paid.
			if Self::roll_is_frozen() {
				return Ok(Self::trust_score_of(who));
			}

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

		/// Whether the roll is being held still right now.
		pub fn roll_is_frozen() -> bool {
			match FrozenUntil::<T>::get() {
				Some(until) => pezframe_system::Pezpallet::<T>::block_number() <= until,
				None => false,
			}
		}

		/// Hold the roll still until `until`.
		///
		/// Called by the payroll when it fixes an epoch's rate. Later calls can only extend
		/// the hold, never shorten it: a second payroll must not be able to release the roll
		/// the first one is still paying against.
		pub fn freeze_until(until: BlockNumberFor<T>) {
			let extended = match FrozenUntil::<T>::get() {
				Some(current) if current > until => current,
				_ => until,
			};
			FrozenUntil::<T>::put(extended);
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

	impl<T: Config> pezpallet_identity_kyc::types::OnCitizenshipRevoked<T::AccountId> for Pezpallet<T> {
		/// Somebody who is no longer a citizen has no standing to hold.
		///
		/// `calculate_trust_score` refuses to compute for a non-citizen, and refusing meant
		/// the old value was never overwritten -- so a revoked citizen kept whatever standing
		/// they had, for good, and the running total kept counting it. Nothing read it for
		/// candidacy, because that checks citizenship too, but the total is what reward shares
		/// are drawn against.
		fn on_citizenship_revoked(who: &T::AccountId) {
			let old_score = TrustScores::<T>::take(who);
			if !old_score.is_zero() {
				let new_total = Self::total_active_trust_score().saturating_sub(old_score);
				TotalActiveTrustScore::<T>::put(new_total);
				Self::deposit_event(Event::TotalTrustScoreUpdated { new_total });
			}
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
