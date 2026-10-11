// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Types shared by request-response protocols and the network backend that carries them.
//!
//! - For every emitted request, a new substream is open and the protocol is negotiated. If the
//! remote supports the protocol, the size of the request is sent as a LEB128 number, followed
//! with the request itself. The remote then sends the size of the response as a LEB128 number,
//! followed with the response.
//!
//! - Requests have a certain time limit before they time out. This time includes the time it
//! takes to send/receive the request and response.
//!
//! - If provided, a "requests processing" channel (the `inbound_queue` of the protocol's
//! configuration) is used to handle incoming requests.

use crate::ReputationChange;

use futures::channel::oneshot;
use std::{io, sync::Arc};

/// Possible failures occurring in the context of sending an outbound request and receiving the
/// response.
#[derive(Debug, Clone, thiserror::Error)]
pub enum OutboundFailure {
	/// The request could not be sent because a dialing attempt failed.
	#[error("Failed to dial the requested peer")]
	DialFailure,
	/// The request timed out before a response was received.
	#[error("Timeout while waiting for a response")]
	Timeout,
	/// The connection closed before a response was received.
	#[error("Connection was closed before a response was received")]
	ConnectionClosed,
	/// The remote supports none of the requested protocols.
	#[error("The remote supports none of the requested protocols")]
	UnsupportedProtocols,
	/// An IO failure happened on an outbound stream.
	#[error("An IO failure happened on an outbound stream")]
	Io(Arc<io::Error>),
}

/// Error in a request.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum RequestFailure {
	#[error("We are not currently connected to the requested peer.")]
	NotConnected,
	#[error("Given protocol hasn't been registered.")]
	UnknownProtocol,
	#[error("The outbound request payload or parameters are invalid for the selected protocol.")]
	InvalidRequest,
	#[error("Remote has closed the substream before answering, thereby signaling that it considers the request as valid, but refused to answer it.")]
	Refused,
	#[error("The remote replied, but the local node is no longer interested in the response.")]
	Obsolete,
	#[error("Problem on the network: {0}")]
	Network(OutboundFailure),
}

/// A single request received by a peer on a request-response protocol.
#[derive(Debug)]
pub struct IncomingRequest {
	/// Who sent the request.
	pub peer: pezsc_network_types::PeerId,

	/// Request sent by the remote. Will always be smaller than the `max_request_size` of the
	/// protocol's configuration.
	pub payload: Vec<u8>,

	/// Channel to send back the response.
	///
	/// There are two ways to indicate that handling the request failed:
	///
	/// 1. Drop `pending_response` and thus not changing the reputation of the peer.
	///
	/// 2. Sending an `Err(())` via `pending_response`, optionally including reputation changes for
	/// the given peer.
	pub pending_response: oneshot::Sender<OutgoingResponse>,
}

/// Response for an incoming request to be send by a request protocol handler.
#[derive(Debug)]
pub struct OutgoingResponse {
	/// The payload of the response.
	///
	/// `Err(())` if none is available e.g. due an error while handling the request.
	pub result: Result<Vec<u8>, ()>,

	/// Reputation changes accrued while handling the request. To be applied to the reputation of
	/// the peer sending the request.
	pub reputation_changes: Vec<ReputationChange>,

	/// If provided, the `oneshot::Sender` will be notified when the request has been sent to the
	/// peer.
	///
	/// > **Note**: Operating systems typically maintain a buffer of a few dozen kilobytes of
	/// >			outgoing data for each TCP socket, and it is not possible for a user
	/// >			application to inspect this buffer. This channel here is not actually notified
	/// >			when the response has been fully sent out, but rather when it has fully been
	/// >			written to the buffer managed by the operating system.
	pub sent_feedback: Option<oneshot::Sender<()>>,
}

/// When sending a request, what to do on a disconnected recipient.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IfDisconnected {
	/// Try to connect to the peer.
	TryConnect,
	/// Just fail if the destination is not yet connected.
	ImmediateError,
}

/// Convenience functions for `IfDisconnected`.
impl IfDisconnected {
	/// Shall we connect to a disconnected peer?
	pub fn should_connect(self) -> bool {
		match self {
			Self::TryConnect => true,
			Self::ImmediateError => false,
		}
	}
}
