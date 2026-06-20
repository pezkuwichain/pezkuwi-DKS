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

//! Helpers for implementing various message-related runtime API methods.

use pezbp_messages::{InboundMessageDetails, MessageNonce, MessagePayload, OutboundMessageDetails};
use pezsp_std::vec::Vec;

/// Implementation of the `To*OutboundLaneApi::message_details`.
pub fn outbound_message_details<Runtime, MessagesPalletInstance>(
	lane: Runtime::LaneId,
	begin: MessageNonce,
	end: MessageNonce,
) -> Vec<OutboundMessageDetails>
where
	Runtime: pezpallet_bridge_messages::Config<MessagesPalletInstance>,
	MessagesPalletInstance: 'static,
{
	(begin..=end)
		.filter_map(|nonce| {
			let message_data = pezpallet_bridge_messages::Pezpallet::<
				Runtime,
				MessagesPalletInstance,
			>::outbound_message_data(lane, nonce)?;
			Some(OutboundMessageDetails {
				nonce,
				// dispatch message weight is always zero at the source chain, since we're paying
				// for dispatch at the target chain
				dispatch_weight: pezframe_support::weights::Weight::zero(),
				size: message_data.len() as _,
			})
		})
		.collect()
}

/// Implementation of the `To*InboundLaneApi::message_details`.
pub fn inbound_message_details<Runtime, MessagesPalletInstance>(
	lane: Runtime::LaneId,
	messages: Vec<(MessagePayload, OutboundMessageDetails)>,
) -> Vec<InboundMessageDetails>
where
	Runtime: pezpallet_bridge_messages::Config<MessagesPalletInstance>,
	MessagesPalletInstance: 'static,
{
	messages
		.into_iter()
		.map(|(payload, details)| {
			pezpallet_bridge_messages::Pezpallet::<Runtime, MessagesPalletInstance>::inbound_message_data(
				lane, payload, details,
			)
		})
		.collect()
}
