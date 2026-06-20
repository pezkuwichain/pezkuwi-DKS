//! Service and ServiceFactory implementation. Specialized wrapper over bizinikiwi service.

// std
use std::{sync::Arc, time::Duration};

// Local Runtime Types
use teyrchain_template_runtime::{
	apis::RuntimeApi,
	opaque::{Block, Hash},
};

use codec::Encode;
use pezkuwi_sdk::{pezcumulus_client_service::TeyrchainTracingExecuteBlock, *};

// Pezcumulus Imports
use pezcumulus_client_bootnodes::{start_bootnode_tasks, StartBootnodeTasksParams};
use pezcumulus_client_cli::CollatorOptions;
use pezcumulus_client_collator::service::CollatorService;
#[docify::export(lookahead_collator)]
use pezcumulus_client_consensus_aura::collators::lookahead::{self as aura, Params as AuraParams};
use pezcumulus_client_consensus_common::TeyrchainBlockImport as TTeyrchainBlockImport;
use pezcumulus_client_service::{
	build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
	BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
	TeyrchainHostFunctions,
};
#[docify::export(pezcumulus_primitives)]
use pezcumulus_primitives_core::{
	relay_chain::{CollatorPair, ValidationCode},
	GetTeyrchainInfo, ParaId,
};
use pezcumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};

// Bizinikiwi Imports
use pezframe_benchmarking_cli::BIZINIKIWI_REFERENCE_HARDWARE;
use pezkuwi_sdk::{
	pezsc_network::PeerId, pezsp_api::ProvideRuntimeApi, pezsp_keystore::KeystorePtr,
};
use pezsc_client_api::Backend;
use pezsc_consensus::ImportQueue;
use pezsc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use pezsc_network::{NetworkBackend, NetworkBlock};
use pezsc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use pezsc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use pezsc_transaction_pool_api::OffchainTransactionPoolFactory;
use prometheus_endpoint::Registry;

#[docify::export(wasm_executor)]
type TeyrchainExecutor = WasmExecutor<TeyrchainHostFunctions>;

type TeyrchainClient = TFullClient<Block, RuntimeApi, TeyrchainExecutor>;

type TeyrchainBackend = TFullBackend<Block>;

type TeyrchainBlockImport = TTeyrchainBlockImport<Block, Arc<TeyrchainClient>, TeyrchainBackend>;

/// Assembly of PartialComponents (enough to run chain ops subcommands)
pub type Service = PartialComponents<
	TeyrchainClient,
	TeyrchainBackend,
	(),
	pezsc_consensus::DefaultImportQueue<Block>,
	pezsc_transaction_pool::TransactionPoolHandle<Block, TeyrchainClient>,
	(TeyrchainBlockImport, Option<Telemetry>, Option<TelemetryWorkerHandle>),
>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
#[docify::export(component_instantiation)]
pub fn new_partial(config: &Configuration) -> Result<Service, pezsc_service::Error> {
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

	let heap_pages = config
		.executor
		.default_heap_pages
		.map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static { extra_pages: h as _ });

	let executor = TeyrchainExecutor::builder()
		.with_execution_method(config.executor.wasm_method)
		.with_onchain_heap_alloc_strategy(heap_pages)
		.with_offchain_heap_alloc_strategy(heap_pages)
		.with_max_runtime_instances(config.executor.max_runtime_instances)
		.with_runtime_cache_size(config.executor.runtime_cache_size)
		.build();

	let (client, backend, keystore_container, task_manager) =
		pezsc_service::new_full_parts_record_import::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
			true,
		)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

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

	let block_import = TeyrchainBlockImport::new(client.clone(), backend.clone());

	let import_queue = build_import_queue(
		client.clone(),
		block_import.clone(),
		config,
		telemetry.as_ref().map(|telemetry| telemetry.handle()),
		&task_manager,
	);

	Ok(PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain: (),
		other: (block_import, telemetry, telemetry_worker_handle),
	})
}

/// Build the import queue for the teyrchain runtime.
fn build_import_queue(
	client: Arc<TeyrchainClient>,
	block_import: TeyrchainBlockImport,
	config: &Configuration,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
) -> pezsc_consensus::DefaultImportQueue<Block> {
	pezcumulus_client_consensus_aura::equivocation_import_queue::fully_verifying_import_queue::<
		pezsp_consensus_aura::sr25519::AuthorityPair,
		_,
		_,
		_,
		_,
	>(
		client,
		block_import,
		move |_, _| async move {
			let timestamp = pezkuwi_sdk::pezsp_timestamp::InherentDataProvider::from_system_time();
			Ok(timestamp)
		},
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
		telemetry,
	)
}

#[allow(clippy::too_many_arguments)]
fn start_consensus(
	client: Arc<TeyrchainClient>,
	backend: Arc<TeyrchainBackend>,
	block_import: TeyrchainBlockImport,
	prometheus_registry: Option<&Registry>,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
	relay_chain_interface: Arc<dyn RelayChainInterface>,
	transaction_pool: Arc<pezsc_transaction_pool::TransactionPoolHandle<Block, TeyrchainClient>>,
	keystore: KeystorePtr,
	relay_chain_slot_duration: Duration,
	para_id: ParaId,
	collator_key: CollatorPair,
	collator_peer_id: PeerId,
	overseer_handle: OverseerHandle,
	announce_block: Arc<dyn Fn(Hash, Option<Vec<u8>>) + Send + Sync>,
) -> Result<(), pezsc_service::Error> {
	let proposer = pezsc_basic_authorship::ProposerFactory::with_proof_recording(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool,
		prometheus_registry,
		telemetry.clone(),
	);
	let collator_service = CollatorService::new(
		client.clone(),
		Arc::new(task_manager.spawn_handle()),
		announce_block,
		client.clone(),
	);

	let params = AuraParams {
		create_inherent_data_providers: move |_, ()| async move { Ok(()) },
		block_import,
		para_client: client.clone(),
		para_backend: backend,
		relay_client: relay_chain_interface,
		code_hash_provider: move |block_hash| {
			client.code_at(block_hash).ok().map(|c| ValidationCode::from(c).hash())
		},
		keystore,
		collator_key,
		collator_peer_id,
		para_id,
		overseer_handle,
		relay_chain_slot_duration,
		proposer,
		collator_service,
		authoring_duration: Duration::from_millis(2000),
		reinitialize: false,
		max_pov_percentage: None,
	};
	let fut =
		aura::run::<Block, pezsp_consensus_aura::sr25519::AuthorityPair, _, _, _, _, _, _, _, _>(
			params,
		);
	task_manager.spawn_essential_handle().spawn("aura", None, fut);

	Ok(())
}

/// Start a node with the given teyrchain `Configuration` and relay chain `Configuration`.
#[pezsc_tracing::logging::prefix_logs_with("Teyrchain")]
pub async fn start_teyrchain_node(
	teyrchain_config: Configuration,
	pezkuwi_config: Configuration,
	collator_options: CollatorOptions,
	hwbench: Option<pezsc_sysinfo::HwBench>,
) -> pezsc_service::error::Result<(TaskManager, Arc<TeyrchainClient>)> {
	let teyrchain_config = prepare_node_config(teyrchain_config);

	let params = new_partial(&teyrchain_config)?;
	let (block_import, mut telemetry, telemetry_worker_handle) = params.other;

	let prometheus_registry = teyrchain_config.prometheus_registry().cloned();
	let net_config = pezsc_network::config::FullNetworkConfiguration::<
		_,
		_,
		pezsc_network::NetworkWorker<Block, Hash>,
	>::new(&teyrchain_config.network, prometheus_registry.clone());

	let client = params.client.clone();
	let backend = params.backend.clone();
	let mut task_manager = params.task_manager;

	let relay_chain_fork_id = pezkuwi_config.chain_spec.fork_id().map(ToString::to_string);
	let teyrchain_fork_id = teyrchain_config.chain_spec.fork_id().map(ToString::to_string);
	let advertise_non_global_ips = teyrchain_config.network.allow_non_globals_in_dht;
	let teyrchain_public_addresses = teyrchain_config.network.public_addresses.clone();

	let (relay_chain_interface, collator_key, relay_chain_network, paranode_rx) =
		build_relay_chain_interface(
			pezkuwi_config,
			&teyrchain_config,
			telemetry_worker_handle,
			&mut task_manager,
			collator_options.clone(),
			hwbench.clone(),
		)
		.await
		.map_err(|e| pezsc_service::Error::Application(Box::new(e) as Box<_>))?;

	let validator = teyrchain_config.role.is_authority();
	let transaction_pool = params.transaction_pool.clone();
	let import_queue_service = params.import_queue.service();

	// Take teyrchain id from runtime.
	let best_hash = client.chain_info().best_hash;
	let para_id = client
		.runtime_api()
		.teyrchain_id(best_hash)
		.map_err(|_| "Failed to retrieve teyrchain id from runtime. Make sure you implement `pezcumulus_primitives_core::GetParachaiNidentity` runtime API.")?;

	// NOTE: because we use Aura here explicitly, we can use `CollatorSybilResistance::Resistant`
	// when starting the network.
	let (network, system_rpc_tx, tx_handler_controller, sync_service) =
		build_network(BuildNetworkParams {
			teyrchain_config: &teyrchain_config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			para_id,
			spawn_handle: task_manager.spawn_handle(),
			relay_chain_interface: relay_chain_interface.clone(),
			import_queue: params.import_queue,
			sybil_resistance_level: CollatorSybilResistance::Resistant, // because of Aura
			metrics: pezsc_network::NetworkWorker::<Block, Hash>::register_notification_metrics(
				teyrchain_config.prometheus_config.as_ref().map(|config| &config.registry),
			),
		})
		.await?;
	let collator_peer_id = network.local_peer_id();

	if teyrchain_config.offchain_worker.enabled {
		use futures::FutureExt;

		let offchain_workers =
			pezsc_offchain::OffchainWorkers::new(pezsc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				keystore: Some(params.keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				is_validator: teyrchain_config.role.is_authority(),
				enable_http_requests: false,
				custom_extensions: move |_| vec![],
			})?;
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-work",
			offchain_workers.run(client.clone(), task_manager.spawn_handle()).boxed(),
		);
	}

	let rpc_builder = {
		let client = client.clone();
		let transaction_pool = transaction_pool.clone();

		Box::new(move |_| {
			let deps =
				crate::rpc::FullDeps { client: client.clone(), pool: transaction_pool.clone() };

			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	pezsc_service::spawn_tasks(pezsc_service::SpawnTasksParams {
		rpc_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: teyrchain_config,
		keystore: params.keystore_container.keystore(),
		backend: backend.clone(),
		network: network.clone(),
		sync_service: sync_service.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
		tracing_execute_block: Some(Arc::new(TeyrchainTracingExecuteBlock::new(client.clone()))),
	})?;

	if let Some(hwbench) = hwbench {
		pezsc_sysinfo::print_hwbench(&hwbench);
		// Here you can check whether the hardware meets your chains' requirements. Putting a link
		// in there and swapping out the requirements for your own are probably a good idea. The
		// requirements for a para-chain are dictated by its relay-chain.
		match BIZINIKIWI_REFERENCE_HARDWARE.check_hardware(&hwbench, false) {
			Err(err) if validator => {
				log::warn!(
				"⚠️  The hardware does not meet the minimal requirements {} for role 'Authority'.",
				err
			);
			},
			_ => {},
		}

		if let Some(ref mut telemetry) = telemetry {
			let telemetry_handle = telemetry.handle();
			task_manager.spawn_handle().spawn(
				"telemetry_hwbench",
				None,
				pezsc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
			);
		}
	}

	let announce_block = {
		let sync_service = sync_service.clone();
		Arc::new(move |hash, data| sync_service.announce_block(hash, data))
	};

	let relay_chain_slot_duration = Duration::from_secs(6);

	let overseer_handle = relay_chain_interface
		.overseer_handle()
		.map_err(|e| pezsc_service::Error::Application(Box::new(e)))?;

	start_relay_chain_tasks(StartRelayChainTasksParams {
		client: client.clone(),
		announce_block: announce_block.clone(),
		para_id,
		relay_chain_interface: relay_chain_interface.clone(),
		task_manager: &mut task_manager,
		da_recovery_profile: if validator {
			DARecoveryProfile::Collator
		} else {
			DARecoveryProfile::FullNode
		},
		import_queue: import_queue_service,
		relay_chain_slot_duration,
		recovery_handle: Box::new(overseer_handle.clone()),
		sync_service: sync_service.clone(),
		prometheus_registry: prometheus_registry.as_ref(),
	})?;

	start_bootnode_tasks(StartBootnodeTasksParams {
		embedded_dht_bootnode: collator_options.embedded_dht_bootnode,
		dht_bootnode_discovery: collator_options.dht_bootnode_discovery,
		para_id,
		task_manager: &mut task_manager,
		relay_chain_interface: relay_chain_interface.clone(),
		relay_chain_fork_id,
		relay_chain_network,
		request_receiver: paranode_rx,
		teyrchain_network: network,
		advertise_non_global_ips,
		teyrchain_genesis_hash: client.chain_info().genesis_hash.encode(),
		teyrchain_fork_id,
		teyrchain_public_addresses,
	});

	if validator {
		start_consensus(
			client.clone(),
			backend,
			block_import,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			relay_chain_interface,
			transaction_pool,
			params.keystore_container.keystore(),
			relay_chain_slot_duration,
			para_id,
			collator_key.expect("Command line arguments do not allow this. qed"),
			collator_peer_id,
			overseer_handle,
			announce_block,
		)?;
	}

	Ok((task_manager, client))
}
