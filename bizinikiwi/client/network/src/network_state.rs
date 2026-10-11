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

//! Information about the networking, for diagnostic purposes.
//!
//! **Warning**: These APIs are not stable.

use pezsc_network_types::multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use std::{
	collections::{HashMap, HashSet},
	time::Duration,
};

/// Returns general information about the networking.
///
/// Meant for general diagnostic purposes.
///
/// **Warning**: This API is not stable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkState {
	/// PeerId of the local node.
	pub peer_id: String,
	/// List of addresses the node is currently listening on.
	pub listened_addresses: HashSet<Multiaddr>,
	/// List of addresses the node knows it can be reached as.
	pub external_addresses: HashSet<Multiaddr>,
	/// List of node we're connected to.
	pub connected_peers: HashMap<String, Peer>,
	/// List of node that we know of but that we're not connected to.
	pub not_connected_peers: HashMap<String, NotConnectedPeer>,
	/// State of the peerset manager.
	pub peerset: serde_json::Value,
}

/// Part of the `NetworkState` struct. Unstable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
	/// How we are connected to the node.
	pub endpoint: PeerEndpoint,
	/// Node information, as provided by the node itself. Can be empty if not known yet.
	pub version_string: Option<String>,
	/// Latest ping duration with this node.
	pub latest_ping_time: Option<Duration>,
	/// List of addresses known for this node.
	pub known_addresses: HashSet<Multiaddr>,
}

/// Part of the `NetworkState` struct. Unstable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotConnectedPeer {
	/// List of addresses known for this node.
	pub known_addresses: HashSet<Multiaddr>,
	/// Node information, as provided by the node itself, if we were ever connected to this node.
	pub version_string: Option<String>,
	/// Latest ping duration with this node, if we were ever connected to this node.
	pub latest_ping_time: Option<Duration>,
}

/// Part of the `NetworkState` struct. Unstable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PeerEndpoint {
	/// We are dialing the given address.
	Dialing(Multiaddr, Endpoint),
	/// We are listening.
	Listening {
		/// Local address of the connection.
		local_addr: Multiaddr,
		/// Address data is sent back to.
		send_back_addr: Multiaddr,
	},
}

/// Part of the `NetworkState` struct. Unstable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Endpoint {
	/// The socket comes from a dialer.
	Dialer,
	/// The socket comes from a listener.
	Listener,
}

#[cfg(test)]
mod tests {
	use super::*;

	/// `system_networkState` returns this struct as JSON. Addresses used to be libp2p's
	/// `Multiaddr`, which serializes to its string form in human-readable formats; the local
	/// type has to produce the same bytes.
	#[test]
	fn json_shape_is_stable() {
		let addr: Multiaddr = "/ip4/127.0.0.1/tcp/30333".parse().unwrap();
		let state = NetworkState {
			peer_id: "local".into(),
			listened_addresses: [addr.clone()].into(),
			external_addresses: Default::default(),
			connected_peers: [(
				"remote".to_string(),
				Peer {
					endpoint: PeerEndpoint::Dialing(addr.clone(), Endpoint::Dialer),
					version_string: None,
					latest_ping_time: Some(Duration::from_millis(5)),
					known_addresses: [addr.clone()].into(),
				},
			)]
			.into(),
			not_connected_peers: Default::default(),
			peerset: serde_json::Value::Null,
		};

		let json = serde_json::to_string(&state).unwrap();
		assert_eq!(
			json,
			concat!(
				r#"{"peerId":"local","listenedAddresses":["/ip4/127.0.0.1/tcp/30333"],"#,
				r#""externalAddresses":[],"connectedPeers":{"remote":{"endpoint":"#,
				r#"{"dialing":["/ip4/127.0.0.1/tcp/30333","dialer"]},"versionString":null,"#,
				r#""latestPingTime":{"secs":0,"nanos":5000000},"#,
				r#""knownAddresses":["/ip4/127.0.0.1/tcp/30333"]}},"notConnectedPeers":{},"#,
				r#""peerset":null}"#,
			)
		);
		assert_eq!(serde_json::from_str::<NetworkState>(&json).unwrap(), state);
	}
}
