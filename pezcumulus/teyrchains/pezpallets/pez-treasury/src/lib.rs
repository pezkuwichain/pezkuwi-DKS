// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # PEZ Treasury Pezpallet
//!
//! A pezpallet for managing the PEZ token distribution and treasury with automated halving
//! mechanics.
//!
//! ## Overview
//!
//! This pezpallet manages the complete lifecycle of PEZ token distribution including:
//!
//! - **Genesis Distribution**: One-time initial distribution to treasury, presale, and founder
//!   accounts
//! - **Halving Mechanism**: Automatic reduction of monthly releases every 20,736,000 blocks
//!   (48 releases of 432,000 blocks each; approximately 4 years)
//! - **Monthly Releases**: Scheduled distribution to incentive and government pots
//! - **Multi-Pot System**: Separate accounts for treasury, incentive rewards, and governance
//!
//! ## Token Economics
//!
//! - **Total Supply**: 5,000,000,000 PEZ (5 billion tokens)
//! - **Treasury Allocation**: 96.25% (4,812,500,000 PEZ)
//! - **Presale Allocation**: 1.875% (93,750,000 PEZ)
//! - **Founder Allocation**: 1.875% (93,750,000 PEZ)
//!
//! ## Halving Schedule
//!
//! - **Halving Period**: Every 20,736,000 blocks, i.e. 48 releases of 432,000 blocks
//! - **Period Duration**: 20,736,000 blocks — approximately 4 years
//! - **Distribution**: 75% to Incentive Pot, 25% to Government Pot
//! - **Automatic Halving**: Monthly release amount halves at the start of each new period
//!
//! The block count is the authoritative figure and the year is descriptive. A release period
//! of 432,000 blocks is 30 days at 10 blocks per minute, so 48 of them come to 1,440 days
//! rather than four calendar years. Quoting the years as though they were exact is how a
//! schedule and its documentation drift apart.
//!
//! ## Security Features
//!
//! - **One-Time Genesis**: Genesis distribution can only occur once (protected by storage flag)
//! - **Privileged Operations**: All extrinsics require privileged origin (root or governance)
//! - **Block-Based Scheduling**: Monthly releases based on block numbers for determinism
//!
//! ## Interface
//!
//! ### Extrinsics
//!
//! - `activate_distribution()` - Record that the population threshold has been reached and
//!   start the schedule. Sent by the chain that holds the citizen register; irreversible.
//!
//! There is no extrinsic that mints PEZ, and none that releases funds by hand. Genesis is the
//! only source of supply; releases happen in `on_initialize` and nobody can bring one forward
//! or hold one back. Changing either takes a runtime upgrade, which is slow and visible --
//! that is the point.
//!
//! ### Storage
//!
//! - `HalvingInfo` - Current halving period data and monthly release amount
//! - `MonthlyReleases` - Historical record of all monthly distributions
//! - `DistributionStarted` - Whether the population threshold has been reached
//!
//! ### Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_pez_treasury::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type Assets = Assets;
//!     type WeightInfo = pezpallet_pez_treasury::weights::BizinikiwiWeight<Runtime>;
//!     type PezAssetId = ConstU32<1>; // PEZ asset ID
//!     type TreasuryPalletId = TreasuryPalletId;
//!     type IncentivePotId = IncentivePotId;
//!     type GovernmentPotId = GovernmentPotId;
//!     type ActivationOrigin = EnsureSiblingChain<PeopleChainLocation>;
//! }
//! ```

pub use pezpallet::*;
pub use weights::WeightInfo;

pub mod migrations;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

extern crate alloc;

use alloc::vec;
use pezframe_support::{
	traits::{
		fungibles::{Inspect, Mutate},
		tokens::Preservation,
		Get,
	},
	PalletId,
};
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezsp_runtime::traits::{AccountIdConversion, Saturating, UniqueSaturatedInto, Zero};
use scale_info::TypeInfo;
use xcm::latest::prelude::*;

/// `note_incentive_funding` in the rewards pallet on the People chain.
///
/// Addressed by index because the two chains do not share a runtime type. The pallet index
/// is configured; the call index is fixed by that pallet's call surface, and a test there
/// pins it so this constant cannot go stale unnoticed.
const NOTE_INCENTIVE_FUNDING_CALL_INDEX: u8 = 2;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	// use pezsp_runtime::traits::CheckedDiv;

	/// Releases per halving period. Named "months" because a release period is 432,000 blocks,
	/// which is 30 days at 10 blocks/minute -- so 48 of them is 1,440 days, approximately but
	/// not exactly four years. The block count below is what the chain actually measures.
	pub const HALVING_PERIOD_MONTHS: u32 = 48;
	pub const BLOCKS_PER_MONTH: u32 = 432_000; // ~30 days * 24 hours * 60 minutes * 10 blocks/minute
	pub const HALVING_PERIOD_BLOCKS: u32 = HALVING_PERIOD_MONTHS * BLOCKS_PER_MONTH;

	pub const TOTAL_SUPPLY: u128 = 5_000_000_000 * 1_000_000_000_000; // 5 billion PEZ (12 decimal)
	pub const TREASURY_ALLOCATION: u128 = 4_812_500_000 * 1_000_000_000_000; // %96.25
	pub const PRESALE_ALLOCATION: u128 = 93_750_000 * 1_000_000_000_000; // %1.875
	pub const FOUNDER_ALLOCATION: u128 = 93_750_000 * 1_000_000_000_000; // %1.875

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(migrations::STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		/// Releases happen here and nowhere else.
		///
		/// There is no extrinsic to bring one forward or hold one back; the schedule belongs
		/// to the chain rather than to whoever holds a key. A release that fails -- a balance
		/// short, a pot that will not accept -- must not take the block with it, so the error
		/// is recorded and the same release is attempted again next block. It is never
		/// skipped: a month that is not paid is not a month that goes away.
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let idle = T::DbWeight::get().reads_writes(3, 0);
			if !DistributionStarted::<T>::get() {
				return idle;
			}
			match Self::do_monthly_release() {
				Ok(()) => T::WeightInfo::release_monthly_funds(),
				// `ReleaseTooEarly` is the ordinary answer on all but one block a month; the
				// rest are worth seeing.
				Err(e) => {
					if e != Error::<T>::ReleaseTooEarly.into() {
						log::warn!(target: "pez-treasury", "monthly release failed: {e:?}");
						Self::deposit_event(Event::MonthlyReleaseFailed);
					}
					idle
				},
			}
		}

		/// What the pallet has recorded must add up to what it has actually paid.
		///
		/// The schedule is derived, not accumulated, so nothing here can drift by a rounding
		/// step: every quantity below has exactly one correct value that can be recomputed
		/// from the release index. That is what makes the check worth running -- a mismatch is
		/// never "close enough", it means a release paid an amount no month was owed, or a
		/// month was paid twice, or the period advanced without a release behind it. PEZ's
		/// five billion is fixed, so those are the failures that cannot be undone.
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use pezframe_support::ensure;

			let started = DistributionStarted::<T>::get();
			ensure!(
				started == TreasuryStartBlock::<T>::get().is_some(),
				"the latch and the start block disagree about whether the schedule has begun"
			);

			let next = NextReleaseMonth::<T>::get();
			if !started {
				ensure!(next == 0, "releases were made before the schedule began");
				ensure!(
					MonthlyReleases::<T>::iter().next().is_none(),
					"a release was recorded before the schedule began"
				);
				ensure!(
					TotalIncentiveReleased::<T>::get().is_zero(),
					"incentive funding was reported before the schedule began"
				);
				return Ok(());
			}

			let halving_data = HalvingInfo::<T>::get();
			let mut counted = 0u32;
			let mut summed: BalanceOf<T> = Zero::zero();
			let mut summed_incentive: BalanceOf<T> = Zero::zero();

			for (index, record) in MonthlyReleases::<T>::iter() {
				ensure!(index < next, "a release is recorded for a month that is not due yet");
				ensure!(record.month_index == index, "a release is filed under the wrong month");

				let owed = Self::amount_for_release(index)
					.map_err(|_| "a recorded release has no derivable amount")?;
				ensure!(record.amount_released == owed, "a release paid the wrong amount");

				let incentive = record
					.amount_released
					.checked_mul(&75u32.into())
					.and_then(|v| v.checked_div(&100u32.into()))
					.ok_or("the incentive share of a recorded release does not compute")?;
				ensure!(record.incentive_amount == incentive, "the incentive share is wrong");
				ensure!(
					record.government_amount == record.amount_released.saturating_sub(incentive),
					"the government share is wrong"
				);

				counted = counted.saturating_add(1);
				summed = summed.saturating_add(record.amount_released);
				summed_incentive = summed_incentive.saturating_add(record.incentive_amount);
			}

			ensure!(
				TotalIncentiveReleased::<T>::get() == summed_incentive,
				"the reported incentive total does not match the records"
			);

			// Every month up to `next` must have been paid. Counting is enough to prove it:
			// the loop above rejects any index at or beyond `next`, so `next` distinct records
			// below `next` leaves no room for a gap.
			ensure!(counted == next, "there is a gap in the release history");
			ensure!(
				summed == halving_data.total_released,
				"total_released does not match the records"
			);

			let period = next.saturating_sub(1) / HALVING_PERIOD_MONTHS;
			ensure!(
				halving_data.current_period == period,
				"the halving period does not follow from the releases made"
			);
			let period_amount =
				Self::amount_for_release(period.saturating_mul(HALVING_PERIOD_MONTHS))
					.map_err(|_| "the current period has no derivable amount")?;
			ensure!(
				halving_data.monthly_amount == period_amount,
				"the stored monthly amount does not match the current period"
			);

			Ok(())
		}
	}

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config + TypeInfo {
		type Assets: Mutate<Self::AccountId>;
		type WeightInfo: weights::WeightInfo;

		#[pezpallet::constant]
		type PezAssetId: Get<<Self::Assets as Inspect<Self::AccountId>>::AssetId>;

		#[pezpallet::constant]
		type TreasuryPalletId: Get<PalletId>;

		#[pezpallet::constant]
		type IncentivePotId: Get<PalletId>;

		#[pezpallet::constant]
		type GovernmentPotId: Get<PalletId>;

		/// Who may say that the population threshold has been reached.
		///
		/// The citizen register lives on another chain, so this pallet cannot count for
		/// itself; it is told once, by that chain, and the runtime binds this to that chain's
		/// sovereign origin. What arrives is a fact, not an instruction -- a second message
		/// changes nothing, because the latch only turns one way.
		type ActivationOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Who may spend from the government pot.
		///
		/// The pot is on this chain; the authority to spend it is not. A payment is legitimate
		/// when the budget behind it was approved by Parliament and the officeholder drawing
		/// against it holds the finance portfolio -- and both of those facts live on the
		/// People chain, with the register and the government. So this pallet does not judge
		/// them. It accepts an instruction from that chain and nothing else, which is why the
		/// runtime binds this to the People chain's origin rather than to root: a key that can
		/// pay out of the government pot is a key that can ignore the budget.
		type GovernmentSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Who may spend from the incentive pot.
		///
		/// Same division as the government pot, for the same reason. The pot is here; the
		/// arithmetic that says who is owed what -- trust scores, the citizen register, the
		/// elected Parliament -- is on the People chain. This pallet holds the money and
		/// takes instruction; it does not decide who has earned it.
		type IncentiveSpendOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Sends the funding report to the chain that does the reward arithmetic.
		type XcmSender: SendXcm;

		/// Where that chain is.
		type RewardsChainLocation: Get<Location>;

		/// The rewards pallet's index on that chain.
		#[pezpallet::constant]
		type RewardsPalletIndex: Get<u8>;
	}

	pub type BalanceOf<T> =
		<<T as Config>::Assets as Inspect<<T as pezframe_system::Config>::AccountId>>::Balance;

	#[pezpallet::storage]
	#[pezpallet::getter(fn halving_info)]
	pub type HalvingInfo<T: Config> = StorageValue<_, HalvingData<T>, ValueQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn monthly_releases)]
	pub type MonthlyReleases<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, MonthlyRelease<T>, OptionQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn next_release_month)]
	pub type NextReleaseMonth<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn treasury_start_block)]
	pub type TreasuryStartBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// Whether the population threshold has been reached and the schedule has begun.
	///
	/// One way only. Once the state has enough citizens to start paying them, a later fall in
	/// the count does not stop the payments -- an economy that switched off when the
	/// population dipped would be worse than one that never started.
	#[pezpallet::storage]
	#[pezpallet::getter(fn distribution_started)]
	pub type DistributionStarted<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Everything the incentive pot has ever been given.
	///
	/// Kept separately rather than derived from `total_released`, because the incentive share
	/// is rounded down each month and the sum of the roundings is not the rounding of the sum.
	/// This is the number reported to the rewards chain, and it is reported as a running
	/// total rather than as a monthly delta: a report that never arrives is corrected by the
	/// next one, instead of leaving a month of funding permanently unaccounted for.
	#[pezpallet::storage]
	#[pezpallet::getter(fn total_incentive_released)]
	pub type TotalIncentiveReleased<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct HalvingData<T: Config> {
		pub current_period: u32,
		pub period_start_block: BlockNumberFor<T>,
		pub monthly_amount: BalanceOf<T>,
		pub total_released: BalanceOf<T>,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct MonthlyRelease<T: Config> {
		pub month_index: u32,
		pub release_block: BlockNumberFor<T>,
		pub amount_released: BalanceOf<T>,
		pub incentive_amount: BalanceOf<T>,
		pub government_amount: BalanceOf<T>,
	}

	impl<T: Config> Default for HalvingData<T> {
		fn default() -> Self {
			Self {
				current_period: 0,
				period_start_block: Zero::zero(),
				monthly_amount: Zero::zero(),
				total_released: Zero::zero(),
			}
		}
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		TreasuryInitialized {
			start_block: BlockNumberFor<T>,
			initial_monthly_amount: BalanceOf<T>,
		},
		MonthlyFundsReleased {
			month_index: u32,
			total_amount: BalanceOf<T>,
			incentive_amount: BalanceOf<T>,
			government_amount: BalanceOf<T>,
		},
		/// A due release could not be made. It will be attempted again next block.
		MonthlyReleaseFailed,
		/// The government spent from its pot.
		GovernmentPotSpent {
			beneficiary: T::AccountId,
			amount: BalanceOf<T>,
		},
		/// A reward was paid out of the incentive pot on the rewards chain's instruction.
		IncentivePotSpent {
			beneficiary: T::AccountId,
			amount: BalanceOf<T>,
		},
		/// The funding report could not be sent. The next release reports the running total
		/// again, so the rewards chain catches up without anything being lost.
		IncentiveFundingReportFailed {
			total: BalanceOf<T>,
		},
		NewHalvingPeriod {
			period: u32,
			new_monthly_amount: BalanceOf<T>,
		},
	}

	#[pezpallet::error]
	pub enum Error<T> {
		TreasuryAlreadyInitialized,
		TreasuryNotInitialized,
		MonthlyReleaseAlreadyDone,
		InsufficientTreasuryBalance,
		InvalidHalvingPeriod,
		ReleaseTooEarly,
		/// A spend of zero was requested.
		NothingToSpend,
		/// The government pot does not hold what was asked for.
		InsufficientGovernmentPotBalance,
		/// The incentive pot does not hold what was asked for.
		InsufficientIncentivePotBalance,
	}

	// There is no genesis config. The schedule cannot start at genesis: it starts in the era
	// the citizen register first reports enough citizens, and that register lives on another
	// chain. A genesis flag that started it early would pay the first month to a state that
	// did not yet have the people it was paying.

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Record that the population threshold has been reached, and start the schedule.
		///
		/// The first release is made in the same era this arrives, not a month later: the
		/// state begins paying its citizens on the day it has enough of them. Calling it
		/// again does nothing.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::activate_distribution())]
		pub fn activate_distribution(origin: OriginFor<T>) -> DispatchResult {
			T::ActivationOrigin::ensure_origin(origin)?;
			ensure!(!DistributionStarted::<T>::get(), Error::<T>::TreasuryAlreadyInitialized);
			Self::do_initialize_treasury()?;
			DistributionStarted::<T>::put(true);
			Ok(())
		}

		/// Pay `amount` out of the government pot to `beneficiary`.
		///
		/// This is the only way anything leaves the government pot, and it cannot originate
		/// here: the caller has to be the People chain, speaking for a budget Parliament
		/// approved and a minister who holds the portfolio. What arrives is already
		/// authorised, so all this does is move the money and record that it moved.
		///
		/// It cannot touch the incentive pot, and it cannot touch the treasury account the
		/// releases come from. Those two hold what has not been handed to the government yet;
		/// only the quarter that has already been released is spendable.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::spend_from_government_pot())]
		pub fn spend_from_government_pot(
			origin: OriginFor<T>,
			beneficiary: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::GovernmentSpendOrigin::ensure_origin(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::NothingToSpend);

			let pot = Self::government_pot_account_id();
			T::Assets::transfer(
				T::PezAssetId::get(),
				&pot,
				&beneficiary,
				amount,
				// The pot is a PalletId account with no provider reference of its own, so it
				// must not be reaped by a payment that happens to empty it.
				Preservation::Preserve,
			)
			.map_err(|_| Error::<T>::InsufficientGovernmentPotBalance)?;

			Self::deposit_event(Event::GovernmentPotSpent { beneficiary, amount });
			Ok(())
		}

		/// Pay `amount` out of the incentive pot to `beneficiary`.
		///
		/// The twin of `spend_from_government_pot`, and the only way anything leaves the
		/// incentive pot. Everything that decides the amount -- the trust score, the epoch
		/// rate, the parliamentary seat -- is computed on the rewards chain, which is why
		/// this accepts that chain's origin and nothing else. Root is deliberately not
		/// accepted: a key that can pay out of the incentive pot is a key that can pay
		/// itself the citizens' share.
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::pay_from_incentive_pot())]
		pub fn pay_from_incentive_pot(
			origin: OriginFor<T>,
			beneficiary: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::IncentiveSpendOrigin::ensure_origin(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::NothingToSpend);

			let pot = Self::incentive_pot_account_id();
			T::Assets::transfer(
				T::PezAssetId::get(),
				&pot,
				&beneficiary,
				amount,
				// Same reason as the government pot: a PalletId account with no provider
				// reference of its own must not be reaped by the payment that empties it.
				Preservation::Preserve,
			)
			.map_err(|_| Error::<T>::InsufficientIncentivePotBalance)?;

			Self::deposit_event(Event::IncentivePotSpent { beneficiary, amount });
			Ok(())
		}

		// `force_genesis_distribution` used to live at call index 2. It minted the treasury,
		// presale and founder allocations -- five billion PEZ -- on top of whatever genesis had
		// already minted, guarded only by a flag that is false on a chain built from a genesis
		// preset. There is now no path in this pallet that can create PEZ. Genesis is the only
		// source of supply, and it is fixed.
	}

	impl<T: Config> Pezpallet<T> {
		pub fn treasury_account_id() -> T::AccountId {
			T::TreasuryPalletId::get().into_account_truncating()
		}

		pub fn incentive_pot_account_id() -> T::AccountId {
			T::IncentivePotId::get().into_account_truncating()
		}

		pub fn government_pot_account_id() -> T::AccountId {
			T::GovernmentPotId::get().into_account_truncating()
		}

		/// Tell the rewards chain how much the incentive pot has been given in total.
		///
		/// Sent unpaid, for the same reason as the other messages between these two system
		/// chains: a report the state owes its citizens should not be lost because a
		/// sovereign account was short of fees.
		///
		/// The running total is sent rather than the month's share. A delta that fails to
		/// arrive is a month of rewards nobody can ever claim; a total that fails to arrive
		/// is corrected by the next release without anyone having to notice.
		fn report_incentive_funding(total: BalanceOf<T>) -> Result<(), SendError> {
			// The wire format is `u128` because the two runtimes agree on the number, not on
			// the type: the rewards chain has no way to name this chain's `Balance`.
			let total: u128 = UniqueSaturatedInto::<u128>::unique_saturated_into(total);
			let call = (
				T::RewardsPalletIndex::get(),
				NOTE_INCENTIVE_FUNDING_CALL_INDEX,
				codec::Compact(total),
			)
				.encode();

			let message = Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: call.into(),
				},
			]);

			let (ticket, _) = T::XcmSender::validate(
				&mut Some(T::RewardsChainLocation::get()),
				&mut Some(message),
			)?;
			T::XcmSender::deliver(ticket)?;
			Ok(())
		}

		pub fn do_initialize_treasury() -> DispatchResult {
			ensure!(
				TreasuryStartBlock::<T>::get().is_none(),
				Error::<T>::TreasuryAlreadyInitialized
			);

			let current_block = pezframe_system::Pezpallet::<T>::block_number();

			let treasury_balance = TREASURY_ALLOCATION;
			let first_period_total =
				treasury_balance.checked_div(2).ok_or(Error::<T>::InvalidHalvingPeriod)?;
			let monthly_amount = first_period_total
				.checked_div(HALVING_PERIOD_MONTHS.into())
				.ok_or(Error::<T>::InvalidHalvingPeriod)?;

			let monthly_amount_balance: BalanceOf<T> =
				monthly_amount.try_into().map_err(|_| Error::<T>::InsufficientTreasuryBalance)?;

			let halving_data = HalvingData {
				current_period: 0,
				period_start_block: current_block,
				monthly_amount: monthly_amount_balance,
				total_released: Zero::zero(),
			};

			TreasuryStartBlock::<T>::put(current_block);
			HalvingInfo::<T>::put(halving_data);
			NextReleaseMonth::<T>::put(0);

			Self::deposit_event(Event::TreasuryInitialized {
				start_block: current_block,
				initial_monthly_amount: monthly_amount_balance,
			});

			Ok(())
		}

		/// The monthly amount for release `index`, derived rather than accumulated.
		///
		/// The period is a function of which release this is -- `index / 48` -- so the amount
		/// cannot drift. The counter it replaces was advanced once per call, which meant a
		/// release that arrived late carried the schedule forward with it: a backlog paid out
		/// in one block would halve on every second call and settle the whole backlog at a
		/// rate no month was ever owed. Derived from the index, a release pays what its own
		/// month is due whenever it happens to be made.
		fn amount_for_release(index: u32) -> Result<BalanceOf<T>, Error<T>> {
			let first_period_total =
				TREASURY_ALLOCATION.checked_div(2).ok_or(Error::<T>::InvalidHalvingPeriod)?;
			let initial = first_period_total
				.checked_div(HALVING_PERIOD_MONTHS.into())
				.ok_or(Error::<T>::InvalidHalvingPeriod)?;
			let period = index / HALVING_PERIOD_MONTHS;
			// Beyond ~127 halvings the amount is zero anyway; the shift would be undefined.
			let amount = if period >= 128 { 0u128 } else { initial >> period };
			amount.try_into().map_err(|_| Error::<T>::InvalidHalvingPeriod)
		}

		pub fn do_monthly_release() -> DispatchResult {
			let start_block =
				TreasuryStartBlock::<T>::get().ok_or(Error::<T>::TreasuryNotInitialized)?;

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			let next_month = NextReleaseMonth::<T>::get();

			ensure!(
				!MonthlyReleases::<T>::contains_key(next_month),
				Error::<T>::MonthlyReleaseAlreadyDone
			);

			// Release 0 falls on the activation block itself: the first distribution is made in
			// the era the population threshold is seen, not a month afterwards. Release `m` is
			// due once `m` full periods have passed since then.
			let blocks_passed = current_block.saturating_sub(start_block);
			let due_after: BlockNumberFor<T> = BLOCKS_PER_MONTH.saturating_mul(next_month).into();
			ensure!(blocks_passed >= due_after, Error::<T>::ReleaseTooEarly);

			let mut halving_data = HalvingInfo::<T>::get();
			let period = next_month / HALVING_PERIOD_MONTHS;
			let monthly_amount = Self::amount_for_release(next_month)?;

			// The halving belongs to the release that opens a new period, not to the one that
			// closes the old one. Release 47 is the forty-eighth of period 0 and is paid in
			// full; release 48 is the first of period 1 and is the one that is halved.
			if period > halving_data.current_period {
				halving_data.current_period = period;
				halving_data.monthly_amount = monthly_amount;
				halving_data.period_start_block = current_block;

				Self::deposit_event(Event::NewHalvingPeriod {
					period: halving_data.current_period,
					new_monthly_amount: halving_data.monthly_amount,
				});
			}

			let incentive_amount = monthly_amount
				.checked_mul(&75u32.into())
				.and_then(|v| v.checked_div(&100u32.into()))
				.ok_or(Error::<T>::InvalidHalvingPeriod)?;
			let government_amount = monthly_amount.saturating_sub(incentive_amount);

			let treasury_account = Self::treasury_account_id();
			let incentive_pot = Self::incentive_pot_account_id();
			let government_pot = Self::government_pot_account_id();

			T::Assets::transfer(
				T::PezAssetId::get(),
				&treasury_account,
				&incentive_pot,
				incentive_amount,
				Preservation::Preserve,
			)
			.map_err(|_| Error::<T>::InsufficientTreasuryBalance)?;

			T::Assets::transfer(
				T::PezAssetId::get(),
				&treasury_account,
				&government_pot,
				government_amount,
				Preservation::Preserve,
			)
			.map_err(|_| Error::<T>::InsufficientTreasuryBalance)?;

			halving_data.total_released =
				halving_data.total_released.saturating_add(monthly_amount);
			HalvingInfo::<T>::put(halving_data);

			let release_info = MonthlyRelease {
				month_index: next_month,
				release_block: current_block,
				amount_released: monthly_amount,
				incentive_amount,
				government_amount,
			};

			MonthlyReleases::<T>::insert(next_month, release_info);
			NextReleaseMonth::<T>::put(next_month.saturating_add(1));

			let incentive_total =
				TotalIncentiveReleased::<T>::get().saturating_add(incentive_amount);
			TotalIncentiveReleased::<T>::put(incentive_total);

			// The money has moved whether or not the report gets through, so a failed send
			// must not undo the release. The running total makes the next report a repair.
			if Self::report_incentive_funding(incentive_total).is_err() {
				Self::deposit_event(Event::IncentiveFundingReportFailed { total: incentive_total });
			}

			Self::deposit_event(Event::MonthlyFundsReleased {
				month_index: next_month,
				total_amount: monthly_amount,
				incentive_amount,
				government_amount,
			});

			Ok(())
		}

		pub fn get_current_halving_info() -> HalvingData<T> {
			HalvingInfo::<T>::get()
		}

		pub fn get_incentive_pot_balance() -> BalanceOf<T> {
			let pot_account = Self::incentive_pot_account_id();
			T::Assets::balance(T::PezAssetId::get(), &pot_account)
		}

		pub fn get_government_pot_balance() -> BalanceOf<T> {
			let pot_account = Self::government_pot_account_id();
			T::Assets::balance(T::PezAssetId::get(), &pot_account)
		}
	}
}
