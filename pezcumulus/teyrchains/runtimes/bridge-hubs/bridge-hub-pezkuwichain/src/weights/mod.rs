// This file is part of Pezcumulus.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Expose the auto generated weight files.

use ::pezpallet_bridge_grandpa::WeightInfoExt as GrandpaWeightInfoExt;
use ::pezpallet_bridge_messages::WeightInfoExt as MessagesWeightInfoExt;
use ::pezpallet_bridge_relayers::WeightInfo as _;
use ::pezpallet_bridge_teyrchains::WeightInfoExt as TeyrchainsWeightInfoExt;

pub mod block_weights;
pub mod extrinsic_weights;
pub mod paritydb_weights;
pub mod pezcumulus_pezpallet_teyrchain_system;
pub mod pezcumulus_pezpallet_weight_reclaim;
pub mod pezcumulus_pezpallet_xcmp_queue;
pub mod pezframe_system;
pub mod pezframe_system_extensions;
pub mod pezpallet_balances;
pub mod pezpallet_bridge_grandpa;
pub mod pezpallet_bridge_messages_pezkuwichain_to_pezkuwichain_bulletin;
pub mod pezpallet_bridge_messages_pezkuwichain_to_zagros;
pub mod pezpallet_bridge_relayers_legacy;
pub mod pezpallet_bridge_relayers_permissionless_lanes;
pub mod pezpallet_bridge_teyrchains;
pub mod pezpallet_collator_selection;
pub mod pezpallet_message_queue;
pub mod pezpallet_multisig;
pub mod pezpallet_session;
pub mod pezpallet_timestamp;
pub mod pezpallet_transaction_payment;
pub mod pezpallet_utility;
pub mod pezpallet_xcm;
pub mod pezsnowbridge_pezpallet_ethereum_client;
pub mod pezsnowbridge_pezpallet_inbound_queue;
pub mod pezsnowbridge_pezpallet_outbound_queue;
pub mod pezsnowbridge_pezpallet_system;
pub mod rocksdb_weights;
pub mod xcm;

pub use block_weights::constants::BlockExecutionWeight;
pub use extrinsic_weights::constants::ExtrinsicBaseWeight;
pub use rocksdb_weights::constants::RocksDbWeight;

use crate::Runtime;
use pezframe_support::weights::Weight;

// import trait from dependency module
use ::pezpallet_bridge_relayers::WeightInfoExt as _;

impl GrandpaWeightInfoExt for pezpallet_bridge_grandpa::WeightInfo<crate::Runtime> {
	fn submit_finality_proof_overhead_from_runtime() -> Weight {
		// our signed extension:
		// 1) checks whether relayer registration is active from validate/pre_dispatch;
		// 2) may slash and deregister relayer from post_dispatch
		// (2) includes (1), so (2) is the worst case
		pezpallet_bridge_relayers_legacy::WeightInfo::<Runtime>::slash_and_deregister()
	}
}

impl MessagesWeightInfoExt
	for pezpallet_bridge_messages_pezkuwichain_to_pezkuwichain_bulletin::WeightInfo<crate::Runtime>
{
	fn expected_extra_storage_proof_size() -> u32 {
		pezbp_pezkuwi_bulletin::EXTRA_STORAGE_PROOF_SIZE
	}

	fn receive_messages_proof_overhead_from_runtime() -> Weight {
		pezpallet_bridge_relayers_permissionless_lanes::WeightInfo::<Runtime>::receive_messages_proof_overhead_from_runtime(
		)
	}

	fn receive_messages_delivery_proof_overhead_from_runtime() -> Weight {
		pezpallet_bridge_relayers_permissionless_lanes::WeightInfo::<Runtime>::receive_messages_delivery_proof_overhead_from_runtime()
	}
}

impl MessagesWeightInfoExt
	for pezpallet_bridge_messages_pezkuwichain_to_zagros::WeightInfo<crate::Runtime>
{
	fn expected_extra_storage_proof_size() -> u32 {
		pezbp_bridge_hub_zagros::EXTRA_STORAGE_PROOF_SIZE
	}

	fn receive_messages_proof_overhead_from_runtime() -> Weight {
		pezpallet_bridge_relayers_legacy::WeightInfo::<Runtime>::receive_messages_proof_overhead_from_runtime(
		)
	}

	fn receive_messages_delivery_proof_overhead_from_runtime() -> Weight {
		pezpallet_bridge_relayers_legacy::WeightInfo::<Runtime>::receive_messages_delivery_proof_overhead_from_runtime()
	}
}

impl TeyrchainsWeightInfoExt for pezpallet_bridge_teyrchains::WeightInfo<crate::Runtime> {
	fn expected_extra_storage_proof_size() -> u32 {
		pezbp_bridge_hub_zagros::EXTRA_STORAGE_PROOF_SIZE
	}

	fn submit_teyrchain_heads_overhead_from_runtime() -> Weight {
		// our signed extension:
		// 1) checks whether relayer registration is active from validate/pre_dispatch;
		// 2) may slash and deregister relayer from post_dispatch
		// (2) includes (1), so (2) is the worst case
		pezpallet_bridge_relayers_legacy::WeightInfo::<Runtime>::slash_and_deregister()
	}
}
