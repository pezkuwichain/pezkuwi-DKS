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

//! Test environment for Nft fractionalization pezpallet.

use super::*;
use crate as pezpallet_nft_fractionalization;

use pezframe::{deps::pezsp_runtime::MultiSignature, testing_prelude::*, traits::Verify};
use pezpallet_nfts::PalletFeatures;

type Block = MockBlock<Test>;
type Signature = MultiSignature;
type AccountPublic = <Signature as Verify>::Signer;
type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

// Configure a mock runtime to test the pezpallet.
construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		NftFractionalization: pezpallet_nft_fractionalization,
		Assets: pezpallet_assets,
		Balances: pezpallet_balances,
		Nfts: pezpallet_nfts,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u64>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
}

impl pezpallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u64;
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetId = u32;
	type AssetIdParameter = u32;
	type ReserveData = ();
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<Self::AccountId>>;
	type ForceOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type AssetDeposit = ConstU64<1>;
	type AssetAccountDeposit = ConstU64<10>;
	type MetadataDepositBase = ConstU64<1>;
	type MetadataDepositPerByte = ConstU64<1>;
	type ApprovalDeposit = ConstU64<1>;
	type StringLimit = ConstU32<50>;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	pezpallet_assets::runtime_benchmarks_enabled! {
		type BenchmarkHelper = ();
	}
}

parameter_types! {
	pub storage Features: PalletFeatures = PalletFeatures::all_enabled();
}

impl pezpallet_nfts::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<pezframe_system::EnsureSigned<Self::AccountId>>;
	type ForceOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type Locker = ();
	type CollectionDeposit = ConstU64<2>;
	type ItemDeposit = ConstU64<1>;
	type MetadataDepositBase = ConstU64<1>;
	type AttributeDepositBase = ConstU64<1>;
	type DepositPerByte = ConstU64<1>;
	type StringLimit = ConstU32<50>;
	type KeyLimit = ConstU32<50>;
	type ValueLimit = ConstU32<50>;
	type ApprovalsLimit = ConstU32<10>;
	type ItemAttributesApprovalsLimit = ConstU32<2>;
	type MaxTips = ConstU32<10>;
	type MaxDeadlineDuration = ConstU64<10000>;
	type MaxAttributesPerCall = ConstU32<2>;
	type Features = Features;
	type OffchainSignature = Signature;
	type OffchainPublic = AccountPublic;
	type WeightInfo = ();
	type BlockNumberProvider = pezframe_system::Pezpallet<Test>;
	pezpallet_nfts::runtime_benchmarks_enabled! {
		type Helper = ();
	}
}

parameter_types! {
	pub const StringLimit: u32 = 50;
	pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
	pub NewAssetSymbol: BoundedVec<u8, StringLimit> = (*b"FRAC").to_vec().try_into().unwrap();
	pub NewAssetName: BoundedVec<u8, StringLimit> = (*b"Frac").to_vec().try_into().unwrap();
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Deposit = ConstU64<1>;
	type Currency = Balances;
	type NewAssetSymbol = NewAssetSymbol;
	type NewAssetName = NewAssetName;
	type NftCollectionId = <Self as pezpallet_nfts::Config>::CollectionId;
	type NftId = <Self as pezpallet_nfts::Config>::ItemId;
	type AssetBalance = <Self as pezpallet_balances::Config>::Balance;
	type AssetId = <Self as pezpallet_assets::Config>::AssetId;
	type Assets = Assets;
	type Nfts = Nfts;
	type PalletId = NftFractionalizationPalletId;
	type WeightInfo = ();
	type StringLimit = StringLimit;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type RuntimeHoldReason = RuntimeHoldReason;
}

// Build genesis storage according to the mock runtime.
pub(crate) fn new_test_ext() -> TestExternalities {
	let t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
