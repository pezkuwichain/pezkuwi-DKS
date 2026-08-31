// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! # TNPoS
//!
//! Selects a validator committee by drawing a fixed number of seats from each of nine
//! independent strata. Buying one stratum outright buys three seats of twenty-seven, which
//! is neither enough to stall the chain nor to fork it; that bound is the design.
//!
//! See `docs/TNPOS_DESIGN.md` for the threat model and the security budget.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pezpallet::*;
pub mod pool;
pub mod sample;
pub mod seed;
pub mod slash;
pub mod weights;
/// Re-exported at the crate root because that is where a generated weights file looks for it.
///
/// `benchmark pallet` writes `impl pezpallet_tnpos::WeightInfo for WeightInfo<T>`, and without
/// this the generated file does not compile -- which is how the first real weights run for this
/// pallet failed, after the benchmark itself had succeeded. Every other pallet here reaches the
/// same place through `pub use pezpallet::*`; this one has no such re-export, so it says it.
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use alloc::vec::Vec;
use pezframe_support::{pezpallet_prelude::*, traits::Get};
use pezframe_system::pezpallet_prelude::*;
use pezkuwi_tnpos_primitives::{
	invariant::{seat, InvariantError, Seating},
	scores::ScoreProvider,
	sortition::Sortition,
	StratumConfig, StratumId,
};
use pezsp_runtime::Saturating;
use slash::Offence;

/// Lets a runtime make an account eligible so `join` can be benchmarked.
///
/// "Eligible" here means everything `do_join` checks, not only the score gate: session
/// keys are part of that since the fix that made `join` refuse a keyless account. Both
/// arrangements have to happen here rather than inside the benchmark itself, because
/// benchmarks run against a real runtime -- scores arrive over XCM from another chain and
/// session keys come from a real keystore, and neither can be conjured from inside a
/// benchmark. The runtime supplies this so the measured path stays the real one:
/// `eligible_for` still reads scores through `Scores`, and `do_join` still reads keys
/// through `HasSessionKeys`, exactly as they do in production.
#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper<AccountId> {
	/// Give `who` whatever standing `stratum` requires, including session keys.
	fn make_eligible(who: &AccountId, stratum: StratumId);
}

/// Reports whether an account can actually serve: session drops keyless validators.
///
/// `pezpallet_session::rotate_session` silently filters out any validator with no
/// registered session keys, logging a warning and moving on. Nothing upstream of that point
/// knows it happened, so a stratum can be seated in storage at its full three seats and
/// still hand session fewer authorities than that -- or, if enough seats went the same way,
/// none at all.
pub trait HasSessionKeys<AccountId> {
	fn has_keys(who: &AccountId) -> bool;
}

/// The pallet answers this from its own register.
///
/// Bound as `type HasSessionKeys = Tnpos;` in the runtime. The trait stays rather than being
/// replaced by a direct read because the tests need to drive it, but the register the pallet
/// keeps is the one answer that cannot disagree with itself -- anything else bound here is a
/// second opinion about a fact this pallet already holds.
impl<T: Config> HasSessionKeys<T::AccountId> for Pezpallet<T> {
	fn has_keys(who: &T::AccountId) -> bool {
		RelayKeys::<T>::contains_key(who)
	}
}

/// Carries a seated committee to the chain that will validate with it.
///
/// Deliberately without a `()` implementation, here and on [`SendKeysToRelay`]. The obvious
/// default -- do nothing, report success -- is the shape of every silent gate this tree has
/// had to dig out: a runtime that forgot to wire delivery would compile, emit
/// `CommitteeExported`, and seat nobody. Making the type system ask the question costs one
/// line per runtime and cannot be answered wrongly by omission.
pub trait SendCommitteeToRelay<AccountId> {
	/// `era` is the identifier the receiving chain records the set under.
	fn send(era: u32, committee: alloc::vec::Vec<AccountId>) -> Result<(), ()>;
}

/// Carries a key registration to the chain whose session pallet will hold it.
///
/// Failure is reported, not swallowed. The caller reverts on `Err`, so this chain never keeps a
/// record of keys the relay did not receive.
pub trait SendKeysToRelay<AccountId> {
	fn set_keys(stash: &AccountId, keys: alloc::vec::Vec<u8>) -> Result<(), ()>;
	fn purge_keys(stash: &AccountId) -> Result<(), ()>;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::{weights::WeightInfo, *};

	/// First version this pallet has ever had on chain. Written down so a future migration
	/// can tell whether it has run.
	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	/// Seats each stratum carries in the specified committee.
	pub const SEATS_PER_STRATUM: u32 = 3;

	/// Eligible members a stratum needs before it may be seated. Defined in `invariant`
	/// alongside `FloorTooLow`, the check that enforces it, and re-exported here for
	/// genesis and the benchmarks.
	pub use pezkuwi_tnpos_primitives::invariant::MIN_ELIGIBLE_PER_STRATUM;

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>> {
		type WeightInfo: crate::weights::WeightInfo;

		/// Where an era's draw comes from. Phase 1 supplies a commit-reveal seed; phase 2
		/// replaces this with ring-VRF tickets without touching the rest of the pallet.
		type Sortition: Sortition<Self::AccountId>;

		/// Cached People-chain scores. Reads go through `ScoreSnapshot::value_if_fresh`.
		type Scores: ScoreProvider<Self::AccountId, BlockNumberFor<Self>>;

		/// Whether an account has registered session keys.
		///
		/// A validator without keys is silently dropped when the session rotates, which
		/// would let a stratum's real share of the committee differ from its seated one --
		/// and, if enough seats went that way, leave the authority set empty.
		type HasSessionKeys: crate::HasSessionKeys<Self::AccountId>;

		/// The relay chain's own `SessionKeys`, mirrored so keys can be checked before they
		/// are forwarded.
		///
		/// This chain cannot name the relay's type, so it declares a structurally identical
		/// one. The mirror has to match field for field and in order -- it is what decodes
		/// the bytes -- and `the_relay_key_mirror_matches` in the runtime tests is what holds
		/// the two definitions together.
		type RelaySessionKeys: pezsp_runtime::traits::OpaqueKeys + codec::Decode;

		/// How a seated committee reaches the chain it will validate.
		///
		/// The committee is only a list in storage until somebody acts on it. Delivery is
		/// separated from seating so the pallet stays independent of where it runs, but it is
		/// not optional: a pallet that draws a committee nobody receives is a mechanism that
		/// was designed and never wired, and this tree has spent a week removing those.
		///
		/// A failed send is logged and the era still advances. The alternative -- refusing to
		/// seat because the message could not go out -- would hand an unreachable relay the
		/// power to freeze this chain's own era clock.
		type SendCommitteeToRelay: crate::SendCommitteeToRelay<Self::AccountId>;

		/// How the keys reach the relay, where the session pallet that uses them lives.
		///
		/// Registration is a local write *and* a message: this chain keeps the register of who
		/// holds keys, and the relay keeps the keys themselves. Both happen in one call, and
		/// the call reverts as a whole if the message cannot be sent -- a local record of a key
		/// the relay never received is the disagreement this design exists to make impossible.
		type SendKeysToRelay: crate::SendKeysToRelay<Self::AccountId>;

		/// May set strata and force an era.
		type ManagerOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// How old a cached score may be before it counts as absent.
		#[pezpallet::constant]
		type MaxScoreAge: Get<BlockNumberFor<Self>>;

		/// Blocks per era.
		#[pezpallet::constant]
		type EraLength: Get<BlockNumberFor<Self>>;

		/// Upper bound on pool members. Bounds every iteration in this pallet.
		#[pezpallet::constant]
		type MaxPoolSize: Get<u32>;

		/// Makes an account eligible for a stratum during benchmarking.
		///
		/// Benchmarks run against a real runtime, whose scores arrive over XCM from another
		/// chain and cannot be conjured from inside a benchmark. The runtime supplies this
		/// so the measured path stays the real one: `eligible_for` still reads scores
		/// through `Scores` exactly as it does in production.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: crate::BenchmarkHelper<Self::AccountId>;
	}

	/// The strata this chain draws from, and what each carries.
	#[pezpallet::storage]
	pub type Strata<T: Config> =
		StorageValue<_, BoundedVec<StratumConfig, ConstU32<16>>, ValueQuery>;

	/// Which stratum each pool member stands in. A member stands in exactly one: an account
	/// in two strata would correlate them, and the security arithmetic assumes they are not.
	#[pezpallet::storage]
	pub type PoolMembers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StratumId, OptionQuery>;

	/// Eligible members per stratum. Kept as a counter so seating never has to iterate.
	#[pezpallet::storage]
	pub type StratumSize<T: Config> = StorageMap<_, Twox64Concat, StratumId, u32, ValueQuery>;

	/// Who has registered relay session keys, and the keys themselves.
	///
	/// The register of holders is here because this is where eligibility is decided; the keys
	/// are here too so a re-send after a failed delivery does not need the holder to type them
	/// again. The relay's session pallet holds the authoritative copy that consensus reads --
	/// but it accepts writes only from this chain, so the two cannot drift.
	#[pezpallet::storage]
	pub type RelayKeys<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<u8, ConstU32<512>>, OptionQuery>;

	#[pezpallet::storage]
	pub type CurrentEra<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pezpallet::storage]
	pub type EraStart<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// The committee seated for the current era, in stratum order.
	#[pezpallet::storage]
	pub type CurrentCommittee<T: Config> = StorageValue<
		_,
		BoundedVec<T::AccountId, ConstU32<{ pezkuwi_tnpos_primitives::invariant::MAX_COMMITTEE }>>,
		ValueQuery,
	>;

	/// Commitments for an era's seed, scoped to the era they are for.
	///
	/// Scoped because a commit-reveal round belongs to one era. An unscoped pot lets an
	/// account commit again after seeing what others revealed, which is not withholding but
	/// steering.
	#[pezpallet::storage]
	pub type SeedCommitments<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		u32,
		Blake2_128Concat,
		T::AccountId,
		[u8; 32],
		OptionQuery,
	>;

	/// The mixed seed for an era's draw, and the era it belongs to.
	///
	/// Spent when that era is drawn. An era with no round of its own has no seed and is
	/// refused, rather than drawing from a value the whole chain has already seen.
	#[pezpallet::storage]
	pub type NextSeed<T: Config> = StorageValue<_, ([u8; 32], u32), OptionQuery>;

	/// Accounts barred from the pool, and the era their ban lifts.
	#[pezpallet::storage]
	pub type Banned<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, u32, OptionQuery>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A member joined `stratum`.
		Joined { who: T::AccountId, stratum: StratumId },
		/// A member left the pool.
		Left { who: T::AccountId },
		/// A seated committee was handed to the router for the relay.
		///
		/// Sent, not accepted. A successful `send` means the message left this chain; whether
		/// the relay's `ah_client` took it depends on an origin check over there, and this
		/// chain never hears the answer. Reading this event as "the relay is now using this
		/// committee" is the mistake `isInBlock` teaches every time.
		CommitteeSentToRelay { era: u32, size: u32 },
		/// The committee was seated but could not even be sent; the relay keeps the old one.
		CommitteeCouldNotBeSent { era: u32 },
		/// The seated committee has fallen below the size its security budget was chosen for.
		///
		/// Removals inside an era shrink the committee and there is no way to refill it: the
		/// seed is consumed and killed at each seating, so a redraw needs the next
		/// commit-reveal round -- half an era away. Refusing the removal instead would leave
		/// an equivocator signing, which is worse than a smaller committee.
		///
		/// So the chain goes on with what is left and says so. `MIN_COMMITTEE` is where the
		/// fork and halt probabilities in the design were computed; below it they are worse
		/// than anyone agreed to, and that has to be an alarm rather than something an
		/// operator infers from a validator set that quietly got shorter.
		CommitteeBelowSecurityFloor { era: u32, size: u32, floor: u32 },
		/// Nothing was sent because the committee is empty; the relay keeps the old one.
		///
		/// Distinct from a failed send: this one is not a delivery problem, it is this chain
		/// having no committee to offer, and the two need different answers.
		EmptyCommitteeNotSent { era: u32 },

		/// Relay session keys were registered here and forwarded.
		RelayKeysSet { who: T::AccountId },
		/// Relay session keys were withdrawn here and on the relay.
		RelayKeysPurged { who: T::AccountId },
		/// A committee was seated. `unseated` names the strata that stood down.
		CommitteeSeated { era: u32, size: u32, quorum: u32, unseated: Vec<StratumId> },
		/// No committee could be seated; the previous one stays.
		SeatingRefused { era: u32 },
		/// The strata configuration changed.
		StrataSet { count: u32 },
		/// A member was punished for `offence` and barred from the pool until `banned_until`.
		Punished { who: T::AccountId, offence: Offence, banned_until: u32 },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		AlreadyInPool,
		NotInPool,
		PoolFull,
		/// The account does not meet this stratum's gate.
		NotEligible,
		/// A score this decision needs is missing or too old. Deliberately not the same as
		/// `NotEligible`: a stalled channel is an outage, not a judgement about the account.
		ScoreUnavailable,
		/// The strata configuration cannot be seated at all.
		UnseatableConfiguration,
		/// A reveal was submitted with no matching commitment.
		NoCommitment,
		/// The revealed preimage does not hash to the commitment on record.
		BadReveal,
		/// This account already committed for this round.
		AlreadyCommitted,
		/// The commit half of this round has closed; only reveals are accepted now.
		CommitWindowClosed,
		/// The reveal half of this round has not opened yet; the commit half is still running.
		RevealWindowNotOpen,
		/// This account is barred from the pool until its ban expires.
		Banned,
		/// This account has no session keys registered. Session silently drops a keyless
		/// validator on rotation, so seating one would let a stratum's storage record
		/// disagree with its real authority count.
		NoSessionKeys,
		/// The bytes do not decode as the relay's `SessionKeys`.
		InvalidRelayKeys,
		/// The ownership proof does not match the keys and the account offering them.
		InvalidKeyOwnershipProof,
		/// The keys are longer than the register will hold.
		RelayKeysTooLong,
		/// The relay could not be reached, so nothing was recorded.
		CouldNotReachRelay,
		/// There are no keys registered for this account.
		NoRelayKeys,
	}

	#[pezpallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub strata: Vec<StratumConfig>,
		pub members: Vec<(T::AccountId, StratumId)>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		/// The nine strata this chain is specified with, three seats each.
		///
		/// Not a placeholder: nine gates of three is the shape of the committee, so a
		/// chainspec that omits the section should get that chain rather than one which
		/// seats nobody. An explicitly empty or otherwise unseatable configuration still
		/// fails in `build`.
		fn default() -> Self {
			Self {
				strata: StratumId::ALL
					.iter()
					.map(|&id| StratumConfig {
						id,
						seats: SEATS_PER_STRATUM,
						min_eligible: MIN_ELIGIBLE_PER_STRATUM,
					})
					.collect(),
				members: Vec::new(),
			}
		}
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let bounded: BoundedVec<StratumConfig, ConstU32<16>> = self
				.strata
				.clone()
				.try_into()
				.expect("genesis declares at most sixteen strata; qed");

			// Validate the configuration, not the population: the pool is legitimately
			// empty at genesis and fills before the first era. Pretending every stratum is
			// full checks exactly the config-level floors -- stratum count, total seats,
			// no stratum carrying zero. A chain that cannot ever be seated must fail to
			// build rather than start and discover it at the first era boundary.
			let as_if_full = alloc::vec![u32::MAX; bounded.len()];
			seat(&bounded, &as_if_full).expect("genesis strata must be seatable; qed");

			Strata::<T>::put(&bounded);
			for (who, stratum) in &self.members {
				PoolMembers::<T>::insert(who, stratum);
				StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_add(1));
			}
			CurrentEra::<T>::put(0u32);
		}
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let mut weight = T::DbWeight::get().reads(1);
			if now < EraStart::<T>::get().saturating_add(T::EraLength::get()) {
				return weight;
			}

			// The era window moves on whether or not a committee could be drawn. The pallet
			// this replaces left it in place on failure and so re-ran the entire selection
			// on every block, paying full weight each time and never recovering.
			EraStart::<T>::put(now);
			weight = weight.saturating_add(T::DbWeight::get().writes(1));
			weight = weight.saturating_add(T::WeightInfo::seat_committee(T::MaxPoolSize::get()));

			if Self::do_seat_committee().is_err() {
				Self::deposit_event(Event::SeatingRefused { era: CurrentEra::<T>::get() });
				log::warn!(target: "tnpos", "no committee could be seated; previous one stands");
			}
			weight
		}

		#[cfg(feature = "try-runtime")]
		fn try_state(_: BlockNumberFor<T>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			let strata = Strata::<T>::get();
			let sizes: Vec<u32> = strata.iter().map(|c| StratumSize::<T>::get(c.id)).collect();
			// A live chain whose strata cannot be seated is a chain running outside its
			// security budget; that must surface as a failure, not as a quiet degradation.
			seat(&strata, &sizes)
				.map_err(|_| "tnpos: strata cannot satisfy the security floors")?;
			Ok(())
		}

		fn integrity_test() {
			// A commit half of zero blocks accepts no contribution, so no seed is ever built
			// and no era is ever drawn -- silently, and forever. The window is half the era,
			// so the era needs at least two blocks for both halves to exist at all.
			assert!(
				T::EraLength::get() >= 2u32.into(),
				"EraLength must be at least two blocks: the commit and reveal halves each need one"
			);
		}
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Join `stratum`. Every gate is measured against current scores.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::join())]
		pub fn join(origin: OriginFor<T>, stratum: StratumId) -> DispatchResult {
			Self::do_join(ensure_signed(origin)?, stratum)
		}

		/// Register the relay session keys this account will validate with.
		///
		/// The keys are checked here and held on the relay. Checked here because this is where
		/// eligibility is decided and an applicant deserves the reason rather than an
		/// unexplained absence from the committee; held there because that is where consensus
		/// reads them.
		///
		/// One call, two writes, and they cannot come apart: the message goes first and the
		/// whole call reverts if it cannot be sent, so this chain never records a key the relay
		/// did not receive. The relay accepts key writes from here and nowhere else, which is
		/// what makes the pair a single register rather than two that agree by habit.
		#[pezpallet::call_index(7)]
		#[pezpallet::weight(T::WeightInfo::join())]
		pub fn set_relay_keys(
			origin: OriginFor<T>,
			keys: alloc::vec::Vec<u8>,
			proof: alloc::vec::Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Decoded as the relay's own key type, so a payload that would be rejected on
			// arrival is rejected before it is recorded.
			let decoded = <T::RelaySessionKeys as codec::Decode>::decode(&mut &keys[..])
				.map_err(|_| Error::<T>::InvalidRelayKeys)?;
			ensure!(
				pezsp_runtime::traits::OpaqueKeys::ownership_proof_is_valid(
					&decoded,
					&who.encode(),
					&proof
				),
				Error::<T>::InvalidKeyOwnershipProof
			);

			let bounded: BoundedVec<u8, ConstU32<512>> =
				keys.clone().try_into().map_err(|_| Error::<T>::RelayKeysTooLong)?;

			T::SendKeysToRelay::set_keys(&who, keys).map_err(|_| Error::<T>::CouldNotReachRelay)?;
			RelayKeys::<T>::insert(&who, bounded);

			Self::deposit_event(Event::RelayKeysSet { who });
			Ok(())
		}

		/// Withdraw the keys, here and on the relay.
		///
		/// Leaves the pool as well, because a member without keys is a seat that the session
		/// would silently drop -- the same reason `join` refuses one. Removing them from the
		/// pool here is what keeps the stratum counts honest.
		#[pezpallet::call_index(8)]
		#[pezpallet::weight(T::WeightInfo::leave())]
		pub fn purge_relay_keys(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(RelayKeys::<T>::contains_key(&who), Error::<T>::NoRelayKeys);

			T::SendKeysToRelay::purge_keys(&who).map_err(|_| Error::<T>::CouldNotReachRelay)?;
			RelayKeys::<T>::remove(&who);
			if PoolMembers::<T>::contains_key(&who) {
				Self::do_leave(who.clone())?;
			}

			Self::deposit_event(Event::RelayKeysPurged { who });
			Ok(())
		}

		/// Leave the pool.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(T::WeightInfo::leave())]
		pub fn leave(origin: OriginFor<T>) -> DispatchResult {
			Self::do_leave(ensure_signed(origin)?)
		}

		/// Seat a new committee now.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(T::WeightInfo::seat_committee(T::MaxPoolSize::get()))]
		pub fn force_new_era(origin: OriginFor<T>) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			match Self::do_seat_committee() {
				Ok(_) => Ok(()),
				Err(e) => {
					Self::deposit_event(Event::SeatingRefused { era: CurrentEra::<T>::get() });
					Err(e.into())
				},
			}
		}

		/// Commit to a future seed contribution by hash. Reveal it in a later call with
		/// `reveal_seed`.
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(T::WeightInfo::commit_seed())]
		pub fn commit_seed(origin: OriginFor<T>, hash: [u8; 32]) -> DispatchResult {
			Self::do_commit_seed(ensure_signed(origin)?, hash)
		}

		/// Reveal a prior commitment. Its preimage is mixed into the next era's seed.
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::WeightInfo::reveal_seed())]
		pub fn reveal_seed(origin: OriginFor<T>, preimage: [u8; 32]) -> DispatchResult {
			Self::do_reveal_seed(ensure_signed(origin)?, preimage)
		}

		/// Punish `who` for `offence`: remove them from the pool and the seated committee, and
		/// bar them from rejoining until their ban expires.
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(T::WeightInfo::report_offence())]
		pub fn report_offence(
			origin: OriginFor<T>,
			who: T::AccountId,
			offence: Offence,
		) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			Self::do_report_offence(who, offence)
		}

		/// Replace the strata configuration.
		///
		/// Refused unless the new configuration could be seated, so the chain cannot be
		/// governed into a shape that is outside its own security budget.
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::WeightInfo::set_strata())]
		pub fn set_strata(origin: OriginFor<T>, strata: Vec<StratumConfig>) -> DispatchResult {
			T::ManagerOrigin::ensure_origin(origin)?;
			let bounded: BoundedVec<StratumConfig, ConstU32<16>> =
				strata.try_into().map_err(|_| Error::<T>::UnseatableConfiguration)?;
			let as_if_full = alloc::vec![u32::MAX; bounded.len()];
			seat(&bounded, &as_if_full).map_err(|_| Error::<T>::UnseatableConfiguration)?;

			let count = bounded.len() as u32;
			Strata::<T>::put(bounded);
			Self::deposit_event(Event::StrataSet { count });
			Ok(())
		}
	}
}

impl<T: Config> Pezpallet<T> {
	/// Send the committee as it currently stands to the chain that validates with it.
	///
	/// Called at seating and again whenever the committee changes inside an era, because the
	/// receiving chain keeps a copy and nothing over there re-reads this storage. While this
	/// pallet was a `SessionManager` the question did not arise -- session asked every time --
	/// and the export replaced that with a single message per era, which silently dropped
	/// mid-era removals: a member banned for equivocation left `CurrentCommittee` here and
	/// kept validating there until the next era.
	///
	/// An empty committee is never sent. The receiving side reads a set of no validators as an
	/// instruction to install none, which stops the chain; when there is nothing to offer, the
	/// recoverable answer is to change nothing and say so.
	pub(crate) fn export_committee(era: u32) {
		let committee = CurrentCommittee::<T>::get();
		let size = committee.len() as u32;
		if committee.is_empty() {
			log::warn!(target: "tnpos", "committee for era {era} is empty; not exporting");
			Self::deposit_event(Event::EmptyCommitteeNotSent { era });
			return;
		}

		match T::SendCommitteeToRelay::send(era, committee.to_vec()) {
			Ok(()) => Self::deposit_event(Event::CommitteeSentToRelay { era, size }),
			Err(()) => {
				log::warn!(target: "tnpos", "committee for era {era} could not be delivered");
				Self::deposit_event(Event::CommitteeCouldNotBeSent { era });
			},
		}
	}
}

// There is deliberately no `SessionManager` implementation here.
//
// One was written while this pallet lived on the relay, where seating the committee and
// running the session were the same act. It does not live there any more: this chain's own
// session belongs to `CollatorSelection`, and the chain the committee validates is two hops
// away. An `impl SessionManager for Tnpos` left behind would tell the next reader that the
// committee seats itself somewhere, and nothing would contradict them -- a dead
// implementation is a claim the compiler keeps agreeing with.
//
// The committee leaves through `SendCommitteeToRelay` and nowhere else.
