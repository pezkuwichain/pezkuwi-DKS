// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Test runtime for pezpallet-tnpos.

use crate as pezpallet_tnpos;
use core::cell::RefCell;
use pezframe_support::{construct_runtime, derive_impl, parameter_types};
use pezkuwi_tnpos_primitives::{scores::ScoreSnapshot, StratumConfig, StratumId};
use pezsp_runtime::BuildStorage;
use std::collections::BTreeMap;

pub type AccountId = u64;
pub type BlockNumber = u64;

#[allow(dead_code)]
pub const ALICE: AccountId = 1;
#[allow(dead_code)]
pub const BOB: AccountId = 2;

construct_runtime!(
	pub enum Test {
		System: pezframe_system,
		Tnpos: pezpallet_tnpos,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = pezframe_system::mocking::MockBlock<Test>;
	type AccountId = AccountId;
	type Lookup = pezsp_runtime::traits::IdentityLookup<AccountId>;
}

parameter_types! {
	pub const MaxScoreAge: BlockNumber = 100;
	pub const EraLength: BlockNumber = 50;
	pub const MaxPoolSize: u32 = 2_000;
}

// Scores are set directly by tests. Nothing here reaches another chain: the real source is
// XCM from the People chain and is M7.1, which is why the runtime still runs on stubs.
thread_local! {
	static SCORES: RefCell<BTreeMap<(AccountId, u8), (u128, BlockNumber)>> =
		RefCell::new(BTreeMap::new());
}

const TRUST: u8 = 0;
const TIKI: u8 = 1;
const PERWERDE: u8 = 2;
const REFERRAL: u8 = 3;
const STAKING: u8 = 4;

fn put_score(who: AccountId, kind: u8, value: u128, at: BlockNumber) {
	SCORES.with(|s| s.borrow_mut().insert((who, kind), (value, at)));
}

pub fn set_perwerde(who: AccountId, v: u128) {
	put_score(who, PERWERDE, v, System::block_number());
}

pub fn set_perwerde_at(who: AccountId, v: u128, at: BlockNumber) {
	put_score(who, PERWERDE, v, at);
}

/// Put `who` in the pool. Contributing to the seed requires membership, so any test that
/// exercises the seed needs this first.
pub fn join_pool(who: AccountId) {
	put_score(who, PERWERDE, 1_000, System::block_number());
	assert!(Tnpos::join(RuntimeOrigin::signed(who), StratumId::Perwerde).is_ok());
}

pub fn set_tiki(who: AccountId, v: u128) {
	put_score(who, TIKI, v, System::block_number());
}

/// An account holding office tikis and nothing else.
///
/// `tiki_of` excludes office tikis, so it reads zero. That exclusion is what keeps the Tiki
/// stratum independent of Meclis; without it the two gates would collapse into one and the
/// security arithmetic would be describing a chain that does not exist.
pub fn set_office_tiki_only(who: AccountId) {
	put_score(who, TIKI, 0, System::block_number());
	put_score(who, TRUST, 1_000, System::block_number());
}

/// Advance the block number, running `on_initialize` at every step.
pub fn run_to_block(n: BlockNumber) {
	use pezframe_support::traits::OnInitialize;
	while System::block_number() < n {
		let next = System::block_number() + 1;
		System::set_block_number(next);
		Tnpos::on_initialize(next);
	}
}

fn read_score(who: &AccountId, kind: u8) -> ScoreSnapshot<BlockNumber> {
	SCORES
		.with(|s| s.borrow().get(&(*who, kind)).copied())
		.map(|(value, last_updated)| ScoreSnapshot { value, last_updated })
		.unwrap_or(ScoreSnapshot { value: 0, last_updated: System::block_number() })
}

pub struct MockScores;
impl pezkuwi_tnpos_primitives::scores::ScoreProvider<AccountId, BlockNumber> for MockScores {
	fn trust_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, TRUST)
	}
	fn tiki_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, TIKI)
	}
	fn perwerde_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, PERWERDE)
	}
	fn referral_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, REFERRAL)
	}
	fn staking_of(who: &AccountId) -> ScoreSnapshot<BlockNumber> {
		read_score(who, STAKING)
	}
}

impl pezpallet_tnpos::Config for Test {
	type WeightInfo = ();
	type Sortition = crate::seed::CommitRevealSortition<Test>;
	type Scores = MockScores;
	type ManagerOrigin = pezframe_system::EnsureRoot<AccountId>;
	type MaxScoreAge = MaxScoreAge;
	type EraLength = EraLength;
	type MaxPoolSize = MaxPoolSize;
}

/// The nine strata at their specified sizes, with a floor small enough for tests.
pub fn nine_strata() -> Vec<StratumConfig> {
	StratumId::ALL
		.iter()
		.map(|&id| StratumConfig { id, seats: 3, min_eligible: 5 })
		.collect()
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	new_test_ext_with_strata(9)
}

/// Build genesis with the first `n` strata. Fewer than five must panic in `build`.
pub fn new_test_ext_with_strata(n: usize) -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pezpallet_tnpos::GenesisConfig::<Test> {
		strata: nine_strata().into_iter().take(n).collect(),
		members: Vec::new(),
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Put `per` eligible members into every stratum.
pub fn fill_every_stratum(per: u32) {
	let mut who: AccountId = 100;
	for &s in StratumId::ALL.iter() {
		for _ in 0..per {
			for kind in [TRUST, TIKI, PERWERDE, STAKING] {
				put_score(who, kind, 1_000, System::block_number());
			}
			assert!(Tnpos::join(RuntimeOrigin::signed(who), s).is_ok());
			who += 1;
		}
	}
	seed_the_era();
}

/// Move directly to the reveal half of the current round, without triggering
/// `on_initialize` -- so this can set up a round without also seating one.
pub fn advance_to_reveal_window() {
	let midpoint = pezpallet_tnpos::EraStart::<Test>::get() + EraLength::get() / 2;
	while System::block_number() < midpoint {
		System::set_block_number(System::block_number() + 1);
	}
}

/// Run one commit-reveal round: commit inside the era's first half, then advance past the
/// midpoint and reveal.
pub fn seed_the_era() {
	let who = pezpallet_tnpos::PoolMembers::<Test>::iter()
		.next()
		.map(|(w, _)| w)
		.unwrap_or(ALICE);
	let pre = [42u8; 32];
	assert!(
		Tnpos::commit_seed(RuntimeOrigin::signed(who), pezsp_io::hashing::blake2_256(&pre)).is_ok()
	);
	advance_to_reveal_window();
	assert!(Tnpos::reveal_seed(RuntimeOrigin::signed(who), pre).is_ok());
}

pub fn clear_seed() {
	pezpallet_tnpos::NextSeed::<Test>::kill();
}

pub fn advance_eras(n: u32) {
	pezpallet_tnpos::CurrentEra::<Test>::mutate(|e| *e = e.saturating_add(n));
}

pub fn empty_stratum(s: StratumId) {
	let members: Vec<AccountId> = pezpallet_tnpos::PoolMembers::<Test>::iter()
		.filter_map(|(w, st)| (st == s).then_some(w))
		.collect();
	for w in members {
		assert!(Tnpos::leave(RuntimeOrigin::signed(w)).is_ok());
	}
}
