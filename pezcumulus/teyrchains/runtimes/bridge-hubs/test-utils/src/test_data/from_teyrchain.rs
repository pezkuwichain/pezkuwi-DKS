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

//! Generating test data for bridges with remote teyrchains.

use super::{
	from_grandpa_chain::make_complex_bridged_grandpa_header_proof, prepare_inbound_xcm,
	XcmAsPlainPayload,
};

use codec::Encode;
use pezbp_messages::{
	source_chain::FromBridgedChainMessagesDeliveryProof,
	target_chain::FromBridgedChainMessagesProof, ChainWithMessages, LaneState,
	UnrewardedRelayersState, Weight,
};
use pezbp_runtime::{
	AccountIdOf, BlockNumberOf, Chain, HeaderOf, Teyrchain, UnverifiedStorageProofParams,
};
use pezbp_test_utils::prepare_teyrchain_heads_proof;
use pezbp_teyrchains::{RelayBlockHash, RelayBlockNumber};
use pezpallet_bridge_grandpa::BridgedHeader;
use pezsp_runtime::traits::Header as HeaderT;
use xcm::latest::prelude::*;

use crate::test_cases::helpers::InboundRelayerId;
use pezbp_header_pez_chain::{justification::GrandpaJustification, ChainWithGrandpa};
use pezbp_messages::{DeliveredMessages, InboundLaneData, MessageNonce, UnrewardedRelayer};
use pezbp_pezkuwi_core::teyrchains::{ParaHash, ParaHead, ParaHeadsProof, ParaId};
use pezpallet_bridge_messages::{
	messages_generation::{
		encode_all_messages, encode_lane_data, prepare_message_delivery_storage_proof,
		prepare_messages_storage_proof,
	},
	BridgedChainOf, LaneIdOf,
};
use pezsp_runtime::SaturatedConversion;

/// Prepare a batch call with relay finality proof, teyrchain head proof and message proof.
pub fn make_complex_relayer_delivery_batch<Runtime, GPI, PPI, MPI>(
	relay_chain_header: BridgedHeader<Runtime, GPI>,
	grandpa_justification: GrandpaJustification<BridgedHeader<Runtime, GPI>>,
	teyrchain_heads: Vec<(ParaId, ParaHash)>,
	para_heads_proof: ParaHeadsProof,
	message_proof: FromBridgedChainMessagesProof<ParaHash, LaneIdOf<Runtime, MPI>>,
	relayer_id_at_bridged_chain: InboundRelayerId<Runtime, MPI>,
) -> pezpallet_utility::Call<Runtime>
where
	Runtime: pezpallet_bridge_grandpa::Config<GPI>
		+ pezpallet_bridge_teyrchains::Config<PPI>
		+ pezpallet_bridge_messages::Config<MPI, InboundPayload = XcmAsPlainPayload>
		+ pezpallet_utility::Config,
	GPI: 'static,
	PPI: 'static,
	MPI: 'static,
	ParaHash: From<
		<<Runtime as pezpallet_bridge_grandpa::Config<GPI>>::BridgedChain as pezbp_runtime::Chain>::Hash,
	>,
	<<Runtime as pezpallet_bridge_grandpa::Config<GPI>>::BridgedChain as pezbp_runtime::Chain>::Hash:
		From<ParaHash>,
	BridgedChainOf<Runtime, MPI>: Chain<Hash = ParaHash> + Teyrchain,
	<Runtime as pezpallet_utility::Config>::RuntimeCall: From<pezpallet_bridge_grandpa::Call<Runtime, GPI>>
		+ From<pezpallet_bridge_teyrchains::Call<Runtime, PPI>>
		+ From<pezpallet_bridge_messages::Call<Runtime, MPI>>,
{
	let relay_chain_header_hash = relay_chain_header.hash();
	let relay_chain_header_number = *relay_chain_header.number();
	let submit_grandpa = pezpallet_bridge_grandpa::Call::<Runtime, GPI>::submit_finality_proof {
		finality_target: Box::new(relay_chain_header),
		justification: grandpa_justification,
	};
	let submit_para_head =
		pezpallet_bridge_teyrchains::Call::<Runtime, PPI>::submit_teyrchain_heads {
			at_relay_block: (
				relay_chain_header_number.saturated_into(),
				relay_chain_header_hash.into(),
			),
			teyrchains: teyrchain_heads,
			teyrchain_heads_proof: para_heads_proof,
		};
	let submit_message = pezpallet_bridge_messages::Call::<Runtime, MPI>::receive_messages_proof {
		relayer_id_at_bridged_chain: relayer_id_at_bridged_chain.into(),
		proof: Box::new(message_proof),
		messages_count: 1,
		dispatch_weight: Weight::from_parts(1000000000, 0),
	};
	pezpallet_utility::Call::<Runtime>::batch_all {
		calls: vec![submit_grandpa.into(), submit_para_head.into(), submit_message.into()],
	}
}

/// Prepare a batch call with relay finality proof, teyrchain head proof and message delivery
/// proof.
pub fn make_complex_relayer_confirmation_batch<Runtime, GPI, PPI, MPI>(
	relay_chain_header: BridgedHeader<Runtime, GPI>,
	grandpa_justification: GrandpaJustification<BridgedHeader<Runtime, GPI>>,
	teyrchain_heads: Vec<(ParaId, ParaHash)>,
	para_heads_proof: ParaHeadsProof,
	message_delivery_proof: FromBridgedChainMessagesDeliveryProof<ParaHash, LaneIdOf<Runtime, MPI>>,
	relayers_state: UnrewardedRelayersState,
) -> pezpallet_utility::Call<Runtime>
where
	Runtime: pezpallet_bridge_grandpa::Config<GPI>
		+ pezpallet_bridge_teyrchains::Config<PPI>
		+ pezpallet_bridge_messages::Config<MPI, OutboundPayload = XcmAsPlainPayload>
		+ pezpallet_utility::Config,
	GPI: 'static,
	PPI: 'static,
	MPI: 'static,
	<Runtime as pezpallet_bridge_grandpa::Config<GPI>>::BridgedChain: pezbp_runtime::Chain<Hash = RelayBlockHash, BlockNumber = RelayBlockNumber>
		+ ChainWithGrandpa,
	BridgedChainOf<Runtime, MPI>: Chain<Hash = ParaHash> + Teyrchain,
	<Runtime as pezpallet_utility::Config>::RuntimeCall: From<pezpallet_bridge_grandpa::Call<Runtime, GPI>>
		+ From<pezpallet_bridge_teyrchains::Call<Runtime, PPI>>
		+ From<pezpallet_bridge_messages::Call<Runtime, MPI>>,
{
	let relay_chain_header_hash = relay_chain_header.hash();
	let relay_chain_header_number = *relay_chain_header.number();
	let submit_grandpa = pezpallet_bridge_grandpa::Call::<Runtime, GPI>::submit_finality_proof {
		finality_target: Box::new(relay_chain_header),
		justification: grandpa_justification,
	};
	let submit_para_head =
		pezpallet_bridge_teyrchains::Call::<Runtime, PPI>::submit_teyrchain_heads {
			at_relay_block: (
				relay_chain_header_number.saturated_into(),
				relay_chain_header_hash.into(),
			),
			teyrchains: teyrchain_heads,
			teyrchain_heads_proof: para_heads_proof,
		};
	let submit_message_delivery_proof =
		pezpallet_bridge_messages::Call::<Runtime, MPI>::receive_messages_delivery_proof {
			proof: message_delivery_proof,
			relayers_state,
		};
	pezpallet_utility::Call::<Runtime>::batch_all {
		calls: vec![
			submit_grandpa.into(),
			submit_para_head.into(),
			submit_message_delivery_proof.into(),
		],
	}
}

/// Prepare a call with message proof.
pub fn make_standalone_relayer_delivery_call<Runtime, MPI>(
	message_proof: FromBridgedChainMessagesProof<ParaHash, LaneIdOf<Runtime, MPI>>,
	relayer_id_at_bridged_chain: InboundRelayerId<Runtime, MPI>,
) -> Runtime::RuntimeCall
where
	Runtime: pezpallet_bridge_messages::Config<MPI, InboundPayload = XcmAsPlainPayload>,
	MPI: 'static,
	Runtime::RuntimeCall: From<pezpallet_bridge_messages::Call<Runtime, MPI>>,
	BridgedChainOf<Runtime, MPI>: Chain<Hash = ParaHash> + Teyrchain,
{
	pezpallet_bridge_messages::Call::<Runtime, MPI>::receive_messages_proof {
		relayer_id_at_bridged_chain: relayer_id_at_bridged_chain.into(),
		proof: Box::new(message_proof),
		messages_count: 1,
		dispatch_weight: Weight::from_parts(1000000000, 0),
	}
	.into()
}

/// Prepare a call with message delivery proof.
pub fn make_standalone_relayer_confirmation_call<Runtime, MPI>(
	message_delivery_proof: FromBridgedChainMessagesDeliveryProof<ParaHash, LaneIdOf<Runtime, MPI>>,
	relayers_state: UnrewardedRelayersState,
) -> Runtime::RuntimeCall
where
	Runtime: pezpallet_bridge_messages::Config<MPI, OutboundPayload = XcmAsPlainPayload>,
	MPI: 'static,
	Runtime::RuntimeCall: From<pezpallet_bridge_messages::Call<Runtime, MPI>>,
	BridgedChainOf<Runtime, MPI>: Chain<Hash = ParaHash> + Teyrchain,
{
	pezpallet_bridge_messages::Call::<Runtime, MPI>::receive_messages_delivery_proof {
		proof: message_delivery_proof,
		relayers_state,
	}
	.into()
}

/// Prepare storage proofs of messages, stored at the source chain.
pub fn make_complex_relayer_delivery_proofs<
	BridgedRelayChain,
	BridgedTeyrchain,
	ThisChainWithMessages,
	LaneId,
>(
	lane_id: LaneId,
	xcm_message: Xcm<()>,
	message_nonce: MessageNonce,
	message_destination: Junctions,
	para_header_number: u32,
	relay_header_number: u32,
	bridged_para_id: u32,
	is_minimal_call: bool,
) -> (
	HeaderOf<BridgedRelayChain>,
	GrandpaJustification<HeaderOf<BridgedRelayChain>>,
	ParaHead,
	Vec<(ParaId, ParaHash)>,
	ParaHeadsProof,
	FromBridgedChainMessagesProof<ParaHash, LaneId>,
)
where
	BridgedRelayChain: pezbp_runtime::Chain<Hash = RelayBlockHash, BlockNumber = RelayBlockNumber>
		+ ChainWithGrandpa,
	BridgedTeyrchain: pezbp_runtime::Chain<Hash = ParaHash> + Teyrchain,
	ThisChainWithMessages: ChainWithMessages,
	LaneId: Copy + Encode,
{
	// prepare message
	let message_payload = prepare_inbound_xcm(xcm_message, message_destination);
	// prepare para storage proof containing message
	let (para_state_root, para_storage_proof) =
		prepare_messages_storage_proof::<BridgedTeyrchain, ThisChainWithMessages, LaneId>(
			lane_id,
			message_nonce..=message_nonce,
			None,
			UnverifiedStorageProofParams::from_db_size(message_payload.len() as u32),
			|_| message_payload.clone(),
			encode_all_messages,
			encode_lane_data,
			false,
			false,
		);

	let (relay_chain_header, justification, bridged_para_head, teyrchain_heads, para_heads_proof) =
		make_complex_bridged_teyrchain_heads_proof::<BridgedRelayChain, BridgedTeyrchain>(
			para_state_root,
			para_header_number,
			relay_header_number,
			bridged_para_id,
			is_minimal_call,
		);

	let message_proof = FromBridgedChainMessagesProof {
		bridged_header_hash: bridged_para_head.hash(),
		storage_proof: para_storage_proof,
		lane: lane_id,
		nonces_start: message_nonce,
		nonces_end: message_nonce,
	};

	(
		relay_chain_header,
		justification,
		bridged_para_head,
		teyrchain_heads,
		para_heads_proof,
		message_proof,
	)
}

/// Prepare storage proofs of message confirmations, stored at the target teyrchain.
pub fn make_complex_relayer_confirmation_proofs<
	BridgedRelayChain,
	BridgedTeyrchain,
	ThisChainWithMessages,
	LaneId,
>(
	lane_id: LaneId,
	para_header_number: u32,
	relay_header_number: u32,
	bridged_para_id: u32,
	relayer_id_at_this_chain: AccountIdOf<ThisChainWithMessages>,
	relayers_state: UnrewardedRelayersState,
) -> (
	HeaderOf<BridgedRelayChain>,
	GrandpaJustification<HeaderOf<BridgedRelayChain>>,
	ParaHead,
	Vec<(ParaId, ParaHash)>,
	ParaHeadsProof,
	FromBridgedChainMessagesDeliveryProof<ParaHash, LaneId>,
)
where
	BridgedRelayChain: pezbp_runtime::Chain<Hash = RelayBlockHash, BlockNumber = RelayBlockNumber>
		+ ChainWithGrandpa,
	BridgedTeyrchain: pezbp_runtime::Chain<Hash = ParaHash> + Teyrchain,
	ThisChainWithMessages: ChainWithMessages,
	LaneId: Copy + Encode,
{
	// prepare para storage proof containing message delivery proof
	let (para_state_root, para_storage_proof) =
		prepare_message_delivery_storage_proof::<BridgedTeyrchain, ThisChainWithMessages, LaneId>(
			lane_id,
			InboundLaneData {
				state: LaneState::Opened,
				relayers: vec![
					UnrewardedRelayer {
						relayer: relayer_id_at_this_chain.into(),
						messages: DeliveredMessages::new(1)
					};
					relayers_state.unrewarded_relayer_entries as usize
				]
				.into(),
				last_confirmed_nonce: 1,
			},
			UnverifiedStorageProofParams::default(),
		);

	let (relay_chain_header, justification, bridged_para_head, teyrchain_heads, para_heads_proof) =
		make_complex_bridged_teyrchain_heads_proof::<BridgedRelayChain, BridgedTeyrchain>(
			para_state_root,
			para_header_number,
			relay_header_number,
			bridged_para_id,
			false,
		);

	let message_delivery_proof = FromBridgedChainMessagesDeliveryProof {
		bridged_header_hash: bridged_para_head.hash(),
		storage_proof: para_storage_proof,
		lane: lane_id,
	};

	(
		relay_chain_header,
		justification,
		bridged_para_head,
		teyrchain_heads,
		para_heads_proof,
		message_delivery_proof,
	)
}

/// Make bridged teyrchain header with given state root and relay header that is finalizing it.
pub fn make_complex_bridged_teyrchain_heads_proof<BridgedRelayChain, BridgedTeyrchain>(
	para_state_root: ParaHash,
	para_header_number: u32,
	relay_header_number: BlockNumberOf<BridgedRelayChain>,
	bridged_para_id: u32,
	is_minimal_call: bool,
) -> (
	HeaderOf<BridgedRelayChain>,
	GrandpaJustification<HeaderOf<BridgedRelayChain>>,
	ParaHead,
	Vec<(ParaId, ParaHash)>,
	ParaHeadsProof,
)
where
	BridgedRelayChain: pezbp_runtime::Chain<Hash = RelayBlockHash, BlockNumber = RelayBlockNumber>
		+ ChainWithGrandpa,
	BridgedTeyrchain: pezbp_runtime::Chain<Hash = ParaHash> + Teyrchain,
{
	let bridged_para_head = ParaHead(
		pezbp_test_utils::test_header_with_root::<HeaderOf<BridgedTeyrchain>>(
			para_header_number.into(),
			para_state_root,
		)
		.encode(),
	);
	let (relay_state_root, para_heads_proof, teyrchain_heads) =
		prepare_teyrchain_heads_proof::<HeaderOf<BridgedTeyrchain>>(vec![(
			bridged_para_id,
			bridged_para_head.clone(),
		)]);
	assert_eq!(bridged_para_head.hash(), teyrchain_heads[0].1);

	let (relay_chain_header, justification) =
		make_complex_bridged_grandpa_header_proof::<BridgedRelayChain>(
			relay_state_root,
			relay_header_number,
			is_minimal_call,
		);

	(relay_chain_header, justification, bridged_para_head, teyrchain_heads, para_heads_proof)
}
