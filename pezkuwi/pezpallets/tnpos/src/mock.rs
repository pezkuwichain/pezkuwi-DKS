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

#[allow(dead_code)]
fn put_score(who: AccountId, kind: u8, value: u128, at: BlockNumber) {
	SCORES.with(|s| s.borrow_mut().insert((who, kind), (value, at)));
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

/// Phase 1's real `Sortition` arrives in a later task. Until then the mock draws from a
/// fixed per-era seed, which is all the genesis and configuration tests here need.
pub struct StubSortition;
impl pezkuwi_tnpos_primitives::sortition::Sortition<AccountId> for StubSortition {
	fn select(
		era: u32,
		stratum: StratumId,
		candidates: &[AccountId],
		k: u32,
	) -> Option<Vec<AccountId>> {
		let mut seed = [0u8; 32];
		seed[..4].copy_from_slice(&era.to_le_bytes());
		Some(pezkuwi_tnpos_primitives::sortition::sample_k(candidates, k, &seed, &[stratum as u8]))
	}
}

impl pezpallet_tnpos::Config for Test {
	type WeightInfo = ();
	type Sortition = StubSortition;
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
