// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! Pezkuwi service builder.

#![cfg(feature = "full-node")]

mod partial;
use partial::PezkuwiPartialComponents;
pub(crate) use partial::{new_partial, new_partial_basics};

use crate::{
	grandpa_support, open_database,
	overseer::{ExtendedOverseerGenArgs, OverseerGen, OverseerGenArgs},
	relay_chain_selection::SelectRelayChain,
	teyrchains_db, workers, Chain, Error, FullBackend, FullClient, IdentifyVariant,
	IsTeyrchainNode, GRANDPA_JUSTIFICATION_PERIOD, KEEP_FINALIZED_FOR_LIVE_NETWORKS,
};
use gum::info;
use pezframe_benchmarking_cli::BIZINIKIWI_REFERENCE_HARDWARE;
use pezkuwi_availability_recovery::FETCH_CHUNKS_THRESHOLD;
use pezkuwi_node_core_approval_voting::Config as ApprovalVotingConfig;
use pezkuwi_node_core_av_store::Config as AvailabilityConfig;
use pezkuwi_node_core_candidate_validation::Config as CandidateValidationConfig;
use pezkuwi_node_core_chain_selection::{
	self as chain_selection_subsystem, Config as ChainSelectionConfig,
};
use pezkuwi_node_core_dispute_coordinator::Config as DisputeCoordinatorConfig;
use pezkuwi_node_network_protocol::{
	peer_set::{PeerSet, PeerSetProtocolNames},
	request_response::{IncomingRequest, ReqProtocolNames},
};
use pezkuwi_node_subsystem_types::DefaultSubsystemClient;
use pezkuwi_overseer::{Handle, OverseerConnector};
use pezkuwi_primitives::Block;
use pezmmr_gadget::MmrGadget;
use pezsc_client_api::Backend;
use pezsc_network::config::FullNetworkConfiguration;
use pezsc_network_sync::WarpSyncConfig;
use pezsc_service::{Configuration, RpcHandlers, TaskManager};
use pezsc_sysinfo::Metric;
use pezsc_telemetry::TelemetryWorkerHandle;
use pezsc_transaction_pool_api::OffchainTransactionPoolFactory;
use pezsp_consensus_beefy::ecdsa_crypto;
use pezsp_runtime::traits::Block as BlockT;
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
	time::Duration,
};

/// Pezkuwi node service initialization parameters.
pub struct NewFullParams<OverseerGenerator: OverseerGen> {
	pub is_teyrchain_node: IsTeyrchainNode,
	pub enable_beefy: bool,
	/// Whether to enable the block authoring backoff on production networks
	/// where it isn't enabled by default.
	pub force_authoring_backoff: bool,
	pub telemetry_worker_handle: Option<TelemetryWorkerHandle>,
	/// The version of the node. TESTING ONLY: `None` can be passed to skip the node/worker version
	/// check, both on startup and in the workers.
	pub node_version: Option<String>,
	/// Whether the node is attempting to run as a secure validator.
	pub secure_validator_mode: bool,
	/// An optional path to a directory containing the workers.
	pub workers_path: Option<std::path::PathBuf>,
	/// Optional custom names for the prepare and execute workers.
	pub workers_names: Option<(String, String)>,
	/// An optional number of the maximum number of pvf execute workers.
	pub execute_workers_max_num: Option<usize>,
	/// An optional maximum number of pvf workers that can be spawned in the pvf prepare pool for
	/// tasks with the priority below critical.
	pub prepare_workers_soft_max_num: Option<usize>,
	/// An optional absolute number of pvf workers that can be spawned in the pvf prepare pool.
	pub prepare_workers_hard_max_num: Option<usize>,
	/// How long finalized data should be kept in the availability store (in hours)
	pub keep_finalized_for: Option<u32>,
	pub overseer_gen: OverseerGenerator,
	pub overseer_message_channel_capacity_override: Option<usize>,
	#[allow(dead_code)]
	pub malus_finality_delay: Option<u32>,
	pub hwbench: Option<pezsc_sysinfo::HwBench>,
	/// Set of invulnerable AH collator `PeerId`s
	pub invulnerable_ah_collators: HashSet<pezkuwi_node_network_protocol::PeerId>,
	/// Override for `HOLD_OFF_DURATION` constant .
	pub collator_protocol_hold_off: Option<Duration>,
}

/// Completely built pezkuwi node service.
pub struct NewFull {
	pub task_manager: TaskManager,
	pub client: Arc<FullClient>,
	pub overseer_handle: Option<Handle>,
	pub network: Arc<dyn pezsc_network::service::traits::NetworkService>,
	pub sync_service: Arc<pezsc_network_sync::SyncingService<Block>>,
	pub rpc_handlers: RpcHandlers,
	pub backend: Arc<FullBackend>,
}

pub struct PezkuwiServiceBuilder<OverseerGenerator, Network>
where
	OverseerGenerator: OverseerGen,
	Network: pezsc_network::NetworkBackend<Block, <Block as BlockT>::Hash>,
{
	config: Configuration,
	params: NewFullParams<OverseerGenerator>,
	overseer_connector: OverseerConnector,
	partial_components: PezkuwiPartialComponents<SelectRelayChain<FullBackend>>,
	net_config: FullNetworkConfiguration<Block, <Block as BlockT>::Hash, Network>,
}

impl<OverseerGenerator, Network> PezkuwiServiceBuilder<OverseerGenerator, Network>
where
	OverseerGenerator: OverseerGen,
	Network: pezsc_network::NetworkBackend<Block, <Block as BlockT>::Hash>,
{
	/// Create new pezkuwi service builder.
	pub fn new(
		mut config: Configuration,
		params: NewFullParams<OverseerGenerator>,
	) -> Result<PezkuwiServiceBuilder<OverseerGenerator, Network>, Error> {
		let basics = new_partial_basics(&mut config, params.telemetry_worker_handle.clone())?;

		let prometheus_registry = config.prometheus_registry().cloned();
		let overseer_connector = OverseerConnector::default();
		let overseer_handle = Handle::new(overseer_connector.handle());
		let auth_or_collator = config.role.is_authority() || params.is_teyrchain_node.is_collator();

		let select_chain = if auth_or_collator {
			let metrics = pezkuwi_node_subsystem_util::metrics::Metrics::register(
				prometheus_registry.as_ref(),
			)?;

			SelectRelayChain::new_with_overseer(
				basics.backend.clone(),
				overseer_handle.clone(),
				metrics,
				Some(basics.task_manager.spawn_handle()),
			)
		} else {
			SelectRelayChain::new_longest_chain(basics.backend.clone())
		};

		let partial_components =
			new_partial::<SelectRelayChain<_>>(&mut config, basics, select_chain)?;

		let net_config = pezsc_network::config::FullNetworkConfiguration::<_, _, Network>::new(
			&config.network,
			config.prometheus_config.as_ref().map(|cfg| cfg.registry.clone()),
		);

		Ok(PezkuwiServiceBuilder {
			config,
			params,
			overseer_connector,
			partial_components,
			net_config,
		})
	}

	/// Get the genesis hash of the pezkuwi service being built.
	pub fn genesis_hash(&self) -> <Block as BlockT>::Hash {
		self.partial_components.client.chain_info().genesis_hash
	}

	/// Add extra request-response protocol to the pezkuwi service.
	pub fn add_extra_request_response_protocol(
		&mut self,
		config: Network::RequestResponseProtocolConfig,
	) {
		self.net_config.add_request_response_protocol(config);
	}

	/// Build pezkuwi service.
	pub fn build(self) -> Result<NewFull, Error> {
		let Self {
			config,
			params:
				NewFullParams {
					is_teyrchain_node,
					enable_beefy,
					force_authoring_backoff,
					telemetry_worker_handle: _,
					node_version,
					secure_validator_mode,
					workers_path,
					workers_names,
					overseer_gen,
					overseer_message_channel_capacity_override,
					malus_finality_delay: _malus_finality_delay,
					hwbench,
					execute_workers_max_num,
					prepare_workers_soft_max_num,
					prepare_workers_hard_max_num,
					keep_finalized_for,
					invulnerable_ah_collators,
					collator_protocol_hold_off,
				},
			overseer_connector,
			partial_components:
				pezsc_service::PartialComponents::<_, _, SelectRelayChain<_>, _, _, _> {
					client,
					backend,
					mut task_manager,
					keystore_container,
					select_chain,
					import_queue,
					transaction_pool,
					other:
						(rpc_extensions_builder, import_setup, rpc_setup, slot_duration, mut telemetry),
				},
			mut net_config,
		} = self;

		let role = config.role;
		let auth_or_collator = config.role.is_authority() || is_teyrchain_node.is_collator();
		let is_offchain_indexing_enabled = config.offchain_worker.indexing_enabled;
		let force_authoring = config.force_authoring;
		let disable_grandpa = config.disable_grandpa;
		let name = config.network.node_name.clone();
		let backoff_authoring_blocks = if !force_authoring_backoff
			&& (config.chain_spec.is_pezkuwi() || config.chain_spec.is_dicle())
		{
			// the block authoring backoff is disabled by default on production networks
			None
		} else {
			let mut backoff =
				pezsc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default();

			if config.chain_spec.is_pezkuwichain()
				|| config.chain_spec.is_versi()
				|| config.chain_spec.is_dev()
			{
				// on testnets that are in flux (like pezkuwichain or versi), finality has stalled
				// sometimes due to operational issues and it's annoying to slow down block
				// production to 1 block per hour.
				backoff.max_interval = 10;
			}

			Some(backoff)
		};
		let shared_voter_state = rpc_setup;
		let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
		let auth_disc_public_addresses = config.network.public_addresses.clone();

		let genesis_hash = client.chain_info().genesis_hash;
		let peer_store_handle = net_config.peer_store_handle();

		let prometheus_registry = config.prometheus_registry().cloned();
		let metrics = Network::register_notification_metrics(
			config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
		);

		// Note: GrandPa is pushed before the Pezkuwi-specific protocols. This doesn't change
		// anything in terms of behaviour, but makes the logs more consistent with the other
		// Bizinikiwi nodes.
		let grandpa_protocol_name =
			pezsc_consensus_grandpa::protocol_standard_name(&genesis_hash, &config.chain_spec);
		let (grandpa_protocol_config, grandpa_notification_service) =
			pezsc_consensus_grandpa::grandpa_peers_set_config::<_, Network>(
				grandpa_protocol_name.clone(),
				metrics.clone(),
				Arc::clone(&peer_store_handle),
			);
		net_config.add_notification_protocol(grandpa_protocol_config);

		let beefy_gossip_proto_name =
			pezsc_consensus_beefy::gossip_protocol_name(&genesis_hash, config.chain_spec.fork_id());
		// `beefy_on_demand_justifications_handler` is given to `beefy-gadget` task to be run,
		// while `beefy_req_resp_cfg` is added to `config.network.request_response_protocols`.
		let (beefy_on_demand_justifications_handler, beefy_req_resp_cfg) =
			pezsc_consensus_beefy::communication::request_response::BeefyJustifsRequestHandler::new::<
				_,
				Network,
			>(
				&genesis_hash,
				config.chain_spec.fork_id(),
				client.clone(),
				prometheus_registry.clone(),
			);
		let beefy_notification_service = match enable_beefy {
			false => None,
			true => {
				let (beefy_notification_config, beefy_notification_service) =
					pezsc_consensus_beefy::communication::beefy_peers_set_config::<_, Network>(
						beefy_gossip_proto_name.clone(),
						metrics.clone(),
						Arc::clone(&peer_store_handle),
					);

				net_config.add_notification_protocol(beefy_notification_config);
				net_config.add_request_response_protocol(beefy_req_resp_cfg);
				Some(beefy_notification_service)
			},
		};

		// validation/collation protocols are enabled only if `Overseer` is enabled
		let peerset_protocol_names =
			PeerSetProtocolNames::new(genesis_hash, config.chain_spec.fork_id());

		// If this is a validator or running alongside a teyrchain node, we need to enable the
		// networking protocols.
		//
		// Collators and teyrchain full nodes require the collator and validator networking to send
		// collations and to be able to recover PoVs.
		let notification_services = if role.is_authority()
			|| is_teyrchain_node.is_running_alongside_teyrchain_node()
		{
			use pezkuwi_network_bridge::{peer_sets_info, IsAuthority};
			let is_authority = if role.is_authority() { IsAuthority::Yes } else { IsAuthority::No };

			peer_sets_info::<_, Network>(
				is_authority,
				&peerset_protocol_names,
				metrics.clone(),
				Arc::clone(&peer_store_handle),
			)
			.into_iter()
			.map(|(config, (peerset, service))| {
				net_config.add_notification_protocol(config);
				(peerset, service)
			})
			.collect::<HashMap<PeerSet, Box<dyn pezsc_network::NotificationService>>>()
		} else {
			std::collections::HashMap::new()
		};

		let req_protocol_names = ReqProtocolNames::new(&genesis_hash, config.chain_spec.fork_id());

		let (collation_req_v1_receiver, cfg) =
			IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
		net_config.add_request_response_protocol(cfg);
		let (collation_req_v2_receiver, cfg) =
			IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
		net_config.add_request_response_protocol(cfg);
		let (available_data_req_receiver, cfg) =
			IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
		net_config.add_request_response_protocol(cfg);
		let (pov_req_receiver, cfg) =
			IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
		net_config.add_request_response_protocol(cfg);
		let (chunk_req_v1_receiver, cfg) =
			IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
		net_config.add_request_response_protocol(cfg);
		let (chunk_req_v2_receiver, cfg) =
			IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
		net_config.add_request_response_protocol(cfg);

		let grandpa_hard_forks = if config.chain_spec.is_dicle() {
			grandpa_support::dicle_hard_forks()
		} else {
			Vec::new()
		};

		let warp_sync = Arc::new(pezsc_consensus_grandpa::warp_proof::NetworkProvider::new(
			backend.clone(),
			import_setup.1.shared_authority_set().clone(),
			grandpa_hard_forks,
		));

		let ext_overseer_args = if is_teyrchain_node.is_running_alongside_teyrchain_node() {
			None
		} else {
			let teyrchains_db = open_database(&config.database)?;
			let candidate_validation_config = if role.is_authority() {
				let (prep_worker_path, exec_worker_path) = workers::determine_workers_paths(
					workers_path,
					workers_names,
					node_version.clone(),
				)?;
				log::info!("🚀 Using prepare-worker binary at: {:?}", prep_worker_path);
				log::info!("🚀 Using execute-worker binary at: {:?}", exec_worker_path);

				Some(CandidateValidationConfig {
					artifacts_cache_path: config
						.database
						.path()
						.ok_or(Error::DatabasePathRequired)?
						.join("pvf-artifacts"),
					node_version,
					secure_validator_mode,
					prep_worker_path,
					exec_worker_path,
					// Default execution workers is 4 because we have 8 cores on the reference
					// hardware, and this accounts for 50% of that cpu capacity.
					pvf_execute_workers_max_num: execute_workers_max_num.unwrap_or(4),
					pvf_prepare_workers_soft_max_num: prepare_workers_soft_max_num.unwrap_or(1),
					pvf_prepare_workers_hard_max_num: prepare_workers_hard_max_num.unwrap_or(2),
				})
			} else {
				None
			};
			let (candidate_req_v2_receiver, cfg) =
				IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
			net_config.add_request_response_protocol(cfg);
			let (dispute_req_receiver, cfg) =
				IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
			net_config.add_request_response_protocol(cfg);
			let approval_voting_config = ApprovalVotingConfig {
				col_approval_data: teyrchains_db::REAL_COLUMNS.col_approval_data,
				slot_duration_millis: slot_duration.as_millis() as u64,
			};
			let dispute_coordinator_config = DisputeCoordinatorConfig {
				col_dispute_data: teyrchains_db::REAL_COLUMNS.col_dispute_coordinator_data,
			};
			let chain_selection_config = ChainSelectionConfig {
				col_data: teyrchains_db::REAL_COLUMNS.col_chain_selection_data,
				stagnant_check_interval: Default::default(),
				stagnant_check_mode: chain_selection_subsystem::StagnantCheckMode::PruneOnly,
			};

			// Dicle + testnets get a higher threshold, we are conservative on Pezkuwi for now.
			let fetch_chunks_threshold =
				if config.chain_spec.is_pezkuwi() { None } else { Some(FETCH_CHUNKS_THRESHOLD) };

			let availability_config = AvailabilityConfig {
				col_data: teyrchains_db::REAL_COLUMNS.col_availability_data,
				col_meta: teyrchains_db::REAL_COLUMNS.col_availability_meta,
				keep_finalized_for: if matches!(
					config.chain_spec.identify_chain(),
					Chain::Pezkuwichain
				) {
					keep_finalized_for.unwrap_or(1)
				} else {
					KEEP_FINALIZED_FOR_LIVE_NETWORKS
				},
			};

			Some(ExtendedOverseerGenArgs {
				keystore: keystore_container.local_keystore(),
				teyrchains_db,
				candidate_validation_config,
				availability_config,
				pov_req_receiver,
				chunk_req_v1_receiver,
				chunk_req_v2_receiver,
				candidate_req_v2_receiver,
				approval_voting_config,
				dispute_req_receiver,
				dispute_coordinator_config,
				chain_selection_config,
				fetch_chunks_threshold,
				invulnerable_ah_collators,
				collator_protocol_hold_off,
			})
		};

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

		if config.offchain_worker.enabled {
			use futures::FutureExt;

			task_manager.spawn_handle().spawn(
				"offchain-workers-runner",
				"offchain-work",
				pezsc_offchain::OffchainWorkers::new(pezsc_offchain::OffchainWorkerOptions {
					runtime_api_provider: client.clone(),
					keystore: Some(keystore_container.keystore()),
					offchain_db: backend.offchain_storage(),
					transaction_pool: Some(OffchainTransactionPoolFactory::new(
						transaction_pool.clone(),
					)),
					network_provider: Arc::new(network.clone()),
					is_validator: role.is_authority(),
					enable_http_requests: false,
					custom_extensions: move |_| vec![],
				})?
				.run(client.clone(), task_manager.spawn_handle())
				.boxed(),
			);
		}

		let network_config = config.network.clone();
		let rpc_handlers = pezsc_service::spawn_tasks(pezsc_service::SpawnTasksParams {
			config,
			backend: backend.clone(),
			client: client.clone(),
			keystore: keystore_container.keystore(),
			network: network.clone(),
			sync_service: sync_service.clone(),
			rpc_builder: Box::new(rpc_extensions_builder),
			transaction_pool: transaction_pool.clone(),
			task_manager: &mut task_manager,
			system_rpc_tx,
			tx_handler_controller,
			telemetry: telemetry.as_mut(),
			tracing_execute_block: None,
		})?;

		if let Some(hwbench) = hwbench {
			pezsc_sysinfo::print_hwbench(&hwbench);
			match BIZINIKIWI_REFERENCE_HARDWARE.check_hardware(&hwbench, role.is_authority()) {
				Err(err) if role.is_authority() => {
					if err
						.0
						.iter()
						.any(|failure| matches!(failure.metric, Metric::Blake2256Parallel { .. }))
					{
						log::warn!(
						"⚠️  Starting January 2025 the hardware will fail the minimal physical CPU cores requirements {} for role 'Authority',\n\
						    find out more when this will become mandatory at:\n\
						    https://wiki.network.pezkuwichain.io/docs/maintain-guides-how-to-validate-polkadot#reference-hardware",
						err
					);
					}
					if err
						.0
						.iter()
						.any(|failure| !matches!(failure.metric, Metric::Blake2256Parallel { .. }))
					{
						log::warn!(
						"⚠️  The hardware does not meet the minimal requirements {} for role 'Authority' find out more at:\n\
						https://wiki.network.pezkuwichain.io/docs/maintain-guides-how-to-validate-polkadot#reference-hardware",
						err
					);
					}
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

		let (block_import, link_half, babe_link, beefy_links) = import_setup;

		let overseer_client = client.clone();
		let spawner = task_manager.spawn_handle();

		let authority_discovery_service =
		// We need the authority discovery if this node is either a validator or running alongside a teyrchain node.
		// Teyrchains node require the authority discovery for finding relay chain validators for sending
		// their PoVs or recovering PoVs.
		if role.is_authority() || is_teyrchain_node.is_running_alongside_teyrchain_node() {
			use futures::StreamExt;
			use pezsc_network::{Event, NetworkEventStream};

			let authority_discovery_role = if role.is_authority() {
				pezsc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore())
			} else {
				// don't publish our addresses when we're not an authority (collator, pezcumulus, ..)
				pezsc_authority_discovery::Role::Discover
			};
			let dht_event_stream =
				network.event_stream("authority-discovery").filter_map(|e| async move {
					match e {
						Event::Dht(e) => Some(e),
						_ => None,
					}
				});
			let (worker, service) = pezsc_authority_discovery::new_worker_and_service_with_config(
				pezsc_authority_discovery::WorkerConfig {
					publish_non_global_ips: auth_disc_publish_non_global_ips,
					public_addresses: auth_disc_public_addresses,
					// Require that authority discovery records are signed.
					strict_record_validation: true,
					persisted_cache_directory: network_config.net_config_path,
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
				Some("authority-discovery"),
				Box::pin(worker.run()),
			);
			Some(service)
		} else {
			None
		};

		let runtime_client = Arc::new(DefaultSubsystemClient::new(
			overseer_client.clone(),
			OffchainTransactionPoolFactory::new(transaction_pool.clone()),
		));

		let overseer_handle = if let Some(authority_discovery_service) = authority_discovery_service
		{
			let (overseer, overseer_handle) = overseer_gen
				.generate::<pezsc_service::SpawnTaskHandle, DefaultSubsystemClient<FullClient>>(
					overseer_connector,
					OverseerGenArgs {
						runtime_client,
						network_service: network.clone(),
						sync_service: sync_service.clone(),
						authority_discovery_service,
						collation_req_v1_receiver,
						collation_req_v2_receiver,
						available_data_req_receiver,
						registry: prometheus_registry.as_ref(),
						spawner,
						is_teyrchain_node,
						overseer_message_channel_capacity_override,
						req_protocol_names,
						peerset_protocol_names,
						notification_services,
					},
					ext_overseer_args,
				)
				.map_err(|e| {
					gum::error!("Failed to init overseer: {}", e);
					e
				})?;
			let handle = Handle::new(overseer_handle.clone());

			{
				let handle = handle.clone();
				task_manager.spawn_essential_handle().spawn_blocking(
					"overseer",
					None,
					Box::pin(async move {
						use futures::{pin_mut, select, FutureExt};

						let forward = pezkuwi_overseer::forward_events(overseer_client, handle);

						let forward = forward.fuse();
						let overseer_fut = overseer.run().fuse();

						pin_mut!(overseer_fut);
						pin_mut!(forward);

						select! {
							() = forward => (),
							() = overseer_fut => (),
							complete => (),
						}
					}),
				);
			}
			Some(handle)
		} else {
			assert!(
				!auth_or_collator,
				"Precondition congruence (false) is guaranteed by manual checking. qed"
			);
			None
		};

		if role.is_authority() {
			let proposer = pezsc_basic_authorship::ProposerFactory::new(
				task_manager.spawn_handle(),
				client.clone(),
				transaction_pool.clone(),
				prometheus_registry.as_ref(),
				telemetry.as_ref().map(|x| x.handle()),
			);

			let client_clone = client.clone();
			let overseer_handle =
				overseer_handle.as_ref().ok_or(Error::AuthoritiesRequireRealOverseer)?.clone();
			let slot_duration = babe_link.config().slot_duration();
			let babe_config = pezsc_consensus_babe::BabeParams {
				keystore: keystore_container.keystore(),
				client: client.clone(),
				select_chain,
				block_import,
				env: proposer,
				sync_oracle: sync_service.clone(),
				justification_sync_link: sync_service.clone(),
				create_inherent_data_providers: move |parent, ()| {
					let client_clone = client_clone.clone();
					let overseer_handle = overseer_handle.clone();

					async move {
						let teyrchain =
						pezkuwi_node_core_teyrchains_inherent::TeyrchainsInherentDataProvider::new(
							client_clone,
							overseer_handle,
							parent,
						);

						let timestamp = pezsp_timestamp::InherentDataProvider::from_system_time();

						let slot =
						pezsp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

						Ok((slot, timestamp, teyrchain))
					}
				},
				force_authoring,
				backoff_authoring_blocks,
				babe_link,
				block_proposal_slot_portion: pezsc_consensus_babe::SlotProportion::new(2f32 / 3f32),
				max_block_proposal_slot_portion: None,
				telemetry: telemetry.as_ref().map(|x| x.handle()),
			};

			let babe = pezsc_consensus_babe::start_babe(babe_config)?;
			task_manager.spawn_essential_handle().spawn_blocking("babe", None, babe);
		}

		// if the node isn't actively participating in consensus then it doesn't
		// need a keystore, regardless of which protocol we use below.
		let keystore_opt =
			if role.is_authority() { Some(keystore_container.keystore()) } else { None };

		// beefy is enabled if its notification service exists
		if let Some(notification_service) = beefy_notification_service {
			let justifications_protocol_name =
				beefy_on_demand_justifications_handler.protocol_name();
			let network_params = pezsc_consensus_beefy::BeefyNetworkParams {
				network: Arc::new(network.clone()),
				sync: sync_service.clone(),
				gossip_protocol_name: beefy_gossip_proto_name,
				justifications_protocol_name,
				notification_service,
				_phantom: core::marker::PhantomData::<Block>,
			};
			let payload_provider = pezsp_consensus_beefy::mmr::MmrRootProvider::new(client.clone());
			let beefy_params = pezsc_consensus_beefy::BeefyParams {
				client: client.clone(),
				backend: backend.clone(),
				payload_provider,
				runtime: client.clone(),
				key_store: keystore_opt.clone(),
				network_params,
				min_block_delta: 8,
				prometheus_registry: prometheus_registry.clone(),
				links: beefy_links,
				on_demand_justifications_handler: beefy_on_demand_justifications_handler,
				is_authority: role.is_authority(),
			};

			let gadget = pezsc_consensus_beefy::start_beefy_gadget::<
				_,
				_,
				_,
				_,
				_,
				_,
				_,
				ecdsa_crypto::AuthorityId,
			>(beefy_params);

			// BEEFY is part of consensus, if it fails we'll bring the node down with it to make
			// sure it is noticed.
			task_manager
				.spawn_essential_handle()
				.spawn_blocking("beefy-gadget", None, gadget);
		}
		// When offchain indexing is enabled, MMR gadget should also run.
		if is_offchain_indexing_enabled {
			task_manager.spawn_essential_handle().spawn_blocking(
				"pezmmr-gadget",
				None,
				MmrGadget::start(
					client.clone(),
					backend.clone(),
					pezsp_mmr_primitives::INDEXING_PREFIX.to_vec(),
				),
			);
		}

		let config = pezsc_consensus_grandpa::Config {
			// FIXME bizinikiwi#1578 make this available through chainspec
			// Grandpa performance can be improved a bit by tuning this parameter, see:
			// https://github.com/pezkuwichain/pezkuwi-sdk/issues/157
			gossip_duration: Duration::from_millis(1000),
			justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
			name: Some(name),
			observer_enabled: false,
			keystore: keystore_opt,
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: grandpa_protocol_name,
		};

		let enable_grandpa = !disable_grandpa;
		if enable_grandpa {
			// start the full GRANDPA voter
			// NOTE: unlike in bizinikiwi we are currently running the full
			// GRANDPA voter protocol for all full nodes (regardless of whether
			// they're validators or not). at this point the full voter should
			// provide better guarantees of block and vote data availability than
			// the observer.

			let mut voting_rules_builder = pezsc_consensus_grandpa::VotingRulesBuilder::default();

			#[cfg(not(feature = "malus"))]
			let _malus_finality_delay = None;

			if let Some(delay) = _malus_finality_delay {
				info!(?delay, "Enabling malus finality delay",);
				voting_rules_builder =
					voting_rules_builder.add(pezsc_consensus_grandpa::BeforeBestBlockBy(delay));
			};

			let grandpa_config = pezsc_consensus_grandpa::GrandpaParams {
				config,
				link: link_half,
				network: network.clone(),
				sync: sync_service.clone(),
				voting_rule: voting_rules_builder.build(),
				prometheus_registry: prometheus_registry.clone(),
				shared_voter_state,
				telemetry: telemetry.as_ref().map(|x| x.handle()),
				notification_service: grandpa_notification_service,
				offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				),
			};

			task_manager.spawn_essential_handle().spawn_blocking(
				"grandpa-voter",
				None,
				pezsc_consensus_grandpa::run_grandpa_voter(grandpa_config)?,
			);
		}

		Ok(NewFull {
			task_manager,
			client,
			overseer_handle,
			network,
			sync_service,
			rpc_handlers,
			backend,
		})
	}
}

/// Create a new full node of arbitrary runtime and executor.
///
/// This is an advanced feature and not recommended for general use. Generally, `build_full` is
/// a better choice.
///
/// `workers_path` is used to get the path to the directory where auxiliary worker binaries reside.
/// If not specified, the main binary's directory is searched first, then `/usr/lib/pezkuwi` is
/// searched. If the path points to an executable rather then directory, that executable is used
/// both as preparation and execution worker (supposed to be used for tests only).
pub fn new_full<
	OverseerGenerator: OverseerGen,
	Network: pezsc_network::NetworkBackend<Block, <Block as BlockT>::Hash>,
>(
	config: Configuration,
	params: NewFullParams<OverseerGenerator>,
) -> Result<NewFull, Error> {
	PezkuwiServiceBuilder::<OverseerGenerator, Network>::new(config, params)?.build()
}
