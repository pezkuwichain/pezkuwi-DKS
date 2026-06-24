// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # Referral Pezpallet
//!
//! A pezpallet for managing user referrals and tracking network growth through invitation
//! mechanics.
//!
//! ## Overview
//!
//! The Referral pezpallet implements a referral system that incentivizes user growth by tracking
//! and rewarding users who successfully invite others to complete KYC verification. Referral
//! counts contribute to trust scores and validator eligibility.
//!
//! ## Referral Workflow
//!
//! ### Initiation Phase
//!
//! 1. User A calls `initiate_referral(user_b_account)` to invite User B
//! 2. System creates a pending referral record linking B to A
//! 3. User B must not have been referred by anyone else
//! 4. Self-referral is prevented
//!
//! ### Confirmation Phase
//!
//! 1. User B completes identity registration and KYC application
//! 2. KYC authority approves User B's application
//! 3. `OnKycApproved` hook automatically fires
//! 4. System:
//!    - Converts pending referral to confirmed referral
//!    - Increments User A's referral count
//!    - Records block number of confirmation
//!    - Emits `ReferralConfirmed` event
//!
//! ## Referral Score System
//!
//! The referral count contributes to the trust score calculation in `pezpallet-trust`:
//! - Each successful referral increases the referrer's reputation
//! - Referral count is used by `ReferralScoreProvider` trait
//! - Higher referral counts improve validator pool eligibility
//! - Community validators require active referral participation
//!
//! ## Security Features
//!
//! - **One Referrer Per User**: Each user can only be referred once
//! - **No Self-Referral**: Users cannot refer themselves
//! - **KYC Verification Required**: Referrals only count after KYC approval
//! - **Immutable History**: Confirmed referrals cannot be changed
//! - **Block Number Recording**: Transparent audit trail
//!
//! ## Interface
//!
//! ### User Extrinsics
//!
//! - `initiate_referral(referred)` - Invite a new user to the ecosystem
//!
//! ### Storage
//!
//! - `PendingReferrals` - Invited users awaiting KYC approval (referred → referrer)
//! - `ReferralCount` - Number of successful referrals per user (referrer → count)
//! - `Referrals` - Confirmed referral records with metadata (referred → ReferralInfo)
//!
//! ### Trait Implementations
//!
//! - `OnKycApproved` - Hook called by `pezpallet-identity-kyc` upon KYC approval
//! - `ReferralScoreProvider` - Query interface for trust score calculation
//! - `InviterProvider` - Query who referred a specific user
//!
//! ## Integration Points
//!
//! ### With pezpallet-identity-kyc
//! - Listens for KYC approval events via `OnKycApproved` hook
//! - Automatically confirms pending referrals upon approval
//!
//! ### With pezpallet-trust
//! - Provides referral scores for composite trust calculation
//! - Contributes to overall reputation metrics
//!
//! ### With pezpallet-validator-pool
//! - Community validator category requires referral participation
//! - Referral count affects pool eligibility
//!
//! ## Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_referral::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type WeightInfo = pezpallet_referral::weights::BizinikiwiWeight<Runtime>;
//! }
//!
//! // Configure pezpallet-identity-kyc to notify referral pezpallet
//! impl pezpallet_identity_kyc::Config for Runtime {
//!     // ...
//!     type OnKycApproved = Referral; // Hook referral confirmation
//! }
//! ```

pub use pezpallet::*;
#[cfg(test)]
mod mock;
pub mod types; // Adding our new types module
pub mod weights;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
extern crate alloc;
use crate::weights::WeightInfo;

/// Trait for notifying trust score system when referral score changes.
/// Defined locally to avoid cyclic dependency with pezpallet-trust.
pub trait TrustScoreUpdater<AccountId> {
	fn on_score_component_changed(who: &AccountId);
}

/// Noop implementation for mock environments and pallets that don't need trust updates.
impl<AccountId> TrustScoreUpdater<AccountId> for () {
	fn on_score_component_changed(_who: &AccountId) {}
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use crate::types::{InviterProvider, RawScore, ReferralScoreProvider, ReferrerStats};
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	use pezpallet_identity_kyc::types::{KycStatus, OnCitizenshipRevoked, OnKycApproved};

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config:
		pezframe_system::Config<RuntimeEvent: From<Event<Self>>>
		+ pezpallet_identity_kyc::Config
		+ TypeInfo
	{
		type WeightInfo: weights::WeightInfo;

		/// Default referrer account - used when no referrer is specified
		/// This allows automatic assignment of founder as referrer for users without invitations
		type DefaultReferrer: Get<Self::AccountId>;

		/// Penalty score per revoked referral
		/// DIRECT RESPONSIBILITY: Bad referrals reduce referrer's score
		/// Default: 3 (each bad referral costs 3x a good referral)
		#[pezpallet::constant]
		type PenaltyPerRevocation: Get<u32>;

		/// Trust score updater - notifies trust pallet when referral score changes
		type TrustScoreUpdater: TrustScoreUpdater<Self::AccountId>;
	}

	// --- Storage Items ---

	/// Holds users awaiting to join system via referral.
	/// (Referred AccountId -> Referrer AccountId)
	#[pezpallet::storage]
	#[pezpallet::getter(fn pending_referrals)]
	pub type PendingReferrals<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

	/// Holds successfully completed referral count per user.
	/// (Referrer AccountId -> Count)
	#[pezpallet::storage]
	#[pezpallet::getter(fn referral_count)]
	pub type ReferralCount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// Holds who a user invited and transaction details.
	/// (Referred AccountId -> ReferralInfo)
	#[pezpallet::storage]
	#[pezpallet::getter(fn referrals)]
	pub type Referrals<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, ReferralInfo<T>, OptionQuery>;

	/// Referrer statistics for direct responsibility tracking
	/// ACCOUNTABILITY: Tracks good and bad referrals for penalty calculation
	#[pezpallet::storage]
	#[pezpallet::getter(fn referrer_stats)]
	pub type ReferrerStatsStorage<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, ReferrerStats, ValueQuery>;

	#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct ReferralInfo<T: Config> {
		pub referrer: T::AccountId,
		pub created_at: BlockNumberFor<T>,
	}

	// --- Events ---
	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// When a user invites another user.
		ReferralInitiated { referrer: T::AccountId, referred: T::AccountId },
		/// When invited user successfully completes KYC process.
		ReferralConfirmed {
			referrer: T::AccountId,
			referred: T::AccountId,
			new_referrer_count: u32,
		},
		/// When a referral is penalized due to revoked citizenship
		/// DIRECT RESPONSIBILITY: Only the referrer is affected
		ReferralPenalized {
			referrer: T::AccountId,
			revoked_citizen: T::AccountId,
			new_penalty_score: u32,
			total_revoked: u32,
		},
	}

	// --- Errors ---
	#[pezpallet::error]
	pub enum Error<T> {
		/// A user cannot invite themselves.
		SelfReferral,
		/// This user has already been invited by someone else.
		AlreadyReferred,
	}

	// --- Extrinsics ---
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Initiates a referral record to invite another user to the system.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(<T as Config>::WeightInfo::initiate_referral())]
		pub fn initiate_referral(origin: OriginFor<T>, referred: T::AccountId) -> DispatchResult {
			let referrer = ensure_signed(origin)?;
			ensure!(referrer != referred, Error::<T>::SelfReferral);
			ensure!(!Referrals::<T>::contains_key(&referred), Error::<T>::AlreadyReferred);
			ensure!(!PendingReferrals::<T>::contains_key(&referred), Error::<T>::AlreadyReferred);

			PendingReferrals::<T>::insert(&referred, &referrer);
			Self::deposit_event(Event::ReferralInitiated { referrer, referred });
			Ok(())
		}

		/// Sudo-only extrinsic to manually confirm a referral (for fixing historical data).
		/// This bypasses the normal KYC approval flow and directly confirms the referral.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(<T as Config>::WeightInfo::force_confirm_referral())]
		pub fn force_confirm_referral(
			origin: OriginFor<T>,
			referrer: T::AccountId,
			referred: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(referrer != referred, Error::<T>::SelfReferral);
			ensure!(!Referrals::<T>::contains_key(&referred), Error::<T>::AlreadyReferred);

			// Increment referrer's count
			let new_count = ReferralCount::<T>::get(&referrer).saturating_add(1);
			ReferralCount::<T>::insert(&referrer, new_count);

			// Update referrer stats for direct responsibility tracking
			ReferrerStatsStorage::<T>::mutate(&referrer, |stats| {
				stats.total_referrals = stats.total_referrals.saturating_add(1);
			});

			// Create and store referral info
			let referral_info = ReferralInfo {
				referrer: referrer.clone(),
				created_at: pezframe_system::Pezpallet::<T>::block_number(),
			};
			Referrals::<T>::insert(referred.clone(), referral_info);

			// Remove from pending if it exists
			PendingReferrals::<T>::remove(&referred);

			// Emit event
			Self::deposit_event(Event::ReferralConfirmed {
				referrer: referrer.clone(),
				referred,
				new_referrer_count: new_count,
			});

			// Notify trust pallet that referrer's score component changed
			T::TrustScoreUpdater::on_score_component_changed(&referrer);

			Ok(())
		}
	}

	// --- Trait Implementations ---

	impl<T: Config> OnKycApproved<T::AccountId> for Pezpallet<T> {
		fn on_kyc_approved(who: &T::AccountId, referrer: &T::AccountId) {
			// Security check: Verify on-chain that the user's KYC status is actually
			// "Approved" before confirming the referral.
			if pezpallet_identity_kyc::Pezpallet::<T>::get_kyc_status(who)
				== pezpallet_identity_kyc::types::KycLevel::Approved
			{
				// Check if this referral already exists (prevent double-counting)
				if Referrals::<T>::contains_key(who) {
					return; // Already processed
				}

				// UPDATED (Gemini suggestion): Use referrer from parameter directly
				// This ensures data consistency between identity-kyc and referral pallets
				// Previously we looked up from storage which could cause data loss

				// Clean up legacy PendingReferrals if exists
				PendingReferrals::<T>::remove(who);

				// Increment referrer's count
				let new_count = ReferralCount::<T>::get(referrer).saturating_add(1);
				ReferralCount::<T>::insert(referrer, new_count);

				// Update referrer stats for direct responsibility tracking
				ReferrerStatsStorage::<T>::mutate(referrer, |stats| {
					stats.total_referrals = stats.total_referrals.saturating_add(1);
				});

				// Create and store referral info
				let referral_info = ReferralInfo {
					referrer: referrer.clone(),
					created_at: pezframe_system::Pezpallet::<T>::block_number(),
				};
				Referrals::<T>::insert(who.clone(), referral_info);

				// Emit confirmation event
				Self::deposit_event(Event::ReferralConfirmed {
					referrer: referrer.clone(),
					referred: who.clone(),
					new_referrer_count: new_count,
				});

				// Notify trust pallet that referrer's score component changed
				T::TrustScoreUpdater::on_score_component_changed(referrer);
			}
		}
	}

	/// Implementation for direct responsibility penalty system
	/// Called when a citizen's status is revoked (malicious actor identified)
	impl<T: Config> OnCitizenshipRevoked<T::AccountId> for Pezpallet<T> {
		fn on_citizenship_revoked(who: &T::AccountId) {
			// Find the referrer of the revoked citizen
			if let Some(referral_info) = Referrals::<T>::get(who) {
				let referrer = referral_info.referrer;
				let penalty_per_revocation = T::PenaltyPerRevocation::get();

				// Update referrer stats - DIRECT RESPONSIBILITY
				// Only the direct referrer is penalized, not the chain
				ReferrerStatsStorage::<T>::mutate(&referrer, |stats| {
					stats.revoked_referrals = stats.revoked_referrals.saturating_add(1);
					stats.penalty_score =
						stats.penalty_score.saturating_add(penalty_per_revocation);
				});

				let updated_stats = ReferrerStatsStorage::<T>::get(&referrer);

				// Emit penalty event
				Self::deposit_event(Event::ReferralPenalized {
					referrer: referrer.clone(),
					revoked_citizen: who.clone(),
					new_penalty_score: updated_stats.penalty_score,
					total_revoked: updated_stats.revoked_referrals,
				});

				// Notify trust pallet that referrer's score component changed
				T::TrustScoreUpdater::on_score_component_changed(&referrer);
			}
		}
	}

	impl<T: Config> ReferralScoreProvider<T::AccountId> for Pezpallet<T> {
		type Score = RawScore;

		fn get_referral_score(who: &T::AccountId) -> RawScore {
			let stats = ReferrerStatsStorage::<T>::get(who);

			// Step 1: "Reverse the unfair ones" - Remove revoked referrals from count
			// This is NOT a penalty, it's correcting the record to reflect reality
			let good_referrals = stats.total_referrals.saturating_sub(stats.revoked_referrals);

			// Step 2: Calculate base score from good referrals
			// Tiered scoring system with max 500 points:
			// 0 referrals = 0 points
			// 1-10 referrals = count * 10 points (10, 20, 30, ..., 100)
			// 11-50 referrals = 100 + ((count - 10) * 5) = 105, 110, ..., 300
			// 51-100 referrals = 300 + ((count - 50) * 4) = 304, 308, ..., 500
			// 101+ referrals = 500 points (maximum)
			let base_score = match good_referrals {
				0 => 0,
				1..=10 => good_referrals * 10,
				11..=50 => 100 + ((good_referrals - 10) * 5),
				51..=100 => 300 + ((good_referrals - 50) * 4),
				_ => 500,
			};

			// Step 3: "Punishment" - Apply stored penalty from PenaltyPerRevocation
			// Uses the pre-calculated penalty_score accumulated in on_citizenship_revoked()
			// This is the actual punishment: "you should have been more careful"
			base_score.saturating_sub(stats.penalty_score)
		}
	}

	impl<T: Config> InviterProvider<T::AccountId> for Pezpallet<T> {
		fn get_inviter(who: &T::AccountId) -> Option<T::AccountId> {
			Referrals::<T>::get(who).map(|info| info.referrer)
		}
	}
}
