// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Joining a stratum.
//!
//! Every gate here is *measured*. The pallet this replaces read the applicant's own
//! declared stake out of the call arguments and compared it against a constant, which let
//! anyone enter the stake stratum for nothing.

use crate::*;

impl<T: Config> Pezpallet<T> {
	/// Whether `who` passes `stratum`'s gate right now.
	///
	/// `ScoreUnavailable` and `NotEligible` are distinct on purpose: the first says the
	/// chain cannot see the account's standing, the second says it has seen it and the
	/// answer is no. Collapsing them would let an outage read as a judgement.
	pub(crate) fn eligible_for(who: &T::AccountId, stratum: StratumId) -> Result<(), Error<T>> {
		let now = pezframe_system::Pezpallet::<T>::block_number();
		let age = T::MaxScoreAge::get();
		let fresh = |s: pezkuwi_tnpos_primitives::scores::ScoreSnapshot<BlockNumberFor<T>>| {
			s.value_if_fresh(now, age).ok_or(Error::<T>::ScoreUnavailable)
		};

		match stratum {
			StratumId::Stake => {
				// Rank inside this stratum is Phragmen's job on Asset Hub; the gate here is
				// only that the account has a staking standing at all.
				ensure!(fresh(T::Scores::staking_of(who))? > 0, Error::<T>::NotEligible);
			},
			StratumId::Perwerde => {
				ensure!(fresh(T::Scores::perwerde_of(who))? > 0, Error::<T>::NotEligible);
			},
			StratumId::Tiki => {
				// Community tikis only. `tiki_of` must exclude the twelve office tikis;
				// including them would tie this stratum to Meclis.
				ensure!(fresh(T::Scores::tiki_of(who))? > 0, Error::<T>::NotEligible);
			},
			StratumId::Meclis
			| StratumId::Divan
			| StratumId::WelatiLottery
			| StratumId::Geography
			| StratumId::Tenure
			| StratumId::Infrastructure => {
				// These gates are attested by their own authorities and reach this chain as
				// trust standing until their dedicated channels land in M7.1.
				ensure!(fresh(T::Scores::trust_of(who))? > 0, Error::<T>::NotEligible);
			},
		}
		Ok(())
	}

	pub(crate) fn do_join(who: T::AccountId, stratum: StratumId) -> DispatchResult {
		ensure!(!PoolMembers::<T>::contains_key(&who), Error::<T>::AlreadyInPool);
		let size: u32 = StratumId::ALL
			.iter()
			.fold(0u32, |a, &s| a.saturating_add(StratumSize::<T>::get(s)));
		ensure!(size < T::MaxPoolSize::get(), Error::<T>::PoolFull);

		Self::eligible_for(&who, stratum)?;

		PoolMembers::<T>::insert(&who, stratum);
		StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_add(1));
		Self::deposit_event(Event::Joined { who, stratum });
		Ok(())
	}

	pub(crate) fn do_leave(who: T::AccountId) -> DispatchResult {
		let stratum = PoolMembers::<T>::take(&who).ok_or(Error::<T>::NotInPool)?;
		StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_sub(1));
		Self::deposit_event(Event::Left { who });
		Ok(())
	}
}
