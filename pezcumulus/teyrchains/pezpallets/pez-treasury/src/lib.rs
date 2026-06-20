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
//! - **Halving Mechanism**: Automatic reduction of monthly releases every 48 months (4 years)
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
//! - **Halving Period**: Every 48 months (4 years)
//! - **Period Duration**: 20,736,000 blocks (~4 years at 10 blocks/minute)
//! - **Distribution**: 75% to Incentive Pot, 25% to Government Pot
//! - **Automatic Halving**: Monthly release amount halves at the start of each new period
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
//! - `force_genesis_distribution()` - Perform initial token distribution (one-time only,
//!   privileged)
//! - `initialize_treasury()` - Initialize the halving mechanism and start monthly releases
//!   (privileged)
//! - `release_monthly_funds()` - Release monthly funds to incentive and government pots
//!   (privileged)
//!
//! ### Storage
//!
//! - `HalvingInfo` - Current halving period data and monthly release amount
//! - `MonthlyReleases` - Historical record of all monthly distributions
//! - `GenesisDistributionDone` - Flag to prevent duplicate genesis distribution
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
//!     type PresaleAccount = PresaleAccount;
//!     type FounderAccount = FounderAccount;
//!     type ForceOrigin = EnsureRoot<AccountId>;
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

use pezframe_support::{
	traits::{
		fungibles::{Inspect, Mutate},
		tokens::Preservation,
		Get,
	},
	PalletId,
};
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezsp_runtime::traits::{AccountIdConversion, Saturating, Zero};
use scale_info::TypeInfo;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	// use pezsp_runtime::traits::CheckedDiv;

	pub const HALVING_PERIOD_MONTHS: u32 = 48; // 4 years = 48 months
	pub const BLOCKS_PER_MONTH: u32 = 432_000; // ~30 days * 24 hours * 60 minutes * 10 blocks/minute
	pub const HALVING_PERIOD_BLOCKS: u32 = HALVING_PERIOD_MONTHS * BLOCKS_PER_MONTH;

	pub const TOTAL_SUPPLY: u128 = 5_000_000_000 * 1_000_000_000_000; // 5 billion PEZ (12 decimal)
	pub const TREASURY_ALLOCATION: u128 = 4_812_500_000 * 1_000_000_000_000; // %96.25
	pub const PRESALE_ALLOCATION: u128 = 93_750_000 * 1_000_000_000_000; // %1.875
	pub const FOUNDER_ALLOCATION: u128 = 93_750_000 * 1_000_000_000_000; // %1.875

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(migrations::STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

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

		#[pezpallet::constant]
		type PresaleAccount: Get<Self::AccountId>;

		#[pezpallet::constant]
		type FounderAccount: Get<Self::AccountId>;

		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
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

	#[pezpallet::storage]
	#[pezpallet::getter(fn genesis_distribution_done)]
	pub type GenesisDistributionDone<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct HalvingData<T: Config> {
		pub current_period: u32,
		pub period_start_block: BlockNumberFor<T>,
		pub monthly_amount: BalanceOf<T>,
		pub total_released: BalanceOf<T>,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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
		NewHalvingPeriod {
			period: u32,
			new_monthly_amount: BalanceOf<T>,
		},
		GenesisDistributionCompleted {
			treasury_amount: BalanceOf<T>,
			presale_amount: BalanceOf<T>,
			founder_amount: BalanceOf<T>,
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
		GenesisDistributionAlreadyDone,
	}

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub initialize_treasury: bool,
		#[serde(skip)]
		pub _phantom: core::marker::PhantomData<T>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if self.initialize_treasury {
				let _ = Pezpallet::<T>::do_initialize_treasury();
			}
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::initialize_treasury())]
		pub fn initialize_treasury(origin: OriginFor<T>) -> DispatchResult {
			T::ForceOrigin::ensure_origin(origin)?;
			Self::do_initialize_treasury()
		}

		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::release_monthly_funds())]
		pub fn release_monthly_funds(origin: OriginFor<T>) -> DispatchResult {
			T::ForceOrigin::ensure_origin(origin)?;
			Self::do_monthly_release()
		}

		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::force_genesis_distribution())]
		pub fn force_genesis_distribution(origin: OriginFor<T>) -> DispatchResult {
			T::ForceOrigin::ensure_origin(origin)?;
			Self::do_genesis_distribution()
		}
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

		pub fn do_genesis_distribution() -> DispatchResult {
			// SECURITY: Ensure genesis distribution can only happen once
			ensure!(
				!GenesisDistributionDone::<T>::get(),
				Error::<T>::GenesisDistributionAlreadyDone
			);

			let treasury_account = Self::treasury_account_id();
			let presale_account = T::PresaleAccount::get();
			let founder_account = T::FounderAccount::get();

			let treasury_amount: BalanceOf<T> = TREASURY_ALLOCATION
				.try_into()
				.map_err(|_| Error::<T>::InsufficientTreasuryBalance)?;
			let presale_amount: BalanceOf<T> = PRESALE_ALLOCATION
				.try_into()
				.map_err(|_| Error::<T>::InsufficientTreasuryBalance)?;
			let founder_amount: BalanceOf<T> = FOUNDER_ALLOCATION
				.try_into()
				.map_err(|_| Error::<T>::InsufficientTreasuryBalance)?;

			T::Assets::mint_into(T::PezAssetId::get(), &treasury_account, treasury_amount)?;
			T::Assets::mint_into(T::PezAssetId::get(), &presale_account, presale_amount)?;
			T::Assets::mint_into(T::PezAssetId::get(), &founder_account, founder_amount)?;

			// Mark genesis distribution as completed
			GenesisDistributionDone::<T>::put(true);

			Self::deposit_event(Event::GenesisDistributionCompleted {
				treasury_amount,
				presale_amount,
				founder_amount,
			});

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

		pub fn do_monthly_release() -> DispatchResult {
			let start_block =
				TreasuryStartBlock::<T>::get().ok_or(Error::<T>::TreasuryNotInitialized)?;

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			let next_month = NextReleaseMonth::<T>::get();

			ensure!(
				!MonthlyReleases::<T>::contains_key(next_month),
				Error::<T>::MonthlyReleaseAlreadyDone
			);

			let blocks_passed = current_block.saturating_sub(start_block);
			let months_passed: u32 = (blocks_passed / BLOCKS_PER_MONTH.into())
				.try_into()
				.map_err(|_| Error::<T>::InvalidHalvingPeriod)?;

			// To release month 0, months_passed must be >= 1 (next_month + 1)
			// To release month 1, months_passed must be >= 2
			ensure!(months_passed > next_month, Error::<T>::ReleaseTooEarly);

			let mut halving_data = HalvingInfo::<T>::get();

			let current_period_passed_months =
				months_passed.saturating_sub(halving_data.current_period * HALVING_PERIOD_MONTHS);

			if current_period_passed_months >= HALVING_PERIOD_MONTHS {
				halving_data.current_period = halving_data.current_period.saturating_add(1);
				halving_data.monthly_amount = halving_data
					.monthly_amount
					.checked_div(&2u32.into())
					.ok_or(Error::<T>::InvalidHalvingPeriod)?;
				halving_data.period_start_block = current_block;

				Self::deposit_event(Event::NewHalvingPeriod {
					period: halving_data.current_period,
					new_monthly_amount: halving_data.monthly_amount,
				});
			}

			let monthly_amount = halving_data.monthly_amount;
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
