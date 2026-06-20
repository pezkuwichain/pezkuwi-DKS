// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

//! Bridge definitions that can be used by multiple BridgeHub flavors.
//! All configurations here should be dedicated to a single chain; in other words, we don't need two
//! chains for a single pezpallet configuration.
//!
//! For example, the messaging pezpallet needs to know the sending and receiving chains, but the
//! GRANDPA tracking pezpallet only needs to be aware of one chain.

use super::{weights, AccountId, Balance, Balances, BlockNumber, Runtime, RuntimeEvent};
use pezbp_relayers::RewardsAccountParams;
use pezbp_teyrchains::SingleParaStoredHeaderDataBuilder;
use pezframe_support::{parameter_types, traits::ConstU32};

parameter_types! {
	pub const RelayChainHeadersToKeep: u32 = 1024;
	pub const TeyrchainHeadsToKeep: u32 = 64;

	pub const ZagrosBridgeTeyrchainPalletName: &'static str = pezbp_zagros::PARAS_PALLET_NAME;
	pub const MaxZagrosParaHeadDataSize: u32 = pezbp_zagros::MAX_NESTED_TEYRCHAIN_HEAD_DATA_SIZE;

	pub storage RequiredStakeForStakeAndSlash: Balance = 1_000_000;
	pub const RelayerStakeLease: u32 = 8;
	pub const RelayerStakeReserveId: [u8; 8] = *b"brdgrlrs";

	pub storage DeliveryRewardInBalance: u64 = 1_000_000;
}

/// Add GRANDPA bridge pezpallet to track Zagros relay chain.
pub type BridgeGrandpaZagrosInstance = pezpallet_bridge_grandpa::Instance3;
impl pezpallet_bridge_grandpa::Config<BridgeGrandpaZagrosInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgedChain = pezbp_zagros::Zagros;
	type MaxFreeHeadersPerBlock = ConstU32<4>;
	type FreeHeadersInterval = ConstU32<5>;
	type HeadersToKeep = RelayChainHeadersToKeep;
	type WeightInfo = weights::pezpallet_bridge_grandpa::WeightInfo<Runtime>;
}

/// Add teyrchain bridge pezpallet to track Zagros BridgeHub teyrchain
pub type BridgeTeyrchainZagrosInstance = pezpallet_bridge_teyrchains::Instance3;
impl pezpallet_bridge_teyrchains::Config<BridgeTeyrchainZagrosInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezpallet_bridge_teyrchains::WeightInfo<Runtime>;
	type BridgesGrandpaPalletInstance = BridgeGrandpaZagrosInstance;
	type ParasPalletName = ZagrosBridgeTeyrchainPalletName;
	type ParaStoredHeaderDataBuilder =
		SingleParaStoredHeaderDataBuilder<pezbp_bridge_hub_zagros::BridgeHubZagros>;
	type HeadsToKeep = TeyrchainHeadsToKeep;
	type MaxParaHeadDataSize = MaxZagrosParaHeadDataSize;
	type OnNewHead = ();
}

/// Allows collect and claim rewards for relayers
pub type RelayersForLegacyLaneIdsMessagesInstance = ();
impl pezpallet_bridge_relayers::Config<RelayersForLegacyLaneIdsMessagesInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RewardBalance = Balance;
	type Reward = RewardsAccountParams<pezbp_messages::LegacyLaneId>;
	type PaymentProcedure = pezbp_relayers::PayRewardFromAccount<
		pezpallet_balances::Pezpallet<Runtime>,
		AccountId,
		pezbp_messages::LegacyLaneId,
		Self::RewardBalance,
	>;
	type StakeAndSlash = pezpallet_bridge_relayers::StakeAndSlashNamed<
		AccountId,
		BlockNumber,
		Balances,
		RelayerStakeReserveId,
		RequiredStakeForStakeAndSlash,
		RelayerStakeLease,
	>;
	type Balance = Balance;
	type WeightInfo = weights::pezpallet_bridge_relayers_legacy::WeightInfo<Runtime>;
}

/// Allows collect and claim rewards for relayers
pub type RelayersForPermissionlessLanesInstance = pezpallet_bridge_relayers::Instance2;
impl pezpallet_bridge_relayers::Config<RelayersForPermissionlessLanesInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RewardBalance = Balance;
	type Reward = RewardsAccountParams<pezbp_messages::HashedLaneId>;
	type PaymentProcedure = pezbp_relayers::PayRewardFromAccount<
		pezpallet_balances::Pezpallet<Runtime>,
		AccountId,
		pezbp_messages::HashedLaneId,
		Self::RewardBalance,
	>;
	type StakeAndSlash = pezpallet_bridge_relayers::StakeAndSlashNamed<
		AccountId,
		BlockNumber,
		Balances,
		RelayerStakeReserveId,
		RequiredStakeForStakeAndSlash,
		RelayerStakeLease,
	>;
	type Balance = Balance;
	type WeightInfo = weights::pezpallet_bridge_relayers_permissionless_lanes::WeightInfo<Runtime>;
}

/// Add GRANDPA bridge pezpallet to track Pezkuwichain Bulletin chain.
pub type BridgeGrandpaPezkuwichainBulletinInstance = pezpallet_bridge_grandpa::Instance4;
impl pezpallet_bridge_grandpa::Config<BridgeGrandpaPezkuwichainBulletinInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BridgedChain = pezbp_pezkuwi_bulletin::PezkuwiBulletin;
	type MaxFreeHeadersPerBlock = ConstU32<4>;
	type FreeHeadersInterval = ConstU32<5>;
	type HeadersToKeep = RelayChainHeadersToKeep;
	// Technically this is incorrect - we have two pezpallet instances and ideally we shall
	// benchmark every instance separately. But the benchmarking engine has a flaw - it
	// messes with components. E.g. in Dicle maximal validators count is 1024 and in
	// Bulletin chain it is 100. But benchmarking engine runs Bulletin benchmarks using
	// components range, computed for Dicle => it causes an error.
	//
	// In practice, however, GRANDPA pezpallet works the same way for all bridged chains, so
	// weights are also the same for both bridges.
	type WeightInfo = weights::pezpallet_bridge_grandpa::WeightInfo<Runtime>;
}
