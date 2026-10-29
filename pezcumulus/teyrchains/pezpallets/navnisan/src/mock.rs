// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Mock runtime for pezpallet-navnisan: balances, a settable clock and a settable office
//! register. Parliament is Root; the court is account [`COURT`].

use crate as pezpallet_navnisan;
use crate::{Moment, PriceBands, MILLIS_PER_DAY};
use core::cell::RefCell;
use pezframe_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{fungible, OnUnbalanced, SortedMembers, UnixTime},
	weights::constants::RocksDbWeight,
};
use pezframe_system::{EnsureRoot, EnsureSignedBy};
use pezsp_runtime::BuildStorage;

type Block = pezframe_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
pub type Balance = u128;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;
pub const COURT: AccountId = 40;
pub const TREASURY: AccountId = 999;
pub const MINISTER: AccountId = 7;
pub const ANCHORER: AccountId = 8;

/// A single-seat office and a many-seat one.
pub const FINANCE: u8 = 1;
pub const CITIZEN: u8 = 9;

pub const PRICES: PriceBands<Balance> = PriceBands { len3: 500, len4: 100, len5plus: 10 };

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const CommitDeposit: Balance = 5;
	pub const SubnameDeposit: Balance = 3;
	pub const TextDepositBase: Balance = 2;
	pub const TextDepositPerByte: Balance = 1;
	pub const MinCommitmentAge: Moment = 60_000;
	pub const MaxCommitmentAge: Moment = MILLIS_PER_DAY;
	pub const GracePeriod: Moment = 30 * MILLIS_PER_DAY;
	pub const MaxYears: u32 = 5;
	pub const MaxHorizon: Moment = 10 * crate::YEAR;
	pub const InstitutionDelay: Moment = MILLIS_PER_DAY;
	pub const DisputeDelay: Moment = 7 * MILLIS_PER_DAY;
}

construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		Navnisan: pezpallet_navnisan,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type DbWeight = RocksDbWeight;
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<Balance>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type RuntimeHoldReason = RuntimeHoldReason;
}

std::thread_local! {
	static NOW: RefCell<Moment> = const { RefCell::new(1_000_000) };
	static HOLDER: RefCell<Option<AccountId>> = const { RefCell::new(Some(MINISTER)) };
}

pub fn now() -> Moment {
	NOW.with(|n| *n.borrow())
}
pub fn advance(ms: Moment) {
	NOW.with(|n| *n.borrow_mut() += ms);
}
pub fn set_holder(who: Option<AccountId>) {
	HOLDER.with(|h| *h.borrow_mut() = who);
}

pub struct Clock;
impl UnixTime for Clock {
	fn now() -> core::time::Duration {
		core::time::Duration::from_millis(now())
	}
}

pub struct Offices;
impl crate::Offices<AccountId> for Offices {
	type Office = u8;
	fn is_single_seat(office: &u8) -> bool {
		*office == FINANCE
	}
	fn holder(office: &u8) -> Option<AccountId> {
		(*office == FINANCE).then(|| HOLDER.with(|h| *h.borrow())).flatten()
	}
}

/// Rent lands in [`TREASURY`], standing in for the runtime's AccumulateAndForward.
pub struct ToTreasury;
impl OnUnbalanced<crate::CreditOf<Test>> for ToTreasury {
	fn on_nonzero_unbalanced(credit: crate::CreditOf<Test>) {
		let _ = <Balances as fungible::Balanced<AccountId>>::resolve(&TREASURY, credit);
	}
}

pub struct CourtMembers;
impl SortedMembers<AccountId> for CourtMembers {
	fn sorted_members() -> Vec<AccountId> {
		vec![COURT]
	}
}

impl crate::Config for Test {
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RentDestination = ToTreasury;
	type Time = Clock;
	type Offices = Offices;
	type GrantOrigin = EnsureRoot<AccountId>;
	type PriceOrigin = EnsureRoot<AccountId>;
	type DisputeOrigin = EnsureSignedBy<CourtMembers, AccountId>;
	type CommitDeposit = CommitDeposit;
	type SubnameDeposit = SubnameDeposit;
	type TextDepositBase = TextDepositBase;
	type TextDepositPerByte = TextDepositPerByte;
	type MinCommitmentAge = MinCommitmentAge;
	type MaxCommitmentAge = MaxCommitmentAge;
	type GracePeriod = GracePeriod;
	type MaxYears = MaxYears;
	type MaxHorizon = MaxHorizon;
	type InstitutionDelay = InstitutionDelay;
	type DisputeDelay = DisputeDelay;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BenchClock;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchClock;
#[cfg(feature = "runtime-benchmarks")]
impl crate::BenchmarkHelper for BenchClock {
	fn set_time(ms: Moment) {
		NOW.with(|n| *n.borrow_mut() = ms);
	}
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pezpallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, 10_000),
			(BOB, 10_000),
			(CAROL, 10_000),
			(MINISTER, 1_000),
			(ANCHORER, 100),
			(TREASURY, 1),
			(COURT, 10),
		],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();
	pezpallet_navnisan::GenesisConfig::<Test> {
		zones: vec![b"gov".to_vec(), b"edu".to_vec()],
		gov_subnames: vec![(b"darayi".to_vec(), Some(FINANCE)), (b"meclis".to_vec(), None)],
		locked: vec![(b"serok".to_vec(), 6), (b"dks".to_vec(), 2)],
		prices: PRICES,
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		NOW.with(|n| *n.borrow_mut() = 1_000_000);
		set_holder(Some(MINISTER));
	});
	ext
}
