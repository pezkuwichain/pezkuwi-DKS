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

use crate::common::{types::TeyrchainClient, ConstructNodeRuntimeApi, NodeBlock};
use pezsc_network::{
	config::FullNetworkConfiguration, service::traits::NetworkService, NetworkBackend,
};
use pezsc_service::{Configuration, TaskManager};
use pezsc_statement_store::Store;
use std::sync::Arc;
use teyrchains_common::Hash;

/// Helper function to setup the statement store in `NodeSpec::start_node`.
///
/// Functions are tailored for internal usage, types are unnecessary opinionated for usage in
/// `NodeSpec::start_node`.

/// Build the statement handler prototype. Register the notification protocol in the network
/// configuration.
pub(crate) fn new_statement_handler_proto<
	Block: NodeBlock,
	RuntimeApi,
	Net: NetworkBackend<Block, Hash>,
>(
	client: &TeyrchainClient<Block, RuntimeApi>,
	teyrchain_config: &Configuration,
	metrics: &pezsc_network::NotificationMetrics,
	net_config: &mut FullNetworkConfiguration<Block, Hash, Net>,
) -> pezsc_network_statement::StatementHandlerPrototype {
	let (statement_handler_proto, statement_config) =
		pezsc_network_statement::StatementHandlerPrototype::new::<_, _, Net>(
			client.chain_info().genesis_hash,
			teyrchain_config.chain_spec.fork_id(),
			metrics.clone(),
			Arc::clone(&net_config.peer_store_handle()),
		);
	net_config.add_notification_protocol(statement_config);
	statement_handler_proto
}

/// Build the statement store, spawn the tasks.
pub(crate) fn build_statement_store<
	Block: NodeBlock,
	RuntimeApi: ConstructNodeRuntimeApi<Block, TeyrchainClient<Block, RuntimeApi>>,
>(
	teyrchain_config: &Configuration,
	task_manager: &mut TaskManager,
	client: Arc<TeyrchainClient<Block, RuntimeApi>>,
	network: Arc<dyn NetworkService + 'static>,
	sync_service: Arc<pezsc_network_sync::service::syncing_service::SyncingService<Block>>,
	local_keystore: Arc<pezsc_keystore::LocalKeystore>,
	statement_handler_proto: pezsc_network_statement::StatementHandlerPrototype,
) -> pezsc_service::error::Result<Arc<Store>> {
	let statement_store = pezsc_statement_store::Store::new_shared(
		&teyrchain_config.data_path,
		Default::default(),
		client,
		local_keystore,
		teyrchain_config.prometheus_registry(),
		&task_manager.spawn_handle(),
	)
	.map_err(|e| pezsc_service::Error::Application(Box::new(e) as Box<_>))?;
	let statement_protocol_executor = {
		let spawn_handle = task_manager.spawn_handle();
		Box::new(move |fut| {
			spawn_handle.spawn("network-statement-validator", Some("networking"), fut);
		})
	};
	let statement_handler = statement_handler_proto.build(
		network,
		sync_service,
		statement_store.clone(),
		teyrchain_config.prometheus_registry(),
		statement_protocol_executor,
	)?;
	task_manager.spawn_handle().spawn(
		"network-statement-handler",
		Some("networking"),
		statement_handler.run(),
	);

	Ok(statement_store)
}
