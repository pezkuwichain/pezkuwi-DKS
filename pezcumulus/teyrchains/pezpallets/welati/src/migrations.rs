// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Storage migrations for pezpallet-welati (Governance)

use super::*;
use pezframe_support::{
	traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};
use pezsp_std::marker::PhantomData;

/// Current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

/// Migration from v0 to v1
/// This migration handles the initial version setup for pezpallet-welati
pub mod v1 {
	use super::*;

	/// Type alias for the pre-upgrade state tuple to reduce type complexity
	#[cfg(feature = "try-runtime")]
	type PreUpgradeState = (u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32);

	pub struct MigrateToV1<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			let current = Pezpallet::<T>::on_chain_storage_version();

			log::info!("🔄 Running migration for pezpallet-welati from {current:?} to 1");

			if current == StorageVersion::new(0) {
				let mut weight = Weight::zero();

				// Example migration logic for governance storage
				// If storage format changes in the future, implement transformation here

				// Count existing storage items for logging
				let officials_count = pezpallet_tiki::TikiHolder::<T>::iter().count() as u64;
				let elections_count = ActiveElections::<T>::iter().count() as u64;
				let proposals_count = ActiveProposals::<T>::iter().count() as u64;

				let migrated = officials_count + elections_count + proposals_count;

				// Its own version, not the pallet's: v2 runs after it.
				StorageVersion::new(1).put::<Pezpallet<T>>();

				log::info!("✅ Migrated {migrated} entries in pezpallet-welati");
				log::info!(
					"   Officials: {officials_count}, Elections: {elections_count}, Proposals: {proposals_count}"
				);

				// Return weight used
				// Reads: all storage items + version read
				// Writes: version write
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(migrated + 1, 1));

				weight
			} else {
				log::info!(
					"👌 pezpallet-welati migration not needed, current version is {current:?}"
				);
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<pezsp_std::vec::Vec<u8>, pezsp_runtime::TryRuntimeError> {
			let current = Pezpallet::<T>::on_chain_storage_version();

			log::info!("🔍 Pre-upgrade check for pezpallet-welati");
			log::info!("   Current version: {current:?}");

			// Encode current storage counts for verification
			let officials_count = pezpallet_tiki::TikiHolder::<T>::iter().count() as u32;
			let parliament_count = ParliamentMembers::<T>::get().len() as u32;
			let diwan_count = DiwanMembers::<T>::get().len() as u32;
			let appointed_count = pezpallet_tiki::UserTikis::<T>::iter().count() as u32;
			let elections_count = ActiveElections::<T>::iter().count() as u32;
			let candidates_count = ElectionCandidates::<T>::iter().count() as u32;
			let votes_count = ElectionVotes::<T>::iter().count() as u32;
			let results_count = ElectionResults::<T>::iter().count() as u32;
			let districts_count = ElectoralDistrictConfig::<T>::iter().count() as u32;
			let nominations_count = PendingNominations::<T>::iter().count() as u32;
			let appointments_count = AppointmentProcesses::<T>::iter().count() as u32;
			let proposals_count = ActiveProposals::<T>::iter().count() as u32;
			let collective_votes_count = CollectiveVotes::<T>::iter().count() as u32;

			log::info!("   TikiHolder entries: {officials_count}");
			log::info!("   ParliamentMembers entries: {parliament_count}");
			log::info!("   DiwanMembers entries: {diwan_count}");
			log::info!("   UserTikis entries: {appointed_count}");
			log::info!("   ActiveElections entries: {elections_count}");
			log::info!("   ElectionCandidates entries: {candidates_count}");
			log::info!("   ElectionVotes entries: {votes_count}");
			log::info!("   ElectionResults entries: {results_count}");
			log::info!("   ElectoralDistrictConfig entries: {districts_count}");
			log::info!("   PendingNominations entries: {nominations_count}");
			log::info!("   AppointmentProcesses entries: {appointments_count}");
			log::info!("   ActiveProposals entries: {proposals_count}");
			log::info!("   CollectiveVotes entries: {collective_votes_count}");

			Ok((
				officials_count,
				parliament_count,
				diwan_count,
				appointed_count,
				elections_count,
				candidates_count,
				votes_count,
				results_count,
				districts_count,
				nominations_count,
				appointments_count,
				proposals_count,
				collective_votes_count,
			)
				.encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			state: pezsp_std::vec::Vec<u8>,
		) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use codec::Decode;

			let (
				pre_officials_count,
				pre_parliament_count,
				pre_diwan_count,
				pre_appointed_count,
				pre_elections_count,
				pre_candidates_count,
				pre_votes_count,
				pre_results_count,
				pre_districts_count,
				pre_nominations_count,
				pre_appointments_count,
				pre_proposals_count,
				pre_collective_votes_count,
			): PreUpgradeState = Decode::decode(&mut &state[..])
				.map_err(|_| "Failed to decode pre-upgrade state")?;

			log::info!("🔍 Post-upgrade check for pezpallet-welati");

			// Verify storage version was updated
			let current_version = Pezpallet::<T>::on_chain_storage_version();
			assert_eq!(
				current_version,
				StorageVersion::new(1),
				"Storage version not updated correctly"
			);
			log::info!("✅ Storage version updated to {current_version:?}");

			// Verify storage counts (should be same or more, never less)
			let post_officials_count = pezpallet_tiki::TikiHolder::<T>::iter().count() as u32;
			let post_parliament_count = ParliamentMembers::<T>::get().len() as u32;
			let post_diwan_count = DiwanMembers::<T>::get().len() as u32;
			let post_appointed_count = pezpallet_tiki::UserTikis::<T>::iter().count() as u32;
			let post_elections_count = ActiveElections::<T>::iter().count() as u32;
			let post_candidates_count = ElectionCandidates::<T>::iter().count() as u32;
			let post_votes_count = ElectionVotes::<T>::iter().count() as u32;
			let post_results_count = ElectionResults::<T>::iter().count() as u32;
			let post_districts_count = ElectoralDistrictConfig::<T>::iter().count() as u32;
			let post_nominations_count = PendingNominations::<T>::iter().count() as u32;
			let post_appointments_count = AppointmentProcesses::<T>::iter().count() as u32;
			let post_proposals_count = ActiveProposals::<T>::iter().count() as u32;
			let post_collective_votes_count = CollectiveVotes::<T>::iter().count() as u32;

			log::info!("   TikiHolder entries: {pre_officials_count} -> {post_officials_count}");
			log::info!(
				"   ParliamentMembers entries: {pre_parliament_count} -> {post_parliament_count}"
			);
			log::info!("   DiwanMembers entries: {pre_diwan_count} -> {post_diwan_count}");
			log::info!("   UserTikis entries: {pre_appointed_count} -> {post_appointed_count}");
			log::info!(
				"   ActiveElections entries: {pre_elections_count} -> {post_elections_count}"
			);
			log::info!(
				"   ElectionCandidates entries: {pre_candidates_count} -> {post_candidates_count}"
			);
			log::info!("   ElectionVotes entries: {pre_votes_count} -> {post_votes_count}");
			log::info!("   ElectionResults entries: {pre_results_count} -> {post_results_count}");
			log::info!("   ElectoralDistrictConfig entries: {pre_districts_count} -> {post_districts_count}");
			log::info!("   PendingNominations entries: {pre_nominations_count} -> {post_nominations_count}");
			log::info!("   AppointmentProcesses entries: {pre_appointments_count} -> {post_appointments_count}");
			log::info!(
				"   ActiveProposals entries: {pre_proposals_count} -> {post_proposals_count}"
			);
			log::info!("   CollectiveVotes entries: {pre_collective_votes_count} -> {post_collective_votes_count}");

			// Verify no data was lost
			assert!(
				post_officials_count >= pre_officials_count,
				"TikiHolder entries decreased during migration"
			);
			assert!(
				post_parliament_count >= pre_parliament_count,
				"ParliamentMembers entries decreased during migration"
			);
			assert!(
				post_diwan_count >= pre_diwan_count,
				"DiwanMembers entries decreased during migration"
			);
			assert!(
				post_appointed_count >= pre_appointed_count,
				"UserTikis entries decreased during migration"
			);
			assert!(
				post_elections_count >= pre_elections_count,
				"ActiveElections entries decreased during migration"
			);
			assert!(
				post_candidates_count >= pre_candidates_count,
				"ElectionCandidates entries decreased during migration"
			);
			assert!(
				post_votes_count >= pre_votes_count,
				"ElectionVotes entries decreased during migration"
			);
			assert!(
				post_results_count >= pre_results_count,
				"ElectionResults entries decreased during migration"
			);
			assert!(
				post_districts_count >= pre_districts_count,
				"ElectoralDistrictConfig entries decreased during migration"
			);
			assert!(
				post_nominations_count >= pre_nominations_count,
				"PendingNominations entries decreased during migration"
			);
			assert!(
				post_appointments_count >= pre_appointments_count,
				"AppointmentProcesses entries decreased during migration"
			);
			assert!(
				post_proposals_count >= pre_proposals_count,
				"ActiveProposals entries decreased during migration"
			);
			assert!(
				post_collective_votes_count >= pre_collective_votes_count,
				"CollectiveVotes entries decreased during migration"
			);

			log::info!("✅ Post-upgrade checks passed for pezpallet-welati");
			Ok(())
		}
	}
}

/// Example migration for future version changes
/// This demonstrates how to handle storage format changes in governance data
// `v2` stood here and was deleted rather than repaired.
//
// It transformed nothing -- its body was four comment lines and the word "template" -- and the
// one thing it did do was wrong: it wrote `StorageVersion::new(2)` while this pallet declares
// version 1. An on-chain version above the declared one is not a small inconsistency. FRAME
// compares the two, a real v2 migration would later see `current >= 2` and skip itself, and the
// transformation nobody wrote would be recorded as already done.
//
// It was scheduled in no runtime, which is the only reason it never fired. A placeholder that
// is harmless only because it is unreachable is one wiring away from being harmful, and the
// wiring is a single line somebody adds while tidying.
//
// The real v2 is below, and the version in `STORAGE_VERSION` moved with it in the same
// commit, on `pezpallet_tiki`'s shape: the pallet declares 2, v1 writes 1, v2 writes 2.

/// v1 -> v2: a founding house seated before the founding term was abolished keeps no term.
///
/// The founding house used to be seated with half an institutional term, recorded in
/// `TermEnds` and on every member. The term was abolished (2026-10-20: the founding house sits
/// until the register reaches the founding threshold and an election replaces it), but a
/// chain whose house was seated before then still carries it. Zagros does: its founding house
/// was seated at block 202 544 with a term ending at 10 714 544. Two things read that term.
/// The scheduler took it for an elected house and opened a Speaker's election that nobody in
/// the founding house can stand in or vote in, since none of them holds a seat's tiki; and the
/// first elected house would be given `10 714 544 + a full term` as its mandate, measured from
/// a previous term that never existed, instead of the first term the calendar sets.
///
/// So, while the house's first election is still owed: the term is removed, every member's
/// recorded term goes back to "not by the clock", and a Speaker's election that has drawn no
/// candidate is withdrawn. One that has drawn a candidate is left alone and logged -- a
/// candidacy carries a deposit, and withdrawing it is a decision, not a repair.
///
/// Nothing on Pezkuwichain needs it: its genesis seats the founding house with today's code
/// and writes version 2, so this never runs there.
pub mod v2 {
	use super::*;

	pub struct DropTheFoundingHousesTerm<T>(PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for DropTheFoundingHousesTerm<T> {
		fn on_runtime_upgrade() -> Weight {
			let current = Pezpallet::<T>::on_chain_storage_version();
			if current != StorageVersion::new(1) {
				log::info!("👌 pezpallet-welati v2 not needed, current version is {current:?}");
				return T::DbWeight::get().reads(1);
			}

			// version, FirstElectionOwed
			let mut reads = 2u64;
			// version
			let mut writes = 1u64;

			if Pezpallet::<T>::the_founding_house_sits() {
				// TermEnds for the house and for the chair, ParliamentMembers
				reads += 3;
				writes += 3;
				TermEnds::<T>::remove(ElectionType::Parliamentary);
				TermEnds::<T>::remove(ElectionType::SpeakerElection);
				ParliamentMembers::<T>::mutate(|members| {
					for member in members.iter_mut() {
						member.term_ends_at = BlockNumberFor::<T>::max_value();
					}
				});

				reads += 1;
				if let Some(id) = OpenElection::<T>::get(ElectionType::SpeakerElection) {
					reads += 1;
					if ElectionCandidates::<T>::iter_prefix(id).next().is_none() {
						writes += 2;
						ActiveElections::<T>::remove(id);
						OpenElection::<T>::remove(ElectionType::SpeakerElection);
						log::info!("pezpallet-welati v2: withdrew Speaker's election {id}");
					} else {
						log::warn!(
							"pezpallet-welati v2: Speaker's election {id} has candidates, left open"
						);
					}
				}
				log::info!("pezpallet-welati v2: the founding house's term is removed");
			}

			StorageVersion::new(2).put::<Pezpallet<T>>();
			T::DbWeight::get().reads_writes(reads, writes)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<pezsp_std::vec::Vec<u8>, pezsp_runtime::TryRuntimeError> {
			use codec::Encode;
			Ok((ParliamentMembers::<T>::get().len() as u32).encode())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(
			state: pezsp_std::vec::Vec<u8>,
		) -> Result<(), pezsp_runtime::TryRuntimeError> {
			use codec::Decode;
			use pezframe_support::ensure;

			let seated_before =
				u32::decode(&mut &state[..]).map_err(|_| "Failed to decode pre-upgrade state")?;
			let members = ParliamentMembers::<T>::get();
			ensure!(
				members.len() as u32 == seated_before,
				"the migration changed who sits in the house"
			);
			if Pezpallet::<T>::the_founding_house_sits() {
				ensure!(
					!TermEnds::<T>::contains_key(ElectionType::Parliamentary),
					"the founding house still carries a term"
				);
				ensure!(
					members.iter().all(|m| m.term_ends_at == BlockNumberFor::<T>::max_value()),
					"a founding member still carries a term"
				);
			}
			ensure!(
				Pezpallet::<T>::on_chain_storage_version() == StorageVersion::new(2),
				"pezpallet-welati is not at version 2"
			);
			Ok(())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{ExtBuilder, Test};
	use pezframe_support::traits::OnRuntimeUpgrade;

	#[test]
	fn test_migration_v1() {
		ExtBuilder::default().build().execute_with(|| {
			// Set initial storage version to 0
			StorageVersion::new(0).put::<Pezpallet<Test>>();

			// Run migration
			let weight = v1::MigrateToV1::<Test>::on_runtime_upgrade();

			// Verify version was updated
			assert_eq!(Pezpallet::<Test>::on_chain_storage_version(), StorageVersion::new(1));

			// Verify weight is non-zero
			assert!(weight != Weight::zero());
		});
	}

	#[test]
	fn test_migration_idempotent() {
		ExtBuilder::default().build().execute_with(|| {
			// Set current version
			STORAGE_VERSION.put::<Pezpallet<Test>>();

			// Run migration again
			let weight = v1::MigrateToV1::<Test>::on_runtime_upgrade();

			// Should be a no-op
			assert_eq!(weight, pezframe_support::weights::constants::RocksDbWeight::get().reads(1));
		});
	}

	mod v2_the_founding_houses_term {
		use super::*;
		use crate::{
			mock::{add_parliament_member, RuntimeOrigin, Welati},
			ActiveElections, CandidateInfo, ElectionCandidates, ElectionType, FirstElectionOwed,
			OpenElection, ParliamentMembers, TermEnds,
		};
		use pezframe_support::assert_ok;

		/// Zagros as it stood: the founding period over, the house's first election owed, and
		/// the founding house still carrying the term it was seated with.
		fn a_founding_house_with_a_term() {
			StorageVersion::new(1).put::<Pezpallet<Test>>();
			FirstElectionOwed::<Test>::insert(ElectionType::Parliamentary, ());
			for member in 7..=10u64 {
				add_parliament_member(member);
			}
			TermEnds::<Test>::insert(ElectionType::Parliamentary, 10_714_544);
		}

		fn speakers_election() -> u32 {
			assert_ok!(Welati::initiate_election(
				RuntimeOrigin::root(),
				ElectionType::SpeakerElection,
				None,
				None
			));
			OpenElection::<Test>::get(ElectionType::SpeakerElection).unwrap()
		}

		#[test]
		fn the_term_goes_and_the_empty_speakers_election_is_withdrawn() {
			ExtBuilder::default().build().execute_with(|| {
				a_founding_house_with_a_term();
				let id = speakers_election();

				v2::DropTheFoundingHousesTerm::<Test>::on_runtime_upgrade();

				assert!(!TermEnds::<Test>::contains_key(ElectionType::Parliamentary));
				assert!(ParliamentMembers::<Test>::get()
					.iter()
					.all(|m| m.term_ends_at == u64::MAX));
				assert_eq!(ParliamentMembers::<Test>::get().len(), 4, "nobody leaves the house");
				assert!(!OpenElection::<Test>::contains_key(ElectionType::SpeakerElection));
				assert!(ActiveElections::<Test>::get(id).is_none());
				assert_eq!(Pezpallet::<Test>::on_chain_storage_version(), StorageVersion::new(2));
			});
		}

		#[test]
		fn a_speakers_election_with_a_candidate_is_left_open() {
			ExtBuilder::default().build().execute_with(|| {
				a_founding_house_with_a_term();
				let id = speakers_election();
				ElectionCandidates::<Test>::insert(
					id,
					7u64,
					CandidateInfo {
						account: 7,
						district_id: None,
						registered_at: 1,
						endorsement_count: 0,
						vote_count: 0,
						deposit_paid: 0,
						campaign_data: Default::default(),
					},
				);

				v2::DropTheFoundingHousesTerm::<Test>::on_runtime_upgrade();

				assert!(!TermEnds::<Test>::contains_key(ElectionType::Parliamentary));
				assert_eq!(OpenElection::<Test>::get(ElectionType::SpeakerElection), Some(id));
				assert!(ActiveElections::<Test>::get(id).is_some());
			});
		}

		#[test]
		fn an_elected_house_keeps_its_term() {
			ExtBuilder::default().build().execute_with(|| {
				a_founding_house_with_a_term();
				// The first election has been counted: the house sitting now was elected.
				FirstElectionOwed::<Test>::remove(ElectionType::Parliamentary);

				v2::DropTheFoundingHousesTerm::<Test>::on_runtime_upgrade();

				assert_eq!(TermEnds::<Test>::get(ElectionType::Parliamentary), Some(10_714_544));
				assert!(ParliamentMembers::<Test>::get()
					.iter()
					.all(|m| m.term_ends_at != u64::MAX));
				assert_eq!(Pezpallet::<Test>::on_chain_storage_version(), StorageVersion::new(2));
			});
		}

		#[test]
		fn it_runs_once() {
			ExtBuilder::default().build().execute_with(|| {
				a_founding_house_with_a_term();
				STORAGE_VERSION.put::<Pezpallet<Test>>();

				v2::DropTheFoundingHousesTerm::<Test>::on_runtime_upgrade();

				assert_eq!(TermEnds::<Test>::get(ElectionType::Parliamentary), Some(10_714_544));
			});
		}
	}
}
