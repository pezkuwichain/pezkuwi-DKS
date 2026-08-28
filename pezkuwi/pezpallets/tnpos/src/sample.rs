// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Seating a committee.

use crate::*;

impl<T: Config> Pezpallet<T> {
	/// Candidates standing in `stratum`, bounded by `MaxPoolSize`.
	fn candidates(stratum: StratumId) -> Vec<T::AccountId> {
		PoolMembers::<T>::iter()
			.take(T::MaxPoolSize::get() as usize)
			.filter_map(|(who, s)| (s == stratum).then_some(who))
			.collect()
	}

	/// Draw a committee for the next era.
	///
	/// A stratum that misses its floor stands down and the committee is smaller for it.
	/// Its seats are never given to another stratum: that repair would concentrate the very
	/// power the strata exist to divide, so `seat` does not offer it.
	pub(crate) fn do_seat_committee() -> Result<Seating, Error<T>> {
		let strata = Strata::<T>::get();
		let sizes: Vec<u32> = strata.iter().map(|c| StratumSize::<T>::get(c.id)).collect();

		let seating = seat(&strata, &sizes).map_err(|e| match e {
			InvariantError::TooFewStrata
			| InvariantError::CommitteeTooSmall
			| InvariantError::LengthMismatch
			| InvariantError::EmptyStratum
			| InvariantError::DuplicateStratum
			| InvariantError::CommitteeTooLarge
			| InvariantError::DegenerateCommitteeSize => Error::<T>::UnseatableConfiguration,
		})?;

		let era = CurrentEra::<T>::get().saturating_add(1);
		let mut committee = Vec::with_capacity(seating.n as usize);
		for cfg in seating.seated.iter() {
			let pool = Self::candidates(cfg.id);
			let drawn = T::Sortition::select(era, cfg.id, &pool, cfg.seats)
				.ok_or(Error::<T>::UnseatableConfiguration)?;
			committee.extend(drawn);
		}

		// The ceiling is named once, in the primitives crate, and `CurrentCommittee` is
		// declared against the same constant -- two places that must agree should not be two
		// numbers. `seat` already refuses configurations above it, so this conversion cannot
		// fail today; the branch stays because a future change to `seat` should surface as a
		// refused era rather than a panic.
		let bounded: BoundedVec<
			T::AccountId,
			ConstU32<{ pezkuwi_tnpos_primitives::invariant::MAX_COMMITTEE }>,
		> = committee.try_into().map_err(|_| Error::<T>::UnseatableConfiguration)?;

		let unseated: Vec<StratumId> = strata
			.iter()
			.filter(|c| !seating.seated.iter().any(|s| s.id == c.id))
			.map(|c| c.id)
			.collect();

		CurrentEra::<T>::put(era);
		EraStart::<T>::put(pezframe_system::Pezpallet::<T>::block_number());
		CurrentCommittee::<T>::put(&bounded);

		Self::deposit_event(Event::CommitteeSeated {
			era,
			size: seating.n,
			quorum: seating.quorum(),
			unseated,
		});
		Ok(seating)
	}
}
