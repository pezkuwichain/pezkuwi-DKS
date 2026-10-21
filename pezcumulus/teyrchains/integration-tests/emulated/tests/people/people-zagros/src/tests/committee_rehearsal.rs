// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The first committee, rehearsed: drawn on People, installed on the relay, noted on the Asset Hub.
//!
//! The chain starts on its genesis validators and stays on them until the pool can seat a
//! committee -- five strata at their floors, which is two hundred and three eligible members at
//! the cheapest. On mainnet that takes months, so the handover from the genesis set to the first
//! drawn committee happens live, long after launch, on a path nothing has ever run end to end.
//! Real nodes cannot rehearse it: the draw would seat accounts that run no node and the chain
//! would stall. The emulator runs the real runtimes without consensus, so it can.
//!
//! The candidates are synthetic. Their standing is written straight into the storage each gate
//! reads -- a seat in the house, a place on the court, an education score, a community tiki, an
//! operating record -- and so are their relay keys and their Asset Hub bond. Everything after
//! that is the real path: members `join` through the extrinsic, the seed is committed and
//! revealed through the extrinsics, the era turns on the pallet's own clock, and the committee
//! leaves through `CommitteeToRelay` as the two messages production sends.
//!
//! Every one of those steps can fail quietly. `CommitteeSentToRelay` means the message left,
//! not that the relay took it; `Transact` reports a message as processed whether or not the call
//! inside it succeeded. So each chain is asked what it now holds, not what it was told.

use crate::imports::*;

use pezframe_support::{BoundedBTreeSet, BoundedVec};
use pezkuwi_tnpos_primitives::{invariant::SEATS_PER_STRATUM, StratumId};
use pezsp_runtime::traits::{BlakeTwo256, Hash};
use std::collections::{BTreeMap, BTreeSet};
use teyrchains_common::AccountId;

type PeopleRuntime = <PeopleZagros as Chain>::Runtime;
type RelayRuntime = <Zagros as Chain>::Runtime;
type AssetHubRuntime = <AssetHubZagros as Chain>::Runtime;

/// The five strata this rehearsal fills, and the first byte of their members' accounts.
///
/// Chosen because each gate reads a state this test can write and nothing on the chain
/// rewrites while the era runs: a seat in the house, a place on the court, an education score,
/// a community tiki and an operating record. Of the four left out, `WelatiLottery` and `Tenure`
/// read the trust score, which `Trust` recomputes for everyone on its own interval -- a written
/// score could be overwritten by the chain's own arithmetic partway through the era, and the
/// rehearsal would be measuring the batch schedule rather than the draw. `Stake` reads the
/// staking oracle at the oracle's own observation height, and `Geography` needs its members
/// spread over three regions for its rotation; neither changes what the handover does.
///
/// Five is the floor, not a convenience: with the court at three and four others at fifty this
/// is the smallest pool that can seat anything, which is exactly the pool mainnet will first
/// reach.
const POPULATED: [(StratumId, u8); 5] = [
	(StratumId::Meclis, 0xA1),
	(StratumId::Divan, 0xA2),
	(StratumId::Perwerde, 0xA3),
	(StratumId::Tiki, 0xA4),
	(StratumId::Infrastructure, 0xA5),
];

/// Pool members who contribute to the seed. More than one, so the mix is exercised; the round
/// needs only one honest contributor to be unpredictable.
const CONTRIBUTORS: usize = 3;

fn candidate(tag: u8, index: u32) -> AccountId {
	let mut raw = [0u8; 32];
	raw[0] = tag;
	raw[1..5].copy_from_slice(&index.to_le_bytes());
	// A fixed tail so these accounts cannot collide with the keyring's.
	raw[31] = 0x7E;
	raw.into()
}

fn preimage_of(who: &AccountId) -> [u8; 32] {
	BlakeTwo256::hash(<AccountId as AsRef<[u8]>>::as_ref(who)).0
}

/// What the gate of `stratum` reads, written for `who`.
fn give_standing(who: &AccountId, stratum: StratumId) {
	match stratum {
		StratumId::Meclis => {
			pezpallet_welati::ParliamentMembers::<PeopleRuntime>::mutate(|house| {
				house
					.try_push(pezpallet_welati::types::ParliamentMember {
						account: who.clone(),
						elected_at: 0,
						term_ends_at: u32::MAX,
						votes_participated: 0,
						total_votes_eligible: 0,
						participation_rate: 100,
						committees: Default::default(),
					})
					.expect("the house has two hundred and one seats; fifty fit")
			});
		},
		StratumId::Divan => {
			pezpallet_welati::DiwanMembers::<PeopleRuntime>::mutate(|court| {
				court
					.try_push(pezpallet_welati::types::DiwanMember {
						account: who.clone(),
						appointed_at: 0,
						term_ends_at: u32::MAX,
						appointed_by: pezpallet_welati::types::AppointmentAuthority::Parliament,
					})
					.expect("the court has eleven seats; three fit")
			});
		},
		StratumId::Perwerde => {
			pezpallet_perwerde::PerwerdeScores::<PeopleRuntime>::insert(who, 100u32);
		},
		StratumId::Tiki => {
			// A community tiki. An office would score nothing here -- `get_tiki_score` filters
			// offices out -- and the member would be refused at the door.
			let tikis: BoundedVec<
				pezpallet_tiki::Tiki,
				<PeopleRuntime as pezpallet_tiki::Config>::MaxTikisPerUser,
			> = vec![pezpallet_tiki::Tiki::Mela].try_into().expect("one tiki fits");
			pezpallet_tiki::UserTikis::<PeopleRuntime>::insert(who, tikis);
		},
		StratumId::Infrastructure => {
			let sessions =
				<PeopleRuntime as pezpallet_tnpos::Config>::InfrastructureSessions::get();
			pezpallet_tnpos::SeatedSessions::<PeopleRuntime>::insert(who, sessions);
		},
		other => panic!("{other:?} is not one of the strata this rehearsal fills"),
	}
}

/// Fill the pool to the floors, one member short in `short` if given, and open the seed round.
///
/// Returns every candidate with the stratum they joined.
fn fill_the_pool(short: Option<StratumId>) -> Vec<(AccountId, StratumId)> {
	// The relay's `ah_client` refuses a validator set unless it is `Active`, which the emulated
	// relay's genesis sets, as every real Zagros preset does.
	Zagros::execute_with(|| {
		assert_eq!(
			pezpallet_staking_async_ah_client::Mode::<RelayRuntime>::get(),
			pezpallet_staking_async_ah_client::OperatingMode::Active,
			"the relay would refuse the committee for a reason the live chain does not have"
		);
		assert!(
			pezpallet_staking_async_ah_client::ValidatorSet::<RelayRuntime>::get().is_none(),
			"the relay already holds a validator set; nothing below could show this one arriving"
		);
	});

	// The floors are read from the chain's own strata, not restated here: a rehearsal that
	// carried its own copy of fifty would go on passing after the chain's number moved.
	let strata = PeopleZagros::execute_with(|| pezpallet_tnpos::Strata::<PeopleRuntime>::get());
	let members: Vec<(AccountId, StratumId)> = POPULATED
		.iter()
		.flat_map(|&(stratum, tag)| {
			let floor = strata
				.iter()
				.find(|c| c.id == stratum)
				.unwrap_or_else(|| panic!("{stratum:?} is not configured on People"))
				.min_eligible;
			let count = if short == Some(stratum) { floor - 1 } else { floor };
			(0..count).map(move |i| (candidate(tag, i), stratum))
		})
		.collect();

	// Every candidate validates on the Asset Hub. That is what the bond report below says on
	// People's side, and it is what `SetValidatorCountToSeatedValidators` looks for before it
	// sizes the election to the committee.
	AssetHubZagros::execute_with(|| {
		for (who, _) in &members {
			pezpallet_staking_async::Validators::<AssetHubRuntime>::insert(
				who,
				pezpallet_staking_async::ValidatorPrefs::default(),
			);
		}
		assert!(
			pezpallet_komite::Committee::<AssetHubRuntime>::get().is_none(),
			"the Asset Hub already holds a committee; nothing below could show this one arriving"
		);
	});

	PeopleZagros::execute_with(|| {
		type RuntimeOrigin = <PeopleZagros as Chain>::RuntimeOrigin;

		let era_length = <PeopleRuntime as pezpallet_tnpos::Config>::EraLength::get();
		let now = <PeopleZagros as Chain>::System::block_number();
		assert!(
			now < pezpallet_tnpos::EraStart::<PeopleRuntime>::get() + era_length / 2,
			"block {now} is past the commit half of the first round"
		);

		// The Asset Hub's report of who is bonded there, as `note_bonded` would store it.
		let bonded: BoundedBTreeSet<
			AccountId,
			<PeopleRuntime as pezpallet_tnpos::Config>::MaxBonded,
		> = members
			.iter()
			.map(|(who, _)| who.clone())
			.collect::<BTreeSet<_>>()
			.try_into()
			.expect("two hundred and three fit in the report bound");
		pezpallet_tnpos::BondedOnAssetHub::<PeopleRuntime>::put(bonded);

		for (who, stratum) in &members {
			// The register's record that relay keys are held. The real call needs an ownership
			// proof signed by six relay session keys and sends one message per member; what
			// `join` and the draw read is this entry, and the keys themselves only matter to
			// the relay's session pallet, which this rehearsal does not rotate.
			let keys: BoundedVec<u8, pezframe_support::traits::ConstU32<512>> =
				vec![0u8; 192].try_into().expect("192 bytes fit in 512");
			pezpallet_tnpos::RelayKeys::<PeopleRuntime>::insert(who, keys);
			give_standing(who, *stratum);

			// Through the extrinsic: the bond, the keys and the stratum's gate are all checked
			// here exactly as they are for a real applicant.
			assert_ok!(pezpallet_tnpos::Pezpallet::<PeopleRuntime>::join(
				RuntimeOrigin::signed(who.clone()),
				*stratum,
			));
		}

		for (who, _) in members.iter().take(CONTRIBUTORS) {
			assert_ok!(pezpallet_tnpos::Pezpallet::<PeopleRuntime>::commit_seed(
				RuntimeOrigin::signed(who.clone()),
				BlakeTwo256::hash(&preimage_of(who)).0,
			));
		}
	});

	members
}

/// Reveal the seed in the second half of the era and stop one block short of its end.
///
/// The clock is moved by setting the block number between blocks rather than by producing six
/// hours of them: nothing in TNPoS reads anything but the number, and the emulator spends real
/// time on every block it builds. The next `PeopleZagros::execute_with` after this builds the
/// boundary block, and the pallet's own `on_initialize` turns the era there.
fn reveal_and_reach_the_boundary(members: &[(AccountId, StratumId)]) {
	let era_length = PeopleZagros::ext_wrapper(|| {
		let era_length = <PeopleRuntime as pezpallet_tnpos::Config>::EraLength::get();
		let start = pezpallet_tnpos::EraStart::<PeopleRuntime>::get();
		<PeopleZagros as Chain>::System::set_block_number(start + era_length / 2);
		era_length
	});

	PeopleZagros::execute_with(|| {
		type RuntimeOrigin = <PeopleZagros as Chain>::RuntimeOrigin;
		for (who, _) in members.iter().take(CONTRIBUTORS) {
			assert_ok!(pezpallet_tnpos::Pezpallet::<PeopleRuntime>::reveal_seed(
				RuntimeOrigin::signed(who.clone()),
				preimage_of(who),
			));
		}
		assert!(
			pezpallet_tnpos::NextSeed::<PeopleRuntime>::get().is_some_and(|(_, era)| era == 1),
			"the revealed contributions did not build a seed for era one"
		);
	});

	PeopleZagros::ext_wrapper(|| {
		let start = pezpallet_tnpos::EraStart::<PeopleRuntime>::get();
		<PeopleZagros as Chain>::System::set_block_number(start + era_length - 1);
	});
}

#[test]
fn the_first_committee_is_drawn_installed_and_noted() {
	let members = fill_the_pool(None);
	let stratum_of: BTreeMap<AccountId, StratumId> = members.iter().cloned().collect();
	reveal_and_reach_the_boundary(&members);

	let committee: Vec<AccountId> = PeopleZagros::execute_with(|| {
		type RuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;
		let floor = pezkuwi_tnpos_primitives::invariant::MIN_COMMITTEE;

		assert_expected_events!(
			PeopleZagros,
			vec![
				RuntimeEvent::Tnpos(pezpallet_tnpos::Event::CommitteeSeated { era, size, .. }) => {
					era: *era == 1,
					size: *size == floor,
				},
				RuntimeEvent::Tnpos(pezpallet_tnpos::Event::CommitteeSentToRelay { era, size }) => {
					era: *era == 1,
					size: *size == floor,
				},
			]
		);
		assert_eq!(pezpallet_tnpos::CurrentEra::<PeopleRuntime>::get(), 1);

		let committee = pezpallet_tnpos::CurrentCommittee::<PeopleRuntime>::get().to_vec();
		assert_eq!(committee.len() as u32, floor, "the committee is not the size it announced");

		// Each seated stratum carries exactly its seats -- no more, because seats are never
		// handed across strata, and no fewer, because a short draw is refused rather than
		// seated. Read from the members' own pool records, not from the setup's list.
		let mut seats: BTreeMap<StratumId, u32> = BTreeMap::new();
		for who in &committee {
			let stratum =
				pezpallet_tnpos::PoolMembers::<PeopleRuntime>::get(who).unwrap_or_else(|| {
					panic!("{who:?} sits on the committee without being in the pool")
				});
			assert_eq!(stratum_of.get(who), Some(&stratum), "{who:?} sits for the wrong stratum");
			*seats.entry(stratum).or_default() += 1;
		}
		assert_eq!(
			seats.len() as u32,
			pezkuwi_tnpos_primitives::invariant::MIN_STRATA,
			"seats came from {seats:?}"
		);
		assert!(seats.values().all(|&n| n == SEATS_PER_STRATUM), "seats came from {seats:?}");

		committee
	});

	// The relay: the message arrived, `ah_client` accepted it, and the set it will hand to the
	// session pallet is this committee and nobody else.
	Zagros::execute_with(|| {
		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			Zagros,
			vec![
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RuntimeEvent::StakingAhClient(
					pezpallet_staking_async_ah_client::Event::ValidatorSetReceived {
						id, new_validator_set_count, leftover, ..
					}
				) => {
					id: *id == 1,
					new_validator_set_count: *new_validator_set_count == committee.len() as u32,
					leftover: !*leftover,
				},
			]
		);

		let mut expected = committee.clone();
		expected.sort();
		assert_eq!(
			pezpallet_staking_async_ah_client::ValidatorSet::<RelayRuntime>::get(),
			Some((1, expected)),
			"the relay holds a different set from the one People drew"
		);
	});

	// The Asset Hub: the snapshot it pays by is this committee, in the order People drew it,
	// and the election is sized to it.
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::Komite(pezpallet_komite::Event::CommitteeNoted { era, members }) => {
					era: *era == 1,
					members: *members == committee.len() as u32,
				},
			]
		);

		let snapshot = pezpallet_komite::Committee::<AssetHubRuntime>::get()
			.expect("the Asset Hub noted no committee");
		assert_eq!(snapshot.era, 1);
		let noted: Vec<AccountId> = snapshot.members.iter().map(|(who, _)| who.clone()).collect();
		assert_eq!(noted, committee, "the Asset Hub holds a different committee from People's");
		assert_eq!(
			pezpallet_staking_async::ValidatorCount::<AssetHubRuntime>::get(),
			committee.len() as u32,
			"the election is not sized to the committee"
		);
	});
}

/// One member short anywhere is no committee at all, and nothing leaves People.
///
/// The general floor is fifty, and here the education stratum has forty-nine. It stands down,
/// four strata remain, and four is below `MIN_STRATA`: the era is refused rather than seated
/// with fewer independent gates than the security budget was computed for. The genesis set
/// must go on validating, so the relay and the Asset Hub must hear nothing.
#[test]
fn one_member_short_of_a_floor_seats_nobody() {
	assert_short_stratum_refuses(StratumId::Perwerde);
}

/// The court's floor is its seat count, three, and two judges are one short of it.
///
/// Separate from the general floor because it is the one exception to it (`min_eligible_for`),
/// and an exception is where an off-by-one hides: a court of two seated would be a stratum
/// whose draw is no draw at all.
#[test]
fn a_court_one_judge_short_seats_nobody() {
	assert_short_stratum_refuses(StratumId::Divan);
}

fn assert_short_stratum_refuses(short: StratumId) {
	let members = fill_the_pool(Some(short));
	reveal_and_reach_the_boundary(&members);

	PeopleZagros::execute_with(|| {
		type RuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;
		assert_expected_events!(
			PeopleZagros,
			vec![
				RuntimeEvent::Tnpos(pezpallet_tnpos::Event::SeatingRefused { era }) => {
					era: *era == 0,
				},
			]
		);
		let sent = <PeopleZagros as Chain>::events().iter().any(|e| {
			matches!(
				e,
				RuntimeEvent::Tnpos(
					pezpallet_tnpos::Event::CommitteeSeated { .. }
						| pezpallet_tnpos::Event::CommitteeSentToRelay { .. }
				)
			)
		});
		assert!(!sent, "a committee was seated with {short:?} below its floor");
		assert_eq!(pezpallet_tnpos::CurrentEra::<PeopleRuntime>::get(), 0);
		assert!(pezpallet_tnpos::CurrentCommittee::<PeopleRuntime>::get().is_empty());
	});

	Zagros::execute_with(|| {
		assert!(
			pezpallet_staking_async_ah_client::ValidatorSet::<RelayRuntime>::get().is_none(),
			"the relay was handed a validator set although People refused the era"
		);
	});

	AssetHubZagros::execute_with(|| {
		assert!(
			pezpallet_komite::Committee::<AssetHubRuntime>::get().is_none(),
			"the Asset Hub noted a committee although People refused the era"
		);
	});
}
