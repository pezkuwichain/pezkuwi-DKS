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

#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over bizinikiwi service.

use pezkuwi_sdk::{
	pezsc_consensus_beefy as beefy, pezsc_consensus_grandpa as grandpa,
	pezsp_consensus_babe::inherents::BabeCreateInherentDataProviders,
	pezsp_consensus_beefy as beefy_primitives, *,
};

use crate::Cli;
use codec::Encode;
use futures::prelude::*;
use pez_kitchensink_runtime::RuntimeApi;
use pez_node_primitives::Block;
use pezframe_benchmarking_cli::BIZINIKIWI_REFERENCE_HARDWARE;
use pezframe_system_rpc_runtime_api::AccountNonceApi;
use pezkuwi_sdk::{pezsp_api::ProvideRuntimeApi, pezsp_core::Pair};
use pezsc_client_api::{Backend, BlockBackend};
use pezsc_consensus_babe::{self, SlotProportion};
use pezsc_network::{
	event::Event, service::traits::NetworkService, NetworkBackend, NetworkEventStream,
};
use pezsc_network_sync::{strategy::warp::WarpSyncConfig, SyncingService};
use pezsc_service::{
	config::Configuration, error::Error as ServiceError, RpcHandlers, TaskManager,
};
use pezsc_statement_store::Store as StatementStore;
use pezsc_telemetry::{Telemetry, TelemetryWorker};
use pezsc_transaction_pool::TransactionPoolHandle;
use pezsc_transaction_pool_api::OffchainTransactionPoolFactory;
use pezsp_runtime::{generic, traits::Block as BlockT, SaturatedConversion};
use std::{path::Path, sync::Arc};

/// Host functions required for kitchensink runtime and Bizinikiwi node.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = (
	pezkuwi_sdk::pezsp_io::BizinikiwiHostFunctions,
	pezkuwi_sdk::pezsp_statement_store::runtime_api::HostFunctions,
);

/// Host functions required for kitchensink runtime and Bizinikiwi node.
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (
	pezkuwi_sdk::pezsp_io::BizinikiwiHostFunctions,
	pezkuwi_sdk::pezsp_statement_store::runtime_api::HostFunctions,
	pezframe_benchmarking::benchmarking::HostFunctions,
);

/// A specialized `WasmExecutor` intended to use across bizinikiwi node. It provides all required
/// HostFunctions.
pub type RuntimeExecutor = pezsc_executor::WasmExecutor<HostFunctions>;

/// The full client type definition.
pub type FullClient = pezsc_service::TFullClient<Block, RuntimeApi, RuntimeExecutor>;
type FullBackend = pezsc_service::TFullBackend<Block>;
type FullSelectChain = pezsc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport =
	grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;
type FullBeefyBlockImport<InnerBlockImport> = beefy::import::BeefyBlockImport<
	Block,
	FullBackend,
	FullClient,
	InnerBlockImport,
	beefy_primitives::ecdsa_crypto::AuthorityId,
>;

/// The transaction pool type definition.
pub type TransactionPool = pezsc_transaction_pool::TransactionPoolHandle<Block, FullClient>;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

/// Fetch the nonce of the given `account` from the chain state.
///
/// Note: Should only be used for tests.
pub fn fetch_nonce(client: &FullClient, account: pezsp_core::sr25519::Pair) -> u32 {
	let best_hash = client.chain_info().best_hash;
	client
		.runtime_api()
		.account_nonce(best_hash, account.public().into())
		.expect("Fetching account nonce works; qed")
}

/// Create a transaction using the given `call`.
///
/// The transaction will be signed by `sender`. If `nonce` is `None` it will be fetched from the
/// state of the best block.
///
/// Note: Should only be used for tests.
pub fn create_extrinsic(
	client: &FullClient,
	sender: pezsp_core::sr25519::Pair,
	function: impl Into<pez_kitchensink_runtime::RuntimeCall>,
	nonce: Option<u32>,
) -> pez_kitchensink_runtime::UncheckedExtrinsic {
	let function = function.into();
	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let best_hash = client.chain_info().best_hash;
	let best_block = client.chain_info().best_number;
	let nonce = nonce.unwrap_or_else(|| fetch_nonce(client, sender.clone()));

	let period = pez_kitchensink_runtime::BlockHashCount::get()
		.checked_next_power_of_two()
		.map(|c| c / 2)
		.unwrap_or(2) as u64;
	let tip = 0;
	let tx_ext: pez_kitchensink_runtime::TxExtension = (
		pezframe_system::AuthorizeCall::<pez_kitchensink_runtime::Runtime>::new(),
		pezframe_system::CheckNonZeroSender::<pez_kitchensink_runtime::Runtime>::new(),
		pezframe_system::CheckSpecVersion::<pez_kitchensink_runtime::Runtime>::new(),
		pezframe_system::CheckTxVersion::<pez_kitchensink_runtime::Runtime>::new(),
		pezframe_system::CheckGenesis::<pez_kitchensink_runtime::Runtime>::new(),
		pezframe_system::CheckEra::<pez_kitchensink_runtime::Runtime>::from(generic::Era::mortal(
			period,
			best_block.saturated_into(),
		)),
		pezframe_system::CheckNonce::<pez_kitchensink_runtime::Runtime>::from(nonce),
		pezframe_system::CheckWeight::<pez_kitchensink_runtime::Runtime>::new(),
		pezpallet_skip_feeless_payment::SkipCheckIfFeeless::from(
			pezpallet_asset_conversion_tx_payment::ChargeAssetTxPayment::<
				pez_kitchensink_runtime::Runtime,
			>::from(tip, None),
		),
		pezframe_metadata_hash_extension::CheckMetadataHash::new(false),
		pezpallet_revive::evm::tx_extension::SetOrigin::<pez_kitchensink_runtime::Runtime>::default(
		),
		pezframe_system::WeightReclaim::<pez_kitchensink_runtime::Runtime>::new(),
	);

	let raw_payload = pez_kitchensink_runtime::SignedPayload::from_raw(
		function.clone(),
		tx_ext.clone(),
		(
			(),
			(),
			pez_kitchensink_runtime::VERSION.spec_version,
			pez_kitchensink_runtime::VERSION.transaction_version,
			genesis_hash,
			best_hash,
			(),
			(),
			(),
			None,
			(),
			(),
		),
	);
	let signature = raw_payload.using_encoded(|e| sender.sign(e));

	generic::UncheckedExtrinsic::new_signed(
		function,
		pezsp_runtime::AccountId32::from(sender.public()).into(),
		pez_kitchensink_runtime::Signature::Sr25519(signature),
		tx_ext,
	)
	.into()
}

/// Creates a new partial node.
pub fn new_partial(
	config: &Configuration,
	mixnet_config: Option<&pezsc_mixnet::Config>,
) -> Result<
	pezsc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		pezsc_consensus::DefaultImportQueue<Block>,
		pezsc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
		(
			impl Fn(
				pezsc_rpc::SubscriptionTaskExecutor,
			) -> Result<jsonrpsee::RpcModule<()>, pezsc_service::Error>,
			(
				pezsc_consensus_babe::BabeBlockImport<
					Block,
					FullClient,
					FullBeefyBlockImport<FullGrandpaBlockImport>,
					BabeCreateInherentDataProviders<Block>,
					FullSelectChain,
				>,
				grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
				pezsc_consensus_babe::BabeLink<Block>,
				beefy::BeefyVoterLinks<Block, beefy_primitives::ecdsa_crypto::AuthorityId>,
			),
			grandpa::SharedVoterState,
			Option<Telemetry>,
			Arc<StatementStore>,
			Option<pezsc_mixnet::ApiBackend>,
		),
	>,
	ServiceError,
> {
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

	let (grandpa_block_import, grandpa_link) = grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	let justification_import = grandpa_block_import.clone();

	let (beefy_block_import, beefy_voter_links, beefy_rpc_links) =
		beefy::beefy_block_import_and_links(
			grandpa_block_import,
			backend.clone(),
			client.clone(),
			config.prometheus_registry().cloned(),
		);

	let babe_config = pezsc_consensus_babe::configuration(&*client)?;
	let slot_duration = babe_config.slot_duration();
	let (block_import, babe_link) = pezsc_consensus_babe::block_import(
		babe_config,
		beefy_block_import,
		client.clone(),
		Arc::new(move |_, _| async move {
			let timestamp = pezkuwi_sdk::pezsp_timestamp::InherentDataProvider::from_system_time();
			let slot =
			pezkuwi_sdk::pezsp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
				*timestamp,
				slot_duration,
			);
			Ok((slot, timestamp))
		}) as BabeCreateInherentDataProviders<Block>,
		select_chain.clone(),
		OffchainTransactionPoolFactory::new(transaction_pool.clone()),
	)?;

	let (import_queue, babe_worker_handle) =
		pezsc_consensus_babe::import_queue(pezsc_consensus_babe::ImportQueueParams {
			link: babe_link.clone(),
			block_import: block_import.clone(),
			justification_import: Some(Box::new(justification_import)),
			client: client.clone(),
			slot_duration,
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		})?;

	let import_setup = (block_import, grandpa_link, babe_link, beefy_voter_links);

	let statement_store = pezsc_statement_store::Store::new_shared(
		&config.data_path,
		Default::default(),
		client.clone(),
		keystore_container.local_keystore(),
		config.prometheus_registry(),
		&task_manager.spawn_handle(),
	)
	.map_err(|e| ServiceError::Other(format!("Statement store error: {:?}", e)))?;

	let (mixnet_api, mixnet_api_backend) = mixnet_config.map(pezsc_mixnet::Api::new).unzip();

	let (rpc_extensions_builder, rpc_setup) = {
		let (_, grandpa_link, _, _) = &import_setup;

		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let shared_voter_state = grandpa::SharedVoterState::empty();
		let shared_voter_state2 = shared_voter_state.clone();

		let finality_proof_provider = grandpa::FinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);

		let client = client.clone();
		let pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.keystore();
		let chain_spec = config.chain_spec.cloned_box();

		let rpc_backend = backend.clone();
		let rpc_statement_store = statement_store.clone();
		let rpc_extensions_builder =
			move |subscription_executor: pez_node_rpc::SubscriptionTaskExecutor| {
				let deps = pez_node_rpc::FullDeps {
					client: client.clone(),
					pool: pool.clone(),
					select_chain: select_chain.clone(),
					chain_spec: chain_spec.cloned_box(),
					babe: pez_node_rpc::BabeDeps {
						keystore: keystore.clone(),
						babe_worker_handle: babe_worker_handle.clone(),
					},
					grandpa: pez_node_rpc::GrandpaDeps {
						shared_voter_state: shared_voter_state.clone(),
						shared_authority_set: shared_authority_set.clone(),
						justification_stream: justification_stream.clone(),
						subscription_executor: subscription_executor.clone(),
						finality_provider: finality_proof_provider.clone(),
					},
					beefy: pez_node_rpc::BeefyDeps::<beefy_primitives::ecdsa_crypto::AuthorityId> {
						beefy_finality_proof_stream: beefy_rpc_links
							.from_voter_justif_stream
							.clone(),
						beefy_best_block_stream: beefy_rpc_links
							.from_voter_best_beefy_stream
							.clone(),
						subscription_executor,
					},
					statement_store: rpc_statement_store.clone(),
					backend: rpc_backend.clone(),
					mixnet_api: mixnet_api.as_ref().cloned(),
				};

				pez_node_rpc::create_full(deps).map_err(Into::into)
			};

		(rpc_extensions_builder, shared_voter_state2)
	};

	Ok(pezsc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: (
			rpc_extensions_builder,
			import_setup,
			rpc_setup,
			telemetry,
			statement_store,
			mixnet_api_backend,
		),
	})
}

/// Result of [`new_full_base`].
pub struct NewFullBase {
	/// The task manager of the node.
	pub task_manager: TaskManager,
	/// The client instance of the node.
	pub client: Arc<FullClient>,
	/// The networking service of the node.
	pub network: Arc<dyn NetworkService>,
	/// The syncing service of the node.
	pub sync: Arc<SyncingService<Block>>,
	/// The transaction pool of the node.
	pub transaction_pool: Arc<TransactionPoolHandle<Block, FullClient>>,
	/// The rpc handlers of the node.
	pub rpc_handlers: RpcHandlers,
}

/// Creates a full service from the configuration.
pub fn new_full_base<N: NetworkBackend<Block, <Block as BlockT>::Hash>>(
	config: Configuration,
	mixnet_config: Option<pezsc_mixnet::Config>,
	disable_hardware_benchmarks: bool,
	with_startup_data: impl FnOnce(
		&pezsc_consensus_babe::BabeBlockImport<
			Block,
			FullClient,
			FullBeefyBlockImport<FullGrandpaBlockImport>,
			BabeCreateInherentDataProviders<Block>,
			FullSelectChain,
		>,
		&pezsc_consensus_babe::BabeLink<Block>,
	),
) -> Result<NewFullBase, ServiceError> {
	let is_offchain_indexing_enabled = config.offchain_worker.indexing_enabled;
	let role = config.role;
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks =
		Some(pezsc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let enable_offchain_worker = config.offchain_worker.enabled;

	let hwbench = (!disable_hardware_benchmarks)
		.then(|| {
			config.database.path().map(|database_path| {
				let _ = std::fs::create_dir_all(&database_path);
				pezsc_sysinfo::gather_hwbench(Some(database_path), &BIZINIKIWI_REFERENCE_HARDWARE)
			})
		})
		.flatten();

	let pezsc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other:
			(rpc_builder, import_setup, rpc_setup, mut telemetry, statement_store, mixnet_api_backend),
	} = new_partial(&config, mixnet_config.as_ref())?;

	let metrics = N::register_notification_metrics(
		config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
	);
	let shared_voter_state = rpc_setup;
	let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
	let auth_disc_public_addresses = config.network.public_addresses.clone();

	let mut net_config = pezsc_network::config::FullNetworkConfiguration::<_, _, N>::new(
		&config.network,
		config.prometheus_config.as_ref().map(|cfg| cfg.registry.clone()),
	);

	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let peer_store_handle = net_config.peer_store_handle();

	let grandpa_protocol_name = grandpa::protocol_standard_name(&genesis_hash, &config.chain_spec);
	let (grandpa_protocol_config, grandpa_notification_service) =
		grandpa::grandpa_peers_set_config::<_, N>(
			grandpa_protocol_name.clone(),
			metrics.clone(),
			Arc::clone(&peer_store_handle),
		);
	net_config.add_notification_protocol(grandpa_protocol_config);

	let beefy_gossip_proto_name =
		beefy::gossip_protocol_name(&genesis_hash, config.chain_spec.fork_id());
	// `beefy_on_demand_justifications_handler` is given to `beefy-gadget` task to be run,
	// while `beefy_req_resp_cfg` is added to `config.network.request_response_protocols`.
	let (beefy_on_demand_justifications_handler, beefy_req_resp_cfg) =
		beefy::communication::request_response::BeefyJustifsRequestHandler::new::<_, N>(
			&genesis_hash,
			config.chain_spec.fork_id(),
			client.clone(),
			prometheus_registry.clone(),
		);

	let (beefy_notification_config, beefy_notification_service) =
		beefy::communication::beefy_peers_set_config::<_, N>(
			beefy_gossip_proto_name.clone(),
			metrics.clone(),
			Arc::clone(&peer_store_handle),
		);

	net_config.add_notification_protocol(beefy_notification_config);
	net_config.add_request_response_protocol(beefy_req_resp_cfg);

	let (statement_handler_proto, statement_config) =
		pezsc_network_statement::StatementHandlerPrototype::new::<_, _, N>(
			genesis_hash,
			config.chain_spec.fork_id(),
			metrics.clone(),
			Arc::clone(&peer_store_handle),
		);
	net_config.add_notification_protocol(statement_config);

	let mixnet_protocol_name =
		pezsc_mixnet::protocol_name(genesis_hash.as_ref(), config.chain_spec.fork_id());
	let mixnet_notification_service = mixnet_config.as_ref().map(|mixnet_config| {
		let (config, notification_service) = pezsc_mixnet::peers_set_config::<_, N>(
			mixnet_protocol_name.clone(),
			mixnet_config,
			metrics.clone(),
			Arc::clone(&peer_store_handle),
		);
		net_config.add_notification_protocol(config);
		notification_service
	});

	let warp_sync = Arc::new(grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		import_setup.1.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, sync_service) =
		pezsc_service::build_network(pezsc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config: Some(WarpSyncConfig::WithProvider(warp_sync)),
			block_relay: None,
			metrics,
		})?;

	if let Some(mixnet_config) = mixnet_config {
		let mixnet = pezsc_mixnet::run(
			mixnet_config,
			mixnet_api_backend.expect("Mixnet API backend created if mixnet enabled"),
			client.clone(),
			sync_service.clone(),
			network.clone(),
			mixnet_protocol_name,
			transaction_pool.clone(),
			Some(keystore_container.keystore()),
			mixnet_notification_service
				.expect("`NotificationService` exists since mixnet was enabled; qed"),
		);
		task_manager.spawn_handle().spawn("mixnet", None, mixnet);
	}

	let net_config_path = config.network.net_config_path.clone();
	let rpc_handlers = pezsc_service::spawn_tasks(pezsc_service::SpawnTasksParams {
		config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore_container.keystore(),
		network: network.clone(),
		rpc_builder: Box::new(rpc_builder),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		telemetry: telemetry.as_mut(),
		tracing_execute_block: None,
	})?;

	if let Some(hwbench) = hwbench {
		pezsc_sysinfo::print_hwbench(&hwbench);
		match BIZINIKIWI_REFERENCE_HARDWARE.check_hardware(&hwbench, false) {
			Err(err) if role.is_authority() => {
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

	let (block_import, grandpa_link, babe_link, beefy_links) = import_setup;

	(with_startup_data)(&block_import, &babe_link);

	if let pezsc_service::config::Role::Authority { .. } = &role {
		let proposer = pezsc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let client_clone = client.clone();
		let slot_duration = babe_link.config().slot_duration();
		let babe_config = pezsc_consensus_babe::BabeParams {
			keystore: keystore_container.keystore(),
			client: client.clone(),
			select_chain,
			env: proposer,
			block_import,
			sync_oracle: sync_service.clone(),
			justification_sync_link: sync_service.clone(),
			create_inherent_data_providers: move |parent, ()| {
				let client_clone = client_clone.clone();
				async move {
					let timestamp =
						pezkuwi_sdk::pezsp_timestamp::InherentDataProvider::from_system_time();

					let slot =
						pezkuwi_sdk::pezsp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

					let storage_proof =
						pezkuwi_sdk::pezsp_transaction_storage_proof::registration::new_data_provider(
							&*client_clone,
							&parent,
						)?;

					Ok((slot, timestamp, storage_proof))
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let babe = pezsc_consensus_babe::start_babe(babe_config)?;
		task_manager.spawn_essential_handle().spawn_blocking(
			"babe-proposer",
			Some("block-authoring"),
			babe,
		);
	}

	// Spawn authority discovery module.
	if role.is_authority() {
		let authority_discovery_role =
			pezsc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
		let dht_event_stream =
			network.event_stream("authority-discovery").filter_map(|e| async move {
				match e {
					Event::Dht(e) => Some(e),
					_ => None,
				}
			});
		let (authority_discovery_worker, _service) =
			pezsc_authority_discovery::new_worker_and_service_with_config(
				pezsc_authority_discovery::WorkerConfig {
					publish_non_global_ips: auth_disc_publish_non_global_ips,
					public_addresses: auth_disc_public_addresses,
					persisted_cache_directory: net_config_path,
					..Default::default()
				},
				client.clone(),
				Arc::new(network.clone()),
				Box::pin(dht_event_stream),
				authority_discovery_role,
				prometheus_registry.clone(),
				task_manager.spawn_handle(),
			);

		task_manager.spawn_handle().spawn(
			"authority-discovery-worker",
			Some("networking"),
			authority_discovery_worker.run(),
		);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore = if role.is_authority() { Some(keystore_container.keystore()) } else { None };

	// beefy is enabled if its notification service exists
	let network_params = beefy::BeefyNetworkParams {
		network: Arc::new(network.clone()),
		sync: sync_service.clone(),
		gossip_protocol_name: beefy_gossip_proto_name,
		justifications_protocol_name: beefy_on_demand_justifications_handler.protocol_name(),
		notification_service: beefy_notification_service,
		_phantom: core::marker::PhantomData::<Block>,
	};
	let beefy_params = beefy::BeefyParams {
		client: client.clone(),
		backend: backend.clone(),
		payload_provider: pezsp_consensus_beefy::mmr::MmrRootProvider::new(client.clone()),
		runtime: client.clone(),
		key_store: keystore.clone(),
		network_params,
		min_block_delta: 8,
		prometheus_registry: prometheus_registry.clone(),
		links: beefy_links,
		on_demand_justifications_handler: beefy_on_demand_justifications_handler,
		is_authority: role.is_authority(),
	};

	let beefy_gadget = beefy::start_beefy_gadget::<_, _, _, _, _, _, _, _>(beefy_params);
	// BEEFY is part of consensus, if it fails we'll bring the node down with it to make sure it
	// is noticed.
	task_manager
		.spawn_essential_handle()
		.spawn_blocking("beefy-gadget", None, beefy_gadget);
	// When offchain indexing is enabled, MMR gadget should also run.
	if is_offchain_indexing_enabled {
		task_manager.spawn_essential_handle().spawn_blocking(
			"pezmmr-gadget",
			None,
			pezmmr_gadget::MmrGadget::start(
				client.clone(),
				backend.clone(),
				pezkuwi_sdk::pezsp_mmr_primitives::INDEXING_PREFIX.to_vec(),
			),
		);
	}

	let grandpa_config = grandpa::Config {
		// FIXME #1578 make this available through chainspec
		gossip_duration: std::time::Duration::from_millis(333),
		justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
		name: Some(name),
		observer_enabled: false,
		keystore,
		local_role: role,
		telemetry: telemetry.as_ref().map(|x| x.handle()),
		protocol_name: grandpa_protocol_name,
	};

	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_params = grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network: network.clone(),
			sync: Arc::new(sync_service.clone()),
			notification_service: grandpa_notification_service,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			voting_rule: grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry: prometheus_registry.clone(),
			shared_voter_state,
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			grandpa::run_grandpa_voter(grandpa_params)?,
		);
	}

	// Spawn statement protocol worker
	let statement_protocol_executor = {
		let spawn_handle = task_manager.spawn_handle();
		Box::new(move |fut| {
			spawn_handle.spawn("network-statement-validator", Some("networking"), fut);
		})
	};
	let statement_handler = statement_handler_proto.build(
		network.clone(),
		sync_service.clone(),
		statement_store.clone(),
		prometheus_registry.as_ref(),
		statement_protocol_executor,
	)?;
	task_manager.spawn_handle().spawn(
		"network-statement-handler",
		Some("networking"),
		statement_handler.run(),
	);

	if enable_offchain_worker {
		let offchain_workers =
			pezsc_offchain::OffchainWorkers::new(pezsc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				keystore: Some(keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				is_validator: role.is_authority(),
				enable_http_requests: true,
				custom_extensions: move |_| {
					vec![Box::new(statement_store.clone().as_statement_store_ext()) as Box<_>]
				},
			})?;
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-work",
			offchain_workers.run(client.clone(), task_manager.spawn_handle()).boxed(),
		);
	}

	Ok(NewFullBase {
		task_manager,
		client,
		network,
		sync: sync_service,
		transaction_pool,
		rpc_handlers,
	})
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration, cli: Cli) -> Result<TaskManager, ServiceError> {
	let mixnet_config = cli.mixnet_params.config(config.role.is_authority());
	let database_path = config.database.path().map(Path::to_path_buf);

	let task_manager = match config.network.network_backend {
		pezsc_network::config::NetworkBackendType::Libp2p => {
			let task_manager = new_full_base::<pezsc_network::NetworkWorker<_, _>>(
				config,
				mixnet_config,
				cli.no_hardware_benchmarks,
				|_, _| (),
			)
			.map(|NewFullBase { task_manager, .. }| task_manager)?;
			task_manager
		},
		pezsc_network::config::NetworkBackendType::Litep2p => {
			let task_manager = new_full_base::<pezsc_network::Litep2pNetworkBackend>(
				config,
				mixnet_config,
				cli.no_hardware_benchmarks,
				|_, _| (),
			)
			.map(|NewFullBase { task_manager, .. }| task_manager)?;
			task_manager
		},
	};

	if let Some(database_path) = database_path {
		pezsc_storage_monitor::StorageMonitorService::try_spawn(
			cli.storage_monitor,
			database_path,
			&task_manager.spawn_essential_handle(),
		)
		.map_err(|e| ServiceError::Application(e.into()))?;
	}

	Ok(task_manager)
}

#[cfg(test)]
mod tests {
	use crate::service::{new_full_base, NewFullBase};
	use codec::Encode;
	use pez_kitchensink_runtime::{
		constants::{currency::CENTS, time::SLOT_DURATION},
		Address, BalancesCall, RuntimeCall, TxExtension,
	};
	use pez_node_primitives::{Block, DigestItem, Signature};
	use pezkuwi_sdk::{
		pezsc_transaction_pool_api::MaintainedTransactionPool,
		pezsp_consensus::{BlockOrigin, Environment, Proposer},
		pezsp_consensus_babe,
		pezsp_core::crypto::Pair,
		pezsp_inherents::InherentDataProvider,
		pezsp_keystore::KeystorePtr,
		pezsp_timestamp, pezsp_tracing, *,
	};
	use pezsc_client_api::BlockBackend;
	use pezsc_consensus::{BlockImport, BlockImportParams, ForkChoiceStrategy};
	use pezsc_consensus_babe::{BabeIntermediate, CompatibleDigestItem, INTERMEDIATE_KEY};
	use pezsc_consensus_epochs::descendent_query;
	use pezsc_keystore::LocalKeystore;
	use pezsc_service_test::TestNetNode;
	use pezsc_transaction_pool_api::ChainEvent;
	use pezsp_keyring::Sr25519Keyring;
	use pezsp_runtime::{
		generic::{self, Digest, Era, SignedPayload},
		key_types::BABE,
		traits::{Block as BlockT, Header as HeaderT, IdentifyAccount, Verify},
		RuntimeAppPublic,
	};
	use std::sync::Arc;

	type AccountPublic = <Signature as Verify>::Signer;

	#[test]
	// It is "ignored", but the node-cli ignored tests are running on the CI.
	// This can be run locally with `cargo test --release -p node-cli test_sync -- --ignored`.
	#[ignore]
	fn test_sync() {
		pezsp_tracing::try_init_simple();

		let keystore_path = tempfile::tempdir().expect("Creates keystore path");
		let keystore: KeystorePtr = LocalKeystore::open(keystore_path.path(), None)
			.expect("Creates keystore")
			.into();
		let alice: pezsp_consensus_babe::AuthorityId = keystore
			.sr25519_generate_new(BABE, Some("//Alice"))
			.expect("Creates authority pair")
			.into();

		let chain_spec = crate::chain_spec::tests::integration_test_config_with_single_authority();

		// For the block factory
		let mut slot = 1u64;

		// For the extrinsics factory
		let bob = Arc::new(Sr25519Keyring::Bob.pair());
		let charlie = Arc::new(Sr25519Keyring::Charlie.pair());
		let mut index = 0;

		pezsc_service_test::sync(
			chain_spec,
			|config| {
				let mut setup_handles = None;
				let NewFullBase { task_manager, client, network, sync, transaction_pool, .. } =
					new_full_base::<pezsc_network::NetworkWorker<_, _>>(
						config,
						None,
						false,
						|block_import: &pezsc_consensus_babe::BabeBlockImport<
							Block,
							_,
							_,
							_,
							_,
						>,
						 babe_link: &pezsc_consensus_babe::BabeLink<Block>| {
							setup_handles = Some((block_import.clone(), babe_link.clone()));
						},
					)?;

				let node = pezsc_service_test::TestNetComponents::new(
					task_manager,
					client,
					network,
					sync,
					transaction_pool,
				);
				Ok((node, setup_handles.unwrap()))
			},
			|service, &mut (ref mut block_import, ref babe_link)| {
				let parent_hash = service.client().chain_info().best_hash;
				let parent_header = service.client().header(parent_hash).unwrap().unwrap();
				let parent_number = *parent_header.number();

				futures::executor::block_on(service.transaction_pool().maintain(
					ChainEvent::NewBestBlock { hash: parent_header.hash(), tree_route: None },
				));

				let mut proposer_factory = pezsc_basic_authorship::ProposerFactory::new(
					service.spawn_handle(),
					service.client(),
					service.transaction_pool(),
					None,
					None,
				);

				let mut digest = Digest::default();

				// even though there's only one authority some slots might be empty,
				// so we must keep trying the next slots until we can claim one.
				let (babe_pre_digest, epoch_descriptor) = loop {
					let epoch_descriptor = babe_link
						.epoch_changes()
						.shared_data()
						.epoch_descriptor_for_child_of(
							descendent_query(&*service.client()),
							&parent_hash,
							parent_number,
							slot.into(),
						)
						.unwrap()
						.unwrap();

					let epoch = babe_link
						.epoch_changes()
						.shared_data()
						.epoch_data(&epoch_descriptor, |slot| {
							pezsc_consensus_babe::Epoch::genesis(babe_link.config(), slot)
						})
						.unwrap();

					if let Some(babe_pre_digest) =
						pezsc_consensus_babe::authorship::claim_slot(slot.into(), &epoch, &keystore)
							.map(|(digest, _)| digest)
					{
						break (babe_pre_digest, epoch_descriptor);
					}

					slot += 1;
				};

				let inherent_data = futures::executor::block_on(
					(
						pezsp_timestamp::InherentDataProvider::new(
							std::time::Duration::from_millis(SLOT_DURATION * slot).into(),
						),
						pezsp_consensus_babe::inherents::InherentDataProvider::new(slot.into()),
					)
						.create_inherent_data(),
				)
				.expect("Creates inherent data");

				digest.push(<DigestItem as CompatibleDigestItem>::babe_pre_digest(babe_pre_digest));

				let new_block = futures::executor::block_on(async move {
					let proposer = proposer_factory.init(&parent_header).await.unwrap();
					Proposer::propose(
						proposer,
						inherent_data,
						digest,
						std::time::Duration::from_secs(1),
						None,
					)
					.await
				})
				.expect("Error making test block")
				.block;

				let (new_header, new_body) = new_block.deconstruct();
				let pre_hash = new_header.hash();
				// sign the pre-sealed hash of the block and then
				// add it to a digest item.
				let to_sign = pre_hash.encode();
				let signature = keystore
					.sr25519_sign(pezsp_consensus_babe::AuthorityId::ID, alice.as_ref(), &to_sign)
					.unwrap()
					.unwrap();
				let item = <DigestItem as CompatibleDigestItem>::babe_seal(signature.into());
				slot += 1;

				let mut params = BlockImportParams::new(BlockOrigin::File, new_header);
				params.post_digests.push(item);
				params.body = Some(new_body);
				params.insert_intermediate(
					INTERMEDIATE_KEY,
					BabeIntermediate::<Block> { epoch_descriptor },
				);
				params.fork_choice = Some(ForkChoiceStrategy::LongestChain);

				futures::executor::block_on(block_import.import_block(params))
					.expect("error importing test block");
			},
			|service, _| {
				let amount = 5 * CENTS;
				let to: Address = AccountPublic::from(bob.public()).into_account().into();
				let from: Address = AccountPublic::from(charlie.public()).into_account().into();
				let genesis_hash = service.client().block_hash(0).unwrap().unwrap();
				let best_hash = service.client().chain_info().best_hash;
				let (spec_version, transaction_version) = {
					let version = service.client().runtime_version_at(best_hash).unwrap();
					(version.spec_version, version.transaction_version)
				};
				let signer = charlie.clone();

				let function = RuntimeCall::Balances(BalancesCall::transfer_allow_death {
					dest: to.into(),
					value: amount,
				});

				let authorize_call = pezframe_system::AuthorizeCall::new();
				let check_non_zero_sender = pezframe_system::CheckNonZeroSender::new();
				let check_spec_version = pezframe_system::CheckSpecVersion::new();
				let check_tx_version = pezframe_system::CheckTxVersion::new();
				let check_genesis = pezframe_system::CheckGenesis::new();
				let check_era = pezframe_system::CheckEra::from(Era::Immortal);
				let check_nonce = pezframe_system::CheckNonce::from(index);
				let check_weight = pezframe_system::CheckWeight::new();
				let tx_payment = pezpallet_skip_feeless_payment::SkipCheckIfFeeless::from(
					pezpallet_asset_conversion_tx_payment::ChargeAssetTxPayment::from(0, None),
				);
				let set_eth_origin = pezpallet_revive::evm::tx_extension::SetOrigin::default();
				let weight_reclaim = pezframe_system::WeightReclaim::new();
				let metadata_hash = pezframe_metadata_hash_extension::CheckMetadataHash::new(false);
				let tx_ext: TxExtension = (
					authorize_call,
					check_non_zero_sender,
					check_spec_version,
					check_tx_version,
					check_genesis,
					check_era,
					check_nonce,
					check_weight,
					tx_payment,
					metadata_hash,
					set_eth_origin,
					weight_reclaim,
				);
				let raw_payload = SignedPayload::from_raw(
					function,
					tx_ext,
					(
						(),
						(),
						spec_version,
						transaction_version,
						genesis_hash,
						genesis_hash,
						(),
						(),
						(),
						None,
						(),
						(),
					),
				);
				let signature = raw_payload.using_encoded(|payload| signer.sign(payload));
				let (function, tx_ext, _) = raw_payload.deconstruct();
				index += 1;
				let utx: pez_kitchensink_runtime::UncheckedExtrinsic =
					generic::UncheckedExtrinsic::new_signed(
						function,
						from.into(),
						signature.into(),
						tx_ext,
					)
					.into();

				utx.into()
			},
		);
	}

	#[test]
	#[ignore]
	fn test_consensus() {
		pezsp_tracing::try_init_simple();

		pezsc_service_test::consensus(
			crate::chain_spec::tests::integration_test_config_with_two_authorities(),
			|config| {
				let NewFullBase { task_manager, client, network, sync, transaction_pool, .. } =
					new_full_base::<pezsc_network::NetworkWorker<_, _>>(
						config,
						None,
						false,
						|_, _| (),
					)?;
				Ok(pezsc_service_test::TestNetComponents::new(
					task_manager,
					client,
					network,
					sync,
					transaction_pool,
				))
			},
			vec!["//Alice".into(), "//Bob".into()],
		)
	}
}
