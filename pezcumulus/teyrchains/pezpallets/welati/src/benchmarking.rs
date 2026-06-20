// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::types::*;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	// ----------------------------------------------------------------
	// ELECTION SYSTEM BENCHMARKS
	// ----------------------------------------------------------------
	#[benchmark]
	fn initiate_election() {
		// This benchmark doesn't need special preparation, just needs to be called with root

		#[extrinsic_call]
		initiate_election(RawOrigin::Root, ElectionType::Parliamentary, None, None);

		assert!(ActiveElections::<T>::get(0).is_some());
	}

	#[benchmark]
	fn register_candidate() {
		// --- SETUP ---
		Pezpallet::<T>::initiate_election(
			RawOrigin::Root.into(),
			ElectionType::Parliamentary,
			None,
			None,
		)
		.unwrap();

		// Simplified endorsers for benchmark - KYC bypass
		let endorsers: Vec<T::AccountId> = (0..T::ParliamentaryEndorsements::get())
			.map(|i| account("endorser", i, 0))
			.collect();

		let new_candidate: T::AccountId = whitelisted_caller();

		// KYC check is already bypassed in test environment

		#[extrinsic_call]
		register_candidate(RawOrigin::Signed(new_candidate.clone()), 0, None, endorsers);

		assert!(ElectionCandidates::<T>::get(0, &new_candidate).is_some());
	}

	#[benchmark]
	fn cast_vote() {
		// --- SETUP ---
		// 1. Prepare election and candidates
		Pezpallet::<T>::initiate_election(
			RawOrigin::Root.into(),
			ElectionType::Parliamentary,
			None,
			None,
		)
		.unwrap();

		let candidate: T::AccountId = account("candidate", 1, 0);
		let voter: T::AccountId = whitelisted_caller();

		// Simplified endorsers for benchmark
		let endorsers: Vec<T::AccountId> = (0..T::ParliamentaryEndorsements::get())
			.map(|i| account("endorser", i, 0))
			.collect();

		// KYC check is already bypassed in test environment

		Pezpallet::<T>::register_candidate(
			RawOrigin::Signed(candidate.clone()).into(),
			0,
			None,
			endorsers,
		)
		.unwrap();

		// 2. Advance to voting period
		let election = ActiveElections::<T>::get(0).unwrap();
		pezframe_system::Pezpallet::<T>::set_block_number(election.voting_start);

		let candidates_to_vote_for = vec![candidate];

		#[extrinsic_call]
		cast_vote(RawOrigin::Signed(voter.clone()), 0, candidates_to_vote_for, None);

		assert!(ElectionVotes::<T>::get(0, &voter).is_some());
	}

	#[benchmark]
	fn finalize_election() {
		// --- SETUP ---
		// 1. Prepare election, candidate and a vote
		Pezpallet::<T>::initiate_election(
			RawOrigin::Root.into(),
			ElectionType::Parliamentary,
			None,
			None,
		)
		.unwrap();

		let candidate: T::AccountId = account("candidate", 1, 0);
		let voter: T::AccountId = account("voter", 2, 0);

		let endorsers: Vec<T::AccountId> = (0..T::ParliamentaryEndorsements::get())
			.map(|i| account("endorser", i, 0))
			.collect();

		// KYC check is already bypassed in test environment

		Pezpallet::<T>::register_candidate(
			RawOrigin::Signed(candidate.clone()).into(),
			0,
			None,
			endorsers,
		)
		.unwrap();

		let election = ActiveElections::<T>::get(0).unwrap();
		pezframe_system::Pezpallet::<T>::set_block_number(election.voting_start);
		Pezpallet::<T>::cast_vote(
			RawOrigin::Signed(voter.clone()).into(),
			0,
			vec![candidate],
			None,
		)
		.unwrap();

		// 2. Advance to election end time
		pezframe_system::Pezpallet::<T>::set_block_number(election.end_block + 1u32.into());

		#[extrinsic_call]
		finalize_election(RawOrigin::Root, 0);

		assert!(ElectionResults::<T>::get(0).is_some());
	}

	// ----------------------------------------------------------------
	// APPOINTMENT SYSTEM BENCHMARKS
	// ----------------------------------------------------------------
	#[benchmark]
	fn nominate_official() {
		// --- SETUP ---
		let nominator: T::AccountId = whitelisted_caller();
		let nominee: T::AccountId = account("nominee", 2, 0);
		let justification = b"Test nomination".to_vec().try_into().unwrap();

		// Set nominator as Serok to pass authorization check
		CurrentOfficials::<T>::insert(GovernmentPosition::Serok, nominator.clone());

		// Ensure the role is not already filled (clean state for benchmark)
		// AppointedOfficials storage should be empty for Dadger role
		// This is important because we added RoleAlreadyFilled check in lib.rs

		#[extrinsic_call]
		nominate_official(
			RawOrigin::Signed(nominator),
			nominee,
			OfficialRole::Dadger,
			justification,
		);

		assert_eq!(NextAppointmentId::<T>::get(), 1);
		// Verify that the role is still not filled (nomination doesn't fill it, approval does)
		assert!(!AppointedOfficials::<T>::contains_key(OfficialRole::Dadger));
	}

	#[benchmark]
	fn approve_appointment() {
		// --- SETUP ---
		let approver: T::AccountId = whitelisted_caller();
		let nominator: T::AccountId = account("nominator", 2, 0);
		let nominee: T::AccountId = account("nominee", 3, 0);
		let justification = b"Test nomination".to_vec().try_into().unwrap();

		// Set nominator as Serok to pass authorization check for nomination
		CurrentOfficials::<T>::insert(GovernmentPosition::Serok, nominator.clone());

		// Use a different role (Dozger) to avoid conflicts with nominate_official benchmark
		Pezpallet::<T>::nominate_official(
			RawOrigin::Signed(nominator).into(),
			nominee.clone(),
			OfficialRole::Dozger,
			justification,
		)
		.unwrap();

		// Set approver as Serok to pass authorization check for approval
		CurrentOfficials::<T>::insert(GovernmentPosition::Serok, approver.clone());

		#[extrinsic_call]
		approve_appointment(RawOrigin::Signed(approver), 0);

		// Verify appointment ID incremented
		assert_eq!(NextAppointmentId::<T>::get(), 1);
		// CRITICAL: Verify that the role was assigned in AppointedOfficials storage
		// This tests the new storage write we added in lib.rs approve_appointment()
		assert_eq!(AppointedOfficials::<T>::get(OfficialRole::Dozger), Some(nominee));
	}

	// ----------------------------------------------------------------
	// COLLECTIVE DECISION BENCHMARKS
	// ----------------------------------------------------------------
	#[benchmark]
	fn submit_proposal() {
		// --- SETUP ---
		let proposer: T::AccountId = whitelisted_caller();

		// Simple member creation for benchmark
		let member: ParliamentMember<T> = ParliamentMember {
			account: proposer.clone(),
			elected_at: 0u32.into(),
			term_ends_at: 1000u32.into(),
			votes_participated: 0,
			total_votes_eligible: 0,
			participation_rate: 100,
			committees: Default::default(),
		};
		let members: BoundedVec<ParliamentMember<T>, T::ParliamentSize> =
			vec![member].try_into().unwrap();
		ParliamentMembers::<T>::put(members);

		let title = b"Test Proposal".to_vec().try_into().unwrap();
		let description = b"Test proposal description".to_vec().try_into().unwrap();

		#[extrinsic_call]
		submit_proposal(
			RawOrigin::Signed(proposer),
			title,
			description,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		);

		assert!(ActiveProposals::<T>::get(0).is_some());
	}

	#[benchmark]
	fn vote_on_proposal() {
		// --- SETUP ---
		let proposer: T::AccountId = account("proposer", 1, 0);
		let voter: T::AccountId = whitelisted_caller();

		// Create two members (proposer and voter)
		let member1: ParliamentMember<T> = ParliamentMember {
			account: proposer.clone(),
			elected_at: 0u32.into(),
			term_ends_at: 1000u32.into(),
			votes_participated: 0,
			total_votes_eligible: 0,
			participation_rate: 100,
			committees: Default::default(),
		};
		let member2: ParliamentMember<T> = ParliamentMember {
			account: voter.clone(),
			elected_at: 0u32.into(),
			term_ends_at: 1000u32.into(),
			votes_participated: 0,
			total_votes_eligible: 0,
			participation_rate: 100,
			committees: Default::default(),
		};
		let members: BoundedVec<ParliamentMember<T>, T::ParliamentSize> =
			vec![member1, member2].try_into().unwrap();
		ParliamentMembers::<T>::put(members);

		let title = b"Test Proposal".to_vec().try_into().unwrap();
		let description = b"Test proposal description".to_vec().try_into().unwrap();
		Pezpallet::<T>::submit_proposal(
			RawOrigin::Signed(proposer).into(),
			title,
			description,
			CollectiveDecisionType::ParliamentSimpleMajority,
			ProposalPriority::Normal,
			None,
		)
		.unwrap();

		let proposal = ActiveProposals::<T>::get(0).unwrap();
		pezframe_system::Pezpallet::<T>::set_block_number(proposal.voting_starts_at + 1u32.into());

		let rationale = Some(b"Test vote rationale".to_vec().try_into().unwrap());

		// Ensure voter hasn't voted yet (clean state for benchmark)
		// This tests our new ProposalAlreadyVoted check
		assert!(!CollectiveVotes::<T>::contains_key(0, &voter));

		#[extrinsic_call]
		vote_on_proposal(RawOrigin::Signed(voter.clone()), 0, VoteChoice::Aye, rationale);

		// Verify vote was recorded
		assert!(CollectiveVotes::<T>::get(0, &voter).is_some());
		// Verify the vote details are correct
		let vote = CollectiveVotes::<T>::get(0, &voter).unwrap();
		assert_eq!(vote.vote, VoteChoice::Aye);
		// This benchmark successfully tests:
		// 1. NotAuthorizedToVote check (voter is in ParliamentMembers)
		// 2. ProposalAlreadyVoted check (voter hasn't voted before)
	}

	impl_benchmark_test_suite!(
		Pezpallet,
		crate::mock::ExtBuilder::default().build(),
		crate::mock::Test
	);
}
