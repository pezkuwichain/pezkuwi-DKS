// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! What an offence costs.
//!
//! Six of the nine strata are entered without capital, so a purely monetary penalty would
//! either be no deterrent there or would put a price on gates that are meant not to have
//! one. What is taken instead is standing: the member leaves the pool and cannot return for
//! a fixed number of eras. For someone whose place here is their record, that is the
//! heavier loss. The stake stratum keeps its existing staking-async slashing on top.

use crate::*;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// Eras out of the pool for failing to take part.
pub const BAN_UNAVAILABLE: u32 = 24;
/// Eras out of the pool for signing two conflicting blocks.
pub const BAN_EQUIVOCATION: u32 = 360;

/// What a member did.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum Offence {
	/// Seated but did not vote. A failure.
	#[codec(index = 0)]
	Unavailable,
	/// Signed two conflicting blocks. An attack: this is the act that can fork the chain.
	#[codec(index = 1)]
	Equivocation,
}

impl Offence {
	/// Eras banned from the pool.
	pub const fn ban_eras(&self) -> u32 {
		match self {
			Offence::Unavailable => BAN_UNAVAILABLE,
			Offence::Equivocation => BAN_EQUIVOCATION,
		}
	}
}

impl<T: Config> Pezpallet<T> {
	pub(crate) fn do_report_offence(who: T::AccountId, offence: Offence) -> DispatchResult {
		if let Some(stratum) = PoolMembers::<T>::take(&who) {
			StratumSize::<T>::mutate(stratum, |n| *n = n.saturating_sub(1));
		}

		// Leave the seated committee too. A member who equivocated must stop counting
		// towards quorum immediately, not at the end of the era.
		let was_seated = CurrentCommittee::<T>::mutate(|c| {
			let before = c.len();
			c.retain(|m| m != &who);
			c.len() != before
		});

		let proposed = CurrentEra::<T>::get().saturating_add(offence.ban_eras());
		// A ban only ever lengthens. Recomputing it from whichever report arrived last would
		// let a trivial offence reported afterwards cut short the penalty for a serious one,
		// and equivocation costs fifteen times what going offline does.
		let banned_until =
			Banned::<T>::get(&who).map_or(proposed, |existing| existing.max(proposed));
		Banned::<T>::insert(&who, banned_until);
		Self::deposit_event(Event::Punished { who, offence, banned_until });

		// Tell the validating chain, or the removal only happened here. It keeps its own copy
		// and re-reads nothing; without this the offender goes on signing until the next era.
		if was_seated {
			let era = CurrentEra::<T>::get();
			let size = CurrentCommittee::<T>::decode_len().unwrap_or_default() as u32;
			let floor = pezkuwi_tnpos_primitives::invariant::MIN_COMMITTEE;
			if size < floor {
				Self::deposit_event(Event::CommitteeBelowSecurityFloor { era, size, floor });
			}
			Self::export_committee(era);
		}
		Ok(())
	}
}

// What follows guards the numbers above. A variant's index is what the chain wrote into
// storage; move it and the old bytes decode as a different offence -- no error, no crash, a
// different answer. Reading the source cannot catch that, because the source will look
// perfectly reasonable: variants sorted by severity, or a new one slotted in where it
// belongs by meaning.

#[cfg(test)]
mod stored_enum_encoding {
	use super::*;
	use scale_info::{TypeDef, TypeInfo};

	#[test]
	fn offence_indices_are_pinned() {
		let info = <Offence as TypeInfo>::type_info();
		let TypeDef::Variant(v) = &info.type_def else { panic!("Offence is not an enum") };
		let got: Vec<(String, u8)> =
			v.variants.iter().map(|x| (x.name.to_string(), x.index)).collect();
		let want = [("Unavailable", 0u8), ("Equivocation", 1)];
		assert_eq!(
			got.len(),
			want.len(),
			"Offence has {} variants, this list pins {} -- a new one needs a number of its \
			 own and a line here",
			got.len(),
			want.len()
		);
		for (i, (name, index)) in want.iter().enumerate() {
			assert_eq!((got[i].0.as_str(), got[i].1), (*name, *index), "Offence variant {i} moved");
		}
	}
}
