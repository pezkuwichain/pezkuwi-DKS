#![cfg_attr(not(feature = "std"), no_std)]

//! # Pezpallet Presale - Multi-Presale Launchpad Platform
//!
//! ## Overview
//!
//! A comprehensive multi-presale launchpad platform for PezkuwiChain that allows:
//! - Multiple simultaneous presales with independent configurations
//! - Platform fee collection (2%): 50% treasury, 25% burn, 25% stakers
//! - Refund system with grace period (24h low fee, after higher fee)
//! - Contribution limits (min/max per wallet, hard cap)
//! - Whitelist/KYC support for compliance
//! - Vesting schedules for gradual token release
//! - Bonus tier system for larger contributors
//! - Emergency controls and governance integration
//!
//! ## Features
//!
//! - **Multi-Presale**: Unlimited simultaneous presales
//! - **Configurable**: Any asset, rate, duration per presale
//! - **Platform Fee**: 2% split (50% treasury, 25% burn, 25% stakers)
//! - **Refunds**: Grace period with reduced fees
//! - **Limits**: Min/max contribution, hard cap
//! - **Whitelist**: Optional whitelist/KYC for presales
//! - **Vesting**: Linear token release schedules
//! - **Bonus Tiers**: Reward larger contributions
//! - **Emergency**: Pause, cancel, withdrawal controls

pub use pezpallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

extern crate alloc;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use codec::DecodeWithMemTracking;
	use pezframe_support::{
		dispatch::DispatchResult,
		pezpallet_prelude::*,
		traits::{
			fungibles::{Inspect, Mutate},
			tokens::{Fortitude, Precision, Preservation},
		},
		BoundedVec, PalletId,
	};
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::traits::{AtLeast32BitUnsigned, Saturating};

	pub type PresaleId = u32;

	#[derive(
		Clone,
		Copy,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
	)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	pub enum PresaleStatus {
		Pending,    // Not started yet
		Active,     // Ongoing
		Paused,     // Emergency paused (future feature)
		Successful, // Ended, soft cap reached
		Failed,     // Ended, soft cap NOT reached
		Cancelled,  // Emergency cancelled
		Finalized,  // Tokens distributed (after Successful)
	}

	#[derive(
		Clone,
		Copy,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
	)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[codec(dumb_trait_bound)]
	pub enum AccessControl {
		Public,    // Anyone can contribute
		Whitelist, // Only whitelisted accounts
	}

	#[derive(
		Clone,
		Copy,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
	)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[codec(dumb_trait_bound)]
	pub struct BonusTier {
		/// Minimum contribution to qualify (in payment asset units)
		pub min_contribution: u128,
		/// Bonus percentage (0-100)
		pub bonus_percentage: u8,
	}

	#[derive(
		Clone,
		Copy,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
	)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[codec(dumb_trait_bound)]
	pub struct VestingSchedule<BlockNumber> {
		/// Percentage released immediately (0-100)
		pub immediate_release_percent: u8,
		/// Linear vesting over N blocks
		pub vesting_duration_blocks: BlockNumber,
		/// Cliff period before vesting starts
		pub cliff_blocks: BlockNumber,
	}

	#[derive(
		Clone,
		Copy,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
	)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[codec(dumb_trait_bound)]
	pub struct ContributionLimits {
		/// Minimum contribution per wallet
		pub min_contribution: u128,
		/// Maximum contribution per wallet
		pub max_contribution: u128,
		/// Minimum funding target (soft cap) - presale succeeds if reached
		pub soft_cap: u128,
		/// Maximum funding target (hard cap) - presale stops when reached
		pub hard_cap: u128,
	}

	#[derive(
		Clone,
		Copy,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
	)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[codec(dumb_trait_bound)]
	pub struct RefundConfig<BlockNumber> {
		/// Grace period for refunds (blocks) - low fee
		pub grace_period_blocks: BlockNumber,
		/// Normal refund fee percentage (0-100)
		pub refund_fee_percent: u8,
		/// Grace period refund fee percentage (0-100)
		pub grace_refund_fee_percent: u8,
	}

	#[derive(
		Clone,
		Copy,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
	)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[codec(dumb_trait_bound)]
	pub struct PresaleCreationParams<BlockNumber> {
		/// Total tokens for sale (with decimals)
		pub tokens_for_sale: u128,
		/// Presale duration in blocks
		pub duration: BlockNumber,
		/// Whether presale requires whitelist
		pub is_whitelist: bool,
		/// Contribution limits (min, max, soft cap, hard cap)
		pub limits: ContributionLimits,
		/// Optional vesting schedule
		pub vesting: Option<VestingSchedule<BlockNumber>>,
		/// Refund configuration
		pub refund_config: RefundConfig<BlockNumber>,
	}

	#[derive(
		Clone,
		Copy,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
	)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[codec(dumb_trait_bound)]
	pub struct ContributionInfo<BlockNumber> {
		/// Total amount contributed
		pub amount: u128,
		/// Block number when first contributed (for grace period calculation)
		pub contributed_at: BlockNumber,
		/// Whether this contribution was refunded
		pub refunded: bool,
		/// Block number when refunded
		pub refunded_at: Option<BlockNumber>,
		/// Fee paid for refund
		pub refund_fee_paid: u128,
	}

	#[derive(
		Clone,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Eq,
		PartialEq,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T, MaxBonusTiers))]
	#[codec(mel_bound(T: Config, MaxBonusTiers: Get<u32>))]
	pub struct PresaleConfig<T: Config, MaxBonusTiers: Get<u32>> {
		/// Presale creator/owner
		pub owner: T::AccountId,
		/// Payment asset (wUSDT, wUSDC, etc.)
		pub payment_asset: T::AssetId,
		/// Reward token asset
		pub reward_asset: T::AssetId,
		/// Total tokens for sale (with decimals)
		/// Example: 10_000_000 * 10^12 = 10M PEZ with 12 decimals
		pub tokens_for_sale: u128,
		/// Presale start block
		pub start_block: BlockNumberFor<T>,
		/// Presale duration in blocks
		pub duration: BlockNumberFor<T>,
		/// Status
		pub status: PresaleStatus,
		/// Access control
		pub access_control: AccessControl,
		/// Contribution limits
		pub limits: ContributionLimits,
		/// Bonus tiers
		pub bonus_tiers: BoundedVec<BonusTier, MaxBonusTiers>,
		/// Optional vesting schedule
		pub vesting: Option<VestingSchedule<BlockNumberFor<T>>>,
		/// Grace period for refunds (blocks) - low fee
		pub grace_period_blocks: BlockNumberFor<T>,
		/// Normal refund fee percentage (0-100)
		pub refund_fee_percent: u8,
		/// Grace period refund fee percentage (0-100)
		pub grace_refund_fee_percent: u8,
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {
		/// Asset ID type
		type AssetId: Parameter + Member + Copy + MaybeSerializeDeserialize + MaxEncodedLen;

		/// Balance type
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ From<u128>
			+ Into<u128>;

		/// Assets handling
		type Assets: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
			+ Mutate<Self::AccountId>;

		/// The presale pezpallet id, used for deriving sub-account treasuries
		#[pezpallet::constant]
		type PalletId: Get<PalletId>;

		/// Platform treasury account (receives 50% of platform fee)
		#[pezpallet::constant]
		type PlatformTreasury: Get<Self::AccountId>;

		/// Staking reward pool account (receives 25% of platform fee)
		#[pezpallet::constant]
		type StakingRewardPool: Get<Self::AccountId>;

		/// Platform fee percentage (e.g., 2 for 2%)
		#[pezpallet::constant]
		type PlatformFeePercent: Get<u8>;

		/// Maximum number of contributors per presale
		#[pezpallet::constant]
		type MaxContributors: Get<u32>;

		/// Maximum bonus tiers per presale
		#[pezpallet::constant]
		type MaxBonusTiers: Get<u32>;

		/// Maximum whitelisted accounts per presale
		#[pezpallet::constant]
		type MaxWhitelistedAccounts: Get<u32>;

		/// Origin that can create presales (must resolve to an AccountId)
		type CreatePresaleOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

		/// Origin for emergency actions
		type EmergencyOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information
		type PresaleWeightInfo: crate::weights::WeightInfo;
	}

	/// Next presale ID
	#[pezpallet::storage]
	#[pezpallet::getter(fn next_presale_id)]
	pub type NextPresaleId<T: Config> = StorageValue<_, PresaleId, ValueQuery>;

	/// Presale configurations
	#[pezpallet::storage]
	#[pezpallet::getter(fn presales)]
	pub type Presales<T: Config> =
		StorageMap<_, Blake2_128Concat, PresaleId, PresaleConfig<T, T::MaxBonusTiers>, OptionQuery>;

	/// Contributions: (presale_id, account) => ContributionInfo
	#[pezpallet::storage]
	#[pezpallet::getter(fn contributions)]
	pub type Contributions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PresaleId,
		Blake2_128Concat,
		T::AccountId,
		ContributionInfo<BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// Contributors list per presale
	#[pezpallet::storage]
	#[pezpallet::getter(fn contributors)]
	pub type Contributors<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		PresaleId,
		BoundedVec<T::AccountId, T::MaxContributors>,
		ValueQuery,
	>;

	/// Total raised per presale
	#[pezpallet::storage]
	#[pezpallet::getter(fn total_raised)]
	pub type TotalRaised<T: Config> = StorageMap<_, Blake2_128Concat, PresaleId, u128, ValueQuery>;

	/// Whitelist: (presale_id, account) => is_whitelisted
	#[pezpallet::storage]
	#[pezpallet::getter(fn whitelisted)]
	pub type WhitelistedAccounts<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PresaleId,
		Blake2_128Concat,
		T::AccountId,
		bool,
		ValueQuery,
	>;

	/// Vesting claims: (presale_id, account) => claimed_amount
	#[pezpallet::storage]
	#[pezpallet::getter(fn vesting_claimed)]
	pub type VestingClaimed<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PresaleId,
		Blake2_128Concat,
		T::AccountId,
		u128,
		ValueQuery,
	>;

	/// Platform analytics
	#[pezpallet::storage]
	#[pezpallet::getter(fn total_platform_volume)]
	pub type TotalPlatformVolume<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn total_platform_fees)]
	pub type TotalPlatformFees<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn successful_presales)]
	pub type SuccessfulPresales<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Presale created [presale_id, owner, payment_asset, reward_asset]
		PresaleCreated {
			presale_id: PresaleId,
			owner: T::AccountId,
			payment_asset: T::AssetId,
			reward_asset: T::AssetId,
		},
		/// Contribution made [presale_id, who, amount, bonus_amount]
		Contributed { presale_id: PresaleId, who: T::AccountId, amount: u128, bonus_amount: u128 },
		/// Presale finalized [presale_id, total_raised]
		PresaleFinalized { presale_id: PresaleId, total_raised: u128 },
		/// Tokens distributed [presale_id, who, amount]
		Distributed { presale_id: PresaleId, who: T::AccountId, amount: u128 },
		/// Refund processed [presale_id, who, amount, fee]
		Refunded { presale_id: PresaleId, who: T::AccountId, amount: u128, fee: u128 },
		/// Presale cancelled
		PresaleCancelled { presale_id: PresaleId },
		/// Platform fee distributed [treasury_share, burn_share, staker_share]
		PlatformFeeDistributed { treasury_share: u128, burn_share: u128, staker_share: u128 },
		/// Account whitelisted [presale_id, account]
		AccountWhitelisted { presale_id: PresaleId, account: T::AccountId },
		/// Vesting tokens claimed [presale_id, who, amount]
		VestingClaimed { presale_id: PresaleId, who: T::AccountId, amount: u128 },
		/// Presale succeeded [presale_id, total_raised, soft_cap]
		PresaleSuccessful { presale_id: PresaleId, total_raised: u128, soft_cap: u128 },
		/// Presale failed [presale_id, total_raised, soft_cap]
		PresaleFailed { presale_id: PresaleId, total_raised: u128, soft_cap: u128 },
		/// Batch refund completed [presale_id, refunded_count, total_refunded]
		BatchRefundCompleted { presale_id: PresaleId, refunded_count: u32, total_refunded: u128 },
		/// Presale extended [presale_id, additional_blocks, new_end_block]
		PresaleExtended {
			presale_id: PresaleId,
			additional_blocks: BlockNumberFor<T>,
			new_end_block: BlockNumberFor<T>,
		},
		/// Batch distribution completed [presale_id, distributed_count, total_distributed]
		BatchDistributionCompleted {
			presale_id: PresaleId,
			distributed_count: u32,
			total_distributed: u128,
		},
	}

	#[pezpallet::error]
	pub enum Error<T> {
		PresaleNotFound,
		PresaleNotActive,
		PresaleEnded,
		PresaleNotEnded,
		AlreadyFinalized,
		ZeroContribution,
		BelowMinContribution,
		AboveMaxContribution,
		HardCapReached,
		NotWhitelisted,
		TooManyContributors,
		ArithmeticOverflow,
		InvalidTokensForSale,
		InvalidFeePercent,
		NoContribution,
		RefundNotAllowed,
		SoftCapReached,
		InsufficientBalance,
		VestingNotEnabled,
		NothingToClaim,
		NotPresaleOwner,
		TooManyBonusTiers,
		// New errors for soft cap
		PresaleNotFailed,
		PresaleNotSuccessful,
		SoftCapNotReached,
		InvalidSoftCap,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Create a new presale
		///
		/// Parameters are grouped into structs for cleaner API:
		/// - `payment_asset`: The asset used for contributions (e.g., wUSDT)
		/// - `reward_asset`: The token being sold
		/// - `params`: Creation parameters including limits, vesting, and refund config
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::PresaleWeightInfo::create_presale())]
		pub fn create_presale(
			origin: OriginFor<T>,
			payment_asset: T::AssetId,
			reward_asset: T::AssetId,
			params: PresaleCreationParams<BlockNumberFor<T>>,
		) -> DispatchResult {
			// Verify caller is authorized to create presales via CreatePresaleOrigin
			let owner = T::CreatePresaleOrigin::ensure_origin(origin)
				.map_err(|_| Error::<T>::NotPresaleOwner)?;

			ensure!(params.tokens_for_sale > 0, Error::<T>::InvalidTokensForSale);
			ensure!(params.limits.soft_cap > 0, Error::<T>::InvalidTokensForSale);
			ensure!(
				params.limits.soft_cap <= params.limits.hard_cap,
				Error::<T>::InvalidTokensForSale
			);
			ensure!(params.refund_config.refund_fee_percent <= 100, Error::<T>::InvalidFeePercent);
			ensure!(
				params.refund_config.grace_refund_fee_percent <= 100,
				Error::<T>::InvalidFeePercent
			);

			let presale_id = NextPresaleId::<T>::get();
			let start_block = <pezframe_system::Pezpallet<T>>::block_number();

			// Start with empty bonus tiers - can be added later
			let bounded_bonus_tiers = BoundedVec::<BonusTier, T::MaxBonusTiers>::default();

			let access_control =
				if params.is_whitelist { AccessControl::Whitelist } else { AccessControl::Public };

			let config = PresaleConfig {
				owner: owner.clone(),
				payment_asset,
				reward_asset,
				tokens_for_sale: params.tokens_for_sale,
				start_block,
				duration: params.duration,
				status: PresaleStatus::Active,
				access_control,
				limits: params.limits,
				bonus_tiers: bounded_bonus_tiers,
				vesting: params.vesting,
				grace_period_blocks: params.refund_config.grace_period_blocks,
				refund_fee_percent: params.refund_config.refund_fee_percent,
				grace_refund_fee_percent: params.refund_config.grace_refund_fee_percent,
			};

			Presales::<T>::insert(presale_id, config);
			NextPresaleId::<T>::put(presale_id.saturating_add(1));

			Self::deposit_event(Event::PresaleCreated {
				presale_id,
				owner,
				payment_asset,
				reward_asset,
			});

			Ok(())
		}

		/// Contribute to a presale
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::PresaleWeightInfo::contribute())]
		pub fn contribute(
			origin: OriginFor<T>,
			presale_id: PresaleId,
			amount: u128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let presale = Presales::<T>::get(presale_id).ok_or(Error::<T>::PresaleNotFound)?;

			// Checks
			ensure!(presale.status == PresaleStatus::Active, Error::<T>::PresaleNotActive);
			ensure!(amount > 0, Error::<T>::ZeroContribution);

			let current_block = <pezframe_system::Pezpallet<T>>::block_number();
			let end_block = presale.start_block + presale.duration;
			ensure!(current_block < end_block, Error::<T>::PresaleEnded);

			// Check whitelist
			if presale.access_control == AccessControl::Whitelist {
				ensure!(
					WhitelistedAccounts::<T>::get(presale_id, &who),
					Error::<T>::NotWhitelisted
				);
			}

			// Check limits
			let existing_contribution = Contributions::<T>::get(presale_id, &who);
			let current_amount = existing_contribution.as_ref().map(|c| c.amount).unwrap_or(0);
			let new_total = current_amount.saturating_add(amount);

			ensure!(new_total >= presale.limits.min_contribution, Error::<T>::BelowMinContribution);
			ensure!(new_total <= presale.limits.max_contribution, Error::<T>::AboveMaxContribution);

			// Calculate remaining capacity and accept only what fits
			let total_raised = TotalRaised::<T>::get(presale_id);
			let remaining_capacity = presale.limits.hard_cap.saturating_sub(total_raised);

			// Accept only what fits (better UX than failing entire transaction)
			let accepted_amount = amount.min(remaining_capacity);

			// Ensure we can accept something
			ensure!(accepted_amount > 0, Error::<T>::HardCapReached);

			// Use accepted_amount for the rest of the function
			let amount = accepted_amount;
			let new_raised = total_raised.saturating_add(amount);

			// Calculate platform fee (2%)
			let platform_fee = amount.saturating_mul(T::PlatformFeePercent::get() as u128) / 100;
			let net_amount = amount.saturating_sub(platform_fee);

			// Transfer payment asset from user to presale treasury
			let treasury = Self::presale_account_id(presale_id);
			let net_amount_balance: T::Balance = net_amount.into();
			T::Assets::transfer(
				presale.payment_asset,
				&who,
				&treasury,
				net_amount_balance,
				Preservation::Expendable, // Allow user account to die if contributing all funds
			)?;

			// Distribute platform fee
			Self::distribute_platform_fee(presale.payment_asset, &who, platform_fee)?;

			// Track contribution with timestamp preservation
			let contribution = if let Some(existing) = existing_contribution {
				// Update existing contribution - preserve original timestamp
				ContributionInfo {
					amount: existing.amount.saturating_add(amount),
					contributed_at: existing.contributed_at, // ✅ Keep original timestamp
					refunded: false,
					refunded_at: None,
					refund_fee_paid: 0,
				}
			} else {
				// New contribution - add to contributors list
				Contributors::<T>::try_mutate(presale_id, |contributors| -> DispatchResult {
					contributors
						.try_push(who.clone())
						.map_err(|_| Error::<T>::TooManyContributors)?;
					Ok(())
				})?;

				// Create new contribution with current timestamp
				ContributionInfo {
					amount,
					contributed_at: current_block, // ✅ Set timestamp for first contribution only
					refunded: false,
					refunded_at: None,
					refund_fee_paid: 0,
				}
			};

			Contributions::<T>::insert(presale_id, &who, contribution);
			TotalRaised::<T>::insert(presale_id, new_raised);

			// Update platform analytics
			TotalPlatformVolume::<T>::mutate(|v| *v = v.saturating_add(amount));
			TotalPlatformFees::<T>::mutate(|f| *f = f.saturating_add(platform_fee));

			// Note: Bonus amount cannot be accurately calculated until finalization
			// when total_raised is known. We emit 0 here and calculate during distribution.
			Self::deposit_event(Event::Contributed { presale_id, who, amount, bonus_amount: 0 });

			Ok(())
		}

		/// Finalize presale - checks soft cap and sets status to Successful or Failed
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::PresaleWeightInfo::finalize_presale(1))]
		pub fn finalize_presale(origin: OriginFor<T>, presale_id: PresaleId) -> DispatchResult {
			ensure_root(origin)?;

			let mut presale = Presales::<T>::get(presale_id).ok_or(Error::<T>::PresaleNotFound)?;

			ensure!(presale.status == PresaleStatus::Active, Error::<T>::PresaleNotActive);

			let current_block = <pezframe_system::Pezpallet<T>>::block_number();
			let end_block = presale.start_block + presale.duration;
			ensure!(current_block >= end_block, Error::<T>::PresaleNotEnded);

			let total_raised = TotalRaised::<T>::get(presale_id);

			// ✅ CHECK SOFT CAP - Set status accordingly
			if total_raised >= presale.limits.soft_cap {
				// SUCCESS: Soft cap reached - distribute tokens
				presale.status = PresaleStatus::Successful;
				Presales::<T>::insert(presale_id, &presale);

				Self::deposit_event(Event::PresaleSuccessful {
					presale_id,
					total_raised,
					soft_cap: presale.limits.soft_cap,
				});

				// Distribution is done via batch_distribute() extrinsic to avoid
				// unbounded iteration. Status is now Successful — call batch_distribute
				// in batches to distribute tokens to all contributors.
				SuccessfulPresales::<T>::mutate(|c| *c = c.saturating_add(1));

				Self::deposit_event(Event::PresaleFinalized { presale_id, total_raised });
			} else {
				// FAILED: Soft cap NOT reached - enable refunds
				presale.status = PresaleStatus::Failed;
				Presales::<T>::insert(presale_id, &presale);

				Self::deposit_event(Event::PresaleFailed {
					presale_id,
					total_raised,
					soft_cap: presale.limits.soft_cap,
				});
			}

			Ok(())
		}

		/// Refund contribution (before presale ends)
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::PresaleWeightInfo::refund())]
		pub fn refund(origin: OriginFor<T>, presale_id: PresaleId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let presale = Presales::<T>::get(presale_id).ok_or(Error::<T>::PresaleNotFound)?;

			ensure!(presale.status == PresaleStatus::Active, Error::<T>::RefundNotAllowed);

			let current_block = <pezframe_system::Pezpallet<T>>::block_number();
			let end_block = presale.start_block + presale.duration;
			ensure!(current_block < end_block, Error::<T>::RefundNotAllowed);

			let mut contribution_info =
				Contributions::<T>::get(presale_id, &who).ok_or(Error::<T>::NoContribution)?;

			ensure!(!contribution_info.refunded, Error::<T>::RefundNotAllowed);
			ensure!(contribution_info.amount > 0, Error::<T>::NoContribution);

			// Calculate fee based on grace period using ORIGINAL contribution timestamp
			let grace_end =
				contribution_info.contributed_at.saturating_add(presale.grace_period_blocks);
			let fee_percent = if current_block <= grace_end {
				presale.grace_refund_fee_percent
			} else {
				presale.refund_fee_percent
			};

			// Calculate what the treasury actually received (after 2% platform fee at contribution
			// time)
			let platform_fee_at_contribution =
				contribution_info.amount.saturating_mul(T::PlatformFeePercent::get() as u128) / 100;
			let net_in_treasury =
				contribution_info.amount.saturating_sub(platform_fee_at_contribution);

			// Calculate refund fee on the net amount in treasury (not original contribution)
			let fee = net_in_treasury.saturating_mul(fee_percent as u128) / 100;
			let refund_amount = net_in_treasury.saturating_sub(fee);

			let treasury = Self::presale_account_id(presale_id);

			// Step 1: Transfer refund amount to user
			let refund_amount_balance: T::Balance = refund_amount.into();
			T::Assets::transfer(
				presale.payment_asset,
				&treasury,
				&who,
				refund_amount_balance,
				Preservation::Expendable,
			)?;

			// Step 2: Distribute fee from remaining treasury balance
			// Treasury now has exactly 'fee' amount left from this contribution
			if fee > 0 {
				Self::distribute_platform_fee(presale.payment_asset, &treasury, fee)?;
			}

			// Update contribution info (mark as refunded instead of removing)
			contribution_info.refunded = true;
			contribution_info.refunded_at = Some(current_block);
			contribution_info.refund_fee_paid = fee;
			Contributions::<T>::insert(presale_id, &who, contribution_info);

			TotalRaised::<T>::mutate(presale_id, |r| {
				*r = r.saturating_sub(contribution_info.amount)
			});

			Self::deposit_event(Event::Refunded { presale_id, who, amount: refund_amount, fee });

			Ok(())
		}

		/// Claim vested tokens
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::PresaleWeightInfo::claim_vested())]
		pub fn claim_vested(origin: OriginFor<T>, presale_id: PresaleId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let presale = Presales::<T>::get(presale_id).ok_or(Error::<T>::PresaleNotFound)?;

			let vesting = presale.vesting.ok_or(Error::<T>::VestingNotEnabled)?;

			ensure!(presale.status == PresaleStatus::Finalized, Error::<T>::PresaleNotActive);

			let contribution_info =
				Contributions::<T>::get(presale_id, &who).ok_or(Error::<T>::NoContribution)?;
			ensure!(contribution_info.amount > 0, Error::<T>::NoContribution);
			ensure!(!contribution_info.refunded, Error::<T>::NoContribution);

			let current_block = <pezframe_system::Pezpallet<T>>::block_number();
			let end_block = presale.start_block + presale.duration;
			let vesting_start = end_block + vesting.cliff_blocks;

			ensure!(current_block >= vesting_start, Error::<T>::NothingToClaim);

			// Get total raised for dynamic calculation
			let total_raised = TotalRaised::<T>::get(presale_id);

			// Calculate total reward using dynamic rate
			let total_reward = Self::calculate_reward_dynamic(
				contribution_info.amount,
				total_raised,
				presale.tokens_for_sale,
			)?;
			let bonus = Self::calculate_bonus(&presale, contribution_info.amount, total_reward);
			let total_with_bonus = total_reward.saturating_add(bonus);

			// Calculate vested amount
			let already_claimed = VestingClaimed::<T>::get(presale_id, &who);
			let vesting_end = vesting_start + vesting.vesting_duration_blocks;

			let claimable = if current_block >= vesting_end {
				// All vested
				total_with_bonus.saturating_sub(already_claimed)
			} else {
				// Linear vesting
				use pezsp_runtime::traits::SaturatedConversion;
				let elapsed = current_block.saturating_sub(vesting_start);
				let elapsed_u128: u128 = elapsed.saturated_into();
				let duration_u128: u128 = vesting.vesting_duration_blocks.saturated_into();
				let vested_percent = elapsed_u128.saturating_mul(100) / duration_u128;
				let immediate_percent = vesting.immediate_release_percent as u128;
				let vesting_percent = 100u128.saturating_sub(immediate_percent);
				let vested_amount =
					total_with_bonus.saturating_mul(vesting_percent).saturating_mul(vested_percent)
						/ 10000;
				let total_unlocked = vested_amount.saturating_add(already_claimed);
				total_unlocked.saturating_sub(already_claimed)
			};

			ensure!(claimable > 0, Error::<T>::NothingToClaim);

			// Transfer tokens
			let treasury = Self::presale_account_id(presale_id);
			let claimable_balance: T::Balance = claimable.into();
			T::Assets::transfer(
				presale.reward_asset,
				&treasury,
				&who,
				claimable_balance,
				Preservation::Preserve,
			)?;
			VestingClaimed::<T>::insert(
				presale_id,
				&who,
				already_claimed.saturating_add(claimable),
			);

			Self::deposit_event(Event::VestingClaimed { presale_id, who, amount: claimable });

			Ok(())
		}

		/// Add account to whitelist (presale owner only)
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(T::PresaleWeightInfo::add_to_whitelist())]
		pub fn add_to_whitelist(
			origin: OriginFor<T>,
			presale_id: PresaleId,
			account: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let presale = Presales::<T>::get(presale_id).ok_or(Error::<T>::PresaleNotFound)?;

			ensure!(who == presale.owner, Error::<T>::NotPresaleOwner);

			WhitelistedAccounts::<T>::insert(presale_id, &account, true);

			Self::deposit_event(Event::AccountWhitelisted { presale_id, account });

			Ok(())
		}

		/// Cancel presale (emergency - owner or root)
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::PresaleWeightInfo::cancel_presale())]
		pub fn cancel_presale(origin: OriginFor<T>, presale_id: PresaleId) -> DispatchResult {
			// Either EmergencyOrigin or Root can cancel
			if T::EmergencyOrigin::ensure_origin(origin.clone()).is_err() {
				ensure_root(origin)?;
			}

			let mut presale = Presales::<T>::get(presale_id).ok_or(Error::<T>::PresaleNotFound)?;

			// Cannot cancel presales that are already finalized, failed, or cancelled
			ensure!(
				matches!(
					presale.status,
					PresaleStatus::Active
						| PresaleStatus::Pending
						| PresaleStatus::Paused
						| PresaleStatus::Successful
				),
				Error::<T>::AlreadyFinalized,
			);

			presale.status = PresaleStatus::Cancelled;
			Presales::<T>::insert(presale_id, presale);

			Self::deposit_event(Event::PresaleCancelled { presale_id });

			Ok(())
		}

		/// Batch refund contributors when presale is cancelled.
		/// Processes refunds in batches to avoid block weight exhaustion.
		/// Anyone can call this to help refund contributors.
		#[pezpallet::call_index(7)]
		#[pezpallet::weight(T::PresaleWeightInfo::batch_refund_failed_presale(*batch_size))]
		pub fn refund_cancelled_presale(
			origin: OriginFor<T>,
			presale_id: PresaleId,
			start_index: u32,
			batch_size: u32,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let presale = Presales::<T>::get(presale_id).ok_or(Error::<T>::PresaleNotFound)?;

			// Only works on cancelled presales
			ensure!(
				matches!(presale.status, PresaleStatus::Cancelled),
				Error::<T>::PresaleNotFound
			);

			let current_block = <pezframe_system::Pezpallet<T>>::block_number();
			let treasury = Self::presale_account_id(presale_id);
			let contributors = Contributors::<T>::get(presale_id);

			let end_index = start_index.saturating_add(batch_size).min(contributors.len() as u32);

			let mut refunded_count = 0u32;
			let mut total_refunded = 0u128;

			for i in start_index..end_index {
				let contributor = &contributors[i as usize];

				if let Some(contribution_info) = Contributions::<T>::get(presale_id, contributor) {
					if !contribution_info.refunded && contribution_info.amount > 0 {
						// Calculate net amount in treasury (original - platform fee already deducted at contribution)
						let platform_fee_at_contribution = contribution_info
							.amount
							.saturating_mul(T::PlatformFeePercent::get() as u128)
							/ 100;
						let net_in_treasury =
							contribution_info.amount.saturating_sub(platform_fee_at_contribution);

						// Refund the full net amount (cancelled presale = no additional fee)
						let refund_amount: T::Balance = net_in_treasury.into();

						T::Assets::transfer(
							presale.payment_asset,
							&treasury,
							contributor,
							refund_amount,
							Preservation::Preserve,
						)?;

						Contributions::<T>::try_mutate(presale_id, contributor, |maybe_info| {
							if let Some(info) = maybe_info {
								info.refunded = true;
								info.refunded_at = Some(current_block);
								info.refund_fee_paid = platform_fee_at_contribution;
							}
							Ok::<_, Error<T>>(())
						})?;

						refunded_count += 1;
						total_refunded = total_refunded.saturating_add(contribution_info.amount);

						Self::deposit_event(Event::Refunded {
							presale_id,
							who: contributor.clone(),
							amount: contribution_info.amount,
							fee: 0,
						});
					}
				}
			}

			Self::deposit_event(Event::BatchRefundCompleted {
				presale_id,
				refunded_count,
				total_refunded,
			});

			Ok(())
		}

		/// Batch refund for FAILED presales (soft cap not reached)
		/// Anyone can call this to help refund contributors
		/// Processes refunds in batches to avoid gas limits
		#[pezpallet::call_index(8)]
		#[pezpallet::weight(T::PresaleWeightInfo::batch_refund_failed_presale(*batch_size))]
		pub fn batch_refund_failed_presale(
			origin: OriginFor<T>,
			presale_id: PresaleId,
			start_index: u32,
			batch_size: u32,
		) -> DispatchResult {
			ensure_signed(origin)?; // Anyone can trigger

			let presale = Presales::<T>::get(presale_id).ok_or(Error::<T>::PresaleNotFound)?;

			// Only works on FAILED presales (soft cap not reached)
			ensure!(presale.status == PresaleStatus::Failed, Error::<T>::PresaleNotFailed);

			let current_block = <pezframe_system::Pezpallet<T>>::block_number();
			let treasury = Self::presale_account_id(presale_id);
			let contributors = Contributors::<T>::get(presale_id);

			// Calculate end index (don't exceed array length)
			let end_index = start_index.saturating_add(batch_size).min(contributors.len() as u32);

			let mut refunded_count = 0u32;
			let mut total_refunded = 0u128;

			// Process batch
			for i in start_index..end_index {
				let contributor = &contributors[i as usize];

				if let Some(contribution_info) = Contributions::<T>::get(presale_id, contributor) {
					// Skip if already refunded or zero amount
					if !contribution_info.refunded && contribution_info.amount > 0 {
						// Calculate net amount in treasury (original - platform fee already deducted at contribution)
						let platform_fee_at_contribution = contribution_info
							.amount
							.saturating_mul(T::PlatformFeePercent::get() as u128)
							/ 100;
						let net_in_treasury =
							contribution_info.amount.saturating_sub(platform_fee_at_contribution);

						// Refund the full net amount (failed presale = no additional fee)
						let refund_amount: T::Balance = net_in_treasury.into();

						T::Assets::transfer(
							presale.payment_asset,
							&treasury,
							contributor,
							refund_amount,
							Preservation::Preserve,
						)?;

						// Mark as refunded
						Contributions::<T>::try_mutate(presale_id, contributor, |maybe_info| {
							if let Some(info) = maybe_info {
								info.refunded = true;
								info.refunded_at = Some(current_block);
								info.refund_fee_paid = 0; // No fee!
							}
							Ok::<_, Error<T>>(())
						})?;

						refunded_count += 1;
						total_refunded = total_refunded.saturating_add(contribution_info.amount);

						Self::deposit_event(Event::Refunded {
							presale_id,
							who: contributor.clone(),
							amount: contribution_info.amount,
							fee: 0,
						});
					}
				}
			}

			Self::deposit_event(Event::BatchRefundCompleted {
				presale_id,
				refunded_count,
				total_refunded,
			});

			Ok(())
		}

		/// Batch distribute tokens for SUCCESSFUL presales
		/// Anyone can call this to help distribute tokens to contributors
		/// Processes distribution in batches to avoid block weight limits
		#[pezpallet::call_index(9)]
		#[pezpallet::weight(T::PresaleWeightInfo::batch_refund_failed_presale(*batch_size))]
		pub fn batch_distribute(
			origin: OriginFor<T>,
			presale_id: PresaleId,
			start_index: u32,
			batch_size: u32,
		) -> DispatchResult {
			ensure_signed(origin)?; // Anyone can trigger

			let mut presale = Presales::<T>::get(presale_id).ok_or(Error::<T>::PresaleNotFound)?;

			// Only works on SUCCESSFUL presales (soft cap reached, not yet finalized)
			ensure!(presale.status == PresaleStatus::Successful, Error::<T>::PresaleNotSuccessful,);

			let total_raised = TotalRaised::<T>::get(presale_id);
			let treasury = Self::presale_account_id(presale_id);
			let contributors = Contributors::<T>::get(presale_id);

			// Calculate end index (don't exceed array length)
			let end_index = start_index.saturating_add(batch_size).min(contributors.len() as u32);

			let mut distributed_count = 0u32;
			let mut total_distributed = 0u128;

			// Process batch
			for i in start_index..end_index {
				let contributor = &contributors[i as usize];

				let contribution_info = match Contributions::<T>::get(presale_id, contributor) {
					Some(info) => info,
					None => continue,
				};

				// Skip if refunded or zero amount
				if contribution_info.refunded || contribution_info.amount == 0 {
					continue;
				}

				// Skip if already distributed (check VestingClaimed for vesting,
				// or check a distribution flag)
				if VestingClaimed::<T>::contains_key(presale_id, contributor) {
					continue;
				}

				// Calculate reward tokens using dynamic rate (overflow-safe)
				let reward_amount = Self::calculate_reward_dynamic(
					contribution_info.amount,
					total_raised,
					presale.tokens_for_sale,
				)?;

				let bonus =
					Self::calculate_bonus(&presale, contribution_info.amount, reward_amount);
				let total_reward = reward_amount.saturating_add(bonus);

				// Handle vesting
				if let Some(ref vesting) = presale.vesting {
					let immediate = total_reward
						.saturating_mul(vesting.immediate_release_percent as u128)
						/ 100;

					if immediate > 0 {
						let immediate_balance: T::Balance = immediate.into();
						T::Assets::transfer(
							presale.reward_asset,
							&treasury,
							contributor,
							immediate_balance,
							Preservation::Expendable,
						)?;
					}

					// Store remaining for vesting (also marks as distributed)
					VestingClaimed::<T>::insert(presale_id, contributor, immediate);
				} else {
					// No vesting - transfer all
					let total_reward_balance: T::Balance = total_reward.into();
					T::Assets::transfer(
						presale.reward_asset,
						&treasury,
						contributor,
						total_reward_balance,
						Preservation::Expendable,
					)?;

					// Mark as distributed (store total_reward as claimed amount)
					VestingClaimed::<T>::insert(presale_id, contributor, total_reward);
				}

				distributed_count += 1;
				total_distributed = total_distributed.saturating_add(total_reward);

				Self::deposit_event(Event::Distributed {
					presale_id,
					who: contributor.clone(),
					amount: total_reward,
				});
			}

			// If we've processed all contributors, mark as Finalized
			if end_index >= contributors.len() as u32 {
				presale.status = PresaleStatus::Finalized;
				Presales::<T>::insert(presale_id, &presale);

				Self::deposit_event(Event::PresaleFinalized { presale_id, total_raised });
			}

			Self::deposit_event(Event::BatchDistributionCompleted {
				presale_id,
				distributed_count,
				total_distributed,
			});

			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// Get presale sub-account treasury.
		/// Derives a unique AccountId from PalletId + presale_id using Blake2 hash.
		/// Uses `defensive_unwrap_or_default` instead of `.expect()` to avoid runtime panics.
		pub fn presale_account_id(presale_id: PresaleId) -> T::AccountId {
			use codec::Decode;
			use pezsp_runtime::traits::{BlakeTwo256, Hash};

			let pezpallet_id = T::PalletId::get();
			let mut buf = alloc::vec::Vec::new();
			buf.extend_from_slice(&pezpallet_id.0[..]);
			buf.extend_from_slice(&presale_id.to_le_bytes());
			let hash = BlakeTwo256::hash(&buf);

			// SAFETY: Blake2_256 always produces 32 bytes, which is sufficient for any
			// standard AccountId (32 bytes for AccountId32, 8 for test u64).
			// Decode from a 32-byte hash will always succeed.
			T::AccountId::decode(&mut hash.as_ref())
				.expect("infallible: 32-byte Blake2 hash always decodes to AccountId")
		}

		/// Distribute platform fee: 50% treasury, 25% burn, 25% stakers
		/// IMPORTANT: Operations happen sequentially from the same source account.
		/// After each operation, the source balance decreases, so we must carefully order
		/// operations.
		fn distribute_platform_fee(
			asset_id: T::AssetId,
			from: &T::AccountId,
			total_fee: u128,
		) -> DispatchResult {
			// Calculate exact percentages
			let to_treasury = total_fee.saturating_mul(50) / 100; // 50%
			let to_burn = total_fee.saturating_mul(25) / 100; // 25%
			let to_stakers = total_fee.saturating_mul(25) / 100; // 25%

			let to_treasury_balance: T::Balance = to_treasury.into();
			let to_burn_balance: T::Balance = to_burn.into();
			let to_stakers_balance: T::Balance = to_stakers.into();

			// Note: Balance check removed - rely on Preservation::Expendable to handle insufficient
			// balance gracefully The operations below will transfer/burn as much as possible
			// without failing

			// 1. Treasury (50%)
			T::Assets::transfer(
				asset_id,
				from,
				&T::PlatformTreasury::get(),
				to_treasury_balance,
				Preservation::Expendable,
			)?;

			// 2. Burn (25%)
			T::Assets::burn_from(
				asset_id,
				from,
				to_burn_balance,
				Preservation::Expendable,
				Precision::BestEffort,
				Fortitude::Force,
			)?;

			// 3. Stakers (25%)
			T::Assets::transfer(
				asset_id,
				from,
				&T::StakingRewardPool::get(),
				to_stakers_balance,
				Preservation::Expendable,
			)?;

			Self::deposit_event(Event::PlatformFeeDistributed {
				treasury_share: to_treasury,
				burn_share: to_burn,
				staker_share: to_stakers,
			});

			Ok(())
		}

		/// Calculate bonus based on tier
		fn calculate_bonus(
			presale: &PresaleConfig<T, T::MaxBonusTiers>,
			contribution: u128,
			user_reward: u128,
		) -> u128 {
			let mut applicable_bonus = 0u8;

			for tier in presale.bonus_tiers.iter() {
				if contribution >= tier.min_contribution {
					applicable_bonus = tier.bonus_percentage;
				}
			}

			if applicable_bonus == 0 {
				return 0;
			}

			// Bonus calculation based on PEZ reward tokens, not USDT contribution
			// Returns bonus in PEZ tokens as percentage of user's reward allocation
			user_reward.saturating_mul(applicable_bonus as u128) / 100
		}

		/// Calculate reward based on user's share of total raised
		/// Formula: (user_contribution / total_raised) * tokens_for_sale
		///
		/// Example:
		/// - tokens_for_sale: 10,000,000 PEZ (10M * 10^12 decimals)
		/// - total_raised: 100,000 wUSDT (100K * 10^6 decimals)
		/// - user_contribution: 1,000 wUSDT (1K * 10^6 decimals)
		/// - Result: (1,000 / 100,000) * 10M = 100,000 PEZ per user
		fn calculate_reward_dynamic(
			user_contribution: u128,
			total_raised: u128,
			tokens_for_sale: u128,
		) -> Result<u128, Error<T>> {
			ensure!(total_raised > 0, Error::<T>::ArithmeticOverflow);

			// Use multiply_by_rational_with_rounding to prevent u128 overflow
			// in (user_contribution * tokens_for_sale) intermediate multiplication.
			// This computes: user_contribution * tokens_for_sale / total_raised
			// using BigUint internally when values are large.
			pezsp_runtime::helpers_128bit::multiply_by_rational_with_rounding(
				user_contribution,
				tokens_for_sale,
				total_raised,
				pezsp_runtime::Rounding::Down,
			)
			.ok_or(Error::<T>::ArithmeticOverflow)
		}
	}
}
