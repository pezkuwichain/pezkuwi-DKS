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

//! Module with configuration which reflects BridgeHubPezkuwichain runtime setup (AccountId,
//! Headers, Hashes...)

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
pub use pezbp_bridge_hub_pezcumulus::*;
use pezbp_messages::*;
use pezbp_runtime::{
	decl_bridge_finality_runtime_apis, decl_bridge_messages_runtime_apis, Chain, ChainId, Teyrchain,
};
use pezframe_support::{
	dispatch::DispatchClass,
	pezsp_runtime::{MultiAddress, MultiSigner, RuntimeDebug, StateVersion},
};

/// BridgeHubPezkuwichain teyrchain.
#[derive(RuntimeDebug)]
pub struct BridgeHubPezkuwichain;

impl Chain for BridgeHubPezkuwichain {
	const ID: ChainId = *b"bhro";

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

impl Teyrchain for BridgeHubPezkuwichain {
	const TEYRCHAIN_ID: u32 = BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID;
	const MAX_HEADER_SIZE: u32 = MAX_BRIDGE_HUB_HEADER_SIZE;
}

impl ChainWithMessages for BridgeHubPezkuwichain {
	const WITH_CHAIN_MESSAGES_PALLET_NAME: &'static str =
		WITH_BRIDGE_HUB_PEZKUWICHAIN_MESSAGES_PALLET_NAME;

	const MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX: MessageNonce =
		MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	const MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
}

/// Public key of the chain account that may be used to verify signatures.
pub type AccountSigner = MultiSigner;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Identifier of BridgeHubPezkuwichain in the Pezkuwichain relay chain.
pub const BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID: u32 = 1002;

/// Name of the With-BridgeHubPezkuwichain messages pezpallet instance that is deployed at bridged
/// chains.
pub const WITH_BRIDGE_HUB_PEZKUWICHAIN_MESSAGES_PALLET_NAME: &str = "BridgePezkuwichainMessages";

/// Name of the With-BridgeHubPezkuwichain bridge-relayers pezpallet instance that is deployed at
/// bridged chains.
pub const WITH_BRIDGE_HUB_PEZKUWICHAIN_RELAYERS_PALLET_NAME: &str = "BridgeRelayers";

/// Pezpallet index of `BridgeZagrosMessages: pezpallet_bridge_messages::<Instance3>`.
pub const WITH_BRIDGE_PEZKUWICHAIN_TO_ZAGROS_MESSAGES_PALLET_INDEX: u8 = 51;
/// Pezpallet index of `BridgePezkuwiBulletinMessages: pezpallet_bridge_messages::<Instance4>`.
pub const WITH_BRIDGE_PEZKUWICHAIN_TO_BULLETIN_MESSAGES_PALLET_INDEX: u8 = 61;

decl_bridge_finality_runtime_apis!(bridge_hub_pezkuwichain);
decl_bridge_messages_runtime_apis!(bridge_hub_pezkuwichain, LegacyLaneId);

pezframe_support::parameter_types! {
	/// The XCM fee that is paid for executing XCM program (with `ExportMessage` instruction) at the Pezkuwichain
	/// BridgeHub.
	/// (initially was calculated by test `BridgeHubPezkuwichain::can_calculate_weight_for_paid_export_message_with_reserve_transfer` + `33%`)
	pub const BridgeHubPezkuwichainBaseXcmFeeInRocs: u128 = 72_091_666;

	/// Transaction fee that is paid at the Pezkuwichain BridgeHub for delivering single inbound message.
	/// (initially was calculated by test `BridgeHubPezkuwichain::can_calculate_fee_for_standalone_message_delivery_transaction` + `33%`)
	pub const BridgeHubPezkuwichainBaseDeliveryFeeInRocs: u128 = 297_685_840;

	/// Transaction fee that is paid at the Pezkuwichain BridgeHub for delivering single outbound message confirmation.
	/// (initially was calculated by test `BridgeHubPezkuwichain::can_calculate_fee_for_standalone_message_confirmation_transaction` + `33%`)
	pub const BridgeHubPezkuwichainBaseConfirmationFeeInRocs: u128 = 56_782_099;
}

/// Wrapper over `BridgeHubPezkuwichain`'s `RuntimeCall` that can be used without a runtime.
#[derive(Decode, Encode)]
pub enum RuntimeCall {
	/// Points to the `pezpallet_xcm_bridge_hub` pezpallet instance for `BridgeHubZagros`.
	#[codec(index = 52)]
	XcmOverBridgeHubZagros(pezbp_xcm_bridge_hub::XcmBridgeHubCall),
}
