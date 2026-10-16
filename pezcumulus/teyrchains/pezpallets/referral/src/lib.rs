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
//! - `initiate_referral(referred)` - Claim to have brought a newcomer to the state
//! - `withdraw_referral(referred)` - Take back a claim of your own
//!
//! ### Storage
//!
//! - `Invitations` - Open claims of having brought somebody (referred, claimant → ())
//! - `CleanupQueue` / `CleanupHead` / `CleanupTail` - Citizens whose leftover claims the chain
//!   clears with spare block weight
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
//! ### With pezpallet-tnpos
//! - Referral score is one of the components trust is computed from, and trust gates six of
//!   the nine strata a validator committee is drawn from
//! - It is not a gate of its own: there is no stratum an account enters by referrals alone
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

// Re-exported so the generated runtime weights can name it.
//
// `pezframe-omni-bencher` writes `impl<T> pezpallet_{name}::WeightInfo for WeightInfo<T>` -- the
// crate root, never the module. Without this line the generated file does not compile and the
// only way to use it was to edit the path by hand after every regeneration, which is a patch
// that silently disappears the next time the weights are taken. Measured 2026-09-21.
pub use weights::WeightInfo;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
extern crate alloc;

/// How many claim rows one step of the idle-time cleanup removes.
///
/// The bound that makes the cleanup safe to run at all: one step costs at most this many rows
/// whatever an attacker piled on an account, so a step is either affordable in the weight a
/// block has left or not taken.
pub const CLAIMS_PER_STEP: u32 = 64;

/// The ceiling of the tiered referral score: what somebody who has brought in a hundred or
/// more citizens is worth. Named because it is both the top tier and the maximum this
/// component reports for weighting.
pub const MAX_REFERRAL_SCORE: u32 = 500;

/// The tiered score a referrer reaches with `good_referrals` vouches, before penalties.
///
/// Public because the weighting has to read it. A trust score is a weighted sum, and the
/// weight a source declares is only true if the source is normalised by the score actually
/// reachable -- which is this table evaluated at the current vouching ceiling, not
/// `MAX_REFERRAL_SCORE`. The table's own top needs a hundred vouches; a lifetime ceiling of
/// fifty can never get there.
pub fn score_for(good_referrals: u32) -> u32 {
	// 0 vouches = 0 · 1-10 = 10 each · 11-50 = 5 each · 51-100 = 4 each · 101+ = the ceiling.
	// The first vouches are worth the most: the score rewards starting a subtree, not running
	// a farm.
	match good_referrals {
		0 => 0,
		1..=10 => good_referrals * 10,
		11..=50 => 100 + ((good_referrals - 10) * 5),
		51..=100 => 300 + ((good_referrals - 50) * 4),
		_ => MAX_REFERRAL_SCORE,
	}
}

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
	use pezframe_support::{pezpallet_prelude::*, weights::WeightMeter};
	use pezframe_system::pezpallet_prelude::*;
	use pezpallet_identity_kyc::types::{KycStatus, OnCitizenshipRevoked, OnKycApproved};

	/// The version this pallet's storage layout is at.
	///
	/// Declared so that the first migration has a baseline to compare against. Without it the
	/// in-code and on-chain versions are both an implicit zero, and a migration cannot tell a
	/// chain that has never been migrated from one that has been migrated to zero.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		/// Clear the claims left about citizens, with whatever weight the block has spare.
		///
		/// Settlement removes only the claim it settles, so that approving a citizen costs the
		/// same however many strangers claimed them. The rest are inert from that moment -- no
		/// new claim about a citizen is accepted and none of the old ones can change anything
		/// -- and they are cleared here, a bounded step at a time, in the order the citizens
		/// were approved.
		fn on_idle(_n: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			Self::clean_up_claims(remaining_weight)
		}

		/// Three records of the same referrals have to agree.
		///
		/// `Referrals` says who vouched for each citizen, `ReferralCount` says how many each
		/// account vouched for, and `ReferrerStats` says the same again alongside the
		/// revocations. They are written together on one path and read separately by three
		/// different things -- the trust score, the earned roles, the penalty. A count that
		/// drifts from the records does not fail anywhere; it just quietly pays somebody for
		/// referrals they did not make.
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use alloc::collections::BTreeMap;
			use pezframe_support::ensure;

			let mut counted: BTreeMap<T::AccountId, u32> = BTreeMap::new();
			for (_, info) in Referrals::<T>::iter() {
				*counted.entry(info.referrer).or_default() += 1;
			}

			for (referrer, count) in ReferralCount::<T>::iter() {
				ensure!(
					counted.get(&referrer).copied().unwrap_or(0) == count,
					"the referral count does not match the referral records"
				);
			}

			for (referrer, stats) in ReferrerStatsStorage::<T>::iter() {
				ensure!(
					stats.total_referrals == counted.get(&referrer).copied().unwrap_or(0),
					"a referrer's statistics do not match the referral records"
				);
				ensure!(
					stats.revoked_referrals <= stats.total_referrals,
					"more of a referrer's citizens were revoked than they ever referred"
				);
			}

			// An invitation is settled by two statements, and only a settled one is counted.
			let mut invitations: BTreeMap<T::AccountId, u32> = BTreeMap::new();
			for (_, inviter) in InvitedBy::<T>::iter() {
				*invitations.entry(inviter).or_default() += 1;
			}
			for (inviter, count) in InvitationCount::<T>::iter() {
				ensure!(
					invitations.get(&inviter).copied().unwrap_or(0) == count,
					"the invitation count does not match who was actually invited"
				);
			}

			// The cleanup queue is a window `[head, tail)`; the head never passes the tail.
			ensure!(
				CleanupHead::<T>::get() <= CleanupTail::<T>::get(),
				"the claim cleanup queue's head has passed its tail"
			);

			Ok(())
		}
	}

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

		/// How many people a citizen may vouch for before having vouched for anyone.
		///
		/// Small on purpose. This is the width of the tree at its newest edge, and a forger
		/// buys accounts far more cheaply than they buy a record.
		#[pezpallet::constant]
		type InitialVouchingCapacity: Get<u32>;

		/// How many settled referrals buy one more place.
		///
		/// Capacity is earned rather than granted: bring one person in and stand behind them,
		/// and after this many you may bring another. A revoked referral is subtracted, so the
		/// account that vouches carelessly loses the room to do it again.
		#[pezpallet::constant]
		type SettledVouchesPerPlace: Get<u32>;

		/// The most anyone may ever have brought in.
		#[pezpallet::constant]
		type MaxVouchingCapacity: Get<u32>;

		/// How many revoked referrals before a record is judged at all.
		///
		/// A floor, because a rate on small numbers says nothing: one revoked out of one is a
		/// hundred per cent and also a single unlucky guarantee.
		#[pezpallet::constant]
		type SuspensionRevocationFloor: Get<u32>;

		/// The share of a voucher's referrals that may be revoked before they stop vouching.
		///
		/// Reached, and this account brings nobody else in until the court says otherwise.
		/// Not a punishment -- the penalty to standing is that -- but a stop: whatever is
		/// producing forgeries here, it does not get to keep producing them while the question
		/// is being answered.
		#[pezpallet::constant]
		type SuspensionRevocationPercent: Get<u32>;

		/// Trust score updater - notifies trust pallet when referral score changes
		type TrustScoreUpdater: TrustScoreUpdater<Self::AccountId>;

		/// How an earned role is awarded, once the evidence for it exists here.
		///
		/// `Serokê Komelê` and `Moderatorê Civakê` are roles for people who have built
		/// something in the community, and the count of who they brought in is the evidence.
		/// Before this the whole `Earned` category was granted through `EnsureRoot` by a call
		/// nobody made, so nobody had ever earned one.
		type EarnedRoles: pezpallet_tiki::EarnedRoleGranter<Self::AccountId, pezpallet_tiki::Tiki>;

		/// Referrals needed for `Serokê Komelê`.
		///
		/// The number is a policy of the state, not a property of the code, which is why it
		/// is a constant a chain spec sets rather than a literal in here.
		#[pezpallet::constant]
		type AssociationHeadThreshold: Get<u32>;

		/// Referrals needed for `Moderatorê Civakê`.
		#[pezpallet::constant]
		type CommunityModeratorThreshold: Get<u32>;
	}

	// --- Storage Items ---

	/// Claims of the form "I brought this person here", one row per claimant.
	///
	/// A claim on its own means nothing and blocks nothing. It becomes an invitation only when
	/// the person it is about names the same account in their application -- two statements,
	/// from the two people who would know.
	///
	/// This replaced a single-entry map that the first caller could fill for any address in
	/// existence, which nobody could clear and which stopped the person who had actually
	/// invited them from recording it. Since a claim carries no weight until it is confirmed,
	/// letting several stand costs nothing and takes the race away.
	///
	/// How many may stand is not bounded -- each costs its claimant a transaction fee -- so
	/// nothing that has to fit in a block walks them. A claim lives until the person it is
	/// about is approved, and is inert from then on: no new claim about a citizen is accepted,
	/// the one the newcomer named is removed at settlement, and the rest are removed by the
	/// chain's idle-time cleanup (`CleanupQueue`) or by their claimant (`withdraw_referral`).
	#[pezpallet::storage]
	#[pezpallet::getter(fn invitation_claim)]
	pub type Invitations<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId, // the person invited
		Blake2_128Concat,
		T::AccountId, // the one claiming to have invited them
		(),
		OptionQuery,
	>;

	/// Citizens whose leftover claims are still to be cleared, oldest approval first.
	///
	/// A queue rather than a walk of `Invitations`, because the claims are keyed by the person
	/// they are about and the cleanup needs to know which prefixes still hold rows without
	/// reading the map. Indexed by position so that a push and a pop are one write each,
	/// whatever the length. The account stored is the one the rows are keyed by: a citizenship
	/// reissued afterwards does not move the rows, so they are cleared under the old account.
	#[pezpallet::storage]
	pub type CleanupQueue<T: Config> = StorageMap<_, Twox64Concat, u64, T::AccountId, OptionQuery>;

	/// The position of the oldest entry in `CleanupQueue`; equal to `CleanupTail` when empty.
	#[pezpallet::storage]
	pub type CleanupHead<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// The position the next entry in `CleanupQueue` is written to.
	#[pezpallet::storage]
	pub type CleanupTail<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Who actually brought each citizen in, once both sides have said so.
	///
	/// Kept for good, unlike the claim it settles. This is a different fact from who vouched:
	/// somebody can bring a hundred people to the state and be named as guarantor by none of
	/// them, because each of them asked a parent or a friend to stand for them instead. That
	/// person has grown the country and taken nothing from it, and there is no way to see it
	/// except by keeping this separately from the referral count.
	#[pezpallet::storage]
	#[pezpallet::getter(fn invited_by)]
	pub type InvitedBy<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

	/// How many citizens each account brought in.
	///
	/// Deliberately not part of the trust score. It is a record of contribution, not a claim
	/// on reward -- what makes it worth reading is precisely that it can be high while the
	/// referral count beside it is low.
	#[pezpallet::storage]
	#[pezpallet::getter(fn invitation_count)]
	pub type InvitationCount<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

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

	#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
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
		/// Both sides agreed on who brought this citizen in.
		InvitationSettled { inviter: T::AccountId, invited: T::AccountId },
		/// When a referral is penalized due to revoked citizenship
		/// DIRECT RESPONSIBILITY: Only the referrer is affected
		ReferralPenalized {
			referrer: T::AccountId,
			revoked_citizen: T::AccountId,
			new_penalty_score: u32,
			total_revoked: u32,
		},
		/// A claimant took back their claim of having brought somebody here.
		ReferralWithdrawn { referrer: T::AccountId, referred: T::AccountId },
	}

	// --- Errors ---
	#[pezpallet::error]
	pub enum Error<T> {
		/// A user cannot invite themselves.
		SelfReferral,
		/// This user has already been invited by someone else.
		AlreadyReferred,
		/// This account is already a citizen; a claim about them can no longer change anything.
		AlreadyCitizen,
		/// There is no claim by you about this account to withdraw.
		NoSuchClaim,
	}

	// --- Extrinsics ---
	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Record that you brought someone to the state.
		///
		/// A claim, not a fact: it takes effect only if that person names you as their inviter
		/// when they apply. So it does not need to be exclusive and is not -- several people
		/// may claim the same newcomer, and the newcomer settles it. The version this replaced
		/// let whoever called first hold the only slot for any address at all, which meant a
		/// stranger could take the place of the person who had actually done the inviting.
		///
		/// Claims are not limited in number; each one costs its fee. What is refused is a claim
		/// that could not matter: about an account whose invitation has settled, or about an
		/// account that is already a citizen. Settlement happens once, at approval, so a claim
		/// made after it would only be a row for the chain to clear.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(<T as Config>::WeightInfo::initiate_referral())]
		pub fn initiate_referral(origin: OriginFor<T>, referred: T::AccountId) -> DispatchResult {
			let inviter = ensure_signed(origin)?;
			ensure!(inviter != referred, Error::<T>::SelfReferral);
			ensure!(InvitedBy::<T>::get(&referred).is_none(), Error::<T>::AlreadyReferred);
			ensure!(
				pezpallet_identity_kyc::Pezpallet::<T>::get_kyc_status(&referred)
					!= pezpallet_identity_kyc::types::KycLevel::Approved,
				Error::<T>::AlreadyCitizen
			);

			Invitations::<T>::insert(&referred, &inviter, ());
			Self::deposit_event(Event::ReferralInitiated { referrer: inviter, referred });
			Ok(())
		}

		// `force_confirm_referral` used to live here: a root call that invented a confirmed
		// referral for two accounts, with no KYC, no citizenship and no application, and
		// incremented the count. It was there to repair historical data. Both chains start
		// again from genesis, so there is no history to repair -- and a call that can write a
		// referral out of nothing is a call that can write a trust score out of nothing.
		// Its index, 1, is left unused rather than given to a different call.

		/// Take back your own claim of having brought somebody here.
		///
		/// A claim is the claimant's statement, so the claimant may withdraw it -- before the
		/// newcomer is approved, to stop it being settled, or after, to clear a row that no
		/// longer means anything without waiting for the chain to get to it. Only your own:
		/// the row is addressed by the caller, so there is no way to name somebody else's.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(<T as Config>::WeightInfo::withdraw_referral())]
		pub fn withdraw_referral(origin: OriginFor<T>, referred: T::AccountId) -> DispatchResult {
			let referrer = ensure_signed(origin)?;
			ensure!(Invitations::<T>::contains_key(&referred, &referrer), Error::<T>::NoSuchClaim);

			Invitations::<T>::remove(&referred, &referrer);
			Self::deposit_event(Event::ReferralWithdrawn { referrer, referred });
			Ok(())
		}
	}

	// --- Trait Implementations ---

	impl<T: Config> OnKycApproved<T::AccountId> for Pezpallet<T> {
		fn on_kyc_approved(
			who: &T::AccountId,
			referrer: &T::AccountId,
			inviter: Option<&T::AccountId>,
		) {
			// Security check: Verify on-chain that the user's KYC status is actually
			// "Approved" before confirming the referral.
			if pezpallet_identity_kyc::Pezpallet::<T>::get_kyc_status(who)
				== pezpallet_identity_kyc::types::KycLevel::Approved
			{
				// Check if this referral already exists (prevent double-counting)
				if Referrals::<T>::contains_key(who) {
					// Already processed. A citizenship renounced and applied for again passes
					// through here a second time, and claims made while the account was not a
					// citizen are as inert now as the first time; queue them the same way.
					Self::queue_leftover_claims(who);
					return;
				}

				// UPDATED (Gemini suggestion): Use referrer from parameter directly
				// This ensures data consistency between identity-kyc and referral pallets
				// Previously we looked up from storage which could cause data loss

				// Settle the invitation, if the two sides agree on it. The new citizen named
				// somebody; that account has to have claimed them. Either statement alone
				// proves nothing -- one is a person crediting whoever they like, the other is
				// a stranger crediting themselves.
				//
				// Nothing here walks the claims. Their number is set by whoever chose to pay
				// for them, and a walk over them here would let a stranger price somebody's
				// approval out of a block. The settled claim is removed at once -- its fact
				// now lives in `InvitedBy` -- and the rest, inert from this moment, are queued
				// for the idle-time cleanup.
				if let Some(inviter) = inviter {
					if Invitations::<T>::take(who, inviter).is_some() {
						InvitedBy::<T>::insert(who, inviter);
						InvitationCount::<T>::mutate(inviter, |n| *n = n.saturating_add(1));
						Self::deposit_event(Event::InvitationSettled {
							inviter: inviter.clone(),
							invited: who.clone(),
						});
					}
				}
				Self::queue_leftover_claims(who);

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
				Self::award_earned_roles(referrer);
			}
		}
	}

	/// Implementation for direct responsibility penalty system
	/// Called when a citizen's status is revoked (malicious actor identified)
	impl<T: Config> OnCitizenshipRevoked<T::AccountId> for Pezpallet<T> {
		fn on_citizenship_revoked(who: &T::AccountId) {
			// Whoever stood for this citizen carries what follows. Standing for someone is
			// saying you believe in them; the cost of a guarantee belongs to the guarantor.
			//
			// One exception, and it is structural rather than moral. The founder approves
			// applications nobody else answered, after the wait -- not because they know the
			// person but because somebody has to keep the queue moving. Charging that to the
			// founder would mean the account doing the state a service is the account whose
			// standing it destroys, and it would do so with certainty: the referral score is
			// capped at five hundred, which the founder reaches almost at once, while the
			// penalty has no ceiling at all. So in that case it falls to whoever brought the
			// person here -- the one claim about them that two people did make. If nobody
			// invited them either, nobody vouched for them in any sense, and nobody pays.
			if let Some(referral_info) = Referrals::<T>::get(who) {
				let stood_in = pezpallet_identity_kyc::ApprovedByFallback::<T>::contains_key(who);
				let named = if stood_in {
					match InvitedBy::<T>::get(who) {
						Some(inviter) => inviter,
						None => return,
					}
				} else {
					referral_info.referrer
				};
				// Who was named is history and stays as written; who pays is whoever answers
				// for that account today. A citizenship reissued after a lost key moves the
				// standing to the new account, and a penalty addressed to the retired one
				// would let the successor keep the score without the liability.
				let referrer =
					pezpallet_identity_kyc::Pezpallet::<T>::account_answering_for(&named);
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

	/// Undo what a revocation charged the voucher.
	impl<T: Config> pezpallet_identity_kyc::types::OnCitizenshipRestored<T::AccountId>
		for Pezpallet<T>
	{
		fn on_citizenship_restored(who: &T::AccountId) {
			if let Some(referral_info) = Referrals::<T>::get(who) {
				let stood_in = pezpallet_identity_kyc::ApprovedByFallback::<T>::contains_key(who);
				let named = if stood_in {
					match InvitedBy::<T>::get(who) {
						Some(inviter) => inviter,
						None => return,
					}
				} else {
					referral_info.referrer
				};
				// Who was named is history and stays as written; who pays is whoever answers
				// for that account today. A citizenship reissued after a lost key moves the
				// standing to the new account, and a penalty addressed to the retired one
				// would let the successor keep the score without the liability.
				let referrer =
					pezpallet_identity_kyc::Pezpallet::<T>::account_answering_for(&named);
				let penalty_per_revocation = T::PenaltyPerRevocation::get();
				ReferrerStatsStorage::<T>::mutate(&referrer, |stats| {
					stats.revoked_referrals = stats.revoked_referrals.saturating_sub(1);
					stats.penalty_score =
						stats.penalty_score.saturating_sub(penalty_per_revocation);
				});
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
			let base_score = score_for(good_referrals);

			// Step 3: "Punishment" - Apply stored penalty from PenaltyPerRevocation
			// Uses the pre-calculated penalty_score accumulated in on_citizenship_revoked()
			// This is the actual punishment: "you should have been more careful"
			base_score.saturating_sub(stats.penalty_score)
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// Queue `who` for the idle-time cleanup if any claim about them is still standing.
		///
		/// One read to find out, whatever the number of claims: the first key under the prefix,
		/// not a count.
		pub(crate) fn queue_leftover_claims(who: &T::AccountId) {
			if Invitations::<T>::iter_key_prefix(who).next().is_some() {
				let tail = CleanupTail::<T>::get();
				CleanupQueue::<T>::insert(tail, who);
				CleanupTail::<T>::put(tail.saturating_add(1));
			}
		}

		/// Work through `CleanupQueue` within `limit`, and report what it cost.
		///
		/// A step clears at most `CLAIMS_PER_STEP` rows under the account at the head, and is
		/// taken only if its worst case fits in what is left -- so the weight reported never
		/// exceeds `limit`. The head is popped once its prefix is empty.
		///
		/// A step that leaves rows behind ends the call. The removal underneath ignores the
		/// cursor it is given (`unhashed::clear_prefix` hands back the prefix itself as one),
		/// and rows removed earlier in the block are still visible to the backend iterator, so a
		/// second removal under the same prefix in the same block would walk the same rows again
		/// and never reach the end. Once per prefix per block is the only progress there is; the
		/// next block starts from the backend, where those rows are gone. Every pass of the loop
		/// therefore either pops the head or returns, so it ends within the queue's length
		/// whatever the weight given.
		pub(crate) fn clean_up_claims(limit: Weight) -> Weight {
			let mut meter = WeightMeter::with_limit(limit);
			// Whether there is anything to do at all: the two ends of the queue.
			if meter.try_consume(T::DbWeight::get().reads(2)).is_err() {
				return Weight::zero();
			}
			let mut head = CleanupHead::<T>::get();
			let tail = CleanupTail::<T>::get();
			let worst_step = <T as Config>::WeightInfo::clean_claims(CLAIMS_PER_STEP);

			while head < tail && meter.can_consume(worst_step) {
				let (removed, finished) = match CleanupQueue::<T>::get(head) {
					Some(who) => {
						let result = Invitations::<T>::clear_prefix(&who, CLAIMS_PER_STEP, None);
						(result.loops.max(result.unique), result.maybe_cursor.is_none())
					},
					// Cannot happen while every position below the tail was written by a push;
					// treated as an empty prefix so the queue still moves past it.
					None => (0, true),
				};
				meter
					.consume(<T as Config>::WeightInfo::clean_claims(removed.min(CLAIMS_PER_STEP)));
				if !finished {
					break;
				}
				CleanupQueue::<T>::remove(head);
				head = head.saturating_add(1);
				CleanupHead::<T>::put(head);
			}
			meter.consumed()
		}

		/// Award the roles this account's record now entitles them to.
		///
		/// Called after the count changes rather than on a schedule, so a role arrives when it
		/// is earned. Crossing a threshold again is not an error -- the count keeps rising --
		/// and a failure to award does not undo the referral that caused it, which is why the
		/// result is logged rather than propagated.
		fn award_earned_roles(who: &T::AccountId) {
			use pezpallet_tiki::EarnedRoleGranter;

			let stats = ReferrerStatsStorage::<T>::get(who);
			let brought_in = stats.total_referrals.saturating_sub(stats.revoked_referrals);

			for (threshold, tiki) in [
				(T::CommunityModeratorThreshold::get(), pezpallet_tiki::Tiki::ModeratorêCivakê),
				(T::AssociationHeadThreshold::get(), pezpallet_tiki::Tiki::SerokêKomele),
			] {
				if threshold > 0 && brought_in >= threshold {
					if let Err(e) = T::EarnedRoles::grant_earned(who, tiki) {
						log::warn!(
							target: "referral",
							"could not award an earned role to {who:?}: {e:?}"
						);
					}
				}
			}
		}
	}

	impl<T: Config> InviterProvider<T::AccountId> for Pezpallet<T> {
		fn get_inviter(who: &T::AccountId) -> Option<T::AccountId> {
			Referrals::<T>::get(who).map(|info| info.referrer)
		}
	}
}

use pezframe_support::traits::Get as _;

impl<T: Config> Pezpallet<T> {
	/// Move this pallet's record of one account onto another, for a court-ordered reissue.
	///
	/// Only what is keyed *by* the account moves. Records that merely name it -- who invited
	/// whom -- are left as written, because they are statements about what happened and the
	/// account that did it is the account that did it. Nothing is lost by leaving them: every
	/// path that acts on a named referrer resolves through `account_answering_for` first.
	///
	/// `Invitations` does not move. A claim only matters until the person it is about is
	/// approved, and a reissue is for somebody who already is a citizen -- so every claim
	/// keyed by `from` is already inert, and moving them would be a walk of a set whose size a
	/// stranger chooses. Rows left under `from` when it was approved were queued under
	/// `from`, and the idle-time cleanup clears them under that key. A claim naming `from`
	/// as the claimant is resolved at settlement like every other reference.
	pub fn rebind_account(
		from: &T::AccountId,
		to: &T::AccountId,
	) -> pezframe_support::pezpallet_prelude::DispatchResult {
		if let Some(v) = InvitedBy::<T>::take(from) {
			InvitedBy::<T>::insert(to, v);
		}
		let invitations = InvitationCount::<T>::take(from);
		if invitations != 0 {
			InvitationCount::<T>::insert(to, invitations);
		}
		let referrals = ReferralCount::<T>::take(from);
		if referrals != 0 {
			ReferralCount::<T>::insert(to, referrals);
		}
		if let Some(v) = Referrals::<T>::take(from) {
			Referrals::<T>::insert(to, v);
		}
		let stats = ReferrerStatsStorage::<T>::take(from);
		ReferrerStatsStorage::<T>::insert(to, stats);
		Ok(())
	}

	/// How many people this account may still vouch into the register.
	///
	/// `initial + settled / per_place`, capped, minus those already brought in. Settled means
	/// referrals that became citizens and stayed citizens: a revoked one is subtracted, so an
	/// account that vouched for a forgery pays for it in the room it has left rather than only
	/// in its standing.
	pub fn vouching_capacity(who: &T::AccountId) -> u32 {
		let stats = ReferrerStatsStorage::<T>::get(who);

		// A record bad enough stops the account vouching entirely. The tree is public and a
		// manufactured cluster shows as a subtree with an anomalous share of revocations --
		// but a revocation only happens once somebody has noticed, so this is a lagging
		// signal and the right response to a lagging signal is to stop the bleeding rather
		// than to punish. It lifts on its own the moment the court restores what it revoked.
		if stats.revoked_referrals >= T::SuspensionRevocationFloor::get()
			&& stats.revoked_referrals.saturating_mul(100)
				>= stats.total_referrals.saturating_mul(T::SuspensionRevocationPercent::get())
		{
			return 0;
		}

		let settled = stats.total_referrals.saturating_sub(stats.revoked_referrals);
		let earned = settled.checked_div(T::SettledVouchesPerPlace::get().max(1)).unwrap_or(0);
		T::InitialVouchingCapacity::get()
			.saturating_add(earned)
			.min(T::MaxVouchingCapacity::get())
	}

	/// What is left of that capacity.
	pub fn vouching_remaining(who: &T::AccountId) -> u32 {
		Self::vouching_capacity(who).saturating_sub(InvitationCount::<T>::get(who))
	}
}

impl<T: Config> pezpallet_identity_kyc::types::VouchingCapacity<T::AccountId> for Pezpallet<T> {
	fn remaining(who: &T::AccountId) -> Option<u32> {
		Some(Self::vouching_remaining(who))
	}
}
