// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Seating a committee.

use crate::*;

impl<T: Config> Pezpallet<T> {
	/// Candidates standing in `stratum`, bounded by `MaxPoolSize`.
	///
	/// Filtered on session keys here as well as at `join`: `join` keeps a keyless account
	/// out, but an account can deregister its keys after joining and would otherwise stay
	/// in `PoolMembers` with nothing to back a seat. Checking both places closes both the
	/// entry and the standing-membership route to the same silent drop in session.
	fn candidates(stratum: StratumId) -> Vec<T::AccountId> {
		PoolMembers::<T>::iter()
			.take(T::MaxPoolSize::get() as usize)
			.filter_map(|(who, s)| {
				(s == stratum && T::HasSessionKeys::has_keys(&who)).then_some(who)
			})
			.collect()
	}

	/// Draw a committee for the next era.
	///
	/// A stratum that misses its floor stands down and the committee is smaller for it.
	/// Its seats are never given to another stratum: that repair would concentrate the very
	/// power the strata exist to divide, so `seat` does not offer it.
	pub(crate) fn do_seat_committee() -> Result<Seating, Error<T>> {
		let outcome = Self::try_seat_committee();
		// The round is spent whether or not it produced a committee. Its preimages are
		// public now, so carrying the value into another era would draw from a seed anyone
		// can recompute.
		NextSeed::<T>::kill();
		outcome
	}

	fn try_seat_committee() -> Result<Seating, Error<T>> {
		let strata = Strata::<T>::get();
		let sizes: Vec<u32> = strata.iter().map(|c| StratumSize::<T>::get(c.id)).collect();

		let seating = seat(&strata, &sizes).map_err(|e| match e {
			InvariantError::TooFewStrata
			| InvariantError::CommitteeTooSmall
			| InvariantError::LengthMismatch
			| InvariantError::EmptyStratum
			| InvariantError::DuplicateStratum
			| InvariantError::CommitteeTooLarge
			| InvariantError::DegenerateCommitteeSize
			| InvariantError::FloorTooLow => Error::<T>::UnseatableConfiguration,
		})?;

		let era = CurrentEra::<T>::get().saturating_add(1);
		let mut committee = Vec::with_capacity(seating.n as usize);
		for cfg in seating.seated.iter() {
			let pool = Self::candidates(cfg.id);
			let drawn = T::Sortition::select(era, cfg.id, &pool, cfg.seats)
				.ok_or(Error::<T>::UnseatableConfiguration)?;
			// `seat` judged this stratum on `StratumSize`, which counts pool membership;
			// `candidates` filters that same pool by session keys. If enough members have
			// deregistered, the draw comes back short -- and a short draw seated anyway
			// would announce a committee, and a quorum, that do not exist.
			ensure!(drawn.len() as u32 == cfg.seats, Error::<T>::UnseatableConfiguration);
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

		// Hand it to the chain that will validate with it.
		Self::export_committee(era);

		Ok(seating)
	}
}
