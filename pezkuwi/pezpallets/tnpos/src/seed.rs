// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Phase 1 randomness: a participatory commit-reveal.
//!
//! `RandomnessFromOneEpochAgo` is computable by everyone an epoch ahead and can be nudged
//! by whoever authors the epoch's last blocks -- both fatal for a draw whose whole value is
//! that nobody can see it coming. Commit-reveal is unpredictable as long as one contributor
//! is honest, which is enough to run on Zagros while ring-VRF lands in phase 2.
//!
//! A round belongs to exactly one era and has two non-overlapping halves: commits only in
//! the first half, reveals only in the second. Without that split a member could wait for
//! everyone else to reveal and then commit and reveal in the same block with a preimage
//! chosen to land the seed where they want -- that is not withholding, it is picking the
//! committee. The seed itself is spent the moment its era is drawn (see `sample.rs`):
//! every preimage is public the instant it is revealed, so carrying a spent seed into
//! another era would let anyone compute that era's committee in advance.

use crate::*;
use pezsp_io::hashing::blake2_256;

impl<T: Config> Pezpallet<T> {
	/// The era a contribution made now is building a seed for, and whether that round's
	/// commit half is still open.
	fn round(now: BlockNumberFor<T>) -> (u32, bool) {
		let half = T::EraLength::get() / 2u32.into();
		let commit_open = now < EraStart::<T>::get().saturating_add(half);
		(CurrentEra::<T>::get().saturating_add(1), commit_open)
	}

	pub(crate) fn do_commit_seed(who: T::AccountId, hash: [u8; 32]) -> DispatchResult {
		ensure!(PoolMembers::<T>::contains_key(&who), Error::<T>::NotInPool);
		let (era, commit_open) = Self::round(pezframe_system::Pezpallet::<T>::block_number());
		// The commit half must close before any reveal is seen. Without that deadline a
		// member could wait for everyone else to reveal, then commit and reveal in one
		// block with a preimage chosen to land the seed where they want -- that is not
		// withholding, it is picking the committee.
		ensure!(commit_open, Error::<T>::CommitWindowClosed);
		ensure!(!SeedCommitments::<T>::contains_key(era, &who), Error::<T>::AlreadyCommitted);
		SeedCommitments::<T>::insert(era, &who, hash);
		Ok(())
	}

	pub(crate) fn do_reveal_seed(who: T::AccountId, preimage: [u8; 32]) -> DispatchResult {
		let (era, commit_open) = Self::round(pezframe_system::Pezpallet::<T>::block_number());
		ensure!(!commit_open, Error::<T>::RevealWindowNotOpen);
		let commitment = SeedCommitments::<T>::take(era, &who).ok_or(Error::<T>::NoCommitment)?;
		ensure!(blake2_256(&preimage) == commitment, Error::<T>::BadReveal);

		// Mix rather than replace: a contributor who reveals last must not be able to pick
		// the outcome by choosing when to speak.
		NextSeed::<T>::mutate(|slot| {
			let base = match slot {
				Some((seed, e)) if *e == era => *seed,
				_ => [0u8; 32],
			};
			let mut buf = [0u8; 64];
			buf[..32].copy_from_slice(&base);
			buf[32..].copy_from_slice(&preimage);
			*slot = Some((blake2_256(&buf), era));
		});
		Ok(())
	}
}

/// Phase 1 `Sortition`: mixed commit-reveal seed, spent through `sample_k`.
pub struct CommitRevealSortition<T>(core::marker::PhantomData<T>);

impl<T: Config> pezkuwi_tnpos_primitives::sortition::Sortition<T::AccountId>
	for CommitRevealSortition<T>
{
	fn select(
		era: u32,
		stratum: StratumId,
		candidates: &[T::AccountId],
		k: u32,
	) -> Option<Vec<T::AccountId>> {
		// No contribution this era means no draw. Refusing degrades the committee; the
		// alternative -- a predictable fallback seed -- would hand the draw to whoever
		// stayed silent.
		//
		// A seed belongs to one era. An era with no round of its own draws nothing rather
		// than reusing a value everyone can already compute.
		let (base, seed_era) = NextSeed::<T>::get()?;
		if seed_era != era {
			return None;
		}
		let mut buf = [0u8; 36];
		buf[..32].copy_from_slice(&base);
		buf[32..].copy_from_slice(&era.to_le_bytes());
		let seed = blake2_256(&buf);
		Some(pezkuwi_tnpos_primitives::sortition::sample_k(candidates, k, &seed, &[stratum as u8]))
	}
}
