// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # Validator Pool Pezpallet
//!
//! A pezpallet for managing a decentralized validator pool with multi-category validation.
//!
//! ## Overview
//!
//! This pezpallet provides a flexible validator pool system that supports four distinct
//! validator categories, each with unique requirements and reward mechanisms:
//!
//! - **Community Validators**: Require referral system participation
//! - **Trust Validators**: Require minimum trust score (verified reputation)
//! - **Tiki Validators**: Require minimum tiki score (engagement metrics)
//! - **Stake Validators**: Require minimum stake amount (economic security)
//!
//! ## Features
//!
//! - Multi-category validator pool management
//! - Era-based validator rotation and selection
//! - Performance tracking (blocks produced, missed, reputation)
//! - Weighted random selection based on category and performance
//! - Configurable pool parameters (max validators, pool size, stake requirements)
//! - **Shadow Mode**: Run TNPoS calculations in parallel with standard NPoS for validation
//!
//! ## Shadow Mode
//!
//! Shadow mode allows TNPoS to run in parallel with standard NPoS consensus without
//! affecting actual validator selection. This enables:
//! - Validation of TNPoS algorithm before full deployment
//! - Comparison metrics between NPoS and TNPoS selections
//! - Data collection for algorithm tuning
//! - Safe testing on production networks
//!
//! ## Interface
//!
//! ### Extrinsics
//!
//! - `join_validator_pool(category)` - Join the validator pool in a specific category
//! - `leave_validator_pool()` - Leave the validator pool
//! - `update_performance_metrics(validator, blocks_produced, blocks_missed)` - Update validator
//!   performance (privileged)
//! - `force_new_era()` - Force start a new era with validator selection (privileged)
//! - `update_category(new_category)` - Switch to a different validator category
//! - `set_pool_parameters(...)` - Configure pool parameters (privileged)
//! - `set_operation_mode(mode)` - Switch between Shadow and Active modes (privileged)
//! - `record_npos_validators(validators)` - Record NPoS selection for comparison (privileged)
//!
//! ### Dependencies
//!
//! This pezpallet requires integration with:
//! - `pezpallet-trust` - Trust score provider
//! - `pezpallet-tiki` - Tiki score provider
//! - `pezpallet-referral` - Referral system provider
//! - `pezpallet-perwerde` - Perwerde score provider
//!
//! ### Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_validator_pool::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type WeightInfo = pezpallet_validator_pool::weights::BizinikiwiWeight<Runtime>;
//!     type Randomness = RandomnessCollectiveFlip;
//!     type TrustSource = Trust;
//!     type TikiSource = Tiki;
//!     type ReferralSource = Referral;
//!     type PerwerdeSource = Perwerde;
//!     type PoolManagerOrigin = EnsureRoot<AccountId>;
//!     type MaxValidators = ConstU32<100>;
//!     type MaxPoolSize = ConstU32<500>;
//!     type MinStakeAmount = ConstU128<1_000_000_000_000>; // 1 token
//! }
//! ```

extern crate alloc;

pub use pezpallet::*;
pub mod types;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use crate::types::*;
use alloc::vec::Vec;
use pezframe_support::{
	dispatch::DispatchResult,
	pezpallet_prelude::*,
	traits::{Get, Randomness},
	weights::Weight,
};
use pezframe_system::pezpallet_prelude::*;
use pezsp_runtime::traits::Zero;

/// Trust score provider trait
pub trait TrustScoreProvider<AccountId> {
	fn trust_score_of(who: &AccountId) -> u128;
}

/// Tiki score provider trait  
pub trait TikiScoreProvider<AccountId> {
	fn get_tiki_score(who: &AccountId) -> u32;
}

/// Weight functions trait for this pezpallet.
pub trait WeightInfo {
	fn join_validator_pool() -> Weight;
	fn leave_validator_pool() -> Weight;
	fn update_performance_metrics() -> Weight;
	fn force_new_era(p: u32) -> Weight;
	fn update_category() -> Weight;
	fn set_pool_parameters() -> Weight;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>> {
		type WeightInfo: crate::WeightInfo;
		type Randomness: Randomness<Self::Hash, BlockNumberFor<Self>>;

		/// Trust score provider
		type TrustSource: TrustScoreProvider<Self::AccountId>;
		/// Tiki score provider  
		type TikiSource: TikiScoreProvider<Self::AccountId>;
		/// Referral system provider
		type ReferralSource: ReferralProvider<Self::AccountId>;
		/// Perwerde score provider
		type PerwerdeSource: PerwerdeProvider<Self::AccountId>;

		/// Origin that can manage the pool
		type PoolManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Maximum number of validators per era
		#[pezpallet::constant]
		type MaxValidators: Get<u32>;

		/// Maximum size of validator pool
		#[pezpallet::constant]
		type MaxPoolSize: Get<u32>;

		/// Minimum stake amount for stake validators
		#[pezpallet::constant]
		type MinStakeAmount: Get<u128>;
	}

	// ============================================================================
	// STORAGE ITEMS
	// ============================================================================

	/// Current era index
	#[pezpallet::storage]
	#[pezpallet::getter(fn current_era)]
	pub type CurrentEra<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// When current era started
	#[pezpallet::storage]
	#[pezpallet::getter(fn era_start)]
	pub type EraStart<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// Current selected validator set for this era
	#[pezpallet::storage]
	#[pezpallet::getter(fn current_validator_set)]
	pub type CurrentValidatorSet<T: Config> =
		StorageValue<_, ValidatorSet<T::AccountId>, OptionQuery>;

	/// Validator pool members and their categories
	#[pezpallet::storage]
	#[pezpallet::getter(fn pool_members)]
	pub type PoolMembers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, ValidatorPoolCategory, OptionQuery>;

	/// Performance metrics for each validator
	#[pezpallet::storage]
	#[pezpallet::getter(fn performance_metrics)]
	pub type PerformanceMetrics<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, ValidatorPerformance, ValueQuery>;

	/// Validator selection history (last 5 eras)
	#[pezpallet::storage]
	pub type SelectionHistory<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<u32, ConstU32<5>>, ValueQuery>;

	/// Pool size counter
	#[pezpallet::storage]
	#[pezpallet::getter(fn pool_size)]
	pub type PoolSize<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Era length in blocks
	#[pezpallet::storage]
	#[pezpallet::getter(fn era_length)]
	pub type EraLength<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	// ============================================================================
	// SHADOW MODE STORAGE
	// ============================================================================

	/// Current operation mode (Shadow or Active)
	#[pezpallet::storage]
	#[pezpallet::getter(fn operation_mode)]
	pub type CurrentOperationMode<T: Config> = StorageValue<_, OperationMode, ValueQuery>;

	/// TNPoS validator selection (shadow mode - what would have been selected)
	#[pezpallet::storage]
	#[pezpallet::getter(fn shadow_validator_set)]
	pub type ShadowValidatorSet<T: Config> =
		StorageValue<_, ValidatorSet<T::AccountId>, OptionQuery>;

	/// NPoS validator set for comparison (recorded from actual consensus)
	#[pezpallet::storage]
	#[pezpallet::getter(fn npos_validator_set)]
	pub type NposValidatorSet<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, ConstU32<100>>, ValueQuery>;

	/// Latest shadow comparison result
	#[pezpallet::storage]
	#[pezpallet::getter(fn shadow_comparison)]
	pub type LatestShadowComparison<T: Config> =
		StorageValue<_, ShadowComparison<T::AccountId>, OptionQuery>;

	/// Cumulative shadow statistics
	#[pezpallet::storage]
	#[pezpallet::getter(fn shadow_statistics)]
	pub type CumulativeShadowStats<T: Config> = StorageValue<_, ShadowStatistics, ValueQuery>;

	/// Per-era analysis data (keeps last 10 eras for detailed analysis)
	#[pezpallet::storage]
	#[pezpallet::getter(fn era_analysis)]
	pub type EraAnalysisData<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, EraAnalysis, OptionQuery>;

	/// Category distribution history (last 10 eras)
	#[pezpallet::storage]
	#[pezpallet::getter(fn category_distribution)]
	pub type CategoryDistributionHistory<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, CategoryDistribution, OptionQuery>;

	/// Shadow mode activation block
	#[pezpallet::storage]
	#[pezpallet::getter(fn shadow_mode_since)]
	pub type ShadowModeSince<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	// ============================================================================
	// EVENTS
	// ============================================================================

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A validator joined the pool
		ValidatorJoinedPool { validator: T::AccountId, category: ValidatorPoolCategory },

		/// A validator left the pool
		ValidatorLeftPool { validator: T::AccountId },

		/// New era started with new validator set
		NewEraStarted { era_index: u32, validator_set: ValidatorSet<T::AccountId> },

		/// Validator performance updated
		PerformanceUpdated { validator: T::AccountId, metrics: ValidatorPerformance },

		/// Pool parameters updated
		PoolParametersUpdated { max_validators: u32, era_length: BlockNumberFor<T> },

		/// Validator category updated
		CategoryUpdated {
			validator: T::AccountId,
			old_category: ValidatorPoolCategory,
			new_category: ValidatorPoolCategory,
		},

		// ============================================================================
		// SHADOW MODE EVENTS
		// ============================================================================
		/// Operation mode changed
		OperationModeChanged {
			old_mode: OperationMode,
			new_mode: OperationMode,
			at_block: BlockNumberFor<T>,
		},

		/// Shadow comparison completed for an era
		ShadowComparisonRecorded {
			era_index: u32,
			overlap_count: u32,
			tnpos_only_count: u32,
			npos_only_count: u32,
		},

		/// NPoS validators recorded for comparison
		NposValidatorsRecorded { era_index: u32, validator_count: u32 },

		/// Era analysis data recorded
		EraAnalysisRecorded { era_index: u32, tnpos_total_stake: u128, npos_total_stake: u128 },

		/// Cumulative statistics updated
		ShadowStatisticsUpdated {
			eras_analyzed: u32,
			avg_overlap_bps: u32,
			tnpos_better_eras: u32,
			npos_better_eras: u32,
		},
	}

	// ============================================================================
	// ERRORS
	// ============================================================================

	#[pezpallet::error]
	pub enum Error<T> {
		/// Validator already in pool
		AlreadyInPool,
		/// Validator not in pool
		NotInPool,
		/// Pool is full
		PoolFull,
		/// Insufficient stake amount
		InsufficientStake,
		/// Insufficient trust score
		InsufficientTrustScore,
		/// Missing required Tiki
		MissingRequiredTiki,
		/// Not enough community support
		InsufficientCommunitySupport,
		/// Era transition too early
		EraTransitionTooEarly,
		/// Invalid category
		InvalidCategory,
		/// Not enough eligible validators
		NotEnoughValidators,
		/// Already in shadow mode
		AlreadyInShadowMode,
		/// Already in active mode
		AlreadyInActiveMode,
		/// Shadow mode not enabled
		ShadowModeNotEnabled,
		/// No comparison data available
		NoComparisonData,
	}

	// ============================================================================
	// HOOKS
	// ============================================================================

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			// Always account for the era_start + era_length reads
			let mut weight = T::DbWeight::get().reads(2);

			// Check if we need to transition to new era
			let era_start = Self::era_start();
			let era_length = Self::era_length();

			if block_number >= era_start + era_length && era_length > Zero::zero() {
				// Account for all DB operations in do_new_era + select_validators_for_era:
				// - PoolMembers::iter() reads up to MaxPoolSize entries
				// - SelectionHistory::get() per member
				// - PerformanceMetrics::get() per member
				// - CurrentEra, EraStart, CurrentValidatorSet writes
				// - SelectionHistory::mutate per selected validator
				let pool_size = Self::pool_size();
				weight = weight.saturating_add(
					T::DbWeight::get().reads(pool_size as u64 * 2), // iter + history per member
				);
				weight = weight.saturating_add(
					T::DbWeight::get().writes(3 + pool_size as u64), // era state + history updates
				);

				// Trigger new era if enough time has passed
				if Self::do_new_era().is_err() {
					// Log error but don't panic
				}
				weight = weight.saturating_add(T::WeightInfo::force_new_era(pool_size));
			}

			weight
		}
	}

	// ============================================================================
	// EXTRINSICS
	// ============================================================================

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Join the validator pool
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::join_validator_pool())]
		pub fn join_validator_pool(
			origin: OriginFor<T>,
			category: ValidatorPoolCategory,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(!PoolMembers::<T>::contains_key(&who), Error::<T>::AlreadyInPool);
			ensure!(Self::pool_size() < T::MaxPoolSize::get(), Error::<T>::PoolFull);

			// Validate category requirements
			Self::validate_category_requirements(&who, &category)?;

			// Add to pool
			PoolMembers::<T>::insert(&who, &category);
			PoolSize::<T>::mutate(|size| *size = size.saturating_add(1));

			// Initialize performance metrics
			let initial_performance = ValidatorPerformance {
				blocks_produced: 0,
				blocks_missed: 0,
				era_points: 0,
				last_active_era: Self::current_era(),
				reputation_score: 100, // Start with neutral reputation
			};
			PerformanceMetrics::<T>::insert(&who, initial_performance);

			Self::deposit_event(Event::ValidatorJoinedPool { validator: who, category });

			Ok(())
		}

		/// Leave the validator pool
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::leave_validator_pool())]
		pub fn leave_validator_pool(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(PoolMembers::<T>::contains_key(&who), Error::<T>::NotInPool);

			// Remove from pool
			PoolMembers::<T>::remove(&who);
			PoolSize::<T>::mutate(|size| *size = size.saturating_sub(1));

			// Clean up performance metrics
			PerformanceMetrics::<T>::remove(&who);
			SelectionHistory::<T>::remove(&who);

			Self::deposit_event(Event::ValidatorLeftPool { validator: who });

			Ok(())
		}

		/// Force new era (sudo only)
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::force_new_era(T::MaxPoolSize::get()))]
		pub fn force_new_era(origin: OriginFor<T>) -> DispatchResult {
			T::PoolManagerOrigin::ensure_origin(origin)?;
			Self::do_new_era()?;
			Ok(())
		}

		/// Update validator category
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::update_category())]
		pub fn update_category(
			origin: OriginFor<T>,
			new_category: ValidatorPoolCategory,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let old_category = PoolMembers::<T>::get(&who).ok_or(Error::<T>::NotInPool)?;

			// Validate new category requirements
			Self::validate_category_requirements(&who, &new_category)?;

			PoolMembers::<T>::insert(&who, &new_category);

			Self::deposit_event(Event::CategoryUpdated {
				validator: who,
				old_category,
				new_category,
			});

			Ok(())
		}

		/// Set pool parameters (sudo only)
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::WeightInfo::set_pool_parameters())]
		pub fn set_pool_parameters(
			origin: OriginFor<T>,
			era_length: BlockNumberFor<T>,
		) -> DispatchResult {
			T::PoolManagerOrigin::ensure_origin(origin)?;

			EraLength::<T>::put(era_length);

			Self::deposit_event(Event::PoolParametersUpdated {
				max_validators: T::MaxValidators::get(),
				era_length,
			});

			Ok(())
		}

		/// Update performance metrics (called by consensus)
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(T::WeightInfo::update_performance_metrics())]
		pub fn update_performance_metrics(
			origin: OriginFor<T>,
			validator: T::AccountId,
			blocks_produced: u32,
			blocks_missed: u32,
			era_points: u32,
		) -> DispatchResult {
			T::PoolManagerOrigin::ensure_origin(origin)?;

			PerformanceMetrics::<T>::mutate(&validator, |metrics| {
				metrics.blocks_produced = metrics.blocks_produced.saturating_add(blocks_produced);
				metrics.blocks_missed = metrics.blocks_missed.saturating_add(blocks_missed);
				metrics.era_points = era_points;
				metrics.last_active_era = Self::current_era();

				// Update reputation based on performance
				let total_blocks = metrics.blocks_produced + metrics.blocks_missed;
				if total_blocks > 0 {
					let success_rate = (metrics.blocks_produced * 100) / total_blocks;
					metrics.reputation_score = success_rate.min(100) as u8;
				}
			});

			let updated_metrics = Self::performance_metrics(&validator);
			Self::deposit_event(Event::PerformanceUpdated { validator, metrics: updated_metrics });

			Ok(())
		}

		// ============================================================================
		// SHADOW MODE EXTRINSICS
		// ============================================================================

		/// Set operation mode (Shadow or Active) - sudo only
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::WeightInfo::set_pool_parameters())]
		pub fn set_operation_mode(origin: OriginFor<T>, new_mode: OperationMode) -> DispatchResult {
			T::PoolManagerOrigin::ensure_origin(origin)?;

			let old_mode = Self::operation_mode();

			// Prevent no-op transitions
			match (&old_mode, &new_mode) {
				(OperationMode::Shadow, OperationMode::Shadow) => {
					return Err(Error::<T>::AlreadyInShadowMode.into());
				},
				(OperationMode::Active, OperationMode::Active) => {
					return Err(Error::<T>::AlreadyInActiveMode.into());
				},
				_ => {},
			}

			let current_block = pezframe_system::Pezpallet::<T>::block_number();

			// Update mode
			CurrentOperationMode::<T>::put(new_mode);

			// Track when shadow mode was enabled
			if new_mode == OperationMode::Shadow {
				ShadowModeSince::<T>::put(current_block);
			}

			Self::deposit_event(Event::OperationModeChanged {
				old_mode,
				new_mode,
				at_block: current_block,
			});

			Ok(())
		}

		/// Record NPoS validators for shadow comparison - called by consensus/staking
		#[pezpallet::call_index(7)]
		#[pezpallet::weight(T::WeightInfo::update_performance_metrics())]
		pub fn record_npos_validators(
			origin: OriginFor<T>,
			validators: Vec<T::AccountId>,
		) -> DispatchResult {
			T::PoolManagerOrigin::ensure_origin(origin)?;

			// Only record in shadow mode
			ensure!(
				Self::operation_mode() == OperationMode::Shadow,
				Error::<T>::ShadowModeNotEnabled
			);

			let era_index = Self::current_era();
			let validator_count = validators.len() as u32;

			// Store NPoS validators
			let bounded_validators: BoundedVec<T::AccountId, ConstU32<100>> =
				validators.try_into().unwrap_or_default();
			NposValidatorSet::<T>::put(&bounded_validators);

			// Perform comparison if we have TNPoS selection
			if let Some(tnpos_set) = Self::shadow_validator_set() {
				Self::do_shadow_comparison(&tnpos_set, &bounded_validators, era_index)?;
			}

			Self::deposit_event(Event::NposValidatorsRecorded { era_index, validator_count });

			Ok(())
		}

		/// Record era end statistics for shadow analysis
		#[pezpallet::call_index(8)]
		#[pezpallet::weight(T::WeightInfo::update_performance_metrics())]
		pub fn record_era_end_stats(
			origin: OriginFor<T>,
			era_index: u32,
			blocks_produced: u32,
			blocks_missed: u32,
		) -> DispatchResult {
			T::PoolManagerOrigin::ensure_origin(origin)?;

			// Update era analysis data
			EraAnalysisData::<T>::mutate(era_index, |maybe_analysis| {
				if let Some(analysis) = maybe_analysis {
					analysis.blocks_produced = blocks_produced;
					analysis.blocks_missed = blocks_missed;
				}
			});

			// Update cumulative statistics
			Self::update_cumulative_stats(era_index, blocks_produced);

			Ok(())
		}
	}

	// ============================================================================
	// INTERNAL METHODS
	// ============================================================================

	impl<T: Config> Pezpallet<T> {
		/// Validate category requirements
		fn validate_category_requirements(
			who: &T::AccountId,
			category: &ValidatorPoolCategory,
		) -> DispatchResult {
			// Skip validation during benchmarking
			#[cfg(feature = "runtime-benchmarks")]
			{
				let _ = (who, category);
				Ok(())
			}

			#[cfg(not(feature = "runtime-benchmarks"))]
			{
				match category {
					ValidatorPoolCategory::StakeValidator { min_stake, trust_threshold } => {
						// Check minimum stake (implementation depends on staking pezpallet)
						ensure!(
							*min_stake >= T::MinStakeAmount::get(),
							Error::<T>::InsufficientStake
						);

						// Check trust score
						let trust_score = T::TrustSource::trust_score_of(who);
						ensure!(
							trust_score >= *trust_threshold,
							Error::<T>::InsufficientTrustScore
						);
					},
					ValidatorPoolCategory::ParliamentaryValidator => {
						// Check if user has Parlementer tiki
						let tiki_score = T::TikiSource::get_tiki_score(who);
						ensure!(tiki_score > 0, Error::<T>::MissingRequiredTiki);
					},
					ValidatorPoolCategory::MeritValidator {
						special_tikis: _,
						community_threshold,
					} => {
						// Check special tikis
						let user_tiki_score = T::TikiSource::get_tiki_score(who);
						ensure!(user_tiki_score > 0, Error::<T>::MissingRequiredTiki);

						// Check community support (referral count)
						let referral_count = T::ReferralSource::get_referral_count(who);
						ensure!(
							referral_count >= *community_threshold,
							Error::<T>::InsufficientCommunitySupport
						);
					},
				}
				Ok(())
			}
		}

		/// Perform new era transition
		pub fn do_new_era() -> DispatchResult {
			let current_era = Self::current_era();
			let new_era = current_era.saturating_add(1);

			// Select new validator set
			let new_validator_set = Self::select_validators_for_era()?;

			// Update storage
			CurrentEra::<T>::put(new_era);
			EraStart::<T>::put(pezframe_system::Pezpallet::<T>::block_number());
			CurrentValidatorSet::<T>::put(&new_validator_set);

			// Update selection history for selected validators
			for validator in new_validator_set.all_validators() {
				SelectionHistory::<T>::mutate(validator, |history| {
					if history.try_push(new_era).is_err() {
						// If full, remove oldest and add new
						history.remove(0);
						let _ = history.try_push(new_era);
					}
				});
			}

			Self::deposit_event(Event::NewEraStarted {
				era_index: new_era,
				validator_set: new_validator_set,
			});

			Ok(())
		}

		/// Select validators for new era using randomness and constraints
		fn select_validators_for_era() -> Result<ValidatorSet<T::AccountId>, Error<T>> {
			let target_validators = T::MaxValidators::get();

			// Target distribution: 10 stake, 6 parliamentary, 5 merit
			let stake_target = (target_validators * 10) / 21;
			let parliamentary_target = (target_validators * 6) / 21;
			let merit_target = target_validators - stake_target - parliamentary_target;

			let mut stake_validators = Vec::new();
			let mut parliamentary_validators = Vec::new();
			let mut merit_validators = Vec::new();

			// Get randomness for selection
			let random_seed = T::Randomness::random(b"validator_selection").0;
			let mut random_index = 0u32;

			// Collect eligible validators by category
			// Bounded by MaxPoolSize to prevent unbounded iteration in on_initialize
			let max_pool = T::MaxPoolSize::get() as usize;
			for (validator, category) in PoolMembers::<T>::iter().take(max_pool) {
				// Skip if selected in last 3 eras (rotation rule)
				let history = SelectionHistory::<T>::get(&validator);
				let current_era = Self::current_era();
				if history.iter().any(|&era| current_era.saturating_sub(era) < 3) {
					continue;
				}

				// Check performance threshold
				let performance = Self::performance_metrics(&validator);
				if performance.reputation_score < 70 {
					continue;
				}

				match category {
					ValidatorPoolCategory::StakeValidator { .. } => {
						if stake_validators.len() < stake_target as usize {
							stake_validators.push(validator);
						}
					},
					ValidatorPoolCategory::ParliamentaryValidator => {
						if parliamentary_validators.len() < parliamentary_target as usize {
							parliamentary_validators.push(validator);
						}
					},
					ValidatorPoolCategory::MeritValidator { .. } => {
						if merit_validators.len() < merit_target as usize {
							merit_validators.push(validator);
						}
					},
				}
			}

			// Shuffle using randomness
			Self::shuffle_validators(&mut stake_validators, &random_seed, &mut random_index);
			Self::shuffle_validators(
				&mut parliamentary_validators,
				&random_seed,
				&mut random_index,
			);
			Self::shuffle_validators(&mut merit_validators, &random_seed, &mut random_index);

			// Take required amounts
			stake_validators.truncate(stake_target as usize);
			parliamentary_validators.truncate(parliamentary_target as usize);
			merit_validators.truncate(merit_target as usize);

			// Ensure minimum validator count
			let total_selected =
				stake_validators.len() + parliamentary_validators.len() + merit_validators.len();
			ensure!(total_selected >= 3, Error::<T>::NotEnoughValidators); // BFT minimum

			let validator_set = ValidatorSet {
				era_index: Self::current_era().saturating_add(1),
				stake_validators: stake_validators
					.try_into()
					.map_err(|_| Error::<T>::NotEnoughValidators)?,
				parliamentary_validators: parliamentary_validators
					.try_into()
					.map_err(|_| Error::<T>::NotEnoughValidators)?,
				merit_validators: merit_validators
					.try_into()
					.map_err(|_| Error::<T>::NotEnoughValidators)?,
			};

			Ok(validator_set)
		}

		/// Simple shuffle implementation using randomness
		fn shuffle_validators(validators: &mut [T::AccountId], seed: &T::Hash, index: &mut u32) {
			let seed_bytes = seed.as_ref();
			for i in (1..validators.len()).rev() {
				let random_byte = seed_bytes.get(*index as usize % seed_bytes.len()).unwrap_or(&0);
				*index = index.saturating_add(1);
				let j = (*random_byte as usize) % (i + 1);
				validators.swap(i, j);
			}
		}

		// ============================================================================
		// SHADOW MODE HELPER METHODS
		// ============================================================================

		/// Perform shadow comparison between TNPoS and NPoS selections
		fn do_shadow_comparison(
			tnpos_set: &ValidatorSet<T::AccountId>,
			npos_validators: &BoundedVec<T::AccountId, ConstU32<100>>,
			era_index: u32,
		) -> DispatchResult {
			let tnpos_all = tnpos_set.all_validators();

			// Calculate overlap
			let mut overlap_count = 0u32;
			let mut tnpos_only = Vec::new();
			let mut npos_only = Vec::new();

			for validator in &tnpos_all {
				if npos_validators.contains(validator) {
					overlap_count += 1;
				} else {
					tnpos_only.push(validator.clone());
				}
			}

			for validator in npos_validators.iter() {
				if !tnpos_all.contains(validator) {
					npos_only.push(validator.clone());
				}
			}

			// Calculate category differences
			let tnpos_stake = tnpos_set.stake_validators.len() as i32;
			let tnpos_parliamentary = tnpos_set.parliamentary_validators.len() as i32;
			let tnpos_merit = tnpos_set.merit_validators.len() as i32;

			// Save lengths before moving
			let tnpos_only_count = tnpos_only.len() as u32;
			let npos_only_count = npos_only.len() as u32;

			// Store comparison
			let comparison = ShadowComparison {
				era_index,
				overlap_count,
				tnpos_only: tnpos_only.try_into().unwrap_or_default(),
				npos_only: npos_only.try_into().unwrap_or_default(),
				stake_diff: tnpos_stake,
				parliamentary_diff: tnpos_parliamentary,
				merit_diff: tnpos_merit,
			};

			LatestShadowComparison::<T>::put(&comparison);

			// Record era analysis
			let block_number = pezframe_system::Pezpallet::<T>::block_number();
			let era_analysis = EraAnalysis {
				era_index,
				recorded_at_block: block_number.try_into().unwrap_or(0u32),
				tnpos_total_stake: 0, // Would need staking integration to fill
				npos_total_stake: 0,
				tnpos_avg_trust: Self::calculate_avg_trust(&tnpos_all),
				npos_avg_trust: Self::calculate_avg_trust(npos_validators),
				tnpos_stake_count: tnpos_stake as u8,
				tnpos_parliamentary_count: tnpos_parliamentary as u8,
				tnpos_merit_count: tnpos_merit as u8,
				blocks_produced: 0, // Filled at era end
				blocks_missed: 0,
			};
			EraAnalysisData::<T>::insert(era_index, era_analysis);

			// Record category distribution
			let target_validators = T::MaxValidators::get();
			let stake_target = (target_validators * 10) / 21;
			let parliamentary_target = (target_validators * 6) / 21;
			let merit_target = target_validators - stake_target - parliamentary_target;

			let distribution = CategoryDistribution {
				stake: tnpos_stake as u8,
				parliamentary: tnpos_parliamentary as u8,
				merit: tnpos_merit as u8,
				target_stake: stake_target as u8,
				target_parliamentary: parliamentary_target as u8,
				target_merit: merit_target as u8,
			};
			CategoryDistributionHistory::<T>::insert(era_index, distribution);

			// Update cumulative stats
			CumulativeShadowStats::<T>::mutate(|stats| {
				stats.eras_analyzed = stats.eras_analyzed.saturating_add(1);
				stats.total_overlap = stats.total_overlap.saturating_add(overlap_count);
				stats.total_tnpos_only = stats.total_tnpos_only.saturating_add(tnpos_only_count);
				stats.total_npos_only = stats.total_npos_only.saturating_add(npos_only_count);

				// Recalculate average overlap
				let total_selections =
					stats.total_overlap + stats.total_tnpos_only + stats.total_npos_only;
				if total_selections > 0 {
					stats.avg_overlap_bps = (stats.total_overlap * 10000) / total_selections;
				}
			});

			// Clean up old era data (keep last 10 eras)
			if era_index > 10 {
				let old_era = era_index.saturating_sub(10);
				EraAnalysisData::<T>::remove(old_era);
				CategoryDistributionHistory::<T>::remove(old_era);
			}

			Self::deposit_event(Event::ShadowComparisonRecorded {
				era_index,
				overlap_count,
				tnpos_only_count,
				npos_only_count,
			});

			Ok(())
		}

		/// Calculate average trust score for a set of validators
		fn calculate_avg_trust(validators: &[T::AccountId]) -> u32 {
			if validators.is_empty() {
				return 0;
			}

			let total_trust: u128 = validators.iter().map(T::TrustSource::trust_score_of).sum();

			(total_trust / validators.len() as u128) as u32
		}

		/// Update cumulative statistics at era end
		fn update_cumulative_stats(era_index: u32, blocks_produced: u32) {
			if let Some(era_analysis) = Self::era_analysis(era_index) {
				CumulativeShadowStats::<T>::mutate(|stats| {
					stats.npos_actual_blocks =
						stats.npos_actual_blocks.saturating_add(blocks_produced as u64);

					// Project TNPoS blocks based on historical performance of selected validators
					let projected = Self::project_tnpos_blocks(era_index);
					stats.tnpos_projected_blocks =
						stats.tnpos_projected_blocks.saturating_add(projected);

					// Compare performance
					if projected > blocks_produced as u64 {
						stats.tnpos_better_eras = stats.tnpos_better_eras.saturating_add(1);
					} else if projected < blocks_produced as u64 {
						stats.npos_better_eras = stats.npos_better_eras.saturating_add(1);
					}

					// Track trust representation
					if era_analysis.tnpos_avg_trust > era_analysis.npos_avg_trust {
						stats.higher_trust_eras = stats.higher_trust_eras.saturating_add(1);
					}
				});

				Self::deposit_event(Event::ShadowStatisticsUpdated {
					eras_analyzed: Self::shadow_statistics().eras_analyzed,
					avg_overlap_bps: Self::shadow_statistics().avg_overlap_bps,
					tnpos_better_eras: Self::shadow_statistics().tnpos_better_eras,
					npos_better_eras: Self::shadow_statistics().npos_better_eras,
				});
			}
		}

		/// Project how many blocks TNPoS selection would have produced
		fn project_tnpos_blocks(_era_index: u32) -> u64 {
			if let Some(tnpos_set) = Self::shadow_validator_set() {
				let validators = tnpos_set.all_validators();
				let mut total_projected = 0u64;
				let validator_count = validators.len() as u64;

				if validator_count == 0 {
					return 0;
				}

				for validator in validators {
					let performance = Self::performance_metrics(&validator);
					let total_blocks = performance.blocks_produced + performance.blocks_missed;
					if total_blocks > 0 {
						let success_rate =
							(performance.blocks_produced as u64 * 100) / total_blocks as u64;
						total_projected = total_projected.saturating_add(success_rate);
					} else {
						// New validator, assume 90% success rate
						total_projected = total_projected.saturating_add(90);
					}
				}

				// Average success rate, projected to typical era blocks
				let avg_success_rate = total_projected / validator_count;
				let era_length = Self::era_length();
				let era_blocks: u64 = era_length.try_into().unwrap_or(14400u64); // Default ~1 day

				(era_blocks * avg_success_rate) / 100
			} else {
				0
			}
		}

		/// Store shadow validator set for era (called during era transition in shadow mode)
		pub fn store_shadow_selection() -> DispatchResult {
			if Self::operation_mode() == OperationMode::Shadow {
				if let Ok(validator_set) = Self::select_validators_for_era() {
					ShadowValidatorSet::<T>::put(validator_set);
				}
			}
			Ok(())
		}
	}

	// ============================================================================
	// SESSION MANAGER IMPLEMENTATION
	// ============================================================================

	impl<T: Config> pezpallet_session::SessionManager<T::AccountId> for Pezpallet<T> {
		fn new_session(new_index: u32) -> Option<Vec<T::AccountId>> {
			// Behavior depends on operation mode
			match Self::operation_mode() {
				OperationMode::Shadow => {
					// In shadow mode: calculate TNPoS selection but don't return it
					// Store it for comparison, let NPoS handle actual selection
					let _ = Self::store_shadow_selection();

					// Log shadow mode activity
					log::info!(
						target: "validator_pool",
						"Shadow mode: TNPoS selection calculated for session {new_index}"
					);

					// Return None - let pezpallet-staking/NPoS provide validators
					None
				},
				OperationMode::Active => {
					// In active mode: TNPoS controls validator selection
					// Trigger era transition if needed
					let _ = Self::do_new_era();

					log::info!(
						target: "validator_pool",
						"Active mode: TNPoS providing validators for session {new_index}"
					);

					Self::current_validator_set().map(|set| set.all_validators())
				},
			}
		}

		fn end_session(end_index: u32) {
			// Update performance metrics for ending session
			if let Some(validator_set) = Self::current_validator_set() {
				for validator in validator_set.all_validators() {
					// Increment era participation
					PerformanceMetrics::<T>::mutate(&validator, |metrics| {
						metrics.last_active_era = Self::current_era();
					});
				}
			}

			log::debug!(
				target: "validator_pool",
				"Session {} ended, mode: {:?}",
				end_index,
				Self::operation_mode()
			);
		}

		fn start_session(start_index: u32) {
			// Called when new session starts
			// In shadow mode, this is a good place to compare selections
			if Self::operation_mode() == OperationMode::Shadow {
				log::debug!(
					target: "validator_pool",
					"Session {start_index} started in shadow mode, comparison data available"
				);
			}
		}
	}

	// ============================================================================
	// GENESIS CONFIG
	// ============================================================================

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// Initial operation mode
		pub operation_mode: OperationMode,
		/// Initial era length in blocks
		pub era_length: BlockNumberFor<T>,
		/// Initial pool members (for testing)
		pub initial_pool_members: Vec<(T::AccountId, ValidatorPoolCategory)>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// Set operation mode
			CurrentOperationMode::<T>::put(self.operation_mode);

			// Set era length
			EraLength::<T>::put(self.era_length);

			// Initialize era
			CurrentEra::<T>::put(0u32);
			EraStart::<T>::put(BlockNumberFor::<T>::zero());

			// Add initial pool members
			for (account, category) in &self.initial_pool_members {
				PoolMembers::<T>::insert(account, category);
				PoolSize::<T>::mutate(|size| *size = size.saturating_add(1));

				// Initialize performance metrics
				let initial_performance = ValidatorPerformance {
					blocks_produced: 0,
					blocks_missed: 0,
					era_points: 0,
					last_active_era: 0,
					reputation_score: 100,
				};
				PerformanceMetrics::<T>::insert(account, initial_performance);
			}

			// Track shadow mode activation if starting in shadow
			if self.operation_mode == OperationMode::Shadow {
				ShadowModeSince::<T>::put(BlockNumberFor::<T>::zero());
			}

			log::info!(
				target: "validator_pool",
				"Genesis: operation_mode={:?}, era_length={:?}, pool_size={}",
				self.operation_mode,
				self.era_length,
				self.initial_pool_members.len()
			);
		}
	}
}
