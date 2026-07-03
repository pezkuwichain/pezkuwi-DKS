#![cfg_attr(not(feature = "std"), no_std)]

//! # Staking Score Pezpallet
//!
//! Calculates time-weighted staking scores from cached staking data received via XCM.
//!
//! ## Overview
//!
//! People Chain does not have direct access to staking data. Staking details are
//! submitted by noter-authorized accounts (or root via XCM Transact) into
//! `CachedStakingDetails`. This pallet aggregates stake from all sources and
//! calculates a score based on amount and duration.
//!
//! ## Noter Delegation
//!
//! The sudo account delegates `receive_staking_details` authority to accounts that
//! hold the `Noter` tiki (role NFT). A bot collects staking data from Relay Chain
//! and Asset Hub, then a noter signs and submits the data to People Chain. The
//! Noter tiki is not a single hardcoded account — any number of independent
//! accounts can hold it, the same way a state can authorize any number of notaries.
//!
//! ## Bonded Registration and Dispute Window
//!
//! Holding the Noter tiki alone is not enough to submit data: an account must
//! also call `register_as_noter()` and post a bond (`NoterBondAmount`), mirroring
//! a real notary's insurance/liability bond. A noter-submitted update does not
//! take effect immediately — it sits in `PendingStakingDetails` for
//! `DisputeWindow` blocks (like a deed's recording/contestability period) during
//! which `T::DisputeOrigin` (any single Council member) can call
//! `dispute_staking_details()` to freeze it pending governance review. If
//! governance (`T::SlashOrigin`) confirms the submission was fraudulent, it can
//! call `slash_noter()` to burn the noter's bond to `T::SlashDestination`.
//!
//! Root (XCM Transact) submissions are exempt from the bond and the dispute
//! window: that origin is chain-authenticated (backed by consensus, not a single
//! private key), so it carries strictly more assurance than any bonded noter.
//!
//! ## Dual-Chain Staking
//!
//! Users can stake on both Relay Chain (direct staking) and Asset Hub (nomination pools).
//! `CachedStakingDetails` is a `StorageDoubleMap` keyed by `(AccountId, StakingSource)`
//! to track stake per source. Score calculation aggregates across all sources.
//!
//! ## Workflow
//!
//! 1. User calls `start_score_tracking()` to opt-in to time-based scoring
//! 2. Bot detects the event, collects staking data from Relay Chain / Asset Hub
//! 3. Noter submits `receive_staking_details()` with the staking data
//! 4. `pezpallet-trust` queries staking score via `StakingScoreProvider` trait
//! 5. Score = base_score(amount_tier) * duration_multiplier, capped at 100

pub use pezpallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::weights::WeightInfo;
	use core::ops::Div;
	use pezframe_support::{
		pezpallet_prelude::*,
		traits::{Currency, ExistenceRequirement, ReservableCurrency},
	};
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::traits::{Saturating, Zero};

	// --- Constants ---
	//
	// Block-time assumption: 5 blocks/minute (12s blocks), matching this
	// chain's actual configured block time (`teyrchains_common::MILLISECS_PER_BLOCK
	// = 12_000`, the same source `people-pezkuwichain`'s own `HOURS`/`DAYS`
	// constants are derived from). Previously hardcoded at 10 blocks/minute
	// (6s blocks) here, which silently doubled every duration-tier threshold
	// in production — the "1 month" tier actually took ~2 real months to
	// reach. If this chain's block time ever changes, these two constants
	// need updating to match — they are not derived automatically.
	pub const MONTH_IN_BLOCKS: u32 = 30 * 24 * 60 * 5;
	pub const HOUR_IN_BLOCKS: u32 = 60 * 5;
	pub const UNITS: u128 = 1_000_000_000_000;

	/// The chain from which staking data originates.
	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		Copy,
		PartialEq,
		Eq,
		TypeInfo,
		Debug,
		MaxEncodedLen,
	)]
	pub enum StakingSource {
		/// Direct staking on the Relay Chain.
		RelayChain = 0,
		/// Staking via nomination pools on Asset Hub.
		AssetHub = 1,
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	/// Trait for checking if an account has noter authority.
	/// Noter-authorized accounts can submit staking details on behalf of users.
	pub trait NoterCheck<AccountId> {
		fn is_noter(who: &AccountId) -> bool;

		/// Grant noter authority to `who`, for benchmarking only. Lets a
		/// runtime's real checker (e.g. one backed by the tiki pallet's role
		/// NFTs) make its own benchmark setup self-contained, instead of this
		/// pallet's generic benchmarks needing to know how an external role
		/// system works. Default no-op; runtimes whose `is_noter` depends on
		/// external state should override it.
		#[cfg(feature = "runtime-benchmarks")]
		fn make_noter(_who: &AccountId) {}
	}

	/// Default implementation: nobody is noter (safe default for tests).
	impl<AccountId> NoterCheck<AccountId> for () {
		fn is_noter(_who: &AccountId) -> bool {
			false
		}
	}

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>>
	where
		BlockNumberFor<Self>: From<u32>,
	{
		/// Balance type used for staking amounts.
		type Balance: Member
			+ Parameter
			+ MaxEncodedLen
			+ Copy
			+ Default
			+ PartialOrd
			+ Saturating
			+ Zero
			+ Div<Output = Self::Balance>
			+ From<u128>;

		/// Callback when staking data changes for an account.
		/// Trust pallet implements this to trigger score recalculation.
		type OnStakingUpdate: OnStakingDataUpdate<Self::AccountId>;

		/// Weight information for extrinsics.
		type WeightInfo: WeightInfo;

		/// Checker for noter authority. Accounts with the Noter tiki can submit
		/// staking details without requiring root origin.
		type NoterChecker: NoterCheck<Self::AccountId>;

		/// Reservable currency used for the noter registration bond.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Bond a Noter-tiki holder must reserve via `register_as_noter()` before
		/// their submissions are accepted. Slashable by `SlashOrigin` if governance
		/// confirms a submission was fraudulent.
		#[pezpallet::constant]
		type NoterBondAmount: Get<BalanceOf<Self>>;

		/// How long a noter-submitted (signed-origin) staking update sits as a
		/// candidate before it takes effect, giving `DisputeOrigin` a window to
		/// freeze it. Root/XCM-Transact submissions are exempt.
		#[pezpallet::constant]
		type DisputeWindow: Get<BlockNumberFor<Self>>;

		/// Origin allowed to dispute (freeze) a pending noter submission before it
		/// matures. Deliberately lightweight — any single authorized member.
		/// `Success = AccountId` so the disputing member's identity can be
		/// recorded in `Event::StakingDetailsDisputed`.
		type DisputeOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

		/// Origin allowed to slash a noter's bond, after confirming a disputed
		/// submission was fraudulent. Deliberately a stronger, collective bar than
		/// `DisputeOrigin`.
		type SlashOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Where a slashed noter's bond goes.
		type SlashDestination: Get<Self::AccountId>;
	}

	/// Balance type of `T::Currency` (the noter bond currency) — distinct from
	/// `T::Balance` above, which represents staking amounts, not reservable funds.
	pub type BalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as pezframe_system::Config>::AccountId,
	>>::Balance;

	// --- Storage ---

	#[pezpallet::storage]
	#[pezpallet::getter(fn staking_start_block)]
	pub type StakingStartBlock<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

	/// Cached staking details received via XCM from various chains.
	/// Keyed by (AccountId, StakingSource) to support stake aggregation across chains.
	#[pezpallet::storage]
	#[pezpallet::getter(fn cached_staking_details)]
	pub type CachedStakingDetails<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		StakingSource,
		StakingDetails<T::Balance>,
		OptionQuery,
	>;

	/// Registered (bonded) noters, and the exact amount each reserved. An
	/// account must appear here — in addition to holding the Noter tiki — before
	/// `receive_staking_details` accepts their submissions.
	#[pezpallet::storage]
	#[pezpallet::getter(fn noter_bond)]
	pub type NoterBonds<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, OptionQuery>;

	/// Block a noter last submitted data at. Gates `unregister_as_noter` until
	/// any submission they made has cleared its dispute window, so a noter can't
	/// submit fraudulent data and immediately withdraw their bond ahead of review.
	#[pezpallet::storage]
	pub type NoterLastSubmission<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

	/// A noter-submitted staking update awaiting its dispute window. Does not
	/// affect `get_staking_score` until finalized into `CachedStakingDetails` —
	/// either opportunistically (before the same account's next submission),
	/// explicitly via `finalize_staking_details`, or discarded via
	/// `dispute_staking_details`. Root/XCM-Transact submissions skip this
	/// entirely and write `CachedStakingDetails` directly.
	#[pezpallet::storage]
	#[pezpallet::getter(fn pending_staking_details)]
	pub type PendingStakingDetails<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		StakingSource,
		PendingSubmission<T::Balance, T::AccountId, BlockNumberFor<T>>,
		OptionQuery,
	>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A user started time-based scoring.
		ScoreTrackingStarted { who: T::AccountId, start_block: BlockNumberFor<T> },
		/// Staking details took effect — either a Root/XCM-Transact submission
		/// (immediate) or a noter submission that has cleared its dispute window.
		StakingDetailsReceived {
			who: T::AccountId,
			source: StakingSource,
			staked_amount: T::Balance,
		},
		/// A noter-signed staking update was recorded as a pending candidate and
		/// will take effect at `matures_at` unless disputed before then.
		StakingDetailsPending {
			who: T::AccountId,
			source: StakingSource,
			staked_amount: T::Balance,
			submitted_by: T::AccountId,
			matures_at: BlockNumberFor<T>,
		},
		/// A pending noter submission was frozen before it could take effect.
		StakingDetailsDisputed { who: T::AccountId, source: StakingSource, disputed_by: T::AccountId },
		/// An account registered as an active (bonded) noter.
		NoterRegistered { who: T::AccountId, bond: BalanceOf<T> },
		/// An account unregistered as noter and reclaimed its bond.
		NoterUnregistered { who: T::AccountId, bond_returned: BalanceOf<T> },
		/// A noter's bond was slashed by governance after a confirmed fraudulent
		/// submission.
		NoterSlashed { who: T::AccountId, amount: BalanceOf<T> },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// User must have stake to start score tracking.
		NoStakeFound,
		/// Score tracking has already been started for this account.
		TrackingAlreadyStarted,
		/// Caller does not have noter authority (missing the Noter tiki).
		NotAuthorized,
		/// Caller holds the Noter tiki but has not posted the registration bond.
		NotRegisteredNoter,
		/// Caller has already registered (and bonded) as a noter.
		AlreadyRegisteredNoter,
		/// Cannot unregister while a submission is still within its dispute
		/// window — wait for it to mature or be disputed first.
		PendingSubmissionExists,
		/// No pending submission exists for this (account, source) pair.
		NoPendingSubmission,
		/// The pending submission's dispute window has not yet elapsed.
		NotYetMatured,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Start time-based score accumulation. One-time opt-in call per user.
		///
		/// The user does not need to have cached staking data yet. A bot will
		/// detect the `ScoreTrackingStarted` event and a noter will submit the
		/// staking data via `receive_staking_details`.
		///
		/// Duration tracking begins at the block this is called, regardless of
		/// when the staking data arrives.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::start_score_tracking())]
		pub fn start_score_tracking(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				StakingStartBlock::<T>::get(&who).is_none(),
				Error::<T>::TrackingAlreadyStarted
			);

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			StakingStartBlock::<T>::insert(&who, current_block);

			// Notify trust pallet. Score may be 0 if CachedStakingDetails is empty.
			T::OnStakingUpdate::on_staking_data_changed(&who);

			Self::deposit_event(Event::ScoreTrackingStarted { who, start_block: current_block });
			Ok(())
		}

		/// Receive staking details for an account.
		///
		/// Root origin (XCM Transact) is chain-authenticated and takes effect
		/// immediately, exactly as before. A signed origin from a *registered*
		/// Noter-tiki holder instead records the update as a pending candidate
		/// (see `PendingStakingDetails`) that only takes effect after
		/// `DisputeWindow` blocks — see the module docs for why.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::receive_staking_details())]
		pub fn receive_staking_details(
			origin: OriginFor<T>,
			who: T::AccountId,
			source: StakingSource,
			staked_amount: T::Balance,
			nominations_count: u32,
			unlocking_chunks_count: u32,
		) -> DispatchResult {
			if ensure_root(origin.clone()).is_ok() {
				Self::apply_staking_update(
					&who,
					source,
					staked_amount,
					nominations_count,
					unlocking_chunks_count,
				);
				return Ok(());
			}

			let caller = ensure_signed(origin)?;
			ensure!(T::NoterChecker::is_noter(&caller), Error::<T>::NotAuthorized);
			ensure!(NoterBonds::<T>::contains_key(&caller), Error::<T>::NotRegisteredNoter);

			// Opportunistically finalize a previously-matured pending entry for
			// this (who, source) before recording the new candidate. Without this,
			// a steady stream of legitimate noter updates would keep resetting the
			// dispute window and the account's effective stake would never
			// progress past whatever Root last committed (often zero).
			Self::maybe_finalize(&who, source);

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			NoterLastSubmission::<T>::insert(&caller, current_block);

			let details =
				StakingDetails { staked_amount, nominations_count, unlocking_chunks_count };
			PendingStakingDetails::<T>::insert(
				&who,
				source,
				PendingSubmission {
					details,
					submitted_by: caller.clone(),
					submitted_at: current_block,
				},
			);

			Self::deposit_event(Event::StakingDetailsPending {
				who,
				source,
				staked_amount,
				submitted_by: caller,
				matures_at: current_block.saturating_add(T::DisputeWindow::get()),
			});
			Ok(())
		}

		/// Register as an active noter: reserve `NoterBondAmount` and become
		/// eligible to submit staking data. Requires already holding the Noter
		/// tiki — this only adds the bond on top of that role, it does not grant
		/// the role itself (that remains a separate governance decision).
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::register_as_noter())]
		pub fn register_as_noter(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(T::NoterChecker::is_noter(&who), Error::<T>::NotAuthorized);
			ensure!(!NoterBonds::<T>::contains_key(&who), Error::<T>::AlreadyRegisteredNoter);

			let bond = T::NoterBondAmount::get();
			T::Currency::reserve(&who, bond)?;
			NoterBonds::<T>::insert(&who, bond);

			Self::deposit_event(Event::NoterRegistered { who, bond });
			Ok(())
		}

		/// Unregister as noter and reclaim the bond. Blocked while a submission
		/// this account made is still within its dispute window, so a noter can't
		/// submit fraudulent data and withdraw ahead of review.
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::unregister_as_noter())]
		pub fn unregister_as_noter(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let bond = NoterBonds::<T>::get(&who).ok_or(Error::<T>::NotRegisteredNoter)?;

			if let Some(last_submission) = NoterLastSubmission::<T>::get(&who) {
				let current_block = pezframe_system::Pezpallet::<T>::block_number();
				ensure!(
					current_block >= last_submission.saturating_add(T::DisputeWindow::get()),
					Error::<T>::PendingSubmissionExists
				);
			}

			T::Currency::unreserve(&who, bond);
			NoterBonds::<T>::remove(&who);
			NoterLastSubmission::<T>::remove(&who);

			Self::deposit_event(Event::NoterUnregistered { who, bond_returned: bond });
			Ok(())
		}

		/// Freeze a pending noter submission before it matures. Discards the
		/// candidate entirely — it does not become effective, and does not by
		/// itself slash the noter's bond (that is a separate governance decision
		/// via `slash_noter`, once the dispute has actually been reviewed).
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::WeightInfo::dispute_staking_details())]
		pub fn dispute_staking_details(
			origin: OriginFor<T>,
			who: T::AccountId,
			source: StakingSource,
		) -> DispatchResult {
			let disputed_by = T::DisputeOrigin::ensure_origin(origin)?;
			ensure!(
				PendingStakingDetails::<T>::contains_key(&who, source),
				Error::<T>::NoPendingSubmission
			);

			PendingStakingDetails::<T>::remove(&who, source);

			Self::deposit_event(Event::StakingDetailsDisputed { who, source, disputed_by });
			Ok(())
		}

		/// Commit a pending noter submission once its dispute window has elapsed.
		/// Permissionless (any signed, fee-paying account may call it) — like
		/// recording a notarized deed once its contestability period has passed,
		/// this is a mechanical step anyone can trigger, not a privileged action.
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(T::WeightInfo::finalize_staking_details())]
		pub fn finalize_staking_details(
			origin: OriginFor<T>,
			who: T::AccountId,
			source: StakingSource,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let pending =
				PendingStakingDetails::<T>::get(&who, source).ok_or(Error::<T>::NoPendingSubmission)?;
			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			ensure!(
				current_block >= pending.submitted_at.saturating_add(T::DisputeWindow::get()),
				Error::<T>::NotYetMatured
			);

			PendingStakingDetails::<T>::remove(&who, source);
			Self::apply_staking_update(
				&who,
				source,
				pending.details.staked_amount,
				pending.details.nominations_count,
				pending.details.unlocking_chunks_count,
			);
			Ok(())
		}

		/// Slash a noter's bond after governance confirms a disputed submission
		/// was fraudulent, and remove their registration. This is a deliberate,
		/// reviewed governance action (`SlashOrigin`) — never automatic — the
		/// same way a real notary's insurance bond is only drawn on after a
		/// finding of fault, not on a bare accusation.
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::WeightInfo::slash_noter())]
		pub fn slash_noter(origin: OriginFor<T>, noter: T::AccountId) -> DispatchResult {
			T::SlashOrigin::ensure_origin(origin)?;

			let bond = NoterBonds::<T>::take(&noter).ok_or(Error::<T>::NotRegisteredNoter)?;
			NoterLastSubmission::<T>::remove(&noter);

			T::Currency::unreserve(&noter, bond);
			T::Currency::transfer(
				&noter,
				&T::SlashDestination::get(),
				bond,
				ExistenceRequirement::AllowDeath,
			)?;

			Self::deposit_event(Event::NoterSlashed { who: noter, amount: bond });
			Ok(())
		}
	}

	// --- Types ---

	/// Raw score type used in staking score calculations.
	pub type RawScore = u32;

	/// Staking details for a single source chain.
	#[derive(
		Default,
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		Debug,
		MaxEncodedLen,
	)]
	pub struct StakingDetails<Balance> {
		pub staked_amount: Balance,
		pub nominations_count: u32,
		pub unlocking_chunks_count: u32,
	}

	/// A noter-submitted staking update awaiting its dispute window.
	#[derive(
		Encode,
		Decode,
		DecodeWithMemTracking,
		Clone,
		PartialEq,
		Eq,
		TypeInfo,
		Debug,
		MaxEncodedLen,
	)]
	pub struct PendingSubmission<Balance, AccountId, BlockNumber> {
		pub details: StakingDetails<Balance>,
		pub submitted_by: AccountId,
		pub submitted_at: BlockNumber,
	}

	// --- Traits ---

	/// Interface for querying staking scores. Used by trust pallet.
	pub trait StakingScoreProvider<AccountId, BlockNumber> {
		/// Returns (score, duration_in_blocks) for the given account.
		fn get_staking_score(who: &AccountId) -> (RawScore, BlockNumber);
	}

	/// Callback trait for when staking data changes.
	/// Trust pallet implements this to recalculate scores on staking updates.
	pub trait OnStakingDataUpdate<AccountId> {
		fn on_staking_data_changed(who: &AccountId);
	}

	impl<AccountId> OnStakingDataUpdate<AccountId> for () {
		fn on_staking_data_changed(_who: &AccountId) {}
	}

	// --- Helpers ---

	impl<T: Config> Pezpallet<T> {
		/// Calculate total cached stake across all sources for a given account.
		pub fn total_cached_stake(who: &T::AccountId) -> T::Balance {
			let mut total = T::Balance::zero();
			for (_, details) in CachedStakingDetails::<T>::iter_prefix(who) {
				total = total.saturating_add(details.staked_amount);
			}
			total
		}

		/// Actually apply a staking update to `CachedStakingDetails`. This is the
		/// single place data becomes *effective* — called directly for Root/XCM
		/// submissions, and via `maybe_finalize` once a noter submission's
		/// dispute window has elapsed. The anti flash-stake duration-reset guard
		/// lives here so it fires when data becomes effective, not at submission
		/// time (which, for noter submissions, is not yet trustworthy).
		fn apply_staking_update(
			who: &T::AccountId,
			source: StakingSource,
			staked_amount: T::Balance,
			nominations_count: u32,
			unlocking_chunks_count: u32,
		) {
			let previous_total = Self::total_cached_stake(who);

			if staked_amount.is_zero() {
				CachedStakingDetails::<T>::remove(who, source);

				let remaining = Self::total_cached_stake(who);
				if remaining.is_zero() {
					StakingStartBlock::<T>::remove(who);
				}
			} else {
				let details =
					StakingDetails { staked_amount, nominations_count, unlocking_chunks_count };
				CachedStakingDetails::<T>::insert(who, source, details);

				let new_total = Self::total_cached_stake(who);
				if !previous_total.is_zero() &&
					new_total > previous_total &&
					StakingStartBlock::<T>::contains_key(who)
				{
					let current_block = pezframe_system::Pezpallet::<T>::block_number();
					StakingStartBlock::<T>::insert(who, current_block);
				}
			}

			T::OnStakingUpdate::on_staking_data_changed(who);
			Self::deposit_event(Event::StakingDetailsReceived { who: who.clone(), source, staked_amount });
		}

		/// If a pending noter submission for `(who, source)` exists and has
		/// cleared its dispute window, commit it via `apply_staking_update` and
		/// clear the pending slot. Returns `true` if it finalized something.
		/// No-op (returns `false`) if there's nothing pending, or it hasn't
		/// matured yet — callers that need to distinguish those two cases (like
		/// `finalize_staking_details`) check the storage directly instead.
		fn maybe_finalize(who: &T::AccountId, source: StakingSource) -> bool {
			let Some(pending) = PendingStakingDetails::<T>::get(who, source) else {
				return false;
			};
			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			if current_block < pending.submitted_at.saturating_add(T::DisputeWindow::get()) {
				return false;
			}

			PendingStakingDetails::<T>::remove(who, source);
			Self::apply_staking_update(
				who,
				source,
				pending.details.staked_amount,
				pending.details.nominations_count,
				pending.details.unlocking_chunks_count,
			);
			true
		}
	}

	// --- StakingScoreProvider Implementation ---

	impl<T: Config> StakingScoreProvider<T::AccountId, BlockNumberFor<T>> for Pezpallet<T> {
		fn get_staking_score(who: &T::AccountId) -> (RawScore, BlockNumberFor<T>) {
			// Aggregate stake from all cached sources.
			let total_staked = Self::total_cached_stake(who);
			let staked_hez: T::Balance = total_staked / UNITS.into();

			if staked_hez.is_zero() {
				return (0, Zero::zero());
			}

			// Amount-based tier scoring.
			let amount_score: u32 = if staked_hez <= 100u128.into() {
				20
			} else if staked_hez <= 250u128.into() {
				30
			} else if staked_hez <= 750u128.into() {
				40
			} else {
				50 // 751+ HEZ
			};

			// Duration-based multiplier.
			let (final_score, duration_for_return) = match StakingStartBlock::<T>::get(who) {
				Some(start_block) => {
					let current_block = pezframe_system::Pezpallet::<T>::block_number();
					let duration_in_blocks = current_block.saturating_sub(start_block);

					let score = if duration_in_blocks >= (12 * MONTH_IN_BLOCKS).into() {
						amount_score.saturating_mul(2) // x2.0 (12+ months)
					} else if duration_in_blocks >= (6 * MONTH_IN_BLOCKS).into() {
						amount_score.saturating_mul(17) / 10 // x1.7 (6-11 months)
					} else if duration_in_blocks >= (3 * MONTH_IN_BLOCKS).into() {
						amount_score.saturating_mul(14) / 10 // x1.4 (3-5 months)
					} else if duration_in_blocks >= MONTH_IN_BLOCKS.into() {
						amount_score.saturating_mul(12) / 10 // x1.2 (1-2 months)
					} else {
						amount_score // x1.0 (< 1 month)
					};

					(score, duration_in_blocks)
				},
				None => (amount_score, Zero::zero()),
			};

			(final_score.min(100), duration_for_return)
		}
	}
}
