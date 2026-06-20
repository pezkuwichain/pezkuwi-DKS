// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test environment for Asset Conversion Ops pezpallet.

use crate as pezpallet_asset_conversion_ops;
use core::default::Default;
use pezframe_support::{
	construct_runtime, derive_impl,
	instances::{Instance1, Instance2},
	ord_parameter_types, parameter_types,
	traits::{
		tokens::{
			fungible::{NativeFromLeft, NativeOrWithId, UnionOf},
			imbalance::ResolveAssetTo,
		},
		AsEnsureOriginWithArg, ConstU32, ConstU64,
	},
	PalletId,
};
use pezframe_system::{EnsureSigned, EnsureSignedBy};
use pezpallet_asset_conversion::{self, AccountIdConverter, AccountIdConverterNoSeed, Ascending};
use pezsp_arithmetic::Permill;
use pezsp_runtime::{traits::AccountIdConversion, BuildStorage};

type Block = pezframe_system::mocking::MockBlock<Test>;

construct_runtime!(
  pub enum Test
  {
	System: pezframe_system,
	Balances: pezpallet_balances,
	Assets: pezpallet_assets::<Instance1>,
	PoolAssets: pezpallet_assets::<Instance2>,
	AssetConversion: pezpallet_asset_conversion,
	AssetConversionOps: pezpallet_asset_conversion_ops,
  }
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<u64>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
}

#[derive_impl(pezpallet_assets::config_preludes::TestDefaultConfig)]
impl pezpallet_assets::Config<Instance1> for Test {
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<Self::AccountId>>;
	type ForceOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type Holder = ();
	type Freezer = ();
}

#[derive_impl(pezpallet_assets::config_preludes::TestDefaultConfig)]
impl pezpallet_assets::Config<Instance2> for Test {
	type Currency = Balances;
	type CreateOrigin =
		AsEnsureOriginWithArg<EnsureSignedBy<AssetConversionOrigin, Self::AccountId>>;
	type ForceOrigin = pezframe_system::EnsureRoot<Self::AccountId>;
	type Holder = ();
	type Freezer = ();
}

parameter_types! {
  pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");
  pub const Native: NativeOrWithId<u32> = NativeOrWithId::Native;
  pub storage LiquidityWithdrawalFee: Permill = Permill::from_percent(0);
}

ord_parameter_types! {
  pub const AssetConversionOrigin: u64 = AccountIdConversion::<u64>::into_account_truncating(&AssetConversionPalletId::get());
}

pub type NativeAndAssets = UnionOf<Balances, Assets, NativeFromLeft, NativeOrWithId<u32>, u64>;
pub type PoolIdToAccountId =
	AccountIdConverter<AssetConversionPalletId, (NativeOrWithId<u32>, NativeOrWithId<u32>)>;
pub type AscendingLocator = Ascending<u64, NativeOrWithId<u32>, PoolIdToAccountId>;

impl pezpallet_asset_conversion::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = <Self as pezpallet_balances::Config>::Balance;
	type HigherPrecisionBalance = pezsp_core::U256;
	type AssetKind = NativeOrWithId<u32>;
	type Assets = NativeAndAssets;
	type PoolId = (Self::AssetKind, Self::AssetKind);
	type PoolLocator = AscendingLocator;
	type PoolAssetId = u32;
	type PoolAssets = PoolAssets;
	type PoolSetupFee = ConstU64<100>;
	type PoolSetupFeeAsset = Native;
	type PoolSetupFeeTarget = ResolveAssetTo<AssetConversionOrigin, Self::Assets>;
	type PalletId = AssetConversionPalletId;
	type WeightInfo = ();
	type LPFee = ConstU32<3>;
	type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
	type MaxSwapPathLength = ConstU32<4>;
	type MintMinLiquidity = ConstU64<100>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

pub type OldPoolIdToAccountId =
	AccountIdConverterNoSeed<(NativeOrWithId<u32>, NativeOrWithId<u32>)>;

impl pezpallet_asset_conversion_ops::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PriorAccountIdConverter = OldPoolIdToAccountId;
	type AssetsRefund = NativeAndAssets;
	type PoolAssetsRefund = PoolAssets;
	type PoolAssetsTeam = PoolAssets;
	type DepositAsset = Balances;
	type WeightInfo = ();
}

pub(crate) fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 10000), (2, 20000), (3, 30000), (4, 40000)],
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
