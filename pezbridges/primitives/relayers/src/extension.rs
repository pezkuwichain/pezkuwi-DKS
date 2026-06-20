// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! All runtime calls, supported by `pezpallet-bridge-relayers` when it acts as a signed
//! extension.

use codec::{Decode, Encode};
use pezbp_header_pez_chain::SubmitFinalityProofInfo;
use pezbp_messages::MessagesCallInfo;
use pezbp_runtime::StaticStrProvider;
use pezbp_teyrchains::SubmitTeyrchainHeadsInfo;
use pezframe_support::{
	dispatch::CallableCallFor, traits::IsSubType, weights::Weight, RuntimeDebugNoBound,
};
use pezframe_system::Config as SystemConfig;
use pezpallet_utility::{Call as UtilityCall, Pezpallet as UtilityPallet};
use pezsp_runtime::{
	traits::Get,
	transaction_validity::{TransactionPriority, TransactionValidityError},
	RuntimeDebug,
};
use pezsp_std::{fmt::Debug, marker::PhantomData, vec, vec::Vec};

/// Type of the call that the signed extension recognizes.
#[derive(PartialEq, RuntimeDebugNoBound)]
pub enum ExtensionCallInfo<RemoteGrandpaChainBlockNumber: Debug, LaneId: Clone + Copy + Debug> {
	/// Relay chain finality + teyrchain finality + message delivery/confirmation calls.
	AllFinalityAndMsgs(
		SubmitFinalityProofInfo<RemoteGrandpaChainBlockNumber>,
		SubmitTeyrchainHeadsInfo,
		MessagesCallInfo<LaneId>,
	),
	/// Relay chain finality + message delivery/confirmation calls.
	RelayFinalityAndMsgs(
		SubmitFinalityProofInfo<RemoteGrandpaChainBlockNumber>,
		MessagesCallInfo<LaneId>,
	),
	/// Teyrchain finality + message delivery/confirmation calls.
	///
	/// This variant is used only when bridging with teyrchain.
	TeyrchainFinalityAndMsgs(SubmitTeyrchainHeadsInfo, MessagesCallInfo<LaneId>),
	/// Standalone message delivery/confirmation call.
	Msgs(MessagesCallInfo<LaneId>),
}

impl<RemoteGrandpaChainBlockNumber: Clone + Copy + Debug, LaneId: Clone + Copy + Debug>
	ExtensionCallInfo<RemoteGrandpaChainBlockNumber, LaneId>
{
	/// Returns true if call is a message delivery call (with optional finality calls).
	pub fn is_receive_messages_proof_call(&self) -> bool {
		match self.messages_call_info() {
			MessagesCallInfo::ReceiveMessagesProof(_) => true,
			MessagesCallInfo::ReceiveMessagesDeliveryProof(_) => false,
		}
	}

	/// Returns the pre-dispatch `finality_target` sent to the `SubmitFinalityProof` call.
	pub fn submit_finality_proof_info(
		&self,
	) -> Option<SubmitFinalityProofInfo<RemoteGrandpaChainBlockNumber>> {
		match *self {
			Self::AllFinalityAndMsgs(info, _, _) => Some(info),
			Self::RelayFinalityAndMsgs(info, _) => Some(info),
			_ => None,
		}
	}

	/// Returns the pre-dispatch `SubmitTeyrchainHeadsInfo`.
	pub fn submit_teyrchain_heads_info(&self) -> Option<&SubmitTeyrchainHeadsInfo> {
		match self {
			Self::AllFinalityAndMsgs(_, info, _) => Some(info),
			Self::TeyrchainFinalityAndMsgs(info, _) => Some(info),
			_ => None,
		}
	}

	/// Returns the pre-dispatch `ReceiveMessagesProofInfo`.
	pub fn messages_call_info(&self) -> &MessagesCallInfo<LaneId> {
		match self {
			Self::AllFinalityAndMsgs(_, _, info) => info,
			Self::RelayFinalityAndMsgs(_, info) => info,
			Self::TeyrchainFinalityAndMsgs(_, info) => info,
			Self::Msgs(info) => info,
		}
	}
}

/// Extra post-dispatch data, associated with the supported runtime call.
#[derive(Default, RuntimeDebug)]
pub struct ExtensionCallData {
	/// Extra weight, consumed by the call. We have some assumptions about normal weight
	/// that may be consumed by expected calls. If the actual weight is larger than that,
	/// we do not refund relayer for this extra weight.
	pub extra_weight: Weight,
	/// Extra size, consumed by the call. We have some assumptions about normal size
	/// of the encoded call. If the actual size is larger than that, we do not refund relayer
	/// for this extra size.
	pub extra_size: u32,
}

/// Signed extension configuration.
///
/// The single `pezpallet-bridge-relayers` instance may be shared by multiple messages
/// pezpallet instances, bridging with different remote networks. We expect every instance
/// of the messages pezpallet to add a separate signed extension to runtime. So it must
/// have a separate configuration.
pub trait ExtensionConfig {
	/// Unique identifier of the signed extension that will use this configuration.
	type IdProvider: StaticStrProvider;
	/// Runtime that optionally supports batched calls. We assume that batched call
	/// succeeds if and only if all of its nested calls succeed.
	type Runtime: pezframe_system::Config;
	/// Relayers pezpallet instance.
	type BridgeRelayersPalletInstance: 'static;
	/// Messages pezpallet instance.
	type BridgeMessagesPalletInstance: 'static;
	/// Additional priority that is added to base message delivery transaction priority
	/// for every additional bundled message.
	type PriorityBoostPerMessage: Get<TransactionPriority>;
	/// Block number for the remote **GRANDPA chain**. Mind that this chain is not
	/// necessarily the chain that we are bridging with. If we are bridging with
	/// teyrchain, it must be its parent relay chain. If we are bridging with the
	/// GRANDPA chain, it must be it.
	type RemoteGrandpaChainBlockNumber: Clone + Copy + Debug;
	/// Lane identifier type.
	type LaneId: Clone + Copy + Decode + Encode + Debug;

	/// Given runtime call, check if it is supported by the transaction extension. Additionally,
	/// check if call (or any of batched calls) are obsolete.
	fn parse_and_check_for_obsolete_call(
		call: &<Self::Runtime as SystemConfig>::RuntimeCall,
	) -> Result<
		Option<ExtensionCallInfo<Self::RemoteGrandpaChainBlockNumber, Self::LaneId>>,
		TransactionValidityError,
	>;

	/// Check if runtime call is already obsolete.
	fn check_obsolete_parsed_call(
		call: &<Self::Runtime as SystemConfig>::RuntimeCall,
	) -> Result<&<Self::Runtime as SystemConfig>::RuntimeCall, TransactionValidityError>;

	/// Given runtime call info, check that this call has been successful and has updated
	/// runtime storage accordingly.
	fn check_call_result(
		call_info: &ExtensionCallInfo<Self::RemoteGrandpaChainBlockNumber, Self::LaneId>,
		call_data: &mut ExtensionCallData,
		relayer: &<Self::Runtime as SystemConfig>::AccountId,
	) -> bool;
}

/// Something that can unpack batch calls (all-or-nothing flavor) of given size.
pub trait BatchCallUnpacker<Runtime: pezframe_system::Config> {
	/// Unpack batch call with no more than `max_packed_calls` calls.
	fn unpack(call: &Runtime::RuntimeCall, max_packed_calls: u32) -> Vec<&Runtime::RuntimeCall>;
}

/// An `BatchCallUnpacker` adapter for runtimes with utility pezpallet.
pub struct RuntimeWithUtilityPallet<Runtime>(PhantomData<Runtime>);

impl<Runtime> BatchCallUnpacker<Runtime> for RuntimeWithUtilityPallet<Runtime>
where
	Runtime: pezpallet_utility::Config<RuntimeCall = <Runtime as SystemConfig>::RuntimeCall>,
	<Runtime as SystemConfig>::RuntimeCall:
		IsSubType<CallableCallFor<UtilityPallet<Runtime>, Runtime>>,
{
	fn unpack(
		call: &<Runtime as pezframe_system::Config>::RuntimeCall,
		max_packed_calls: u32,
	) -> Vec<&<Runtime as pezframe_system::Config>::RuntimeCall> {
		match call.is_sub_type() {
			Some(UtilityCall::<Runtime>::batch_all { ref calls })
				if calls.len() <= max_packed_calls as usize =>
			{
				calls.iter().collect()
			},
			Some(_) => vec![],
			None => vec![call],
		}
	}
}

impl<Runtime: pezframe_system::Config> BatchCallUnpacker<Runtime> for () {
	fn unpack(call: &Runtime::RuntimeCall, _max_packed_calls: u32) -> Vec<&Runtime::RuntimeCall> {
		vec![call]
	}
}
