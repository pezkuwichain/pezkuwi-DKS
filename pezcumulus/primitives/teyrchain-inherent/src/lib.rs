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

//! Pezcumulus teyrchain inherent
//!
//! The [`TeyrchainInherentData`] is the data that is passed by the collator to the teyrchain
//! runtime. The runtime will use this data to execute messages from other teyrchains/the relay
//! chain or to read data from the relay chain state. When the teyrchain is validated by a teyrchain
//! validator on the relay chain, this data is checked for correctness. If the data passed by the
//! collator to the runtime isn't correct, the teyrchain candidate is considered invalid.
//!
//! To create a [`TeyrchainInherentData`] for a specific relay chain block, there exists the
//! `TeyrchainInherentDataExt` trait in `pezcumulus-client-teyrchain-inherent` that helps with this.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use pezcumulus_primitives_core::{
	relay_chain::{
		ApprovedPeerId, BlakeTwo256, BlockNumber as RelayChainBlockNumber, Hash as RelayHash,
		HashT as _, Header as RelayHeader,
	},
	InboundDownwardMessage, InboundHrmpMessage, ParaId, PersistedValidationData,
};
use pezsp_inherents::InherentIdentifier;
use scale_info::TypeInfo;

/// The identifier for the teyrchain inherent.
pub const TEYRCHAIN_INHERENT_IDENTIFIER_V0: InherentIdentifier = *b"sysi1337";
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"sysi1338";

/// Legacy TeyrchainInherentData that is kept around for backward compatibility.
/// Can be removed once we can safely assume that teyrchain nodes provide the
/// `relay_parent_descendants` and `collator_peer_id` fields.
pub mod v0 {
	use alloc::{collections::BTreeMap, vec::Vec};
	use pezcumulus_primitives_core::{
		InboundDownwardMessage, InboundHrmpMessage, ParaId, PersistedValidationData,
	};
	use scale_info::TypeInfo;

	/// The inherent data that is passed by the collator to the teyrchain runtime.
	#[derive(
		codec::Encode,
		codec::Decode,
		codec::DecodeWithMemTracking,
		pezsp_core::RuntimeDebug,
		Clone,
		PartialEq,
		TypeInfo,
	)]
	pub struct TeyrchainInherentData {
		pub validation_data: PersistedValidationData,
		/// A storage proof of a predefined set of keys from the relay-chain.
		///
		/// Specifically this witness contains the data for:
		///
		/// - the current slot number at the given relay parent
		/// - active host configuration as per the relay parent,
		/// - the relay dispatch queue sizes
		/// - the list of egress HRMP channels (in the list of recipients form)
		/// - the metadata for the egress HRMP channels
		pub relay_chain_state: pezsp_trie::StorageProof,
		/// Downward messages in the order they were sent.
		pub downward_messages: Vec<InboundDownwardMessage>,
		/// HRMP messages grouped by channels. The messages in the inner vec must be in order they
		/// were sent. In combination with the rule of no more than one message in a channel per
		/// block, this means `sent_at` is **strictly** greater than the previous one (if any).
		pub horizontal_messages: BTreeMap<ParaId, Vec<InboundHrmpMessage>>,
	}
}

/// The inherent data that is passed by the collator to the teyrchain runtime.
#[derive(
	codec::Encode,
	codec::Decode,
	codec::DecodeWithMemTracking,
	pezsp_core::RuntimeDebug,
	Clone,
	PartialEq,
	TypeInfo,
)]
pub struct TeyrchainInherentData {
	pub validation_data: PersistedValidationData,
	/// A storage proof of a predefined set of keys from the relay-chain.
	///
	/// Specifically this witness contains the data for:
	///
	/// - the current slot number at the given relay parent
	/// - active host configuration as per the relay parent,
	/// - the relay dispatch queue sizes
	/// - the list of egress HRMP channels (in the list of recipients form)
	/// - the metadata for the egress HRMP channels
	pub relay_chain_state: pezsp_trie::StorageProof,
	/// Downward messages in the order they were sent.
	pub downward_messages: Vec<InboundDownwardMessage>,
	/// HRMP messages grouped by channels. The messages in the inner vec must be in order they
	/// were sent. In combination with the rule of no more than one message in a channel per block,
	/// this means `sent_at` is **strictly** greater than the previous one (if any).
	pub horizontal_messages: BTreeMap<ParaId, Vec<InboundHrmpMessage>>,
	/// Contains the relay parent header and its descendants.
	/// This information is used to ensure that a teyrchain node builds blocks
	/// at a specified offset from the chain tip rather than directly at the tip.
	pub relay_parent_descendants: Vec<RelayHeader>,
	/// Contains the collator peer ID, which is later sent by the teyrchain to the
	/// relay chain via a UMP signal to promote the reputation of the given peer ID.
	pub collator_peer_id: Option<ApprovedPeerId>,
}

// Upgrades the TeyrchainInherentData v0 to the newest format.
impl Into<TeyrchainInherentData> for v0::TeyrchainInherentData {
	fn into(self) -> TeyrchainInherentData {
		TeyrchainInherentData {
			validation_data: self.validation_data,
			relay_chain_state: self.relay_chain_state,
			downward_messages: self.downward_messages,
			horizontal_messages: self.horizontal_messages,
			relay_parent_descendants: Vec::new(),
			collator_peer_id: None,
		}
	}
}

#[cfg(feature = "std")]
impl TeyrchainInherentData {
	/// Transforms [`TeyrchainInherentData`] into [`v0::TeyrchainInherentData`]. Can be used
	/// to create inherent data compatible with old runtimes.
	fn as_v0(&self) -> v0::TeyrchainInherentData {
		v0::TeyrchainInherentData {
			validation_data: self.validation_data.clone(),
			relay_chain_state: self.relay_chain_state.clone(),
			downward_messages: self.downward_messages.clone(),
			horizontal_messages: self.horizontal_messages.clone(),
		}
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl pezsp_inherents::InherentDataProvider for TeyrchainInherentData {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut pezsp_inherents::InherentData,
	) -> Result<(), pezsp_inherents::Error> {
		inherent_data.put_data(TEYRCHAIN_INHERENT_IDENTIFIER_V0, &self.as_v0())?;
		inherent_data.put_data(INHERENT_IDENTIFIER, &self)
	}

	async fn try_handle_error(
		&self,
		_: &pezsp_inherents::InherentIdentifier,
		_: &[u8],
	) -> Option<Result<(), pezsp_inherents::Error>> {
		None
	}
}

/// An inbound message whose content was hashed.
#[derive(
	codec::Encode,
	codec::Decode,
	codec::DecodeWithMemTracking,
	pezsp_core::RuntimeDebug,
	Clone,
	PartialEq,
	TypeInfo,
)]
pub struct HashedMessage {
	pub sent_at: RelayChainBlockNumber,
	pub msg_hash: pezsp_core::H256,
}

impl From<&InboundDownwardMessage<RelayChainBlockNumber>> for HashedMessage {
	fn from(msg: &InboundDownwardMessage<RelayChainBlockNumber>) -> Self {
		Self { sent_at: msg.sent_at, msg_hash: MessageQueueChain::hash_msg_data(&msg.msg) }
	}
}

impl From<&InboundHrmpMessage> for HashedMessage {
	fn from(msg: &InboundHrmpMessage) -> Self {
		Self { sent_at: msg.sent_at, msg_hash: MessageQueueChain::hash_msg_data(&msg.data) }
	}
}

/// This struct provides ability to extend a message queue chain (MQC) and compute a new head.
///
/// MQC is an instance of a [hash chain] applied to a message queue. Using a hash chain it's
/// possible to represent a sequence of messages using only a single hash.
///
/// A head for an empty chain is agreed to be a zero hash.
///
/// An instance is used to track either DMP from the relay chain or HRMP across a channel.
/// But a given instance is never used to track both. Therefore, you should call either
/// `extend_downward` or `extend_hrmp`, but not both methods on a single instance.
///
/// [hash chain]: https://en.wikipedia.org/wiki/Hash_chain
#[derive(Default, Clone, codec::Encode, codec::Decode, scale_info::TypeInfo)]
pub struct MessageQueueChain(RelayHash);

impl MessageQueueChain {
	/// Create a new instance initialized to `hash`.
	pub fn new(hash: RelayHash) -> Self {
		Self(hash)
	}

	/// Hash the provided message data.
	fn hash_msg_data(msg: &Vec<u8>) -> pezsp_core::H256 {
		BlakeTwo256::hash_of(msg)
	}

	/// Extend the hash chain with a `HashedMessage`.
	pub fn extend_with_hashed_msg(&mut self, hashed_msg: &HashedMessage) -> &mut Self {
		let prev_head = self.0;
		self.0 = BlakeTwo256::hash_of(&(prev_head, hashed_msg.sent_at, &hashed_msg.msg_hash));
		self
	}

	/// Extend the hash chain with an HRMP message. This method should be used only when
	/// this chain is tracking HRMP.
	pub fn extend_hrmp(&mut self, horizontal_message: &InboundHrmpMessage) -> &mut Self {
		self.extend_with_hashed_msg(&horizontal_message.into())
	}

	/// Extend the hash chain with a downward message. This method should be used only when
	/// this chain is tracking DMP.
	pub fn extend_downward(&mut self, downward_message: &InboundDownwardMessage) -> &mut Self {
		self.extend_with_hashed_msg(&downward_message.into())
	}

	/// Return the current head of the message queue chain.
	/// This is agreed to be the zero hash for an empty chain.
	pub fn head(&self) -> RelayHash {
		self.0
	}
}
