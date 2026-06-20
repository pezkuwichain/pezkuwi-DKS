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

//! Mock file for session benchmarking.

#![cfg(test)]

use pezframe_election_provider_support::{
	bounds::{ElectionBounds, ElectionBoundsBuilder},
	onchain, SequentialPhragmen,
};
use pezframe_support::{
	derive_impl, parameter_types,
	traits::{ConstU32, ConstU64},
};
use pezsp_runtime::{traits::IdentityLookup, BuildStorage, KeyTypeId};

type AccountId = u64;
type Nonce = u32;

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		Staking: pezpallet_staking,
		Session: pezpallet_session,
		Historical: pezpallet_session::historical
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Nonce = Nonce;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u64>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type ExistentialDeposit = ConstU64<10>;
	type AccountStore = System;
}

impl pezpallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}
impl pezpallet_session::historical::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type FullIdentification = ();
	type FullIdentificationOf = pezpallet_staking::UnitIdentificationOf<Self>;
}

pezsp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub foo: pezsp_runtime::testing::UintAuthorityId,
	}
}

pub struct TestSessionHandler;
impl pezpallet_session::SessionHandler<AccountId> for TestSessionHandler {
	// corresponds to the opaque key id above
	const KEY_TYPE_IDS: &'static [KeyTypeId] = &[KeyTypeId([100u8, 117u8, 109u8, 121u8])];

	fn on_genesis_session<Ks: pezsp_runtime::traits::OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

	fn on_new_session<Ks: pezsp_runtime::traits::OpaqueKeys>(
		_: bool,
		_: &[(AccountId, Ks)],
		_: &[(AccountId, Ks)],
	) {
	}

	fn on_disabled(_: u32) {}
}

impl pezpallet_session::Config for Test {
	type SessionManager = pezpallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type Keys = SessionKeys;
	type ShouldEndSession = pezpallet_session::PeriodicSessions<(), ()>;
	type NextSessionRotation = pezpallet_session::PeriodicSessions<(), ()>;
	type SessionHandler = TestSessionHandler;
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = pezsp_runtime::traits::ConvertInto;
	type DisablingStrategy = ();
	type WeightInfo = ();
	type Currency = Balances;
	// Note: setting to a large amount to ensure bench setup can handle increasing the balance of
	// the validator before setting session keys; see `ensure_can_pay_key_deposit`.
	type KeyDeposit = ConstU64<2000000000>;
}
pezpallet_staking_reward_curve::build! {
	const I_NPOS: pezsp_runtime::curve::PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}
parameter_types! {
	pub const RewardCurve: &'static pezsp_runtime::curve::PiecewiseLinear<'static> = &I_NPOS;
	pub static ElectionsBounds: ElectionBounds = ElectionBoundsBuilder::default().build();
	pub const Sort: bool = true;
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Test;
	type Solver = SequentialPhragmen<AccountId, pezsp_runtime::Perbill>;
	type DataProvider = Staking;
	type WeightInfo = ();
	type MaxWinnersPerPage = ConstU32<100>;
	type MaxBackersPerWinner = ConstU32<100>;
	type Sort = Sort;
	type Bounds = ElectionsBounds;
}

#[derive_impl(pezpallet_staking::config_preludes::TestDefaultConfig)]
impl pezpallet_staking::Config for Test {
	type OldCurrency = Balances;
	type Currency = Balances;
	type CurrencyBalance = <Self as pezpallet_balances::Config>::Balance;
	type UnixTime = pezpallet_timestamp::Pezpallet<Self>;
	type AdminOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type SessionInterface = Self;
	type EraPayout = pezpallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GenesisElectionProvider = Self::ElectionProvider;
	type VoterList = pezpallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type TargetList = pezpallet_staking::UseValidatorsMap<Self>;
}

impl crate::Config for Test {}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pezsp_io::TestExternalities::new(t)
}
