// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # PEZ Rewards Pezpallet
//!
//! A pezpallet for distributing PEZ token rewards based on trust scores with epoch-based mechanics.
//!
//! ## Overview
//!
//! This pezpallet implements a sophisticated reward distribution system that incentivizes
//! ecosystem participation through trust-based rewards. The system operates in monthly
//! epochs with automatic reward calculation, distribution, and clawback mechanisms.
//!
//! ## Core Mechanisms
//!
//! ### Epoch System
//!
//! - **Duration**: 1 month (~432,000 blocks at 10 blocks/minute)
//! - **States**: Open → ClaimPeriod → Closed
//! - **Claim Window**: 1 week after epoch finalization (~100,800 blocks)
//! - **Automatic Progression**: Scheduler-driven state transitions
//!
//! ### Reward Distribution
//!
//! 1. **Trust Score Recording**: Users record their trust scores during the Open epoch
//! 2. **Epoch Finalization**: Total pool and per-trust-point rewards calculated
//! 3. **Claim Period**: Users claim proportional rewards based on their trust scores
//! 4. **Clawback**: Unclaimed rewards returned to designated recipient after claim period
//!
//! ### Parliamentary NFT Rewards
//!
//! - **Allocation**: 10% of each epoch's incentive pool reserved for NFT holders
//! - **NFT Collection**: ID 100 with 201 Parliamentary NFTs
//! - **Automatic Distribution**: Pro-rata distribution to all NFT holders at epoch finalization
//!
//! ## Reward Calculation Formula
//!
//! ```text
//! user_reward = (user_trust_score / total_trust_score) * epoch_reward_pool
//! ```
//!
//! Where:
//! - `epoch_reward_pool` = Incentive pot balance - 10% parliamentary allocation
//! - `total_trust_score` = Sum of all recorded trust scores in epoch
//! - `user_trust_score` = User's trust score snapshot from epoch
//!
//! ## Interface
//!
//! ### User Extrinsics
//!
//! - `record_trust_score()` - Record current trust score for active epoch
//! - `claim_reward(epoch_index)` - Claim reward from a finalized epoch (within claim period)
//!
//! ### Privileged Extrinsics
//!
//! - `initialize_rewards_system()` - Start the first epoch (one-time, root)
//! - `finalize_epoch()` - Calculate rewards and start claim period (scheduler/root)
//! - `close_epoch(epoch_index)` - Close claim period and claw back unclaimed rewards
//!   (scheduler/root)
//!
//! ### Storage
//!
//! - `EpochInfo` - Current epoch metadata (index, start block, completion count)
//! - `EpochRewardPools` - Historical reward pool data for each epoch
//! - `UserEpochScores` - User trust score snapshots per epoch
//! - `ClaimedRewards` - Tracking claimed rewards per user per epoch
//! - `EpochStatus` - Current state (Open/ClaimPeriod/Closed) for each epoch
//! - `ParliamentaryNftOwners` - Mapping of Parliamentary NFT IDs to owners
//!
//! ## Dependencies
//!
//! This pezpallet requires integration with:
//! - `pezpallet-trust` - Trust score provider
//! - `pezpallet-pez-treasury` - Incentive pot funding source
//! - `pezpallet-nfts` - Parliamentary NFT collection (optional)
//!
//! ## Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_pez_rewards::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type Assets = Assets;
//!     type PezAssetId = ConstU32<1>; // PEZ asset ID
//!     type WeightInfo = pezpallet_pez_rewards::weights::BizinikiwiWeight<Runtime>;
//!     type TrustScoreSource = Trust;
//!     type IncentivePotId = IncentivePotId;
//!     type ClawbackRecipient = ClawbackRecipient; // Governance account
//!     type ForceOrigin = EnsureRoot<AccountId>;
//!     type CollectionId = u32;
//!     type ItemId = u32;
//! }
//! ```

pub use pezpallet::*;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::{
	traits::{
		fungibles::{Inspect, Mutate},
		tokens::Preservation,
		Get,
	},
	PalletId, Parameter,
};
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezpallet_trust::TrustScoreProvider;
use pezsp_runtime::traits::{AccountIdConversion, Member, Saturating, Zero};
use scale_info::TypeInfo;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::traits::{CheckedDiv, CheckedMul};

	/// Epoch (period) constants
	// pub const BLOCKS_PER_EPOCH: u32 = 20; // CHANGED FOR TESTING - Original is 432_000
	pub const BLOCKS_PER_EPOCH: u32 = 432_000; // 1 month = ~30 days * 24 hours * 60 minutes * 10 blocks/minute
	pub const CLAIM_PERIOD_BLOCKS: u32 = 100_800; // 1 week = ~7 days * 24 hours * 60 minutes * 10 blocks/minute

	/// Parliamentary NFT constants
	pub const PARLIAMENTARY_COLLECTION_ID: u32 = 100;
	pub const PARLIAMENTARY_NFT_COUNT: u32 = 201;
	pub const PARLIAMENTARY_REWARD_PERCENT: u32 = 10; // 10% of incentive pool

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config + pezpallet_trust::Config + TypeInfo {
		type Assets: Mutate<Self::AccountId>;
		#[pezpallet::constant]
		type PezAssetId: Get<<Self::Assets as Inspect<Self::AccountId>>::AssetId>;
		type WeightInfo: crate::weights::WeightInfo;

		/// Trust score provider
		type TrustScoreSource: pezpallet_trust::TrustScoreProvider<Self::AccountId>;

		/// Authority to spend from incentive pot
		#[pezpallet::constant]
		type IncentivePotId: Get<PalletId>;

		/// Clawback recipient (Qazi Muhammed)
		#[pezpallet::constant]
		type ClawbackRecipient: Get<Self::AccountId>;

		/// Authority check for root origin
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// NFT Collection ID ve Item ID types - must match pezpallet_nfts::Config
		type CollectionId: Member + Parameter + MaxEncodedLen + Copy + From<u32> + Into<u32>;
		type ItemId: Member + Parameter + MaxEncodedLen + Copy + From<u32> + Into<u32>;
	}

	pub type BalanceOf<T> =
		<<T as Config>::Assets as Inspect<<T as pezframe_system::Config>::AccountId>>::Balance;

	/// Storage holding epoch (period) information
	#[pezpallet::storage]
	#[pezpallet::getter(fn epoch_info)]
	pub type EpochInfo<T: Config> = StorageValue<_, EpochData<T>, ValueQuery>;

	/// Storage holding total reward pool for each epoch
	#[pezpallet::storage]
	#[pezpallet::getter(fn epoch_reward_pools)]
	pub type EpochRewardPools<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, EpochRewardPool<T>, OptionQuery>;

	/// Storage holding user's trust score for a specific epoch
	#[pezpallet::storage]
	#[pezpallet::getter(fn user_epoch_scores)]
	pub type UserEpochScores<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32, // epoch_index
		Blake2_128Concat,
		T::AccountId, // user
		u128,         // trust_score
		OptionQuery,
	>;

	/// Storage tracking whether user has claimed reward from a specific epoch
	#[pezpallet::storage]
	#[pezpallet::getter(fn claimed_rewards)]
	pub type ClaimedRewards<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32, // epoch_index
		Blake2_128Concat,
		T::AccountId, // user
		BalanceOf<T>, // claimed_amount
		OptionQuery,
	>;

	/// Storage holding epoch state (Open, ClaimPeriod, Closed)
	#[pezpallet::storage]
	#[pezpallet::getter(fn epoch_status)]
	pub type EpochStatus<T: Config> = StorageMap<_, Blake2_128Concat, u32, EpochState, ValueQuery>;

	/// Total amount claimed from each epoch's trust score reward pool
	/// Used to calculate correct clawback amount (total_allocated - total_claimed)
	#[pezpallet::storage]
	#[pezpallet::getter(fn epoch_total_claimed)]
	pub type EpochTotalClaimed<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, BalanceOf<T>, ValueQuery>;

	/// Parliamentary NFT ID to owner mapping
	/// This will be populated by governance or runtime integration
	#[pezpallet::storage]
	#[pezpallet::getter(fn parliamentary_nft_owners)]
	pub type ParliamentaryNftOwners<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,          // nft_id
		T::AccountId, // owner
		OptionQuery,
	>;

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct EpochData<T: Config> {
		pub current_epoch: u32,
		pub epoch_start_block: BlockNumberFor<T>,
		pub total_epochs_completed: u32,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct EpochRewardPool<T: Config> {
		pub epoch_index: u32,
		pub total_reward_pool: BalanceOf<T>, // Total reward for this epoch
		pub total_trust_score: u128,         // Total trust score in this epoch
		pub reward_per_trust_point: BalanceOf<T>, // Reward per trust point
		pub participants_count: u32,         // Number of participants
		pub claim_deadline: BlockNumberFor<T>, // Claim deadline
	}

	#[derive(
		Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default,
	)]
	pub enum EpochState {
		#[default]
		Open, // Active epoch - scores being collected
		ClaimPeriod, // Claim period - claims can be made for 1 week
		Closed,      // Closed - unclaimed rewards have been clawed back
	}

	impl<T: Config> Default for EpochData<T> {
		fn default() -> Self {
			Self { current_epoch: 0, epoch_start_block: Zero::zero(), total_epochs_completed: 0 }
		}
	}

	// Part to be added to Event enum in lib.rs (around line ~174)

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New epoch started
		NewEpochStarted { epoch_index: u32, start_block: BlockNumberFor<T> },
		/// Epoch reward pool calculated and claim period started
		EpochRewardPoolCalculated {
			epoch_index: u32,
			total_pool: BalanceOf<T>,
			total_trust_score: u128,
			participants_count: u32,
			claim_deadline: BlockNumberFor<T>,
		},
		/// User claimed their reward
		RewardClaimed { user: T::AccountId, epoch_index: u32, amount: BalanceOf<T> },
		/// Epoch claim period ended and unclaimed rewards were clawed back
		EpochClosed {
			epoch_index: u32,
			unclaimed_amount: BalanceOf<T>,
			clawback_recipient: T::AccountId,
		},
		/// User's trust score recorded for epoch
		TrustScoreRecorded { user: T::AccountId, epoch_index: u32, trust_score: u128 },
		/// Parliamentary NFT reward automatically distributed
		ParliamentaryNftRewardDistributed {
			nft_id: u32,
			owner: T::AccountId,
			amount: BalanceOf<T>,
			epoch: u32,
		},
		/// Parliamentary NFT owner registered (NEW EVENT - for tests.rs:590)
		ParliamentaryOwnerRegistered { nft_id: u32, owner: T::AccountId },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// Reward system not yet initialized
		RewardsNotInitialized,
		/// Epoch not yet finished
		EpochNotFinished,
		/// Reward already claimed for this epoch
		RewardAlreadyClaimed,
		/// Reward pool not yet calculated for this epoch
		RewardPoolNotCalculated,
		/// User has no trust score for this epoch
		NoTrustScoreForEpoch,
		/// Claim period has expired
		ClaimPeriodExpired,
		/// Epoch already closed
		EpochAlreadyClosed,
		/// Insufficient incentive pot balance
		InsufficientIncentivePot,
		/// Invalid epoch index
		InvalidEpochIndex,
		/// Calculation overflow
		CalculationOverflow,
		/// System already initialized
		AlreadyInitialized, // ADD THIS LINE (for tests.rs:37)
		/// User has no reward to claim from this epoch
		NoRewardToClaim, /* ADD THIS LINE (for tests.rs:251 and 333)
		                  * EpochNotFinished already exists in lib.rs as shown in 'help' */
	}

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub start_rewards_system: bool,
		#[serde(skip)]
		pub _phantom: core::marker::PhantomData<T>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if self.start_rewards_system {
				let _ = Pezpallet::<T>::do_initialize_rewards_system();
			}
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Initialize reward system (root only)
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(<T as Config>::WeightInfo::initialize_rewards_system())]
		pub fn initialize_rewards_system(origin: OriginFor<T>) -> DispatchResult {
			<T as Config>::ForceOrigin::ensure_origin(origin)?;
			Self::do_initialize_rewards_system()
		}

		/// Record user's current trust score
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(<T as Config>::WeightInfo::record_trust_score())]
		pub fn record_trust_score(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_record_trust_score(&who)
		}

		/// Finalize epoch and calculate reward pool (called by scheduler)
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(<T as Config>::WeightInfo::finalize_epoch())]
		pub fn finalize_epoch(origin: OriginFor<T>) -> DispatchResult {
			<T as Config>::ForceOrigin::ensure_origin(origin)?;
			Self::do_finalize_epoch()
		}

		/// Claim reward
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(<T as Config>::WeightInfo::claim_reward())]
		pub fn claim_reward(origin: OriginFor<T>, epoch_index: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_claim_reward(&who, epoch_index)
		}

		/// Close epoch and claw back unclaimed rewards (called by scheduler)
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(<T as Config>::WeightInfo::close_epoch())]
		pub fn close_epoch(origin: OriginFor<T>, epoch_index: u32) -> DispatchResult {
			<T as Config>::ForceOrigin::ensure_origin(origin)?;
			Self::do_close_epoch(epoch_index)
		}

		/// Register parliamentary NFT owner (governance only)
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(<T as Config>::WeightInfo::register_parliamentary_nft_owner())]
		pub fn register_parliamentary_nft_owner(
			origin: OriginFor<T>,
			nft_id: u32,
			owner: T::AccountId,
		) -> DispatchResult {
			<T as Config>::ForceOrigin::ensure_origin(origin)?;
			Self::do_register_parliamentary_nft_owner(nft_id, owner);
			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// Return incentive pot account
		pub fn incentive_pot_account_id() -> T::AccountId {
			<T as Config>::IncentivePotId::get().into_account_truncating()
		}

		/// Initialize reward system
		pub fn do_initialize_rewards_system() -> DispatchResult {
			// GUARD: Check if already initialized
			if EpochInfo::<T>::exists() {
				return Err(Error::<T>::AlreadyInitialized.into());
			}

			let current_block = pezframe_system::Pezpallet::<T>::block_number();

			let epoch_data = EpochData {
				current_epoch: 0,
				epoch_start_block: current_block,
				total_epochs_completed: 0,
			};

			EpochInfo::<T>::put(epoch_data);
			EpochStatus::<T>::insert(0, EpochState::Open);

			Self::deposit_event(Event::NewEpochStarted {
				epoch_index: 0,
				start_block: current_block,
			});

			Ok(())
		}

		/// Record user's trust score for current epoch
		pub fn do_record_trust_score(who: &T::AccountId) -> DispatchResult {
			let epoch_data = EpochInfo::<T>::get();
			let current_epoch = epoch_data.current_epoch;

			// Scores can only be recorded in open epochs
			let epoch_state = EpochStatus::<T>::get(current_epoch);
			ensure!(epoch_state == EpochState::Open, Error::<T>::EpochAlreadyClosed);

			// Get trust score
			let trust_score = <T as Config>::TrustScoreSource::trust_score_of(who);
			let trust_score_u128: u128 = trust_score;

			// FIX: Also record zero scores (tests expect this)
			UserEpochScores::<T>::insert(current_epoch, who, trust_score_u128);

			Self::deposit_event(Event::TrustScoreRecorded {
				user: who.clone(),
				epoch_index: current_epoch,
				trust_score: trust_score_u128,
			});

			Ok(())
		}

		/// Finalize epoch and calculate reward pool
		pub fn do_finalize_epoch() -> DispatchResult {
			let mut epoch_data = EpochInfo::<T>::get();
			let current_epoch = epoch_data.current_epoch;
			let current_block = pezframe_system::Pezpallet::<T>::block_number();

			// Check if epoch has finished
			let epoch_duration = current_block.saturating_sub(epoch_data.epoch_start_block);
			ensure!(epoch_duration >= BLOCKS_PER_EPOCH.into(), Error::<T>::EpochNotFinished);

			// GUARD: Epoch already finalized?
			let epoch_state = EpochStatus::<T>::get(current_epoch);
			ensure!(epoch_state == EpochState::Open, Error::<T>::EpochAlreadyClosed);

			// Get incentive pot balance
			let incentive_pot = Self::incentive_pot_account_id();
			let total_reward_pool = T::Assets::balance(T::PezAssetId::get(), &incentive_pot);

			ensure!(total_reward_pool > Zero::zero(), Error::<T>::InsufficientIncentivePot);

			// Parliamentary rewards distribute et (10%)
			Self::distribute_parliamentary_rewards(current_epoch, total_reward_pool)?;

			// Remaining 90% for trust score rewards
			let trust_score_pool = total_reward_pool
				.checked_mul(&90u32.into())
				.and_then(|v| v.checked_div(&100u32.into()))
				.unwrap_or_else(Zero::zero);

			// Calculate total trust score of all users in this epoch
			let mut total_trust_score = 0u128;
			let mut participants_count = 0u32;

			for (_, trust_score) in UserEpochScores::<T>::iter_prefix(current_epoch) {
				total_trust_score = total_trust_score.saturating_add(trust_score);
				participants_count = participants_count.saturating_add(1);
			}

			let reward_per_trust_point = if total_trust_score > 0 {
				let trust_score_balance = BalanceOf::<T>::try_from(total_trust_score)
					.map_err(|_| Error::<T>::CalculationOverflow)?;
				trust_score_pool.checked_div(&trust_score_balance).unwrap_or_else(Zero::zero)
			} else {
				Zero::zero()
			};

			// Talep son tarihini belirle (1 hafta sonra)
			let claim_deadline = current_block.saturating_add(CLAIM_PERIOD_BLOCKS.into());

			// Save reward pool information
			let reward_pool = EpochRewardPool {
				epoch_index: current_epoch,
				total_reward_pool: trust_score_pool,
				total_trust_score,
				reward_per_trust_point,
				participants_count,
				claim_deadline,
			};

			EpochRewardPools::<T>::insert(current_epoch, reward_pool);

			// FIX: Set epoch state to ClaimPeriod (not Closed!)
			EpochStatus::<T>::insert(current_epoch, EpochState::ClaimPeriod);

			// Start new epoch
			let new_epoch = epoch_data.current_epoch.saturating_add(1);
			epoch_data.current_epoch = new_epoch;
			epoch_data.epoch_start_block = current_block;
			epoch_data.total_epochs_completed = epoch_data.total_epochs_completed.saturating_add(1);
			EpochInfo::<T>::put(epoch_data);
			EpochStatus::<T>::insert(new_epoch, EpochState::Open);

			// FIX: Show trust_score_pool in event (not total_reward_pool)
			Self::deposit_event(Event::EpochRewardPoolCalculated {
				epoch_index: current_epoch,
				total_pool: trust_score_pool, // ← 90% pool
				total_trust_score,
				participants_count,
				claim_deadline,
			});

			Self::deposit_event(Event::NewEpochStarted {
				epoch_index: new_epoch,
				start_block: current_block,
			});

			Ok(())
		}

		pub fn do_claim_reward(who: &T::AccountId, epoch_index: u32) -> DispatchResult {
			let current_block = pezframe_system::Pezpallet::<T>::block_number();

			let epoch_state = EpochStatus::<T>::get(epoch_index);
			ensure!(epoch_state == EpochState::ClaimPeriod, Error::<T>::ClaimPeriodExpired);

			ensure!(
				!ClaimedRewards::<T>::contains_key(epoch_index, who),
				Error::<T>::RewardAlreadyClaimed
			);

			let reward_pool = EpochRewardPools::<T>::get(epoch_index)
				.ok_or(Error::<T>::RewardPoolNotCalculated)?;

			ensure!(current_block <= reward_pool.claim_deadline, Error::<T>::ClaimPeriodExpired);

			let user_trust_score = UserEpochScores::<T>::get(epoch_index, who)
				.ok_or(Error::<T>::NoTrustScoreForEpoch)?;

			let user_trust_balance = BalanceOf::<T>::try_from(user_trust_score)
				.map_err(|_| Error::<T>::CalculationOverflow)?;
			let reward_amount = reward_pool
				.reward_per_trust_point
				.checked_mul(&user_trust_balance)
				.ok_or(Error::<T>::CalculationOverflow)?;

			// FIX: If reward is 0, there is nothing to claim
			ensure!(reward_amount > Zero::zero(), Error::<T>::NoRewardToClaim);

			let incentive_pot = Self::incentive_pot_account_id();
			T::Assets::transfer(
				T::PezAssetId::get(),
				&incentive_pot,
				who,
				reward_amount,
				Preservation::Expendable,
			)?;
			ClaimedRewards::<T>::insert(epoch_index, who, reward_amount);

			// Track total claimed for this epoch (used by clawback calculation)
			EpochTotalClaimed::<T>::mutate(epoch_index, |total| {
				*total = total.saturating_add(reward_amount);
			});

			Self::deposit_event(Event::RewardClaimed {
				user: who.clone(),
				epoch_index,
				amount: reward_amount,
			});

			Ok(())
		}

		/// Close epoch and claw back only unclaimed rewards (not entire pot)
		pub fn do_close_epoch(epoch_index: u32) -> DispatchResult {
			let current_block = pezframe_system::Pezpallet::<T>::block_number();

			let epoch_state = EpochStatus::<T>::get(epoch_index);
			ensure!(epoch_state == EpochState::ClaimPeriod, Error::<T>::EpochAlreadyClosed);

			let reward_pool = EpochRewardPools::<T>::get(epoch_index)
				.ok_or(Error::<T>::RewardPoolNotCalculated)?;

			ensure!(current_block > reward_pool.claim_deadline, Error::<T>::ClaimPeriodExpired);

			// Calculate unclaimed amount: total allocated - total claimed
			let total_claimed = EpochTotalClaimed::<T>::get(epoch_index);
			let unclaimed_amount = reward_pool.total_reward_pool.saturating_sub(total_claimed);

			let incentive_pot = Self::incentive_pot_account_id();
			let clawback_recipient = <T as Config>::ClawbackRecipient::get();

			if unclaimed_amount > Zero::zero() {
				// Only transfer the unclaimed portion, not the entire pot balance
				let pot_balance = T::Assets::balance(T::PezAssetId::get(), &incentive_pot);
				// Transfer the lesser of unclaimed_amount and actual pot balance (safety)
				let transfer_amount = core::cmp::min(unclaimed_amount, pot_balance);
				if transfer_amount > Zero::zero() {
					T::Assets::transfer(
						T::PezAssetId::get(),
						&incentive_pot,
						&clawback_recipient,
						transfer_amount,
						Preservation::Expendable,
					)?;
				}
			}

			EpochStatus::<T>::insert(epoch_index, EpochState::Closed);

			Self::deposit_event(Event::EpochClosed {
				epoch_index,
				unclaimed_amount,
				clawback_recipient,
			});

			Ok(())
		}

		/// Return current epoch information
		pub fn get_current_epoch_info() -> EpochData<T> {
			EpochInfo::<T>::get()
		}

		/// Return reward pool information for specific epoch
		pub fn get_epoch_reward_pool(epoch_index: u32) -> Option<EpochRewardPool<T>> {
			EpochRewardPools::<T>::get(epoch_index)
		}

		/// Return user's trust score for specific epoch
		pub fn get_user_trust_score_for_epoch(
			epoch_index: u32,
			who: &T::AccountId,
		) -> Option<u128> {
			UserEpochScores::<T>::get(epoch_index, who)
		}

		/// Return reward amount claimed by user from specific epoch
		pub fn get_claimed_reward(epoch_index: u32, who: &T::AccountId) -> Option<BalanceOf<T>> {
			ClaimedRewards::<T>::get(epoch_index, who)
		}

		/// Distribute rewards to parliamentary NFT holders automatically
		pub fn distribute_parliamentary_rewards(
			epoch: u32,
			total_incentive_pool: BalanceOf<T>,
		) -> DispatchResult {
			let parliamentary_allocation = total_incentive_pool
				.checked_mul(&PARLIAMENTARY_REWARD_PERCENT.into())
				.and_then(|v| v.checked_div(&100u32.into()))
				.unwrap_or_else(Zero::zero);
			let per_nft_reward = parliamentary_allocation
				.checked_div(&PARLIAMENTARY_NFT_COUNT.into())
				.unwrap_or_else(Zero::zero);

			// Skip the loop entirely if per_nft_reward rounds to zero
			if per_nft_reward.is_zero() {
				return Ok(());
			}

			let incentive_pot = Self::incentive_pot_account_id();

			for nft_id in 1..=PARLIAMENTARY_NFT_COUNT {
				if let Some(owner) = Self::get_parliamentary_nft_owner(nft_id) {
					T::Assets::transfer(
						T::PezAssetId::get(),
						&incentive_pot,
						&owner,
						per_nft_reward,
						Preservation::Expendable, /* Allow source account to be deleted even if
						                           * it has no tokens during fund transfer */
					)?;

					Self::deposit_event(Event::ParliamentaryNftRewardDistributed {
						nft_id,
						owner,
						amount: per_nft_reward,
						epoch,
					});
				}
			}

			Ok(())
		}

		/// Get parliamentary NFT owner from our storage
		pub fn get_parliamentary_nft_owner(nft_id: u32) -> Option<T::AccountId> {
			ParliamentaryNftOwners::<T>::get(nft_id)
		}

		/// Register parliamentary NFT owner (can be called by governance)
		pub fn do_register_parliamentary_nft_owner(nft_id: u32, owner: T::AccountId) {
			ParliamentaryNftOwners::<T>::insert(nft_id, owner.clone());

			// NEW: Emit event
			Self::deposit_event(Event::ParliamentaryOwnerRegistered { nft_id, owner });
		}
	}
}
