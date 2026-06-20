// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate as pezpallet_trust;
use pezframe_support::{
	derive_impl, parameter_types,
	traits::{ConstU16, ConstU64},
};
use pezframe_system as system;
use pezsp_core::H256;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		IdentityKyc: pezpallet_identity_kyc,
		TrustPallet: pezpallet_trust,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig as pezframe_system::DefaultConfig)]
impl system::Config for Test {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
}

impl pezpallet_balances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Balance = u128;
	type DustRemoval = ();
	type ExistentialDeposit = pezframe_support::traits::ConstU128<1>;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type FreezeIdentifier = ();
	type MaxLocks = pezframe_support::traits::ConstU32<10>;
	type MaxReserves = pezframe_support::traits::ConstU32<10>;
	type MaxFreezes = pezframe_support::traits::ConstU32<10>;
	type DoneSlashHandler = ();
}

pub struct NoOpOnKycApproved;
impl pezpallet_identity_kyc::types::OnKycApproved<u64> for NoOpOnKycApproved {
	fn on_kyc_approved(_who: &u64, _referrer: &u64) {}
}

pub struct NoOpOnCitizenshipRevoked;
impl pezpallet_identity_kyc::types::OnCitizenshipRevoked<u64> for NoOpOnCitizenshipRevoked {
	fn on_citizenship_revoked(_who: &u64) {}
}

pub struct NoOpCitizenNftProvider;
impl pezpallet_identity_kyc::types::CitizenNftProvider<u64> for NoOpCitizenNftProvider {
	fn mint_citizen_nft(_who: &u64) -> Result<(), pezsp_runtime::DispatchError> {
		Ok(())
	}

	fn mint_citizen_nft_confirmed(_who: &u64) -> Result<(), pezsp_runtime::DispatchError> {
		Ok(())
	}

	fn burn_citizen_nft(_who: &u64) -> Result<(), pezsp_runtime::DispatchError> {
		Ok(())
	}
}

pub struct DefaultReferrerAccount;
impl pezframe_support::traits::Get<u64> for DefaultReferrerAccount {
	fn get() -> u64 {
		100 // Founder account for tests
	}
}

impl pezpallet_identity_kyc::Config for Test {
	type Currency = Balances;
	type GovernanceOrigin = pezframe_system::EnsureRoot<u64>;
	type WeightInfo = ();
	type OnKycApproved = NoOpOnKycApproved;
	type OnCitizenshipRevoked = NoOpOnCitizenshipRevoked;
	type CitizenNftProvider = NoOpCitizenNftProvider;
	type DefaultReferrer = DefaultReferrerAccount;
	type KycApplicationDeposit = pezframe_support::traits::ConstU128<100>;
	type MaxStringLength = pezframe_support::traits::ConstU32<128>;
	type MaxCidLength = pezframe_support::traits::ConstU32<64>;
}

parameter_types! {
	pub const ScoreMultiplierBase: u128 = 1000;
	pub const TrustUpdateInterval: u64 = 100; // Test için kısa interval
	pub const MaxBatchSizeValue: u32 = 100; // Max users per batch
}

pub struct MockStakingScoreProvider;
impl pezpallet_trust::StakingScoreProvider<u64, u64> for MockStakingScoreProvider {
	fn get_staking_score(_who: &u64) -> (u32, u64) {
		(100, 0)
	}
}

pub struct MockReferralScoreProvider;
impl pezpallet_trust::ReferralScoreProvider<u64> for MockReferralScoreProvider {
	fn get_referral_score(_who: &u64) -> u32 {
		50
	}
}

pub struct MockPerwerdeScoreProvider;
impl pezpallet_trust::PerwerdeScoreProvider<u64> for MockPerwerdeScoreProvider {
	fn get_perwerde_score(_who: &u64) -> u32 {
		30
	}
}

pub struct MockTikiScoreProvider;
impl pezpallet_trust::TikiScoreProvider<u64> for MockTikiScoreProvider {
	fn get_tiki_score(_who: &u64) -> u32 {
		20
	}
}

pub struct MockCitizenshipStatusProvider;
impl pezpallet_trust::CitizenshipStatusProvider<u64> for MockCitizenshipStatusProvider {
	fn is_citizen(who: &u64) -> bool {
		// Test için: 1-100 arası hesaplar vatandaş, 999 değil
		*who >= 1 && *who <= 100 && *who != 999
	}
}

impl pezpallet_trust::Config for Test {
	type WeightInfo = ();
	type Score = u128;
	type ScoreMultiplierBase = ScoreMultiplierBase;
	type UpdateInterval = TrustUpdateInterval;
	type MaxBatchSize = MaxBatchSizeValue;
	type StakingScoreSource = MockStakingScoreProvider;
	type ReferralScoreSource = MockReferralScoreProvider;
	type PerwerdeScoreSource = MockPerwerdeScoreProvider;
	type TikiScoreSource = MockTikiScoreProvider;
	type CitizenshipSource = MockCitizenshipStatusProvider;
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
