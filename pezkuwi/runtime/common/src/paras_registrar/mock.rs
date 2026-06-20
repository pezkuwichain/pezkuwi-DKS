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

//! Mocking utilities for testing in paras_registrar pezpallet.

#[cfg(test)]
use super::*;
use crate::paras_registrar;
use alloc::collections::btree_map::BTreeMap;
use pezframe_support::{derive_impl, parameter_types};
use pezframe_system::limits;
use pezkuwi_primitives::{Balance, BlockNumber, MAX_CODE_SIZE};
use pezkuwi_runtime_teyrchains::{configuration, origin, shared};
use pezsp_core::{ConstUint, H256};
use pezsp_io::TestExternalities;
use pezsp_keyring::Sr25519Keyring;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	transaction_validity::TransactionPriority,
	BuildStorage, Perbill,
};

type UncheckedExtrinsic = pezframe_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = pezframe_system::mocking::MockBlockU32<Test>;

pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		Configuration: configuration,
		Teyrchains: paras,
		ParasShared: shared,
		Registrar: paras_registrar,
		TeyrchainsOrigin: origin,
	}
);

impl<C> pezframe_system::offchain::CreateTransactionBase<C> for Test
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type RuntimeCall = RuntimeCall;
}

impl<C> pezframe_system::offchain::CreateBare<C> for Test
where
	RuntimeCall: From<C>,
{
	fn create_bare(call: Self::RuntimeCall) -> Self::Extrinsic {
		UncheckedExtrinsic::new_bare(call)
	}
}

const NORMAL_RATIO: Perbill = Perbill::from_percent(75);
parameter_types! {
	pub BlockWeights: limits::BlockWeights =
		pezframe_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, u64::MAX));
	pub BlockLength: limits::BlockLength =
		limits::BlockLength::max_with_normal_ratio(4 * 1024 * 1024, NORMAL_RATIO);
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<u64>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type DbWeight = ();
	type BlockWeights = BlockWeights;
	type BlockLength = BlockLength;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
}

impl shared::Config for Test {
	type DisabledValidators = ();
}

impl origin::Config for Test {}

parameter_types! {
	pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl paras::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = paras::TestWeightInfo;
	type UnsignedPriority = ParasUnsignedPriority;
	type QueueFootprinter = ();
	type NextSessionRotation = crate::mock::TestNextSessionRotation;
	type OnNewHead = ();
	type AssignCoretime = ();
	type Fungible = Balances;
	type CooldownRemovalMultiplier = ConstUint<1>;
	type AuthorizeCurrentCodeOrigin = pezframe_system::EnsureRoot<u64>;
}

impl configuration::Config for Test {
	type WeightInfo = configuration::TestWeightInfo;
}

parameter_types! {
	pub const ParaDeposit: Balance = 10;
	pub const DataDepositPerByte: Balance = 1;
	pub const MaxRetries: u32 = 3;
}

impl Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnSwap = MockSwap;
	type ParaDeposit = ParaDeposit;
	type DataDepositPerByte = DataDepositPerByte;
	type WeightInfo = TestWeightInfo;
}

pub fn new_test_ext() -> TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	configuration::GenesisConfig::<Test> {
		config: configuration::HostConfiguration {
			max_code_size: MAX_CODE_SIZE,
			max_head_data_size: 1 * 1024 * 1024, // 1 MB
			..Default::default()
		},
	}
	.assimilate_storage(&mut t)
	.unwrap();

	pezpallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 10_000_000), (2, 10_000_000), (3, 10_000_000)],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

parameter_types! {
	pub static SwapData: BTreeMap<ParaId, u64> = BTreeMap::new();
}

pub struct MockSwap;
impl OnSwap for MockSwap {
	fn on_swap(one: ParaId, other: ParaId) {
		let mut swap_data = SwapData::get();
		let one_data = swap_data.remove(&one).unwrap_or_default();
		let other_data = swap_data.remove(&other).unwrap_or_default();
		swap_data.insert(one, other_data);
		swap_data.insert(other, one_data);
		SwapData::set(swap_data);
	}
}

pub const BLOCKS_PER_SESSION: u32 = 3;

pub const VALIDATORS: &[Sr25519Keyring] = &[
	Sr25519Keyring::Alice,
	Sr25519Keyring::Bob,
	Sr25519Keyring::Charlie,
	Sr25519Keyring::Dave,
	Sr25519Keyring::Ferdie,
];

pub fn run_to_block(n: BlockNumber) {
	// NOTE that this function only simulates modules of interest. Depending on new pezpallet may
	// require adding it here.
	System::run_to_block_with::<AllPalletsWithSystem>(
		n,
		pezframe_system::RunToBlockHooks::default().before_finalize(|bn| {
			// Session change every 3 blocks.
			if (bn + 1) % BLOCKS_PER_SESSION == 0 {
				let session_index = shared::CurrentSessionIndex::<Test>::get() + 1;
				let validators_pub_keys = VALIDATORS.iter().map(|v| v.public().into()).collect();

				shared::Pezpallet::<Test>::set_session_index(session_index);
				shared::Pezpallet::<Test>::set_active_validators_ascending(validators_pub_keys);

				Teyrchains::test_on_new_session();
			}
		}),
	);
}

pub fn run_to_session(n: BlockNumber) {
	let block_number = n * BLOCKS_PER_SESSION;
	run_to_block(block_number);
}

pub fn test_genesis_head(size: usize) -> HeadData {
	HeadData(vec![0u8; size])
}

pub fn test_validation_code(size: usize) -> ValidationCode {
	let validation_code = vec![0u8; size as usize];
	ValidationCode(validation_code)
}

pub fn para_origin(id: ParaId) -> RuntimeOrigin {
	pezkuwi_runtime_teyrchains::Origin::Teyrchain(id).into()
}

pub fn max_code_size() -> u32 {
	configuration::ActiveConfig::<Test>::get().max_code_size
}

pub fn max_head_size() -> u32 {
	configuration::ActiveConfig::<Test>::get().max_head_data_size
}
