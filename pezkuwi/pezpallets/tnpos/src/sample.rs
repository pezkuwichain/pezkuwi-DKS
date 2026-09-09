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

	/// The geography stratum's candidates, grouped by region and in index order.
	///
	/// Sorted so the rotation below is a function of the era and the regions present, and not
	/// of whatever order the pool happens to iterate in -- storage order is not something a
	/// reader can reproduce, and a seat allocation nobody can reproduce is not auditable.
	fn candidates_by_region() -> Vec<(u8, Vec<T::AccountId>)> {
		let mut groups: Vec<(u8, Vec<T::AccountId>)> = Vec::new();
		for who in Self::candidates(StratumId::Geography) {
			let Some(region) = T::Scores::region_of(&who) else { continue };
			match groups.iter_mut().find(|(r, _)| *r == region) {
				Some((_, members)) => members.push(who),
				None => groups.push((region, alloc::vec![who])),
			}
		}
		groups.sort_by_key(|(r, _)| *r);
		groups
	}

	/// Which regions take this era's geography seats.
	///
	/// Three seats and six regions, so a uniform draw over the whole marked pool would let the
	/// most populous region take all three and the region label would decide nothing -- the
	/// stratum would be "citizens a notary vouched for", which is not what it is named after.
	/// Instead the seats rotate: the regions present are ordered, an offset advances with the
	/// era, and three consecutive regions each supply one member. Six regions complete a cycle
	/// in two eras, and every region present is served before any is served twice.
	///
	/// The rotation is deliberately *not* random. Randomness is what decides which member of a
	/// region sits; which regions are served is a distribution guarantee, and a guarantee that
	/// holds only on average is not one.
	pub(crate) fn regions_for_era(era: u32, present: usize) -> Vec<usize> {
		if present == 0 {
			return Vec::new();
		}
		let take = core::cmp::min(SEATS_PER_STRATUM as usize, present);
		// The offset advances by a whole eraful, not by one. Advancing by one would overlap
		// consecutive eras -- 0,1,2 then 1,2,3 -- so two eras would reach four regions instead
		// of six and the two at the back of the list would wait far longer than the two at the
		// front. Advancing by `take` serves every region once before any is served twice.
		let offset = (era as usize).saturating_mul(take) % present;
		(0..take).map(|i| (offset + i) % present).collect()
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
		let by_region = Self::candidates_by_region();

		let sizes: Vec<u32> = strata
			.iter()
			.map(|c| {
				let size = StratumSize::<T>::get(c.id);
				// Geography needs one region per seat, so fewer regions than seats means the
				// stratum cannot be drawn however many marks exist. Reported as zero here
				// rather than discovered in the draw: `seat` decides seating from this number,
				// and a stratum that stands down leaves a committee of twenty-four with its
				// seats unredistributed -- whereas a short draw further down fails the whole
				// era, and one thin stratum must not cost the chain its validators.
				if c.id == StratumId::Geography && by_region.len() < SEATS_PER_STRATUM as usize {
					0
				} else {
					size
				}
			})
			.collect();

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
			// Geography draws once per region rather than once for the stratum, so that the
			// three seats land in three different places. Every other stratum draws once.
			let drawn = if cfg.id == StratumId::Geography {
				let mut picked = Vec::with_capacity(cfg.seats as usize);
				for i in Self::regions_for_era(era, by_region.len()) {
					let (region, members) = &by_region[i];
					// Salted with the region: without it every region would be shuffled by
					// the same domain and each would return the same position of its own list.
					let one = T::Sortition::select(era, cfg.id, members, 1, &[*region])
						.ok_or(Error::<T>::UnseatableConfiguration)?;
					picked.extend(one);
				}
				picked
			} else {
				let pool = Self::candidates(cfg.id);
				T::Sortition::select(era, cfg.id, &pool, cfg.seats, &[])
					.ok_or(Error::<T>::UnseatableConfiguration)?
			};
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
