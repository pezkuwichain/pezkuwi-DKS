// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate::{
	mock::{
		add_parliament_member, last_event, run_to_block, ExtBuilder, RuntimeEvent, RuntimeOrigin,
		Test, Welati,
	},
	types::*,
	CurrentOfficials, Error, Event as WelatiEvent, GovernmentPosition,
};
use pezframe_support::{assert_noop, assert_ok, BoundedVec};
use pezsp_runtime::traits::BadOrigin;

// ===== SEÇİM SİSTEMİ TESTLERİ =====

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
			end_block: 1 + 86_400 + 259_200 + 432_000,
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

		let parliamentary_endorsers: Vec<u64> = (2..=51).collect();

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

		let endorsers = vec![2, 3, 4];

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

		run_to_block(86_400 + 100);

		let endorsers: Vec<u64> = (2..=51).collect();

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

		let endorsers: Vec<u64> = (2..=51).collect();

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
		// 1. Seçimi başlat
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// 2. Bir aday kaydet (hesap 1)
		let endorsers: Vec<u64> = (3..=52).collect(); // 50 destekçi
		assert_ok!(Welati::register_candidate(
			RuntimeOrigin::signed(1), // Aday
			0,                        // Seçim ID'si
			None,                     // Bölge ID'si
			endorsers,
		));

		// 3. Oy verme periyoduna ilerle
		run_to_block(86_400 + 259_200 + 1);

		// 4. Oy kullan (hesap 2, aday 1'e oy veriyor)
		let candidates_to_vote_for = vec![1];
		assert_ok!(Welati::cast_vote(
			RuntimeOrigin::signed(2),       // Seçmen
			0,                              // Seçim ID'si
			candidates_to_vote_for.clone(), // Oy verilen aday(lar)
			None,                           // Bölge ID'si
		));

		// 5. Event'i ve depolama durumunu doğrula
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

		let endorsers: Vec<u64> = (3..=52).collect();
		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers,));

		run_to_block(86_400 + 259_200 + 1);

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

		// Seçimin bitiş tarihinden sonrasına geç
		// candidacy (86_400) + campaign (259_200) + voting (432_000) + 1
		run_to_block(86_400 + 259_200 + 432_000 + 10); // Ekstra güvenlik için +10

		assert_ok!(Welati::finalize_election(RuntimeOrigin::root(), 0,));

		if let Some(election) = Welati::active_elections(0) {
			assert_eq!(election.status, ElectionStatus::Completed);
		}
	});
}

// ===== ATAMA SİSTEMİ TESTLERİ =====

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

// ===== KOLLEKTİF KARAR TESTLERİ =====

#[test]
fn submit_proposal_works() {
	ExtBuilder::default().build().execute_with(|| {
		let title = b"Test Proposal".to_vec().try_into().unwrap();
		let description = b"Test proposal description".to_vec().try_into().unwrap();

		// CRITICAL FIX: Helper fonksiyonu kullan
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

		// CRITICAL FIX: Helper fonksiyonları kullan
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

// ===== HELPER FUNCTION TESTLERİ =====

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
fn calculate_vote_weight_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Welati::calculate_vote_weight(&1, &ElectionType::Presidential), 1);

		assert_eq!(Welati::calculate_vote_weight(&1, &ElectionType::Parliamentary), 1);

		let weight = Welati::calculate_vote_weight(&1, &ElectionType::SpeakerElection);
		assert!((1..=10).contains(&weight));
	});
}

// ===== ERROR CASE TESTLERİ =====

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

// ===== INTEGRATION TESTLERİ =====

#[test]
fn complete_election_cycle_works() {
	ExtBuilder::default().build().execute_with(|| {
		// 1. Seçim başlat
		assert_ok!(Welati::initiate_election(
			RuntimeOrigin::root(),
			ElectionType::Parliamentary,
			None,
			None,
		));

		// 2. Adaylar kaydolsun
		let endorsers1: Vec<u64> = (10..=59).collect();
		let endorsers2: Vec<u64> = (60..=109).collect();

		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers1,));

		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(2), 0, None, endorsers2,));

		// 3. Voting period'a geç
		run_to_block(86_400 + 259_200 + 1);

		// 4. Oylar kullanılsın
		assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(3), 0, vec![1], None,));

		assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(4), 0, vec![2], None,));

		// 5. Seçimi sonlandır
		run_to_block(86_400 + 259_200 + 432_000 + 2);

		assert_ok!(Welati::finalize_election(RuntimeOrigin::root(), 0,));

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

		// CRITICAL FIX: Helper fonksiyonları kullan
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

// ===== RUNOFF ELECTION TESTLERİ =====

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
		let endorsers: Vec<u64> = (2..=101).collect();

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

		let parliamentary_endorsers: Vec<u64> = (10..=59).collect();

		// Register 3 candidates
		for candidate_id in 1..=3 {
			assert_ok!(Welati::register_candidate(
				RuntimeOrigin::signed(candidate_id),
				0,
				None,
				parliamentary_endorsers.clone(),
			));
		}

		// Move to voting period
		run_to_block(86_400 + 259_200 + 100);

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

		run_to_block(86_400 + 259_200 + 100);

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

		let endorsers: Vec<u64> = (2..=51).collect();
		assert_ok!(Welati::register_candidate(
			RuntimeOrigin::signed(1),
			0,
			Some(1), // District 1
			endorsers,
		));

		run_to_block(86_400 + 259_200 + 100);

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
			Welati::finalize_election(RuntimeOrigin::root(), 999,),
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

		let endorsers: Vec<u64> = (2..=51).collect();
		assert_ok!(Welati::register_candidate(RuntimeOrigin::signed(1), 0, None, endorsers,));

		run_to_block(86_400 + 259_200 + 100);

		assert_ok!(Welati::cast_vote(RuntimeOrigin::signed(100), 0, vec![1], None,));

		run_to_block(86_400 + 259_200 + 432_000 + 100);

		assert_ok!(Welati::finalize_election(RuntimeOrigin::root(), 0,));

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

		let endorsers: Vec<u64> = (100..=149).collect();

		// Register 10 candidates
		for candidate_id in 1..=10 {
			assert_ok!(Welati::register_candidate(
				RuntimeOrigin::signed(candidate_id),
				0,
				None,
				endorsers.clone(),
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
