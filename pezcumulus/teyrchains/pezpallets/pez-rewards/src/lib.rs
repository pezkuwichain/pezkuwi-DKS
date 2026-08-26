// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(not(feature = "std"), no_std)]

//! # PEZ Rewards
//!
//! The state's monthly payroll: it works out what each citizen is owed out of the incentive
//! pot, and what each parliamentary seat is owed, and instructs the chain that holds the
//! money to pay it.
//!
//! ## Where the money is, and where the arithmetic is
//!
//! The incentive pot lives on the Asset Hub, filled by `pezpallet-pez-treasury` out of the
//! monthly release. Everything that decides who is owed what -- the citizen register, trust
//! scores, the elected Parliament -- lives here on the People chain. Neither can be moved to
//! the other cheaply, so neither is: this pallet keeps a local ledger of what the pot has
//! been given and what has been drawn against it, and every payment goes out as an XCM
//! instruction to `pay_from_incentive_pot`.
//!
//! The pot's funding arrives the same way in reverse: the treasury reports a **running
//! total** after each release. A report that goes missing is repaired by the next one,
//! instead of leaving a month of rewards that nobody can ever claim.
//!
//! ## A frozen payroll
//!
//! An epoch is finalised in constant time. The denominator is `TotalActiveTrustScore`, which
//! the trust pallet already keeps and already proves; the numerator is read from each
//! claimant when they claim. Between those two moments the trust roll is **frozen**, so the
//! number used to compute the rate and the number used to compute a share are the same
//! number.
//!
//! That is what makes this cheap and fair at once. There is no per-citizen snapshot to write,
//! no registration to remember, and no advantage in claiming early or late -- there is
//! nothing a claimant can do between finalisation and their claim that changes what they get.
//!
//! The freeze carries its own expiry rather than a flag someone has to clear. If the epoch is
//! never closed, the roll thaws anyway when the claim window ends.
//!
//! ## Parliamentary seats
//!
//! Ten per cent of each epoch goes to Parliament, divided by the size of the house and not by
//! the number of people sitting in it. Dividing by the number sitting would make removing a
//! member profitable for the rest. An empty or forfeited seat simply goes unclaimed, and what
//! goes unclaimed stays in the pot for the following month.
//!
//! A seat is the `Parlementer` tiki and nothing else. `welati::ParliamentMembers` says who won
//! the seat -- it is what makes this a lookup over two hundred and one accounts instead of the
//! whole population -- but the tiki is what says whether they still hold it. A member the
//! Diwan has removed, or who has lost their citizenship, fails that check and is paid nothing,
//! whatever the roll still says.
//!
//! ## Citizenship
//!
//! There is no citizenship check here, and that is deliberate rather than an omission. When a
//! citizenship is revoked the trust pallet takes the score away, so a revoked citizen's share
//! is zero by arithmetic. A second check would be a second place for the same rule to live.

pub use pezpallet::*;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

extern crate alloc;

use alloc::vec;
use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::traits::Get;
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezsp_runtime::traits::{Saturating, Zero};
use scale_info::TypeInfo;
use xcm::latest::prelude::*;

/// `pay_from_incentive_pot` in the treasury pallet on the chain that holds the pot.
const PAY_FROM_INCENTIVE_POT_CALL_INDEX: u8 = 3;

/// The trust roll this pallet pays against.
///
/// Three questions, and one instruction. The instruction is the unusual one: the payroll is
/// only fair if the roll cannot move between the moment the rate is computed and the moment
/// a share is drawn against it, so this pallet asks the trust pallet to hold still.
pub trait TrustRoll<AccountId, BlockNumber> {
	/// What `who` is worth on the roll right now.
	fn score_of(who: &AccountId) -> u128;

	/// The sum of every citizen's score. The denominator of the whole payroll.
	fn total_score() -> u128;

	/// Stop recalculating scores until `until`.
	///
	/// Expiring rather than latching: a freeze that had to be lifted by a later call would
	/// leave the roll frozen for good the first time that call did not happen.
	fn freeze_until(until: BlockNumber);
}

/// Who sits in Parliament, and who actually holds the seat.
///
/// Two questions rather than one because they have two different answers. The roll is written
/// by the election and can go stale between elections; the seat is a tiki and is always
/// current. Payment needs both: the roll to know where to look, the seat to know whether to
/// pay.
pub trait ParliamentRoll<AccountId, BlockNumber> {
	/// When `who` was seated, if the current house's roll names them.
	fn seated_at(who: &AccountId) -> Option<BlockNumber>;

	/// Whether `who` holds a parliamentary seat at this moment.
	fn holds_seat(who: &AccountId) -> bool;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	/// One month at 10 blocks a minute -- the same period the treasury releases on, so an
	/// epoch is funded by exactly one release.
	pub const BLOCKS_PER_EPOCH: u32 = 432_000;

	/// One week to claim.
	pub const CLAIM_PERIOD_BLOCKS: u32 = 100_800;

	/// Seats in the house. The divisor of the parliamentary share, always -- see the module
	/// documentation for why it is not the number of members.
	pub const PARLIAMENT_SIZE: u32 = 201;

	/// Parliament's share of an epoch, as a percentage.
	pub const PARLIAMENTARY_REWARD_PERCENT: u128 = 10;

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
	pub trait Config: pezframe_system::Config + TypeInfo {
		type WeightInfo: crate::weights::WeightInfo;

		/// The trust roll: the denominator, each share, and the freeze.
		type TrustSource: TrustRoll<Self::AccountId, BlockNumberFor<Self>>;

		/// Who sits in Parliament and who holds a seat.
		type ParliamentSource: ParliamentRoll<Self::AccountId, BlockNumberFor<Self>>;

		/// Who may report what the incentive pot has been given.
		///
		/// The chain that holds the pot, and nothing else. Root is deliberately not accepted:
		/// a key that can report funding is a key that can conjure a payroll out of a pot
		/// that has no money in it.
		type FundingOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Sends payment instructions to the chain that holds the pot.
		type XcmSender: SendXcm;

		/// Where that chain is.
		type TreasuryChainLocation: Get<Location>;

		/// The treasury pallet's index on that chain.
		#[pezpallet::constant]
		type TreasuryPalletIndex: Get<u8>;

		/// Who may start the very first epoch, on a chain whose genesis did not.
		type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// Where the epoch clock is.
	#[pezpallet::storage]
	#[pezpallet::getter(fn epoch_info)]
	pub type EpochInfo<T: Config> = StorageValue<_, EpochData<T>, ValueQuery>;

	/// What each finalised epoch pays, per trust point and per seat.
	#[pezpallet::storage]
	#[pezpallet::getter(fn epoch_reward_pools)]
	pub type EpochRewardPools<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, EpochRewardPool<T>, OptionQuery>;

	/// What each claimant has already been paid, so nobody is paid twice.
	#[pezpallet::storage]
	#[pezpallet::getter(fn claimed_rewards)]
	pub type ClaimedRewards<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Blake2_128Concat,
		T::AccountId,
		u128,
		OptionQuery,
	>;

	/// Open, being claimed against, or done.
	#[pezpallet::storage]
	#[pezpallet::getter(fn epoch_status)]
	pub type EpochStatus<T: Config> = StorageMap<_, Blake2_128Concat, u32, EpochState, ValueQuery>;

	/// The one epoch currently open to claims, if any.
	///
	/// Kept so that closing an epoch does not have to search for it. There can only ever be
	/// one: the claim window is a week and an epoch is a month.
	#[pezpallet::storage]
	#[pezpallet::getter(fn epoch_in_claim)]
	pub type EpochInClaim<T: Config> = StorageValue<_, u32, OptionQuery>;

	/// Everything the incentive pot has been given, as last reported by the chain holding it.
	#[pezpallet::storage]
	#[pezpallet::getter(fn reported_incentive_total)]
	pub type ReportedIncentiveTotal<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// Everything this pallet has instructed to be paid out of it.
	///
	/// The difference between the two is what is left, and it is the only measure of the pot
	/// this chain has -- the balance itself is on the other side of a bridge.
	#[pezpallet::storage]
	#[pezpallet::getter(fn paid_out_total)]
	pub type PaidOutTotal<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct EpochData<T: Config> {
		pub current_epoch: u32,
		pub epoch_start_block: BlockNumberFor<T>,
		pub total_epochs_completed: u32,
	}

	#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
	pub struct EpochRewardPool<T: Config> {
		pub epoch_index: u32,
		/// What one trust point is worth this epoch.
		pub reward_per_trust_point: u128,
		/// What one parliamentary seat is worth this epoch.
		pub seat_share: u128,
		/// The block the roll was measured at. A seat taken after this is not paid for this
		/// epoch -- an election counted during the claim window must not pay the new house
		/// for the old house's month.
		pub finalized_at: BlockNumberFor<T>,
		pub claim_deadline: BlockNumberFor<T>,
	}

	#[derive(
		Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen, Default,
	)]
	pub enum EpochState {
		#[default]
		#[codec(index = 0)]
		Open,
		#[codec(index = 1)]
		ClaimPeriod,
		#[codec(index = 2)]
		Closed,
	}

	impl<T: Config> Default for EpochData<T> {
		fn default() -> Self {
			Self { current_epoch: 0, epoch_start_block: Zero::zero(), total_epochs_completed: 0 }
		}
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new epoch began.
		NewEpochStarted { epoch_index: u32, start_block: BlockNumberFor<T> },
		/// An epoch was finalised and is now open to claims.
		EpochFinalized {
			epoch_index: u32,
			available: u128,
			reward_per_trust_point: u128,
			seat_share: u128,
			claim_deadline: BlockNumberFor<T>,
		},
		/// A claim was paid.
		RewardClaimed { who: T::AccountId, epoch_index: u32, amount: u128 },
		/// An epoch's claim window closed. Whatever went unclaimed stayed in the pot.
		EpochClosed { epoch_index: u32 },
		/// The chain holding the pot reported what it has been given.
		IncentiveFundingNoted { total: u128 },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// The epoch clock has not been started.
		RewardsNotInitialized,
		/// It has already been started.
		AlreadyInitialized,
		/// Nothing has been calculated for that epoch.
		RewardPoolNotCalculated,
		/// That epoch is not open to claims.
		NotInClaimPeriod,
		/// The claim window for that epoch has closed.
		ClaimPeriodExpired,
		/// This account has already been paid for that epoch.
		RewardAlreadyClaimed,
		/// There is nothing owed.
		NoRewardToClaim,
		/// A funding report went backwards. The total is cumulative and cannot shrink.
		FundingReportWentBackwards,
		/// The payment instruction could not be sent to the chain holding the pot.
		CouldNotReachTreasury,
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

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		/// The payroll runs itself.
		///
		/// It can, now that finalising an epoch is constant work: there is no per-citizen
		/// loop to page over. Leaving it to an extrinsic somebody has to remember to call
		/// would mean the month's rewards quietly did not happen the month nobody called.
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut weight = T::DbWeight::get().reads(2);

			if let Some(epoch) = EpochInClaim::<T>::get() {
				if Self::claim_window_has_closed(epoch, n) {
					let _ = Self::do_close_epoch(epoch);
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(2, 3));
				}
			}

			if Self::epoch_is_due(n) {
				let _ = Self::do_finalize_epoch(n);
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(4, 6));
			}

			weight
		}

		/// What the pallet has paid must be what it has recorded paying, and it can never
		/// have paid out more than the pot was given.
		///
		/// The second of those is the one that matters on a fixed supply: this pallet cannot
		/// mint, but it can instruct a payment, and an instruction for money that is not
		/// there is a payment that fails on the far side of a bridge with nothing here to
		/// show for it.
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use pezframe_support::ensure;

			let reported = ReportedIncentiveTotal::<T>::get();
			let paid = PaidOutTotal::<T>::get();
			ensure!(paid <= reported, "more has been paid out than the pot was ever given");

			let mut summed = 0u128;
			for (epoch, _who, amount) in ClaimedRewards::<T>::iter() {
				let state = EpochStatus::<T>::get(epoch);
				ensure!(
					state != EpochState::Open,
					"a reward was paid for an epoch that is still collecting"
				);
				ensure!(
					EpochRewardPools::<T>::contains_key(epoch),
					"a reward was paid for an epoch that was never finalised"
				);
				summed = summed.saturating_add(amount);
			}
			ensure!(summed == paid, "the claims recorded do not add up to what was paid out");

			// One claim window at a time, and it is the one the pallet says it is. Two open
			// windows would let the same pot be promised twice.
			let mut in_claim = None;
			for (epoch, state) in EpochStatus::<T>::iter() {
				if state == EpochState::ClaimPeriod {
					ensure!(in_claim.is_none(), "two epochs are open to claims at once");
					in_claim = Some(epoch);
				}
			}
			ensure!(
				in_claim == EpochInClaim::<T>::get(),
				"the epoch marked as open to claims is not the one being claimed against"
			);

			if EpochInfo::<T>::exists() {
				let current = EpochInfo::<T>::get().current_epoch;
				ensure!(
					EpochStatus::<T>::get(current) == EpochState::Open,
					"the current epoch is not collecting"
				);
			}

			Ok(())
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Start the epoch clock on a chain whose genesis did not.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(<T as Config>::WeightInfo::initialize_rewards_system())]
		pub fn initialize_rewards_system(origin: OriginFor<T>) -> DispatchResult {
			<T as Config>::ForceOrigin::ensure_origin(origin)?;
			Self::do_initialize_rewards_system()
		}

		/// Draw what you are owed for a finalised epoch.
		///
		/// The only call a citizen makes. There is nothing to register beforehand: the roll
		/// is frozen between finalisation and the end of the window, so the share computed
		/// here is the share the rate was computed against, whenever it is drawn.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(<T as Config>::WeightInfo::claim_reward())]
		pub fn claim_reward(origin: OriginFor<T>, epoch_index: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_claim_reward(&who, epoch_index)
		}

		/// Record what the incentive pot has been given, in total, to date.
		///
		/// Cumulative rather than incremental, so a report that never arrives costs nothing
		/// but a month's delay.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(<T as Config>::WeightInfo::note_incentive_funding())]
		pub fn note_incentive_funding(
			origin: OriginFor<T>,
			#[pezpallet::compact] total: u128,
		) -> DispatchResult {
			T::FundingOrigin::ensure_origin(origin)?;
			ensure!(
				total >= ReportedIncentiveTotal::<T>::get(),
				Error::<T>::FundingReportWentBackwards
			);
			ReportedIncentiveTotal::<T>::put(total);
			Self::deposit_event(Event::IncentiveFundingNoted { total });
			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		pub fn do_initialize_rewards_system() -> DispatchResult {
			ensure!(!EpochInfo::<T>::exists(), Error::<T>::AlreadyInitialized);

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			EpochInfo::<T>::put(EpochData {
				current_epoch: 0,
				epoch_start_block: current_block,
				total_epochs_completed: 0,
			});
			EpochStatus::<T>::insert(0, EpochState::Open);

			Self::deposit_event(Event::NewEpochStarted {
				epoch_index: 0,
				start_block: current_block,
			});
			Ok(())
		}

		/// What the pot still holds, as far as this chain can tell.
		pub fn available_funds() -> u128 {
			ReportedIncentiveTotal::<T>::get().saturating_sub(PaidOutTotal::<T>::get())
		}

		fn epoch_is_due(now: BlockNumberFor<T>) -> bool {
			if !EpochInfo::<T>::exists() {
				return false;
			}
			let epoch_data = EpochInfo::<T>::get();
			if EpochStatus::<T>::get(epoch_data.current_epoch) != EpochState::Open {
				return false;
			}
			now.saturating_sub(epoch_data.epoch_start_block) >= BLOCKS_PER_EPOCH.into()
		}

		fn claim_window_has_closed(epoch: u32, now: BlockNumberFor<T>) -> bool {
			match EpochRewardPools::<T>::get(epoch) {
				Some(pool) => now > pool.claim_deadline,
				None => false,
			}
		}

		/// Fix the rate for the epoch that has just ended, and freeze the roll it was
		/// measured against.
		///
		/// Constant work. The denominator is a single value the trust pallet already keeps
		/// and already proves correct; nothing is written per citizen.
		pub fn do_finalize_epoch(now: BlockNumberFor<T>) -> DispatchResult {
			let mut epoch_data = EpochInfo::<T>::get();
			let epoch = epoch_data.current_epoch;

			let available = Self::available_funds();
			let claim_deadline = now.saturating_add(CLAIM_PERIOD_BLOCKS.into());

			// An epoch with nothing behind it still has to end, or the clock stops and every
			// later month is lost with it. It simply pays nothing.
			let (reward_per_trust_point, seat_share) = if available.is_zero() {
				(0, 0)
			} else {
				let seat_pool =
					available.saturating_mul(PARLIAMENTARY_REWARD_PERCENT).saturating_div(100);
				let seat_share = seat_pool.saturating_div(PARLIAMENT_SIZE as u128);
				let citizen_pool = available.saturating_sub(seat_pool);
				let total = T::TrustSource::total_score();
				let rate = if total.is_zero() { 0 } else { citizen_pool.saturating_div(total) };
				(rate, seat_share)
			};

			EpochRewardPools::<T>::insert(
				epoch,
				EpochRewardPool {
					epoch_index: epoch,
					reward_per_trust_point,
					seat_share,
					finalized_at: now,
					claim_deadline,
				},
			);
			EpochStatus::<T>::insert(epoch, EpochState::ClaimPeriod);
			EpochInClaim::<T>::put(epoch);

			// Hold the roll still for as long as it can be drawn against. Everyone is then
			// measured by the same ruler at the same instant, whenever they get round to
			// claiming, and there is nothing to gain by claiming at any particular moment.
			T::TrustSource::freeze_until(claim_deadline);

			let new_epoch = epoch.saturating_add(1);
			epoch_data.current_epoch = new_epoch;
			epoch_data.epoch_start_block = now;
			epoch_data.total_epochs_completed = epoch_data.total_epochs_completed.saturating_add(1);
			EpochInfo::<T>::put(epoch_data);
			EpochStatus::<T>::insert(new_epoch, EpochState::Open);

			Self::deposit_event(Event::EpochFinalized {
				epoch_index: epoch,
				available,
				reward_per_trust_point,
				seat_share,
				claim_deadline,
			});
			Self::deposit_event(Event::NewEpochStarted {
				epoch_index: new_epoch,
				start_block: now,
			});
			Ok(())
		}

		/// What `who` is owed for `epoch`, without paying it.
		pub fn entitlement(who: &T::AccountId, epoch: u32) -> u128 {
			let pool = match EpochRewardPools::<T>::get(epoch) {
				Some(pool) => pool,
				None => return 0,
			};

			let mut amount =
				pool.reward_per_trust_point.saturating_mul(T::TrustSource::score_of(who));

			// A seat is only paid if it was held when the roll was measured and is still held
			// now. The first keeps a house elected during the claim window from being paid
			// for the previous house's month; the second is what makes removal by the Diwan,
			// or a lost citizenship, stop the salary the same day.
			if T::ParliamentSource::holds_seat(who) {
				if let Some(seated_at) = T::ParliamentSource::seated_at(who) {
					if seated_at <= pool.finalized_at {
						amount = amount.saturating_add(pool.seat_share);
					}
				}
			}

			amount
		}

		pub fn do_claim_reward(who: &T::AccountId, epoch: u32) -> DispatchResult {
			let now = pezframe_system::Pezpallet::<T>::block_number();
			let pool =
				EpochRewardPools::<T>::get(epoch).ok_or(Error::<T>::RewardPoolNotCalculated)?;

			ensure!(
				EpochStatus::<T>::get(epoch) == EpochState::ClaimPeriod,
				Error::<T>::NotInClaimPeriod
			);
			ensure!(now <= pool.claim_deadline, Error::<T>::ClaimPeriodExpired);
			ensure!(
				!ClaimedRewards::<T>::contains_key(epoch, who),
				Error::<T>::RewardAlreadyClaimed
			);

			let amount = Self::entitlement(who, epoch);
			ensure!(!amount.is_zero(), Error::<T>::NoRewardToClaim);

			// Recorded before the instruction goes out. If the send fails the whole call
			// reverts, so the record and the money cannot disagree; recording afterwards
			// would leave a moment where the pot had been drawn on and nothing said so.
			ClaimedRewards::<T>::insert(epoch, who, amount);
			PaidOutTotal::<T>::put(PaidOutTotal::<T>::get().saturating_add(amount));

			Self::send_payment(who, amount).map_err(|_| Error::<T>::CouldNotReachTreasury)?;

			Self::deposit_event(Event::RewardClaimed {
				who: who.clone(),
				epoch_index: epoch,
				amount,
			});
			Ok(())
		}

		/// Close a claim window. Nothing moves: what was not claimed never left the pot, and
		/// is available again the following month.
		pub fn do_close_epoch(epoch: u32) -> DispatchResult {
			ensure!(
				EpochStatus::<T>::get(epoch) == EpochState::ClaimPeriod,
				Error::<T>::NotInClaimPeriod
			);
			EpochStatus::<T>::insert(epoch, EpochState::Closed);
			EpochInClaim::<T>::kill();
			Self::deposit_event(Event::EpochClosed { epoch_index: epoch });
			Ok(())
		}

		/// Ask the chain holding the pot to pay `amount` to `who`.
		///
		/// Unpaid, like every other message between these two system chains: a reward the
		/// state owes a citizen should not be lost because a sovereign account was short of
		/// fees.
		fn send_payment(who: &T::AccountId, amount: u128) -> Result<(), SendError> {
			let call = (
				T::TreasuryPalletIndex::get(),
				PAY_FROM_INCENTIVE_POT_CALL_INDEX,
				who,
				codec::Compact(amount),
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
				&mut Some(T::TreasuryChainLocation::get()),
				&mut Some(message),
			)?;
			T::XcmSender::deliver(ticket)?;
			Ok(())
		}
	}
}

// ===== STORED ENUM ENCODING =====
//
// SCALE encodes a fieldless enum by the variant's position, and three of these are storage
// keys. Insert a variant in the middle -- grouping by ministry, or alphabetising, is the most
// natural thing anyone would do -- and every key already written decodes as a different
// value. It does not break; it quietly means something else. A judge becomes a treasurer.
//
// The explicit indices pin the number to the variant rather than to its position, and this
// holds those numbers to what they were when the chain started. A variant may be added at the
// end with the next free number; nothing here may be renumbered, and a number left behind by
// a removed variant is not reusable.
//
// Generating those indices is itself the hazard this guards against: the first attempt lost
// nineteen variants whose names carry Kurdish letters and silently shifted everything after
// them. Two of the shifts collided and the codec derive refused to compile; the rest would
// have gone through.

#[cfg(test)]
mod stored_enum_encoding {
	use super::*;
	use codec::Encode;

	#[test]
	fn epochstate_indices_are_pinned() {
		let pinned: &[(&str, u8, &dyn Fn() -> Vec<u8>)] = &[
			("Open", 0u8, &|| pezpallet::EpochState::Open.encode()),
			("ClaimPeriod", 1u8, &|| pezpallet::EpochState::ClaimPeriod.encode()),
			("Closed", 2u8, &|| pezpallet::EpochState::Closed.encode()),
		];
		let moved: Vec<&str> = pinned
			.iter()
			.filter(|(_, want, enc)| enc() != vec![*want])
			.map(|(name, _, _)| *name)
			.collect();
		assert!(moved.is_empty(), "`EpochState` indices moved: {moved:?}");
		assert_eq!(pinned.len(), 3, "a variant was added or removed");
	}
}
