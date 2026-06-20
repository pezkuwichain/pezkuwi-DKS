// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// Pezcumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezcumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezcumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Teyrchain bootnodes advertisement and discovery service.

use crate::{
	advertisement::{BootnodeAdvertisement, BootnodeAdvertisementParams},
	config::paranode_protocol_name,
	discovery::{BootnodeDiscovery, BootnodeDiscoveryParams},
};
use log::{debug, error};
use num_traits::Zero;
use pezcumulus_primitives_core::{relay_chain::BlockId, ParaId};
use pezcumulus_relay_chain_interface::RelayChainInterface;
use pezsc_network::{
	request_responses::IncomingRequest, service::traits::NetworkService, Multiaddr,
};
use pezsc_service::TaskManager;
use std::sync::Arc;

/// Log target for this crate.
const LOG_TARGET: &str = "bootnodes";

/// Bootnode advertisement task params.
pub struct StartBootnodeTasksParams<'a> {
	/// Enable embedded DHT bootnode.
	pub embedded_dht_bootnode: bool,
	/// Enable DHT bootnode discovery.
	pub dht_bootnode_discovery: bool,
	/// Teyrchain ID.
	pub para_id: ParaId,
	/// Task manager.
	pub task_manager: &'a mut TaskManager,
	/// Relay chain interface.
	pub relay_chain_interface: Arc<dyn RelayChainInterface>,
	/// Relay chain fork ID.
	pub relay_chain_fork_id: Option<String>,
	/// Relay chain network service.
	pub relay_chain_network: Arc<dyn NetworkService>,
	/// `/paranode` protocol request receiver.
	pub request_receiver: async_channel::Receiver<IncomingRequest>,
	/// Teyrchain node network service.
	pub teyrchain_network: Arc<dyn NetworkService>,
	/// Whether to advertise non-global IP addresses.
	pub advertise_non_global_ips: bool,
	/// Teyrchain genesis hash.
	pub teyrchain_genesis_hash: Vec<u8>,
	/// Teyrchain fork ID.
	pub teyrchain_fork_id: Option<String>,
	/// Teyrchain public addresses provided by the operator.
	pub teyrchain_public_addresses: Vec<Multiaddr>,
}

async fn bootnode_advertisement(
	para_id: ParaId,
	relay_chain_interface: Arc<dyn RelayChainInterface>,
	relay_chain_network: Arc<dyn NetworkService>,
	request_receiver: async_channel::Receiver<IncomingRequest>,
	teyrchain_network: Arc<dyn NetworkService>,
	advertise_non_global_ips: bool,
	teyrchain_genesis_hash: Vec<u8>,
	teyrchain_fork_id: Option<String>,
	public_addresses: Vec<Multiaddr>,
) {
	let bootnode_advertisement = BootnodeAdvertisement::new(BootnodeAdvertisementParams {
		para_id,
		relay_chain_interface,
		relay_chain_network,
		request_receiver,
		teyrchain_network,
		advertise_non_global_ips,
		teyrchain_genesis_hash,
		teyrchain_fork_id,
		public_addresses,
	});

	if let Err(e) = bootnode_advertisement.run().await {
		error!(target: LOG_TARGET, "Bootnode advertisement terminated with error: {e}");
	}
}

async fn bootnode_discovery(
	para_id: ParaId,
	teyrchain_network: Arc<dyn NetworkService>,
	teyrchain_genesis_hash: Vec<u8>,
	teyrchain_fork_id: Option<String>,
	relay_chain_interface: Arc<dyn RelayChainInterface>,
	relay_chain_fork_id: Option<String>,
	relay_chain_network: Arc<dyn NetworkService>,
) {
	let relay_chain_genesis_hash =
		match relay_chain_interface.header(BlockId::Number(Zero::zero())).await {
			Ok(Some(header)) => header.hash().as_bytes().to_vec(),
			Ok(None) => {
				error!(
					target: LOG_TARGET,
					"Bootnode discovery: relay chain genesis hash does not exist",
				);
				// Make essential task fail.
				return;
			},
			Err(e) => {
				error!(
					target: LOG_TARGET,
					"Bootnode discovery: failed to obtain relay chain genesis hash: {e}",
				);
				// Make essential task fail.
				return;
			},
		};

	let paranode_protocol_name =
		paranode_protocol_name(relay_chain_genesis_hash, relay_chain_fork_id.as_deref());

	let bootnode_discovery = BootnodeDiscovery::new(BootnodeDiscoveryParams {
		para_id,
		teyrchain_network,
		teyrchain_genesis_hash,
		teyrchain_fork_id,
		relay_chain_interface,
		relay_chain_network,
		paranode_protocol_name,
	});

	match bootnode_discovery.run().await {
		// Do not terminate the essentil task if bootnode discovery succeeded.
		Ok(()) => std::future::pending().await,
		Err(e) => error!(target: LOG_TARGET, "Bootnode discovery terminated with error: {e}"),
	}
}

/// Start teyrchain bootnode advertisement and discovery tasks.
pub fn start_bootnode_tasks(
	StartBootnodeTasksParams {
		embedded_dht_bootnode,
		dht_bootnode_discovery,
		para_id,
		task_manager,
		relay_chain_interface,
		relay_chain_fork_id,
		relay_chain_network,
		request_receiver,
		teyrchain_network,
		advertise_non_global_ips,
		teyrchain_genesis_hash,
		teyrchain_fork_id,
		teyrchain_public_addresses,
	}: StartBootnodeTasksParams,
) {
	debug!(
		target: LOG_TARGET,
		"Embedded DHT bootnode enabled: {embedded_dht_bootnode}; \
		 DHT bootnode discovery enabled: {dht_bootnode_discovery}",
	);

	if embedded_dht_bootnode {
		task_manager.spawn_essential_handle().spawn(
			"pezcumulus-dht-bootnode-advertisement",
			None,
			bootnode_advertisement(
				para_id,
				relay_chain_interface.clone(),
				relay_chain_network.clone(),
				request_receiver,
				teyrchain_network.clone(),
				advertise_non_global_ips,
				teyrchain_genesis_hash.clone(),
				teyrchain_fork_id.clone(),
				teyrchain_public_addresses,
			),
		);
	}

	if dht_bootnode_discovery {
		task_manager.spawn_essential_handle().spawn(
			"pezcumulus-dht-bootnode-discovery",
			None,
			bootnode_discovery(
				para_id,
				teyrchain_network,
				teyrchain_genesis_hash,
				teyrchain_fork_id,
				relay_chain_interface,
				relay_chain_fork_id,
				relay_chain_network,
			),
		);
	}
}
