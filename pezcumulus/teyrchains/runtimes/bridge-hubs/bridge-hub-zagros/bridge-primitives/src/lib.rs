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

//! Module with configuration which reflects BridgeHubZagros runtime setup
//! (AccountId, Headers, Hashes...)

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
pub use pezbp_bridge_hub_pezcumulus::*;
use pezbp_messages::*;
use pezbp_runtime::{
	decl_bridge_finality_runtime_apis, decl_bridge_messages_runtime_apis, Chain, ChainId, Teyrchain,
};
use pezframe_support::dispatch::DispatchClass;
use pezsp_runtime::StateVersion;

/// BridgeHubZagros teyrchain.
#[derive(Debug)]
pub struct BridgeHubZagros;

impl Chain for BridgeHubZagros {
	const ID: ChainId = *b"bhwd";

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

impl Teyrchain for BridgeHubZagros {
	const TEYRCHAIN_ID: u32 = BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID;
	const MAX_HEADER_SIZE: u32 = MAX_BRIDGE_HUB_HEADER_SIZE;
}

impl ChainWithMessages for BridgeHubZagros {
	const WITH_CHAIN_MESSAGES_PALLET_NAME: &'static str =
		WITH_BRIDGE_HUB_ZAGROS_MESSAGES_PALLET_NAME;

	const MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX: MessageNonce =
		MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX;
	const MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX: MessageNonce =
		MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX;
}

/// Identifier of BridgeHubZagros in the Zagros relay chain.
pub const BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID: u32 = 1002;

/// Name of the With-BridgeHubZagros messages pezpallet instance that is deployed at bridged chains.
pub const WITH_BRIDGE_HUB_ZAGROS_MESSAGES_PALLET_NAME: &str = "BridgeZagrosMessages";

/// Name of the With-BridgeHubZagros bridge-relayers pezpallet instance that is deployed at bridged
/// chains.
pub const WITH_BRIDGE_HUB_ZAGROS_RELAYERS_PALLET_NAME: &str = "BridgeRelayers";

/// Pezpallet index of `BridgePezkuwichainMessages: pezpallet_bridge_messages::<Instance1>`.
pub const WITH_BRIDGE_ZAGROS_TO_PEZKUWICHAIN_MESSAGES_PALLET_INDEX: u8 = 44;

decl_bridge_finality_runtime_apis!(bridge_hub_zagros);
decl_bridge_messages_runtime_apis!(bridge_hub_zagros, LegacyLaneId);

// These three are the fees this chain charges for bridge work, and each one is the figure the
// test named beside it measures, plus the 33% headroom upstream also leaves. They had been
// carrying the numbers upstream measured on its own chains, which are ~300x ours: our CENTS is
// UNITS/30_000 where upstream's is UNITS/100, so a constant copied across that boundary
// overcharges by the ratio between the two scales. The sibling chain's constants were already at our scale, which is how
// the asymmetry showed.
//
// They are measured figures, not derived ones: re-run the tests and update these when the
// weights are re-benchmarked, because the numbers move with the weights.
pezframe_support::parameter_types! {
	/// The XCM fee that is paid for executing XCM program (with `ExportMessage` instruction) at the Zagros
	/// BridgeHub.
	/// (calculated by test `BridgeHubZagros::can_calculate_weight_for_paid_export_message_with_reserve_transfer` + `33%`)
	pub const BridgeHubZagrosBaseXcmFeeInWnds: u128 = 76_541_499;

	/// Transaction fee that is paid at the Zagros BridgeHub for delivering single inbound message.
	/// (calculated by test `BridgeHubZagros::can_calculate_fee_for_standalone_message_delivery_transaction` + `33%`)
	pub const BridgeHubZagrosBaseDeliveryFeeInWnds: u128 = 295_185_160;

	/// Transaction fee that is paid at the Zagros BridgeHub for delivering single outbound message confirmation.
	/// (calculated by test `BridgeHubZagros::can_calculate_fee_for_standalone_message_confirmation_transaction` + `33%`)
	pub const BridgeHubZagrosBaseConfirmationFeeInWnds: u128 = 54_052_251;
}

/// Wrapper over `BridgeHubZagros`'s `RuntimeCall` that can be used without a runtime.
#[derive(Decode, Encode)]
pub enum RuntimeCall {
	/// Points to the `pezpallet_xcm_bridge_hub` pezpallet instance for `BridgeHubPezkuwichain`.
	#[codec(index = 45)]
	XcmOverBridgeHubPezkuwichain(pezbp_xcm_bridge_hub::XcmBridgeHubCall),
}
