// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{
	mock::{
		add_parliament_member, endorsed_by, install_prime_minister, last_event, make_citizen,
		run_to_block, seat_president, AirdropCeiling, ExtBuilder, LargeAirdropDelay, RuntimeEvent,
		RuntimeOrigin, System, Test, Welati,
	},
	types::*,
	AirdropProposals, Error, Event as WelatiEvent, GovernmentPosition, NextAirdropId,
};
use pezframe_support::{assert_noop, assert_ok, BoundedVec};
use pezpallet_tiki::Tiki;
use pezsp_runtime::{traits::BadOrigin, DispatchResult};

// ===== ELECTION SYSTEM TESTS =====

#[test]
fn initiate_election_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));

		let expected_event = RuntimeEvent::Welati(WelatiEvent::ElectionStarted {
			election_id: 0,
			election_type: ElectionType::Presidential,
			start_block: 1,
			// Read from the constants rather than written out: the numbers themselves are a
			// runtime choice, and a test that hardcodes them fails when the choice changes
			// without anything actually being wrong.
			end_block: 1
				+ crate::mock::CandidacyPeriod::get()
				+ crate::mock::CampaignPeriod::get()
				+ crate::mock::ElectionPeriod::get(),
		});
		assert_eq!(last_event(), expected_event);

		assert!(Welati::active_elections(0).is_some());
		assert_eq!(Welati::next_election_id(), 1);
	});
}

#[test]
fn initiate_election_fails_for_non_root() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			Welati::initiate_election(
				RuntimeOrigin::signed(1),
				ElectionType::Presidential,
				None,
				None,
			),
			BadOrigin
		);
	});
}

#[test]
fn register_candidate_works_for_parliamentary() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		let parliamentary_endorsers = endorsed_by(0, 1, (2..=51).collect());

		assert_ok!(Welati::register_candidate(
			RuntimeOrigin::signed(1),
			0,
			None,
			parliamentary_endorsers,
		));

		assert_eq!(
			last_event(),
			RuntimeEvent::Welati(WelatiEvent::CandidateRegistered {
				election_id: 0,
				candidate: 1,
				deposit_paid: 10_000,
			})
		);

		assert!(Welati::election_candidates(0, 1).is_some());
	});
}

#[test]
fn register_candidate_fails_insufficient_endorsements() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));

		let endorsers = endorsed_by(0, 1, vec![2, 3, 4]);

		assert_noop!(
			Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers,),
			Error::<Test>::InsufficientEndorsements
		);
	});
}

#[test]
fn register_candidate_fails_after_deadline() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// Support is gathered while candidacies are open, which is the only time it can be
		// given -- so this happens before the deadline, and only the registration is late.
		let endorsers = endorsed_by(0, 1, (2..=51).collect());

		run_to_block(crate::mock::CandidacyPeriod::get() + 2);

		assert_noop!(
			Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers,),
			Error::<Test>::CandidacyPeriodExpired
		);
	});
}

#[test]
fn register_candidate_fails_already_candidate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		let endorsers = endorsed_by(0, 1, (2..=51).collect());

		assert_ok!(Welati::register_candidate(
			RuntimeOrigin::signed(1),
			0,
			None,
			endorsers.clone(),
		));

		assert_noop!(
			Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers,),
			Error::<Test>::AlreadyCandidate
		);
	});
}

#[test]
fn cast_vote_works() {
	ExtBuilder::default().build().execute_with(|| {
		// 1. Start the election
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// 2. Register a candidate (account 1)
		let endorsers = endorsed_by(0, 1, (3..=52).collect());
		assert_ok!(Welati::register_candidate(
			RuntimeOrigin::signed(1), // Candidate
			0,                        // Election ID
			None,                     // District ID
			endorsers,
		));

		// 3. Advance to the voting period
		run_to_block(crate::mock::CandidacyPeriod::get() + crate::mock::CampaignPeriod::get() + 1);

		// 4. Cast a vote (account 2 votes for candidate 1)
		let candidates_to_vote_for = vec![1];
		assert_ok!(Welati::cast_vote(
			RuntimeOrigin::signed(2),       // Voter
			0,                              // Election ID
			candidates_to_vote_for.clone(), // Candidate(s) voted for
			None,                           // District ID
		));

		// 5. Verify the event and storage state
		assert_eq!(
			last_event(),
			RuntimeEvent::Welati(WelatiEvent::VoteCast {
				election_id: 0,
				voter: 2,
				candidates: candidates_to_vote_for,
				district_id: None,
			})
		);
		assert!(Welati::election_votes(0, 2).is_some());
	});
}

#[test]
fn cast_vote_fails_already_voted() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		let endorsers = endorsed_by(0, 1, (3..=52).collect());
		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers,));

		run_to_block(crate::mock::CandidacyPeriod::get() + crate::mock::CampaignPeriod::get() + 1);

		let candidates = vec![1];

		assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(2), 0, candidates.clone(), None,));

		assert_noop!(
			Welati::cast_vote(RuntimeOrigin::signed(2), 0, candidates, None,),
			Error::<Test>::AlreadyVoted
		);
	});
}

#[test]
fn cast_vote_fails_wrong_period() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		let candidates = vec![1];

		assert_noop!(
			Welati::cast_vote(RuntimeOrigin::signed(2), 0, candidates, None,),
			Error::<Test>::VotingPeriodNotStarted
		);
	});
}

#[test]
fn finalize_election_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// A candidate and enough voters to clear the 40% Parliamentary quorum
		// (MockTrustProvider's fixed citizen_count() is 110, so 44+ votes).
		let endorsers = endorsed_by(0, 1, (200..=249).collect());
		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers,));

		run_to_block(crate::mock::CandidacyPeriod::get() + crate::mock::CampaignPeriod::get() + 1);
		for voter in 300..=343u64 {
			assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(voter), 0, vec![1], None,));
		}

		// Move past the election end date
		// candidacy + campaign + voting, then past the end
		run_to_block(
			crate::mock::CandidacyPeriod::get()
				+ crate::mock::CampaignPeriod::get()
				+ crate::mock::ElectionPeriod::get()
				+ 10,
		); // +10 for extra safety

		assert_ok!(Welati::finalize_election(RuntimeOrigin::signed(99), 0,));

		if let Some(election) = Welati::active_elections(0) {
			assert_eq!(election.status, ElectionStatus::Completed);
		}
	});
}

// ===== APPOINTMENT SYSTEM TESTS =====

/// The account the tests seat as Serok, and the one they seat in Parliament.
///
/// They are different people on purpose. An appointment has two parties in this state, and a
/// fixture that let one account play both would test nothing about the separation.
const SEROK: u64 = 1;
const MP: u64 = 42;

/// Seat a Parliament that can confirm, and a Serok who can nominate.
fn seat_the_two_bodies() {
	pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Serok, SEROK);
	add_parliament_member(MP);
}

/// Carry an appointment through whichever body confirms that particular office.
///
/// Which body it is belongs to the office rather than to the caller: this asks the same
/// `requires_parliament_approval` the pallet branches on, so a test cannot drift from the
/// pallet by hard-coding the wrong route.
fn install(process_id: u32, role: OfficialRole) -> DispatchResult {
	if role.requires_parliament_approval() {
		Welati::confirm_appointment(RuntimeOrigin::signed(MP), process_id)
	} else {
		Welati::approve_appointment(RuntimeOrigin::signed(SEROK), process_id)
	}
}

#[test]
fn nominate_official_works() {
	ExtBuilder::default().build().execute_with(|| {
		// Setup: Make user 1 the Serok (President) so they can nominate
		pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Serok, 1);

		let justification = b"Qualified candidate".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(1),
			2,
			OfficialRole::Dadger,
			justification,
		));

		assert_eq!(Welati::next_appointment_id(), 1);
	});
}

#[test]
fn approve_appointment_works() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		// A Noter is a clerk of the executive rather than a check on it, so the office's own
		// rule puts the whole appointment in the President's signature.
		let justification = b"Qualified candidate".to_vec().try_into().unwrap();
		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			2,
			OfficialRole::Noter,
			justification,
		));

		assert_ok!(Welati::approve_appointment(RuntimeOrigin::signed(SEROK), 0,));
	});
}

#[test]
fn a_judge_is_not_seated_by_the_president_alone() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		let justification = b"Qualified candidate".to_vec().try_into().unwrap();
		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			2,
			OfficialRole::Dadger,
			justification,
		));

		// The office carries its own rule, and the President is not the body that rule names.
		// He put the process on the parliamentary track by nominating; his own signature no
		// longer reaches it.
		assert_noop!(
			Welati::approve_appointment(RuntimeOrigin::signed(SEROK), 0),
			Error::<Test>::AppointmentAlreadyProcessed
		);
		assert!(!pezpallet_tiki::UserTikis::<Test>::get(2).contains(&Tiki::Dadger));

		assert_ok!(Welati::confirm_appointment(RuntimeOrigin::signed(MP), 0));
		assert!(pezpallet_tiki::UserTikis::<Test>::get(2).contains(&Tiki::Dadger));
	});
}

#[test]
fn a_stranger_cannot_confirm_and_neither_can_the_president() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		let justification = b"Qualified candidate".to_vec().try_into().unwrap();
		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			2,
			OfficialRole::Dadger,
			justification,
		));

		assert_noop!(Welati::confirm_appointment(RuntimeOrigin::signed(999), 0), BadOrigin);
		assert_noop!(Welati::confirm_appointment(RuntimeOrigin::signed(SEROK), 0), BadOrigin);
	});
}

#[test]
fn nobody_is_both_parties_to_their_own_appointment() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		let justification = b"I am qualified".to_vec().try_into().unwrap();
		assert_noop!(
			Welati::nominate_official(
				RuntimeOrigin::signed(SEROK),
				SEROK,
				OfficialRole::Xezinedar,
				justification,
			),
			Error::<Test>::CannotNominateSelf
		);
	});
}

#[test]
fn the_record_says_which_body_agreed_to_seat_them() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();
		run_to_block(9);

		let reason = || b"Qualified candidate".to_vec().try_into().unwrap();

		// The President's own track names the person, because there the decision is one
		// person's.
		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			2,
			OfficialRole::Noter,
			reason(),
		));
		assert_ok!(Welati::approve_appointment(RuntimeOrigin::signed(SEROK), 0));
		let clerk = Welati::appointment_processes(0).expect("recorded");
		assert_eq!(clerk.confirmed_by, Some(ConfirmedBy::Appointer(SEROK)));
		assert_eq!(clerk.confirmed_at, Some(9));

		// Parliament's track names the House, not the member who submitted the call. A
		// register that recorded MP as the decision would be recording a clerk.
		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			3,
			OfficialRole::Dadger,
			reason(),
		));
		assert_ok!(Welati::confirm_appointment(RuntimeOrigin::signed(MP), 1));
		let judge = Welati::appointment_processes(1).expect("recorded");
		assert_eq!(judge.confirmed_by, Some(ConfirmedBy::Parliament));
		assert_eq!(judge.confirmed_at, Some(9));
	});
}

#[test]
fn the_legislature_draws_the_line_and_the_president_cannot() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();
		let reason = || b"Qualified candidate".to_vec().try_into().unwrap();

		// A Noter needs nobody's consent at founding.
		assert!(!Welati::confirmation_is_required(&OfficialRole::Noter));

		// The executive cannot decide which of his own appointments need consent.
		assert_noop!(
			Welati::set_confirmation_requirement(
				RuntimeOrigin::signed(SEROK),
				OfficialRole::Noter,
				true
			),
			BadOrigin
		);

		// The House can, and the next nomination lands on the parliamentary track.
		assert_ok!(Welati::set_confirmation_requirement(
			RuntimeOrigin::signed(MP),
			OfficialRole::Noter,
			true
		));
		assert!(Welati::confirmation_is_required(&OfficialRole::Noter));

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			2,
			OfficialRole::Noter,
			reason(),
		));
		assert_noop!(
			Welati::approve_appointment(RuntimeOrigin::signed(SEROK), 0),
			Error::<Test>::AppointmentAlreadyProcessed
		);
		assert_ok!(Welati::confirm_appointment(RuntimeOrigin::signed(MP), 0));

		// Putting it back stores nothing: the map holds departures from the constitution, so
		// an office restored to its founding rule leaves no entry behind.
		assert_ok!(Welati::set_confirmation_requirement(
			RuntimeOrigin::signed(MP),
			OfficialRole::Noter,
			false
		));
		assert_eq!(crate::ConfirmationRequired::<Test>::get(OfficialRole::Noter), None);
		assert!(!Welati::confirmation_is_required(&OfficialRole::Noter));
	});
}

#[test]
fn moving_the_line_does_not_move_a_nomination_already_in_flight() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();
		let reason = || b"Qualified candidate".to_vec().try_into().unwrap();

		// A judge is nominated while judges need Parliament.
		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			2,
			OfficialRole::Dadger,
			reason(),
		));

		// Parliament then exempts the office. That is the law from here on, and it is not
		// how this pending nomination gets seated -- otherwise a confirmation requirement
		// could be erased one nomination at a time, after the fact.
		assert_ok!(Welati::set_confirmation_requirement(
			RuntimeOrigin::signed(MP),
			OfficialRole::Dadger,
			false
		));
		assert_noop!(
			Welati::approve_appointment(RuntimeOrigin::signed(SEROK), 0),
			Error::<Test>::AppointmentAlreadyProcessed
		);
		assert_ok!(Welati::confirm_appointment(RuntimeOrigin::signed(MP), 0));
	});
}

#[test]
fn the_president_names_the_prime_minister_and_the_house_seats_him() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::signed(SEROK), 2));

		// Naming is not seating. Until the House acts there is no head of government, and the
		// President cannot supply the second signature himself.
		assert_eq!(pezpallet_tiki::TikiHolder::<Test>::get(Tiki::SerokWeziran), None);
		assert_noop!(Welati::confirm_prime_minister(RuntimeOrigin::signed(SEROK)), BadOrigin);

		assert_ok!(Welati::confirm_prime_minister(RuntimeOrigin::signed(MP)));
		assert_eq!(pezpallet_tiki::TikiHolder::<Test>::get(Tiki::SerokWeziran), Some(2));
		assert_eq!(crate::PendingPrimeMinister::<Test>::get(), None);
	});
}

#[test]
fn the_house_can_refuse_the_prime_minister_out_loud() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::signed(SEROK), 2));
		assert_ok!(Welati::reject_prime_minister(RuntimeOrigin::signed(MP)));

		assert_eq!(pezpallet_tiki::TikiHolder::<Test>::get(Tiki::SerokWeziran), None);
		// Refusing consumes the nomination, so a later confirmation cannot resurrect it.
		assert_noop!(
			Welati::confirm_prime_minister(RuntimeOrigin::signed(MP)),
			Error::<Test>::NoNomineeStanding
		);

		// The President may put a different name forward, which is the whole remedy.
		assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::signed(SEROK), 3));
		assert_ok!(Welati::confirm_prime_minister(RuntimeOrigin::signed(MP)));
		assert_eq!(pezpallet_tiki::TikiHolder::<Test>::get(Tiki::SerokWeziran), Some(3));
	});
}

#[test]
fn naming_again_replaces_the_standing_nominee() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::signed(SEROK), 2));
		assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::signed(SEROK), 3));

		// Withdrawing is the only power the President keeps over a name already sent, and it
		// works by sending another one -- he never reaches the seat itself.
		assert_ok!(Welati::confirm_prime_minister(RuntimeOrigin::signed(MP)));
		assert_eq!(pezpallet_tiki::TikiHolder::<Test>::get(Tiki::SerokWeziran), Some(3));
	});
}

/// The court's separation is the split, not a confirmation on top of it.
///
/// Six seats are the house's -- it elects them from candidates who stand for themselves, and
/// the President reaches none of them. Five are the President's, and he seats them with his
/// own signature. Each half is chosen by a body the other does not control.
///
/// A parliamentary confirmation on the President's five was written and reverted. It would
/// have let the house touch all eleven seats while its own six stayed beyond the President's
/// reach: one-sided, not separated. This test holds the decision, so the confirmation cannot
/// come back as a tidy-up.
#[test]
fn the_president_seats_his_five_and_the_house_cannot_reach_them() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();
		make_citizen(2);
		pezpallet_tiki::UserTikis::<Test>::mutate(2, |tikis| {
			let _ = tikis.try_push(Tiki::Hiquqnas);
		});

		// One call, one signature, one judge.
		assert_ok!(Welati::appoint_diwan_member(RuntimeOrigin::signed(SEROK), 2));
		assert_eq!(Welati::diwan_members().len(), 1);

		// And the house has no call that reaches this half of the bench: a member of
		// parliament is not an appointing authority here.
		assert_noop!(
			Welati::appoint_diwan_member(RuntimeOrigin::signed(MP), 3),
			Error::<Test>::NotAuthorizedToNominate
		);
	});
}

/// The emission rate is the Treasurer's, and the finance minister cannot reach it.
///
/// This is the separation the office exists for. `WezîrêDarayiyê` draws against what Parliament
/// appropriated; `Xezinedar` decides what comes into being. One hand spends, the other creates,
/// and a state where they are the same hand pays its bills by printing.
#[test]
fn only_the_treasurer_moves_the_emission_rate() {
	ExtBuilder::default().build().execute_with(|| {
		let treasurer = 20u64;
		let minister = 21u64;
		make_citizen(treasurer);
		make_citizen(minister);
		pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Xezinedar, treasurer);
		pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::WezireDarayiye, minister);

		let rate = pezsp_runtime::Perbill::from_percent(8);

		assert_noop!(
			Welati::set_emission_rate(RuntimeOrigin::signed(minister), rate),
			Error::<Test>::NotTheTreasurer
		);
		assert_noop!(
			Welati::set_emission_rate(RuntimeOrigin::signed(SEROK), rate),
			Error::<Test>::NotTheTreasurer
		);
		assert_noop!(Welati::set_emission_rate(RuntimeOrigin::root(), rate), BadOrigin);

		assert_ok!(Welati::set_emission_rate(RuntimeOrigin::signed(treasurer), rate));
		assert_eq!(crate::EmissionRate::<Test>::get().map(|(r, _)| r), Some(rate));
	});
}

/// A mandate cannot be spent in one call.
///
/// The ceiling on the treasury chain says how high the rate may ever go. Without a limit on the
/// step, the ceiling is the only limit and one call reaches it -- which is the difference
/// between an office with a mandate and an office with a lever.
#[test]
fn the_rate_moves_by_steps_and_not_in_one_leap() {
	ExtBuilder::default().build().execute_with(|| {
		let treasurer = 20u64;
		make_citizen(treasurer);
		pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Xezinedar, treasurer);
		let pct = pezsp_runtime::Perbill::from_percent;

		// The first setting has nothing to step from -- there is no rate on record yet, so
		// there is no distance to measure. Every one after it is bounded.
		assert_ok!(Welati::set_emission_rate(RuntimeOrigin::signed(treasurer), pct(8)));

		run_to_block(50);
		assert_noop!(
			Welati::set_emission_rate(RuntimeOrigin::signed(treasurer), pct(10)),
			Error::<Test>::EmissionStepTooLarge
		);
		// Down is bounded the same way: starving the chain's security is a move too.
		assert_noop!(
			Welati::set_emission_rate(RuntimeOrigin::signed(treasurer), pct(4)),
			Error::<Test>::EmissionStepTooLarge
		);

		assert_ok!(Welati::set_emission_rate(RuntimeOrigin::signed(treasurer), pct(9)));
		assert_eq!(crate::EmissionRate::<Test>::get().map(|(r, _)| r), Some(pct(9)));
	});
}

/// And not twice in quick succession.
///
/// A rate moved faster than its own effect can be observed is not policy. The interval is what
/// makes the step limit mean something: without it, two calls in two blocks cover the same
/// ground one large call would have.
#[test]
fn the_rate_cannot_be_moved_twice_in_quick_succession() {
	ExtBuilder::default().build().execute_with(|| {
		let treasurer = 20u64;
		make_citizen(treasurer);
		pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Xezinedar, treasurer);
		let pct = pezsp_runtime::Perbill::from_percent;

		assert_ok!(Welati::set_emission_rate(RuntimeOrigin::signed(treasurer), pct(8)));

		run_to_block(System::block_number() + 3);
		assert_noop!(
			Welati::set_emission_rate(RuntimeOrigin::signed(treasurer), pct(9)),
			Error::<Test>::EmissionChangedTooRecently
		);

		run_to_block(System::block_number() + 10);
		assert_ok!(Welati::set_emission_rate(RuntimeOrigin::signed(treasurer), pct(9)));
	});
}

// ===== COLLECTIVE DECISION TESTS =====

#[test]
fn submit_proposal_works() {
	ExtBuilder::default().build().execute_with(|| {
		let title = b"Test Proposal".to_vec().try_into().unwrap();
		let description = b"Test proposal description".to_vec().try_into().unwrap();

		// CRITICAL FIX: Use the helper function
		add_parliament_member(1);

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title,
			description,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		));

		assert_eq!(Welati::next_proposal_id(), 1);
		assert!(Welati::active_proposals(0).is_some());
	});
}

#[test]
fn vote_on_proposal_works() {
	ExtBuilder::default().build().execute_with(|| {
		let title = b"Test Proposal".to_vec().try_into().unwrap();
		let description = b"Test proposal description".to_vec().try_into().unwrap();

		// CRITICAL FIX: Use the helper functions
		add_parliament_member(1);
		add_parliament_member(2);

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title,
			description,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		));

		let proposal = Welati::active_proposals(0).unwrap();
		run_to_block(proposal.voting_starts_at + 1);

		let rationale = Some(b"Good proposal".to_vec().try_into().unwrap());

		assert_ok!(Welati::vote_on_proposal(
			RuntimeOrigin::signed(2),
			0,
			VoteChoice::Aye,
			rationale,
		));

		assert!(Welati::collective_votes(0, 2).is_some());
	});
}

// ===== HELPER FUNCTION TESTS =====

#[test]
fn get_required_trust_score_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Welati::get_required_trust_score(&ElectionType::Presidential), 250);

		assert_eq!(Welati::get_required_trust_score(&ElectionType::Parliamentary), 100);

		assert_eq!(Welati::get_required_trust_score(&ElectionType::ConstitutionalCourt), 275);
	});
}

#[test]
fn get_required_endorsements_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Welati::get_required_endorsements(&ElectionType::Presidential), 100);

		assert_eq!(Welati::get_required_endorsements(&ElectionType::Parliamentary), 50);

		assert_eq!(Welati::get_required_endorsements(&ElectionType::SpeakerElection), 0);
	});
}

#[test]
fn get_minimum_turnout_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Welati::get_minimum_turnout(&ElectionType::Presidential), 50);

		assert_eq!(Welati::get_minimum_turnout(&ElectionType::Parliamentary), 40);

		assert_eq!(Welati::get_minimum_turnout(&ElectionType::SpeakerElection), 30);
	});
}

#[test]
fn one_citizen_is_one_vote_in_every_election() {
	// The Diwan and the Speaker used to be elected on a trust-weighted ballot, up to ten
	// votes for one person. Standing decides who may stand; it must not decide what a vote
	// weighs. The weighting also broke the quorum: turnout counted weights against a head
	// count, so a hundred voters at weight ten cleared a thousand-vote bar.
	ExtBuilder::default().build().execute_with(|| {
		for election_type in [
			ElectionType::Presidential,
			ElectionType::Parliamentary,
			ElectionType::SpeakerElection,
			ElectionType::ConstitutionalCourt,
		] {
			assert_eq!(Welati::calculate_vote_weight(&1, &election_type), 1);
		}

		// And it does not depend on who is asking: an account with every component maxed
		// counts for exactly as much as one with nothing.
		assert_eq!(
			Welati::calculate_vote_weight(&1, &ElectionType::ConstitutionalCourt),
			Welati::calculate_vote_weight(&9_999, &ElectionType::ConstitutionalCourt)
		);
	});
}

#[test]
fn parliament_is_elected_at_the_midpoint_of_a_presidency() {
	// The stagger, and the one shortened term that creates it. Without this the two clocks
	// start together and stay together, and a single vote hands one side both the government
	// and the legislature for four years.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		crate::TermEnds::<Test>::remove(ElectionType::Presidential);

		let now = System::block_number();
		let term = crate::mock::TermLength::get();

		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), vec![2, 3]));
		let parliament_ends = crate::TermEnds::<Test>::get(ElectionType::Parliamentary).unwrap();
		assert_eq!(parliament_ends, now + term / 2, "the first house sits half a term");

		// Every house after it serves a full term, so the offset persists.
		crate::ParliamentMembers::<Test>::kill();
		crate::PendingSeatGrants::<Test>::kill();
		crate::PendingSeatRevokes::<Test>::kill();
		crate::PendingSeatTerm::<Test>::kill();
		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), vec![4, 5]));
		assert_eq!(
			crate::TermEnds::<Test>::get(ElectionType::Parliamentary).unwrap(),
			parliament_ends + term,
			"and the ones after it serve whole terms from that anchor"
		);
	});
}

// ===== ERROR CASE TESTS =====

#[test]
fn election_not_found_error_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			Welati::register_candidate(RuntimeOrigin::signed(1), 999, None, vec![2, 3],),
			Error::<Test>::ElectionNotFound
		);
	});
}

#[test]
fn proposal_not_found_error_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			Welati::vote_on_proposal(RuntimeOrigin::signed(1), 999, VoteChoice::Aye, None,),
			Error::<Test>::ProposalNotFound
		);
	});
}

// ===== INTEGRATION TESTS =====

#[test]
fn complete_election_cycle_works() {
	ExtBuilder::default().build().execute_with(|| {
		// 1. Start the election
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// 2. Register candidates
		let endorsers1 = endorsed_by(0, 1, (10..=59).collect());
		let endorsers2 = endorsed_by(0, 2, (60..=109).collect());

		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers1,));

		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(2), 0, None, endorsers2,));

		// 3. Move to the voting period
		run_to_block(crate::mock::CandidacyPeriod::get() + crate::mock::CampaignPeriod::get() + 1);

		// 4. Cast votes — enough to clear the 40% Parliamentary quorum
		// (MockTrustProvider's fixed citizen_count() is 110, so 44+ votes).
		assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(3), 0, vec![1], None,));

		assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(4), 0, vec![2], None,));

		for voter in 500..=541u64 {
			assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(voter), 0, vec![1], None,));
		}

		// 5. Finalize the election
		run_to_block(
			crate::mock::CandidacyPeriod::get()
				+ crate::mock::CampaignPeriod::get()
				+ crate::mock::ElectionPeriod::get()
				+ 2,
		);

		assert_ok!(Welati::finalize_election(RuntimeOrigin::signed(99), 0,));

		assert!(Welati::election_results(0).is_some());
	});
}

#[test]
fn complete_appointment_cycle_works() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		let justification = b"Experienced lawyer".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			5,
			OfficialRole::Dadger,
			justification,
		));

		// A judge is nominated by the President and confirmed by Parliament, and the record
		// walks both moments: it waits before it is approved.
		let waiting = Welati::appointment_processes(0).expect("the process was recorded");
		assert_eq!(waiting.status, AppointmentStatus::WaitingParliamentaryApproval);

		assert_ok!(Welati::confirm_appointment(RuntimeOrigin::signed(MP), 0));

		// `if let Some(..)` used to stand here, so a vanished process asserted nothing at all.
		let process = Welati::appointment_processes(0).expect("the process survived approval");
		assert_eq!(process.status, AppointmentStatus::Approved);
	});
}

#[test]
fn complete_proposal_cycle_works() {
	ExtBuilder::default().build().execute_with(|| {
		let title = b"Budget Amendment".to_vec().try_into().unwrap();
		let description = b"Increase education budget by 10%".to_vec().try_into().unwrap();

		// CRITICAL FIX: Use the helper functions
		add_parliament_member(1);
		add_parliament_member(2);
		add_parliament_member(3);

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title,
			description,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::High,
			None,
		));

		let proposal = Welati::active_proposals(0).unwrap();
		run_to_block(proposal.voting_starts_at + 1);

		assert_ok!(Welati::vote_on_proposal(RuntimeOrigin::signed(2), 0, VoteChoice::Aye, None,));

		assert_ok!(Welati::vote_on_proposal(RuntimeOrigin::signed(3), 0, VoteChoice::Aye, None,));

		if let Some(proposal) = Welati::active_proposals(0) {
			assert_eq!(proposal.aye_votes, 2);
		}
	});
}

// ===== RUNOFF ELECTION TESTS =====

#[test]
fn initiate_runoff_election_works() {
	ExtBuilder::default().build().execute_with(|| {
		let runoff_candidates: BoundedVec<u64, _> = vec![1, 2].try_into().unwrap();

		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			Some(runoff_candidates),
		));

		assert!(Welati::active_elections(0).is_some());
		assert!(Welati::election_candidates(0, 1).is_some());
		assert!(Welati::election_candidates(0, 2).is_some());

		if let Some(election) = Welati::active_elections(0) {
			assert_eq!(election.status, ElectionStatus::CampaignPeriod);
		}
	});
}

#[test]
fn runoff_election_fails_with_wrong_candidate_count() {
	ExtBuilder::default().build().execute_with(|| {
		let invalid_candidates: Result<BoundedVec<u64, _>, _> = vec![1, 2, 3].try_into();

		if let Ok(candidates) = invalid_candidates {
			assert_noop!(
				Welati::initiate_election(
					RuntimeOrigin::root(),
					ElectionType::Presidential,
					None,
					Some(candidates),
				),
				Error::<Test>::InvalidInitialCandidates
			);
		}
	});
}

#[test]
fn runoff_election_fails_for_non_presidential() {
	ExtBuilder::default().build().execute_with(|| {
		let runoff_candidates: BoundedVec<u64, _> = vec![1, 2].try_into().unwrap();

		assert_noop!(
			Welati::initiate_election(
				RuntimeOrigin::root(),
				ElectionType::Parliamentary,
				None,
				Some(runoff_candidates),
			),
			Error::<Test>::InvalidElectionType
		);
	});
}

// ============================================================================
// ELECTION SYSTEM - EDGE CASES (8 tests)
// ============================================================================

#[test]
fn initiate_election_with_districts() {
	ExtBuilder::default().build().execute_with(|| {
		let districts = vec![
			ElectoralDistrict {
				district_id: 1,
				name: b"District 1".to_vec().try_into().unwrap(),
				seat_count: 5,
				voter_population: 10_000,
				geographic_bounds: None,
			},
			ElectoralDistrict {
				district_id: 2,
				name: b"District 2".to_vec().try_into().unwrap(),
				seat_count: 3,
				voter_population: 6_000,
				geographic_bounds: None,
			},
		];

		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			Some(districts.clone()),
			None,
		));

		let election = Welati::active_elections(0).unwrap();
		assert_eq!(election.districts.len(), 2);
		assert_eq!(election.election_type, ElectionType::Parliamentary);
	});
}

#[test]
fn register_candidate_presidential_with_max_endorsements() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));

		// Presidential requires 100 endorsements
		let endorsers = endorsed_by(0, 1, (2..=101).collect());

		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers,));

		let candidate_info = Welati::election_candidates(0, 1).unwrap();
		assert_eq!(candidate_info.endorsers.len(), 100);
	});
}

#[test]
fn register_candidate_fails_election_not_found() {
	ExtBuilder::default().build().execute_with(|| {
		let endorsers = vec![2, 3];

		assert_noop!(
			Welati::register_candidate(
				RuntimeOrigin::signed(1),
				999, // Non-existent election
				None,
				endorsers,
			),
			Error::<Test>::ElectionNotFound
		);
	});
}

#[test]
fn cast_vote_multiple_candidates_parliamentary() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// Each candidate needs endorsers of their own: one citizen, one endorsement, per
		// election. Sharing a list between candidates -- which this test used to do -- is
		// exactly what the rule refuses, because support that can be given to everybody
		// distinguishes nobody.
		for candidate_id in 1..=3u64 {
			let base = 10 + (candidate_id - 1) * 50;
			let endorsers = endorsed_by(0, candidate_id, (base..base + 50).collect());
			assert_ok!(Welati::register_candidate(
				RuntimeOrigin::signed(candidate_id),
				0,
				None,
				endorsers,
			));
		}

		// Move to voting period
		run_to_block(crate::mock::CandidacyPeriod::get() + crate::mock::CampaignPeriod::get() + 2);

		// Vote for multiple candidates (parliamentary allows this)
		let candidates_to_vote = vec![1, 2, 3];
		assert_ok!(Welati::cast_vote(
			RuntimeOrigin::signed(100),
			0,
			candidates_to_vote.clone(),
			None,
		));

		let vote_info = Welati::election_votes(0, 100).unwrap();
		assert_eq!(vote_info.candidates.len(), 3);
	});
}

#[test]
fn cast_vote_fails_invalid_candidate() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		run_to_block(crate::mock::CandidacyPeriod::get() + crate::mock::CampaignPeriod::get() + 2);

		// Try to vote for non-existent candidate
		assert_noop!(
			Welati::cast_vote(RuntimeOrigin::signed(100), 0, vec![999], None,),
			Error::<Test>::ElectionNotFound
		);
	});
}

#[test]
fn cast_vote_with_district_id() {
	ExtBuilder::default().build().execute_with(|| {
		let districts = vec![ElectoralDistrict {
			district_id: 1,
			name: b"District 1".to_vec().try_into().unwrap(),
			seat_count: 5,
			voter_population: 10_000,
			geographic_bounds: None,
		}];

		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			Some(districts),
			None,
		));

		let endorsers = endorsed_by(0, 1, (2..=51).collect());
		assert_ok!(Welati::register_candidate(
			RuntimeOrigin::signed(1),
			0,
			Some(1), // District 1
			endorsers,
		));

		run_to_block(crate::mock::CandidacyPeriod::get() + crate::mock::CampaignPeriod::get() + 2);

		assert_ok!(Welati::cast_vote(
			RuntimeOrigin::signed(100),
			0,
			vec![1],
			Some(1), // Vote in District 1
		));

		let vote_info = Welati::election_votes(0, 100).unwrap();
		assert_eq!(vote_info.district_id, Some(1));
	});
}

#[test]
fn finalize_election_fails_not_started() {
	ExtBuilder::default().build().execute_with(|| {
		// Try to finalize non-existent election
		assert_noop!(
			Welati::finalize_election(RuntimeOrigin::signed(99), 999,),
			Error::<Test>::ElectionNotFound
		);
	});
}

#[test]
fn finalize_election_updates_election_status() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		let endorsers = endorsed_by(0, 1, (2..=51).collect());
		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers,));

		run_to_block(crate::mock::CandidacyPeriod::get() + crate::mock::CampaignPeriod::get() + 2);

		// Enough votes to clear the 40% Parliamentary quorum (MockTrustProvider's
		// fixed citizen_count() is 110, so 44+ votes).
		assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(100), 0, vec![1], None,));
		for voter in 600..=642u64 {
			assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(voter), 0, vec![1], None,));
		}

		run_to_block(
			crate::mock::CandidacyPeriod::get()
				+ crate::mock::CampaignPeriod::get()
				+ crate::mock::ElectionPeriod::get()
				+ 100,
		);

		assert_ok!(Welati::finalize_election(RuntimeOrigin::signed(99), 0,));

		let election = Welati::active_elections(0).unwrap();
		assert_eq!(election.status, ElectionStatus::Completed);
	});
}

// ============================================================================
// NOMINATION & APPOINTMENT SYSTEM (7 tests)
// ============================================================================

#[test]
fn nominate_official_fails_not_authorized() {
	ExtBuilder::default().build().execute_with(|| {
		// Regular user cannot nominate
		let justification = b"Test justification".to_vec().try_into().unwrap();

		assert_noop!(
			Welati::nominate_official(
				RuntimeOrigin::signed(999),
				2,
				OfficialRole::Dadger,
				justification,
			),
			Error::<Test>::NotAuthorizedToNominate
		);
	});
}

#[test]
fn a_bench_takes_more_than_one_judge_and_a_single_seat_takes_one() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		let reason = || b"Qualified candidate".to_vec().try_into().unwrap();
		let appoint = |who: u64, role: OfficialRole| {
			Welati::nominate_official(RuntimeOrigin::signed(SEROK), who, role, reason())?;
			let id = Welati::next_appointment_id() - 1;
			install(id, role)
		};

		// A state has more than one judge. Refusing the second was the register confusing
		// an office with the person holding it.
		assert_ok!(appoint(2, OfficialRole::Dadger));
		assert_ok!(appoint(3, OfficialRole::Dadger));
		assert!(pezpallet_tiki::UserTikis::<Test>::get(2).contains(&Tiki::Dadger));
		assert!(pezpallet_tiki::UserTikis::<Test>::get(3).contains(&Tiki::Dadger));

		// The Treasury has one seat, and `tiki` is where that is written down.
		assert_ok!(appoint(4, OfficialRole::Xezinedar));
		assert_noop!(appoint(5, OfficialRole::Xezinedar), Error::<Test>::RoleAlreadyFilled);
		assert_eq!(pezpallet_tiki::TikiHolder::<Test>::get(Tiki::Xezinedar), Some(4));
	});
}

#[test]
fn nominate_official_requires_president() {
	ExtBuilder::default().build().execute_with(|| {
		// Without president, cannot nominate officials
		let justification = b"Test justification".to_vec().try_into().unwrap();

		assert_noop!(
			Welati::nominate_official(
				RuntimeOrigin::signed(1),
				2,
				OfficialRole::Dadger,
				justification,
			),
			Error::<Test>::NotAuthorizedToNominate
		);
	});
}

#[test]
fn approve_appointment_fails_not_authorized() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		let justification = b"Qualified candidate".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			2,
			OfficialRole::Noter,
			justification,
		));

		let process_id = Welati::next_appointment_id() - 1;

		// Regular user cannot approve
		assert_noop!(
			Welati::approve_appointment(RuntimeOrigin::signed(999), process_id,),
			Error::<Test>::NotAuthorizedToApprove
		);
	});
}

#[test]
fn approve_appointment_fails_already_processed() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		let justification = b"Qualified candidate".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(SEROK),
			2,
			OfficialRole::Noter,
			justification,
		));

		let process_id = Welati::next_appointment_id() - 1;

		// First approval
		assert_ok!(Welati::approve_appointment(RuntimeOrigin::signed(SEROK), process_id,));

		// Try to approve again
		assert_noop!(
			Welati::approve_appointment(RuntimeOrigin::signed(SEROK), process_id,),
			Error::<Test>::AppointmentAlreadyProcessed
		);
	});
}

#[test]
fn approve_appointment_process_not_found() {
	ExtBuilder::default().build().execute_with(|| {
		pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Serok, 1);

		assert_noop!(
			Welati::approve_appointment(
				RuntimeOrigin::signed(1),
				999, // Non-existent process
			),
			Error::<Test>::AppointmentProcessNotFound
		);
	});
}

#[test]
fn nominate_and_approve_multiple_officials() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_two_bodies();

		// Two of these three offices answer to Parliament and one does not, which is the
		// point of running them together: the route is a property of the office.
		let officials = vec![
			(2, OfficialRole::Dadger),
			(3, OfficialRole::Dozger),
			(4, OfficialRole::Xezinedar),
		];

		for (nominee, role) in officials {
			let justification = b"Qualified candidate".to_vec().try_into().unwrap();

			assert_ok!(Welati::nominate_official(
				RuntimeOrigin::signed(SEROK),
				nominee,
				role,
				justification,
			));

			let process_id = Welati::next_appointment_id() - 1;
			assert_ok!(install(process_id, role));

			let process =
				Welati::appointment_processes(process_id).expect("the process was recorded");
			assert_eq!(process.status, AppointmentStatus::Approved);
		}
	});
}

// ============================================================================
// PROPOSAL & VOTING SYSTEM (5 tests)
// ============================================================================

#[test]
fn submit_proposal_fails_not_authorized() {
	ExtBuilder::default().build().execute_with(|| {
		// Regular user cannot submit proposal without being parliament member
		let title = b"Test proposal".to_vec().try_into().unwrap();
		let description = b"Test description".to_vec().try_into().unwrap();

		assert_noop!(
			Welati::submit_proposal(
				RuntimeOrigin::signed(999),
				title,
				description,
				CollectiveDecisionType::ParliamentSimpleMajority,
				ProposalPriority::Normal,
				None,
			),
			Error::<Test>::NotAuthorizedToPropose
		);
	});
}

#[test]
fn vote_on_proposal_fails_not_authorized() {
	ExtBuilder::default().build().execute_with(|| {
		// Add user to parliament
		add_parliament_member(1);

		let title = b"Test proposal".to_vec().try_into().unwrap();
		let description = b"Test description".to_vec().try_into().unwrap();

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title,
			description,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		));

		let proposal_id = Welati::next_proposal_id() - 1;

		// Non-parliament member cannot vote
		assert_noop!(
			Welati::vote_on_proposal(
				RuntimeOrigin::signed(999),
				proposal_id,
				VoteChoice::Aye,
				None,
			),
			Error::<Test>::NotAuthorizedToVote
		);
	});
}

#[test]
fn vote_on_proposal_fails_already_voted() {
	ExtBuilder::default().build().execute_with(|| {
		add_parliament_member(1);
		add_parliament_member(2);

		let title = b"Test proposal".to_vec().try_into().unwrap();
		let description = b"Test description".to_vec().try_into().unwrap();

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title,
			description,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		));

		let proposal_id = Welati::next_proposal_id() - 1;

		// Voting opens some blocks after a proposal is filed, so the house has time to read
		// it. The test used to vote immediately, which the pallet allowed because the window
		// it wrote onto the proposal was never checked.
		let opens_at = Welati::active_proposals(proposal_id).unwrap().voting_starts_at;
		run_to_block(opens_at + 1);

		// First vote
		assert_ok!(Welati::vote_on_proposal(
			RuntimeOrigin::signed(1),
			proposal_id,
			VoteChoice::Aye,
			None,
		));

		// Try to vote again
		assert_noop!(
			Welati::vote_on_proposal(RuntimeOrigin::signed(1), proposal_id, VoteChoice::Nay, None,),
			Error::<Test>::ProposalAlreadyVoted
		);
	});
}

#[test]
fn vote_on_proposal_fails_proposal_not_found() {
	ExtBuilder::default().build().execute_with(|| {
		add_parliament_member(1);

		assert_noop!(
			Welati::vote_on_proposal(
				RuntimeOrigin::signed(1),
				999, // Non-existent proposal
				VoteChoice::Aye,
				None,
			),
			Error::<Test>::ProposalNotFound
		);
	});
}

#[test]
fn proposal_with_multiple_votes() {
	ExtBuilder::default().build().execute_with(|| {
		// Add 5 parliament members
		for i in 1..=5 {
			add_parliament_member(i);
		}

		let title = b"Test proposal".to_vec().try_into().unwrap();
		let description = b"Test description".to_vec().try_into().unwrap();

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title,
			description,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		));

		let proposal_id = Welati::next_proposal_id() - 1;
		let opens_at = Welati::active_proposals(proposal_id).unwrap().voting_starts_at;
		run_to_block(opens_at + 1);

		// Multiple votes: 3 aye, 1 nay, 1 abstain
		assert_ok!(Welati::vote_on_proposal(
			RuntimeOrigin::signed(1),
			proposal_id,
			VoteChoice::Aye,
			None
		));
		assert_ok!(Welati::vote_on_proposal(
			RuntimeOrigin::signed(2),
			proposal_id,
			VoteChoice::Aye,
			None
		));
		assert_ok!(Welati::vote_on_proposal(
			RuntimeOrigin::signed(3),
			proposal_id,
			VoteChoice::Aye,
			None
		));
		assert_ok!(Welati::vote_on_proposal(
			RuntimeOrigin::signed(4),
			proposal_id,
			VoteChoice::Nay,
			None
		));
		assert_ok!(Welati::vote_on_proposal(
			RuntimeOrigin::signed(5),
			proposal_id,
			VoteChoice::Abstain,
			None
		));

		// Verify all votes recorded
		assert!(Welati::collective_votes(proposal_id, 1).is_some());
		assert!(Welati::collective_votes(proposal_id, 2).is_some());
		assert!(Welati::collective_votes(proposal_id, 3).is_some());
		assert!(Welati::collective_votes(proposal_id, 4).is_some());
		assert!(Welati::collective_votes(proposal_id, 5).is_some());
	});
}

// ============================================================================
// INTEGRATION & STORAGE TESTS (5 tests)
// ============================================================================

#[test]
fn storage_consistency_multi_election() {
	ExtBuilder::default().build().execute_with(|| {
		// Create multiple elections
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));

		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// Verify storage consistency
		assert!(Welati::active_elections(0).is_some());
		assert!(Welati::active_elections(1).is_some());
		assert_eq!(Welati::next_election_id(), 2);

		let election_0 = Welati::active_elections(0).unwrap();
		let election_1 = Welati::active_elections(1).unwrap();

		assert_eq!(election_0.election_id, 0);
		assert_eq!(election_1.election_id, 1);
		assert_eq!(election_0.election_type, ElectionType::Presidential);
		assert_eq!(election_1.election_type, ElectionType::Parliamentary);
	});
}

#[test]
fn multiple_candidates_same_election() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// Ten candidates, each with their own fifty endorsers.
		for candidate_id in 1..=10u64 {
			let base = 100 + (candidate_id - 1) * 50;
			let endorsers = endorsed_by(0, candidate_id, (base..base + 50).collect());
			assert_ok!(Welati::register_candidate(
				RuntimeOrigin::signed(candidate_id),
				0,
				None,
				endorsers,
			));
		}

		// Verify all candidates registered
		for candidate_id in 1..=10 {
			assert!(Welati::election_candidates(0, candidate_id).is_some());
		}

		let election = Welati::active_elections(0).unwrap();
		assert_eq!(election.candidates.len(), 10);
	});
}

#[test]
fn election_id_increments_correctly() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Welati::next_election_id(), 0);

		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));
		assert_eq!(Welati::next_election_id(), 1);

		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));
		assert_eq!(Welati::next_election_id(), 2);

		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));
		assert_eq!(Welati::next_election_id(), 3);
	});
}

#[test]
fn appointment_id_increments_correctly() {
	ExtBuilder::default().build().execute_with(|| {
		pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Serok, 1);

		assert_eq!(Welati::next_appointment_id(), 0);

		let justification1 = b"Qualified candidate".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(1),
			2,
			OfficialRole::Dadger,
			justification1,
		));
		assert_eq!(Welati::next_appointment_id(), 1);

		let justification2 = b"Another qualified candidate".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(1),
			3,
			OfficialRole::Dozger,
			justification2,
		));
		assert_eq!(Welati::next_appointment_id(), 2);
	});
}

#[test]
fn proposal_id_increments_correctly() {
	ExtBuilder::default().build().execute_with(|| {
		add_parliament_member(1);

		assert_eq!(Welati::next_proposal_id(), 0);

		let title1 = b"Proposal 1".to_vec().try_into().unwrap();
		let description1 = b"First proposal".to_vec().try_into().unwrap();

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title1,
			description1,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		));
		assert_eq!(Welati::next_proposal_id(), 1);

		let title2 = b"Proposal 2".to_vec().try_into().unwrap();
		let description2 = b"Second proposal".to_vec().try_into().unwrap();

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title2,
			description2,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		));
		assert_eq!(Welati::next_proposal_id(), 2);
	});
}

// ============================================================================
// Additional Tests to reach 53 total tests (3 new tests)
// ============================================================================

#[test]
fn multiple_elections_different_types() {
	ExtBuilder::default().build().execute_with(|| {
		pezframe_system::Pezpallet::<Test>::set_block_number(1);

		// Start presidential election
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));

		// Start parliamentary election
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// Both elections should be active
		assert!(Welati::active_elections(0).is_some());
		assert!(Welati::active_elections(1).is_some());

		// Elections should have different types
		let election0 = Welati::active_elections(0).unwrap();
		let election1 = Welati::active_elections(1).unwrap();

		assert_eq!(election0.election_type, ElectionType::Presidential);
		assert_eq!(election1.election_type, ElectionType::Parliamentary);

		// Next election ID should be 2
		assert_eq!(Welati::next_election_id(), 2);
	});
}

#[test]
fn sequential_elections_id_increment() {
	ExtBuilder::default().build().execute_with(|| {
		pezframe_system::Pezpallet::<Test>::set_block_number(1);

		// Initial ID should be 0
		assert_eq!(Welati::next_election_id(), 0);

		// Create first election
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));

		assert_eq!(Welati::next_election_id(), 1);

		// Create second election
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		assert_eq!(Welati::next_election_id(), 2);

		// Verify both elections exist
		assert!(Welati::active_elections(0).is_some());
		assert!(Welati::active_elections(1).is_some());
	});
}

#[test]
fn proposal_and_election_storage_independent() {
	ExtBuilder::default().build().execute_with(|| {
		pezframe_system::Pezpallet::<Test>::set_block_number(1);
		add_parliament_member(1);

		// Create a proposal
		let title = b"Test Proposal".to_vec().try_into().unwrap();
		let desc = b"Test Description".to_vec().try_into().unwrap();

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title,
			desc,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		));

		// Create an election
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));

		// Verify both storages are independent
		assert_eq!(Welati::next_proposal_id(), 1);
		assert_eq!(Welati::next_election_id(), 1);

		// Verify both exist
		assert!(Welati::active_proposals(0).is_some());
		assert!(Welati::active_elections(0).is_some());

		// Create another proposal
		let title2 = b"Second Proposal".to_vec().try_into().unwrap();
		let desc2 = b"Second Description".to_vec().try_into().unwrap();

		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			title2,
			desc2,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::High,
			None,
		));

		// Proposal ID incremented, election ID unchanged
		assert_eq!(Welati::next_proposal_id(), 2);
		assert_eq!(Welati::next_election_id(), 1);
	});
}

// ===== THE POPULATION GATE =====
//
// The citizen register is here; the money is on the Asset Hub. Nobody there can see how many
// citizens the state has, so they have to be told -- once, when it first passes the threshold.

mod population_gate {
	use super::*;
	use crate::{
		mock::{clear_sent_xcm, sent_xcm, set_citizen_count},
		PopulationGateReported,
	};

	#[test]
	fn nothing_is_sent_below_the_threshold() {
		ExtBuilder::default().build().execute_with(|| {
			set_citizen_count(99); // the mock threshold is 100
			clear_sent_xcm();

			run_to_block(100); // ten check periods

			assert!(!PopulationGateReported::<Test>::get());
			assert!(
				sent_xcm().is_empty(),
				"the treasury was told before there was anything to tell"
			);
		});
	}

	#[test]
	fn crossing_the_threshold_sends_exactly_one_message() {
		ExtBuilder::default().build().execute_with(|| {
			set_citizen_count(99);
			clear_sent_xcm();
			run_to_block(30);
			assert!(sent_xcm().is_empty());

			set_citizen_count(100);
			run_to_block(40);

			assert!(PopulationGateReported::<Test>::get());
			assert_eq!(sent_xcm().len(), 1);
		});
	}

	#[test]
	fn the_latch_does_not_turn_back() {
		// A state whose population dips below the threshold does not stop paying its citizens.
		// Nor does it send a second message: the treasury has already started, and a stream of
		// era-by-era messages that change nothing is a stream nobody would notice going wrong.
		ExtBuilder::default().build().execute_with(|| {
			set_citizen_count(100);
			clear_sent_xcm();
			run_to_block(20);
			assert_eq!(sent_xcm().len(), 1);

			set_citizen_count(50);
			run_to_block(100);

			assert!(PopulationGateReported::<Test>::get());
			assert_eq!(sent_xcm().len(), 1, "the gate reported more than once");
		});
	}

	#[test]
	fn the_message_addresses_the_treasury_chain_and_its_activation_call() {
		ExtBuilder::default().build().execute_with(|| {
			set_citizen_count(100);
			clear_sent_xcm();
			run_to_block(20);

			let (destination, message) = sent_xcm().pop().expect("nothing was sent");
			assert_eq!(destination, crate::mock::TreasuryChain::get());

			let transact = message
				.into_iter()
				.find_map(|i| match i {
					xcm::latest::Instruction::Transact { call, .. } => Some(call),
					_ => None,
				})
				.expect("no Transact in the message");

			// Pallet 70, call 0: `pez_treasury::activate_distribution`, and no arguments --
			// the treasury is being told a fact, not handed parameters.
			assert_eq!(transact.into_encoded(), vec![70u8, 0u8]);
		});
	}

	#[test]
	fn the_check_only_runs_on_era_boundaries() {
		// The point of the period is that this is not on the hot path. If the gate ran every
		// block it would read the register once per block for the entire life of the chain
		// before the threshold, and once per block forever after it.
		ExtBuilder::default().build().execute_with(|| {
			set_citizen_count(100);
			clear_sent_xcm();

			run_to_block(9); // period is 10, so no boundary has been crossed
			assert!(sent_xcm().is_empty());

			run_to_block(10);
			assert_eq!(sent_xcm().len(), 1);
		});
	}
}

// ===== THE APPOINTMENT CHAIN =====
//
// The President names the Prime Minister; the Prime Minister names the cabinet. What limits
// the President is not who he may appoint but that the minister he appoints cannot write his
// own budget -- Parliament approves it, and the finance minister only spends what was
// approved. These tests are that chain, link by link.

mod appointments {
	use super::*;
	use crate::mock::{holder_of, make_citizen, seat_president};
	use pezpallet_tiki::Tiki;

	const SEROK: u64 = 1;
	const PM: u64 = 2;
	const MINISTER: u64 = 3;
	const OUTSIDER: u64 = 4;

	#[test]
	fn the_president_appoints_the_prime_minister() {
		ExtBuilder::default().build().execute_with(|| {
			seat_president(SEROK);
			make_citizen(PM);

			install_prime_minister(RuntimeOrigin::signed(SEROK), PM);

			assert_eq!(holder_of(Tiki::SerokWeziran), Some(PM));
		});
	}

	#[test]
	fn nobody_else_appoints_the_prime_minister() {
		ExtBuilder::default().build().execute_with(|| {
			seat_president(SEROK);
			make_citizen(PM);

			assert_noop!(
				Welati::appoint_prime_minister(RuntimeOrigin::signed(OUTSIDER), PM),
				Error::<Test>::NotAuthorizedToNominate
			);
			assert_eq!(holder_of(Tiki::SerokWeziran), None);
		});
	}

	#[test]
	fn root_may_still_appoint_while_sudo_exists() {
		// The one arm that is meant to go away. When sudo is removed this test is the thing
		// that has to be deleted, deliberately, rather than a behaviour that quietly changes.
		ExtBuilder::default().build().execute_with(|| {
			make_citizen(PM);
			install_prime_minister(RuntimeOrigin::root(), PM);
			assert_eq!(holder_of(Tiki::SerokWeziran), Some(PM));
		});
	}

	#[test]
	fn the_prime_minister_appoints_ministers_and_the_president_does_not() {
		ExtBuilder::default().build().execute_with(|| {
			seat_president(SEROK);
			make_citizen(PM);
			make_citizen(MINISTER);
			install_prime_minister(RuntimeOrigin::signed(SEROK), PM);

			// The President names the head of government, not the cabinet.
			assert_noop!(
				Welati::appoint_minister(
					RuntimeOrigin::signed(SEROK),
					MINISTER,
					Tiki::WezireDarayiye
				),
				Error::<Test>::NotThePrimeMinister
			);

			assert_ok!(Welati::appoint_minister(
				RuntimeOrigin::signed(PM),
				MINISTER,
				Tiki::WezireDarayiye
			));
			assert_eq!(holder_of(Tiki::WezireDarayiye), Some(MINISTER));
		});
	}

	#[test]
	fn a_named_portfolio_has_one_holder_and_changing_it_is_one_step() {
		ExtBuilder::default().build().execute_with(|| {
			make_citizen(PM);
			make_citizen(MINISTER);
			make_citizen(OUTSIDER);
			install_prime_minister(RuntimeOrigin::root(), PM);
			assert_ok!(Welati::appoint_minister(
				RuntimeOrigin::signed(PM),
				MINISTER,
				Tiki::WezireDarayiye
			));

			// Appointing a second finance minister moves the office rather than duplicating
			// it: the outgoing holder loses it in the same call.
			assert_ok!(Welati::appoint_minister(
				RuntimeOrigin::signed(PM),
				OUTSIDER,
				Tiki::WezireDarayiye
			));

			assert_eq!(holder_of(Tiki::WezireDarayiye), Some(OUTSIDER));
			assert!(
				!pezpallet_tiki::UserTikis::<Test>::get(MINISTER).contains(&Tiki::WezireDarayiye)
			);
		});
	}

	#[test]
	fn the_generic_wezir_can_be_held_by_several_people_at_once() {
		// So the state can create a ministry for something nobody has thought of yet without
		// a runtime upgrade -- which is what it cannot do with the named portfolios.
		ExtBuilder::default().build().execute_with(|| {
			make_citizen(PM);
			make_citizen(MINISTER);
			make_citizen(OUTSIDER);
			install_prime_minister(RuntimeOrigin::root(), PM);

			assert_ok!(Welati::appoint_minister(RuntimeOrigin::signed(PM), MINISTER, Tiki::Wezir));
			assert_ok!(Welati::appoint_minister(RuntimeOrigin::signed(PM), OUTSIDER, Tiki::Wezir));

			assert!(pezpallet_tiki::UserTikis::<Test>::get(MINISTER).contains(&Tiki::Wezir));
			assert!(pezpallet_tiki::UserTikis::<Test>::get(OUTSIDER).contains(&Tiki::Wezir));
		});
	}

	#[test]
	fn only_cabinet_posts_can_be_appointed_this_way() {
		// The Prime Minister runs the government; he does not hand out judgeships or seats in
		// Parliament, and this call is not a way around the paths that do.
		ExtBuilder::default().build().execute_with(|| {
			make_citizen(PM);
			make_citizen(MINISTER);
			install_prime_minister(RuntimeOrigin::root(), PM);

			for forbidden in [Tiki::Serok, Tiki::Parlementer, Tiki::Dadger, Tiki::SerokWeziran] {
				assert_noop!(
					Welati::appoint_minister(RuntimeOrigin::signed(PM), MINISTER, forbidden),
					Error::<Test>::NotACabinetPost
				);
			}
		});
	}

	#[test]
	fn dismissing_the_prime_minister_leaves_the_cabinet_standing() {
		// Emptying the cabinet here would mean one call darkens every ministry at the moment
		// there is nobody left to refill them.
		ExtBuilder::default().build().execute_with(|| {
			make_citizen(PM);
			make_citizen(MINISTER);
			install_prime_minister(RuntimeOrigin::root(), PM);
			assert_ok!(Welati::appoint_minister(
				RuntimeOrigin::signed(PM),
				MINISTER,
				Tiki::WezireDarayiye
			));

			assert_ok!(Welati::dismiss_prime_minister(RuntimeOrigin::root()));

			assert_eq!(holder_of(Tiki::SerokWeziran), None);
			assert_eq!(holder_of(Tiki::WezireDarayiye), Some(MINISTER));
		});
	}

	#[test]
	fn a_minister_is_someone_holding_a_cabinet_tiki() {
		// `is_minister` used to read a map that nothing in the pallet ever wrote, so it was
		// false for everyone and every authority check built on it was silently off.
		ExtBuilder::default().build().execute_with(|| {
			make_citizen(PM);
			make_citizen(MINISTER);
			assert!(!Welati::is_minister(&MINISTER));

			install_prime_minister(RuntimeOrigin::root(), PM);
			assert_ok!(Welati::appoint_minister(
				RuntimeOrigin::signed(PM),
				MINISTER,
				Tiki::WezireDarayiye
			));

			assert!(Welati::is_minister(&MINISTER));
		});
	}
}

// ===== THE BUDGET =====
//
// Three bodies, three different records, and a payment needs all three to agree: the caller
// holds the finance portfolio (the tiki), the allowance covers it (Parliament's vote), and the
// pot has the money (checked on the treasury chain, on arrival). What a real state does is
// stop the executive writing its own cheques, and that is what this is.

mod budget {
	use super::*;
	use crate::{
		mock::{clear_sent_xcm, make_citizen, sent_xcm},
		ApprovedBudget,
	};
	use codec::Encode;
	use pezpallet_tiki::Tiki;

	const PM: u64 = 2;
	const FINANCE: u64 = 3;
	const OUTSIDER: u64 = 4;
	const BENEFICIARY: u64 = 5;

	/// Seat a finance minister, the way the appointment chain would.
	fn seat_finance_minister() {
		make_citizen(PM);
		make_citizen(FINANCE);
		install_prime_minister(RuntimeOrigin::root(), PM);
		assert_ok!(Welati::appoint_minister(
			RuntimeOrigin::signed(PM),
			FINANCE,
			Tiki::WezireDarayiye
		));
	}

	/// Put `amount` through Parliament and have it carried.
	fn parliament_approves(amount: u128) {
		let id = Welati::next_proposal_id();
		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			b"Budget".to_vec().try_into().unwrap(),
			b"A budget for the year".to_vec().try_into().unwrap(),
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			Some(amount),
		));
		let proposal = Welati::active_proposals(id).unwrap();
		run_to_block(proposal.voting_starts_at + 1);
		for member in 1..=(proposal.threshold as u64) {
			add_parliament_member(member);
			assert_ok!(Welati::vote_on_proposal(
				RuntimeOrigin::signed(member),
				id,
				VoteChoice::Aye,
				None
			));
		}
		assert_ok!(Welati::finalize_proposal(RuntimeOrigin::signed(99), id));
	}

	#[test]
	fn only_parliament_raises_the_allowance() {
		// There is no extrinsic that sets it. The only way the number moves is a proposal
		// that reached its threshold -- not the President, not a minister, not root.
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			assert_eq!(ApprovedBudget::<Test>::get(), 0);

			parliament_approves(1_000);
			assert_eq!(ApprovedBudget::<Test>::get(), 1_000);
		});
	}

	#[test]
	fn approvals_accumulate() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			parliament_approves(1_000);
			parliament_approves(500);
			assert_eq!(ApprovedBudget::<Test>::get(), 1_500);
		});
	}

	#[test]
	fn the_finance_minister_spends_within_the_allowance() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			seat_finance_minister();
			parliament_approves(1_000);
			clear_sent_xcm();

			assert_ok!(Welati::spend_budget(RuntimeOrigin::signed(FINANCE), BENEFICIARY, 400));

			assert_eq!(ApprovedBudget::<Test>::get(), 600);
			assert_eq!(sent_xcm().len(), 1, "the treasury chain was not asked to pay");
		});
	}

	#[test]
	fn nobody_but_the_finance_minister_spends() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			seat_finance_minister();
			parliament_approves(1_000);
			clear_sent_xcm();

			// Not an outsider, and not the Prime Minister who appointed him: the portfolio is
			// what authorises a payment, not the rank above it.
			for who in [OUTSIDER, PM] {
				assert_noop!(
					Welati::spend_budget(RuntimeOrigin::signed(who), BENEFICIARY, 100),
					Error::<Test>::NotTheFinanceMinister
				);
			}

			assert_eq!(ApprovedBudget::<Test>::get(), 1_000);
			assert!(sent_xcm().is_empty());
		});
	}

	#[test]
	fn the_minister_cannot_spend_past_the_allowance() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			seat_finance_minister();
			parliament_approves(1_000);
			clear_sent_xcm();

			assert_noop!(
				Welati::spend_budget(RuntimeOrigin::signed(FINANCE), BENEFICIARY, 1_001),
				Error::<Test>::BudgetExceeded
			);

			assert_eq!(ApprovedBudget::<Test>::get(), 1_000);
			assert!(sent_xcm().is_empty(), "a refused payment was still sent");
		});
	}

	#[test]
	fn the_allowance_cannot_be_spent_twice() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			seat_finance_minister();
			parliament_approves(1_000);
			clear_sent_xcm();

			assert_ok!(Welati::spend_budget(RuntimeOrigin::signed(FINANCE), BENEFICIARY, 1_000));
			assert_eq!(ApprovedBudget::<Test>::get(), 0);

			assert_noop!(
				Welati::spend_budget(RuntimeOrigin::signed(FINANCE), BENEFICIARY, 1),
				Error::<Test>::BudgetExceeded
			);
			assert_eq!(sent_xcm().len(), 1);
		});
	}

	#[test]
	fn a_dismissed_minister_stops_being_able_to_pay() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			seat_finance_minister();
			parliament_approves(1_000);

			assert_ok!(Welati::dismiss_minister(
				RuntimeOrigin::signed(PM),
				FINANCE,
				Tiki::WezireDarayiye
			));

			assert_noop!(
				Welati::spend_budget(RuntimeOrigin::signed(FINANCE), BENEFICIARY, 100),
				Error::<Test>::NotTheFinanceMinister
			);
		});
	}

	#[test]
	fn the_payment_names_the_treasury_pallet_its_spend_call_and_the_beneficiary() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			seat_finance_minister();
			parliament_approves(1_000);
			clear_sent_xcm();

			assert_ok!(Welati::spend_budget(RuntimeOrigin::signed(FINANCE), BENEFICIARY, 400));

			let (destination, message) = sent_xcm().pop().expect("nothing was sent");
			assert_eq!(destination, crate::mock::TreasuryChain::get());

			let transact = message
				.into_iter()
				.find_map(|i| match i {
					xcm::latest::Instruction::Transact { call, .. } => Some(call),
					_ => None,
				})
				.expect("no Transact in the message");

			// Pallet 70, call 1: `pez_treasury::spend_from_government_pot`, then the
			// beneficiary and the amount. The amount is plain, not compact -- that pallet's
			// call takes a bare `Balance`. This expectation used to be copied from the code
			// it was checking rather than from the call it has to decode as, so it agreed
			// with a message the treasury chain could not read. The Asset Hub runtimes now
			// pin the same bytes from the other side.
			let expected = (70u8, 1u8, BENEFICIARY, 400u128).encode();
			assert_eq!(transact.into_encoded(), expected);
		});
	}

	#[test]
	fn a_payment_of_nothing_is_refused() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			seat_finance_minister();
			parliament_approves(1_000);
			assert_noop!(
				Welati::spend_budget(RuntimeOrigin::signed(FINANCE), BENEFICIARY, 0),
				Error::<Test>::NothingToSpend
			);
		});
	}
}

// ===== FINALISING A PROPOSAL =====
//
// Before this existed, `vote_on_proposal` recorded ayes and nays and no code path anywhere
// read them: Parliament could vote and nothing could follow. The threshold was computed when a
// proposal was made, written onto it, and never compared against anything.

mod proposals {
	use super::*;
	use crate::ApprovedBudget;

	/// Submit a proposal, optionally asking for a budget, and open voting.
	fn open_proposal(budget: Option<u128>) -> u32 {
		let id = Welati::next_proposal_id();
		assert_ok!(Welati::submit_proposal(
			RuntimeOrigin::signed(1),
			b"Budget".to_vec().try_into().unwrap(),
			b"A budget for the year".to_vec().try_into().unwrap(),
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			budget,
		));
		let proposal = Welati::active_proposals(id).unwrap();
		run_to_block(proposal.voting_starts_at + 1);
		id
	}

	/// Enough members to carry a simple majority of the mock's Parliament, then their ayes.
	fn carry(id: u32) {
		let threshold = Welati::active_proposals(id).unwrap().threshold;
		for member in 1..=(threshold as u64) {
			add_parliament_member(member);
			assert_ok!(Welati::vote_on_proposal(
				RuntimeOrigin::signed(member),
				id,
				VoteChoice::Aye,
				None
			));
		}
	}

	#[test]
	fn a_proposal_that_reaches_its_threshold_passes() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			let id = open_proposal(None);
			carry(id);

			assert_ok!(Welati::finalize_proposal(RuntimeOrigin::signed(99), id));

			assert_eq!(Welati::active_proposals(id).unwrap().status, ProposalStatus::Approved);
		});
	}

	#[test]
	fn a_proposal_still_open_and_short_of_its_threshold_cannot_be_finalised() {
		// Otherwise anyone could close a vote the moment it was going their way.
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			let id = open_proposal(None);

			assert_ok!(Welati::vote_on_proposal(
				RuntimeOrigin::signed(1),
				id,
				VoteChoice::Aye,
				None
			));

			assert_noop!(
				Welati::finalize_proposal(RuntimeOrigin::signed(99), id),
				Error::<Test>::ProposalStillOpen
			);
		});
	}

	#[test]
	fn a_proposal_that_runs_out_of_time_without_the_votes_is_rejected() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			let id = open_proposal(None);
			assert_ok!(Welati::vote_on_proposal(
				RuntimeOrigin::signed(1),
				id,
				VoteChoice::Aye,
				None
			));

			let expires = Welati::active_proposals(id).unwrap().expires_at;
			run_to_block(expires + 1);

			assert_ok!(Welati::finalize_proposal(RuntimeOrigin::signed(99), id));
			assert_eq!(Welati::active_proposals(id).unwrap().status, ProposalStatus::Rejected);
		});
	}

	#[test]
	fn a_proposal_is_only_finalised_once() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			let id = open_proposal(None);
			carry(id);
			assert_ok!(Welati::finalize_proposal(RuntimeOrigin::signed(99), id));

			assert_noop!(
				Welati::finalize_proposal(RuntimeOrigin::signed(99), id),
				Error::<Test>::ProposalNotActive
			);
		});
	}

	#[test]
	fn what_a_proposal_asks_for_survives_the_trip_to_storage() {
		// The field this replaced was `#[codec(skip)]`, so it was dropped on the way in and
		// read back empty every time -- which is why the proposal system could never enact
		// anything, whatever the vote said.
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			let id = open_proposal(Some(5_000));
			assert_eq!(Welati::active_proposals(id).unwrap().budget_amount, Some(5_000));
		});
	}

	#[test]
	fn a_passed_budget_proposal_raises_the_allowance() {
		// The whole point of the chain: Parliament votes, and the vote is what moves the
		// number the finance minister spends against. Nobody holds a key that does this.
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			let id = open_proposal(Some(5_000));
			carry(id);

			assert_eq!(ApprovedBudget::<Test>::get(), 0);
			assert_ok!(Welati::finalize_proposal(RuntimeOrigin::signed(99), id));
			assert_eq!(ApprovedBudget::<Test>::get(), 5_000);
		});
	}

	#[test]
	fn a_rejected_budget_proposal_raises_nothing() {
		ExtBuilder::default().build().execute_with(|| {
			add_parliament_member(1);
			let id = open_proposal(Some(5_000));
			assert_ok!(Welati::vote_on_proposal(
				RuntimeOrigin::signed(1),
				id,
				VoteChoice::Nay,
				None
			));

			let expires = Welati::active_proposals(id).unwrap().expires_at;
			run_to_block(expires + 1);
			assert_ok!(Welati::finalize_proposal(RuntimeOrigin::signed(99), id));

			assert_eq!(Welati::active_proposals(id).unwrap().status, ProposalStatus::Rejected);
			assert_eq!(ApprovedBudget::<Test>::get(), 0);
		});
	}
}

// ===== THE ELECTORAL CLOCK =====
//
// Before this, `initiate_election` required root and nothing called it, and the President had
// no term at all. An office could be won once and held for ever: the term was recorded for
// Parliament, nothing read it, and the only way a second election ever happened was somebody
// with sudo remembering to ask for one. A state whose elections depend on an outside key is
// not governing itself.

mod electoral_clock {
	use super::*;
	use crate::{
		mock::{holder_of, System},
		FailedAttempts, OpenElection, TermEnds,
	};
	use pezpallet_tiki::Tiki;

	/// Blocks from opening an election to having a result.
	fn cycle() -> u64 {
		crate::mock::CandidacyPeriod::get()
			+ crate::mock::CampaignPeriod::get()
			+ crate::mock::ElectionPeriod::get()
	}

	/// Endorse `candidate` with as many citizens as the office requires, each signing for
	/// themselves, and hand back the list for the candidacy.
	fn gather_endorsements(id: u32, candidate: u64, count: u32) -> Vec<u64> {
		let mut endorsers = Vec::new();
		// Skip the candidate: nobody nominates themselves into a threshold.
		for who in (1..=110u64).filter(|w| *w != candidate).take(count as usize) {
			assert_ok!(Welati::endorse_candidate(RuntimeOrigin::signed(who), id, candidate));
			endorsers.push(who);
		}
		endorsers
	}

	/// Run a presidential election through to a seated winner.
	fn elect_president(winner: u64) -> u32 {
		let id = Welati::next_election_id();
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Presidential,
			None,
			None,
		));
		let endorsers =
			gather_endorsements(id, winner, crate::mock::PresidentialEndorsements::get());
		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(winner), id, None, endorsers));
		let election = Welati::active_elections(id).unwrap();
		run_to_block(election.voting_start + 1);
		// A presidential election needs half the country to turn out, so the helper has to
		// produce a real turnout rather than one vote.
		for voter in 1..=60u64 {
			assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(voter), id, vec![winner], None));
		}
		run_to_block(election.end_block + 1);
		assert_ok!(Welati::finalize_election(RuntimeOrigin::signed(99), id));
		id
	}

	#[test]
	fn winning_starts_a_mandate_that_runs_out() {
		ExtBuilder::default().build().execute_with(|| {
			elect_president(2);

			let ends_at = TermEnds::<Test>::get(ElectionType::Presidential)
				.expect("the winner took office under no mandate");
			assert_eq!(ends_at, System::block_number() + crate::mock::TermLength::get());
			assert_eq!(holder_of(Tiki::Serok), Some(2));
		});
	}

	#[test]
	fn the_next_election_opens_by_itself() {
		// The whole point. Nobody calls anything; the clock does it.
		ExtBuilder::default().build().execute_with(|| {
			elect_president(2);
			let ends_at = TermEnds::<Test>::get(ElectionType::Presidential).unwrap();
			OpenElection::<Test>::remove(ElectionType::Presidential);

			// Well before the run-up, nothing happens.
			run_to_block(ends_at - cycle() - 10);
			assert!(!OpenElection::<Test>::contains_key(ElectionType::Presidential));

			// One cycle out, the election opens on its own.
			run_to_block(ends_at - cycle() + 1);
			assert!(
				OpenElection::<Test>::contains_key(ElectionType::Presidential),
				"the mandate ran down and no election was called"
			);
		});
	}

	#[test]
	fn only_one_election_per_office_is_open_at_a_time() {
		ExtBuilder::default().build().execute_with(|| {
			elect_president(2);
			let ends_at = TermEnds::<Test>::get(ElectionType::Presidential).unwrap();
			OpenElection::<Test>::remove(ElectionType::Presidential);

			run_to_block(ends_at - cycle() + 1);
			let opened = OpenElection::<Test>::get(ElectionType::Presidential).unwrap();

			// The run-up lasts many blocks; the scheduler must not open one per block.
			run_to_block(ends_at - cycle() + 50);
			assert_eq!(OpenElection::<Test>::get(ElectionType::Presidential), Some(opened));
		});
	}

	#[test]
	fn the_calendar_does_not_drift() {
		// A mandate measured from the moment of counting would push the next one back by
		// however late the count was, and every cycle would slide a little further.
		ExtBuilder::default().build().execute_with(|| {
			elect_president(2);
			let first_end = TermEnds::<Test>::get(ElectionType::Presidential).unwrap();

			// The second election is counted well after the first term expired.
			run_to_block(first_end + 500);
			OpenElection::<Test>::remove(ElectionType::Presidential);
			elect_president(3);

			// The count came in after the first term had already expired, so the calendar
			// steps whole terms until it lands in the future -- it does not restart from
			// whenever the arithmetic happened to arrive.
			let second_end = TermEnds::<Test>::get(ElectionType::Presidential).unwrap();
			let length = crate::mock::TermLength::get();
			assert_eq!(
				(second_end - first_end) % length,
				0,
				"the term was measured from the count instead of from the previous term"
			);
			assert!(second_end > System::block_number());
		});
	}

	#[test]
	fn an_officeholder_keeps_going_for_one_cycle_and_no_longer() {
		// Emptying the office the instant the clock runs out would leave the state without a
		// President while the replacement is still being counted. Never emptying it would let
		// a broken election keep somebody in place for ever.
		ExtBuilder::default().build().execute_with(|| {
			elect_president(2);
			let ends_at = TermEnds::<Test>::get(ElectionType::Presidential).unwrap();

			run_to_block(ends_at + 1);
			assert_eq!(
				pezpallet_tiki::Pezpallet::<Test>::current_holder(&Tiki::Serok),
				Some(2),
				"the office emptied mid-handover"
			);

			System::set_block_number(ends_at + cycle() + 1);
			assert_eq!(
				pezpallet_tiki::Pezpallet::<Test>::current_holder(&Tiki::Serok),
				None,
				"the mandate never actually ended"
			);
			// The raw map still names them, which is why nothing reads it directly.
			assert_eq!(holder_of(Tiki::Serok), Some(2));
		});
	}

	#[test]
	fn a_vacancy_calls_an_election_without_waiting_for_the_clock() {
		ExtBuilder::default().build().execute_with(|| {
			elect_president(2);
			OpenElection::<Test>::remove(ElectionType::Presidential);

			// The President stops being President -- resignation, impeachment, loss of
			// citizenship; the reason does not change the answer.
			assert_ok!(pezpallet_tiki::Pezpallet::<Test>::internal_revoke_role(&2, Tiki::Serok));
			assert_eq!(holder_of(Tiki::Serok), None);

			run_to_block(System::block_number() + 1);
			assert!(
				OpenElection::<Test>::contains_key(ElectionType::Presidential),
				"the presidency fell empty and nothing called an election"
			);
		});
	}

	#[test]
	fn the_speaker_acts_while_the_presidency_is_empty() {
		ExtBuilder::default().build().execute_with(|| {
			elect_president(2);
			assert_eq!(Welati::acting_president(), Some(2));

			assert_ok!(Welati::seat_unique_tiki(&3, Tiki::SerokiMeclise));
			assert_ok!(pezpallet_tiki::Pezpallet::<Test>::internal_revoke_role(&2, Tiki::Serok));

			assert_eq!(Welati::acting_president(), Some(3));
			// Acting is not holding: the office itself is still empty, which is what keeps
			// the scheduler calling the by-election.
			assert_eq!(holder_of(Tiki::Serok), None);
		});
	}

	#[test]
	fn nobody_serves_more_consecutive_terms_than_the_office_allows() {
		ExtBuilder::default().build().execute_with(|| {
			elect_president(2);
			let first_end = TermEnds::<Test>::get(ElectionType::Presidential).unwrap();
			run_to_block(first_end + 1);
			OpenElection::<Test>::remove(ElectionType::Presidential);
			elect_president(2);

			// Two terms is the limit in the mock, so a third candidacy is refused.
			let id = Welati::next_election_id();
			assert_ok!(Welati::initiate_election(
				RuntimeOrigin::root(),
				ElectionType::Presidential,
				None,
				None,
			));
			let for_two = gather_endorsements(id, 2, crate::mock::PresidentialEndorsements::get());
			assert_noop!(
				Welati::register_candidate(RuntimeOrigin::signed(2), id, None, for_two),
				Error::<Test>::TermLimitReached
			);

			// Somebody else may stand. The endorsers who already spoke for account 2 cannot
			// speak again, so this needs fresh ones -- which is the rule working.
			// Accounts 1..=101 have already spoken for account 2 in this election and cannot
			// speak again, so a second candidate has to find support elsewhere -- which is
			// the rule doing its work.
			assert_noop!(
				Welati::endorse_candidate(RuntimeOrigin::signed(50), id, 3),
				Error::<Test>::AlreadyEndorsed
			);
			assert_ok!(Welati::endorse_candidate(RuntimeOrigin::signed(105), id, 3));
			assert_noop!(
				Welati::register_candidate(RuntimeOrigin::signed(3), id, None, vec![105]),
				Error::<Test>::InsufficientEndorsements
			);
		});
	}

	#[test]
	fn a_failed_turnout_ends_the_election_instead_of_freezing_it() {
		// This used to return an error and leave the record untouched, so the election stayed
		// `Active` for ever: it could never be counted again, and nothing would open a
		// replacement. The office simply stopped existing.
		ExtBuilder::default().build().execute_with(|| {
			crate::mock::set_citizen_count(1_000_000); // nobody's votes will reach the quorum
			let id = Welati::next_election_id();
			assert_ok!(Welati::initiate_election(
				RuntimeOrigin::root(),
				ElectionType::Presidential,
				None,
				None,
			));
			let endorsers =
				gather_endorsements(id, 2, crate::mock::PresidentialEndorsements::get());
			assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(2), id, None, endorsers));
			let election = Welati::active_elections(id).unwrap();
			run_to_block(election.voting_start + 1);
			assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(1), id, vec![2], None));
			run_to_block(election.end_block + 1);

			assert_ok!(Welati::finalize_election(RuntimeOrigin::signed(99), id));

			assert_eq!(
				Welati::active_elections(id).unwrap().status,
				ElectionStatus::FailedForTurnout
			);
			assert!(!OpenElection::<Test>::contains_key(ElectionType::Presidential));
			assert_eq!(FailedAttempts::<Test>::get(ElectionType::Presidential), 1);
		});
	}

	#[test]
	fn the_re_run_does_not_ask_again_for_a_quorum_the_country_missed() {
		// Applying the same threshold unchanged produces the same failure for ever, and an
		// office that can never be filled is worse than one filled by a small turnout.
		ExtBuilder::default().build().execute_with(|| {
			crate::mock::set_citizen_count(1_000_000);
			FailedAttempts::<Test>::insert(ElectionType::Presidential, 1);

			let id = Welati::next_election_id();
			assert_ok!(Welati::initiate_election(
				RuntimeOrigin::root(),
				ElectionType::Presidential,
				None,
				None,
			));
			let endorsers =
				gather_endorsements(id, 2, crate::mock::PresidentialEndorsements::get());
			assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(2), id, None, endorsers));
			let election = Welati::active_elections(id).unwrap();
			run_to_block(election.voting_start + 1);
			assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(1), id, vec![2], None));
			run_to_block(election.end_block + 1);

			assert_ok!(Welati::finalize_election(RuntimeOrigin::signed(99), id));

			assert_eq!(Welati::active_elections(id).unwrap().status, ElectionStatus::Completed);
			assert_eq!(holder_of(Tiki::Serok), Some(2));
			assert_eq!(FailedAttempts::<Test>::get(ElectionType::Presidential), 0);
		});
	}

	#[test]
	fn counting_the_votes_needs_no_special_authority() {
		ExtBuilder::default().build().execute_with(|| {
			let id = Welati::next_election_id();
			assert_ok!(Welati::initiate_election(
				RuntimeOrigin::root(),
				ElectionType::Presidential,
				None,
				None,
			));
			let endorsers =
				gather_endorsements(id, 2, crate::mock::PresidentialEndorsements::get());
			assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(2), id, None, endorsers));
			let election = Welati::active_elections(id).unwrap();
			run_to_block(election.voting_start + 1);
			assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(1), id, vec![2], None));
			run_to_block(election.end_block + 1);

			// Any citizen may ask for the sum to be taken. It decides nothing: the votes are
			// cast and the rules were fixed when the election opened.
			assert_ok!(Welati::finalize_election(RuntimeOrigin::signed(50), id));
		});
	}
}

// ===== THE INVARIANT CAN FAIL =====
//
// `try_state` runs after every block of every test above. That only means something if it can
// reject a bad state; a check that always passes reads as coverage and is worse than none.

#[cfg(feature = "try-runtime")]
mod invariant {
	use super::*;
	use crate::{mock::System, ConsecutiveTerms, CurrentOfficials, OpenElection, TermEnds};
	use pezframe_support::traits::Hooks;
	use pezpallet_tiki::Tiki;

	fn check() -> Result<(), pezsp_runtime::TryRuntimeError> {
		<Welati as Hooks<u64>>::try_state(System::block_number())
	}

	fn assert_rejected(what: &str) {
		assert!(check().is_err(), "try_state accepted a state where {what}");
	}

	/// A President in office, recorded in both places.
	fn seated_president() {
		assert_ok!(Welati::seat_unique_tiki(&2, Tiki::Serok));
		pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Serok, 2);
		TermEnds::<Test>::insert(ElectionType::Presidential, 10_000);
	}

	#[test]
	fn an_ordinary_state_passes() {
		ExtBuilder::default().build().execute_with(|| {
			seated_president();
			assert_ok!(check());
		});
	}

	#[test]
	fn an_office_voting_on_an_election_that_does_not_exist_is_caught() {
		// A stale entry here stops the scheduler from ever opening another election for that
		// office -- it would quietly stop being contested, with nothing to say why.
		ExtBuilder::default().build().execute_with(|| {
			seated_president();
			OpenElection::<Test>::insert(ElectionType::Presidential, 999);
			assert_rejected("an office was voting on an election that does not exist");
		});
	}

	#[test]
	fn an_office_voting_on_another_offices_election_is_caught() {
		ExtBuilder::default().build().execute_with(|| {
			seated_president();
			assert_ok!(Welati::initiate_election(
				RuntimeOrigin::root(),
				ElectionType::Parliamentary,
				None,
				None,
			));
			let id = OpenElection::<Test>::get(ElectionType::Parliamentary).unwrap();
			OpenElection::<Test>::insert(ElectionType::Presidential, id);
			assert_rejected("one election was open for two different offices");
		});
	}

	#[test]
	fn a_holder_the_government_register_does_not_name_is_caught() {
		// The shape of a real failure: "who is President" answered differently depending on
		// which record the asking pallet happens to read.
		ExtBuilder::default().build().execute_with(|| {
			seated_president();
			pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Serok, 3);
			assert_rejected("the office and the register named different people");
		});
	}

	#[test]
	fn an_elected_office_held_under_no_mandate_is_caught() {
		// An officeholder no clock will ever remove: nothing schedules an election, because
		// there is no term running down to schedule it against.
		ExtBuilder::default().build().execute_with(|| {
			seated_president();
			TermEnds::<Test>::remove(ElectionType::Presidential);
			assert_rejected("somebody held an elected office with no mandate recorded");
		});
	}

	#[test]
	fn serving_past_the_term_limit_is_caught() {
		ExtBuilder::default().build().execute_with(|| {
			seated_president();
			ConsecutiveTerms::<Test>::insert(
				ElectionType::Presidential,
				2,
				crate::mock::MaxConsecutiveTerms::get() + 1,
			);
			assert_rejected("somebody had served past the term limit");
		});
	}
}

// ===== PARLIAMENTARY SEATS =====
//
// The seat is the `Parlementer` tiki and nothing else. `ParliamentMembers` records who won it
// so that the reward pallet has two hundred and one accounts to look at instead of the whole
// population; the tiki is what says whether they still hold it. Everything below is about
// keeping those two from drifting, and about the handover not landing in one block.

/// Who currently holds a parliamentary seat, by the only measure that counts.
fn seat_holders() -> Vec<u64> {
	let mut held: Vec<u64> = pezpallet_tiki::UserTikis::<Test>::iter()
		.filter(|(_, tikis)| tikis.contains(&pezpallet_tiki::Tiki::Parlementer))
		.map(|(who, _)| who)
		.collect();
	held.sort();
	held
}

fn listed_members() -> Vec<u64> {
	let mut listed: Vec<u64> =
		Welati::parliament_members().iter().map(|member| member.account).collect();
	listed.sort();
	listed
}

/// One election cycle, as the mock configures it.
const CYCLE: u64 = 60;

#[test]
fn the_founding_parliament_is_seated_and_starts_the_clock() {
	// The house that passes the first budget cannot itself have been elected -- there is
	// nobody to elect it yet. What matters is that seating it starts the institutional clock,
	// because that clock is the only thing that opens the first real election.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		let founding: Vec<u64> = (2..=6).collect();

		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), founding.clone()));

		assert_eq!(listed_members(), founding);
		assert!(
			crate::TermEnds::<Test>::contains_key(ElectionType::Parliamentary),
			"seating the founding house has to start the clock, or the first election never opens"
		);

		// The seats arrive over the following blocks, not in the call.
		run_to_block(System::block_number() + 2);
		assert_eq!(seat_holders(), founding);
	});
}

#[test]
fn the_founding_parliament_is_temporary_by_construction() {
	// It is not temporary because anyone promised to replace it: the seats carry the same
	// term an elected house would, so they run out on their own.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), vec![2, 3]));
		run_to_block(System::block_number() + 2);

		let term_end = crate::TermEnds::<Test>::get(ElectionType::Parliamentary).unwrap();
		assert_eq!(
			pezpallet_tiki::TikiExpiry::<Test>::get(2, pezpallet_tiki::Tiki::Parlementer),
			Some(term_end + CYCLE),
			"a seat outlives its term by one election cycle, so the house is never empty \
			 while the next one is being counted"
		);

		assert!(pezpallet_tiki::Pezpallet::<Test>::has_tiki(
			&2,
			&pezpallet_tiki::Tiki::Parlementer
		));
		run_to_block(term_end + CYCLE + 1);
		assert!(
			!pezpallet_tiki::Pezpallet::<Test>::has_tiki(&2, &pezpallet_tiki::Tiki::Parlementer),
			"the seat has to stop counting when its term is over, without anyone removing it"
		);
	});
}

#[test]
fn only_the_president_or_root_can_seat_the_founding_parliament() {
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		assert_noop!(
			Welati::seat_founding_parliament(RuntimeOrigin::signed(7), vec![2, 3]),
			Error::<Test>::NotAuthorizedToNominate
		);
		assert!(listed_members().is_empty());

		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::root(), vec![2, 3]));
	});
}

#[test]
fn the_founding_parliament_cannot_be_seated_over_a_sitting_house() {
	// The one power here is to constitute a house where there is none. If it could be used
	// again it would be a power to dissolve an elected Parliament and appoint another.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), vec![2, 3]));

		assert_noop!(
			Welati::seat_founding_parliament(RuntimeOrigin::signed(1), vec![4, 5]),
			Error::<Test>::ParliamentAlreadySeated
		);
	});
}

#[test]
fn seats_change_hands_a_few_at_a_time() {
	// Each seat rewrites a citizen NFT. Two hundred and one of those in the block that counts
	// the votes would be a block nothing else fits into, so the queue is drained over several.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		let founding: Vec<u64> = (2..=12).collect(); // eleven, one more than a block takes
		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), founding.clone()));

		assert!(seat_holders().is_empty(), "no seat is given in the call itself");

		run_to_block(System::block_number() + 1);
		assert_eq!(seat_holders().len(), 10, "ten seats a block, no more");

		run_to_block(System::block_number() + 1);
		assert_eq!(seat_holders(), founding, "and the rest on the next block");
		assert!(
			crate::PendingSeatTerm::<Test>::get().is_none(),
			"a finished handover leaves no term behind"
		);
	});
}

#[test]
fn a_member_who_keeps_their_seat_never_loses_it() {
	// Revoking everyone and re-granting would be simpler to write and wrong to run: the trust
	// score behind the seat would dip, and anything reading authority mid-handover would see
	// a sitting member as a private citizen.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), vec![2, 3, 4]));
		run_to_block(System::block_number() + 2);
		assert_eq!(seat_holders(), vec![2, 3, 4]);

		// A new house: 3 stays, 2 and 4 go, 5 arrives.
		let incoming = vec![3u64, 5];
		let outgoing: Vec<u64> = vec![2, 3, 4];
		let term_end = crate::TermEnds::<Test>::get(ElectionType::Parliamentary).unwrap() + 500;
		assert_ok!(Welati::queue_seat_handover(&outgoing, &incoming, term_end));

		assert_eq!(
			Welati::pending_seat_revokes().to_vec(),
			vec![2, 4],
			"only the members who actually left"
		);
		assert_eq!(Welati::pending_seat_grants().to_vec(), vec![5], "only the members who arrived");
		assert!(
			pezpallet_tiki::Pezpallet::<Test>::has_tiki(&3, &pezpallet_tiki::Tiki::Parlementer),
			"the returning member is never unseated, not even for a block"
		);
		assert_eq!(
			pezpallet_tiki::TikiExpiry::<Test>::get(3, pezpallet_tiki::Tiki::Parlementer),
			Some(term_end + CYCLE),
			"but their seat runs to the new term, not the old one"
		);
	});
}

#[test]
fn a_winner_who_is_no_longer_a_citizen_does_not_stall_the_queue() {
	// A seat that cannot be given is a seat that stays empty -- and its share of the
	// parliamentary reward stays in the pot. What it must not do is stop the other two
	// hundred seats from changing hands.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		// 77 was never made a citizen, so `internal_grant_role` will refuse it.
		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), vec![2, 77, 3]));
		run_to_block(System::block_number() + 2);

		assert_eq!(seat_holders(), vec![2, 3], "the other seats were still given");
		assert!(Welati::pending_seat_grants().is_empty(), "the queue drained");
		System::assert_has_event(RuntimeEvent::Welati(WelatiEvent::SeatCouldNotBeTaken {
			who: 77,
		}));
	});
}

#[test]
fn the_clock_and_not_the_roll_opens_the_next_election() {
	// Parliament is replaced by its term running out. There is no "the house is empty" arm
	// in the scheduler for it -- see `office_is_vacant` -- so this pins what does open the
	// election, and that seating the founding house is what starts that clock at all.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), vec![2, 3]));
		run_to_block(System::block_number() + 2);
		assert!(Welati::scheduled_election(ElectionType::Parliamentary).is_none());

		let term_end = crate::TermEnds::<Test>::get(ElectionType::Parliamentary).unwrap();
		// The election opens one cycle before the term ends, so the count is finished in time.
		run_to_block(term_end - CYCLE);
		assert!(
			Welati::scheduled_election(ElectionType::Parliamentary).is_some(),
			"the founding house has to hand over to an elected one on schedule"
		);
	});
}

#[cfg(feature = "try-runtime")]
#[test]
#[should_panic(expected = "somebody holds a parliamentary seat that no election gave them")]
fn a_seat_nobody_was_given_is_caught() {
	// The invariant that makes the two records safe to keep apart. If a seat could appear
	// without an election behind it, the reward pallet would pay it and the house would have
	// a member nobody chose.
	ExtBuilder::default().build().execute_with(|| {
		make_citizen(9);
		assert_ok!(pezpallet_tiki::Pezpallet::<Test>::internal_grant_role(
			&9,
			pezpallet_tiki::Tiki::Parlementer
		));
		crate::mock::check_invariants();
	});
}

// ===== THE COURT =====
//
// Eleven seats. Six the sitting house elects, five the President appoints. The two halves
// carry different conditions on purpose: the elected six answer to a vote and need no
// qualification beyond citizenship; the appointed five answer to nobody, so they have to be
// able to read what they rule on.

/// Give `who` a tiki that qualifies them for an appointed seat.
fn make_qualified(who: u64, tiki: pezpallet_tiki::Tiki) {
	make_citizen(who);
	assert_ok!(pezpallet_tiki::Pezpallet::<Test>::internal_grant_role(&who, tiki));
}

fn bench() -> Vec<u64> {
	Welati::diwan_members().iter().map(|m| m.account).collect()
}

#[test]
fn the_president_may_only_appoint_somebody_qualified() {
	// The whole reason the appointed half exists. A court that rules on whether an upgrade is
	// constitutional needs somebody who can read the upgrade; one that rules on a slash needs
	// somebody who can read the ledger. Citizenship alone is the elected half's bar, not this
	// one.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		make_citizen(7); // a citizen and nothing more

		assert_noop!(
			Welati::appoint_diwan_member(RuntimeOrigin::signed(1), 7),
			Error::<Test>::NotQualifiedForTheCourt
		);

		make_qualified(8, pezpallet_tiki::Tiki::Bernamenivîs);
		assert_ok!(Welati::appoint_diwan_member(RuntimeOrigin::signed(1), 8));
		assert_eq!(bench(), vec![8]);
		assert!(pezpallet_tiki::Pezpallet::<Test>::has_tiki(
			&8,
			&pezpallet_tiki::Tiki::EndameDiwane
		));
	});
}

#[test]
fn the_qualifying_pool_is_wider_than_law() {
	// A bench of lawyers could not read a runtime. Each of these five competences is on the
	// court because the caseload needs it.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		for (who, tiki) in [
			(10, pezpallet_tiki::Tiki::Hiquqnas),             // law
			(11, pezpallet_tiki::Tiki::PisporêEwlehiyaSîber), // the chain
			(12, pezpallet_tiki::Tiki::Aborînas),             // the economy
			(13, pezpallet_tiki::Tiki::Hilbijartinkar),       // elections
			(14, pezpallet_tiki::Tiki::Rewsenbîr),            // society
		] {
			make_qualified(who, tiki);
			assert_ok!(Welati::appoint_diwan_member(RuntimeOrigin::signed(1), who));
		}
		assert_eq!(bench().len(), 5);
	});
}

#[test]
fn the_president_cannot_take_more_than_five_seats() {
	// Six of the eleven are the house's. If the President could fill them the court would be
	// an office of the presidency.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		for who in 10..=14u64 {
			make_qualified(who, pezpallet_tiki::Tiki::Hiquqnas);
			assert_ok!(Welati::appoint_diwan_member(RuntimeOrigin::signed(1), who));
		}

		make_qualified(15, pezpallet_tiki::Tiki::Hiquqnas);
		assert_noop!(
			Welati::appoint_diwan_member(RuntimeOrigin::signed(1), 15),
			Error::<Test>::AppointedCourtSeatsAreFull
		);
	});
}

#[test]
fn nobody_takes_two_seats() {
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		make_qualified(10, pezpallet_tiki::Tiki::Dadger);
		assert_ok!(Welati::appoint_diwan_member(RuntimeOrigin::signed(1), 10));
		assert_noop!(
			Welati::appoint_diwan_member(RuntimeOrigin::signed(1), 10),
			Error::<Test>::AlreadyOnTheCourt
		);
	});
}

#[test]
fn only_the_president_or_root_appoints_to_the_court() {
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		make_qualified(10, pezpallet_tiki::Tiki::Dadger);

		assert_noop!(
			Welati::appoint_diwan_member(RuntimeOrigin::signed(7), 10),
			Error::<Test>::NotAuthorizedToNominate
		);
		assert_ok!(Welati::appoint_diwan_member(RuntimeOrigin::root(), 10));
	});
}

#[test]
fn only_the_house_votes_for_the_courts_elected_seats() {
	// The one election with a restricted electorate. Those six seats belong to Parliament, so
	// Parliament casts them -- a citizen who is not a member has no vote here.
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		assert_ok!(Welati::seat_founding_parliament(RuntimeOrigin::signed(1), vec![2, 3]));
		run_to_block(System::block_number() + 2);

		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::ConstitutionalCourt,
			None,
			None,
		));
		let endorsers = endorsed_by(0, 4, (200..=249).collect());
		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(4), 0, None, endorsers));
		let voting_start = Welati::active_elections(0).unwrap().voting_start;
		run_to_block(voting_start);

		// 300 is a citizen but not a member of the house.
		make_citizen(300);
		assert_noop!(
			Welati::cast_vote(RuntimeOrigin::signed(300), 0, vec![4], None),
			Error::<Test>::NotAParliamentMember
		);

		// 2 holds the Parlementer tiki, so 2 may.
		assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(2), 0, vec![4], None));
	});
}

#[cfg(feature = "try-runtime")]
#[test]
#[should_panic(expected = "the President has taken more of the court than they appoint")]
fn a_president_who_overfills_the_court_is_caught() {
	ExtBuilder::default().build().execute_with(|| {
		seat_president(1);
		let mut members = Welati::diwan_members();
		for who in 10..=16u64 {
			make_qualified(who, pezpallet_tiki::Tiki::Dadger);
			let _ = members.try_push(crate::types::DiwanMember {
				account: who,
				appointed_at: 1,
				term_ends_at: 999_999,
				appointed_by: crate::types::AppointmentAuthority::President(1),
			});
		}
		crate::DiwanMembers::<Test>::put(members);
		crate::mock::check_invariants();
	});
}

#[cfg(feature = "try-runtime")]
#[test]
#[should_panic(expected = "an appointed member of the court holds none of the qualifying tikis")]
fn an_unqualified_appointee_is_caught() {
	ExtBuilder::default().build().execute_with(|| {
		make_citizen(9);
		let mut members = Welati::diwan_members();
		let _ = members.try_push(crate::types::DiwanMember {
			account: 9,
			appointed_at: 1,
			term_ends_at: 999_999,
			appointed_by: crate::types::AppointmentAuthority::President(1),
		});
		crate::DiwanMembers::<Test>::put(members);
		crate::mock::check_invariants();
	});
}

mod citizen_tally {
	use super::*;
	use crate::types::CitizenTally;
	use pezframe_support::traits::VoteTally;
	use pezsp_runtime::Perbill;

	pezframe_support::parameter_types! {
		pub storage Roll: u32 = 0;
	}
	type Tally = CitizenTally<Roll>;

	fn tally(ayes: u32, nays: u32) -> Tally {
		let mut t = <Tally as VoteTally<u32, ()>>::new(());
		t.ayes = ayes;
		t.nays = nays;
		t
	}

	#[test]
	fn every_citizen_counts_once() {
		ExtBuilder::default().build().execute_with(|| {
			// Not a weight: a hundred citizens who vote aye are a hundred ayes, whatever they hold.
			let t = tally(100, 40);
			assert_eq!(VoteTally::<u32, ()>::ayes(&t, ()), 100);
			assert_eq!(
				VoteTally::<u32, ()>::approval(&t, ()),
				Perbill::from_rational(100u32, 140u32)
			);
		});
	}

	#[test]
	fn support_is_measured_against_the_whole_roll_not_the_turnout() {
		ExtBuilder::default().build().execute_with(|| {
			Roll::set(&1_000);
			let t = tally(100, 40);
			// 100 of a thousand citizens, not 100 of the 140 who bothered.
			assert_eq!(
				VoteTally::<u32, ()>::support(&t, ()),
				Perbill::from_rational(100u32, 1_000u32)
			);
			assert_eq!(
				VoteTally::<u32, ()>::approval(&t, ()),
				Perbill::from_rational(100u32, 140u32)
			);
		});
	}

	#[test]
	fn silence_is_not_consent() {
		ExtBuilder::default().build().execute_with(|| {
			Roll::set(&1_000);
			let t = tally(0, 0);
			// The failure this guards: dividing by turnout would make an unvoted proposal
			// unanimously supported.
			assert_eq!(VoteTally::<u32, ()>::support(&t, ()), Perbill::zero());
			assert_eq!(VoteTally::<u32, ()>::approval(&t, ()), Perbill::zero());
		});
	}

	#[test]
	fn an_empty_roll_gives_no_support_rather_than_dividing_by_zero() {
		ExtBuilder::default().build().execute_with(|| {
			Roll::set(&0);
			let t = tally(5, 0);
			assert_eq!(VoteTally::<u32, ()>::support(&t, ()), Perbill::zero());
		});
	}

	#[test]
	fn a_whole_roll_voting_aye_is_full_support_and_full_approval() {
		ExtBuilder::default().build().execute_with(|| {
			Roll::set(&7);
			let t = tally(7, 0);
			assert_eq!(VoteTally::<u32, ()>::support(&t, ()), Perbill::one());
			assert_eq!(VoteTally::<u32, ()>::approval(&t, ()), Perbill::one());
		});
	}
}

// ===== ANSWERING A STATE REFERENDUM =====
//
// The ballot box counts heads. Everything below is about that being true in the awkward
// cases: a citizen who changes their mind, one who tries to answer twice, one whose trust has
// gone to zero, and a question that is no longer open.

mod state_referendum {
	use super::*;
	use crate::{
		mock::{set_trust_score, MockElectorate, MockPollState, MockPolls, TestPolls},
		ReferendumVotes,
	};
	use pezframe_support::traits::{Polling, VoteTally};

	const VOTER: u64 = 7;
	const OTHER: u64 = 8;
	/// The poll the mock starts with, open and empty.
	const OPEN: u32 = 1;

	fn tally() -> crate::types::CitizenTally<MockElectorate> {
		TestPolls::as_ongoing(OPEN).expect("poll 1 is open in the mock").0
	}

	#[test]
	fn a_citizen_answers_once_and_the_count_moves_by_one() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(Welati::answer_referendum(RuntimeOrigin::signed(VOTER), OPEN, true));

			let t = tally();
			assert_eq!(t.ayes, 1);
			assert_eq!(t.nays, 0);
			assert_eq!(ReferendumVotes::<Test>::get(OPEN, VOTER), Some(true));
		});
	}

	#[test]
	fn two_citizens_are_two_voices_not_two_stakes() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(Welati::answer_referendum(RuntimeOrigin::signed(VOTER), OPEN, true));
			assert_ok!(Welati::answer_referendum(RuntimeOrigin::signed(OTHER), OPEN, true));

			assert_eq!(tally().ayes, 2);
		});
	}

	#[test]
	fn changing_sides_moves_the_count_by_one_not_two() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(Welati::answer_referendum(RuntimeOrigin::signed(VOTER), OPEN, true));
			assert_ok!(Welati::answer_referendum(RuntimeOrigin::signed(VOTER), OPEN, false));

			// The earlier aye has to come back out, or one citizen stands on both sides.
			let t = tally();
			assert_eq!(t.ayes, 0);
			assert_eq!(t.nays, 1);
			assert_eq!(ReferendumVotes::<Test>::get(OPEN, VOTER), Some(false));
		});
	}

	#[test]
	fn the_same_answer_twice_is_refused() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(Welati::answer_referendum(RuntimeOrigin::signed(VOTER), OPEN, true));
			assert_noop!(
				Welati::answer_referendum(RuntimeOrigin::signed(VOTER), OPEN, true),
				Error::<Test>::AlreadyAnsweredThatWay
			);
			assert_eq!(tally().ayes, 1);
		});
	}

	#[test]
	fn trust_of_zero_does_not_vote() {
		ExtBuilder::default().build().execute_with(|| {
			set_trust_score(0);
			assert_noop!(
				Welati::answer_referendum(RuntimeOrigin::signed(VOTER), OPEN, true),
				Error::<Test>::NoTrustToVote
			);
			set_trust_score(1000);
		});
	}

	#[test]
	fn a_question_that_is_not_open_takes_no_answer() {
		ExtBuilder::default().build().execute_with(|| {
			assert_noop!(
				Welati::answer_referendum(RuntimeOrigin::signed(VOTER), 99, true),
				Error::<Test>::ReferendumNotOngoing
			);
		});
	}

	#[test]
	fn answers_are_kept_until_the_question_is_settled_and_then_anyone_may_discard_them() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(Welati::answer_referendum(RuntimeOrigin::signed(VOTER), OPEN, true));
			assert_ok!(Welati::answer_referendum(RuntimeOrigin::signed(OTHER), OPEN, false));

			// While it is open the record stands: it is what stops a second vote.
			assert_noop!(
				Welati::clear_referendum_answers(RuntimeOrigin::signed(VOTER), OPEN, 10),
				Error::<Test>::ReferendumStillOngoing
			);

			let mut polls = MockPolls::get();
			polls.insert(OPEN, MockPollState::Completed(1, true));
			MockPolls::set(polls);

			assert_ok!(Welati::clear_referendum_answers(RuntimeOrigin::signed(VOTER), OPEN, 10));
			assert_eq!(ReferendumVotes::<Test>::get(OPEN, VOTER), None);
			assert_eq!(ReferendumVotes::<Test>::get(OPEN, OTHER), None);
		});
	}

	#[test]
	fn support_is_measured_against_the_whole_roll_not_against_who_turned_up() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(Welati::answer_referendum(RuntimeOrigin::signed(VOTER), OPEN, true));

			let t = tally();
			// One aye out of a hundred citizens is one percent of the register, however
			// lopsided the turnout was.
			assert_eq!(t.support(0u16), pezsp_runtime::Perbill::from_rational(1u32, 100u32));
			// ..and unanimous among those who came, which is a different question.
			assert_eq!(t.approval(0u16), pezsp_runtime::Perbill::from_percent(100));
		});
	}
}

// ===== CITIZENS' INITIATIVE =====
//
// No state lets one person put a question to the whole country alone. What a citizen may do
// alone is ask; the register decides whether the asking becomes a question. These are about
// where that line sits: the threshold moving with the roll, one signature per person, the
// window running out, and the deposit going somewhere rather than nowhere.

mod initiative {
	use super::*;
	use crate::{
		mock::{launched, set_citizen_count, set_trust_score, Balances},
		InitiativeBacking, Initiatives,
	};
	use pezframe_support::traits::{Currency, ReservableCurrency};

	const PROPOSER: u64 = 11;
	const SIGNER: u64 = 12;
	const TREASURY: u64 = 999;
	const DEPOSIT: u128 = 10;

	fn hash() -> pezsp_core::H256 {
		pezsp_core::H256::repeat_byte(9)
	}

	fn open() -> u32 {
		let _ = Balances::deposit_creating(&PROPOSER, 1_000);
		assert_ok!(Welati::open_initiative(RuntimeOrigin::signed(PROPOSER), 0, hash(), 42));
		0
	}

	#[test]
	fn opening_takes_a_deposit_and_counts_the_proposer_as_the_first_signature() {
		ExtBuilder::default().build().execute_with(|| {
			let id = open();

			let init = Initiatives::<Test>::get(id).expect("opened");
			assert_eq!(init.proposer, PROPOSER);
			assert_eq!(init.backing, 1, "asking is backing");
			assert_eq!(Balances::reserved_balance(PROPOSER), DEPOSIT);
			assert!(InitiativeBacking::<Test>::contains_key(id, PROPOSER));
		});
	}

	#[test]
	fn the_threshold_is_a_share_of_the_roll_not_a_fixed_count() {
		ExtBuilder::default().build().execute_with(|| {
			// One percent of the mock register.
			set_citizen_count(1_000);
			assert_eq!(Welati::initiative_threshold(), 10);

			set_citizen_count(100_000);
			assert_eq!(Welati::initiative_threshold(), 1_000);

			// An empty register must not let a proposal through unasked.
			set_citizen_count(0);
			assert_eq!(Welati::initiative_threshold(), 1);

			set_citizen_count(110);
		});
	}

	#[test]
	fn one_signature_per_citizen() {
		ExtBuilder::default().build().execute_with(|| {
			let id = open();
			assert_ok!(Welati::back_initiative(RuntimeOrigin::signed(SIGNER), id));
			assert_noop!(
				Welati::back_initiative(RuntimeOrigin::signed(SIGNER), id),
				Error::<Test>::AlreadyBacked
			);
			assert_eq!(Initiatives::<Test>::get(id).unwrap().backing, 2);
		});
	}

	#[test]
	fn trust_of_zero_neither_opens_nor_signs() {
		ExtBuilder::default().build().execute_with(|| {
			let id = open();
			set_trust_score(0);
			assert_noop!(
				Welati::back_initiative(RuntimeOrigin::signed(SIGNER), id),
				Error::<Test>::NoTrustToVote
			);
			assert_noop!(
				Welati::open_initiative(RuntimeOrigin::signed(SIGNER), 0, hash(), 42),
				Error::<Test>::NoTrustToVote
			);
			set_trust_score(1000);
		});
	}

	#[test]
	fn backing_stops_when_the_window_closes() {
		ExtBuilder::default().build().execute_with(|| {
			let id = open();
			System::set_block_number(System::block_number() + 101);
			assert_noop!(
				Welati::back_initiative(RuntimeOrigin::signed(SIGNER), id),
				Error::<Test>::InitiativeClosed
			);
		});
	}

	#[test]
	fn enough_backing_reaches_the_ballot_and_returns_the_deposit() {
		ExtBuilder::default().build().execute_with(|| {
			// Two signatures clear one percent of a register of a hundred and ten.
			let id = open();
			assert_ok!(Welati::back_initiative(RuntimeOrigin::signed(SIGNER), id));

			assert_ok!(Welati::launch_initiative(RuntimeOrigin::signed(SIGNER), id));

			assert_eq!(launched(), vec![(PROPOSER, 0u16, hash(), 42u32)]);
			assert_eq!(Balances::reserved_balance(PROPOSER), 0, "it did what it was taken for");
			assert!(Initiatives::<Test>::get(id).is_none());
			assert!(!InitiativeBacking::<Test>::contains_key(id, PROPOSER));
		});
	}

	#[test]
	fn too_little_backing_reaches_nothing() {
		ExtBuilder::default().build().execute_with(|| {
			set_citizen_count(100_000); // threshold 1000, and only the proposer has asked
			let id = open();
			assert_noop!(
				Welati::launch_initiative(RuntimeOrigin::signed(SIGNER), id),
				Error::<Test>::NotEnoughBacking
			);
			assert!(launched().is_empty());
			set_citizen_count(110);
		});
	}

	#[test]
	fn a_lapsed_deposit_goes_to_the_treasury_rather_than_nowhere() {
		ExtBuilder::default().build().execute_with(|| {
			set_citizen_count(100_000);
			let id = open();
			let before = Balances::free_balance(TREASURY);

			// Still inside the window, so there is nothing to settle yet.
			assert_noop!(
				Welati::close_lapsed_initiative(RuntimeOrigin::signed(SIGNER), id),
				Error::<Test>::InitiativeStillOpen
			);

			System::set_block_number(System::block_number() + 101);
			assert_ok!(Welati::close_lapsed_initiative(RuntimeOrigin::signed(SIGNER), id));

			assert_eq!(Balances::reserved_balance(PROPOSER), 0);
			// A forfeit deposit moves, it does not vanish: destroying HEZ would pay it out to
			// every other holder rather than to the state.
			assert_eq!(Balances::free_balance(TREASURY), before + DEPOSIT);
			assert!(Initiatives::<Test>::get(id).is_none());
			set_citizen_count(110);
		});
	}
}

mod initiative_cooldown {
	use super::*;
	use crate::mock::{set_citizen_count, Balances};
	use pezframe_support::traits::Currency;

	const PROPOSER: u64 = 21;

	fn hash() -> pezsp_core::H256 {
		pezsp_core::H256::repeat_byte(8)
	}

	#[test]
	fn a_lapsed_proposer_waits_before_asking_again() {
		ExtBuilder::default().build().execute_with(|| {
			set_citizen_count(100_000); // threshold far above one signature
			let _ = Balances::deposit_creating(&PROPOSER, 1_000);

			assert_ok!(Welati::open_initiative(RuntimeOrigin::signed(PROPOSER), 0, hash(), 42));
			System::set_block_number(System::block_number() + 101);
			assert_ok!(Welati::close_lapsed_initiative(RuntimeOrigin::signed(PROPOSER), 0));

			// Re-asking the next block would make the window settle nothing.
			assert_noop!(
				Welati::open_initiative(RuntimeOrigin::signed(PROPOSER), 0, hash(), 42),
				Error::<Test>::InitiativeCooldown
			);

			System::set_block_number(System::block_number() + 51);
			assert_ok!(Welati::open_initiative(RuntimeOrigin::signed(PROPOSER), 0, hash(), 42));

			set_citizen_count(110);
		});
	}
}

// ===== STORED ENUM INDEX TESTS =====
//
// A variant's index is what the chain wrote into storage. Move it and the old bytes decode as
// a different variant -- no error, no crash, a different answer. `VoteChoice` is the one that
// shows what that means: shift it and a recorded Aye reads back as Nay.
//
// The check reads `scale_info` rather than encoding values, so it also fails when a variant is
// added without a number, and it names the variant that moved.

use scale_info::{TypeDef, TypeInfo};

fn pinned<T: TypeInfo + 'static>(name: &str, expected: &[(&str, u8)]) {
	let info = <T as TypeInfo>::type_info();
	let TypeDef::Variant(v) = info.type_def() else { panic!("{name} is not an enum") };
	let got: Vec<(String, u8)> =
		v.variants().iter().map(|x| (x.name.to_string(), x.index)).collect();
	assert_eq!(
		got.len(),
		expected.len(),
		"{name}: the chain has {} variants, this list pins {} -- a new variant needs a \
		 number of its own and a line here",
		got.len(),
		expected.len()
	);
	for (i, (want_name, want_index)) in expected.iter().enumerate() {
		let (have_name, have_index) = &got[i];
		assert_eq!(
			(have_name.as_str(), *have_index),
			(*want_name, *want_index),
			"{name}: variant {i} is now {have_name}={have_index}, was {want_name}={want_index}"
		);
	}
}

#[test]
fn appointmentauthority_indices_are_pinned() {
	pinned::<AppointmentAuthority<crate::mock::Test>>(
		"AppointmentAuthority",
		&[("Parliament", 0u8), ("President", 1u8)],
	);
}

#[test]
fn appointmentstatus_indices_are_pinned() {
	pinned::<AppointmentStatus>(
		"AppointmentStatus",
		&[
			("WaitingNomination", 0u8),
			("WaitingPresidentialApproval", 1u8),
			("WaitingParliamentaryApproval", 2u8),
			("Approved", 3u8),
			("Rejected", 4u8),
			("Expired", 5u8),
		],
	);
}

#[test]
fn collectivedecisiontype_indices_are_pinned() {
	pinned::<CollectiveDecisionType>(
		"CollectiveDecisionType",
		&[
			("ParliamentSimpleMajority", 0u8),
			("ParliamentSuperMajority", 1u8),
			("ParliamentAbsoluteMajority", 2u8),
			("HybridDecision", 3u8),
			("ExecutiveDecision", 4u8),
			("VetoOverride", 5u8),
		],
	);
}

#[test]
fn committeetype_indices_are_pinned() {
	pinned::<CommitteeType>(
		"CommitteeType",
		&[
			("Budget", 0u8),
			("ForeignAffairs", 1u8),
			("Justice", 2u8),
			("Technology", 3u8),
			("Education", 4u8),
			("Health", 5u8),
			("Constitutional", 6u8),
		],
	);
}

#[test]
fn electionstatus_indices_are_pinned() {
	pinned::<ElectionStatus>(
		"ElectionStatus",
		&[
			("CandidacyPeriod", 0u8),
			("CampaignPeriod", 1u8),
			("VotingPeriod", 2u8),
			("Completed", 3u8),
			("Cancelled", 4u8),
			("FailedForTurnout", 5u8),
		],
	);
}

#[test]
fn nominationstatus_indices_are_pinned() {
	pinned::<NominationStatus>(
		"NominationStatus",
		&[
			("Pending", 0u8),
			("Approved", 1u8),
			("Rejected", 2u8),
			("Cancelled", 3u8),
			("Expired", 4u8),
		],
	);
}

#[test]
fn proposalpriority_indices_are_pinned() {
	pinned::<ProposalPriority>(
		"ProposalPriority",
		&[("Low", 0u8), ("Normal", 1u8), ("High", 2u8), ("Urgent", 3u8), ("Critical", 4u8)],
	);
}

#[test]
fn proposalstatus_indices_are_pinned() {
	pinned::<ProposalStatus>(
		"ProposalStatus",
		&[
			("Draft", 0u8),
			("Active", 1u8),
			("Approved", 2u8),
			("Rejected", 3u8),
			("Cancelled", 4u8),
			("Expired", 5u8),
			("Vetoed", 6u8),
			("UnderConstitutionalReview", 7u8),
		],
	);
}

#[test]
fn votechoice_indices_are_pinned() {
	pinned::<VoteChoice>("VoteChoice", &[("Aye", 0u8), ("Nay", 1u8), ("Abstain", 2u8)]);
}

#[test]
fn votetype_indices_are_pinned() {
	pinned::<VoteType>("VoteType", &[("Citizen", 0u8), ("Weighted", 1u8), ("Delegated", 2u8)]);
}

// ===== TERM EXPIRY =====

/// A President whose term has run out is not the President any more.
///
/// `tiki` keeps the term next to the office and offers `current_holder`, which reads the map
/// and then checks the expiry. Every authority check in this pallet read the raw map instead,
/// which is the exact case a term exists to prevent: nobody has to remove a lapsed holder for
/// them to keep the office, and `tiki`'s own comment says as much above `current_holder`.
///
/// The powers below are the ones that answer to the seat, so each has to stop answering:
/// appointing and dismissing a Prime Minister, seating a Diwan member, nominating and
/// approving an official, and proposing an executive decision.
#[test]
fn an_expired_president_holds_no_authority() {
	ExtBuilder::default().build().execute_with(|| {
		let president = 1u64;
		let nominee = 2u64;
		seat_president(president);
		make_citizen(nominee);

		// Give the seat a term, the way an election does.
		let ends_at = System::block_number() + 10;
		pezpallet_tiki::TikiExpiry::<Test>::insert(president, pezpallet_tiki::Tiki::Serok, ends_at);

		assert!(Welati::is_serok(&president), "still in office before the term ends");
		assert_ok!(Welati::ensure_serok(RuntimeOrigin::signed(president)));

		run_to_block(ends_at + 1);

		assert!(!Welati::is_serok(&president), "the term ran out and nobody removed them");
		assert_noop!(
			Welati::ensure_serok(RuntimeOrigin::signed(president)),
			pezsp_runtime::DispatchError::BadOrigin
		);
		// These two go through `ensure_root_or_serok`, which answers with its own reason
		// rather than a bare bad origin. Pinned as they are rather than smoothed over: what
		// matters is that the seat no longer opens them.
		assert_noop!(
			Welati::appoint_prime_minister(RuntimeOrigin::signed(president), nominee),
			Error::<Test>::NotAuthorizedToNominate
		);
		assert_noop!(
			Welati::appoint_diwan_member(RuntimeOrigin::signed(president), nominee),
			Error::<Test>::NotAuthorizedToNominate
		);
	});
}

// ===== THE AIRDROP POT =====
//
// Forty million HEZ sits in a keyless treasury instance on the Asset Hub and this chain is the
// only thing that can spend it. What follows holds the shape of that authority: who may
// propose, who must sign, when a third signature is required, and that the wait above the
// ceiling is real rather than decorative.
//
// The recipients are usually exchanges, whose customers cannot be enumerated on chain -- which
// is why the mechanism is discretionary at all. What replaces a rule is the signature count.

const PM: u64 = 7;
const TREASURER: u64 = 8;
const OUTSIDER: u64 = 9;
const EXCHANGE: u64 = 77;

fn seat_the_three_offices() {
	pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Serok, SEROK);
	pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::SerokWeziran, PM);
	pezpallet_tiki::TikiHolder::<Test>::insert(Tiki::Xezinedar, TREASURER);
}

/// Only the Prime Minister proposes.
#[test]
fn an_airdrop_is_proposed_by_the_prime_minister_alone() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_three_offices();
		for who in [SEROK, TREASURER, OUTSIDER] {
			assert_noop!(
				Welati::propose_airdrop(RuntimeOrigin::signed(who), EXCHANGE, 100, b"x".to_vec()),
				Error::<Test>::NotThePrimeMinister
			);
		}
		assert_ok!(Welati::propose_airdrop(
			RuntimeOrigin::signed(PM),
			EXCHANGE,
			100,
			b"listing".to_vec()
		));
		assert!(AirdropProposals::<Test>::get(0).is_some());
	});
}

/// A proposal alone moves nothing, and the President's signature is what completes a small one.
#[test]
fn a_small_airdrop_needs_the_president_and_nobody_else() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_three_offices();
		assert_ok!(Welati::propose_airdrop(
			RuntimeOrigin::signed(PM),
			EXCHANGE,
			100,
			b"listing".to_vec()
		));

		// Proposed is not approved.
		assert_noop!(
			Welati::pay_airdrop(RuntimeOrigin::signed(OUTSIDER), 0),
			Error::<Test>::AirdropNotApproved
		);

		assert_ok!(Welati::approve_airdrop(RuntimeOrigin::signed(SEROK), 0));
		let p = AirdropProposals::<Test>::get(0).unwrap();
		assert!(p.approved_by_president);
		// Below the ceiling there is no wait: payable in the block it was approved.
		assert_eq!(p.payable_from, System::block_number());
	});
}

/// Above the ceiling the President is not enough.
#[test]
fn a_large_airdrop_needs_the_treasurer_too() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_three_offices();
		let large = AirdropCeiling::get() + 1;
		assert_ok!(Welati::propose_airdrop(
			RuntimeOrigin::signed(PM),
			EXCHANGE,
			large,
			b"big".to_vec()
		));
		assert_ok!(Welati::approve_airdrop(RuntimeOrigin::signed(SEROK), 0));

		// One signature short, and the shortfall is what stops it -- not the delay.
		assert_noop!(
			Welati::pay_airdrop(RuntimeOrigin::signed(OUTSIDER), 0),
			Error::<Test>::AirdropNotApproved
		);

		assert_ok!(Welati::approve_airdrop(RuntimeOrigin::signed(TREASURER), 0));
		let p = AirdropProposals::<Test>::get(0).unwrap();
		assert!(p.approved_by_president && p.approved_by_treasurer);
	});
}

/// The wait above the ceiling is real, and it starts at the last signature.
///
/// If it started at the proposal, a proposal left sitting for a week would be payable the
/// moment it was signed -- and the week in which anyone could have objected would have passed
/// before there was anything to object to.
#[test]
fn a_large_airdrop_waits_and_the_wait_starts_at_the_last_signature() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_three_offices();
		let large = AirdropCeiling::get() + 1;
		assert_ok!(Welati::propose_airdrop(
			RuntimeOrigin::signed(PM),
			EXCHANGE,
			large,
			b"big".to_vec()
		));

		// A week passes with the proposal sitting unsigned.
		System::set_block_number(System::block_number() + LargeAirdropDelay::get() * 2);
		assert_ok!(Welati::approve_airdrop(RuntimeOrigin::signed(SEROK), 0));
		let signed_at = System::block_number();
		assert_ok!(Welati::approve_airdrop(RuntimeOrigin::signed(TREASURER), 0));

		let p = AirdropProposals::<Test>::get(0).unwrap();
		assert_eq!(
			p.payable_from,
			signed_at + LargeAirdropDelay::get(),
			"the wait must start when the last signature lands, not when the proposal was made"
		);
		assert_noop!(
			Welati::pay_airdrop(RuntimeOrigin::signed(OUTSIDER), 0),
			Error::<Test>::AirdropNotYetPayable
		);

		System::set_block_number(p.payable_from);
		assert_ok!(Welati::pay_airdrop(RuntimeOrigin::signed(OUTSIDER), 0));
		assert!(AirdropProposals::<Test>::get(0).is_none(), "a paid proposal is gone");
	});
}

/// The same office cannot sign twice to make up the numbers.
#[test]
fn one_office_cannot_supply_two_signatures() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_three_offices();
		let large = AirdropCeiling::get() + 1;
		assert_ok!(Welati::propose_airdrop(
			RuntimeOrigin::signed(PM),
			EXCHANGE,
			large,
			b"big".to_vec()
		));
		assert_ok!(Welati::approve_airdrop(RuntimeOrigin::signed(SEROK), 0));
		assert_noop!(
			Welati::approve_airdrop(RuntimeOrigin::signed(SEROK), 0),
			Error::<Test>::AlreadyApproved
		);
		// And someone holding no office at all supplies none.
		assert_noop!(
			Welati::approve_airdrop(RuntimeOrigin::signed(OUTSIDER), 0),
			Error::<Test>::NotThePresident
		);
	});
}

/// A paid proposal cannot be paid again.
#[test]
fn an_airdrop_cannot_be_paid_twice() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_three_offices();
		assert_ok!(Welati::propose_airdrop(
			RuntimeOrigin::signed(PM),
			EXCHANGE,
			100,
			b"listing".to_vec()
		));
		assert_ok!(Welati::approve_airdrop(RuntimeOrigin::signed(SEROK), 0));
		assert_ok!(Welati::pay_airdrop(RuntimeOrigin::signed(OUTSIDER), 0));
		assert_noop!(
			Welati::pay_airdrop(RuntimeOrigin::signed(OUTSIDER), 0),
			Error::<Test>::AirdropNotFound
		);
	});
}

/// The proposer may withdraw, and the President may refuse.
#[test]
fn a_proposal_can_be_withdrawn_or_refused() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_three_offices();
		for canceller in [PM, SEROK] {
			assert_ok!(Welati::propose_airdrop(
				RuntimeOrigin::signed(PM),
				EXCHANGE,
				100,
				b"x".to_vec()
			));
			let id = NextAirdropId::<Test>::get() - 1;
			assert_noop!(
				Welati::cancel_airdrop(RuntimeOrigin::signed(OUTSIDER), id),
				Error::<Test>::NotThePresident
			);
			assert_ok!(Welati::cancel_airdrop(RuntimeOrigin::signed(canceller), id));
			assert!(AirdropProposals::<Test>::get(id).is_none());
		}
	});
}

/// Ids are never reused, so an id in a log always names the same proposal.
#[test]
fn airdrop_ids_are_not_reused() {
	ExtBuilder::default().build().execute_with(|| {
		seat_the_three_offices();
		for _ in 0..3 {
			assert_ok!(Welati::propose_airdrop(
				RuntimeOrigin::signed(PM),
				EXCHANGE,
				100,
				b"x".to_vec()
			));
		}
		assert_ok!(Welati::cancel_airdrop(RuntimeOrigin::signed(PM), 1));
		assert_ok!(Welati::propose_airdrop(
			RuntimeOrigin::signed(PM),
			EXCHANGE,
			100,
			b"x".to_vec()
		));
		assert_eq!(NextAirdropId::<Test>::get(), 4, "a cancelled id is not handed out again");
		assert!(AirdropProposals::<Test>::get(1).is_none());
		assert!(AirdropProposals::<Test>::get(3).is_some());
	});
}
