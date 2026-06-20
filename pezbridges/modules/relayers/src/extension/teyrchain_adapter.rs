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

//! Adapter that allows using `pezpallet-bridge-relayers` as a signed extension in the
//! bridge with remote teyrchain.

use crate::{
	extension::{
		grandpa_adapter::verify_submit_finality_proof_succeeded, verify_messages_call_succeeded,
	},
	Config as BridgeRelayersConfig, LOG_TARGET,
};

use core::marker::PhantomData;
use pezbp_relayers::{BatchCallUnpacker, ExtensionCallData, ExtensionCallInfo, ExtensionConfig};
use pezbp_runtime::{StaticStrProvider, Teyrchain};
use pezframe_support::dispatch::{DispatchInfo, PostDispatchInfo};
use pezframe_system::Config as SystemConfig;
use pezpallet_bridge_grandpa::{
	CallSubType as BridgeGrandpaCallSubtype, Config as BridgeGrandpaConfig,
};
use pezpallet_bridge_messages::{
	CallSubType as BridgeMessagesCallSubType, Config as BridgeMessagesConfig, LaneIdOf,
};
use pezpallet_bridge_teyrchains::{
	CallSubType as BridgeTeyrchainsCallSubtype, Config as BridgeTeyrchainsConfig,
	SubmitTeyrchainHeadsHelper,
};
use pezsp_runtime::{
	traits::{Dispatchable, Get},
	transaction_validity::{TransactionPriority, TransactionValidityError},
};

/// Adapter to be used in signed extension configuration, when bridging with remote teyrchains.
pub struct WithTeyrchainExtensionConfig<
	// signed extension identifier
	IdProvider,
	// runtime that implements `BridgeMessagesConfig<BridgeMessagesPalletInstance>`, which
	// uses `BridgeTeyrchainsConfig<BridgeTeyrchainsPalletInstance>` to receive messages and
	// confirmations from the remote chain.
	Runtime,
	// batch call unpacker
	BatchCallUnpacker,
	// instance of the `pezpallet-bridge-teyrchains`, tracked by this extension
	BridgeTeyrchainsPalletInstance,
	// instance of BridgedChain `pezpallet-bridge-messages`, tracked by this extension
	BridgeMessagesPalletInstance,
	// instance of `pezpallet-bridge-relayers`, tracked by this extension
	BridgeRelayersPalletInstance,
	// message delivery transaction priority boost for every additional message
	PriorityBoostPerMessage,
>(
	PhantomData<(
		IdProvider,
		Runtime,
		BatchCallUnpacker,
		BridgeTeyrchainsPalletInstance,
		BridgeMessagesPalletInstance,
		BridgeRelayersPalletInstance,
		PriorityBoostPerMessage,
	)>,
);

impl<ID, R, BCU, PI, MI, RI, P> ExtensionConfig
	for WithTeyrchainExtensionConfig<ID, R, BCU, PI, MI, RI, P>
where
	ID: StaticStrProvider,
	R: BridgeRelayersConfig<RI>
		+ BridgeMessagesConfig<MI>
		+ BridgeTeyrchainsConfig<PI>
		+ BridgeGrandpaConfig<R::BridgesGrandpaPalletInstance>,
	BCU: BatchCallUnpacker<R>,
	PI: 'static,
	MI: 'static,
	RI: 'static,
	P: Get<TransactionPriority>,
	R::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>
		+ BridgeGrandpaCallSubtype<R, R::BridgesGrandpaPalletInstance>
		+ BridgeTeyrchainsCallSubtype<R, PI>
		+ BridgeMessagesCallSubType<R, MI>,
	<R as BridgeMessagesConfig<MI>>::BridgedChain: Teyrchain,
{
	type IdProvider = ID;
	type Runtime = R;
	type BridgeMessagesPalletInstance = MI;
	type BridgeRelayersPalletInstance = RI;
	type PriorityBoostPerMessage = P;
	type RemoteGrandpaChainBlockNumber =
		pezpallet_bridge_grandpa::BridgedBlockNumber<R, R::BridgesGrandpaPalletInstance>;
	type LaneId = LaneIdOf<R, Self::BridgeMessagesPalletInstance>;

	fn parse_and_check_for_obsolete_call(
		call: &R::RuntimeCall,
	) -> Result<
		Option<ExtensionCallInfo<Self::RemoteGrandpaChainBlockNumber, Self::LaneId>>,
		TransactionValidityError,
	> {
		let calls = BCU::unpack(call, 3);
		let total_calls = calls.len();
		let mut calls = calls.into_iter().map(Self::check_obsolete_parsed_call).rev();

		let msgs_call = calls.next().transpose()?.and_then(|c| c.call_info());
		let para_finality_call = calls.next().transpose()?.and_then(|c| {
			let r = c.submit_teyrchain_heads_info_for(
				<R as BridgeMessagesConfig<Self::BridgeMessagesPalletInstance>>::BridgedChain::TEYRCHAIN_ID,
			);
			r
		});
		let relay_finality_call =
			calls.next().transpose()?.and_then(|c| c.submit_finality_proof_info());
		Ok(match (total_calls, relay_finality_call, para_finality_call, msgs_call) {
			(3, Some(relay_finality_call), Some(para_finality_call), Some(msgs_call)) => {
				Some(ExtensionCallInfo::AllFinalityAndMsgs(
					relay_finality_call,
					para_finality_call,
					msgs_call,
				))
			},
			(2, None, Some(para_finality_call), Some(msgs_call)) => {
				Some(ExtensionCallInfo::TeyrchainFinalityAndMsgs(para_finality_call, msgs_call))
			},
			(1, None, None, Some(msgs_call)) => Some(ExtensionCallInfo::Msgs(msgs_call)),
			_ => None,
		})
	}

	fn check_obsolete_parsed_call(
		call: &R::RuntimeCall,
	) -> Result<&R::RuntimeCall, TransactionValidityError> {
		call.check_obsolete_submit_finality_proof()?;
		call.check_obsolete_submit_teyrchain_heads()?;
		call.check_obsolete_call()?;
		Ok(call)
	}

	fn check_call_result(
		call_info: &ExtensionCallInfo<Self::RemoteGrandpaChainBlockNumber, Self::LaneId>,
		call_data: &mut ExtensionCallData,
		relayer: &R::AccountId,
	) -> bool {
		verify_submit_finality_proof_succeeded::<Self, R::BridgesGrandpaPalletInstance>(
			call_info, call_data, relayer,
		) && verify_submit_teyrchain_head_succeeded::<Self, PI>(call_info, call_data, relayer)
			&& verify_messages_call_succeeded::<Self>(call_info, call_data, relayer)
	}
}

/// If the batch call contains the teyrchain state update call, verify that it
/// has been successful.
///
/// Only returns false when teyrchain state update call has failed.
pub(crate) fn verify_submit_teyrchain_head_succeeded<C, PI>(
	call_info: &ExtensionCallInfo<C::RemoteGrandpaChainBlockNumber, C::LaneId>,
	_call_data: &mut ExtensionCallData,
	relayer: &<C::Runtime as SystemConfig>::AccountId,
) -> bool
where
	C: ExtensionConfig,
	PI: 'static,
	C::Runtime: BridgeTeyrchainsConfig<PI>,
{
	let Some(para_proof_info) = call_info.submit_teyrchain_heads_info() else { return true };

	if !SubmitTeyrchainHeadsHelper::<C::Runtime, PI>::was_successful(para_proof_info) {
		// we only refund relayer if all calls have updated chain state
		tracing::trace!(
			target: LOG_TARGET,
			id_provider=%C::IdProvider::STR,
			lane_id=?call_info.messages_call_info().lane_id(),
			?relayer,
			"Relayer has submitted invalid teyrchain finality proof"
		);
		return false;
	}

	true
}
