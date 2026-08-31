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

use crate::VoterBagsListInstance;
use pezframe_election_provider_support::VoteWeight;
use pezframe_support::{
	derive_impl, parameter_types,
	pezpallet_prelude::*,
	traits::{ConstBool, Nothing, VariantCountOf},
	PalletId,
};
use pezsp_runtime::{
	traits::{BlockNumberProvider, Convert, IdentityLookup},
	BuildStorage, FixedU128, Perbill,
};

type AccountId = u128;
type BlockNumber = u64;
type Balance = u128;

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<Balance>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 10;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Runtime {
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	pub static BondingDuration: u32 = 3;
}

/// A mock `RcClientInterface` for benchmarks that don't need session/validator-set management.
pub struct MockRcClient;
impl pezpallet_staking_async_rc_client::RcClientInterface for MockRcClient {
	type AccountId = AccountId;

	fn validator_set(
		_new_validator_set: Vec<Self::AccountId>,
		_id: u32,
		_prune_up_to: Option<u32>,
	) {
	}
}

#[derive_impl(pezpallet_staking_async::config_preludes::TestDefaultConfig)]
impl pezpallet_staking_async::Config for Runtime {
	type OldCurrency = Balances;
	type Currency = Balances;
	type AdminOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type EraPayout = ();
	type DisableMinting = ConstBool<true>;
	type BondingDuration = BondingDuration;
	type RewardPots = pezpallet_staking_async::SequentialTest;
	type ElectionProvider =
		pezframe_election_provider_support::NoElection<(AccountId, BlockNumber, Staking, (), ())>;
	type VoterList = VoterList;
	type TargetList = pezpallet_staking_async::UseValidatorsMap<Self>;
	type EventListeners = (Pools, DelegatedStaking);
	type RcClientInterface = MockRcClient;
}

parameter_types! {
	pub static BagThresholds: &'static [VoteWeight] = &[10, 20, 30, 40, 50, 60, 1_000, 2_000, 10_000];
}

impl pezpallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type BagThresholds = BagThresholds;
	type ScoreProvider = Staking;
	type Score = VoteWeight;
	type MaxAutoRebagPerBlock = ();
}

pub struct BalanceToU256;
impl Convert<Balance, pezsp_core::U256> for BalanceToU256 {
	fn convert(n: Balance) -> pezsp_core::U256 {
		n.into()
	}
}

pub struct U256ToBalance;
impl Convert<pezsp_core::U256, Balance> for U256ToBalance {
	fn convert(n: pezsp_core::U256) -> Balance {
		n.try_into().unwrap()
	}
}

/// Always reports block 0 so commission `throttle_from` is deterministic.
/// While benchmarking on AH, nom-pools `BlockNumberProvider` will be `RelaychainDataProvider`.
pub struct BenchmarkBlockNumberProvider;
impl BlockNumberProvider for BenchmarkBlockNumberProvider {
	type BlockNumber = BlockNumber;
	fn current_block_number() -> Self::BlockNumber {
		0
	}
}

parameter_types! {
	pub static PostUnbondingPoolsWindow: u32 = 10;
	pub const PoolsPalletId: PalletId = PalletId(*b"py/nopls");
	pub const MaxPointsToBalance: u8 = 10;
}

impl pezpallet_nomination_pools::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = BalanceToU256;
	type U256ToBalance = U256ToBalance;
	type StakeAdapter =
		pezpallet_nomination_pools::adapter::DelegateStake<Self, Staking, DelegatedStaking>;
	type PostUnbondingPoolsWindow = PostUnbondingPoolsWindow;
	type MaxMetadataLen = ConstU32<256>;
	type MaxUnbonding = ConstU32<8>;
	type PalletId = PoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type AdminOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type BlockNumberProvider = BenchmarkBlockNumberProvider;
	type Filter = Nothing;
}

parameter_types! {
	pub const DelegatedStakingPalletId: PalletId = PalletId(*b"py/dlstk");
	pub const SlashRewardFraction: Perbill = Perbill::from_percent(1);
}
impl pezpallet_delegated_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = DelegatedStakingPalletId;
	type Currency = Balances;
	type OnSlash = ();
	type SlashRewardFraction = SlashRewardFraction;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CoreStaking = Staking;
}

impl crate::Config for Runtime {}

type Block = pezframe_system::mocking::MockBlock<Runtime>;

pezframe_support::construct_runtime!(
	pub enum Runtime {
		System: pezframe_system,
		Balances: pezpallet_balances,
		Staking: pezpallet_staking_async,
		VoterList: pezpallet_bags_list::<Instance1>,
		Pools: pezpallet_nomination_pools,
		DelegatedStaking: pezpallet_delegated_staking,
	}
);

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut storage = pezframe_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	let _ = pezpallet_nomination_pools::GenesisConfig::<Runtime> {
		min_join_bond: 2,
		min_create_bond: 2,
		max_pools: Some(3),
		max_members_per_pool: Some(3),
		max_members: Some(3 * 3),
		global_max_commission: Some(Perbill::from_percent(50)),
	}
	.assimilate_storage(&mut storage);
	pezsp_io::TestExternalities::from(storage)
}
