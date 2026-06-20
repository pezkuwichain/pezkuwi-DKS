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

//! Module with configuration which reflects AssetHubPezkuwichain runtime setup.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use codec::{Decode, Encode};
use scale_info::TypeInfo;

pub use pezbp_bridge_hub_pezcumulus::*;
use pezbp_messages::*;
use pezbp_runtime::{
	decl_bridge_finality_runtime_apis, decl_bridge_messages_runtime_apis, Chain, ChainId, Teyrchain,
};
pub use pezbp_xcm_bridge_hub_router::XcmBridgeHubRouterCall;
use pezframe_support::{
	dispatch::DispatchClass,
	pezsp_runtime::{MultiAddress, MultiSigner, RuntimeDebug, StateVersion},
};
use testnet_teyrchains_constants::pezkuwichain::currency::UNITS;
use xcm::latest::prelude::*;

/// `AssetHubPezkuwichain` Runtime `Call` enum.
///
/// The enum represents a subset of possible `Call`s we can send to `AssetHubPezkuwichain` chain.
/// Ideally this code would be auto-generated from metadata, because we want to
/// avoid depending directly on the ENTIRE runtime just to get the encoding of `Dispatchable`s.
///
/// All entries here (like pretty much in the entire file) must be kept in sync with
/// `AssetHubPezkuwichain` `construct_runtime`, so that we maintain SCALE-compatibility.
#[allow(clippy::large_enum_variant)]
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub enum Call {
	/// `ToZagrosXcmRouter` bridge pezpallet.
	#[codec(index = 45)]
	ToZagrosXcmRouter(XcmBridgeHubRouterCall),
}

pezframe_support::parameter_types! {
	/// Some sane weight to execute `xcm::Transact(pezpallet-xcm-bridge-hub-router::Call::report_bridge_status)`.
	pub const XcmBridgeHubRouterTransactCallMaxWeight: pezframe_support::weights::Weight = pezframe_support::weights::Weight::from_parts(200_000_000, 6144);
	/// Should match the `AssetDeposit` of the `ForeignAssets` pezpallet on Asset Hub.
	pub const CreateForeignAssetDeposit: u128 = UNITS / 10;
}

/// Builds an (un)congestion XCM program with the `report_bridge_status` call for
/// `ToZagrosXcmRouter`.
pub fn build_congestion_message<RuntimeCall>(
	bridge_id: pezsp_core::H256,
	is_congested: bool,
) -> alloc::vec::Vec<Instruction<RuntimeCall>> {
	alloc::vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		Transact {
			origin_kind: OriginKind::Xcm,
			fallback_max_weight: Some(XcmBridgeHubRouterTransactCallMaxWeight::get()),
			call: Call::ToZagrosXcmRouter(XcmBridgeHubRouterCall::report_bridge_status {
				bridge_id,
				is_congested,
			})
			.encode()
			.into(),
		},
		ExpectTransactStatus(MaybeErrorCode::Success),
	]
}

/// Identifier of AssetHubPezkuwichain in the Pezkuwichain relay chain.
pub const ASSET_HUB_PEZKUWICHAIN_TEYRCHAIN_ID: u32 = 1000;

/// AssetHubPezkuwichain teyrchain.
#[derive(RuntimeDebug)]

pub struct AssetHubPezkuwichain;

impl Chain for AssetHubPezkuwichain {
	const ID: ChainId = *b"ahro";

	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hasher;
	type Header = Header;

	type AccountId = AccountId;
	type Balance = Balance;
	type Nonce = Nonce;
	type Signature = Signature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		*BlockLength::get().max.get(DispatchClass::Normal)
	}

	fn max_extrinsic_weight() -> Weight {
		BlockWeightsForAsyncBacking::get()
			.get(DispatchClass::Normal)
			.max_extrinsic
			.unwrap_or(Weight::MAX)
	}
}

impl Teyrchain for AssetHubPezkuwichain {
	const TEYRCHAIN_ID: u32 = ASSET_HUB_PEZKUWICHAIN_TEYRCHAIN_ID;
	const MAX_HEADER_SIZE: u32 = MAX_ASSET_HUB_HEADER_SIZE;
}

/// Describing permissionless lanes instance
impl ChainWithMessages for AssetHubPezkuwichain {
	const WITH_CHAIN_MESSAGES_PALLET_NAME: &'static str =
		WITH_ASSET_HUB_PEZKUWICHAIN_MESSAGES_PALLET_NAME;

	const MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX: MessageNonce =
		MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	const MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
}

/// Public key of the chain account that may be used to verify signatures.
pub type AccountSigner = MultiSigner;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Name of the With-AssetHubPezkuwichain messages pezpallet instance that is deployed at bridged
/// chains.
pub const WITH_ASSET_HUB_PEZKUWICHAIN_MESSAGES_PALLET_NAME: &str = "BridgePezkuwichainMessages";

/// Name of the With-AssetHubPezkuwichain bridge-relayers pezpallet instance that is deployed at
/// bridged chains.
pub const WITH_ASSET_HUB_PEZKUWICHAIN_RELAYERS_PALLET_NAME: &str = "BridgeRelayers";

/// Pezpallet index of `BridgeZagrosMessages: pezpallet_bridge_messages::<Instance1>`.
pub const WITH_BRIDGE_PEZKUWICHAIN_TO_ZAGROS_MESSAGES_PALLET_INDEX: u8 = 62;

decl_bridge_finality_runtime_apis!(asset_hub_pezkuwichain);
decl_bridge_messages_runtime_apis!(asset_hub_pezkuwichain, HashedLaneId);
