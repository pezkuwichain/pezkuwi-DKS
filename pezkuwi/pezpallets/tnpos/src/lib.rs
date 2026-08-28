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
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use alloc::vec::Vec;
use pezframe_support::{pezpallet_prelude::*, traits::Get};
use pezframe_system::pezpallet_prelude::*;
use pezkuwi_tnpos_primitives::{
	invariant::seat, scores::ScoreProvider, sortition::Sortition, StratumConfig, StratumId,
};

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
