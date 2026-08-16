// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
#![cfg_attr(not(feature = "std"), no_std)]

use pezframe_support::traits::tokens::Balance as BalanceT;
use pezsnowbridge_core::PricingParameters;
use pezsnowbridge_merkle_tree::MerkleProof;
use pezsnowbridge_outbound_queue_primitives::v1::{Command, Fee};

pezsp_api::decl_runtime_apis! {
	pub trait OutboundQueueApi<Balance> where Balance: BalanceT
	{
		/// Generate a merkle proof for a committed message identified by `leaf_index`.
		/// The merkle root is stored in the block header as a
		/// `pezsp_runtime::generic::DigestItem::Other`
		fn prove_message(leaf_index: u64) -> Option<MerkleProof>;

		/// Calculate the delivery fee for `command`
		fn calculate_fee(command: Command, parameters: Option<PricingParameters<Balance>>) -> Fee<Balance>;
	}
}
