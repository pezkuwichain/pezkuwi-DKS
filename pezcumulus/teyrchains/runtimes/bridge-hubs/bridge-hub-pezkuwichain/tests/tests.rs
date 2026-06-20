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
use hex_literal::hex;
use pezbp_pezkuwi_core::Signature;
use pezbridge_hub_pezkuwichain_runtime::{
	bridge_common_config, bridge_to_bulletin_config, bridge_to_zagros_config,
	xcm_config::{RelayNetwork, TokenLocation, XcmConfig},
	AllPalletsWithoutSystem, Block, BridgeRejectObsoleteHeadersAndMessages, Executive,
	ExistentialDeposit, PezkuwiXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, SessionKeys,
	TeyrchainSystem, TransactionPayment, TxExtension, UncheckedExtrinsic,
};
use pezbridge_hub_test_utils::{GovernanceOrigin, SlotDurations};
use pezframe_support::{dispatch::GetDispatchInfo, parameter_types, traits::ConstU8};
use pezsnowbridge_core::ChannelId;
use pezsp_consensus_aura::SlotDuration;
use pezsp_core::{crypto::Ss58Codec, H160};
use pezsp_keyring::Sr25519Keyring::Alice;
use pezsp_runtime::{
	generic::{Era, SignedPayload},
	AccountId32, Perbill,
};
use testnet_teyrchains_constants::pezkuwichain::{consensus::*, fee::WeightToFee};
use teyrchains_common::{AccountId, AuraId, Balance};
use teyrchains_runtimes_test_utils::ExtBuilder;
use xcm::latest::{prelude::*, PEZKUWICHAIN_GENESIS_HASH, ZAGROS_GENESIS_HASH};
use xcm_runtime_pezapis::conversions::LocationToAccountHelper;

parameter_types! {
	pub Governance: GovernanceOrigin<RuntimeOrigin> = GovernanceOrigin::Location(Location::parent());
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
		(bridge_to_zagros_config::OnBridgeHubPezkuwichainRefundBridgeHubZagrosMessages::default(),),
		pezframe_metadata_hash_extension::CheckMetadataHash::new(false),
	)
		.into();
	let payload = SignedPayload::new(call.clone(), tx_ext.clone()).unwrap();
	let signature = payload.using_encoded(|e| sender.sign(e));
	UncheckedExtrinsic::new_signed(call, account_id.into(), Signature::Sr25519(signature), tx_ext)
}

fn construct_and_apply_extrinsic(
	relayer_at_target: pezsp_keyring::Sr25519Keyring,
	call: RuntimeCall,
) -> pezsp_runtime::DispatchOutcome {
	let xt = construct_extrinsic(relayer_at_target, call);
	let r = Executive::apply_extrinsic(xt);
	r.unwrap()
}

fn construct_and_estimate_extrinsic_fee(call: RuntimeCall) -> Balance {
	let info = call.get_dispatch_info();
	let xt = construct_extrinsic(Alice, call);
	TransactionPayment::compute_fee(xt.encoded_size() as _, &info, 0)
}

fn collator_session_keys() -> pezbridge_hub_test_utils::CollatorSessionKeys<Runtime> {
	pezbridge_hub_test_utils::CollatorSessionKeys::new(
		AccountId::from(Alice),
		AccountId::from(Alice),
		SessionKeys { aura: AuraId::from(Alice.public()) },
	)
}

fn slot_durations() -> SlotDurations {
	SlotDurations {
		relay: SlotDuration::from_millis(RELAY_CHAIN_SLOT_DURATION_MILLIS.into()),
		para: SlotDuration::from_millis(SLOT_DURATION),
	}
}

pezbridge_hub_test_utils::test_cases::include_teleports_for_native_asset_works!(
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	(),
	WeightToFee,
	TeyrchainSystem,
	collator_session_keys(),
	slot_durations(),
	ExistentialDeposit::get(),
	Box::new(|runtime_event_encoded: Vec<u8>| {
		match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
			Ok(RuntimeEvent::PezkuwiXcm(event)) => Some(event),
			_ => None,
		}
	}),
	pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID
);

mod bridge_hub_zagros_tests {
	use super::*;
	use bridge_common_config::{
		BridgeGrandpaZagrosInstance, BridgeTeyrchainZagrosInstance, DeliveryRewardInBalance,
		RelayersForLegacyLaneIdsMessagesInstance,
	};
	use bridge_to_zagros_config::{
		BridgeHubZagrosLocation, WithBridgeHubZagrosMessagesInstance,
		XcmOverBridgeHubZagrosInstance, ZagrosGlobalConsensusNetwork,
	};
	use pezbp_messages::LegacyLaneId;
	use pezbridge_hub_pezkuwichain_runtime::{
		bridge_to_ethereum_config::EthereumGatewayAddress, xcm_config::LocationToAccountId,
	};
	use pezbridge_hub_test_utils::test_cases::from_teyrchain;
	use pezcumulus_primitives_core::UpwardMessageSender;

	// Random para id of sibling chain used in tests.
	pub const SIBLING_TEYRCHAIN_ID: u32 = 2053;
	// Random para id of bridged chain from different global consensus used in tests.
	pub const BRIDGED_LOCATION_TEYRCHAIN_ID: u32 = 1075;

	parameter_types! {
		pub SiblingTeyrchainLocation: Location = Location::new(1, [Teyrchain(SIBLING_TEYRCHAIN_ID)]);
		pub BridgedUniversalLocation: InteriorLocation = [GlobalConsensus(ZagrosGlobalConsensusNetwork::get()), Teyrchain(BRIDGED_LOCATION_TEYRCHAIN_ID)].into();
	}

	// Runtime from tests PoV
	type RuntimeTestsAdapter = from_teyrchain::WithRemoteTeyrchainHelperAdapter<
		Runtime,
		AllPalletsWithoutSystem,
		BridgeGrandpaZagrosInstance,
		BridgeTeyrchainZagrosInstance,
		WithBridgeHubZagrosMessagesInstance,
		RelayersForLegacyLaneIdsMessagesInstance,
	>;

	#[test]
	fn initialize_bridge_by_governance_works() {
		// for PezkuwichainBulletin finality
		pezbridge_hub_test_utils::test_cases::initialize_bridge_by_governance_works::<
			Runtime,
			BridgeGrandpaZagrosInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
		)
	}

	#[test]
	fn change_bridge_grandpa_pallet_mode_by_governance_works() {
		// for Zagros finality
		pezbridge_hub_test_utils::test_cases::change_bridge_grandpa_pallet_mode_by_governance_works::<
			Runtime,
			BridgeGrandpaZagrosInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
		)
	}

	#[test]
	fn change_bridge_teyrchains_pallet_mode_by_governance_works() {
		// for Zagros finality
		pezbridge_hub_test_utils::test_cases::change_bridge_teyrchains_pallet_mode_by_governance_works::<
			Runtime,
			BridgeTeyrchainZagrosInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
		)
	}

	#[test]
	fn change_bridge_messages_pallet_mode_by_governance_works() {
		// for Zagros finality
		pezbridge_hub_test_utils::test_cases::change_bridge_messages_pallet_mode_by_governance_works::<
			Runtime,
			WithBridgeHubZagrosMessagesInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
		)
	}

	#[test]
	fn change_ethereum_gateway_by_governance_works() {
		pezbridge_hub_test_utils::test_cases::change_storage_constant_by_governance_works::<
			Runtime,
			EthereumGatewayAddress,
			H160,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
			|| (EthereumGatewayAddress::key().to_vec(), EthereumGatewayAddress::get()),
			|_| [1; 20].into(),
		)
	}

	#[test]
	fn change_ethereum_nonces_by_governance_works() {
		let channel_id_one: ChannelId = [1; 32].into();
		let channel_id_two: ChannelId = [2; 32].into();
		let nonce = 42;

		// Reset a single inbound channel
		pezbridge_hub_test_utils::test_cases::set_storage_keys_by_governance_works::<Runtime>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
			vec![
				(
					pezsnowbridge_pezpallet_outbound_queue::Nonce::<Runtime>::hashed_key_for::<
						ChannelId,
					>(channel_id_one)
					.to_vec(),
					0u64.encode(),
				),
				(
					pezsnowbridge_pezpallet_inbound_queue::Nonce::<Runtime>::hashed_key_for::<
						ChannelId,
					>(channel_id_one)
					.to_vec(),
					0u64.encode(),
				),
			],
			|| {
				// Outbound
				pezsnowbridge_pezpallet_outbound_queue::Nonce::<Runtime>::insert::<ChannelId, u64>(
					channel_id_one,
					nonce,
				);
				pezsnowbridge_pezpallet_outbound_queue::Nonce::<Runtime>::insert::<ChannelId, u64>(
					channel_id_two,
					nonce,
				);

				// Inbound
				pezsnowbridge_pezpallet_inbound_queue::Nonce::<Runtime>::insert::<ChannelId, u64>(
					channel_id_one,
					nonce,
				);
				pezsnowbridge_pezpallet_inbound_queue::Nonce::<Runtime>::insert::<ChannelId, u64>(
					channel_id_two,
					nonce,
				);
			},
			|| {
				// Outbound
				assert_eq!(
					pezsnowbridge_pezpallet_outbound_queue::Nonce::<Runtime>::get(channel_id_one),
					0
				);
				assert_eq!(
					pezsnowbridge_pezpallet_outbound_queue::Nonce::<Runtime>::get(channel_id_two),
					nonce
				);

				// Inbound
				assert_eq!(
					pezsnowbridge_pezpallet_inbound_queue::Nonce::<Runtime>::get(channel_id_one),
					0
				);
				assert_eq!(
					pezsnowbridge_pezpallet_inbound_queue::Nonce::<Runtime>::get(channel_id_two),
					nonce
				);
			},
		);
	}

	#[test]
	fn change_delivery_reward_by_governance_works() {
		pezbridge_hub_test_utils::test_cases::change_storage_constant_by_governance_works::<
			Runtime,
			DeliveryRewardInBalance,
			u64,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
			|| (DeliveryRewardInBalance::key().to_vec(), DeliveryRewardInBalance::get()),
			|old_value| old_value.checked_mul(2).unwrap(),
		)
	}

	#[test]
	fn handle_export_message_from_system_teyrchain_add_to_outbound_queue_works() {
		// for Zagros
		pezbridge_hub_test_utils::test_cases::handle_export_message_from_system_teyrchain_to_outbound_queue_works::<
			Runtime,
			XcmConfig,
			WithBridgeHubZagrosMessagesInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			SIBLING_TEYRCHAIN_ID,
			Box::new(|runtime_event_encoded: Vec<u8>| {
				match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
					Ok(RuntimeEvent::BridgeZagrosMessages(event)) => Some(event),
					_ => None,
				}
			}),
			|| ExportMessage { network: ZagrosGlobalConsensusNetwork::get(), destination: [Teyrchain(BRIDGED_LOCATION_TEYRCHAIN_ID)].into(), xcm: Xcm(vec![]) },
			Some((TokenLocation::get(), ExistentialDeposit::get()).into()),
			// value should be >= than value generated by `can_calculate_weight_for_paid_export_message_with_reserve_transfer`
			Some((TokenLocation::get(), pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichainBaseXcmFeeInRocs::get()).into()),
			|| {
				PezkuwiXcm::force_xcm_version(RuntimeOrigin::root(), Box::new(BridgeHubZagrosLocation::get()), XCM_VERSION).expect("version saved!");

				// we need to create lane between sibling teyrchain and remote destination
				pezbridge_hub_test_utils::ensure_opened_bridge::<
					Runtime,
					XcmOverBridgeHubZagrosInstance,
					LocationToAccountId,
					TokenLocation,
				>(
					SiblingTeyrchainLocation::get(),
					BridgedUniversalLocation::get(),
					false,
					|locations, _fee| {
						pezbridge_hub_test_utils::open_bridge_with_storage::<
							Runtime,
							XcmOverBridgeHubZagrosInstance
						>(locations, LegacyLaneId([0, 0, 0, 1]))
					}
				).1
			},
		)
	}

	#[test]
	fn message_dispatch_routing_works() {
		// from Zagros
		pezbridge_hub_test_utils::test_cases::message_dispatch_routing_works::<
			Runtime,
			AllPalletsWithoutSystem,
			XcmConfig,
			TeyrchainSystem,
			WithBridgeHubZagrosMessagesInstance,
			RelayNetwork,
			ZagrosGlobalConsensusNetwork,
			ConstU8<2>,
		>(
			collator_session_keys(),
			slot_durations(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			SIBLING_TEYRCHAIN_ID,
			Box::new(|runtime_event_encoded: Vec<u8>| {
				match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
					Ok(RuntimeEvent::TeyrchainSystem(event)) => Some(event),
					_ => None,
				}
			}),
			Box::new(|runtime_event_encoded: Vec<u8>| {
				match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
					Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
					_ => None,
				}
			}),
			|| <TeyrchainSystem as UpwardMessageSender>::ensure_successful_delivery(),
		)
	}

	#[test]
	fn relayed_incoming_message_works() {
		// from Zagros
		from_teyrchain::relayed_incoming_message_works::<RuntimeTestsAdapter>(
			collator_session_keys(),
			slot_durations(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
			SIBLING_TEYRCHAIN_ID,
			ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
			|| {
				// we need to create lane between sibling teyrchain and remote destination
				pezbridge_hub_test_utils::ensure_opened_bridge::<
					Runtime,
					XcmOverBridgeHubZagrosInstance,
					LocationToAccountId,
					TokenLocation,
				>(
					SiblingTeyrchainLocation::get(),
					BridgedUniversalLocation::get(),
					false,
					|locations, _fee| {
						pezbridge_hub_test_utils::open_bridge_with_storage::<
							Runtime,
							XcmOverBridgeHubZagrosInstance,
						>(locations, LegacyLaneId([0, 0, 0, 1]))
					},
				)
				.1
			},
			construct_and_apply_extrinsic,
			true,
		)
	}

	#[test]
	fn free_relay_extrinsic_works() {
		// from Zagros
		from_teyrchain::free_relay_extrinsic_works::<RuntimeTestsAdapter>(
			collator_session_keys(),
			slot_durations(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
			SIBLING_TEYRCHAIN_ID,
			ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
			|| {
				// we need to create lane between sibling teyrchain and remote destination
				pezbridge_hub_test_utils::ensure_opened_bridge::<
					Runtime,
					XcmOverBridgeHubZagrosInstance,
					LocationToAccountId,
					TokenLocation,
				>(
					SiblingTeyrchainLocation::get(),
					BridgedUniversalLocation::get(),
					false,
					|locations, _fee| {
						pezbridge_hub_test_utils::open_bridge_with_storage::<
							Runtime,
							XcmOverBridgeHubZagrosInstance,
						>(locations, LegacyLaneId([0, 0, 0, 1]))
					},
				)
				.1
			},
			construct_and_apply_extrinsic,
			false,
		)
	}

	#[test]
	pub fn can_calculate_weight_for_paid_export_message_with_reserve_transfer() {
		pezbridge_hub_test_utils::check_sane_fees_values(
			"pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichainBaseXcmFeeInRocs",
			pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichainBaseXcmFeeInRocs::get(),
			|| {
				pezbridge_hub_test_utils::test_cases::can_calculate_weight_for_paid_export_message_with_reserve_transfer::<
					Runtime,
					XcmConfig,
					WeightToFee,
				>()
			},
			Perbill::from_percent(25),
			Some(-25),
			&format!(
				"Estimate fee for `ExportMessage` for runtime: {:?}",
				<Runtime as pezframe_system::Config>::Version::get()
			),
		)
	}

	#[test]
	fn can_calculate_fee_for_standalone_message_delivery_transaction() {
		pezbridge_hub_test_utils::check_sane_fees_values(
			"pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichainBaseDeliveryFeeInRocs",
			pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichainBaseDeliveryFeeInRocs::get(),
			|| {
				from_teyrchain::can_calculate_fee_for_standalone_message_delivery_transaction::<
					RuntimeTestsAdapter,
				>(collator_session_keys(), construct_and_estimate_extrinsic_fee)
			},
			Perbill::from_percent(25),
			Some(-25),
			&format!(
				"Estimate fee for `single message delivery` for runtime: {:?}",
				<Runtime as pezframe_system::Config>::Version::get()
			),
		)
	}

	#[test]
	fn can_calculate_fee_for_standalone_message_confirmation_transaction() {
		pezbridge_hub_test_utils::check_sane_fees_values(
			"pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichainBaseConfirmationFeeInRocs",
			pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichainBaseConfirmationFeeInRocs::get(),
			|| {
				from_teyrchain::can_calculate_fee_for_standalone_message_confirmation_transaction::<
					RuntimeTestsAdapter,
				>(collator_session_keys(), construct_and_estimate_extrinsic_fee)
			},
			Perbill::from_percent(25),
			Some(-25),
			&format!(
				"Estimate fee for `single message confirmation` for runtime: {:?}",
				<Runtime as pezframe_system::Config>::Version::get()
			),
		)
	}
}

mod bridge_hub_bulletin_tests {
	use super::*;
	use bridge_common_config::BridgeGrandpaPezkuwichainBulletinInstance;
	use bridge_to_bulletin_config::{
		PezkuwichainBulletinGlobalConsensusNetwork,
		PezkuwichainBulletinGlobalConsensusNetworkLocation,
		WithPezkuwichainBulletinMessagesInstance, XcmOverPezkuwiBulletinInstance,
	};
	use pezbp_messages::LegacyLaneId;
	use pezbridge_hub_pezkuwichain_runtime::{
		bridge_common_config::RelayersForLegacyLaneIdsMessagesInstance,
		xcm_config::LocationToAccountId,
	};
	use pezbridge_hub_test_utils::test_cases::from_grandpa_chain;
	use pezcumulus_primitives_core::UpwardMessageSender;

	// Random para id of sibling chain used in tests.
	pub const SIBLING_PEOPLE_TEYRCHAIN_ID: u32 =
		pezkuwichain_runtime_constants::system_teyrchain::PEOPLE_ID;

	parameter_types! {
																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																					pub SiblingPeopleTeyrchainLocation: Location = Location::new(1, [Teyrchain(SIBLING_PEOPLE_TEYRCHAIN_ID)]);
																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																																					pub BridgedBulletinLocation: InteriorLocation = [GlobalConsensus(PezkuwichainBulletinGlobalConsensusNetwork::get())].into();
	}

	// Runtime from tests PoV
	type RuntimeTestsAdapter = from_grandpa_chain::WithRemoteGrandpaChainHelperAdapter<
		Runtime,
		AllPalletsWithoutSystem,
		BridgeGrandpaPezkuwichainBulletinInstance,
		WithPezkuwichainBulletinMessagesInstance,
		RelayersForLegacyLaneIdsMessagesInstance,
	>;

	#[test]
	fn initialize_bridge_by_governance_works() {
		// for Bulletin finality
		pezbridge_hub_test_utils::test_cases::initialize_bridge_by_governance_works::<
			Runtime,
			BridgeGrandpaPezkuwichainBulletinInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
		)
	}

	#[test]
	fn change_bridge_grandpa_pallet_mode_by_governance_works() {
		// for Bulletin finality
		pezbridge_hub_test_utils::test_cases::change_bridge_grandpa_pallet_mode_by_governance_works::<
			Runtime,
			BridgeGrandpaPezkuwichainBulletinInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
		)
	}

	#[test]
	fn change_bridge_messages_pallet_mode_by_governance_works() {
		// for Bulletin finality
		pezbridge_hub_test_utils::test_cases::change_bridge_messages_pallet_mode_by_governance_works::<
			Runtime,
			WithPezkuwichainBulletinMessagesInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			Governance::get(),
		)
	}

	#[test]
	fn handle_export_message_from_system_teyrchain_add_to_outbound_queue_works() {
		// for Bulletin
		pezbridge_hub_test_utils::test_cases::handle_export_message_from_system_teyrchain_to_outbound_queue_works::<
			Runtime,
			XcmConfig,
			WithPezkuwichainBulletinMessagesInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			SIBLING_PEOPLE_TEYRCHAIN_ID,
			Box::new(|runtime_event_encoded: Vec<u8>| {
				match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
					Ok(RuntimeEvent::BridgePezkuwiBulletinMessages(event)) => Some(event),
					_ => None,
				}
			}),
			|| ExportMessage {
				network: PezkuwichainBulletinGlobalConsensusNetwork::get(),
				destination: Here,
				xcm: Xcm(vec![]),
			},
			Some((TokenLocation::get(), ExistentialDeposit::get()).into()),
			None,
			|| {
				PezkuwiXcm::force_xcm_version(RuntimeOrigin::root(), Box::new(PezkuwichainBulletinGlobalConsensusNetworkLocation::get()), XCM_VERSION).expect("version saved!");

				// we need to create lane between PezkuwichainPeople and PezkuwichainBulletin
				pezbridge_hub_test_utils::ensure_opened_bridge::<
					Runtime,
					XcmOverPezkuwiBulletinInstance,
					LocationToAccountId,
					TokenLocation,
				>(
					SiblingPeopleTeyrchainLocation::get(),
					BridgedBulletinLocation::get(),
					false,
					|locations, _fee| {
						pezbridge_hub_test_utils::open_bridge_with_storage::<
							Runtime,
							XcmOverPezkuwiBulletinInstance
						>(locations, LegacyLaneId([0, 0, 0, 0]))
					}
				).1
			},
		)
	}

	#[test]
	fn message_dispatch_routing_works() {
		// from Bulletin
		pezbridge_hub_test_utils::test_cases::message_dispatch_routing_works::<
			Runtime,
			AllPalletsWithoutSystem,
			XcmConfig,
			TeyrchainSystem,
			WithPezkuwichainBulletinMessagesInstance,
			RelayNetwork,
			PezkuwichainBulletinGlobalConsensusNetwork,
			ConstU8<2>,
		>(
			collator_session_keys(),
			slot_durations(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			SIBLING_PEOPLE_TEYRCHAIN_ID,
			Box::new(|runtime_event_encoded: Vec<u8>| {
				match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
					Ok(RuntimeEvent::TeyrchainSystem(event)) => Some(event),
					_ => None,
				}
			}),
			Box::new(|runtime_event_encoded: Vec<u8>| {
				match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
					Ok(RuntimeEvent::XcmpQueue(event)) => Some(event),
					_ => None,
				}
			}),
			|| <TeyrchainSystem as UpwardMessageSender>::ensure_successful_delivery(),
		)
	}

	#[test]
	fn relayed_incoming_message_works() {
		// from Bulletin
		from_grandpa_chain::relayed_incoming_message_works::<RuntimeTestsAdapter>(
			collator_session_keys(),
			slot_durations(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			SIBLING_PEOPLE_TEYRCHAIN_ID,
			ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
			|| {
				// we need to create lane between PezkuwichainPeople and PezkuwichainBulletin
				pezbridge_hub_test_utils::ensure_opened_bridge::<
					Runtime,
					XcmOverPezkuwiBulletinInstance,
					LocationToAccountId,
					TokenLocation,
				>(
					SiblingPeopleTeyrchainLocation::get(),
					BridgedBulletinLocation::get(),
					false,
					|locations, _fee| {
						pezbridge_hub_test_utils::open_bridge_with_storage::<
							Runtime,
							XcmOverPezkuwiBulletinInstance,
						>(locations, LegacyLaneId([0, 0, 0, 0]))
					},
				)
				.1
			},
			construct_and_apply_extrinsic,
			false,
		)
	}

	#[test]
	fn free_relay_extrinsic_works() {
		// from Bulletin
		from_grandpa_chain::free_relay_extrinsic_works::<RuntimeTestsAdapter>(
			collator_session_keys(),
			slot_durations(),
			pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
			SIBLING_PEOPLE_TEYRCHAIN_ID,
			ByGenesis(PEZKUWICHAIN_GENESIS_HASH),
			|| {
				// we need to create lane between PezkuwichainPeople and PezkuwichainBulletin
				pezbridge_hub_test_utils::ensure_opened_bridge::<
					Runtime,
					XcmOverPezkuwiBulletinInstance,
					LocationToAccountId,
					TokenLocation,
				>(
					SiblingPeopleTeyrchainLocation::get(),
					BridgedBulletinLocation::get(),
					false,
					|locations, _fee| {
						pezbridge_hub_test_utils::open_bridge_with_storage::<
							Runtime,
							XcmOverPezkuwiBulletinInstance,
						>(locations, LegacyLaneId([0, 0, 0, 0]))
					},
				)
				.1
			},
			construct_and_apply_extrinsic,
			false,
		)
	}
}

#[test]
fn change_required_stake_by_governance_works() {
	pezbridge_hub_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		bridge_common_config::RequiredStakeForStakeAndSlash,
		Balance,
	>(
		collator_session_keys(),
		pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
		Governance::get(),
		|| {
			(
				bridge_common_config::RequiredStakeForStakeAndSlash::key().to_vec(),
				bridge_common_config::RequiredStakeForStakeAndSlash::get(),
			)
		},
		|old_value| old_value.checked_mul(2).unwrap(),
	)
}

#[test]
fn location_conversion_works() {
	// the purpose of hardcoded values is to catch an unintended location conversion logic
	// change.
	struct TestCase {
		description: &'static str,
		location: Location,
		expected_account_id_str: &'static str,
	}

	let test_cases = vec![
		// DescribeTerminus
		TestCase {
			description: "DescribeTerminus Parent",
			location: Location::new(1, Here),
			expected_account_id_str: "5Dt6dpkWPwLaH4BBCKJwjiWrFVAGyYk3tLUabvyn4v7KtESG",
		},
		TestCase {
			description: "DescribeTerminus Sibling",
			location: Location::new(1, [Teyrchain(1111)]),
			expected_account_id_str: "5Eg2fnssmmJnF3z1iZ1NouAuzciDaaDQH7qURAy3w15jULDk",
		},
		// DescribePalletTerminal
		TestCase {
			description: "DescribePalletTerminal Parent",
			location: Location::new(1, [PalletInstance(50)]),
			expected_account_id_str: "5CnwemvaAXkWFVwibiCvf2EjqwiqBi29S5cLLydZLEaEw6jZ",
		},
		TestCase {
			description: "DescribePalletTerminal Sibling",
			location: Location::new(1, [Teyrchain(1111), PalletInstance(50)]),
			expected_account_id_str: "5GFBgPjpEQPdaxEnFirUoa51u5erVx84twYxJVuBRAT2UP2g",
		},
		// DescribeAccountId32Terminal
		TestCase {
			description: "DescribeAccountId32Terminal Parent",
			location: Location::new(
				1,
				[Junction::AccountId32 { network: None, id: AccountId::from(Alice).into() }],
			),
			expected_account_id_str: "5EueAXd4h8u75nSbFdDJbC29cmi4Uo1YJssqEL9idvindxFL",
		},
		TestCase {
			description: "DescribeAccountId32Terminal Sibling",
			location: Location::new(
				1,
				[
					Teyrchain(1111),
					Junction::AccountId32 { network: None, id: AccountId::from(Alice).into() },
				],
			),
			expected_account_id_str: "5Dmbuiq48fU4iW58FKYqoGbbfxFHjbAeGLMtjFg6NNCw3ssr",
		},
		// DescribeAccountKey20Terminal
		TestCase {
			description: "DescribeAccountKey20Terminal Parent",
			location: Location::new(1, [AccountKey20 { network: None, key: [0u8; 20] }]),
			expected_account_id_str: "5F5Ec11567pa919wJkX6VHtv2ZXS5W698YCW35EdEbrg14cg",
		},
		TestCase {
			description: "DescribeAccountKey20Terminal Sibling",
			location: Location::new(
				1,
				[Teyrchain(1111), AccountKey20 { network: None, key: [0u8; 20] }],
			),
			expected_account_id_str: "5CB2FbUds2qvcJNhDiTbRZwiS3trAy6ydFGMSVutmYijpPAg",
		},
		// DescribeTreasuryVoiceTerminal
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Treasury, part: BodyPart::Voice }]),
			expected_account_id_str: "5CUjnE2vgcUCuhxPwFoQ5r7p1DkhujgvMNDHaF2bLqRp4D5F",
		},
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Sibling",
			location: Location::new(
				1,
				[Teyrchain(1111), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5G6TDwaVgbWmhqRUKjBhRRnH4ry9L9cjRymUEmiRsLbSE4gB",
		},
		// DescribeBodyTerminal
		TestCase {
			description: "DescribeBodyTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Unit, part: BodyPart::Voice }]),
			expected_account_id_str: "5EBRMTBkDisEXsaN283SRbzx9Xf2PXwUxxFCJohSGo4jYe6B",
		},
		TestCase {
			description: "DescribeBodyTerminal Sibling",
			location: Location::new(
				1,
				[Teyrchain(1111), Plurality { id: BodyId::Unit, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5DBoExvojy8tYnHgLL97phNH975CyT45PWTZEeGoBZfAyRMH",
		},
		// ExternalConsensusLocationsConverterFor
		TestCase {
			description: "Describe Ethereum Location",
			location: Location::new(2, [GlobalConsensus(Ethereum { chain_id: 11155111 })]),
			expected_account_id_str: "5GjRnmh5o3usSYzVmsxBWzHEpvJyHK4tKNPhjpUR3ASrruBy",
		},
		TestCase {
			description: "Describe Ethereum AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: 11155111 }),
					AccountKey20 {
						network: None,
						key: hex!("87d1f7fdfEe7f651FaBc8bFCB6E086C278b77A7d"),
					},
				],
			),
			expected_account_id_str: "5HV4j4AsqT349oLRZmTjhGKDofPBWmWaPUfWGaRkuvzkjW9i",
		},
		TestCase {
			description: "Describe Zagros Location",
			location: Location::new(2, [GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH))]),
			expected_account_id_str: "5Fb4pyqFuYLZ43USEAcVUBhFTfTckG9zv9kUaVnmR79YgBCe",
		},
		TestCase {
			description: "Describe Zagros AccountID",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Junction::AccountId32 { network: None, id: AccountId::from(Alice).into() },
				],
			),
			expected_account_id_str: "5EEB1syXCCSEFk26ZYjH47WMp1QjYHf3q5zcnqWWY9Tr6gUc",
		},
		TestCase {
			description: "Describe Zagros AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					AccountKey20 { network: None, key: [0u8; 20] },
				],
			),
			expected_account_id_str: "5FzaTcFwUMyX5Sfe7wRGuc3zw1cbpGAGZpmAsxS4tBX6x6U3",
		},
		TestCase {
			description: "Describe Zagros Treasury Plurality",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Plurality { id: BodyId::Treasury, part: BodyPart::Voice },
				],
			),
			expected_account_id_str: "5CpdRCmCYwnxS1mifwEddYHDJR8ydDfTpi1gwAQKQvfAjjzu",
		},
		TestCase {
			description: "Describe Zagros Teyrchain Location",
			location: Location::new(
				2,
				[GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)), Teyrchain(1000)],
			),
			expected_account_id_str: "5CkWf1L181BiSbvoofnzfSg8ZLiBK3i1U4sknzETHk8QS2mA",
		},
		TestCase {
			description: "Describe Zagros Teyrchain AccountID",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Teyrchain(1000),
					Junction::AccountId32 { network: None, id: AccountId::from(Alice).into() },
				],
			),
			expected_account_id_str: "5HBG915qTKYWzqEs4VocHLCa7ftC7JfJCpvSxk6LmXWJvhbU",
		},
		TestCase {
			description: "Describe Zagros Teyrchain AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Teyrchain(1000),
					AccountKey20 { network: None, key: [0u8; 20] },
				],
			),
			expected_account_id_str: "5EFpSvq8BUAjdjY4tuGhGXZ66P16iQnX7nxsNoHy7TM6NhMa",
		},
		TestCase {
			description: "Describe Zagros Teyrchain Treasury Plurality",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Teyrchain(1000),
					Plurality { id: BodyId::Treasury, part: BodyPart::Voice },
				],
			),
			expected_account_id_str: "5GfwA4qaz9wpQPPHmf5MSKqvsPyrfx1yYeeZB1SUkqDuRuZ1",
		},
		TestCase {
			description: "Describe Zagros USDT Location",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)),
					Teyrchain(1000),
					PalletInstance(50),
					GeneralIndex(1984),
				],
			),
			expected_account_id_str: "5Hd77ZjbVRrYiRXER8qo9DRDB8ZzaKtRswZoypMnMLdixzMs",
		},
	];

	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_keys().collators())
		.with_session_keys(collator_session_keys().session_keys())
		.with_para_id(1000.into())
		.build()
		.execute_with(|| {
			for tc in test_cases {
				let expected = AccountId::from_string(tc.expected_account_id_str)
					.expect("Invalid AccountId string");

				let got = LocationToAccountHelper::<
					AccountId,
					pezbridge_hub_pezkuwichain_runtime::xcm_config::LocationToAccountId,
				>::convert_location(tc.location.into())
				.unwrap();

				assert_eq!(got, expected, "{}", tc.description);
			}
		});
}

#[test]
fn xcm_payment_api_works() {
	teyrchains_runtimes_test_utils::test_cases::xcm_payment_api_with_native_token_works::<
		Runtime,
		RuntimeCall,
		RuntimeOrigin,
		Block,
		WeightToFee,
	>();
}
