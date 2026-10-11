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

//! Interfaces between the peer store and the protocols that use it.
//!
//! The peer store itself lives with the network backend; see `litep2p::peerstore`.

use pezsc_network_common::{role::ObservedRole, types::ReputationChange};
use std::{collections::HashSet, fmt::Debug, sync::Arc};

/// Trait describing the required functionality from a `Peerset` handle.
pub trait ProtocolHandle: Debug + Send + Sync {
	/// Disconnect peer.
	fn disconnect_peer(&self, peer_id: pezsc_network_types::PeerId);
}

/// Trait providing peer reputation management and connection candidates.
pub trait PeerStoreProvider: Debug + Send + Sync {
	/// Check whether the peer is banned.
	fn is_banned(&self, peer_id: &pezsc_network_types::PeerId) -> bool;

	/// Register a protocol handle to disconnect peers whose reputation drops below the threshold.
	fn register_protocol(&self, protocol_handle: Arc<dyn ProtocolHandle>);

	/// Report peer disconnection for reputation adjustment.
	fn report_disconnect(&self, peer_id: pezsc_network_types::PeerId);

	/// Adjust peer reputation.
	fn report_peer(&self, peer_id: pezsc_network_types::PeerId, change: ReputationChange);

	/// Set peer role.
	fn set_peer_role(&self, peer_id: &pezsc_network_types::PeerId, role: ObservedRole);

	/// Get peer reputation.
	fn peer_reputation(&self, peer_id: &pezsc_network_types::PeerId) -> i32;

	/// Get peer role, if available.
	fn peer_role(&self, peer_id: &pezsc_network_types::PeerId) -> Option<ObservedRole>;

	/// Get candidates with highest reputations for initiating outgoing connections.
	fn outgoing_candidates(
		&self,
		count: usize,
		ignored: HashSet<pezsc_network_types::PeerId>,
	) -> Vec<pezsc_network_types::PeerId>;

	/// Add known peer.
	fn add_known_peer(&self, peer_id: pezsc_network_types::PeerId);
}
