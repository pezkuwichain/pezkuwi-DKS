// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::types::*;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;
use pezpallet_tiki::Tiki;

#[benchmarks]
mod benchmarks {
	use super::*;

	// ----------------------------------------------------------------
	// SHARED SETUP
	//
	// Four of this pallet's benchmarks did not execute at all, and the weights recorded for
	// them could not be regenerated. Each failure was the runtime refusing something the setup
	// had not arranged -- which is the benchmark being wrong, not the pallet.
	// ----------------------------------------------------------------

	/// Record the endorsements `register_candidate` insists on, and return the endorsers.
	///
	/// The check reads `Endorsements` and asks that each name on the list put itself there for
	/// this candidate. It is deliberately not behind the `runtime-benchmarks` bypass the trust
	/// and KYC checks use -- what it replaced was a check that did not check -- so a benchmark
	/// has to satisfy it rather than skip it. Writing the storage directly is the setup doing
	/// what `endorse_candidate` would do, without measuring a hundred of those calls inside
	/// this one.
	fn endorse_for<T: Config>(election_id: u32, candidate: &T::AccountId) -> Vec<T::AccountId> {
		let endorsers: Vec<T::AccountId> = (0..T::ParliamentaryEndorsements::get())
			.map(|i| account("endorser", i, 0))
			.collect();
		for endorser in &endorsers {
			Endorsements::<T>::insert(election_id, endorser, candidate.clone());
		}
		endorsers
	}

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

		let new_candidate: T::AccountId = whitelisted_caller();
		let endorsers = endorse_for::<T>(0, &new_candidate);

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

		let endorsers = endorse_for::<T>(0, &candidate);

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

	/// Counting a full ballot, which is what this call costs on its worst day.
	///
	/// The cost is in the candidate list: votes are tallied as they are cast, and this walks
	/// `election.candidates` reading each one's running total. The list is bounded at five
	/// hundred, so five hundred is the number to measure -- the previous benchmark registered
	/// one candidate, and had it ever run it would have priced a five-hundred-candidate count
	/// at a one-candidate count. It did not run: it handed `Root` to a call that takes a
	/// signed origin, so it failed with `BadOrigin` and the recorded weight came from
	/// somewhere else.
	///
	/// A linear component would let a three-candidate election pay for three. That is the
	/// better shape and it changes `WeightInfo`'s signature, so it is written down as work
	/// rather than smuggled in here; a ceiling that overcharges is the safe direction to be
	/// wrong in while it waits.
	#[benchmark]
	fn finalize_election() {
		Pezpallet::<T>::initiate_election(
			RawOrigin::Root.into(),
			ElectionType::Parliamentary,
			None,
			None,
		)
		.unwrap();

		let mut election = ActiveElections::<T>::get(0).unwrap();

		// The ballot, written straight to storage. Five hundred `register_candidate` calls
		// would be five hundred calls' worth of setup for a list this only reads.
		let full: u32 = 500;
		let mut candidates = Vec::new();
		for i in 0..full {
			let candidate: T::AccountId = account("candidate", i, 0);
			ElectionCandidates::<T>::insert(
				0,
				&candidate,
				CandidateInfo::<T> {
					account: candidate.clone(),
					district_id: None,
					registered_at: pezframe_system::Pezpallet::<T>::block_number(),
					endorsers: Default::default(),
					vote_count: full.saturating_sub(i),
					deposit_paid: 0,
					campaign_data: Default::default(),
				},
			);
			candidates.push(candidate);
		}
		election.candidates = candidates.try_into().expect("five hundred is the bound");

		// Turnout, so the count reaches the tally rather than stopping at the quorum. A
		// benchmark that failed for turnout would measure the refusal.
		election.total_votes = u32::MAX;
		election.status = ElectionStatus::VotingPeriod;
		ActiveElections::<T>::insert(0, &election);

		pezframe_system::Pezpallet::<T>::set_block_number(election.end_block + 1u32.into());

		// Permissionless, and has been since the count stopped waiting on an outside key.
		let counter: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		finalize_election(RawOrigin::Signed(counter), 0);

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
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::Serok, nominator.clone());

		// Ensure the role is not already filled (clean state for benchmark)
		// nobody holds the bench yet
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
		assert!(pezpallet_tiki::TikiHolder::<T>::get(Tiki::Dadger).is_none());
	}

	#[benchmark]
	fn approve_appointment() {
		// --- SETUP ---
		let approver: T::AccountId = whitelisted_caller();
		let nominator: T::AccountId = account("nominator", 2, 0);
		let nominee: T::AccountId = account("nominee", 3, 0);
		let justification = b"Test nomination".to_vec().try_into().unwrap();

		// Set nominator as Serok to pass authorization check for nomination
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::Serok, nominator.clone());

		// Use a different role (Dozger) to avoid conflicts with nominate_official benchmark
		Pezpallet::<T>::nominate_official(
			RawOrigin::Signed(nominator).into(),
			nominee.clone(),
			OfficialRole::Dozger,
			justification,
		)
		.unwrap();

		// Set approver as Serok to pass authorization check for approval
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::Serok, approver.clone());

		// Seating writes the nominee's register NFT metadata, so the nominee has to have one.
		<T as Config>::BenchmarkHelper::make_citizen(&nominee);

		#[extrinsic_call]
		approve_appointment(RawOrigin::Signed(approver), 0);

		// Verify appointment ID incremented
		assert_eq!(NextAppointmentId::<T>::get(), 1);
		// The appointment has to reach the register, not just this pallet's own map.
		// This tests the new storage write we added in lib.rs approve_appointment()
		assert!(pezpallet_tiki::UserTikis::<T>::get(&nominee).contains(&Tiki::Dozger));
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

	// ----------------------------------------------------------------
	// AIRDROP AND PRESALE POT BENCHMARKS
	//
	// These seven shipped borrowing `nominate_official` and friends, on the reasoning that the
	// work is the same shape. It is not: `propose_presale` also opens a proposal, which is two
	// more writes, and both payment calls send an XCM. A stand-in was measured wrong in this
	// tree once already -- the TNPoS committee weights undercharged by a hundredfold -- and the
	// direction of that error is the dangerous one, because a call that costs less than it
	// takes is a call somebody can repeat.
	// ----------------------------------------------------------------

	/// Seat the offices these calls check, and return them.
	fn seat_pot_offices<T: Config>() -> (T::AccountId, T::AccountId, T::AccountId) {
		let president: T::AccountId = account("president", 20, 0);
		let prime_minister: T::AccountId = account("prime_minister", 21, 0);
		let finance_minister: T::AccountId = account("finance_minister", 22, 0);
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::Serok, president.clone());
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::SerokWeziran, prime_minister.clone());
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::WezireDarayiye, finance_minister.clone());
		(president, prime_minister, finance_minister)
	}

	#[benchmark]
	fn propose_airdrop() {
		let (_, prime_minister, _) = seat_pot_offices::<T>();
		let beneficiary: T::AccountId = account("beneficiary", 23, 0);

		#[extrinsic_call]
		propose_airdrop(
			RawOrigin::Signed(prime_minister),
			beneficiary,
			1_000u128,
			// The bound, not a short string: the record is a `BoundedVec` and the worst case
			// is the one that fills it.
			vec![0u8; 256],
		);

		assert!(AirdropProposals::<T>::get(0).is_some());
	}

	#[benchmark]
	fn approve_airdrop() {
		let (president, prime_minister, _) = seat_pot_offices::<T>();
		let beneficiary: T::AccountId = account("beneficiary", 23, 0);
		Pezpallet::<T>::propose_airdrop(
			RawOrigin::Signed(prime_minister).into(),
			beneficiary,
			1_000u128,
			vec![0u8; 256],
		)
		.unwrap();

		#[extrinsic_call]
		approve_airdrop(RawOrigin::Signed(president), 0);

		assert!(AirdropProposals::<T>::get(0).unwrap().approved_by_president);
	}

	#[benchmark]
	fn pay_airdrop() {
		let (president, prime_minister, _) = seat_pot_offices::<T>();
		let beneficiary: T::AccountId = account("beneficiary", 23, 0);
		let caller: T::AccountId = whitelisted_caller();
		Pezpallet::<T>::propose_airdrop(
			RawOrigin::Signed(prime_minister).into(),
			beneficiary,
			1_000u128,
			vec![0u8; 256],
		)
		.unwrap();
		Pezpallet::<T>::approve_airdrop(RawOrigin::Signed(president).into(), 0).unwrap();

		// Without this the router refuses the spend and the benchmark measures the
		// refusal instead of the send -- which is how a call that sends an XCM ends up
		// priced as one that does not.
		<T as Config>::BenchmarkHelper::ensure_treasury_reachable();

		#[extrinsic_call]
		pay_airdrop(RawOrigin::Signed(caller), 0);

		assert!(AirdropProposals::<T>::get(0).is_none());
	}

	#[benchmark]
	fn cancel_airdrop() {
		let (_, prime_minister, _) = seat_pot_offices::<T>();
		let beneficiary: T::AccountId = account("beneficiary", 23, 0);
		Pezpallet::<T>::propose_airdrop(
			RawOrigin::Signed(prime_minister.clone()).into(),
			beneficiary,
			1_000u128,
			vec![0u8; 256],
		)
		.unwrap();

		#[extrinsic_call]
		cancel_airdrop(RawOrigin::Signed(prime_minister), 0);

		assert!(AirdropProposals::<T>::get(0).is_none());
	}

	#[benchmark]
	fn propose_presale() {
		let (_, _, finance_minister) = seat_pot_offices::<T>();
		let beneficiary: T::AccountId = account("beneficiary", 23, 0);

		#[extrinsic_call]
		// `Locked` rather than `Transfer`: it is the arm that does the extra arithmetic, and a
		// benchmark of the cheaper arm would price the dearer one.
		propose_presale(
			RawOrigin::Signed(finance_minister),
			PresaleVerb::Locked { months: 12 },
			beneficiary,
			1_000u128,
			vec![0u8; 256],
		);

		// Both records: the release and the vote it hangs on. This call writes two.
		assert!(PresaleProposals::<T>::get(0).is_some());
		assert!(ActiveProposals::<T>::get(0).is_some());
	}

	#[benchmark]
	fn execute_presale() {
		let (_, _, finance_minister) = seat_pot_offices::<T>();
		let beneficiary: T::AccountId = account("beneficiary", 23, 0);
		let caller: T::AccountId = whitelisted_caller();
		Pezpallet::<T>::propose_presale(
			RawOrigin::Signed(finance_minister).into(),
			PresaleVerb::Transfer,
			beneficiary,
			1_000u128,
			vec![0u8; 256],
		)
		.unwrap();

		// Carry the vote. The tally is written here rather than cast, because a hundred and
		// one `vote_on_proposal` calls would be benchmarked into this one's cost.
		let vote_id = PresaleProposals::<T>::get(0).unwrap().vote_id;
		ActiveProposals::<T>::mutate(vote_id, |maybe| {
			let p = maybe.as_mut().unwrap();
			p.aye_votes = p.threshold;
		});
		Pezpallet::<T>::finalize_proposal(RawOrigin::Signed(caller.clone()).into(), vote_id)
			.unwrap();

		// Without this the router refuses the spend and the benchmark measures the
		// refusal instead of the send -- which is how a call that sends an XCM ends up
		// priced as one that does not.
		<T as Config>::BenchmarkHelper::ensure_treasury_reachable();

		#[extrinsic_call]
		execute_presale(RawOrigin::Signed(caller), 0);

		assert!(PresaleProposals::<T>::get(0).is_none());
		assert_eq!(PresaleSentTotal::<T>::get(), 1_000u128);
	}

	#[benchmark]
	fn cancel_presale() {
		let (_, _, finance_minister) = seat_pot_offices::<T>();
		let beneficiary: T::AccountId = account("beneficiary", 23, 0);
		Pezpallet::<T>::propose_presale(
			RawOrigin::Signed(finance_minister.clone()).into(),
			PresaleVerb::Transfer,
			beneficiary,
			1_000u128,
			vec![0u8; 256],
		)
		.unwrap();

		#[extrinsic_call]
		cancel_presale(RawOrigin::Signed(finance_minister), 0);

		assert!(PresaleProposals::<T>::get(0).is_none());
	}

	impl_benchmark_test_suite!(
		Pezpallet,
		crate::mock::ExtBuilder::default().build(),
		crate::mock::Test
	);
}
