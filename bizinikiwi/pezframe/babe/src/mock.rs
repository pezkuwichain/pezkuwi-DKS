// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test utilities

use crate::{self as pezpallet_babe, Config, CurrentSlot};
use codec::Encode;
use pezframe_election_provider_support::{
	bounds::{ElectionBounds, ElectionBoundsBuilder},
	onchain, SequentialPhragmen,
};
use pezframe_support::{
	derive_impl, parameter_types,
	traits::{ConstU128, ConstU32, ConstU64, OnInitialize},
};
use pezpallet_session::historical as pezpallet_session_historical;
use pezsp_consensus_babe::{AuthorityId, AuthorityPair, Randomness, Slot, VrfSignature};
use pezsp_core::{
	crypto::{Pair, VrfSecret},
	ConstBool, U256,
};
use pezsp_io;
use pezsp_runtime::{
	curve::PiecewiseLinear,
	impl_opaque_keys,
	testing::{Digest, DigestItem, Header, TestXt},
	traits::{Header as _, OpaqueKeys},
	BuildStorage, DispatchError, Perbill,
};
use pezsp_staking::{EraIndex, SessionIndex};

type DummyValidatorId = u64;

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Authorship: pezpallet_authorship,
		Balances: pezpallet_balances,
		Historical: pezpallet_session_historical,
		Offences: pezpallet_offences,
		Babe: pezpallet_babe,
		Staking: pezpallet_staking,
		Session: pezpallet_session,
		Timestamp: pezpallet_timestamp,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u128>;
}

impl<C> pezframe_system::offchain::CreateTransactionBase<C> for Test
where
	RuntimeCall: From<C>,
{
	type RuntimeCall = RuntimeCall;
	type Extrinsic = TestXt<RuntimeCall, ()>;
}

impl<C> pezframe_system::offchain::CreateBare<C> for Test
where
	RuntimeCall: From<C>,
{
	fn create_bare(call: Self::RuntimeCall) -> Self::Extrinsic {
		TestXt::new_bare(call)
	}
}

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub babe_authority: super::Pezpallet<Test>,
	}
}

impl pezpallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as pezframe_system::Config>::AccountId;
	type ValidatorIdOf = pezsp_runtime::traits::ConvertInto;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pezpallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <MockSessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = MockSessionKeys;
	type DisablingStrategy = ();
	type WeightInfo = ();
	type Currency = Balances;
	type KeyDeposit = ();
}

impl pezpallet_session::historical::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type FullIdentification = ();
	type FullIdentificationOf = pezpallet_staking::UnitIdentificationOf<Self>;
}

impl pezpallet_authorship::Config for Test {
	type FindAuthor = pezpallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = ();
}

impl pezpallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

type Balance = u128;
#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
}

pezpallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000u64,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub const SessionsPerEra: SessionIndex = 3;
	pub const BondingDuration: EraIndex = 3;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub static ElectionsBounds: ElectionBounds = ElectionBoundsBuilder::default().build();
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Test;
	type Solver = SequentialPhragmen<DummyValidatorId, Perbill>;
	type DataProvider = Staking;
	type WeightInfo = ();
	type MaxWinnersPerPage = ConstU32<100>;
	type MaxBackersPerWinner = ConstU32<100>;
	type Sort = ConstBool<true>;
	type Bounds = ElectionsBounds;
}

#[derive_impl(pezpallet_staking::config_preludes::TestDefaultConfig)]
impl pezpallet_staking::Config for Test {
	type OldCurrency = Balances;
	type Currency = Balances;
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type AdminOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type SessionInterface = Self;
	type UnixTime = pezpallet_timestamp::Pezpallet<Test>;
	type EraPayout = pezpallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GenesisElectionProvider = Self::ElectionProvider;
	type VoterList = pezpallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type TargetList = pezpallet_staking::UseValidatorsMap<Self>;
}

impl pezpallet_offences::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pezpallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

parameter_types! {
	pub const EpochDuration: u64 = 3;
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl Config for Test {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ConstU64<1>;
	type EpochChangeTrigger = crate::ExternalTrigger;
	type DisabledValidators = Session;
	type WeightInfo = ();
	type MaxAuthorities = ConstU32<10>;
	type MaxNominators = ConstU32<100>;
	type KeyOwnerProof = pezsp_session::MembershipProof;
	type EquivocationReportSystem =
		super::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

pub fn go_to_block(n: u64, s: u64) {
	use pezframe_support::traits::OnFinalize;

	Babe::on_finalize(System::block_number());
	Session::on_finalize(System::block_number());
	Staking::on_finalize(System::block_number());

	let parent_hash = if System::block_number() > 1 {
		let hdr = System::finalize();
		hdr.hash()
	} else {
		System::parent_hash()
	};

	let pre_digest = make_secondary_plain_pre_digest(0, s.into());

	System::reset_events();
	System::initialize(&n, &parent_hash, &pre_digest);

	Babe::on_initialize(n);
	Session::on_initialize(n);
	Staking::on_initialize(n);
}

/// Slots will grow accordingly to blocks
pub fn progress_to_block(n: u64) {
	let mut slot = u64::from(CurrentSlot::<Test>::get()) + 1;
	for i in System::block_number() + 1..=n {
		go_to_block(i, slot);
		slot += 1;
	}
}

/// Progress to the first block at the given session
pub fn start_session(session_index: SessionIndex) {
	let missing = (session_index - Session::current_index()) * 3;
	progress_to_block(System::block_number() + missing as u64 + 1);
	assert_eq!(Session::current_index(), session_index);
}

/// Progress to the first block at the given era
pub fn start_era(era_index: EraIndex) {
	start_session((era_index * 3).into());
	assert_eq!(pezpallet_staking::CurrentEra::<Test>::get(), Some(era_index));
}

pub fn make_primary_pre_digest(
	authority_index: pezsp_consensus_babe::AuthorityIndex,
	slot: pezsp_consensus_babe::Slot,
	vrf_signature: VrfSignature,
) -> Digest {
	let digest_data = pezsp_consensus_babe::digests::PreDigest::Primary(
		pezsp_consensus_babe::digests::PrimaryPreDigest { authority_index, slot, vrf_signature },
	);
	let log = DigestItem::PreRuntime(pezsp_consensus_babe::BABE_ENGINE_ID, digest_data.encode());
	Digest { logs: vec![log] }
}

pub fn make_secondary_plain_pre_digest(
	authority_index: pezsp_consensus_babe::AuthorityIndex,
	slot: pezsp_consensus_babe::Slot,
) -> Digest {
	let digest_data = pezsp_consensus_babe::digests::PreDigest::SecondaryPlain(
		pezsp_consensus_babe::digests::SecondaryPlainPreDigest { authority_index, slot },
	);
	let log = DigestItem::PreRuntime(pezsp_consensus_babe::BABE_ENGINE_ID, digest_data.encode());
	Digest { logs: vec![log] }
}

pub fn make_secondary_vrf_pre_digest(
	authority_index: pezsp_consensus_babe::AuthorityIndex,
	slot: pezsp_consensus_babe::Slot,
	vrf_signature: VrfSignature,
) -> Digest {
	let digest_data = pezsp_consensus_babe::digests::PreDigest::SecondaryVRF(
		pezsp_consensus_babe::digests::SecondaryVRFPreDigest {
			authority_index,
			slot,
			vrf_signature,
		},
	);
	let log = DigestItem::PreRuntime(pezsp_consensus_babe::BABE_ENGINE_ID, digest_data.encode());
	Digest { logs: vec![log] }
}

pub fn make_vrf_signature_and_randomness(
	slot: Slot,
	pair: &pezsp_consensus_babe::AuthorityPair,
) -> (VrfSignature, Randomness) {
	let transcript = pezsp_consensus_babe::make_vrf_transcript(
		&pezpallet_babe::Randomness::<Test>::get(),
		slot,
		0,
	);

	let randomness = pair
		.as_ref()
		.make_bytes(pezsp_consensus_babe::RANDOMNESS_VRF_CONTEXT, &transcript);

	let signature = pair.as_ref().vrf_sign(&transcript.into());

	(signature, randomness)
}

pub fn new_test_ext(authorities_len: usize) -> pezsp_io::TestExternalities {
	new_test_ext_with_pairs(authorities_len).1
}

pub fn new_test_ext_with_pairs(
	authorities_len: usize,
) -> (Vec<AuthorityPair>, pezsp_io::TestExternalities) {
	let pairs = (0..authorities_len)
		.map(|i| AuthorityPair::from_seed(&U256::from(i).to_little_endian()))
		.collect::<Vec<_>>();

	let public = pairs.iter().map(|p| p.public()).collect();

	(pairs, new_test_ext_raw_authorities(public))
}

pub fn new_test_ext_raw_authorities(authorities: Vec<AuthorityId>) -> pezsp_io::TestExternalities {
	pezsp_tracing::try_init_simple();
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let balances: Vec<_> = (0..authorities.len()).map(|i| (i as u64, 10_000_000)).collect();

	pezpallet_balances::GenesisConfig::<Test> { balances, ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	// stashes are the index.
	let session_keys: Vec<_> = authorities
		.iter()
		.enumerate()
		.map(|(i, k)| {
			(i as u64, i as u64, MockSessionKeys { babe_authority: AuthorityId::from(k.clone()) })
		})
		.collect();

	// NOTE: this will initialize the babe authorities
	// through OneSessionHandler::on_genesis_session
	pezpallet_session::GenesisConfig::<Test> { keys: session_keys, ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	// controllers are same as stash
	let stakers: Vec<_> = (0..authorities.len())
		.map(|i| (i as u64, i as u64, 10_000, pezpallet_staking::StakerStatus::<u64>::Validator))
		.collect();

	let staking_config = pezpallet_staking::GenesisConfig::<Test> {
		stakers,
		validator_count: 8,
		force_era: pezpallet_staking::Forcing::ForceNew,
		minimum_validator_count: 0,
		invulnerables: vec![],
		..Default::default()
	};

	staking_config.assimilate_storage(&mut t).unwrap();

	t.into()
}

/// Creates an equivocation at the current block, by generating two headers.
pub fn generate_equivocation_proof(
	offender_authority_index: u32,
	offender_authority_pair: &AuthorityPair,
	slot: Slot,
) -> pezsp_consensus_babe::EquivocationProof<Header> {
	use pezsp_consensus_babe::digests::CompatibleDigestItem;

	let current_block = System::block_number();
	let current_slot = CurrentSlot::<Test>::get();

	let make_header = || {
		// We don't want to change any state, so we build the headers in a transaction and revert it
		// afterward.
		pezframe_support::storage::with_transaction(|| {
			let parent_hash = System::parent_hash();
			let pre_digest = make_secondary_plain_pre_digest(offender_authority_index, slot);
			System::reset_events();
			System::set_block_number(System::block_number() - 1);
			System::initialize(&current_block, &parent_hash, &pre_digest);
			System::set_block_number(current_block);
			Timestamp::set_timestamp(*current_slot * Babe::slot_duration());
			let header = System::finalize();

			pezsp_runtime::TransactionOutcome::Rollback(Ok::<_, DispatchError>(header))
		})
		.unwrap()
	};

	// Sign the header prehash and sign it, adding it to the block as the seal
	// digest item
	let seal_header = |header: &mut Header| {
		let prehash = header.hash();
		let seal = <DigestItem as CompatibleDigestItem>::babe_seal(
			offender_authority_pair.sign(prehash.as_ref()),
		);
		header.digest_mut().push(seal);
	};

	// Generate two headers at the current block
	let mut h1 = make_header();
	let mut h2 = make_header();

	seal_header(&mut h1);
	seal_header(&mut h2);

	pezsp_consensus_babe::EquivocationProof {
		slot,
		offender: offender_authority_pair.public(),
		first_header: h1,
		second_header: h2,
	}
}
