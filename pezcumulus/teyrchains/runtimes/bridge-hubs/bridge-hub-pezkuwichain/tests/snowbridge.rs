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

#![cfg(test)]

use codec::{Decode, Encode};
use pezbp_pezkuwi_core::Signature;
use pezbridge_hub_pezkuwichain_runtime::{
	bridge_to_zagros_config::OnBridgeHubPezkuwichainRefundBridgeHubZagrosMessages,
	xcm_config::XcmConfig, AllPalletsWithoutSystem, BridgeRejectObsoleteHeadersAndMessages,
	Executive, MessageQueueServiceWeight, Runtime, RuntimeCall, RuntimeEvent, SessionKeys,
	TxExtension, UncheckedExtrinsic,
};
use pezcumulus_primitives_core::XcmError::FailedToTransactAsset;
use pezframe_support::parameter_types;
use pezsnowbridge_pezpallet_ethereum_client::WeightInfo;
use pezsp_core::H160;
use pezsp_keyring::Sr25519Keyring::Alice;
use pezsp_runtime::{
	generic::{Era, SignedPayload},
	AccountId32,
};
use teyrchains_common::{AccountId, AuraId, Balance};

parameter_types! {
		pub const DefaultBridgeHubEthereumBaseFee: Balance = 3_833_568_200_000;
}

fn collator_session_keys() -> pezbridge_hub_test_utils::CollatorSessionKeys<Runtime> {
	pezbridge_hub_test_utils::CollatorSessionKeys::new(
		AccountId::from(Alice),
		AccountId::from(Alice),
		SessionKeys { aura: AuraId::from(Alice.public()) },
	)
}

#[test]
pub fn transfer_token_to_ethereum_works() {
	pezsnowbridge_runtime_test_common::send_transfer_token_message_success::<Runtime, XcmConfig>(
		11155111,
		collator_session_keys(),
		1002,
		1000,
		H160::random(),
		H160::random(),
		DefaultBridgeHubEthereumBaseFee::get(),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::EthereumOutboundQueue(event)) => Some(event),
				_ => None,
			}
		}),
	)
}

#[test]
pub fn unpaid_transfer_token_to_ethereum_should_work() {
	pezsnowbridge_runtime_test_common::send_unpaid_transfer_token_message::<Runtime, XcmConfig>(
		11155111,
		collator_session_keys(),
		1002,
		1000,
		H160::random(),
		H160::random(),
	)
}

#[test]
pub fn transfer_token_to_ethereum_insufficient_fund() {
	pezsnowbridge_runtime_test_common::send_transfer_token_message_failure::<Runtime, XcmConfig>(
		11155111,
		collator_session_keys(),
		1002,
		1000,
		1_000_000_000,
		H160::random(),
		H160::random(),
		DefaultBridgeHubEthereumBaseFee::get(),
		FailedToTransactAsset("Funds are unavailable"),
	)
}

#[test]
fn max_message_queue_service_weight_is_more_than_beacon_extrinsic_weights() {
	let max_message_queue_weight = MessageQueueServiceWeight::get();
	let force_checkpoint =
		<Runtime as pezsnowbridge_pezpallet_ethereum_client::Config>::WeightInfo::force_checkpoint(
		);
	let submit_checkpoint =
		<Runtime as pezsnowbridge_pezpallet_ethereum_client::Config>::WeightInfo::submit();
	max_message_queue_weight.all_gt(force_checkpoint);
	max_message_queue_weight.all_gt(submit_checkpoint);
}

#[test]
fn ethereum_client_consensus_extrinsics_work() {
	pezsnowbridge_runtime_test_common::ethereum_extrinsic(
		collator_session_keys(),
		1002,
		construct_and_apply_extrinsic,
	);
}

#[test]
fn ethereum_to_pezkuwi_message_extrinsics_work() {
	pezsnowbridge_runtime_test_common::ethereum_to_pezkuwi_message_extrinsics_work(
		collator_session_keys(),
		1002,
		construct_and_apply_extrinsic,
	);
}

/// Tests that the digest items are as expected when a Ethereum Outbound message is received.
/// If the MessageQueue pezpallet is configured before (i.e. the MessageQueue pezpallet is listed
/// before the EthereumOutboundQueue in the construct_runtime macro) the EthereumOutboundQueue, this
/// test will fail.
#[test]
pub fn ethereum_outbound_queue_processes_messages_before_message_queue_works() {
	pezsnowbridge_runtime_test_common::ethereum_outbound_queue_processes_messages_before_message_queue_works::<
		Runtime,
		XcmConfig,
		AllPalletsWithoutSystem,
	>(
		11155111,
		collator_session_keys(),
		1002,
		1000,
		H160::random(),
		H160::random(),
		DefaultBridgeHubEthereumBaseFee::get(),
		Box::new(|runtime_event_encoded: Vec<u8>| {
			match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
				Ok(RuntimeEvent::EthereumOutboundQueue(event)) => Some(event),
				_ => None,
			}
		}),
	)
}

fn construct_extrinsic(
	sender: pezsp_keyring::Sr25519Keyring,
	call: RuntimeCall,
) -> UncheckedExtrinsic {
	let account_id = AccountId32::from(sender.public());
	let tx_ext: TxExtension = (
		(
			pezframe_system::AuthorizeCall::<Runtime>::new(),
			pezframe_system::CheckNonZeroSender::<Runtime>::new(),
			pezframe_system::CheckSpecVersion::<Runtime>::new(),
			pezframe_system::CheckTxVersion::<Runtime>::new(),
			pezframe_system::CheckGenesis::<Runtime>::new(),
			pezframe_system::CheckEra::<Runtime>::from(Era::immortal()),
			pezframe_system::CheckNonce::<Runtime>::from(
				pezframe_system::Pezpallet::<Runtime>::account(&account_id).nonce,
			),
			pezframe_system::CheckWeight::<Runtime>::new(),
		),
		pezpallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
		BridgeRejectObsoleteHeadersAndMessages::default(),
		(OnBridgeHubPezkuwichainRefundBridgeHubZagrosMessages::default(),),
		pezframe_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
	)
		.into();
	let payload = SignedPayload::new(call.clone(), tx_ext.clone()).unwrap();
	let signature = payload.using_encoded(|e| sender.sign(e));
	UncheckedExtrinsic::new_signed(call, account_id.into(), Signature::Sr25519(signature), tx_ext)
}

fn construct_and_apply_extrinsic(
	origin: pezsp_keyring::Sr25519Keyring,
	call: RuntimeCall,
) -> pezsp_runtime::DispatchOutcome {
	let xt = construct_extrinsic(origin, call);
	let r = Executive::apply_extrinsic(xt);
	r.unwrap()
}
