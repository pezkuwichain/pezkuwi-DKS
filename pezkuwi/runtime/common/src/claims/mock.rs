// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! Mocking utilities for testing in claims pezpallet.

#[cfg(test)]
use super::*;
use secp_utils::*;

// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate::claims;
use pezframe_support::{
	derive_impl, ord_parameter_types, parameter_types, traits::WithdrawReasons,
};
use pezpallet_balances;
use pezsp_runtime::{traits::Identity, BuildStorage};

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		Vesting: pezpallet_vesting,
		Claims: claims,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type AccountData = pezpallet_balances::AccountData<u64>;
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 1;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pezpallet_vesting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = Identity;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
	pub Prefix: &'static [u8] = b"Pay RUSTs to the TEST account:";
}
ord_parameter_types! {
	pub const Six: u64 = 6;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type Prefix = Prefix;
	type MoveClaimOrigin = pezframe_system::EnsureSignedBy<Six, u64>;
	type WeightInfo = TestWeightInfo;
}

pub fn alice() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Alice")).unwrap()
}
pub fn bob() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Bob")).unwrap()
}
pub fn dave() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Dave")).unwrap()
}
pub fn eve() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Eve")).unwrap()
}
pub fn frank() -> libsecp256k1::SecretKey {
	libsecp256k1::SecretKey::parse(&keccak_256(b"Frank")).unwrap()
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	// We use default for brevity, but you can configure as desired if needed.
	pezpallet_balances::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	claims::GenesisConfig::<Test> {
		claims: vec![
			(eth(&alice()), 100, None, None),
			(eth(&dave()), 200, None, Some(StatementKind::Regular)),
			(eth(&eve()), 300, Some(42), Some(StatementKind::Saft)),
			(eth(&frank()), 400, Some(43), None),
		],
		vesting: vec![(eth(&alice()), (50, 10, 1))],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

pub fn total_claims() -> u64 {
	100 + 200 + 300 + 400
}
