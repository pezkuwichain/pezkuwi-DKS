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
//! call `slash_noter()` to hand the noter's bond to `T::SlashDestination`. Nothing is burnt.
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
//!
//! ## Freshness
//!
//! Every time data for a source becomes effective -- changed or not -- `LastConfirmed`
//! records the block. The duration multiplier counts only up to the newest confirmation plus
//! `MaxDataAge`: when confirmations stop, the multiplier holds where it was instead of
//! climbing on data nobody is checking. Noters confirm unchanged stake on a schedule shorter
//! than `MaxDataAge`, so this is reached only when they stop.

pub use pezpallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Live proofs, for tests here and in the runtimes, and for the benchmark.
#[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
pub mod hub_fixture;
pub mod hub_proof;
pub mod migrations;
pub mod weights;

// Re-exported so the generated runtime weights can name it.
//
// `pezframe-omni-bencher` writes `impl<T> pezpallet_{name}::WeightInfo for WeightInfo<T>` -- the
// crate root, never the module. Without this line the generated file does not compile and the
// only way to use it was to edit the path by hand after every regeneration, which is a patch
// that silently disappears the next time the weights are taken. Measured 2026-09-21.
pub use weights::WeightInfo;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::weights::WeightInfo;
	use codec::Encode;
	use core::ops::Div;
	use pezframe_support::{
		pezpallet_prelude::*,
		traits::{BalanceStatus, Currency, OnUnbalanced, ReservableCurrency},
	};
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::traits::{One, Saturating, Zero};
	use pezsp_std::vec::Vec;

	// --- Constants ---
	//
	// Block-time assumption: 10 blocks/minute (6s blocks), matching this
	// chain's actual configured slot duration (`testnet_teyrchains_constants::
	// pezkuwichain::consensus::MILLISECS_PER_BLOCK = 6_000` — the module
	// `people-pezkuwichain` actually imports its `HOURS`/`DAYS`/`SlotDuration`
	// from; `teyrchains_common::MILLISECS_PER_BLOCK = 12_000` is a
	// same-named but unrelated, unused-for-timing constant in a different
	// crate and does not govern this runtime's real block cadence). An
	// earlier revision of this file assumed the 12s figure and halved every
	// duration-tier threshold below (confirmed against live mainnet state via
	// try-runtime and empirical block-time measurement before it shipped). If
	// this chain's block time ever changes, these two constants need
	// updating to match — they are not derived automatically.
	pub const MONTH_IN_BLOCKS: u32 = 30 * 24 * 60 * 10;
	pub const HOUR_IN_BLOCKS: u32 = 60 * 10;

	/// The most proof a stake needs: a relay path to the hub's head and a hub path to a bond,
	/// a pool and its account. Measured at about 6 KiB on Zagros; a large state deepens the
	/// tries, not by an order of magnitude.
	pub const MAX_PROOF_KIB: u32 = 64;
	pub const PRUNE_RELAY_ROOTS_PER_BLOCK: u32 = 4;

	/// The size of a stake proof, in whole KiB, which is what its weight is measured by.
	pub fn proof_kib(relay_proof: &[Vec<u8>], hub_proof: &[Vec<u8>]) -> u32 {
		let bytes: usize = relay_proof.iter().chain(hub_proof).map(|n| n.len()).sum();
		bytes.div_ceil(1024).try_into().unwrap_or(u32::MAX)
	}

	/// The months at which a stake's duration multiplier steps up.
	pub const TIER_MONTHS: [u32; 4] = [1, 3, 6, 12];

	/// The most accounts told in one block that their stake has reached a longer tier, and
	/// the most blocks looked through to find them. Each is a read, a removal and one write
	/// on the listener's side: 250 is a few per cent of a block.
	pub const MAX_TIER_CROSSINGS_PER_BLOCK: u32 = 250;
	pub const MAX_CROSSING_BLOCKS_SCANNED: u32 = 16;
	pub const UNITS: u128 = 1_000_000_000_000;

	/// The ceiling the score is clamped to: the largest amount tier (50) at the longest
	/// duration multiplier (x2). Named rather than written twice, because it is both the cap
	/// applied at the end of the calculation and the maximum this component reports to trust.
	pub const MAX_STAKING_SCORE: RawScore = 100;

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
		#[codec(index = 0)]
		RelayChain = 0,
		/// Staking via nomination pools on Asset Hub.
		#[codec(index = 1)]
		AssetHub = 1,
	}

	/// The version this pallet's storage layout is at.
	///
	/// Declared so that the first migration has a baseline to compare against. Without it the
	/// in-code and on-chain versions are both an implicit zero, and a migration cannot tell a
	/// chain that has never been migrated from one that has been migrated to zero.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T>
	where
		BlockNumberFor<T>: From<u32>,
	{
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			// `on_finalize` below: the relay parent and the cursor read, the root and the cursor
			// written, and up to `PRUNE_RELAY_ROOTS_PER_BLOCK` old roots removed.
			Self::announce_tier_crossings(now).saturating_add(
				T::DbWeight::get().reads_writes(2, 2 + u64::from(PRUNE_RELAY_ROOTS_PER_BLOCK)),
			)
		}

		/// Keep this block's relay parent root. The validation data is set by an inherent
		/// after `on_initialize`, so it is read here.
		fn on_finalize(_now: BlockNumberFor<T>) {
			Self::keep_relay_root();
		}

		/// What the pallet has recorded about noters and stake has to hold together.
		///
		/// This is the one input to the trust score that the People chain cannot see for
		/// itself -- it is told, by bonded accounts, about state on another chain. The bond
		/// and the dispute window are what make being told acceptable, so a registration
		/// without a bond behind it, or a pending submission from somebody who is no longer a
		/// registered noter, is the assurance quietly not being there.
		#[cfg(feature = "try-runtime")]
		fn try_state(_n: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use pezframe_support::ensure;

			for (noter, bond) in NoterBonds::<T>::iter() {
				ensure!(
					T::Currency::reserved_balance(&noter) >= bond,
					"a noter's bond is recorded but not actually reserved"
				);
			}

			for (who, source, pending) in PendingStakingDetails::<T>::iter() {
				ensure!(
					NoterBonds::<T>::contains_key(Self::noter_answering_for(&pending.submitted_by)),
					"a pending submission is from somebody who is not a bonded noter"
				);
				ensure!(
					pending.submitted_by != who,
					"a noter submitted staking data about themselves"
				);
				let _ = source;
			}

			// The hub speaks for a stake that is there. A mark with nothing behind it would keep
			// a clock current on data that does not exist.
			for who in ReportedByChain::<T>::iter_keys() {
				ensure!(
					CachedStakingDetails::<T>::contains_key(&who, StakingSource::AssetHub),
					"the hub is marked as reporting a stake that is not recorded"
				);
			}

			// A crossing behind the cursor would never be announced.
			if let Some(next) = NextCrossingBlock::<T>::get() {
				for (at, _, ()) in TierCrossings::<T>::iter() {
					ensure!(at >= next, "a tier crossing was left behind by the cursor");
				}
			}

			// A confirmation vouches for a stake. One with no stake behind it would keep an
			// account's clock running on data that is not there.
			for (who, source, _) in LastConfirmed::<T>::iter() {
				ensure!(
					CachedStakingDetails::<T>::contains_key(&who, source),
					"a stake confirmation outlived the stake it confirms"
				);
			}

			Ok(())
		}
	}

	/// Where this block's relay parent is read from.
	pub trait RelayParentProvider {
		/// The relay block this block is built on and its storage root, once the block's
		/// validation data is in; `None` before.
		fn relay_parent() -> Option<(u32, pezsp_core::H256)>;
	}

	impl RelayParentProvider for () {
		fn relay_parent() -> Option<(u32, pezsp_core::H256)> {
			None
		}
	}

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

	/// What a slash produces: value taken out of an account and not yet given to anyone.
	/// Whoever handles it decides where it lands, and dropping it would destroy it.
	pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as pezframe_system::Config>::AccountId,
	>>::NegativeImbalance;

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

		/// What becomes of a slashed noter's bond.
		///
		/// A handler rather than an account, because on a teyrchain the treasury usually lives
		/// on another chain and reaching it takes a teleport, which an address cannot do. This
		/// was a bare `Get<AccountId>` pointing at the relay treasury's derived address, an
		/// address with no pallet behind it on this side: the bond was not burnt, it was
		/// parked somewhere nobody can spend from.
		type SlashDestination: OnUnbalanced<NegativeImbalanceOf<Self>>;

		/// How much of the gap between opting in and the stake being recorded is forgiven.
		///
		/// Staking data arrives through a bot and a noter, so there is always a delay between
		/// a user asking for their time to be counted and the chain being able to see any
		/// stake. That delay is the system's, not the user's, and this is how much of it the
		/// user is not charged for.
		///
		/// A constant of its own rather than a multiple of `DisputeWindow`: the real delay is
		/// how often the bot runs plus the window it then waits out, and those are two
		/// different things that would not move together.
		#[pezpallet::constant]
		type OracleGracePeriod: Get<BlockNumberFor<Self>>;

		/// How long stake data keeps earning time after it was last confirmed.
		///
		/// The duration multiplier is a claim that a stake has been held, and the chain only
		/// knows that from what a noter or the relay last told it. Counting on from there
		/// indefinitely meant a bot that stopped left every score climbing towards the
		/// twelve-month multiplier on data nobody was checking any more -- including for
		/// accounts that had since unstaked. Past this age the clock stops at the last
		/// confirmation plus this allowance; the next confirmation starts it again, with
		/// nothing lost.
		///
		/// Noters confirm unchanged stake on a schedule shorter than this, so a healthy
		/// account never reaches it. It is reached only when confirmations stop.
		#[pezpallet::constant]
		type MaxDataAge: Get<BlockNumberFor<Self>>;

		/// The relay block this block is built on, and that block's storage root.
		///
		/// A stake proof hangs from a relay root, so the chain has to know the roots it may
		/// hang from. They are kept for `RelayRootsKept` relay blocks; see `RelayRoots`.
		type RelayParent: RelayParentProvider;

		/// How many relay blocks back a stake proof may be anchored.
		#[pezpallet::constant]
		type RelayRootsKept: Get<u32>;

		/// The Asset Hub's teyrchain id: whose head in the relay a proof reads.
		#[pezpallet::constant]
		type AssetHubParaId: Get<u32>;

		/// The hub's nomination-pools pallet id, from which a pool's bonded account derives.
		#[pezpallet::constant]
		type HubPoolsPalletId: Get<pezframe_support::PalletId>;
	}

	/// Balance type of `T::Currency` (the noter bond currency) — distinct from
	/// `T::Balance` above, which represents staking amounts, not reservable funds.
	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as pezframe_system::Config>::AccountId>>::Balance;

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

	/// The account a noter's standing moved to when its key was reissued.
	///
	/// A pending submission names the key that made it, and a dispute against it is charged
	/// to whoever answers for that key now -- otherwise the successor holds the bond while
	/// the retired key, which nobody can sign for, takes the dispute.
	#[pezpallet::storage]
	pub type NoterSucceededBy<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

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

	/// The block a non-zero stake was first recorded for an account.
	///
	/// The duration multiplier is anchored to when the user opted in, so that a slow bot costs
	/// them nothing. On its own that let anyone opt in on an empty account, wait a year and
	/// then stake, collecting the twelve-month multiplier the moment the funds landed. This is
	/// the other end of the measurement: time is credited from the opt-in, but never more of
	/// it than the stake has actually existed for, plus the grace the delay is worth.
	///
	/// Cleared with the stake, so somebody who unstakes entirely and returns years later
	/// starts again rather than carrying the old date.
	#[pezpallet::storage]
	#[pezpallet::getter(fn stake_first_seen)]
	pub type StakeFirstSeen<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

	/// The block each source's stake was last confirmed at -- written whenever data for it
	/// becomes effective, changed or not.
	///
	/// This is what `MaxDataAge` is measured from. Cleared with the stake it vouches for.
	#[pezpallet::storage]
	#[pezpallet::getter(fn last_confirmed)]
	pub type LastConfirmed<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		StakingSource,
		BlockNumberFor<T>,
		OptionQuery,
	>;

	/// Who reaches a longer duration tier at a block, by that block.
	///
	/// A tier is reached by time passing and by nothing else, so nothing announces it: the
	/// multiplier is worked out when it is read, and a score derived from it -- trust -- keeps
	/// the old value until something makes it look again. The dates are known the moment the
	/// clock starts, so they are written down then and announced when they come.
	#[pezpallet::storage]
	pub type TierCrossings<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		Blake2_128Concat,
		T::AccountId,
		(),
		OptionQuery,
	>;

	/// The block `who`'s duration is counted from, as of the last time their crossings were
	/// written down. A confirmation that changes nothing writes nothing.
	#[pezpallet::storage]
	pub type CrossingsScheduledFrom<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

	/// The first block whose tier crossings are still to be announced.
	#[pezpallet::storage]
	pub type NextCrossingBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// Accounts that have asked for their staking time to be counted.
	///
	/// Separate from the block the clock starts at. Opting in says "count my duration"; the
	/// clock itself starts when there is stake to count -- otherwise a person could opt in on
	/// day one, stake a year later, and collect the twelve-month multiplier the moment the
	/// funds arrived. The multiplier is for having kept a stake, not for having asked early.
	#[pezpallet::storage]
	#[pezpallet::getter(fn tracking_opted_in)]
	pub type TrackingOptIn<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	/// The relay storage roots a stake proof may hang from, by relay block.
	///
	/// This chain only ever sees its current relay parent's root, and a proof is built against
	/// some recent relay block, not necessarily the one this block lands on. So each block's
	/// relay parent is kept for `RelayRootsKept` relay blocks.
	#[pezpallet::storage]
	pub type RelayRoots<T: Config> =
		StorageMap<_, Twox64Concat, u32, pezsp_core::H256, OptionQuery>;

	/// The oldest relay block still in `RelayRoots`, so pruning walks forward from it.
	#[pezpallet::storage]
	pub type OldestRelayRoot<T: Config> = StorageValue<_, u32, OptionQuery>;

	/// The relay block `who`'s stake was last proven at. A proof has to be newer: an older
	/// one is a true statement about the past, and showing it again would undo a later one.
	#[pezpallet::storage]
	pub type ProvenAt<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u32, OptionQuery>;

	/// Accounts whose stake has been proven from the hub's own state.
	///
	/// A change on the hub can be proven here by anyone, free of charge, so for these accounts
	/// no proof means no change: their duration clock is current without anybody re-confirming
	/// unchanged stake. A noter's three-day confirmations were what kept every other clock
	/// running, and at forty million they do not fit in the chain.
	#[pezpallet::storage]
	pub type ReportedByChain<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	/// How many times a pending update for this account and source has been frozen.
	///
	/// Read as an escalation: the first objection is one member's to make, and the second is
	/// not. Without it a single member could freeze the same account's data on every
	/// resubmission, for ever and at no cost -- and an account whose stake never lands scores
	/// zero, which is a trust score of zero, which is no candidacy for any office at all. One
	/// person should not be able to do that to a citizen quietly.
	#[pezpallet::storage]
	#[pezpallet::getter(fn dispute_count)]
	pub type DisputeCount<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		StakingSource,
		u32,
		ValueQuery,
	>;

	/// How many of each noter's submissions have been frozen.
	///
	/// Evidence rather than punishment. Slashing is a governance decision after review, and
	/// this is part of what there is to review -- one disputed submission is an accusation, a
	/// pattern of them is something else.
	#[pezpallet::storage]
	#[pezpallet::getter(fn disputes_against)]
	pub type DisputesAgainstNoter<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// Disputes against each noter that governance has not yet ruled on.
	///
	/// The bond is what a disputed submission is answered from, so it stays reserved until
	/// they are ruled on: slashed, or dismissed. It used to be withdrawable an hour after the
	/// submission -- before any review could have happened (L-7, 2026-10-21).
	#[pezpallet::storage]
	pub type UnresolvedDisputes<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	/// A stake that went to zero, kept for `OracleGracePeriod` in case it was not real.
	///
	/// (opt-in start, first seen, total before the lapse, lapsed at). A zero used to erase the
	/// holding duration at once and for good, so one false zero that nobody disputed within
	/// the hour cost a citizen years of multiplier, and a correction a few minutes later could
	/// not give it back (L-7, 2026-10-21).
	#[pezpallet::storage]
	pub type LapsedStake<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		(Option<BlockNumberFor<T>>, Option<BlockNumberFor<T>>, T::Balance, BlockNumberFor<T>),
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
		StakingDetailsDisputed {
			who: T::AccountId,
			source: StakingSource,
			disputed_by: T::AccountId,
		},
		/// An account registered as an active (bonded) noter.
		NoterRegistered { who: T::AccountId, bond: BalanceOf<T> },
		/// An account unregistered as noter and reclaimed its bond.
		NoterUnregistered { who: T::AccountId, bond_returned: BalanceOf<T> },
		/// A noter's bond was slashed by governance after a confirmed fraudulent
		/// submission.
		NoterSlashed { who: T::AccountId, amount: BalanceOf<T> },
		/// Governance ruled a noter's open disputes unfounded; the bond is free again.
		NoterDisputesDismissed { who: T::AccountId, dismissed: u32 },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// A noter may not submit staking data about themselves.
		NoterCannotAttestSelf,
		/// This account's data has been frozen before; freezing it again is governance's.
		RepeatDisputeNeedsGovernance,
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
		/// A dispute against this noter is awaiting governance's ruling; the bond stays.
		DisputeUnresolved,
		/// Different data is already pending for this account and source; it matures or is
		/// disputed first.
		SubmissionAwaitingWindow,
		/// No disputes against this noter are awaiting a ruling.
		NoUnresolvedDisputes,
		/// The relay block is not one this chain keeps a root for: too old, or not yet seen.
		UnknownRelayBlock,
		/// A stake proof as new or newer has already been accepted for this account.
		ProofNotNewer,
		/// The proof does not hash to the root, or does not cover what it has to.
		InvalidProof,
		/// More proof than a stake needs.
		ProofTooLarge,
		/// Stake is counted only for an account that asked for it.
		NotTracking,
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
			TrackingOptIn::<T>::insert(&who, ());
			// The clock starts here rather than when the data lands, deliberately: staking
			// details arrive through a noter, asynchronously, and a slow bot should not cost
			// the user duration they actually held the stake for. See
			// `duration_counts_from_optin_not_from_data_arrival`.
			StakingStartBlock::<T>::insert(&who, current_block);
			Self::schedule_tier_crossings(&who);

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
			// A notary does not notarise their own deed. The bond makes a false submission
			// expensive, but it does not make it improper to be the only witness to your own
			// stake -- and this is the one submission a noter has a direct interest in.
			ensure!(caller != who, Error::<T>::NoterCannotAttestSelf);

			// Opportunistically finalize a previously-matured pending entry for
			// this (who, source) before recording the new candidate. Without this,
			// a steady stream of legitimate noter updates would keep resetting the
			// dispute window and the account's effective stake would never
			// progress past whatever Root last committed (often zero).
			Self::maybe_finalize(&who, source);

			let details =
				StakingDetails { staked_amount, nominations_count, unlocking_chunks_count };

			// A submission still in its window is not replaced. Each replacement used to
			// restart the window, so a noter resubmitting every fifty-nine minutes -- even the
			// right numbers -- kept an account's data from ever taking effect, and anyone else's
			// submission was overwritten on the way (L-7, 2026-10-21). The same data again is
			// the bot confirming what it already sent: accepted, and the window left running.
			if let Some(pending) = PendingStakingDetails::<T>::get(&who, source) {
				if pending.details == details {
					return Ok(());
				}
				return Err(Error::<T>::SubmissionAwaitingWindow.into());
			}

			let current_block = pezframe_system::Pezpallet::<T>::block_number();
			NoterLastSubmission::<T>::insert(&caller, current_block);
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
			ensure!(UnresolvedDisputes::<T>::get(&who) == 0, Error::<T>::DisputeUnresolved);

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
			let already = DisputeCount::<T>::get(&who, source);

			// The first objection is one member's to make. Repeating it is not.
			//
			// Freezing discards the candidate, so a noter has to resubmit -- and a member who
			// could freeze every resubmission for nothing would keep that account's stake at
			// zero indefinitely. Zero stake is a trust score of zero, and a trust score of
			// zero is no candidacy for any office. One member should not be able to remove a
			// citizen from public life by repeating a free action.
			let disputed_by = if already == 0 {
				T::DisputeOrigin::ensure_origin(origin)?
			} else {
				T::SlashOrigin::ensure_origin(origin.clone())
					.map_err(|_| Error::<T>::RepeatDisputeNeedsGovernance)?;
				// The stronger origin is collective and has no single account behind it, so
				// the event records whoever signed for it where there is one.
				ensure_signed(origin).unwrap_or_else(|_| who.clone())
			};

			let pending = PendingStakingDetails::<T>::get(&who, source)
				.ok_or(Error::<T>::NoPendingSubmission)?;

			PendingStakingDetails::<T>::remove(&who, source);
			DisputeCount::<T>::insert(&who, source, already.saturating_add(1));
			let noter = Self::noter_answering_for(&pending.submitted_by);
			DisputesAgainstNoter::<T>::mutate(&noter, |n| *n = n.saturating_add(1));
			UnresolvedDisputes::<T>::mutate(&noter, |n| *n = n.saturating_add(1));

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

			let pending = PendingStakingDetails::<T>::get(&who, source)
				.ok_or(Error::<T>::NoPendingSubmission)?;
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
			UnresolvedDisputes::<T>::remove(&noter);

			T::Currency::unreserve(&noter, bond);
			let (slashed, _) = T::Currency::slash(&noter, bond);
			T::SlashDestination::on_unbalanced(slashed);

			Self::deposit_event(Event::NoterSlashed { who: noter, amount: bond });
			Ok(())
		}

		/// Rule a noter's open disputes unfounded, freeing the bond they held in place.
		///
		/// The other ruling is `slash_noter`. Same origin: whoever may find a submission
		/// fraudulent may find that it was not. Charged as `slash_noter`, which does strictly
		/// more storage work.
		#[pezpallet::call_index(7)]
		#[pezpallet::weight(T::WeightInfo::slash_noter())]
		pub fn dismiss_disputes(origin: OriginFor<T>, noter: T::AccountId) -> DispatchResult {
			T::SlashOrigin::ensure_origin(origin)?;
			let dismissed = UnresolvedDisputes::<T>::take(&noter);
			ensure!(dismissed != 0, Error::<T>::NoUnresolvedDisputes);
			Self::deposit_event(Event::NoterDisputesDismissed { who: noter, dismissed });
			Ok(())
		}

		/// Prove `who`'s stake from the Asset Hub's own state.
		///
		/// `relay_proof` reaches the hub's head under the root of `relay_block`, which this
		/// chain kept; `hub_proof` reaches `who`'s bond, nominations and pool membership under
		/// that head's state root. Nobody is believed and nobody is authorised: anyone may
		/// submit, and a proof either hashes to the root or is refused. The register is not
		/// written from abroad -- it reads what is there.
		///
		/// Free when it changes what is recorded, charged when it does not. A change is worth
		/// telling, so anybody can afford to tell it -- including a fall in somebody else's
		/// stake, which its owner would not hurry to report; repeating what is already known is
		/// what a fee is for.
		#[pezpallet::call_index(8)]
		#[pezpallet::weight(T::WeightInfo::prove_stake(proof_kib(relay_proof, hub_proof)))]
		pub fn prove_stake(
			origin: OriginFor<T>,
			who: T::AccountId,
			relay_block: u32,
			relay_proof: Vec<Vec<u8>>,
			hub_proof: Vec<Vec<u8>>,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			ensure!(
				proof_kib(&relay_proof, &hub_proof) <= MAX_PROOF_KIB,
				Error::<T>::ProofTooLarge
			);
			ensure!(TrackingOptIn::<T>::contains_key(&who), Error::<T>::NotTracking);
			let relay_root =
				RelayRoots::<T>::get(relay_block).ok_or(Error::<T>::UnknownRelayBlock)?;
			ensure!(
				ProvenAt::<T>::get(&who).map_or(true, |at| relay_block > at),
				Error::<T>::ProofNotNewer
			);

			let account: [u8; 32] =
				who.encode().try_into().map_err(|_| Error::<T>::InvalidProof)?;
			let hub_root = crate::hub_proof::hub_state_root(
				relay_root,
				&relay_proof,
				T::AssetHubParaId::get(),
			)
			.map_err(|_| Error::<T>::InvalidProof)?;
			let stake = crate::hub_proof::hub_stake(
				hub_root,
				&hub_proof,
				&account,
				T::HubPoolsPalletId::get(),
			)
			.map_err(|_| Error::<T>::InvalidProof)?;

			ProvenAt::<T>::insert(&who, relay_block);
			let staked: T::Balance = stake.staked.into();
			let proven = StakingDetails {
				staked_amount: staked,
				nominations_count: stake.nominations,
				unlocking_chunks_count: stake.unlocking_chunks,
			};
			let changed = CachedStakingDetails::<T>::get(&who, StakingSource::AssetHub)
				.map_or(!staked.is_zero(), |recorded| recorded != proven);
			if !staked.is_zero() {
				ReportedByChain::<T>::insert(&who, ());
			}
			if !changed {
				return Ok(Pays::Yes.into());
			}
			// What the hub's state says supersedes what a noter said it would.
			PendingStakingDetails::<T>::remove(&who, StakingSource::AssetHub);
			Self::apply_staking_update(
				&who,
				StakingSource::AssetHub,
				staked,
				stake.nominations,
				stake.unlocking_chunks,
			);
			Ok(Pays::No.into())
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
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, TypeInfo, Debug, MaxEncodedLen,
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

		/// The most this component can ever report.
		///
		/// Declared here rather than assumed by the reader: trust weights its inputs as
		/// percentages, and a percentage of an unknown range is not a percentage.
		fn max_score() -> RawScore;
	}

	/// Callback trait for when staking data changes.
	/// Trust pallet implements this to recalculate scores on staking updates.
	pub trait OnStakingDataUpdate<AccountId> {
		fn on_staking_data_changed(who: &AccountId);

		/// `who`'s stake has reached a longer duration tier. Nothing about it changed; the
		/// time did. Announced from `on_initialize`, up to `MAX_TIER_CROSSINGS_PER_BLOCK` a
		/// block, and charged as one write: a listener should note it and do the work in its
		/// own budget, not here.
		fn on_staking_tier_reached(who: &AccountId) {
			Self::on_staking_data_changed(who);
		}

		/// Put `who` where the callback does its full work, for benchmarking only. Without it
		/// a listener that bails out early for most accounts -- the trust pallet skips anyone
		/// who is not a citizen -- is measured on its cheap path, and every call that notifies
		/// it is undercharged. Default no-op; a listener with such a gate should override it.
		#[cfg(feature = "runtime-benchmarks")]
		fn prepare_worst_case(_who: &AccountId) {}
	}

	impl<AccountId> OnStakingDataUpdate<AccountId> for () {
		fn on_staking_data_changed(_who: &AccountId) {}
	}

	// --- Helpers ---

	impl<T: Config> Pezpallet<T> {
		/// Move what a court-ordered reissue may honestly move, and no more.
		///
		/// **The observed stake deliberately does not move, and this is the one place a
		/// reissue leaves something behind.** Everything else a reissue carries is a record of
		/// what a person did -- a certificate earned, a citizen vouched for, an office won --
		/// and those are facts about the person whatever key wrote them down. Cached stake is
		/// not that. It is this chain's belief about funds bonded on another one, and those
		/// funds are bonded to the key that was lost. Carrying the belief across would credit
		/// the successor with money nobody can move, and nothing would ever correct it: the
		/// cache is only written when a noter observes a stake, and no noter will ever observe
		/// one for an account that has none. The successor's stake is whatever they bond.
		///
		/// What does move is the account's standing *as a noter* -- its bond, its submission
		/// clock and the disputes recorded against it. Those are conduct, not holdings, and
		/// leaving them behind would let somebody shed a bad noting record by losing a key.
		pub fn rebind_account(
			from: &T::AccountId,
			to: &T::AccountId,
		) -> pezframe_support::pezpallet_prelude::DispatchResult {
			// The record and the money it describes move together. Moving only the record
			// left the reserve on the lost key: unregistering then released the successor's
			// other deposits, and a slash took its free balance (L-7, 2026-10-22).
			let mut was_noter = false;
			if let Some(bond) = NoterBonds::<T>::take(from) {
				was_noter = true;
				match T::Currency::repatriate_reserved(from, to, bond, BalanceStatus::Reserved) {
					Ok(left_behind) => {
						let moved = bond.saturating_sub(left_behind);
						if !moved.is_zero() {
							NoterBonds::<T>::insert(to, moved);
						}
					},
					// The successor cannot take it; the bond stays where its money is, and
					// governance can still slash it there.
					Err(_) => NoterBonds::<T>::insert(from, bond),
				}
			}
			if let Some(at) = NoterLastSubmission::<T>::take(from) {
				was_noter = true;
				NoterLastSubmission::<T>::insert(to, at);
			}
			if was_noter {
				NoterSucceededBy::<T>::insert(from, to);
			}
			let disputes = DisputesAgainstNoter::<T>::take(from);
			if disputes != 0 {
				DisputesAgainstNoter::<T>::insert(to, disputes);
			}
			let open = UnresolvedDisputes::<T>::take(from);
			if open != 0 {
				UnresolvedDisputes::<T>::insert(to, open);
			}
			if TrackingOptIn::<T>::take(from).is_some() {
				TrackingOptIn::<T>::insert(to, ());
			}
			Ok(())
		}

		/// The account that answers today for submissions `who` made as a noter.
		///
		/// Bounded: a noter can lose a key twice, and a chain long enough to matter is not
		/// one, so the walk stops after a fixed number of steps.
		pub fn noter_answering_for(who: &T::AccountId) -> T::AccountId {
			let mut current = who.clone();
			for _ in 0..8 {
				match NoterSucceededBy::<T>::get(&current) {
					Some(next) => current = next,
					None => break,
				}
			}
			current
		}

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
				LastConfirmed::<T>::remove(who, source);
				// Nothing left on the hub for it to speak for, whoever said so.
				if source == StakingSource::AssetHub {
					ReportedByChain::<T>::remove(who);
				}

				let remaining = Self::total_cached_stake(who);
				if remaining.is_zero() && !previous_total.is_zero() {
					// Not erased yet: kept for the grace period, in case the zero was not real.
					LapsedStake::<T>::insert(
						who,
						(
							StakingStartBlock::<T>::take(who),
							StakeFirstSeen::<T>::take(who),
							previous_total,
							pezframe_system::Pezpallet::<T>::block_number(),
						),
					);
				}
			} else {
				let details =
					StakingDetails { staked_amount, nominations_count, unlocking_chunks_count };
				CachedStakingDetails::<T>::insert(who, source, details);
				LastConfirmed::<T>::insert(
					who,
					source,
					pezframe_system::Pezpallet::<T>::block_number(),
				);

				let new_total = Self::total_cached_stake(who);
				// What the comparison below measures against: the stake before a lapse that
				// turned out to be brief, or the stake just before this update.
				let mut compared_with = previous_total;
				if previous_total.is_zero() {
					let now = pezframe_system::Pezpallet::<T>::block_number();
					match LapsedStake::<T>::take(who) {
						// Back within the grace period: the lapse is treated as never having
						// happened, so the duration carries on from where it was.
						Some((start, first_seen, before, lapsed_at))
							if now <= lapsed_at.saturating_add(T::OracleGracePeriod::get()) =>
						{
							if let Some(start) = start {
								StakingStartBlock::<T>::insert(who, start);
							}
							StakeFirstSeen::<T>::insert(who, first_seen.unwrap_or(now));
							compared_with = before;
						},
						_ => StakeFirstSeen::<T>::insert(who, now),
					}
				}
				if !compared_with.is_zero()
					&& new_total > compared_with
					&& StakingStartBlock::<T>::contains_key(who)
				{
					let current_block = pezframe_system::Pezpallet::<T>::block_number();
					StakingStartBlock::<T>::insert(who, current_block);
				}
			}

			Self::schedule_tier_crossings(who);
			T::OnStakingUpdate::on_staking_data_changed(who);
			Self::deposit_event(Event::StakingDetailsReceived {
				who: who.clone(),
				source,
				staked_amount,
			});
		}

		/// The block `who`'s duration is counted from: the later of the opt-in and the stake
		/// first being seen less the grace period -- the two limits `get_staking_score` takes
		/// the smaller of. None while either is missing, when there is no duration at all.
		pub fn duration_origin(who: &T::AccountId) -> Option<BlockNumberFor<T>> {
			let start = StakingStartBlock::<T>::get(who)?;
			let first_seen = StakeFirstSeen::<T>::get(who)?;
			Some(start.max(first_seen.saturating_sub(T::OracleGracePeriod::get())))
		}

		/// Write down the blocks at which `who` reaches each longer tier, if the block their
		/// duration counts from has moved since the last time.
		///
		/// A date written for a clock that was later reset stays where it is: announcing it
		/// costs a recomputation that finds nothing changed, which is cheaper than finding and
		/// removing it.
		pub(crate) fn schedule_tier_crossings(who: &T::AccountId) {
			let Some(origin) = Self::duration_origin(who) else { return };
			if CrossingsScheduledFrom::<T>::get(who) == Some(origin) {
				return;
			}
			CrossingsScheduledFrom::<T>::insert(who, origin);
			let now = pezframe_system::Pezpallet::<T>::block_number();
			for months in TIER_MONTHS {
				let at = origin.saturating_add((months * MONTH_IN_BLOCKS).into());
				if at > now {
					TierCrossings::<T>::insert(at, who, ());
				}
			}
		}

		/// Keep the relay parent's root, and drop one that has aged past `RelayRootsKept`.
		pub(crate) fn keep_relay_root() {
			let Some((number, root)) = T::RelayParent::relay_parent() else { return };
			RelayRoots::<T>::insert(number, root);
			let horizon = number.saturating_sub(T::RelayRootsKept::get());
			// A few a block: the relay parent usually advances by one, and by more only when
			// this chain missed a relay block, so pruning keeps up.
			let mut oldest = OldestRelayRoot::<T>::get().unwrap_or(number);
			for _ in 0..PRUNE_RELAY_ROOTS_PER_BLOCK {
				if oldest >= horizon {
					break;
				}
				RelayRoots::<T>::remove(oldest);
				oldest = oldest.saturating_add(1);
			}
			OldestRelayRoot::<T>::put(oldest);
		}

		/// Announce the tier crossings that have come due, oldest first, within a block's
		/// budget. What does not fit waits for the next block; the cursor does not move past
		/// a block until all of it is announced.
		pub(crate) fn announce_tier_crossings(now: BlockNumberFor<T>) -> Weight {
			let db = T::DbWeight::get();
			let mut cursor = NextCrossingBlock::<T>::get().unwrap_or(now);
			let mut announced = 0u32;
			let mut scanned = 0u32;
			while cursor <= now && scanned < MAX_CROSSING_BLOCKS_SCANNED {
				scanned += 1;
				let room = MAX_TIER_CROSSINGS_PER_BLOCK.saturating_sub(announced) as usize;
				for (who, ()) in TierCrossings::<T>::drain_prefix(cursor).take(room) {
					T::OnStakingUpdate::on_staking_tier_reached(&who);
					announced += 1;
				}
				if announced >= MAX_TIER_CROSSINGS_PER_BLOCK {
					break;
				}
				cursor = cursor.saturating_add(One::one());
			}
			NextCrossingBlock::<T>::put(cursor);
			db.reads_writes(1u64.saturating_add(scanned.into()), 1)
				.saturating_add(db.reads_writes(1, 2).saturating_mul(announced.into()))
		}

		/// The block the duration multiplier may count up to for `who`.
		///
		/// Now, while the newest confirmation of any of the account's sources is within
		/// `MaxDataAge`; otherwise that confirmation plus `MaxDataAge`, so the multiplier holds
		/// where it was rather than climbing on data nobody has checked. Stake recorded before
		/// confirmations were kept has none, and is reckoned confirmed when it was first seen.
		pub fn confirmed_clock(who: &T::AccountId) -> BlockNumberFor<T> {
			let now = pezframe_system::Pezpallet::<T>::block_number();
			match Self::newest_confirmation(who) {
				Some(at) => now.min(at.saturating_add(T::MaxDataAge::get())),
				None => now,
			}
		}

		/// The newest confirmation of any of `who`'s sources; stake recorded before
		/// confirmations were kept is reckoned confirmed when it was first seen, and stake the
		/// Asset Hub reports itself is confirmed now.
		pub fn newest_confirmation(who: &T::AccountId) -> Option<BlockNumberFor<T>> {
			// The hub reports every change, so what it last said is still what is there.
			if ReportedByChain::<T>::contains_key(who) {
				return Some(pezframe_system::Pezpallet::<T>::block_number());
			}
			LastConfirmed::<T>::iter_prefix_values(who)
				.max()
				.or_else(|| StakeFirstSeen::<T>::get(who))
		}

		/// The height the oracle's word on `who` is current as of, for a reader that applies its
		/// own freshness bound.
		///
		/// Now, while the newest confirmation is within `MaxDataAge` -- this pallet's one rule
		/// for when unchanged stake is still believed. Otherwise that confirmation, so the
		/// reader's bound sees how old the word really is. Zero when nothing was ever
		/// confirmed. Not `get_staking_score`'s second value: that is how long the stake has
		/// been held, a duration and not a height.
		pub fn observed_at(who: &T::AccountId) -> BlockNumberFor<T> {
			let now = pezframe_system::Pezpallet::<T>::block_number();
			match Self::newest_confirmation(who) {
				Some(at) if now <= at.saturating_add(T::MaxDataAge::get()) => now,
				Some(at) => at,
				None => Zero::zero(),
			}
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
		/// The largest amount tier at the longest duration multiplier, which is also the cap
		/// the calculation applies at the end.
		fn max_score() -> RawScore {
			MAX_STAKING_SCORE
		}

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
					let current_block = Self::confirmed_clock(who);

					// Counted from the opt-in, so the delay in getting the data here is not
					// charged to the user -- but never for longer than there has actually
					// been a stake, plus what that delay is reckoned to be worth. Without the
					// second half, opting in on an empty account and staking a year later
					// would collect the twelve-month multiplier on arrival.
					let since_optin = current_block.saturating_sub(start_block);
					let duration_in_blocks = match StakeFirstSeen::<T>::get(who) {
						Some(first_seen) => since_optin.min(
							current_block
								.saturating_sub(first_seen)
								.saturating_add(T::OracleGracePeriod::get()),
						),
						// No stake has ever been recorded, so there is nothing to have held.
						None => Zero::zero(),
					};

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

			(final_score.min(MAX_STAKING_SCORE), duration_for_return)
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
	fn stakingsource_indices_are_pinned() {
		let pinned: &[(&str, u8, &dyn Fn() -> Vec<u8>)] = &[
			("RelayChain", 0u8, &|| pezpallet::StakingSource::RelayChain.encode()),
			("AssetHub", 1u8, &|| pezpallet::StakingSource::AssetHub.encode()),
		];
		let moved: Vec<&str> = pinned
			.iter()
			.filter(|(_, want, enc)| enc() != vec![*want])
			.map(|(name, _, _)| *name)
			.collect();
		assert!(moved.is_empty(), "`StakingSource` indices moved: {moved:?}");
		assert_eq!(pinned.len(), 2, "a variant was added or removed");
	}
}
