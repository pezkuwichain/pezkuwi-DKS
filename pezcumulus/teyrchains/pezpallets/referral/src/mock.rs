// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// pezkuwi/pallets/referral/src/mock.rs (Updated for new trustless model)

use crate as pezpallet_referral;
use pezframe_support::{construct_runtime, derive_impl, parameter_types, traits::ConstU128};
use pezframe_system::EnsureRoot;
use pezsp_core::H256;
use pezsp_runtime::BuildStorage;

type Block = pezframe_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
pub type Balance = u128;

// Test accounts
pub const FOUNDER: AccountId = 100;
pub const REFERRER: AccountId = 1;
pub const REFERRED: AccountId = 2;
pub const USER_3: AccountId = 3;

construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		IdentityKyc: pezpallet_identity_kyc,
		Referral: pezpallet_referral,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<Balance>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
}

parameter_types! {
	pub const KycApplicationDepositAmount: Balance = 100;
	pub const MaxStringLen: u32 = 50;
	pub const MaxCidLen: u32 = 128;
	pub const PenaltyPerRevocationAmount: u32 = 3;
}

// Mock implementation for CitizenNftProvider
pub struct MockCitizenNftProvider;
impl pezpallet_identity_kyc::types::CitizenNftProvider<AccountId> for MockCitizenNftProvider {
	fn mint_citizen_nft(_who: &AccountId) -> pezsp_runtime::DispatchResult {
		Ok(())
	}

	fn mint_citizen_nft_confirmed(_who: &AccountId) -> pezsp_runtime::DispatchResult {
		Ok(())
	}

	fn burn_citizen_nft(_who: &AccountId) -> pezsp_runtime::DispatchResult {
		Ok(())
	}
}

impl pezpallet_identity_kyc::Config for Test {
	type Currency = Balances;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
	type OnKycApproved = Referral; // Referral pezpallet handles KYC approval hook
	type OnCitizenshipRevoked = Referral; // Referral pezpallet handles revocation penalty
	type CitizenNftProvider = MockCitizenNftProvider;
	type DefaultReferrer = DefaultReferrerAccount;
	type KycApplicationDeposit = KycApplicationDepositAmount;
	type MaxStringLength = MaxStringLen;
	type MaxCidLength = MaxCidLen;
}

// Default referrer for testing (founder account)
pub struct DefaultReferrerAccount;
impl pezframe_support::traits::Get<AccountId> for DefaultReferrerAccount {
	fn get() -> AccountId {
		FOUNDER
	}
}

impl pezpallet_referral::Config for Test {
	type WeightInfo = ();
	type DefaultReferrer = DefaultReferrerAccount;
	type PenaltyPerRevocation = PenaltyPerRevocationAmount;
	type TrustScoreUpdater = ();
}

/// Build test externalities with founding citizens
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(FOUNDER, 1_000_000),
			(REFERRER, 10_000),
			(REFERRED, 10_000),
			(USER_3, 10_000),
		],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	// Add founding citizens via genesis config
	pezpallet_identity_kyc::GenesisConfig::<Test> {
		founding_citizens: vec![
			(FOUNDER, H256::from_low_u64_be(1)),
			(REFERRER, H256::from_low_u64_be(2)),
		],
		_phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
