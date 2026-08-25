// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{
	mock::{
		add_parliament_member, endorsed_by, last_event, make_citizen, run_to_block, seat_president,
		ExtBuilder, RuntimeEvent, RuntimeOrigin, System, Test, Welati,
	},
	types::*,
	CurrentOfficials, Error, Event as WelatiEvent, GovernmentPosition,
};
use pezframe_support::{assert_noop, assert_ok, BoundedVec};
use pezsp_runtime::traits::BadOrigin;

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

#[test]
fn nominate_official_works() {
	ExtBuilder::default().build().execute_with(|| {
		// Setup: Make user 1 the Serok (President) so they can nominate
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 1);

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
		// Setup: Make user 1 the Serok
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 1);

		let justification = b"Qualified candidate".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(1),
			2,
			OfficialRole::Dadger,
			justification,
		));

		assert_ok!(Welati::approve_appointment(RuntimeOrigin::signed(1), 0,));
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
		// Setup: Make user 1 the Serok
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 1);

		let justification = b"Experienced lawyer".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(1),
			5,
			OfficialRole::Dadger,
			justification,
		));

		assert_ok!(Welati::approve_appointment(RuntimeOrigin::signed(1), 0,));

		if let Some(process) = Welati::appointment_processes(0) {
			assert_eq!(process.status, AppointmentStatus::Approved);
		}
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
fn nominate_official_fails_role_already_filled() {
	ExtBuilder::default().build().execute_with(|| {
		// Set Serok (President) first
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 1);

		let justification1 = b"Qualified candidate".to_vec().try_into().unwrap();

		// Nominate Dadger
		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(1),
			2,
			OfficialRole::Dadger,
			justification1,
		));

		let process_id = Welati::next_appointment_id() - 1;

		// Approve appointment
		assert_ok!(Welati::approve_appointment(RuntimeOrigin::signed(1), process_id,));

		let justification2 = b"Another candidate".to_vec().try_into().unwrap();

		// Try to nominate same role again
		assert_noop!(
			Welati::nominate_official(
				RuntimeOrigin::signed(1),
				3,
				OfficialRole::Dadger,
				justification2,
			),
			Error::<Test>::RoleAlreadyFilled
		);
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
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 1);

		let justification = b"Qualified candidate".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(1),
			2,
			OfficialRole::Dadger,
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
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 1);

		let justification = b"Qualified candidate".to_vec().try_into().unwrap();

		assert_ok!(Welati::nominate_official(
			RuntimeOrigin::signed(1),
			2,
			OfficialRole::Dadger,
			justification,
		));

		let process_id = Welati::next_appointment_id() - 1;

		// First approval
		assert_ok!(Welati::approve_appointment(RuntimeOrigin::signed(1), process_id,));

		// Try to approve again
		assert_noop!(
			Welati::approve_appointment(RuntimeOrigin::signed(1), process_id,),
			Error::<Test>::AppointmentAlreadyProcessed
		);
	});
}

#[test]
fn approve_appointment_process_not_found() {
	ExtBuilder::default().build().execute_with(|| {
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 1);

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
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 1);

		let officials = vec![
			(2, OfficialRole::Dadger),
			(3, OfficialRole::Dozger),
			(4, OfficialRole::Xezinedar),
		];

		for (nominee, role) in officials {
			let justification = b"Qualified candidate".to_vec().try_into().unwrap();

			assert_ok!(Welati::nominate_official(
				RuntimeOrigin::signed(1),
				nominee,
				role,
				justification,
			));

			let process_id = Welati::next_appointment_id() - 1;

			assert_ok!(Welati::approve_appointment(RuntimeOrigin::signed(1), process_id,));

			// Verify appointment was processed
			assert!(Welati::appointment_processes(process_id).is_some());
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
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 1);

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

			assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::signed(SEROK), PM));

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
			assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::root(), PM));
			assert_eq!(holder_of(Tiki::SerokWeziran), Some(PM));
		});
	}

	#[test]
	fn the_prime_minister_appoints_ministers_and_the_president_does_not() {
		ExtBuilder::default().build().execute_with(|| {
			seat_president(SEROK);
			make_citizen(PM);
			make_citizen(MINISTER);
			assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::signed(SEROK), PM));

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
			assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::root(), PM));
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
			assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::root(), PM));

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
			assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::root(), PM));

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
			assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::root(), PM));
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

			assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::root(), PM));
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
		assert_ok!(Welati::appoint_prime_minister(RuntimeOrigin::root(), PM));
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
			// beneficiary and the amount as the treasury chain will decode them.
			let expected = (70u8, 1u8, BENEFICIARY, codec::Compact(400u128)).encode();
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
		CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 2);
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
			CurrentOfficials::<Test>::insert(GovernmentPosition::Serok, 3);
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
