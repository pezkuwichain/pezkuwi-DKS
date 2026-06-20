// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use crate as pezpallet_tiki;
use crate::Tiki as TikiEnum;
use pezframe_support::{
	assert_ok, construct_runtime, parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32, ConstU64},
};
use pezsp_core::H256;
use pezsp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};

#[cfg(feature = "runtime-benchmarks")]
use pezsp_runtime::testing::{TestSignature, UintAuthorityId};
#[cfg(feature = "runtime-benchmarks")]
use pezsp_runtime::RuntimeAppPublic;

// Custom BenchmarkHelper for pezpallet_identity (uses TestSignature in mock)
#[cfg(feature = "runtime-benchmarks")]
pub struct IdentityBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_identity::BenchmarkHelper<UintAuthorityId, TestSignature>
	for IdentityBenchmarkHelper
{
	fn sign_message(message: &[u8]) -> (UintAuthorityId, TestSignature) {
		let signer = UintAuthorityId(0);
		let signature =
			<UintAuthorityId as RuntimeAppPublic>::sign(&signer, &message.to_vec()).unwrap();
		(signer, signature)
	}
}

// Custom BenchmarkHelper for pezpallet_nfts (uses u64 AccountId in mock)
#[cfg(feature = "runtime-benchmarks")]
pub struct NftsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_nfts::BenchmarkHelper<u32, u32, UintAuthorityId, AccountId, TestSignature>
	for NftsBenchmarkHelper
{
	fn collection(i: u16) -> u32 {
		i.into()
	}
	fn item(i: u16) -> u32 {
		i.into()
	}
	fn signer() -> (UintAuthorityId, AccountId) {
		let signer = UintAuthorityId(0);
		let account: AccountId = 1u64;
		(signer, account)
	}
	fn sign(signer: &UintAuthorityId, data: &[u8]) -> TestSignature {
		<UintAuthorityId as RuntimeAppPublic>::sign(signer, &data).unwrap()
	}
}

type Block = pezframe_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
pub type Balance = u128;

// Runtime'ı oluştur - Identity ve IdentityKyc pezpallet'lerini de ekle
construct_runtime!(
	pub enum Test
	{
		System: pezframe_system::{Pezpallet, Call, Config<T>, Storage, Event<T>},
		Balances: pezpallet_balances::{Pezpallet, Call, Storage, Event<T>},
		Identity: pezpallet_identity::{Pezpallet, Call, Storage, Event<T>},
		IdentityKyc: pezpallet_identity_kyc::{Pezpallet, Call, Storage, Event<T>},
		Nfts: pezpallet_nfts::{Pezpallet, Call, Storage, Event<T>},
		Tiki: pezpallet_tiki::{Pezpallet, Call, Config<T>, Storage, Event<T>},
	}
);

impl pezframe_system::Config for Test {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = pezframe_support::weights::constants::RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type RuntimeTask = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = (); // Eksik olan trait
	type ExtensionsWeightInfo = ();
}

impl pezpallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type DoneSlashHandler = ();
}

// pezpallet_identity::Config implementasyonu
parameter_types! {
	pub const BasicDeposit: Balance = 1000;
	pub const ByteDeposit: Balance = 10;
	pub const SubAccountDeposit: Balance = 100;
	pub const MaxSubAccounts: u32 = 10;
	pub const MaxRegistrars: u32 = 10;
	pub const UsernameDeposit: Balance = 100;
	pub const PendingUsernameExpiration: u64 = 100;
	pub const UsernameGracePeriod: u64 = 50;
	pub const MaxSuffixLength: u32 = 10;
	pub const MaxUsernameLength: u32 = 32;
}

impl pezpallet_identity::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type IdentityInformation = pezpallet_identity::legacy::IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = ();
	type ForceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type RegistrarOrigin = pezframe_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type OffchainSignature = pezsp_runtime::testing::TestSignature;
	type SigningPublicKey =
		<pezsp_runtime::testing::TestSignature as pezsp_runtime::traits::Verify>::Signer;
	type UsernameAuthorityOrigin = pezframe_system::EnsureRoot<AccountId>;
	type UsernameDeposit = UsernameDeposit;
	type PendingUsernameExpiration = PendingUsernameExpiration;
	type UsernameGracePeriod = UsernameGracePeriod;
	type MaxSuffixLength = MaxSuffixLength;
	type MaxUsernameLength = MaxUsernameLength;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = IdentityBenchmarkHelper;
}

parameter_types! {
	pub const MaxAdditionalFields: u32 = 10;
}

// pezpallet_identity_kyc::Config parameters
parameter_types! {
	pub const KycApplicationDepositAmount: Balance = 100;
	pub const MaxCidLength: u32 = 100;
}

// Mock implementation for OnKycApproved hook (updated for new trait signature)
pub struct MockOnKycApproved;
impl pezpallet_identity_kyc::types::OnKycApproved<AccountId> for MockOnKycApproved {
	fn on_kyc_approved(_who: &AccountId, _referrer: &AccountId) {
		// No-op for tests
	}
}

// Mock implementation for OnCitizenshipRevoked hook
pub struct MockOnCitizenshipRevoked;
impl pezpallet_identity_kyc::types::OnCitizenshipRevoked<AccountId> for MockOnCitizenshipRevoked {
	fn on_citizenship_revoked(_who: &AccountId) {
		// No-op for tests
	}
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

pub struct DefaultReferrerAccount;
impl pezframe_support::traits::Get<AccountId> for DefaultReferrerAccount {
	fn get() -> AccountId {
		100
	}
}

impl pezpallet_identity_kyc::Config for Test {
	type Currency = Balances;
	type WeightInfo = ();
	type GovernanceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type KycApplicationDeposit = KycApplicationDepositAmount;
	type MaxStringLength = ConstU32<50>;
	type MaxCidLength = MaxCidLength;
	type OnKycApproved = MockOnKycApproved;
	type OnCitizenshipRevoked = MockOnCitizenshipRevoked;
	type CitizenNftProvider = MockCitizenNftProvider;
	type DefaultReferrer = DefaultReferrerAccount;
}

parameter_types! {
	pub Features: pezpallet_nfts::PalletFeatures = pezpallet_nfts::PalletFeatures::default();
}

impl pezpallet_nfts::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = pezframe_system::EnsureRoot<AccountId>;
	type CreateOrigin = AsEnsureOriginWithArg<pezframe_system::EnsureSigned<AccountId>>;
	type Locker = ();
	type CollectionDeposit = ConstU128<0>;
	type ItemDeposit = ConstU128<0>;
	type MetadataDepositBase = ConstU128<0>;
	type AttributeDepositBase = ConstU128<0>;
	type DepositPerByte = ConstU128<0>;
	type StringLimit = ConstU32<256>;
	type KeyLimit = ConstU32<64>;
	type ValueLimit = ConstU32<256>;
	type ApprovalsLimit = ConstU32<10>;
	type ItemAttributesApprovalsLimit = ConstU32<20>;
	type MaxTips = ConstU32<10>;
	type MaxDeadlineDuration = ConstU64<10000>;
	type MaxAttributesPerCall = ConstU32<10>;
	type Features = Features;
	type OffchainSignature = pezsp_runtime::testing::TestSignature;
	type OffchainPublic = pezsp_runtime::testing::UintAuthorityId;
	type WeightInfo = ();
	type BlockNumberProvider = System;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = NftsBenchmarkHelper;
}

parameter_types! {
	pub const TikiCollectionId: u32 = 0;
	pub const MaxTikisPerUser: u32 = 100;
}

impl crate::Config for Test {
	type AdminOrigin = pezframe_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type TikiCollectionId = TikiCollectionId;
	type MaxTikisPerUser = MaxTikisPerUser;
	type Tiki = TikiEnum;
	type TrustScoreUpdater = ();
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 10000), (2, 10000), (3, 10000), (4, 10000), (5, 10000)],
		dev_accounts: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);

		// Tiki koleksiyonunu oluştur - mint permissions ile
		assert_ok!(Nfts::force_create(
			RuntimeOrigin::root(),
			1, // owner
			pezpallet_nfts::CollectionConfig {
				settings: pezpallet_nfts::CollectionSettings::all_enabled(),
				max_supply: None,
				mint_settings: pezpallet_nfts::MintSettings {
					mint_type: pezpallet_nfts::MintType::Public,
					price: None,
					start_block: None,
					end_block: None,
					default_item_settings: pezpallet_nfts::ItemSettings::all_enabled(),
				},
			}
		));
	});
	ext
}
