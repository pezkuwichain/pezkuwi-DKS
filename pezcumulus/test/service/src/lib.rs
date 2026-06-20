// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.

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

//! Crate used for testing with Pezcumulus.

#![warn(missing_docs)]

/// Utilities used for benchmarking
pub mod bench_utils;

pub mod chain_spec;

use pezcumulus_client_collator::service::CollatorService;
use pezcumulus_client_consensus_aura::{
	collators::{
		lookahead::{self as aura, Params as AuraParams},
		slot_based::{
			self as slot_based, Params as SlotBasedParams, SlotBasedBlockImport,
			SlotBasedBlockImportHandle,
		},
	},
	ImportQueueParams,
};
use pezsc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use pezsp_consensus_aura::sr25519::AuthorityPair;
use prometheus::Registry;
use runtime::AccountId;
use std::{
	collections::HashSet,
	future::Future,
	net::{Ipv4Addr, SocketAddr, SocketAddrV4},
	time::Duration,
};
use url::Url;

use crate::runtime::Weight;
use pezcumulus_client_cli::{CollatorOptions, RelayChainMode};
use pezcumulus_client_consensus_common::TeyrchainBlockImport as TTeyrchainBlockImport;
use pezcumulus_client_pov_recovery::{RecoveryDelayRange, RecoveryHandle};
use pezcumulus_client_service::{
	build_network, prepare_node_config, start_relay_chain_tasks, BuildNetworkParams,
	CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
	TeyrchainTracingExecuteBlock,
};
use pezcumulus_primitives_core::{relay_chain::ValidationCode, GetTeyrchainInfo, ParaId};
use pezcumulus_relay_chain_inprocess_interface::RelayChainInProcessInterface;
use pezcumulus_relay_chain_interface::{RelayChainError, RelayChainInterface, RelayChainResult};
use pezcumulus_relay_chain_minimal_node::build_minimal_relay_chain_node_with_rpc;

use pezcumulus_test_runtime::{Hash, NodeBlock as Block, RuntimeApi};

use bizinikiwi_test_client::{
	BlockchainEventsExt, RpcHandlersExt, RpcTransactionError, RpcTransactionOutput,
};
use pezframe_system_rpc_runtime_api::AccountNonceApi;
use pezkuwi_node_subsystem::{errors::RecoveryError, messages::AvailabilityRecoveryMessage};
use pezkuwi_overseer::Handle as OverseerHandle;
use pezkuwi_primitives::{CandidateHash, CollatorPair};
use pezkuwi_service::ProvideRuntimeApi;
use pezsc_consensus::ImportQueue;
use pezsc_network::{
	config::{FullNetworkConfiguration, TransportConfig},
	multiaddr,
	service::traits::NetworkService,
	NetworkBackend, NetworkBlock, NetworkStateInfo,
};
use pezsc_service::{
	config::{
		BlocksPruning, DatabaseSource, ExecutorConfiguration, KeystoreConfig, MultiaddrWithPeerId,
		NetworkConfiguration, OffchainWorkerConfig, PruningMode, RpcBatchRequestConfig,
		RpcConfiguration, RpcEndpoint, WasmExecutionMethod,
	},
	BasePath, ChainSpec as ChainSpecService, Configuration, Error as ServiceError,
	PartialComponents, Role, RpcHandlers, TFullBackend, TFullClient, TaskManager,
};
use pezsp_arithmetic::traits::SaturatedConversion;
use pezsp_blockchain::HeaderBackend;
use pezsp_core::Pair;
use pezsp_keyring::Sr25519Keyring;
use pezsp_runtime::{codec::Encode, generic, MultiAddress};
use pezsp_state_machine::BasicExternalities;
use std::sync::Arc;

pub use chain_spec::*;
pub use pezcumulus_test_runtime as runtime;
pub use pezsp_keyring::Sr25519Keyring as Keyring;

const LOG_TARGET: &str = "pezcumulus-test-service";

/// The signature of the announce block fn.
pub type AnnounceBlockFn = Arc<dyn Fn(Hash, Option<Vec<u8>>) + Send + Sync>;

type HostFunctions = (
	pezsp_io::BizinikiwiHostFunctions,
	pezcumulus_client_service::storage_proof_size::HostFunctions,
);
/// The client type being used by the test service.
pub type Client = TFullClient<runtime::NodeBlock, runtime::RuntimeApi, WasmExecutor<HostFunctions>>;

/// The backend type being used by the test service.
pub type Backend = TFullBackend<Block>;

/// The block-import type being used by the test service.
pub type TeyrchainBlockImport =
	TTeyrchainBlockImport<Block, SlotBasedBlockImport<Block, Arc<Client>, Client>, Backend>;

/// Transaction pool type used by the test service
pub type TransactionPool = Arc<pezsc_transaction_pool::TransactionPoolHandle<Block, Client>>;

/// Recovery handle that fails regularly to simulate unavailable povs.
pub struct FailingRecoveryHandle {
	overseer_handle: OverseerHandle,
	counter: u32,
	failed_hashes: HashSet<CandidateHash>,
}

impl FailingRecoveryHandle {
	/// Create a new FailingRecoveryHandle
	pub fn new(overseer_handle: OverseerHandle) -> Self {
		Self { overseer_handle, counter: 0, failed_hashes: Default::default() }
	}
}

#[async_trait::async_trait]
impl RecoveryHandle for FailingRecoveryHandle {
	async fn send_recovery_msg(
		&mut self,
		message: AvailabilityRecoveryMessage,
		origin: &'static str,
	) {
		let AvailabilityRecoveryMessage::RecoverAvailableData(ref receipt, _, _, _, _) = message;
		let candidate_hash = receipt.hash();

		// For every 3rd block we immediately signal unavailability to trigger
		// a retry. The same candidate is never failed multiple times to ensure progress.
		if self.counter.is_multiple_of(3) && self.failed_hashes.insert(candidate_hash) {
			tracing::info!(target: LOG_TARGET, ?candidate_hash, "Failing pov recovery.");

			let AvailabilityRecoveryMessage::RecoverAvailableData(_, _, _, _, back_sender) =
				message;
			back_sender
				.send(Err(RecoveryError::Unavailable))
				.expect("Return channel should work here.");
		} else {
			self.overseer_handle.send_msg(message, origin).await;
		}
		self.counter += 1;
	}
}

/// Assembly of PartialComponents (enough to run chain ops subcommands)
pub type Service = PartialComponents<
	Client,
	Backend,
	(),
	pezsc_consensus::import_queue::BasicQueue<Block>,
	pezsc_transaction_pool::TransactionPoolHandle<Block, Client>,
	(TeyrchainBlockImport, SlotBasedBlockImportHandle<Block>),
>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial(
	config: &mut Configuration,
	enable_import_proof_record: bool,
) -> Result<Service, pezsc_service::Error> {
	let heap_pages = config
		.executor
		.default_heap_pages
		.map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static { extra_pages: h as _ });

	let executor = WasmExecutor::builder()
		.with_execution_method(config.executor.wasm_method)
		.with_onchain_heap_alloc_strategy(heap_pages)
		.with_offchain_heap_alloc_strategy(heap_pages)
		.with_max_runtime_instances(config.executor.max_runtime_instances)
		.with_runtime_cache_size(config.executor.runtime_cache_size)
		.build();

	let (client, backend, keystore_container, task_manager) =
		pezsc_service::new_full_parts_record_import::<Block, RuntimeApi, _>(
			config,
			None,
			executor,
			enable_import_proof_record,
		)?;
	let client = Arc::new(client);

	let (block_import, slot_based_handle) =
		SlotBasedBlockImport::new(client.clone(), client.clone());
	let block_import = TeyrchainBlockImport::new(block_import, backend.clone());

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

	let slot_duration = pezsc_consensus_aura::slot_duration(&*client)?;
	let import_queue =
		pezcumulus_client_consensus_aura::import_queue::<AuthorityPair, _, _, _, _, _>(
			ImportQueueParams {
				block_import: block_import.clone(),
				client: client.clone(),
				create_inherent_data_providers: move |_, ()| async move {
					let timestamp = pezsp_timestamp::InherentDataProvider::from_system_time();

					let slot =
					pezsp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						*timestamp,
						slot_duration,
					);

					Ok((slot, timestamp))
				},
				spawner: &task_manager.spawn_essential_handle(),
				registry: None,
				telemetry: None,
			},
		)?;

	let params = PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain: (),
		other: (block_import, slot_based_handle),
	};

	Ok(params)
}

async fn build_relay_chain_interface(
	relay_chain_config: Configuration,
	teyrchain_prometheus_registry: Option<&Registry>,
	collator_key: Option<CollatorPair>,
	collator_options: CollatorOptions,
	task_manager: &mut TaskManager,
) -> RelayChainResult<Arc<dyn RelayChainInterface + 'static>> {
	let relay_chain_node = match collator_options.relay_chain_mode {
		pezcumulus_client_cli::RelayChainMode::Embedded => pezkuwi_test_service::new_full(
			relay_chain_config,
			if let Some(ref key) = collator_key {
				pezkuwi_service::IsTeyrchainNode::Collator(key.clone())
			} else {
				pezkuwi_service::IsTeyrchainNode::Collator(CollatorPair::generate().0)
			},
			None,
			pezkuwi_service::CollatorOverseerGen,
			Some("Relaychain"),
		)
		.map_err(|e| RelayChainError::Application(Box::new(e) as Box<_>))?,
		pezcumulus_client_cli::RelayChainMode::ExternalRpc(rpc_target_urls) => {
			return build_minimal_relay_chain_node_with_rpc(
				relay_chain_config,
				teyrchain_prometheus_registry,
				task_manager,
				rpc_target_urls,
			)
			.await
			.map(|r| r.0)
		},
	};

	task_manager.add_child(relay_chain_node.task_manager);
	tracing::info!("Using inprocess node.");
	Ok(Arc::new(RelayChainInProcessInterface::new(
		relay_chain_node.client.clone(),
		relay_chain_node.backend.clone(),
		relay_chain_node.sync_service.clone(),
		relay_chain_node.overseer_handle.ok_or(RelayChainError::GenericError(
			"Overseer should be running in full node.".to_string(),
		))?,
	)))
}

/// Start a node with the given teyrchain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[pezsc_tracing::logging::prefix_logs_with("Teyrchain")]
pub async fn start_node_impl<RB, Net: NetworkBackend<Block, Hash>>(
	teyrchain_config: Configuration,
	collator_key: Option<CollatorPair>,
	relay_chain_config: Configuration,
	wrap_announce_block: Option<Box<dyn FnOnce(AnnounceBlockFn) -> AnnounceBlockFn>>,
	fail_pov_recovery: bool,
	rpc_ext_builder: RB,
	collator_options: CollatorOptions,
	proof_recording_during_import: bool,
	use_slot_based_collator: bool,
) -> pezsc_service::error::Result<(
	TaskManager,
	Arc<Client>,
	Arc<dyn NetworkService>,
	RpcHandlers,
	TransactionPool,
	Arc<Backend>,
)>
where
	RB: Fn(Arc<Client>) -> Result<jsonrpsee::RpcModule<()>, pezsc_service::Error> + Send + 'static,
{
	let mut teyrchain_config = prepare_node_config(teyrchain_config);

	let params = new_partial(&mut teyrchain_config, proof_recording_during_import)?;

	let transaction_pool = params.transaction_pool.clone();
	let mut task_manager = params.task_manager;

	let client = params.client.clone();
	let backend = params.backend.clone();

	let block_import = params.other.0;
	let slot_based_handle = params.other.1;
	let relay_chain_interface = build_relay_chain_interface(
		relay_chain_config,
		teyrchain_config.prometheus_registry(),
		collator_key.clone(),
		collator_options.clone(),
		&mut task_manager,
	)
	.await
	.map_err(|e| pezsc_service::Error::Application(Box::new(e) as Box<_>))?;

	let import_queue_service = params.import_queue.service();
	let prometheus_registry = teyrchain_config.prometheus_registry().cloned();
	let net_config = FullNetworkConfiguration::<Block, Hash, Net>::new(
		&teyrchain_config.network,
		prometheus_registry.clone(),
	);

	let best_hash = client.chain_info().best_hash;
	let para_id = client
		.runtime_api()
		.teyrchain_id(best_hash)
		.map_err(|e| pezsc_service::Error::Application(Box::new(e) as Box<_>))?;
	tracing::info!("Teyrchain id: {:?}", para_id);

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
			metrics: Net::register_notification_metrics(
				teyrchain_config.prometheus_config.as_ref().map(|config| &config.registry),
			),
			sybil_resistance_level: CollatorSybilResistance::Resistant,
		})
		.await?;

	let keystore = params.keystore_container.keystore();
	let rpc_builder = {
		let client = client.clone();
		Box::new(move |_| rpc_ext_builder(client.clone()))
	};

	let rpc_handlers = pezsc_service::spawn_tasks(pezsc_service::SpawnTasksParams {
		rpc_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: teyrchain_config,
		keystore: keystore.clone(),
		backend: backend.clone(),
		network: network.clone(),
		sync_service: sync_service.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: None,
		tracing_execute_block: Some(Arc::new(TeyrchainTracingExecuteBlock::new(client.clone()))),
	})?;

	let announce_block = {
		let sync_service = sync_service.clone();
		Arc::new(move |hash, data| sync_service.announce_block(hash, data))
	};

	let announce_block = wrap_announce_block
		.map(|w| (w)(announce_block.clone()))
		.unwrap_or_else(|| announce_block);

	let overseer_handle = relay_chain_interface
		.overseer_handle()
		.map_err(|e| pezsc_service::Error::Application(Box::new(e)))?;

	let recovery_handle: Box<dyn RecoveryHandle> = if fail_pov_recovery {
		Box::new(FailingRecoveryHandle::new(overseer_handle.clone()))
	} else {
		Box::new(overseer_handle.clone())
	};
	let relay_chain_slot_duration = Duration::from_secs(6);

	start_relay_chain_tasks(StartRelayChainTasksParams {
		client: client.clone(),
		announce_block: announce_block.clone(),
		para_id,
		relay_chain_interface: relay_chain_interface.clone(),
		task_manager: &mut task_manager,
		// Increase speed of recovery for testing purposes.
		da_recovery_profile: DARecoveryProfile::Other(RecoveryDelayRange {
			min: Duration::from_secs(1),
			max: Duration::from_secs(5),
		}),
		import_queue: import_queue_service,
		relay_chain_slot_duration,
		recovery_handle,
		sync_service: sync_service.clone(),
		prometheus_registry: None,
	})?;

	let collator_peer_id = network.local_peer_id();
	if let Some(collator_key) = collator_key {
		let proposer = pezsc_basic_authorship::ProposerFactory::with_proof_recording(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			None,
		);

		let collator_service = CollatorService::new(
			client.clone(),
			Arc::new(task_manager.spawn_handle()),
			announce_block,
			client.clone(),
		);

		let client_for_aura = client.clone();

		if use_slot_based_collator {
			tracing::info!(target: LOG_TARGET, "Starting block authoring with slot based authoring.");
			let params = SlotBasedParams {
				create_inherent_data_providers: move |_, ()| async move { Ok(()) },
				block_import,
				para_client: client.clone(),
				para_backend: backend.clone(),
				relay_client: relay_chain_interface,
				code_hash_provider: move |block_hash| {
					client_for_aura.code_at(block_hash).ok().map(|c| ValidationCode::from(c).hash())
				},
				keystore,
				collator_key,
				relay_chain_slot_duration,
				para_id,
				proposer,
				collator_service,
				authoring_duration: Duration::from_millis(2000),
				reinitialize: false,
				slot_offset: Duration::from_secs(1),
				block_import_handle: slot_based_handle,
				spawner: task_manager.spawn_essential_handle(),
				export_pov: None,
				max_pov_percentage: None,
				collator_peer_id,
			};

			slot_based::run::<Block, AuthorityPair, _, _, _, _, _, _, _, _, _>(params);
		} else {
			tracing::info!(target: LOG_TARGET, "Starting block authoring with lookahead collator.");
			let params = AuraParams {
				create_inherent_data_providers: move |_, ()| async move { Ok(()) },
				block_import,
				para_client: client.clone(),
				para_backend: backend.clone(),
				relay_client: relay_chain_interface,
				code_hash_provider: move |block_hash| {
					client_for_aura.code_at(block_hash).ok().map(|c| ValidationCode::from(c).hash())
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

			let fut = aura::run::<Block, AuthorityPair, _, _, _, _, _, _, _, _>(params);
			task_manager.spawn_essential_handle().spawn("aura", None, fut);
		}
	}

	Ok((task_manager, client, network, rpc_handlers, transaction_pool, backend))
}

/// A Pezcumulus test node instance used for testing.
pub struct TestNode {
	/// TaskManager's instance.
	pub task_manager: TaskManager,
	/// Client's instance.
	pub client: Arc<Client>,
	/// Node's network.
	pub network: Arc<dyn NetworkService>,
	/// The `MultiaddrWithPeerId` to this node. This is useful if you want to pass it as "boot
	/// node" to other nodes.
	pub addr: MultiaddrWithPeerId,
	/// RPCHandlers to make RPC queries.
	pub rpc_handlers: RpcHandlers,
	/// Node's transaction pool
	pub transaction_pool: TransactionPool,
	/// Node's backend
	pub backend: Arc<Backend>,
}

/// A builder to create a [`TestNode`].
pub struct TestNodeBuilder {
	para_id: ParaId,
	tokio_handle: tokio::runtime::Handle,
	key: Sr25519Keyring,
	collator_key: Option<CollatorPair>,
	teyrchain_nodes: Vec<MultiaddrWithPeerId>,
	teyrchain_nodes_exclusive: bool,
	relay_chain_nodes: Vec<MultiaddrWithPeerId>,
	wrap_announce_block: Option<Box<dyn FnOnce(AnnounceBlockFn) -> AnnounceBlockFn>>,
	storage_update_func_teyrchain: Option<Box<dyn Fn()>>,
	storage_update_func_relay_chain: Option<Box<dyn Fn()>>,
	relay_chain_mode: RelayChainMode,
	endowed_accounts: Vec<AccountId>,
	record_proof_during_import: bool,
}

impl TestNodeBuilder {
	/// Create a new instance of `Self`.
	///
	/// `para_id` - The teyrchain id this node is running for.
	/// `tokio_handle` - The tokio handler to use.
	/// `key` - The key that will be used to generate the name and that will be passed as
	/// `dev_seed`.
	pub fn new(para_id: ParaId, tokio_handle: tokio::runtime::Handle, key: Sr25519Keyring) -> Self {
		TestNodeBuilder {
			key,
			para_id,
			tokio_handle,
			collator_key: None,
			teyrchain_nodes: Vec::new(),
			teyrchain_nodes_exclusive: false,
			relay_chain_nodes: Vec::new(),
			wrap_announce_block: None,
			storage_update_func_teyrchain: None,
			storage_update_func_relay_chain: None,
			endowed_accounts: Default::default(),
			relay_chain_mode: RelayChainMode::Embedded,
			record_proof_during_import: true,
		}
	}

	/// Enable collator for this node.
	pub fn enable_collator(mut self) -> Self {
		let collator_key = CollatorPair::generate().0;
		self.collator_key = Some(collator_key);
		self
	}

	/// Instruct the node to exclusively connect to registered teyrchain nodes.
	///
	/// Teyrchain nodes can be registered using [`Self::connect_to_teyrchain_node`] and
	/// [`Self::connect_to_teyrchain_nodes`].
	pub fn exclusively_connect_to_registered_teyrchain_nodes(mut self) -> Self {
		self.teyrchain_nodes_exclusive = true;
		self
	}

	/// Make the node connect to the given teyrchain node.
	///
	/// By default the node will not be connected to any node or will be able to discover any other
	/// node.
	pub fn connect_to_teyrchain_node(mut self, node: &TestNode) -> Self {
		self.teyrchain_nodes.push(node.addr.clone());
		self
	}

	/// Make the node connect to the given teyrchain nodes.
	///
	/// By default the node will not be connected to any node or will be able to discover any other
	/// node.
	pub fn connect_to_teyrchain_nodes<'a>(
		mut self,
		nodes: impl IntoIterator<Item = &'a TestNode>,
	) -> Self {
		self.teyrchain_nodes.extend(nodes.into_iter().map(|n| n.addr.clone()));
		self
	}

	/// Make the node connect to the given relay chain node.
	///
	/// By default the node will not be connected to any node or will be able to discover any other
	/// node.
	pub fn connect_to_relay_chain_node(
		mut self,
		node: &pezkuwi_test_service::PezkuwiTestNode,
	) -> Self {
		self.relay_chain_nodes.push(node.addr.clone());
		self
	}

	/// Make the node connect to the given relay chain nodes.
	///
	/// By default the node will not be connected to any node or will be able to discover any other
	/// node.
	pub fn connect_to_relay_chain_nodes<'a>(
		mut self,
		nodes: impl IntoIterator<Item = &'a pezkuwi_test_service::PezkuwiTestNode>,
	) -> Self {
		self.relay_chain_nodes.extend(nodes.into_iter().map(|n| n.addr.clone()));
		self
	}

	/// Wrap the announce block function of this node.
	pub fn wrap_announce_block(
		mut self,
		wrap: impl FnOnce(AnnounceBlockFn) -> AnnounceBlockFn + 'static,
	) -> Self {
		self.wrap_announce_block = Some(Box::new(wrap));
		self
	}

	/// Allows accessing the teyrchain storage before the test node is built.
	pub fn update_storage_teyrchain(mut self, updater: impl Fn() + 'static) -> Self {
		self.storage_update_func_teyrchain = Some(Box::new(updater));
		self
	}

	/// Allows accessing the relay chain storage before the test node is built.
	pub fn update_storage_relay_chain(mut self, updater: impl Fn() + 'static) -> Self {
		self.storage_update_func_relay_chain = Some(Box::new(updater));
		self
	}

	/// Connect to full node via RPC.
	pub fn use_external_relay_chain_node_at_url(mut self, network_address: Url) -> Self {
		self.relay_chain_mode = RelayChainMode::ExternalRpc(vec![network_address]);
		self
	}

	/// Connect to full node via RPC.
	pub fn use_external_relay_chain_node_at_port(mut self, port: u16) -> Self {
		let mut localhost_url =
			Url::parse("ws://localhost").expect("Should be able to parse localhost Url");
		localhost_url.set_port(Some(port)).expect("Should be able to set port");
		self.relay_chain_mode = RelayChainMode::ExternalRpc(vec![localhost_url]);
		self
	}

	/// Accounts which will have an initial balance.
	pub fn endowed_accounts(mut self, accounts: Vec<AccountId>) -> TestNodeBuilder {
		self.endowed_accounts = accounts;
		self
	}

	/// Record proofs during import.
	pub fn import_proof_recording(mut self, should_record_proof: bool) -> TestNodeBuilder {
		self.record_proof_during_import = should_record_proof;
		self
	}

	/// Build the [`TestNode`].
	pub async fn build(self) -> TestNode {
		let teyrchain_config = node_config(
			self.storage_update_func_teyrchain.unwrap_or_else(|| Box::new(|| ())),
			self.tokio_handle.clone(),
			self.key,
			self.teyrchain_nodes,
			self.teyrchain_nodes_exclusive,
			self.para_id,
			self.collator_key.is_some(),
			self.endowed_accounts,
		)
		.expect("could not generate Configuration");

		let mut relay_chain_config = pezkuwi_test_service::node_config(
			self.storage_update_func_relay_chain.unwrap_or_else(|| Box::new(|| ())),
			self.tokio_handle,
			self.key,
			self.relay_chain_nodes,
			false,
		);

		let collator_options = CollatorOptions {
			relay_chain_mode: self.relay_chain_mode,
			embedded_dht_bootnode: true,
			dht_bootnode_discovery: true,
		};

		relay_chain_config.network.node_name =
			format!("{} (relay chain)", relay_chain_config.network.node_name);

		let (task_manager, client, network, rpc_handlers, transaction_pool, backend) =
			match relay_chain_config.network.network_backend {
				pezsc_network::config::NetworkBackendType::Libp2p => {
					start_node_impl::<_, pezsc_network::NetworkWorker<_, _>>(
						teyrchain_config,
						self.collator_key,
						relay_chain_config,
						self.wrap_announce_block,
						false,
						|_| Ok(jsonrpsee::RpcModule::new(())),
						collator_options,
						self.record_proof_during_import,
						false,
					)
					.await
					.expect("could not create Pezcumulus test service")
				},
				pezsc_network::config::NetworkBackendType::Litep2p => {
					start_node_impl::<_, pezsc_network::Litep2pNetworkBackend>(
						teyrchain_config,
						self.collator_key,
						relay_chain_config,
						self.wrap_announce_block,
						false,
						|_| Ok(jsonrpsee::RpcModule::new(())),
						collator_options,
						self.record_proof_during_import,
						false,
					)
					.await
					.expect("could not create Pezcumulus test service")
				},
			};
		let peer_id = network.local_peer_id();
		let multiaddr = pezkuwi_test_service::get_listen_address(network.clone()).await;
		let addr = MultiaddrWithPeerId { multiaddr, peer_id };

		TestNode { task_manager, client, network, addr, rpc_handlers, transaction_pool, backend }
	}
}

/// Create a Pezcumulus `Configuration`.
///
/// By default a TCP socket will be used, therefore you need to provide nodes if you want the
/// node to be connected to other nodes.
///
/// If `nodes_exclusive` is `true`, the node will only connect to the given `nodes` and not to any
/// other node.
///
/// The `storage_update_func` can be used to make adjustments to the runtime genesis.
pub fn node_config(
	storage_update_func: impl Fn(),
	tokio_handle: tokio::runtime::Handle,
	key: Sr25519Keyring,
	nodes: Vec<MultiaddrWithPeerId>,
	nodes_exclusive: bool,
	para_id: ParaId,
	is_collator: bool,
	endowed_accounts: Vec<AccountId>,
) -> Result<Configuration, ServiceError> {
	let base_path = BasePath::new_temp_dir()?;
	let root = base_path.path().join(format!("pezcumulus_test_service_{}", key));
	let role = if is_collator { Role::Authority } else { Role::Full };
	let key_seed = key.to_seed();
	let mut spec = Box::new(chain_spec::get_chain_spec_with_extra_endowed(
		Some(para_id),
		endowed_accounts,
		pezcumulus_test_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
	));

	let mut storage = spec.as_storage_builder().build_storage().expect("could not build storage");

	BasicExternalities::execute_with_storage(&mut storage, storage_update_func);
	spec.set_storage(storage);

	let mut network_config = NetworkConfiguration::new(
		format!("{} (teyrchain)", key_seed),
		"network/test/0.1",
		Default::default(),
		None,
	);

	if nodes_exclusive {
		network_config.default_peers_set.reserved_nodes = nodes;
		network_config.default_peers_set.non_reserved_mode =
			pezsc_network::config::NonReservedPeerMode::Deny;
	} else {
		network_config.boot_nodes = nodes;
	}

	network_config.allow_non_globals_in_dht = true;

	let addr: multiaddr::Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().expect("valid address; qed");
	network_config.listen_addresses.push(addr.clone());
	network_config.transport =
		TransportConfig::Normal { enable_mdns: false, allow_private_ip: true };

	Ok(Configuration {
		impl_name: "pezcumulus-test-node".to_string(),
		impl_version: "0.1".to_string(),
		role,
		tokio_handle,
		transaction_pool: Default::default(),
		network: network_config,
		keystore: KeystoreConfig::InMemory,
		database: DatabaseSource::RocksDb { path: root.join("db"), cache_size: 128 },
		trie_cache_maximum_size: Some(64 * 1024 * 1024),
		warm_up_trie_cache: None,
		state_pruning: Some(PruningMode::ArchiveAll),
		blocks_pruning: BlocksPruning::KeepAll,
		chain_spec: spec,
		executor: ExecutorConfiguration {
			wasm_method: WasmExecutionMethod::Compiled {
				instantiation_strategy:
					pezsc_executor_wasmtime::InstantiationStrategy::PoolingCopyOnWrite,
			},
			..ExecutorConfiguration::default()
		},
		rpc: RpcConfiguration {
			addr: None,
			max_connections: Default::default(),
			cors: None,
			methods: Default::default(),
			max_request_size: Default::default(),
			max_response_size: Default::default(),
			id_provider: None,
			max_subs_per_conn: Default::default(),
			port: 9945,
			message_buffer_capacity: Default::default(),
			batch_config: RpcBatchRequestConfig::Unlimited,
			rate_limit: None,
			rate_limit_whitelisted_ips: Default::default(),
			rate_limit_trust_proxy_headers: Default::default(),
			request_logger_limit: 1024,
		},
		prometheus_config: None,
		telemetry_endpoints: None,
		offchain_worker: OffchainWorkerConfig { enabled: true, indexing_enabled: false },
		force_authoring: false,
		disable_grandpa: false,
		dev_key_seed: Some(key_seed),
		tracing_targets: None,
		tracing_receiver: Default::default(),
		announce_block: true,
		data_path: root,
		base_path,
		wasm_runtime_overrides: None,
	})
}

impl TestNode {
	/// Wait for `count` blocks to be imported in the node and then exit. This function will not
	/// return if no blocks are ever created, thus you should restrict the maximum amount of time of
	/// the test execution.
	pub fn wait_for_blocks(&self, count: usize) -> impl Future<Output = ()> {
		self.client.wait_for_blocks(count)
	}

	/// Send an extrinsic to this node.
	pub async fn send_extrinsic(
		&self,
		function: impl Into<runtime::RuntimeCall>,
		caller: Sr25519Keyring,
	) -> Result<RpcTransactionOutput, RpcTransactionError> {
		let extrinsic = construct_extrinsic(&self.client, function, caller.pair(), Some(0));

		self.rpc_handlers.send_transaction(extrinsic.into()).await
	}

	/// Register a teyrchain at this relay chain.
	pub async fn schedule_upgrade(&self, validation: Vec<u8>) -> Result<(), RpcTransactionError> {
		let call = pezframe_system::Call::set_code { code: validation };

		self.send_extrinsic(
			runtime::SudoCall::sudo_unchecked_weight {
				call: Box::new(call.into()),
				weight: Weight::from_parts(1_000, 0),
			},
			Sr25519Keyring::Alice,
		)
		.await
		.map(drop)
	}
}

/// Fetch account nonce for key pair
pub fn fetch_nonce(client: &Client, account: pezsp_core::sr25519::Public) -> u32 {
	let best_hash = client.chain_info().best_hash;
	client
		.runtime_api()
		.account_nonce(best_hash, account.into())
		.expect("Fetching account nonce works; qed")
}

/// Construct an extrinsic that can be applied to the test runtime.
pub fn construct_extrinsic(
	client: &Client,
	function: impl Into<runtime::RuntimeCall>,
	caller: pezsp_core::sr25519::Pair,
	nonce: Option<u32>,
) -> runtime::UncheckedExtrinsic {
	let function = function.into();
	let current_block_hash = client.info().best_hash;
	let current_block = client.info().best_number.saturated_into();
	let genesis_block = client.hash(0).unwrap().unwrap();
	let nonce = nonce.unwrap_or_else(|| fetch_nonce(client, caller.public()));
	let period = runtime::BlockHashCount::get()
		.checked_next_power_of_two()
		.map(|c| c / 2)
		.unwrap_or(2) as u64;
	let tip = 0;
	let tx_ext: runtime::TxExtension = (
		pezframe_system::AuthorizeCall::<runtime::Runtime>::new(),
		pezframe_system::CheckNonZeroSender::<runtime::Runtime>::new(),
		pezframe_system::CheckSpecVersion::<runtime::Runtime>::new(),
		pezframe_system::CheckGenesis::<runtime::Runtime>::new(),
		pezframe_system::CheckEra::<runtime::Runtime>::from(generic::Era::mortal(
			period,
			current_block,
		)),
		pezframe_system::CheckNonce::<runtime::Runtime>::from(nonce),
		pezframe_system::CheckWeight::<runtime::Runtime>::new(),
		pezpallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from(tip),
	)
		.into();
	let raw_payload = runtime::SignedPayload::from_raw(
		function.clone(),
		tx_ext.clone(),
		((), (), runtime::VERSION.spec_version, genesis_block, current_block_hash, (), (), ()),
	);
	let signature = raw_payload.using_encoded(|e| caller.sign(e));
	runtime::UncheckedExtrinsic::new_signed(
		function,
		MultiAddress::Id(caller.public().into()),
		runtime::Signature::Sr25519(signature),
		tx_ext,
	)
}

/// Run a relay-chain validator node.
///
/// This is essentially a wrapper around
/// [`run_validator_node`](pezkuwi_test_service::run_validator_node).
pub fn run_relay_chain_validator_node(
	tokio_handle: tokio::runtime::Handle,
	key: Sr25519Keyring,
	storage_update_func: impl Fn(),
	boot_nodes: Vec<MultiaddrWithPeerId>,
	port: Option<u16>,
) -> pezkuwi_test_service::PezkuwiTestNode {
	let mut config = pezkuwi_test_service::node_config(
		storage_update_func,
		tokio_handle.clone(),
		key,
		boot_nodes,
		true,
	);

	if let Some(port) = port {
		config.rpc.addr = Some(vec![RpcEndpoint {
			batch_config: config.rpc.batch_config,
			cors: config.rpc.cors.clone(),
			listen_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)),
			max_connections: config.rpc.max_connections,
			max_payload_in_mb: config.rpc.max_request_size,
			max_payload_out_mb: config.rpc.max_response_size,
			max_subscriptions_per_connection: config.rpc.max_subs_per_conn,
			max_buffer_capacity_per_connection: config.rpc.message_buffer_capacity,
			rpc_methods: config.rpc.methods,
			rate_limit: config.rpc.rate_limit,
			rate_limit_trust_proxy_headers: config.rpc.rate_limit_trust_proxy_headers,
			rate_limit_whitelisted_ips: config.rpc.rate_limit_whitelisted_ips.clone(),
			retry_random_port: true,
			is_optional: false,
		}]);
	}

	let mut workers_path = std::env::current_exe().unwrap();
	workers_path.pop();
	workers_path.pop();

	tokio_handle.block_on(async move {
		pezkuwi_test_service::run_validator_node(config, Some(workers_path)).await
	})
}
