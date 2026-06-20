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
use crate::{
	bridge_to_ethereum_config::InboundQueueV2Location, xcm_config::XcmConfig, RuntimeCall,
	XcmRouter,
};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pezbp_messages::LegacyLaneId;
use pezbp_relayers::RewardsAccountParams;
use pezframe_support::parameter_types;
use scale_info::TypeInfo;
use testnet_teyrchains_constants::zagros::{
	locations::AssetHubLocation, snowbridge::EthereumNetwork,
};
use xcm::{opaque::latest::Location, VersionedLocation};
use xcm_executor::XcmExecutor;

parameter_types! {
	pub storage RequiredStakeForStakeAndSlash: Balance = 1_000_000;
	pub const RelayerStakeLease: u32 = 8;
	pub const RelayerStakeReserveId: [u8; 8] = *b"brdgrlrs";
}

/// Showcasing that we can handle multiple different rewards with the same pezpallet.
#[derive(
	Clone,
	Copy,
	Debug,
	Decode,
	DecodeWithMemTracking,
	Encode,
	Eq,
	MaxEncodedLen,
	PartialEq,
	TypeInfo,
)]
pub enum BridgeReward {
	/// Rewards for the R/W bridge—distinguished by the `RewardsAccountParams` key.
	PezkuwichainZagros(RewardsAccountParams<LegacyLaneId>),
	/// Rewards for Snowbridge.
	Snowbridge,
}

impl From<RewardsAccountParams<LegacyLaneId>> for BridgeReward {
	fn from(value: RewardsAccountParams<LegacyLaneId>) -> Self {
		Self::PezkuwichainZagros(value)
	}
}

/// An enum representing the different types of supported beneficiaries.
#[derive(
	Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo,
)]
pub enum BridgeRewardBeneficiaries {
	/// A local chain account.
	LocalAccount(AccountId),
	/// A beneficiary specified by a VersionedLocation.
	AssetHubLocation(VersionedLocation),
}

impl From<pezsp_runtime::AccountId32> for BridgeRewardBeneficiaries {
	fn from(value: pezsp_runtime::AccountId32) -> Self {
		BridgeRewardBeneficiaries::LocalAccount(value)
	}
}

/// Implementation of `pezbp_relayers::PaymentProcedure` as a pay/claim rewards scheme.
pub struct BridgeRewardPayer;
impl pezbp_relayers::PaymentProcedure<AccountId, BridgeReward, u128> for BridgeRewardPayer {
	type Error = pezsp_runtime::DispatchError;
	type Beneficiary = BridgeRewardBeneficiaries;

	fn pay_reward(
		relayer: &AccountId,
		reward_kind: BridgeReward,
		reward: u128,
		beneficiary: BridgeRewardBeneficiaries,
	) -> Result<(), Self::Error> {
		match reward_kind {
			BridgeReward::PezkuwichainZagros(lane_params) => {
				match beneficiary {
					BridgeRewardBeneficiaries::LocalAccount(account) => {
						pezbp_relayers::PayRewardFromAccount::<
							Balances,
							AccountId,
							LegacyLaneId,
							u128,
						>::pay_reward(
							&relayer, lane_params, reward, account,
						)
					},
					BridgeRewardBeneficiaries::AssetHubLocation(_) => Err(Self::Error::Other("`AssetHubLocation` beneficiary is not supported for `PezkuwichainZagros` rewards!")),
				}
			},
			BridgeReward::Snowbridge => {
				match beneficiary {
					BridgeRewardBeneficiaries::LocalAccount(_) => Err(Self::Error::Other("`LocalAccount` beneficiary is not supported for `Snowbridge` rewards!")),
					BridgeRewardBeneficiaries::AssetHubLocation(account_location) => {
						let account_location = Location::try_from(account_location)
							.map_err(|_| Self::Error::Other("`AssetHubLocation` beneficiary location version is not supported for `Snowbridge` rewards!"))?;
						pezsnowbridge_core::reward::PayAccountOnLocation::<
							AccountId,
							u128,
							EthereumNetwork,
							AssetHubLocation,
							InboundQueueV2Location,
							XcmRouter,
							XcmExecutor<XcmConfig>,
							RuntimeCall
						>::pay_reward(
							relayer, (), reward, account_location
						)
					}
				}
			}
		}
	}
}

/// Allows collect and claim rewards for relayers
pub type BridgeRelayersInstance = ();
impl pezpallet_bridge_relayers::Config<BridgeRelayersInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type RewardBalance = u128;
	type Reward = BridgeReward;
	type PaymentProcedure = BridgeRewardPayer;

	type StakeAndSlash = pezpallet_bridge_relayers::StakeAndSlashNamed<
		AccountId,
		BlockNumber,
		Balances,
		RelayerStakeReserveId,
		RequiredStakeForStakeAndSlash,
		RelayerStakeLease,
	>;
	type Balance = Balance;
	type WeightInfo = weights::pezpallet_bridge_relayers::WeightInfo<Runtime>;
}
