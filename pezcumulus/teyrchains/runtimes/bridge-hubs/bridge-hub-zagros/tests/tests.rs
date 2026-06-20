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

use crate::bridge_common_config::BridgeRewardBeneficiaries;
use bridge_common_config::{BridgeRelayersInstance, BridgeReward, RequiredStakeForStakeAndSlash};
use bridge_to_pezkuwichain_config::{
	BridgeGrandpaPezkuwichainInstance, BridgeHubPezkuwichainLocation,
	BridgeTeyrchainPezkuwichainInstance, DeliveryRewardInBalance,
	WithBridgeHubPezkuwichainMessagesInstance, XcmOverBridgeHubPezkuwichainInstance,
};
use codec::{Decode, Encode};
use hex_literal::hex;
use pezbp_messages::LegacyLaneId;
use pezbp_pezkuwi_core::Signature;
use pezbp_relayers::{PayRewardFromAccount, RewardsAccountOwner, RewardsAccountParams};
use pezbridge_hub_test_utils::{
	test_cases::{from_teyrchain, run_test},
	GovernanceOrigin, SlotDurations,
};
use pezbridge_hub_zagros_runtime::{
	bridge_common_config, bridge_to_pezkuwichain_config,
	bridge_to_pezkuwichain_config::PezkuwichainGlobalConsensusNetwork,
	xcm_config::{
		GovernanceLocation, LocationToAccountId, RelayNetwork, XcmConfig, ZagrosLocation,
	},
	AllPalletsWithoutSystem, Balances, Block, BridgeRejectObsoleteHeadersAndMessages,
	BridgeRelayers, Executive, ExistentialDeposit, PezkuwiXcm, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeOrigin, SessionKeys, TeyrchainSystem, TransactionPayment, TxExtension,
	UncheckedExtrinsic,
};
use pezcumulus_primitives_core::UpwardMessageSender;
use pezframe_support::{
	assert_err, assert_ok,
	dispatch::GetDispatchInfo,
	parameter_types,
	traits::{
		fungible::{Inspect, Mutate},
		ConstU8,
	},
};
use pezsp_consensus_aura::SlotDuration;
use pezsp_core::crypto::Ss58Codec;
use pezsp_keyring::Sr25519Keyring::{Alice, Bob};
use pezsp_runtime::{
	generic::{Era, SignedPayload},
	AccountId32, Either, Perbill,
};
use testnet_teyrchains_constants::zagros::{consensus::*, fee::WeightToFee};
use teyrchains_common::{AccountId, AuraId, Balance};
use teyrchains_runtimes_test_utils::ExtBuilder;
use xcm::{
	latest::{prelude::*, PEZKUWICHAIN_GENESIS_HASH, ZAGROS_GENESIS_HASH},
	VersionedLocation,
};
use xcm_runtime_pezapis::conversions::LocationToAccountHelper;

// Random para id of sibling chain used in tests.
pub const SIBLING_TEYRCHAIN_ID: u32 = 2053;
// Random para id of sibling chain used in tests.
pub const SIBLING_SYSTEM_TEYRCHAIN_ID: u32 = 1008;
// Random para id of bridged chain from different global consensus used in tests.
pub const BRIDGED_LOCATION_TEYRCHAIN_ID: u32 = 1075;

parameter_types! {
	pub SiblingTeyrchainLocation: Location = Location::new(1, [Teyrchain(SIBLING_TEYRCHAIN_ID)]);
	pub SiblingSystemTeyrchainLocation: Location = Location::new(1, [Teyrchain(SIBLING_SYSTEM_TEYRCHAIN_ID)]);
	pub BridgedUniversalLocation: InteriorLocation = [GlobalConsensus(PezkuwichainGlobalConsensusNetwork::get()), Teyrchain(BRIDGED_LOCATION_TEYRCHAIN_ID)].into();
	pub Governance: GovernanceOrigin<RuntimeOrigin> = GovernanceOrigin::Location(GovernanceLocation::get());
}

// Runtime from tests PoV
type RuntimeTestsAdapter = from_teyrchain::WithRemoteTeyrchainHelperAdapter<
	Runtime,
	AllPalletsWithoutSystem,
	BridgeGrandpaPezkuwichainInstance,
	BridgeTeyrchainPezkuwichainInstance,
	WithBridgeHubPezkuwichainMessagesInstance,
	BridgeRelayersInstance,
>;

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
		(bridge_to_pezkuwichain_config::OnBridgeHubZagrosRefundBridgeHubPezkuwichainMessages::default(),),
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
	pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID
);

#[test]
fn initialize_bridge_by_governance_works() {
	pezbridge_hub_test_utils::test_cases::initialize_bridge_by_governance_works::<
		Runtime,
		BridgeGrandpaPezkuwichainInstance,
	>(
		collator_session_keys(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
		Governance::get(),
	)
}

#[test]
fn change_bridge_grandpa_pallet_mode_by_governance_works() {
	pezbridge_hub_test_utils::test_cases::change_bridge_grandpa_pallet_mode_by_governance_works::<
		Runtime,
		BridgeGrandpaPezkuwichainInstance,
	>(
		collator_session_keys(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
		Governance::get(),
	)
}

#[test]
fn change_bridge_teyrchains_pallet_mode_by_governance_works() {
	pezbridge_hub_test_utils::test_cases::change_bridge_teyrchains_pallet_mode_by_governance_works::<
		Runtime,
		BridgeTeyrchainPezkuwichainInstance,
	>(
		collator_session_keys(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
		Governance::get(),
	)
}

#[test]
fn change_bridge_messages_pallet_mode_by_governance_works() {
	pezbridge_hub_test_utils::test_cases::change_bridge_messages_pallet_mode_by_governance_works::<
		Runtime,
		WithBridgeHubPezkuwichainMessagesInstance,
	>(
		collator_session_keys(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
		Governance::get(),
	)
}

#[test]
fn change_delivery_reward_by_governance_works() {
	pezbridge_hub_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		DeliveryRewardInBalance,
		u64,
	>(
		collator_session_keys(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
		Governance::get(),
		|| (DeliveryRewardInBalance::key().to_vec(), DeliveryRewardInBalance::get()),
		|old_value| old_value.checked_mul(2).unwrap(),
	)
}

#[test]
fn change_required_stake_by_governance_works() {
	pezbridge_hub_test_utils::test_cases::change_storage_constant_by_governance_works::<
		Runtime,
		RequiredStakeForStakeAndSlash,
		Balance,
	>(
		collator_session_keys(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
		Governance::get(),
		|| (RequiredStakeForStakeAndSlash::key().to_vec(), RequiredStakeForStakeAndSlash::get()),
		|old_value| old_value.checked_mul(2).unwrap(),
	)
}

#[test]
fn handle_export_message_from_system_teyrchain_add_to_outbound_queue_works() {
	pezbridge_hub_test_utils::test_cases::handle_export_message_from_system_teyrchain_to_outbound_queue_works::<
			Runtime,
			XcmConfig,
			WithBridgeHubPezkuwichainMessagesInstance,
		>(
			collator_session_keys(),
			pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
			SIBLING_TEYRCHAIN_ID,
			Box::new(|runtime_event_encoded: Vec<u8>| {
				match RuntimeEvent::decode(&mut &runtime_event_encoded[..]) {
					Ok(RuntimeEvent::BridgePezkuwichainMessages(event)) => Some(event),
					_ => None,
				}
			}),
			|| ExportMessage { network: PezkuwichainGlobalConsensusNetwork::get(), destination: [Teyrchain(BRIDGED_LOCATION_TEYRCHAIN_ID)].into(), xcm: Xcm(vec![]) },
			Some((ZagrosLocation::get(), ExistentialDeposit::get()).into()),
			// value should be >= than value generated by `can_calculate_weight_for_paid_export_message_with_reserve_transfer`
			Some((ZagrosLocation::get(), pezbp_bridge_hub_zagros::BridgeHubZagrosBaseXcmFeeInWnds::get()).into()),
			|| {
				PezkuwiXcm::force_xcm_version(RuntimeOrigin::root(), Box::new(BridgeHubPezkuwichainLocation::get()), XCM_VERSION).expect("version saved!");

				// we need to create lane between sibling teyrchain and remote destination
				pezbridge_hub_test_utils::ensure_opened_bridge::<
					Runtime,
					XcmOverBridgeHubPezkuwichainInstance,
					LocationToAccountId,
					ZagrosLocation,
				>(
					SiblingTeyrchainLocation::get(),
					BridgedUniversalLocation::get(),
					false,
					|locations, _fee| {
						pezbridge_hub_test_utils::open_bridge_with_storage::<
							Runtime, XcmOverBridgeHubPezkuwichainInstance
						>(locations, LegacyLaneId([0, 0, 0, 1]))
					}
				).1
			},
		)
}

#[test]
fn message_dispatch_routing_works() {
	pezbridge_hub_test_utils::test_cases::message_dispatch_routing_works::<
		Runtime,
		AllPalletsWithoutSystem,
		XcmConfig,
		TeyrchainSystem,
		WithBridgeHubPezkuwichainMessagesInstance,
		RelayNetwork,
		bridge_to_pezkuwichain_config::PezkuwichainGlobalConsensusNetwork,
		ConstU8<2>,
	>(
		collator_session_keys(),
		slot_durations(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
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
	from_teyrchain::relayed_incoming_message_works::<RuntimeTestsAdapter>(
		collator_session_keys(),
		slot_durations(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
		pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
		SIBLING_TEYRCHAIN_ID,
		ByGenesis(ZAGROS_GENESIS_HASH),
		|| {
			// we need to create lane between sibling teyrchain and remote destination
			pezbridge_hub_test_utils::ensure_opened_bridge::<
				Runtime,
				XcmOverBridgeHubPezkuwichainInstance,
				LocationToAccountId,
				ZagrosLocation,
			>(
				SiblingTeyrchainLocation::get(),
				BridgedUniversalLocation::get(),
				false,
				|locations, _fee| {
					pezbridge_hub_test_utils::open_bridge_with_storage::<
						Runtime,
						XcmOverBridgeHubPezkuwichainInstance,
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
	// from Pezkuwichain
	from_teyrchain::free_relay_extrinsic_works::<RuntimeTestsAdapter>(
		collator_session_keys(),
		slot_durations(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
		pezbp_bridge_hub_pezkuwichain::BRIDGE_HUB_PEZKUWICHAIN_TEYRCHAIN_ID,
		SIBLING_TEYRCHAIN_ID,
		ByGenesis(ZAGROS_GENESIS_HASH),
		|| {
			// we need to create lane between sibling teyrchain and remote destination
			pezbridge_hub_test_utils::ensure_opened_bridge::<
				Runtime,
				XcmOverBridgeHubPezkuwichainInstance,
				LocationToAccountId,
				ZagrosLocation,
			>(
				SiblingTeyrchainLocation::get(),
				BridgedUniversalLocation::get(),
				false,
				|locations, _fee| {
					pezbridge_hub_test_utils::open_bridge_with_storage::<
						Runtime,
						XcmOverBridgeHubPezkuwichainInstance,
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
pub fn can_calculate_weight_for_paid_export_message_with_reserve_transfer() {
	pezbridge_hub_test_utils::check_sane_fees_values(
		"pezbp_bridge_hub_zagros::BridgeHubZagrosBaseXcmFeeInWnds",
		pezbp_bridge_hub_zagros::BridgeHubZagrosBaseXcmFeeInWnds::get(),
		|| {
			pezbridge_hub_test_utils::test_cases::can_calculate_weight_for_paid_export_message_with_reserve_transfer::<
			Runtime,
			XcmConfig,
			WeightToFee,
		>()
		},
		Perbill::from_percent(33),
		Some(-33),
		&format!(
			"Estimate fee for `ExportMessage` for runtime: {:?}",
			<Runtime as pezframe_system::Config>::Version::get()
		),
	)
}

#[test]
pub fn can_calculate_fee_for_standalone_message_delivery_transaction() {
	pezbridge_hub_test_utils::check_sane_fees_values(
		"pezbp_bridge_hub_zagros::BridgeHubZagrosBaseDeliveryFeeInWnds",
		pezbp_bridge_hub_zagros::BridgeHubZagrosBaseDeliveryFeeInWnds::get(),
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
pub fn can_calculate_fee_for_standalone_message_confirmation_transaction() {
	pezbridge_hub_test_utils::check_sane_fees_values(
		"pezbp_bridge_hub_zagros::BridgeHubZagrosBaseConfirmationFeeInWnds",
		pezbp_bridge_hub_zagros::BridgeHubZagrosBaseConfirmationFeeInWnds::get(),
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

#[test]
fn location_conversion_works() {
	// the purpose of hardcoded values is to catch an unintended location conversion logic change.
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
			description: "Describe Pezkuwichain Location",
			location: Location::new(2, [GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH))]),
			expected_account_id_str: "5FfpYGrFybJXFsQk7dabr1vEbQ5ycBBu85vrDjPJsF3q4A8P",
		},
		TestCase {
			description: "Describe Pezkuwichain AccountID",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					xcm::prelude::AccountId32 { network: None, id: AccountId::from(Alice).into() },
				],
			),
			expected_account_id_str: "5CYn32qPAc8FpQP55Br6AS2ZKhfCHD8Tt3v4CnCZo1rhDPd4",
		},
		TestCase {
			description: "Describe Pezkuwichain AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					AccountKey20 { network: None, key: [0u8; 20] },
				],
			),
			expected_account_id_str: "5GbRhbJWb2hZY7TCeNvTqZXaP3x3UY5xt4ccxpV1ZtJS1gFL",
		},
		TestCase {
			description: "Describe Pezkuwichain Treasury Plurality",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Plurality { id: BodyId::Treasury, part: BodyPart::Voice },
				],
			),
			expected_account_id_str: "5EGi9NgJNGoMawY8ubnCDLmbdEW6nt2W2U2G3j9E3jXmspT7",
		},
		TestCase {
			description: "Describe Pezkuwichain Teyrchain Location",
			location: Location::new(
				2,
				[GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)), Teyrchain(1000)],
			),
			expected_account_id_str: "5CQeLKM7XC1xNBiQLp26Wa948cudjYRD5VzvaTG3BjnmUvLL",
		},
		TestCase {
			description: "Describe Pezkuwichain Teyrchain AccountID",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Teyrchain(1000),
					xcm::prelude::AccountId32 { network: None, id: AccountId::from(Alice).into() },
				],
			),
			expected_account_id_str: "5CWnqmyXccGPg27BTxGmycvdEs5HvQq2FQY61xsS8H7uAvmW",
		},
		TestCase {
			description: "Describe Pezkuwichain Teyrchain AccountKey",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Teyrchain(1000),
					AccountKey20 { network: None, key: [0u8; 20] },
				],
			),
			expected_account_id_str: "5G121Rtddxn6zwMD2rZZGXxFHZ2xAgzFUgM9ki4A8wMGo4e2",
		},
		TestCase {
			description: "Describe Pezkuwichain Teyrchain Treasury Plurality",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Teyrchain(1000),
					Plurality { id: BodyId::Treasury, part: BodyPart::Voice },
				],
			),
			expected_account_id_str: "5FNk7za2pQ71NHnN1jA63hJxJwdQywiVGnK6RL3nYjCdkWDF",
		},
		TestCase {
			description: "Describe Pezkuwichain USDT Location",
			location: Location::new(
				2,
				[
					GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					Teyrchain(1000),
					PalletInstance(50),
					GeneralIndex(1984),
				],
			),
			expected_account_id_str: "5HNfT779KHeAL7PaVBTQDVxrT6dfJZJoQMTScxLSahBc9kxF",
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

				let got =
					LocationToAccountHelper::<AccountId, LocationToAccountId>::convert_location(
						tc.location.into(),
					)
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

#[test]
pub fn bridge_rewards_works() {
	run_test::<Runtime, _>(
		collator_session_keys(),
		pezbp_bridge_hub_zagros::BRIDGE_HUB_ZAGROS_TEYRCHAIN_ID,
		vec![],
		|| {
			// reward in WNDs
			let reward1: u128 = 2_000_000_000;
			// reward in WETH
			let reward2: u128 = 3_000_000_000;

			// prepare accounts
			let account1 = AccountId32::from(Alice);
			let account2 = AccountId32::from(Bob);
			let reward1_for = RewardsAccountParams::new(
				LegacyLaneId([1; 4]),
				*b"test",
				RewardsAccountOwner::ThisChain,
			);
			let expected_reward1_account =
				PayRewardFromAccount::<(), AccountId, LegacyLaneId, ()>::rewards_account(
					reward1_for,
				);
			assert_ok!(Balances::mint_into(&expected_reward1_account, ExistentialDeposit::get()));
			assert_ok!(Balances::mint_into(&expected_reward1_account, reward1.into()));
			assert_ok!(Balances::mint_into(&account1, ExistentialDeposit::get()));
			// To pay for delivery to AH when claiming the reward on BH
			assert_ok!(Balances::mint_into(&account2, ExistentialDeposit::get() * 10000));

			// register rewards
			use pezbp_relayers::RewardLedger;
			BridgeRelayers::register_reward(&account1, BridgeReward::from(reward1_for), reward1);
			BridgeRelayers::register_reward(&account2, BridgeReward::Snowbridge, reward2);

			// check stored rewards
			assert_eq!(
				BridgeRelayers::relayer_reward(&account1, BridgeReward::from(reward1_for)),
				Some(reward1)
			);
			assert_eq!(BridgeRelayers::relayer_reward(&account1, BridgeReward::Snowbridge), None,);
			assert_eq!(
				BridgeRelayers::relayer_reward(&account2, BridgeReward::Snowbridge),
				Some(reward2),
			);
			assert_eq!(
				BridgeRelayers::relayer_reward(&account2, BridgeReward::from(reward1_for)),
				None,
			);

			// claim rewards
			assert_ok!(BridgeRelayers::claim_rewards(
				RuntimeOrigin::signed(account1.clone()),
				reward1_for.into()
			));
			assert_eq!(Balances::total_balance(&account1), ExistentialDeposit::get() + reward1);
			assert_eq!(
				BridgeRelayers::relayer_reward(&account1, BridgeReward::from(reward1_for)),
				None,
			);

			// already claimed
			assert_err!(
				BridgeRelayers::claim_rewards(
					RuntimeOrigin::signed(account1.clone()),
					reward1_for.into()
				),
				pezpallet_bridge_relayers::Error::<Runtime, BridgeRelayersInstance>::NoRewardForRelayer
			);

			// Local account claiming is not supported for Snowbridge
			assert_err!(
				BridgeRelayers::claim_rewards(
					RuntimeOrigin::signed(account2.clone()),
					BridgeReward::Snowbridge
				),
				pezpallet_bridge_relayers::Error::<Runtime, BridgeRelayersInstance>::FailedToPayReward
			);

			let claim_location = VersionedLocation::V5(Location::new(
				1,
				[
					Teyrchain(1000),
					xcm::latest::Junction::AccountId32 {
						id: account2.clone().into(),
						network: None,
					},
				],
			));
			// In unit tests without proper HRMP channel setup, the claim will fail at XCM sending.
			assert_err!(
				BridgeRelayers::claim_rewards_to(
					RuntimeOrigin::signed(account2.clone()),
					BridgeReward::Snowbridge,
					BridgeRewardBeneficiaries::AssetHubLocation(claim_location)
				),
				pezpallet_bridge_relayers::Error::<Runtime, BridgeRelayersInstance>::FailedToPayReward
			);
		},
	);
}

#[test]
fn governance_authorize_upgrade_works() {
	use zagros_runtime_constants::system_teyrchain::{ASSET_HUB_ID, COLLECTIVES_ID};

	// no - random para
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Teyrchain(12334)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// ok - AssetHub
	assert_ok!(teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(Location::new(1, Teyrchain(ASSET_HUB_ID)))));
	// no - Collectives
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Teyrchain(COLLECTIVES_ID)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - Collectives Voice of Fellows plurality
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::LocationAndDescendOrigin(
			Location::new(1, Teyrchain(COLLECTIVES_ID)),
			Plurality { id: BodyId::Technical, part: BodyPart::Voice }.into()
		)),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);

	// ok - relaychain
	assert_ok!(teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(Location::parent())));

	// ok - governance location
	assert_ok!(teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(Governance::get()));
}
