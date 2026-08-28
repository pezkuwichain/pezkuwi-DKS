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
pub mod weights;

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

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::{weights::WeightInfo, *};

	/// First version this pallet has ever had on chain. Written down so a future migration
	/// can tell whether it has run.
	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	/// Seats each stratum carries in the specified committee.
	pub const SEATS_PER_STRATUM: u32 = 3;

	/// Eligible members a stratum needs before it may be seated. Section 5 of the design
	/// puts a stratum's chance of losing all three seats to a ten-member adversary under
	/// one percent at this floor.
	pub const MIN_ELIGIBLE_PER_STRATUM: u32 = 50;

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

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A member joined `stratum`.
		Joined { who: T::AccountId, stratum: StratumId },
		/// A member left the pool.
		Left { who: T::AccountId },
		/// A committee was seated. `unseated` names the strata that stood down.
		CommitteeSeated { era: u32, size: u32, quorum: u32, unseated: Vec<StratumId> },
		/// No committee could be seated; the previous one stays.
		SeatingRefused { era: u32 },
		/// The strata configuration changed.
		StrataSet { count: u32 },
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
