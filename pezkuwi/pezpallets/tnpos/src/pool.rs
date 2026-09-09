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
			StratumId::Meclis => {
				// The house itself, not a score anybody in the country can reach. Until now
				// this read trust like the four below, which meant the stratum claiming
				// parliamentary independence was filled by citizens who were not in
				// parliament: the name promised a separate authority and the gate delivered
				// the same one as five others.
				//
				// Two hundred and one seats against a floor of fifty, so it can be seated;
				// whether it is depends on fifty members running nodes, and if they do not it
				// stays empty and its seats are not redistributed. A stratum that cannot be
				// filled by the body it names should be empty rather than filled by somebody
				// else.
				ensure!(T::Scores::is_meclis_member(who), Error::<T>::NotEligible);
			},
			StratumId::Divan => {
				// The bench itself, for the same reason -- and with its own floor, because
				// eleven can never be fifty. That exception is defensible here and nowhere
				// else: a court seat cannot be manufactured, and six of eleven means holding
				// the house and the presidency together.
				//
				// If fewer than three judges run nodes the stratum is not seated, which is the
				// same graceful failure any short stratum has.
				ensure!(T::Scores::is_diwan_member(who), Error::<T>::NotEligible);
			},
			StratumId::Geography => {
				// An attested region, and nothing else. The register decides who has one: the
				// citizen claims it, a notary confirms it, and the court can cancel it. What
				// this gate reads is the settled answer.
				ensure!(T::Scores::region_of(who).is_some(), Error::<T>::NotEligible);
			},
			StratumId::WelatiLottery
			| StratumId::Tenure
			| StratumId::Infrastructure => {
				// Still trust, and still M7.1's work. Each needs a decision first, and they are
				// not the same shape: `WelatiLottery` reads "gated only by citizenship" in its
				// own definition, and citizenship is *weaker* than trust because trust gates on
				// stake -- moving to it would loosen this stratum. The other three have no
				// measured quantity behind them yet: there is no definition of a region, of
				// service, or of operational contribution to read.
				ensure!(fresh(T::Scores::trust_of(who))? > 0, Error::<T>::NotEligible);
			},
		}
		Ok(())
	}

	pub(crate) fn do_join(who: T::AccountId, stratum: StratumId) -> DispatchResult {
		if let Some(until) = Banned::<T>::get(&who) {
			ensure!(CurrentEra::<T>::get() >= until, Error::<T>::Banned);
			Banned::<T>::remove(&who);
		}

		ensure!(!PoolMembers::<T>::contains_key(&who), Error::<T>::AlreadyInPool);
		let size: u32 = StratumId::ALL
			.iter()
			.fold(0u32, |a, &s| a.saturating_add(StratumSize::<T>::get(s)));
		ensure!(size < T::MaxPoolSize::get(), Error::<T>::PoolFull);

		// Session drops a keyless validator silently on rotation, so an account with no
		// keys must never enter the pool in the first place: refusing here is the only
		// place the applicant sees the reason, rather than discovering it as an
		// unexplained absence from the committee.
		ensure!(T::HasSessionKeys::has_keys(&who), Error::<T>::NoSessionKeys);

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
