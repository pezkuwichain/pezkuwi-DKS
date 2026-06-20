// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
//! Implementation for [`pezsnowbridge_outbound_queue_primitives::v2::SendMessage`]
use super::*;
use codec::Encode;
use pezframe_support::{
	ensure,
	traits::{EnqueueMessage, Get},
};
use pezsnowbridge_outbound_queue_primitives::{
	v2::{Message, SendMessage},
	SendError,
};
use pezsp_core::H256;
use pezsp_runtime::BoundedVec;

impl<T> SendMessage for Pezpallet<T>
where
	T: Config,
{
	type Ticket = Message;

	fn validate(message: &Message) -> Result<Self::Ticket, SendError> {
		// The inner payload should not be too large
		let payload = message.encode();
		ensure!(
			payload.len() < T::MaxMessagePayloadSize::get() as usize,
			SendError::MessageTooLarge
		);

		Ok(message.clone())
	}

	fn deliver(ticket: Self::Ticket) -> Result<H256, SendError> {
		let origin = ticket.origin.into();

		let message =
			BoundedVec::try_from(ticket.encode()).map_err(|_| SendError::MessageTooLarge)?;

		T::MessageQueue::enqueue_message(message.as_bounded_slice(), origin);
		Self::deposit_event(Event::MessageQueued { message: ticket.clone() });
		Ok(ticket.id)
	}
}
