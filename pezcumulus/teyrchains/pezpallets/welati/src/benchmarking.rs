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
	fn finalize_election(c: Linear<0, MAX_ELECTION_CANDIDATES>) {
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
		//
		// Sized by the component rather than fixed at the bound, and the call is why: it charges
		// `finalize_election(MAX_ELECTION_CANDIDATES)` up front and refunds
		// `finalize_election(counted)` once it knows the real number, so the weight function has
		// to have a slope for the refund to mean anything. Fixed at five hundred the bencher
		// emitted `finalize_election()` with no argument at all -- it would not even compile
		// against the trait, and the slope in the hand-written body was derived from `DbWeight`
		// by a person rather than measured. Measured 2026-09-21.
		let full: u32 = c;
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
		election.candidates = candidates
			.try_into()
			.expect("the component is bounded by MAX_ELECTION_CANDIDATES; qed");

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

	// ----------------------------------------------------------------
	// LIVENESS: the court's seats and the two elected single-holder offices
	//
	// These four borrowed `nominate_official`'s weight until they had benchmarks of their own.
	// ----------------------------------------------------------------

	/// Fill the bench to its size with `member` in the last seat, so every lookup that searches
	/// the bench for them -- not only the filter that drops them -- walks every seat first.
	fn a_full_bench_with<T: Config>(member: &T::AccountId) {
		let seats = T::DiwanSize::get();
		let bench: Vec<DiwanMember<T>> = (0..seats)
			.map(|i| {
				let account = if i + 1 == seats { member.clone() } else { account("judge", i, 0) };
				DiwanMember {
					account,
					appointed_at: Zero::zero(),
					term_ends_at: T::CourtTermLength::get(),
					appointed_by: AppointmentAuthority::Parliament,
				}
			})
			.collect();
		DiwanMembers::<T>::put(BoundedVec::<_, T::DiwanSize>::try_from(bench).unwrap());
	}

	#[benchmark]
	fn court_check_in() {
		let judge: T::AccountId = whitelisted_caller();
		a_full_bench_with::<T>(&judge);

		#[extrinsic_call]
		court_check_in(RawOrigin::Signed(judge.clone()));

		assert!(CourtLastActive::<T>::get(&judge).is_some());
	}

	#[benchmark]
	fn vacate_inactive_court_seat() {
		let judge: T::AccountId = account("silent", 0, 0);
		let caller: T::AccountId = whitelisted_caller();
		// A real citizen holding the court's tiki, so the revocation does its full work.
		<T as Config>::BenchmarkHelper::make_citizen(&judge);
		pezpallet_tiki::Pezpallet::<T>::internal_grant_role(&judge, Tiki::EndameDiwane).unwrap();
		a_full_bench_with::<T>(&judge);
		pezframe_system::Pezpallet::<T>::set_block_number(T::CourtInactivityPeriod::get());

		#[extrinsic_call]
		vacate_inactive_court_seat(RawOrigin::Signed(caller), judge.clone());

		assert!(!Pezpallet::<T>::is_diwan_member(&judge));
		assert_eq!(DiwanMembers::<T>::get().len() as u32, T::DiwanSize::get() - 1);
	}

	#[benchmark]
	fn office_check_in() {
		let president: T::AccountId = whitelisted_caller();
		<T as Config>::BenchmarkHelper::make_citizen(&president);
		Pezpallet::<T>::seat_unique_tiki(&president, Tiki::Serok).unwrap();

		#[extrinsic_call]
		office_check_in(RawOrigin::Signed(president.clone()), Tiki::Serok);

		assert_eq!(
			OfficeLastActive::<T>::get(Tiki::Serok).map(|(holder, _)| holder),
			Some(president)
		);
	}

	#[benchmark]
	fn vacate_silent_office() {
		let president: T::AccountId = account("silent", 1, 0);
		let caller: T::AccountId = whitelisted_caller();
		<T as Config>::BenchmarkHelper::make_citizen(&president);
		Pezpallet::<T>::seat_unique_tiki(&president, Tiki::Serok).unwrap();
		let now = pezframe_system::Pezpallet::<T>::block_number();
		// The last check-in on record is a predecessor's, so the silence is counted from the
		// start of the term: the branch that reads the calendar as well.
		OfficeLastActive::<T>::insert(Tiki::Serok, (account::<T::AccountId>("former", 0, 0), now));
		// An elected mandate: only an office with a recorded term can be emptied for silence.
		TermEnds::<T>::insert(ElectionType::Presidential, now + T::TermLength::get());
		pezframe_system::Pezpallet::<T>::set_block_number(now + T::OfficeInactivityPeriod::get());

		#[extrinsic_call]
		vacate_silent_office(RawOrigin::Signed(caller), Tiki::Serok);

		assert!(pezpallet_tiki::Pezpallet::<T>::current_holder(&Tiki::Serok).is_none());
		assert!(OfficeLastActive::<T>::get(Tiki::Serok).is_none());
	}

	// ----------------------------------------------------------------
	// THE CALLS THAT BORROWED `nominate_official`
	//
	// Seventeen more calls were annotated with `nominate_official`'s weight, and most of them
	// do work that one does not: seating or unseating an office rewrites the holder's register
	// NFT and their trust score, two of them send an XCM, and the bench and the house are
	// relayed to their collectives whole. Each benchmark below sets up the dearest path its
	// call can take.
	// ----------------------------------------------------------------

	/// Make `who` a citizen in both registers: identity-kyc's roll and the tiki NFT.
	///
	/// The two are separate on purpose and different calls read different ones -- `claim_region`
	/// and `mark_dormant` ask identity-kyc, anything that seats an office asks for the NFT. The
	/// roll is written directly; an application, its referral and its approval would be three
	/// pallets' worth of setup for a status this only reads.
	fn enrol<T: Config>(who: &T::AccountId) {
		<T as Config>::BenchmarkHelper::make_citizen(who);
		pezpallet_identity_kyc::KycStatuses::<T>::insert(
			who,
			pezpallet_identity_kyc::types::KycLevel::Approved,
		);
		pezpallet_identity_kyc::CitizenSince::<T>::insert(
			who,
			pezframe_system::Pezpallet::<T>::block_number(),
		);
		pezpallet_identity_kyc::CitizenCount::<T>::mutate(|n| *n = n.saturating_add(1));
	}

	/// Seat `who` in a single-holder office the way the pallet does, NFT and all, so that
	/// unseating them later does the register's full work.
	fn seat<T: Config>(who: &T::AccountId, office: Tiki) {
		<T as Config>::BenchmarkHelper::make_citizen(who);
		Pezpallet::<T>::seat_unique_tiki(who, office).unwrap();
	}

	#[benchmark]
	fn set_emission_rate() {
		let treasurer: T::AccountId = whitelisted_caller();
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::Xezinedar, treasurer.clone());
		// A rate already on record, so the interval and the step are both checked: the first
		// call ever skips them.
		EmissionRate::<T>::put((pezsp_runtime::Perbill::default(), BlockNumberFor::<T>::default()));
		let now = T::MinEmissionInterval::get();
		pezframe_system::Pezpallet::<T>::set_block_number(now);
		let rate = T::MaxEmissionStep::get();
		<T as Config>::BenchmarkHelper::ensure_treasury_reachable();

		#[extrinsic_call]
		set_emission_rate(RawOrigin::Signed(treasurer), rate);

		assert_eq!(EmissionRate::<T>::get(), Some((rate, now)));
	}

	#[benchmark]
	fn reissue_citizenship() -> Result<(), BenchmarkError> {
		let origin =
			T::ReissueOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let from: T::AccountId = account("lost", 0, 0);
		let to: T::AccountId = account("found", 0, 0);
		let referrer: T::AccountId = account("referrer", 0, 0);
		let now = pezframe_system::Pezpallet::<T>::block_number();

		// Everything a reissue moves, present, so every `take` finds a row and every row is
		// written again under the new account.
		enrol::<T>(&from);
		let hash = pezsp_core::H256::repeat_byte(7);
		pezpallet_identity_kyc::IdentityHashes::<T>::insert(&from, hash);
		pezpallet_identity_kyc::IdentityHashToAccount::<T>::insert(hash, from.clone());
		pezpallet_identity_kyc::CitizenReferrers::<T>::insert(&from, referrer.clone());
		pezpallet_identity_kyc::ApprovedByFallback::<T>::insert(&from, ());
		pezpallet_identity_kyc::HonoraryCitizens::<T>::insert(&from, ());

		pezpallet_referral::InvitedBy::<T>::insert(&from, referrer.clone());
		pezpallet_referral::InvitationCount::<T>::insert(&from, 1);
		pezpallet_referral::ReferralCount::<T>::insert(&from, 1);
		pezpallet_referral::Referrals::<T>::insert(
			&from,
			pezpallet_referral::ReferralInfo::<T> { referrer: referrer.clone(), created_at: now },
		);
		pezpallet_referral::ReferrerStatsStorage::<T>::mutate(&from, |stats| {
			stats.total_referrals = 1
		});
		// One open claim, so the walk over them runs. The number of claims naming an account
		// is not bounded by anything this call can see.
		pezpallet_referral::Invitations::<T>::insert(&from, &referrer, ());

		pezpallet_trust::TrustScores::<T>::insert(
			&from,
			<T as pezpallet_trust::Config>::Score::try_from(1u128).unwrap_or_default(),
		);

		// As many tikis as an account may hold, each with its holder, provenance and term, so
		// the register's per-role move runs to its bound. Whatever the enum decodes to, in
		// order, rather than a list written here that would drift from it.
		let max_tikis = <T as pezpallet_tiki::Config>::MaxTikisPerUser::get();
		let mut held: Vec<Tiki> = Vec::new();
		for index in 0..=u8::MAX {
			if held.len() as u32 >= max_tikis {
				break;
			}
			if let Ok(tiki) = <Tiki as codec::Decode>::decode(&mut &[index][..]) {
				held.push(tiki);
			}
		}
		for tiki in &held {
			if pezpallet_tiki::Pezpallet::<T>::is_unique_role(tiki) {
				pezpallet_tiki::TikiHolder::<T>::insert(tiki, from.clone());
			}
			pezpallet_tiki::RoleAssignmentTypeOf::<T>::insert(
				&from,
				tiki,
				pezpallet_tiki::RoleAssignmentType::Appointed,
			);
			pezpallet_tiki::TikiExpiry::<T>::insert(&from, tiki, now + T::TermLength::get());
		}
		pezpallet_tiki::UserTikis::<T>::insert(
			&from,
			BoundedVec::<_, <T as pezpallet_tiki::Config>::MaxTikisPerUser>::try_from(held)
				.unwrap(),
		);

		// A seat on a full bench and in a full house, last in each, so both rewrites walk the
		// whole roll; and every record of this pallet's that follows the person.
		a_full_bench_with::<T>(&from);
		let house: Vec<ParliamentMember<T>> = (0..T::ParliamentSize::get())
			.map(|i| ParliamentMember {
				account: if i + 1 == T::ParliamentSize::get() {
					from.clone()
				} else {
					account("member", i, 0)
				},
				elected_at: now,
				term_ends_at: now + T::TermLength::get(),
				votes_participated: 0,
				total_votes_eligible: 0,
				participation_rate: 100,
				committees: Default::default(),
			})
			.collect();
		ParliamentMembers::<T>::put(BoundedVec::<_, T::ParliamentSize>::try_from(house).unwrap());
		CourtLastActive::<T>::insert(&from, now);
		OfficeLastActive::<T>::insert(Tiki::Serok, (from.clone(), now));
		OfficeLastActive::<T>::insert(Tiki::SerokiMeclise, (from.clone(), now));
		InitiativeCooldownUntil::<T>::insert(&from, now);
		LastSeenInGovernance::<T>::insert(&from, now);
		Dormant::<T>::insert(&from, ());
		AttestedRegion::<T>::insert(&from, Region::Bakur);
		ClaimedRegion::<T>::insert(&from, Region::Bakur);
		PendingPrimeMinister::<T>::put(from.clone());
		for election in [
			ElectionType::Presidential,
			ElectionType::Parliamentary,
			ElectionType::SpeakerElection,
			ElectionType::ConstitutionalCourt,
		] {
			ConsecutiveTerms::<T>::insert(election, &from, 1);
		}

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, from.clone(), to.clone());

		assert_eq!(pezpallet_identity_kyc::SupersededBy::<T>::get(&from), Some(to.clone()));
		assert!(Pezpallet::<T>::is_parliament_member(&to));
		assert!(Pezpallet::<T>::is_diwan_member(&to));
		Ok(())
	}

	#[benchmark]
	fn claim_region() {
		let who: T::AccountId = whitelisted_caller();
		enrol::<T>(&who);
		pezpallet_referral::ReferralCount::<T>::insert(&who, T::GeographicMarkReferrals::get());

		#[extrinsic_call]
		claim_region(RawOrigin::Signed(who.clone()), Region::Bakur);

		assert_eq!(ClaimedRegion::<T>::get(&who), Some(Region::Bakur));
	}

	#[benchmark]
	fn attest_region() {
		let notary: T::AccountId = whitelisted_caller();
		let who: T::AccountId = account("claimant", 0, 0);
		<T as Config>::BenchmarkHelper::make_citizen(&notary);
		pezpallet_tiki::Pezpallet::<T>::internal_grant_role(&notary, Tiki::Noter).unwrap();
		ClaimedRegion::<T>::insert(&who, Region::Bakur);

		#[extrinsic_call]
		attest_region(RawOrigin::Signed(notary), who.clone(), Region::Bakur);

		assert_eq!(AttestedRegion::<T>::get(&who), Some(Region::Bakur));
	}

	#[benchmark]
	fn withdraw_region() {
		let who: T::AccountId = whitelisted_caller();
		// Both marks, so both are taken.
		ClaimedRegion::<T>::insert(&who, Region::Bakur);
		AttestedRegion::<T>::insert(&who, Region::Rojava);

		#[extrinsic_call]
		withdraw_region(RawOrigin::Signed(who.clone()));

		assert!(ClaimedRegion::<T>::get(&who).is_none());
		assert!(AttestedRegion::<T>::get(&who).is_none());
	}

	#[benchmark]
	fn revoke_region() -> Result<(), BenchmarkError> {
		let origin = T::GeographicRevokeOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		let who: T::AccountId = account("attested", 0, 0);
		AttestedRegion::<T>::insert(&who, Region::Bakur);

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, who.clone());

		assert!(AttestedRegion::<T>::get(&who).is_none());
		Ok(())
	}

	#[benchmark]
	fn mark_dormant() {
		let caller: T::AccountId = whitelisted_caller();
		let who: T::AccountId = account("absent", 0, 0);
		enrol::<T>(&who);
		// Never seen taking part, so the date falls back to admission: the second read.
		let since = pezframe_system::Pezpallet::<T>::block_number();
		pezframe_system::Pezpallet::<T>::set_block_number(since + T::DormancyPeriod::get());

		#[extrinsic_call]
		mark_dormant(RawOrigin::Signed(caller), who.clone());

		assert!(Dormant::<T>::contains_key(&who));
	}

	#[benchmark]
	fn court_whitelists_on_the_relay() -> Result<(), BenchmarkError> {
		let origin =
			T::FastTrackOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		// Without it the send is refused and the benchmark measures the refusal.
		<T as Config>::BenchmarkHelper::ensure_relay_reachable();

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, T::Hash::default());

		Ok(())
	}

	#[benchmark]
	fn appoint_diwan_member() {
		let president: T::AccountId = whitelisted_caller();
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::Serok, president.clone());

		// One seat short of full, and that seat the President's last: the elected half
		// complete, every appointed seat but one taken.
		let elected = T::DiwanElectedSeats::get();
		let seats = T::DiwanSize::get().saturating_sub(1);
		let bench: Vec<DiwanMember<T>> = (0..seats)
			.map(|i| DiwanMember {
				account: account("judge", i, 0),
				appointed_at: Zero::zero(),
				term_ends_at: T::CourtTermLength::get(),
				appointed_by: if i < elected {
					AppointmentAuthority::Parliament
				} else {
					AppointmentAuthority::President(president.clone())
				},
			})
			.collect();
		DiwanMembers::<T>::put(BoundedVec::<_, T::DiwanSize>::try_from(bench).unwrap());

		// Qualified by the last tiki on the list, so the qualification check asks about every
		// one before it.
		let appointee: T::AccountId = account("appointee", 0, 0);
		<T as Config>::BenchmarkHelper::make_citizen(&appointee);
		pezpallet_tiki::Pezpallet::<T>::internal_grant_role(&appointee, Tiki::ParêzvaneÇandî)
			.unwrap();

		#[extrinsic_call]
		appoint_diwan_member(RawOrigin::Signed(president), appointee.clone());

		assert!(Pezpallet::<T>::is_diwan_member(&appointee));
		assert_eq!(DiwanMembers::<T>::get().len() as u32, T::DiwanSize::get());
	}

	/// Seating `m` founding members.
	///
	/// Linear in the list: it is decoded, written into the house, queued for seating and
	/// relayed to the collective, each in full. The call is charged for the list it is given,
	/// so the component runs over every size the house can have.
	#[benchmark]
	fn seat_founding_parliament(m: Linear<1, { T::ParliamentSize::get() }>) {
		let president: T::AccountId = whitelisted_caller();
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::Serok, president.clone());
		let members: Vec<T::AccountId> = (0..m).map(|i| account("founder", i, 0)).collect();

		#[extrinsic_call]
		seat_founding_parliament(RawOrigin::Signed(president), members);

		assert_eq!(ParliamentMembers::<T>::get().len() as u32, m);
		assert_eq!(PendingSeatGrants::<T>::get().len() as u32, m);
	}

	#[benchmark]
	fn appoint_prime_minister() {
		let president: T::AccountId = whitelisted_caller();
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::Serok, president.clone());
		let nominee: T::AccountId = account("nominee", 0, 0);

		#[extrinsic_call]
		appoint_prime_minister(RawOrigin::Signed(president), nominee.clone());

		assert_eq!(PendingPrimeMinister::<T>::get(), Some(nominee));
	}

	#[benchmark]
	fn confirm_prime_minister() -> Result<(), BenchmarkError> {
		let origin = T::ConfirmationOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		// An incumbent, so seating the nominee unseats somebody first: two register writes,
		// not one.
		let incumbent: T::AccountId = account("incumbent", 0, 0);
		seat::<T>(&incumbent, Tiki::SerokWeziran);
		let nominee: T::AccountId = account("nominee", 0, 0);
		<T as Config>::BenchmarkHelper::make_citizen(&nominee);
		PendingPrimeMinister::<T>::put(nominee.clone());

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin);

		assert_eq!(pezpallet_tiki::TikiHolder::<T>::get(Tiki::SerokWeziran), Some(nominee));
		Ok(())
	}

	#[benchmark]
	fn reject_prime_minister() -> Result<(), BenchmarkError> {
		let origin = T::ConfirmationOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		PendingPrimeMinister::<T>::put(account::<T::AccountId>("nominee", 0, 0));

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin);

		assert!(PendingPrimeMinister::<T>::get().is_none());
		Ok(())
	}

	#[benchmark]
	fn dismiss_prime_minister() {
		let president: T::AccountId = whitelisted_caller();
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::Serok, president.clone());
		let prime_minister: T::AccountId = account("prime_minister", 0, 0);
		seat::<T>(&prime_minister, Tiki::SerokWeziran);

		#[extrinsic_call]
		dismiss_prime_minister(RawOrigin::Signed(president));

		assert!(pezpallet_tiki::TikiHolder::<T>::get(Tiki::SerokWeziran).is_none());
	}

	#[benchmark]
	fn appoint_minister() {
		let prime_minister: T::AccountId = whitelisted_caller();
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::SerokWeziran, prime_minister.clone());
		// A single-holder portfolio with somebody in it: the dearer of the two arms, which
		// unseats the incumbent before seating the appointee.
		let incumbent: T::AccountId = account("incumbent", 0, 0);
		seat::<T>(&incumbent, Tiki::WezireDarayiye);
		let minister: T::AccountId = account("minister", 0, 0);
		<T as Config>::BenchmarkHelper::make_citizen(&minister);

		#[extrinsic_call]
		appoint_minister(RawOrigin::Signed(prime_minister), minister.clone(), Tiki::WezireDarayiye);

		assert_eq!(pezpallet_tiki::TikiHolder::<T>::get(Tiki::WezireDarayiye), Some(minister));
	}

	#[benchmark]
	fn dismiss_minister() {
		let prime_minister: T::AccountId = whitelisted_caller();
		pezpallet_tiki::TikiHolder::<T>::insert(Tiki::SerokWeziran, prime_minister.clone());
		// A single-holder portfolio, which also clears the holder record.
		let minister: T::AccountId = account("minister", 0, 0);
		seat::<T>(&minister, Tiki::WezireDarayiye);

		#[extrinsic_call]
		dismiss_minister(RawOrigin::Signed(prime_minister), minister.clone(), Tiki::WezireDarayiye);

		assert!(pezpallet_tiki::TikiHolder::<T>::get(Tiki::WezireDarayiye).is_none());
	}

	#[benchmark]
	fn set_confirmation_requirement() -> Result<(), BenchmarkError> {
		let origin = T::ConfirmationOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		// Away from the founding rule, so the entry is written rather than cleared.
		let role = OfficialRole::Dadger;
		let required = !role.requires_parliament_approval();

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, role, required);

		assert_eq!(ConfirmationRequired::<T>::get(role), Some(required));
		Ok(())
	}

	impl_benchmark_test_suite!(
		Pezpallet,
		crate::mock::ExtBuilder::default().build(),
		crate::mock::Test
	);
}
