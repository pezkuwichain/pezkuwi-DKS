// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # Identity & KYC Pezpallet - TRUSTLESS MODEL
//!
//! A privacy-preserving, decentralized citizenship verification system.
//!
//! ## Overview
//!
//! This pezpallet implements a **TRUSTLESS** citizenship verification where:
//! - NO personal data is stored on-chain (only hash)
//! - NO central authority/bot approves applications
//! - Existing citizens vouch for new applicants (referral-based)
//! - Direct responsibility: Referrers are accountable for their referrals
//!
//! ## Security Design (Kurdish People Safety)
//!
//! This system is designed to protect vulnerable populations (like Kurdish people)
//! from hostile regimes that might try to identify applicants:
//! - Only H256 hash of identity stored on-chain
//! - Actual documents stored off-chain (IPFS/encrypted)
//! - No admin can see or leak personal data
//! - Referral chain creates accountability without central authority
//!
//! ## Citizenship Workflow
//!
//! ### 1. Application Phase
//! - User creates identity hash off-chain: `H256(name + email + documents)`
//! - User calls `apply_for_citizenship(identity_hash, referrer_account)`
//! - Referrer MUST be an existing citizen (KycLevel::Approved)
//! - Status changes to `PendingReferral`
//!
//! ### 2. Referrer Approval Phase
//! - Referrer reviews applicant (off-chain verification)
//! - Referrer calls `approve_referral(applicant)` to vouch for them
//! - Status changes to `ReferrerApproved`
//! - Referrer takes personal responsibility for this referral
//!
//! ### 3. Self-Confirmation Phase (Welati NFT Only)
//! - Applicant calls `confirm_citizenship()` to complete the process
//! - Status changes to `Approved`
//! - Citizen NFT (Welati) is minted via self-confirmation
//! - Referral hooks are triggered
//!
//! ## KYC Levels
//!
//! - **NotStarted** - No application submitted
//! - **PendingReferral** - Waiting for referrer approval
//! - **ReferrerApproved** - Referrer approved, ready for self-confirmation
//! - **Approved** - Full citizen with all rights
//! - **Revoked** - Citizenship revoked (governance decision)
//!
//! ## Privacy Features
//!
//! - **Hash-only storage**: No personal data on-chain
//! - **Off-chain documents**: IPFS or encrypted storage
//! - **No admin access**: Decentralized verification
//! - **Referral accountability**: Social trust, not central authority
//!
//! ## Direct Responsibility Model
//!
//! When a citizen is found to be malicious:
//! - ONLY their direct referrer is penalized
//! - Penalty: Trust score reduction + potential citizenship review
//! - Chain reactions are limited to direct relationships
//! - Good referrals from bad actors are NOT penalized
//!
//! ## Interface
//!
//! ### User Extrinsics
//!
//! - `apply_for_citizenship(identity_hash, referrer)` - Submit citizenship application
//! - `confirm_citizenship()` - Self-confirm after referrer approval (Welati only)
//! - `renounce_citizenship()` - Voluntarily give up citizenship
//!
//! ### Referrer Extrinsics
//!
//! - `approve_referral(applicant)` - Vouch for an applicant
//!
//! ### Governance Extrinsics (Root only)
//!
//! - `revoke_citizenship(who)` - Revoke citizenship (governance decision)
//!
//! ## Runtime Integration Example
//!
//! ```ignore
//! impl pezpallet_identity_kyc::Config for Runtime {
//!     type RuntimeEvent = RuntimeEvent;
//!     type Currency = Balances;
//!     type WeightInfo = pezpallet_identity_kyc::weights::BizinikiwiWeight<Runtime>;
//!     type OnKycApproved = Referral;
//!     type CitizenNftProvider = Tiki;
//!     type KycApplicationDeposit = ConstU128<1_000_000_000_000>; // Spam prevention
//!     type MaxStringLength = ConstU32<128>;
//!     type MaxCidLength = ConstU32<64>;
//! }
//! ```

pub use pezpallet::*;
pub mod types;
use types::*;
pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

extern crate alloc;
use pezframe_support::{pezpallet_prelude::*, traits::ReservableCurrency};
use pezframe_system::pezpallet_prelude::*;
use pezsp_core::H256;
use pezsp_runtime::traits::Saturating;
use pezsp_runtime::traits::Zero;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;

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
		/// The maintained citizen count must equal what a walk of `KycStatuses` would give.
		///
		/// A counter that is kept by hand fails in exactly one way: some path changes a status
		/// and does not say so. Nothing about that shows up at the call site -- the count is
		/// simply wrong from then on, and it is wrong in whichever direction lets an election
		/// reach quorum it should not, or a population gate open early. Comparing against the
		/// source of truth is the only way to see it, and doing that here means a test sees it
		/// rather than a chain.
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use pezframe_support::ensure;

			let kept = CitizenCount::<T>::get();
			let counted = Self::count_citizens_by_scan();
			ensure!(kept == counted, "CitizenCount does not match the number of Approved records");

			// The identity hash is what makes one person one citizen. If the forward and
			// reverse maps disagree, the uniqueness they exist to enforce is gone: two
			// accounts could hold the same hash with only one of them visible from either
			// side, which is the whole of the sybil defence.
			for (account, hash) in IdentityHashes::<T>::iter() {
				ensure!(
					IdentityHashToAccount::<T>::get(hash).as_ref() == Some(&account),
					"an identity hash does not point back at the account that holds it"
				);
			}

			// Honorary citizens are citizens; the register is only a note about how they came
			// in. So every name in it has to be someone this pallet considers a citizen, and
			// the count has to match the register -- the treasury reads one of these numbers.
			let mut honorary = 0u32;
			for (account, _) in HonoraryCitizens::<T>::iter() {
				ensure!(
					KycStatuses::<T>::get(&account) == KycLevel::Approved,
					"somebody is in the honorary register without being a citizen"
				);
				honorary = honorary.saturating_add(1);
			}
			ensure!(
				HonoraryCitizenCount::<T>::get() == honorary,
				"the honorary count does not match the honorary register"
			);
			ensure!(
				HonoraryCitizenCount::<T>::get() <= kept,
				"more citizens were named than there are citizens"
			);

			Ok(())
		}
	}

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>> {
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Origin that can revoke citizenship (governance/root)
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type WeightInfo: WeightInfo;

		/// Default referrer account (founder) - used when no valid referrer is provided
		type DefaultReferrer: Get<Self::AccountId>;

		/// Hook called when citizenship is approved - used by referral pezpallet
		type OnKycApproved: crate::types::OnKycApproved<Self::AccountId>;

		/// Hook called when citizenship is revoked - used by referral pezpallet for penalty
		type OnCitizenshipRevoked: crate::types::OnCitizenshipRevoked<Self::AccountId>;

		/// Provider for minting citizen NFTs - used by tiki pezpallet
		type CitizenNftProvider: crate::types::CitizenNftProvider<Self::AccountId>;

		/// How long an applicant waits on their referrer before the founder may step in.
		///
		/// Citizenship needs a human to vouch for it -- that is what keeps the register from
		/// filling with accounts nobody has met. But a referrer who never gets round to it
		/// would otherwise leave the applicant waiting for ever, and the answer cannot be to
		/// drop the application: that punishes the applicant for somebody else's silence.
		/// After this long the founder may approve instead, and becomes the referrer of
		/// record -- including for the accountability that follows a referral.
		#[pezpallet::constant]
		type ReferralFallbackPeriod: Get<BlockNumberFor<Self>>;

		/// How long a new citizen waits before they may vouch for anyone.
		///
		/// Vouching is the only thing standing between the register and a manufactured
		/// population: there is no authority in this path, only a citizen saying "I know this
		/// person". A chain of that kind grows as fast as its newest member can vouch, so the
		/// newest member waits. The delay is what a forger cannot buy -- they can afford the
		/// deposits and the accounts, and they cannot afford the calendar.
		#[pezpallet::constant]
		type VouchingWaitingPeriod: Get<BlockNumberFor<Self>>;

		/// How many more citizens an account may bring in, asked of whoever counts them.
		type VouchingCapacity: crate::types::VouchingCapacity<Self::AccountId>;

		/// Deposit required to apply (spam prevention, returned on approval)
		#[pezpallet::constant]
		type KycApplicationDeposit: Get<BalanceOf<Self>>;

		/// Max string length for legacy storage
		#[pezpallet::constant]
		type MaxStringLength: Get<u32>;

		/// Max CID length for legacy storage
		#[pezpallet::constant]
		type MaxCidLength: Get<u32>;
	}

	pub type BalanceOf<T> = <<T as Config>::Currency as pezframe_support::traits::Currency<
		<T as pezframe_system::Config>::AccountId,
	>>::Balance;

	// ============= STORAGE =============

	/// Citizenship applications (applicant -> application)
	/// PRIVACY: Only hash stored, no personal data
	#[pezpallet::storage]
	#[pezpallet::getter(fn applications)]
	pub type Applications<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		CitizenshipApplication<T::AccountId, BlockNumberFor<T>>,
	>;

	/// Current citizenship status per account
	#[pezpallet::storage]
	#[pezpallet::getter(fn kyc_status_of)]
	pub type KycStatuses<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, KycLevel, ValueQuery>;

	/// Identity hashes of approved citizens (for verification)
	/// Can be used to prove citizenship without revealing identity
	#[pezpallet::storage]
	#[pezpallet::getter(fn identity_hash_of)]
	pub type IdentityHashes<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, H256>;

	/// When each citizen was admitted.
	///
	/// Needed because vouching capacity is earned rather than granted: an account that became
	/// a citizen this block has none. Nothing else in the register knew the date -- membership
	/// was a yes or a no, with no age to it.
	#[pezpallet::storage]
	pub type CitizenSince<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>>;

	/// Reverse mapping: identity hash -> account ID (uniqueness enforcement)
	/// Ensures no two accounts can register with the same identity hash.
	///
	/// IMPORTANT: This map is populated at `apply_for_citizenship` time (not just
	/// at `confirm_citizenship` time) so that the hash is *reserved* for the whole
	/// lifetime of an in-flight application. This prevents two different accounts
	/// from concurrently applying with the same identity_hash (Sybil resistance) -
	/// without this early reservation, two pending applications could both later
	/// confirm and silently overwrite each other's reverse mapping.
	#[pezpallet::storage]
	#[pezpallet::getter(fn identity_hash_owner)]
	pub type IdentityHashToAccount<T: Config> = StorageMap<_, Blake2_128Concat, H256, T::AccountId>;

	/// Referrer of approved citizens (for direct responsibility tracking)
	/// Kept permanently for penalty system even after application is removed
	#[pezpallet::storage]
	#[pezpallet::getter(fn citizen_referrer)]
	pub type CitizenReferrers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId>;

	/// How many accounts currently hold `KycLevel::Approved`.
	///
	/// Kept rather than counted. `citizen_count()` walks the whole of `KycStatuses`, which is
	/// one storage read per record every time it is asked -- and it is asked by things that
	/// run repeatedly: election turnout on every finalisation, and the treasury's population
	/// gate on every era until it opens. That cost grows with the population it measures, so
	/// the closer the state gets to a milestone the more each check costs.
	///
	/// It lives here because this pallet owns the transitions. Updated in the same call that
	/// writes the status, it cannot drift; kept anywhere else it would depend on being told.
	#[pezpallet::storage]
	#[pezpallet::getter(fn approved_citizen_count)]
	pub type CitizenCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Citizenships the founder signed because nobody else would.
	///
	/// The founder is the guarantor of last resort: after the fallback period they may approve
	/// an application the named guarantor never answered. That is a structural role rather
	/// than a judgement about the person, so when one of these citizens is later revoked the
	/// consequence should not land on the founder for having kept the queue moving. Recorded
	/// here so `referral` can tell the two cases apart -- the founder standing for somebody
	/// they chose to stand for, and the founder standing in.
	#[pezpallet::storage]
	#[pezpallet::getter(fn approved_by_fallback)]
	pub type ApprovedByFallback<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	/// Citizens the state named rather than citizens who applied.
	///
	/// Honorary citizenship is citizenship: the same status, the same rights, counted in the
	/// same population. The register exists only so the distinction can be read from the
	/// chain -- how many people came through the referral process and how many the state
	/// conferred it on. Anyone can check the ratio; nobody is treated differently for it.
	#[pezpallet::storage]
	#[pezpallet::getter(fn is_honorary_citizen)]
	pub type HonoraryCitizens<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	/// How many of the citizens were named rather than admitted.
	///
	/// Kept alongside the map for the same reason `CitizenCount` is kept: the number is asked
	/// for, and walking the map to answer costs one read per honorary citizen.
	#[pezpallet::storage]
	#[pezpallet::getter(fn honorary_citizen_count)]
	pub type HonoraryCitizenCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	// `Identities` and `PendingKycApplications` used to live here, described as kept for a
	// migration. There was no migration -- this pallet has no `migrations.rs` -- and nothing
	// anywhere wrote to either of them: no insert, no mutate, no put, in the whole tree.
	//
	// They are gone rather than carried because of what they were shaped to hold. `Identities`
	// stored a name and an email as plain bytes, which is exactly the data this pallet exists
	// to keep off the chain; the design that wanted them there was replaced by the hash-only
	// one. Leaving an empty map of that shape in place is leaving a door for some later call
	// to write through.
	//
	// Removing storage without a migration is only safe because both chains that run this
	// pallet start again from genesis, so there are no keys to orphan. If that ever changes,
	// this needs a purge migration -- and a purge, not a move: the right thing to do with that
	// data is delete it.

	// ============= GENESIS CONFIG =============

	/// Genesis configuration for bootstrapping initial citizens
	/// BOOTSTRAP: Solves chicken-egg problem - first citizens need to exist for others to join
	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// List of founding citizens (AccountId, IdentityHash)
		/// These accounts start with Approved status and can accept referrals immediately
		pub founding_citizens: alloc::vec::Vec<(T::AccountId, H256)>,
		#[serde(skip)]
		pub _phantom: core::marker::PhantomData<T>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// Initialize founding citizens with Approved status
			for (account, identity_hash) in &self.founding_citizens {
				// Set status to Approved (citizen)
				KycStatuses::<T>::insert(account, KycLevel::Approved);
				// Store identity hash
				IdentityHashes::<T>::insert(account, *identity_hash);
				// Citizens since the first block, which is what lets them vouch at all: the
				// waiting period counts from this date and the founding generation has no
				// earlier one. Without it the tree could never take its first branch.
				CitizenSince::<T>::insert(account, BlockNumberFor::<T>::from(0u32));
				// Store reverse mapping for uniqueness enforcement
				IdentityHashToAccount::<T>::insert(*identity_hash, account);
			}
			// The founders are the first citizens, so the count starts at their number rather
			// than at zero.
			CitizenCount::<T>::put(self.founding_citizens.len() as u32);
		}
	}

	// ============= EVENTS =============

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New citizenship application submitted
		CitizenshipApplied { applicant: T::AccountId, referrer: T::AccountId, identity_hash: H256 },
		/// Referrer approved the application
		ReferralApproved { referrer: T::AccountId, applicant: T::AccountId },
		/// Applicant self-confirmed their citizenship (Welati NFT minted)
		CitizenshipConfirmed { who: T::AccountId },
		/// Citizenship was revoked (by governance)
		CitizenshipRevoked { who: T::AccountId },
		/// User renounced their citizenship
		CitizenshipRenounced { who: T::AccountId },
		/// Application was cancelled by the applicant
		ApplicationCancelled { who: T::AccountId },
		/// Citizenship was given back after having been taken.
		CitizenshipRestored { who: T::AccountId },
		/// The state conferred citizenship on someone directly.
		HonoraryCitizenshipRegistered { who: T::AccountId },
	}

	// ============= ERRORS =============

	#[pezpallet::error]
	pub enum Error<T> {
		/// Only a citizen may vouch, and this account is not one.
		NotEligibleToVouch,
		/// A citizen admitted this recently may not vouch yet.
		VouchingTooSoon,
		/// This account has brought in as many as its record allows.
		VouchingCapacityReached,
		/// Application already exists for this account
		ApplicationAlreadyExists,
		/// No application found for this account
		ApplicationNotFound,
		/// Referrer is not a citizen (must have Approved status)
		ReferrerNotCitizen,
		/// Cannot refer yourself
		SelfReferral,
		/// Cannot approve referral in current state (must be PendingReferral)
		CannotApproveInCurrentState,
		/// Cannot confirm in current state (must be ReferrerApproved)
		CannotConfirmInCurrentState,
		/// Cannot revoke in current state (must be Approved)
		CannotRevokeInCurrentState,
		/// User is not a citizen (cannot renounce)
		NotACitizen,
		/// Only the referrer can approve this application
		NotTheReferrer,
		/// Cannot cancel application in current state (must be PendingReferral)
		CannotCancelInCurrentState,
		/// Identity hash already registered by another account
		IdentityHashAlreadyUsed,
		/// Cannot restore: this account's citizenship was not revoked.
		NotRevoked,
		/// This account is already a citizen.
		AlreadyACitizen,
	}

	// ============= EXTRINSICS =============

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Apply for citizenship with identity hash and referrer
		///
		/// TRUSTLESS: No admin involved, referrer vouches for applicant
		/// PRIVACY: Only hash stored, actual identity is off-chain
		///
		/// # Arguments
		/// - `identity_hash`: H256 hash of identity documents (calculated off-chain)
		/// - `referrer`: Optional account of existing citizen who will vouch for you.
		///               If None or invalid, DefaultReferrer (founder) is used.
		///
		/// # Workflow
		/// 1. Applicant submits hash + optional referrer
		/// 2. If referrer is None/invalid, DefaultReferrer is used
		/// 3. Deposit is reserved (spam prevention)
		/// 4. Status becomes PendingReferral
		/// 5. Referrer must call approve_referral
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::apply_for_citizenship())]
		pub fn apply_for_citizenship(
			origin: OriginFor<T>,
			identity_hash: H256,
			referrer: Option<T::AccountId>,
			inviter: Option<T::AccountId>,
		) -> DispatchResult {
			let applicant = ensure_signed(origin)?;

			// Must not have existing application
			ensure!(
				KycStatuses::<T>::get(&applicant) == KycLevel::NotStarted,
				Error::<T>::ApplicationAlreadyExists
			);

			// Identity hash must be unique - no other account can use the same hash
			ensure!(
				!IdentityHashToAccount::<T>::contains_key(identity_hash),
				Error::<T>::IdentityHashAlreadyUsed
			);

			// Determine the actual referrer:
			// 1. Use provided referrer if valid (approved citizen and not self)
			// 2. Fall back to DefaultReferrer otherwise
			let actual_referrer = referrer
				.filter(|r| *r != applicant) // Not self-referral
				.filter(|r| KycStatuses::<T>::get(r) == KycLevel::Approved) // Must be citizen
				.unwrap_or_else(T::DefaultReferrer::get);

			// Verify the actual referrer is valid (including DefaultReferrer)
			ensure!(
				KycStatuses::<T>::get(&actual_referrer) == KycLevel::Approved,
				Error::<T>::ReferrerNotCitizen
			);

			// Cannot refer yourself (even with DefaultReferrer)
			ensure!(applicant != actual_referrer, Error::<T>::SelfReferral);

			// Reserve deposit (spam prevention, returned on approval)
			let deposit = T::KycApplicationDeposit::get();
			T::Currency::reserve(&applicant, deposit)?;

			// Reserve the identity hash immediately (not just at confirm time).
			// This closes the Sybil window where two different accounts could
			// both apply with the same identity_hash before either confirms -
			// see the doc comment on IdentityHashToAccount for details.
			IdentityHashToAccount::<T>::insert(identity_hash, &applicant);

			// Store application (only hash, no personal data)
			// Naming yourself as the one who brought you here is not a thing that can be true.
			let inviter = inviter.filter(|i| *i != applicant);

			let application = CitizenshipApplication {
				identity_hash,
				referrer: actual_referrer.clone(),
				inviter,
				applied_at: pezframe_system::Pezpallet::<T>::block_number(),
			};
			Applications::<T>::insert(&applicant, application);

			// Update status
			KycStatuses::<T>::insert(&applicant, KycLevel::PendingReferral);

			Self::deposit_event(Event::CitizenshipApplied {
				applicant,
				referrer: actual_referrer,
				identity_hash,
			});
			Ok(())
		}

		/// Referrer approves an applicant's citizenship application
		///
		/// TRUSTLESS: Referrer takes personal responsibility for this referral
		/// ACCOUNTABILITY: If applicant turns out malicious, referrer is penalized
		///
		/// # Arguments
		/// - `applicant`: Account of the person you're vouching for
		///
		/// # Requirements
		/// - Caller must be the referrer specified in the application
		/// - Application must be in PendingReferral state
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::approve_referral())]
		pub fn approve_referral(origin: OriginFor<T>, applicant: T::AccountId) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// Two limits on the act of vouching, and neither is about the applicant.
			//
			// The register admits people on one citizen's word, so the word has to cost
			// something or the population can be manufactured. It costs waiting -- a citizen
			// admitted today vouches for nobody -- and it costs having vouched well before:
			// capacity is earned from settled referrals and lost to revoked ones.
			//
			// The founder is exempt from both. The root of the tree has no earlier record to
			// earn from, and the fallback that lets it approve a stalled application would
			// stop working if it could run out.
			if caller != T::DefaultReferrer::get() {
				let since =
					CitizenSince::<T>::get(&caller).ok_or(Error::<T>::NotEligibleToVouch)?;
				// The founding generation waits for nothing. The waiting period exists to slow
				// a chain of vouching -- one forged citizen admitting the next within minutes
				// -- and the founding citizens were not vouched in. They were written at
				// genesis, checked off chain, and their number is fixed, so an attacker cannot
				// grow that set and there is nothing here to slow down. Making them wait would
				// close the register for a month and prevent no attack at all.
				let admitted_at_genesis = since.is_zero();
				ensure!(
					admitted_at_genesis
						|| pezframe_system::Pezpallet::<T>::block_number().saturating_sub(since)
							>= T::VouchingWaitingPeriod::get(),
					Error::<T>::VouchingTooSoon
				);
				ensure!(
					T::VouchingCapacity::remaining(&caller) != Some(0),
					Error::<T>::VouchingCapacityReached
				);
			}

			// Must be in PendingReferral state
			ensure!(
				KycStatuses::<T>::get(&applicant) == KycLevel::PendingReferral,
				Error::<T>::CannotApproveInCurrentState
			);

			// Get application
			let mut application =
				Applications::<T>::get(&applicant).ok_or(Error::<T>::ApplicationNotFound)?;

			// Normally only the referrer may approve. After the fallback period the founder
			// may too, and becomes the referrer of record -- so the accountability that
			// follows a referral follows the person who actually vouched, not the one who
			// stayed silent.
			//
			// The original referrer is never locked out: they can still approve on the last
			// day of the tenth year if they get round to it. What the fallback adds is that
			// the applicant is no longer waiting on one person alone.
			let founder = T::DefaultReferrer::get();
			let fallback_open = pezframe_system::Pezpallet::<T>::block_number()
				> application.applied_at.saturating_add(T::ReferralFallbackPeriod::get());

			if application.referrer != caller {
				ensure!(caller == founder && fallback_open, Error::<T>::NotTheReferrer);
				application.referrer = caller.clone();
				Applications::<T>::insert(&applicant, application.clone());
				ApprovedByFallback::<T>::insert(&applicant, ());
			}

			// Re-check the referrer's *current* KYC status. The referrer may have
			// been an Approved citizen when the application was created, but could
			// have been revoked by governance since then (e.g. for running a
			// referral-selling scheme). A revoked citizen must not retain the
			// ability to vouch new citizens into the system.
			ensure!(
				KycStatuses::<T>::get(&caller) == KycLevel::Approved,
				Error::<T>::ReferrerNotCitizen
			);

			// Update status to ReferrerApproved
			KycStatuses::<T>::insert(&applicant, KycLevel::ReferrerApproved);

			Self::deposit_event(Event::ReferralApproved { referrer: caller, applicant });
			Ok(())
		}

		/// Self-confirm citizenship after referrer approval
		///
		/// TRUSTLESS: Applicant confirms themselves, no admin needed
		/// WELATI ONLY: This mints the Citizen NFT via self-confirmation
		///
		/// # Workflow
		/// 1. Deposit is returned
		/// 2. Identity hash is stored permanently
		/// 3. Status becomes Approved
		/// 4. Citizen NFT (Welati) is minted
		/// 5. Referral hooks are triggered
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::confirm_citizenship())]
		pub fn confirm_citizenship(origin: OriginFor<T>) -> DispatchResult {
			let applicant = ensure_signed(origin)?;

			// Must be in ReferrerApproved state
			ensure!(
				KycStatuses::<T>::get(&applicant) == KycLevel::ReferrerApproved,
				Error::<T>::CannotConfirmInCurrentState
			);

			// Get application
			let application =
				Applications::<T>::take(&applicant).ok_or(Error::<T>::ApplicationNotFound)?;

			// Return deposit
			let deposit = T::KycApplicationDeposit::get();
			T::Currency::unreserve(&applicant, deposit);

			// Store identity hash permanently (for proof of citizenship)
			IdentityHashes::<T>::insert(&applicant, application.identity_hash);

			// The date the vouching clock runs from.
			CitizenSince::<T>::insert(&applicant, pezframe_system::Pezpallet::<T>::block_number());

			// Reverse mapping was already reserved for this applicant at apply time
			// (see apply_for_citizenship); re-affirm it here for good measure. This
			// is a no-op in the normal path since no other account can hold the
			// same identity_hash (the apply-time reservation guarantees that).
			IdentityHashToAccount::<T>::insert(application.identity_hash, &applicant);

			// Store referrer permanently (for direct responsibility tracking)
			// This is needed even after Applications is removed for penalty system
			CitizenReferrers::<T>::insert(&applicant, application.referrer.clone());

			// Update status to Approved
			KycStatuses::<T>::insert(&applicant, KycLevel::Approved);
			CitizenCount::<T>::mutate(|n| *n = n.saturating_add(1));

			// The NFT is not a decoration on citizenship, it is how the rest of the state sees
			// it: every tiki requires one, and `tiki::is_citizen` reads it. This used to log a
			// failure and carry on, which produced people who were `Approved` here, counted in
			// the population the treasury pays, and not citizens at all as far as any other
			// pallet could tell. Better for the call to fail and be retried.
			T::CitizenNftProvider::mint_citizen_nft_confirmed(&applicant)?;

			// Trigger referral hooks (for referral pezpallet)
			// Pass referrer parameter to avoid data loss between pallets
			T::OnKycApproved::on_kyc_approved(
				&applicant,
				&application.referrer,
				application.inviter.as_ref(),
			);

			Self::deposit_event(Event::CitizenshipConfirmed { who: applicant });
			Ok(())
		}

		/// Revoke citizenship (governance only)
		///
		/// Used for malicious actors identified by governance
		/// DIRECT RESPONSIBILITY: Triggers penalty for the referrer via referral pezpallet
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::revoke_citizenship())]
		pub fn revoke_citizenship(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(
				KycStatuses::<T>::get(&who) == KycLevel::Approved,
				Error::<T>::CannotRevokeInCurrentState
			);

			// Update status
			KycStatuses::<T>::insert(&who, KycLevel::Revoked);
			CitizenCount::<T>::mutate(|n| *n = n.saturating_sub(1));

			// Burn citizen NFT
			if let Err(e) = T::CitizenNftProvider::burn_citizen_nft(&who) {
				log::warn!("Failed to burn citizen NFT for {who:?}: {e:?}");
			}

			// Trigger direct responsibility penalty for the referrer
			// This hook notifies the referral pezpallet to penalize the referrer
			T::OnCitizenshipRevoked::on_citizenship_revoked(&who);

			Self::deposit_event(Event::CitizenshipRevoked { who });
			Ok(())
		}

		/// Give citizenship back to someone it was taken from.
		///
		/// `Revoked` was a terminal state: `apply_for_citizenship` requires `NotStarted`, so a
		/// revocation could never be undone by any path, correct or not. A state with a
		/// constitutional court and no way to restore what the court finds was wrongly taken
		/// is missing the half of judicial review that matters to the person.
		///
		/// Restores directly to `Approved` rather than sending them back to the start. Making
		/// someone re-apply, find a referrer and wait would be a second penalty for a
		/// revocation that has just been found unjustified.
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::WeightInfo::revoke_citizenship())]
		pub fn restore_citizenship(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(KycStatuses::<T>::get(&who) == KycLevel::Revoked, Error::<T>::NotRevoked);

			KycStatuses::<T>::insert(&who, KycLevel::Approved);
			CitizenCount::<T>::mutate(|n| *n = n.saturating_add(1));

			// The NFT was burned when citizenship was taken, so it has to be minted again --
			// and if that fails the restoration fails with it, for the same reason approval
			// does: a citizen the rest of the state cannot see is not restored.
			T::CitizenNftProvider::mint_citizen_nft(&who)?;

			Self::deposit_event(Event::CitizenshipRestored { who });
			Ok(())
		}

		/// Renounce citizenship (voluntary exit)
		///
		/// Users can freely leave the system
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::WeightInfo::renounce_citizenship())]
		pub fn renounce_citizenship(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(KycStatuses::<T>::get(&who) == KycLevel::Approved, Error::<T>::NotACitizen);

			// Burn citizen NFT
			T::CitizenNftProvider::burn_citizen_nft(&who)?;

			// Reset status
			KycStatuses::<T>::insert(&who, KycLevel::NotStarted);
			CitizenCount::<T>::mutate(|n| *n = n.saturating_sub(1));

			// Remove identity hash and reverse mapping
			if let Some(hash) = IdentityHashes::<T>::take(&who) {
				IdentityHashToAccount::<T>::remove(hash);
			}

			Self::deposit_event(Event::CitizenshipRenounced { who });
			Ok(())
		}

		/// Cancel pending application and retrieve deposit
		///
		/// Useful if referrer is unresponsive or user made a mistake.
		/// SAFETY: Only works in PendingReferral state (not yet approved)
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(T::WeightInfo::cancel_application())]
		pub fn cancel_application(origin: OriginFor<T>) -> DispatchResult {
			let applicant = ensure_signed(origin)?;

			// Must be in PendingReferral state (not yet approved by referrer)
			ensure!(
				KycStatuses::<T>::get(&applicant) == KycLevel::PendingReferral,
				Error::<T>::CannotCancelInCurrentState
			);

			// Remove application and release the reserved identity hash so it can
			// be used again (by this account or another) in a future application.
			let application =
				Applications::<T>::take(&applicant).ok_or(Error::<T>::ApplicationNotFound)?;
			IdentityHashToAccount::<T>::remove(application.identity_hash);

			// Reset status
			KycStatuses::<T>::insert(&applicant, KycLevel::NotStarted);

			// Unreserve deposit
			let deposit = T::KycApplicationDeposit::get();
			T::Currency::unreserve(&applicant, deposit);

			Self::deposit_event(Event::ApplicationCancelled { who: applicant });
			Ok(())
		}
	}
}

// ============= TRAIT IMPLEMENTATIONS =============

pub use types::KycStatus;

impl<T: Config> types::KycStatus<T::AccountId> for Pezpallet<T> {
	fn get_kyc_status(who: &T::AccountId) -> KycLevel {
		KycStatuses::<T>::get(who)
	}
}

/// Helper methods for checking citizenship
impl<T: Config> Pezpallet<T> {
	/// Check if account is a citizen
	pub fn is_citizen(who: &T::AccountId) -> bool {
		KycStatuses::<T>::get(who) == KycLevel::Approved
	}

	/// Count total number of citizens.
	///
	/// Reads the maintained counter rather than walking `KycStatuses`. Callers run this
	/// repeatedly -- election turnout on every finalisation, the treasury's population gate
	/// on every era until it opens -- and a walk costs one read per record each time.
	pub fn citizen_count() -> u32 {
		CitizenCount::<T>::get()
	}

	/// Count citizens the slow way, by walking every record.
	///
	/// This is what `citizen_count` used to do. It is kept as the thing the counter is
	/// checked against in `try_state`: a maintained counter always fails the same way, by
	/// some path changing the status and forgetting to say so, and the only way to catch
	/// that is to compare against the source of truth.
	pub fn count_citizens_by_scan() -> u32 {
		KycStatuses::<T>::iter()
			.filter(|(_, status)| *status == KycLevel::Approved)
			.count() as u32
	}

	/// Record someone the state has named a citizen.
	///
	/// Called by `tiki` when honorary citizenship is granted. Without it the two registers
	/// disagree: the person holds the citizen NFT and the Welati tiki, and this pallet -- the
	/// one the treasury counts and every election reads -- has never heard of them. They
	/// would be a citizen to some pallets and a stranger to others.
	///
	/// What is written here is ordinary citizenship. The only difference recorded anywhere is
	/// the honorary register, which exists so the chain can be asked how many citizens
	/// applied and how many were named, and for nothing else.
	pub fn register_honorary_citizen(who: &T::AccountId) -> pezsp_runtime::DispatchResult {
		ensure!(KycStatuses::<T>::get(who) != KycLevel::Approved, Error::<T>::AlreadyACitizen);

		KycStatuses::<T>::insert(who, KycLevel::Approved);
		CitizenCount::<T>::mutate(|n| *n = n.saturating_add(1));
		HonoraryCitizens::<T>::insert(who, ());
		HonoraryCitizenCount::<T>::mutate(|n| *n = n.saturating_add(1));

		Self::deposit_event(Event::HonoraryCitizenshipRegistered { who: who.clone() });
		Ok(())
	}

	/// Get the referrer of a citizen or applicant
	/// Checks both pending applications and approved citizen records
	pub fn get_referrer(who: &T::AccountId) -> Option<T::AccountId> {
		// First check permanent storage (for approved citizens)
		CitizenReferrers::<T>::get(who)
			// Then check pending applications
			.or_else(|| Applications::<T>::get(who).map(|app| app.referrer))
	}

	/// Get identity hash of a citizen
	pub fn get_identity_hash(who: &T::AccountId) -> Option<H256> {
		IdentityHashes::<T>::get(who)
	}
}

/// Trait for trust pezpallet integration
pub trait CitizenshipStatusProvider<AccountId> {
	fn is_citizen(who: &AccountId) -> bool;
}

impl<T: Config> CitizenshipStatusProvider<T::AccountId> for Pezpallet<T> {
	fn is_citizen(who: &T::AccountId) -> bool {
		KycStatuses::<T>::get(who) == KycLevel::Approved
	}
}
