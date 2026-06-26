// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
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

use crate::cli::Consensus;
use futures::FutureExt;
use pez_minimal_template_runtime::{interface::OpaqueBlock as Block, RuntimeApi};
use pezkuwi_sdk::{
	pezsc_client_api::backend::Backend,
	pezsc_executor::WasmExecutor,
	pezsc_service::{error::Error as ServiceError, Configuration, TaskManager},
	pezsc_telemetry::{Telemetry, TelemetryWorker},
	pezsc_transaction_pool_api::OffchainTransactionPoolFactory,
	*,
};
use pezsp_runtime::traits::Block as BlockT;
use std::sync::Arc;

type HostFunctions = pezkuwi_sdk::pezsp_io::BizinikiwiHostFunctions;

#[docify::export]
pub(crate) type FullClient =
	pezsc_service::TFullClient<Block, RuntimeApi, WasmExecutor<HostFunctions>>;

type FullBackend = pezsc_service::TFullBackend<Block>;
type FullSelectChain = pezsc_consensus::LongestChain<FullBackend, Block>;

/// Assembly of PartialComponents (enough to run chain ops subcommands)
pub type Service = pezsc_service::PartialComponents<
	FullClient,
	FullBackend,
	FullSelectChain,
	pezsc_consensus::DefaultImportQueue<Block>,
	pezsc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
	Option<Telemetry>,
>;

pub fn new_partial(config: &Configuration) -> Result<Service, ServiceError> {
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, pezsc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = pezsc_service::new_wasm_executor(&config.executor);

	let (client, backend, keystore_container, task_manager) =
		pezsc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = pezsc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = Arc::from(
		pezsc_transaction_pool::Builder::new(
			task_manager.spawn_essential_handle(),
			client.clone(),
			config.role.is_authority().into(),
		)
		.with_options(config.transaction_pool.clone())
		.with_prometheus(config.prometheus_registry())
		.build(),
	);

	let import_queue = pezsc_consensus_manual_seal::import_queue(
		Box::new(client.clone()),
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
	);

	Ok(pezsc_service::PartialComponents {
		client,
		backend,
		task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (telemetry),
	})
}

/// Builds a new service for a full client.
pub fn new_full<Network: pezsc_network::NetworkBackend<Block, <Block as BlockT>::Hash>>(
	config: Configuration,
	consensus: Consensus,
) -> Result<TaskManager, ServiceError> {
	let pezsc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: mut telemetry,
	} = new_partial(&config)?;

	let net_config = pezsc_network::config::FullNetworkConfiguration::<
		Block,
		<Block as BlockT>::Hash,
		Network,
	>::new(
		&config.network,
		config.prometheus_config.as_ref().map(|cfg| cfg.registry.clone()),
	);
	let metrics = Network::register_notification_metrics(
		config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
	);

	let (network, system_rpc_tx, tx_handler_controller, sync_service) =
		pezsc_service::build_network(pezsc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config: None,
			block_relay: None,
			metrics,
		})?;

	if config.offchain_worker.enabled {
		let offchain_workers =
			pezsc_offchain::OffchainWorkers::new(pezsc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				is_validator: config.role.is_authority(),
				keystore: Some(keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				enable_http_requests: true,
				custom_extensions: |_| vec![],
			})?;
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-worker",
			offchain_workers.run(client.clone(), task_manager.spawn_handle()).boxed(),
		);
	}

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();

		Box::new(move |_| {
			let deps = crate::rpc::FullDeps { client: client.clone(), pool: pool.clone() };
			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	let prometheus_registry = config.prometheus_registry().cloned();

	let _rpc_handlers = pezsc_service::spawn_tasks(pezsc_service::SpawnTasksParams {
		network,
		client: client.clone(),
		keystore: keystore_container.keystore(),
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_builder: rpc_extensions_builder,
		backend,
		system_rpc_tx,
		tx_handler_controller,
		sync_service,
		config,
		telemetry: telemetry.as_mut(),
		tracing_execute_block: None,
	})?;

	let proposer = pezsc_basic_authorship::ProposerFactory::new(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool.clone(),
		prometheus_registry.as_ref(),
		telemetry.as_ref().map(|x| x.handle()),
	);

	match consensus {
		Consensus::InstantSeal => {
			let params = pezsc_consensus_manual_seal::InstantSealParams {
				block_import: client.clone(),
				env: proposer,
				client,
				pool: transaction_pool,
				select_chain,
				consensus_data_provider: None,
				create_inherent_data_providers: move |_, ()| async move {
					Ok(pezkuwi_sdk::pezsp_timestamp::InherentDataProvider::from_system_time())
				},
			};

			let authorship_future = pezsc_consensus_manual_seal::run_instant_seal(params);

			task_manager.spawn_essential_handle().spawn_blocking(
				"instant-seal",
				None,
				authorship_future,
			);
		},
		Consensus::ManualSeal(block_time) => {
			let (mut sink, commands_stream) = futures::channel::mpsc::channel(1024);
			task_manager.spawn_handle().spawn("block_authoring", None, async move {
				loop {
					futures_timer::Delay::new(std::time::Duration::from_millis(block_time)).await;
					sink.try_send(pezsc_consensus_manual_seal::EngineCommand::SealNewBlock {
						create_empty: true,
						finalize: true,
						parent_hash: None,
						sender: None,
					})
					.unwrap();
				}
			});

			let params = pezsc_consensus_manual_seal::ManualSealParams {
				block_import: client.clone(),
				env: proposer,
				client,
				pool: transaction_pool,
				select_chain,
				commands_stream: Box::pin(commands_stream),
				consensus_data_provider: None,
				create_inherent_data_providers: move |_, ()| async move {
					Ok(pezkuwi_sdk::pezsp_timestamp::InherentDataProvider::from_system_time())
				},
			};
			let authorship_future = pezsc_consensus_manual_seal::run_manual_seal(params);

			task_manager.spawn_essential_handle().spawn_blocking(
				"manual-seal",
				None,
				authorship_future,
			);
		},
		_ => {},
	}

	Ok(task_manager)
}
