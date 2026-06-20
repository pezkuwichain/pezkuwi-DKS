// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use super::Pezpallet;
use codec::{Decode, Encode, MaxEncodedLen};
use pezframe_support::traits::ProcessMessage;
pub use pezsnowbridge_merkle_tree::MerkleProof;
use pezsp_core::H256;
use pezsp_runtime::RuntimeDebug;
use pezsp_std::prelude::*;
use scale_info::TypeInfo;

pub type ProcessMessageOriginOf<T> = <Pezpallet<T> as ProcessMessage>::Origin;

/// Pending order
#[derive(Encode, Decode, TypeInfo, Clone, Eq, PartialEq, RuntimeDebug, MaxEncodedLen)]
pub struct PendingOrder<BlockNumber> {
	/// The nonce used to identify the message
	pub nonce: u64,
	/// The block number in which the message was committed
	pub block_number: BlockNumber,
	/// The fee in Ether provided by the user to incentivize message delivery
	#[codec(compact)]
	pub fee: u128,
}

/// Hook that will be called when a new message commitment is constructed.
pub trait OnNewCommitment {
	fn on_new_commitment(commitment: H256);
}

impl OnNewCommitment for () {
	fn on_new_commitment(_commitment: H256) {}
}
