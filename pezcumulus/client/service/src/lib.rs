// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// Pezcumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezcumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezcumulus. If not, see <https://www.gnu.org/licenses/>.

//! Pezcumulus service
//!
//! Provides functions for starting a collator node or a normal full node.

use futures::{channel::mpsc, StreamExt};
use pezcumulus_client_cli::CollatorOptions;
use pezcumulus_client_network::{AssumeSybilResistance, RequireSecondedInBlockAnnounce};
use pezcumulus_client_pov_recovery::{PoVRecovery, RecoveryDelayRange, RecoveryHandle};
use pezcumulus_primitives_core::{CollectCollationInfo, ParaId};
pub use pezcumulus_primitives_proof_size_hostfunction::storage_proof_size;
use pezcumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use pezcumulus_relay_chain_interface::{RelayChainInterface, RelayChainResult};
use pezcumulus_relay_chain_minimal_node::build_minimal_relay_chain_node_with_rpc;
use pezkuwi_primitives::{CandidateEvent, CollatorPair, OccupiedCoreAssumption};
use pezsc_client_api::{
	Backend as BackendT, BlockBackend, BlockchainEvents, Finalizer, ProofProvider, UsageProvider,
};
use pezsc_consensus::{
	import_queue::{ImportQueue, ImportQueueService},
	BlockImport,
};
use pezsc_network::{
	config::SyncMode, request_responses::IncomingRequest, service::traits::NetworkService,
	NetworkBackend,
};
use pezsc_network_sync::SyncingService;
use pezsc_network_transactions::TransactionsHandlerController;
use pezsc_service::{Configuration, SpawnTaskHandle, TaskManager, WarpSyncConfig};
use pezsc_telemetry::{log, TelemetryWorkerHandle};
use pezsc_tracing::block::TracingExecuteBlock;
use pezsc_utils::mpsc::TracingUnboundedSender;
use pezsp_api::{ApiExt, Core, ProofRecorder, ProvideRuntimeApi};
use pezsp_blockchain::{HeaderBackend, HeaderMetadata};
use pezsp_core::Decode;
use pezsp_runtime::{
	traits::{Block as BlockT, BlockIdTo, Header},
	SaturatedConversion, Saturating,
};
use pezsp_trie::proof_size_extension::ProofSizeExt;
use prometheus::{Histogram, HistogramOpts, Registry};
use std::{
	sync::Arc,
	time::{Duration, Instant},
};

/// Host functions that should be used in teyrchain nodes.
///
/// Contains the standard bizinikiwi host functions, as well as a
/// host function to enable PoV-reclaim on teyrchain nodes.
pub type TeyrchainHostFunctions = (
	pezcumulus_primitives_proof_size_hostfunction::storage_proof_size::HostFunctions,
	pezsp_io::BizinikiwiHostFunctions,
);

// Given the sporadic nature of the explicit recovery operation and the
// possibility to retry infinite times this value is more than enough.
// In practice here we expect no more than one queued messages.
const RECOVERY_CHAN_SIZE: usize = 8;
const LOG_TARGET_SYNC: &str = "sync::pezcumulus";

/// A hint about how long the node should wait before attempting to recover missing block data
/// from the data availability layer.
pub enum DARecoveryProfile {
	/// Collators use an aggressive recovery profile by default.
	Collator,
	/// Full nodes use a passive recovery profile by default, as they are not direct
	/// victims of withholding attacks.
	FullNode,
	/// Provide an explicit recovery profile.
	Other(RecoveryDelayRange),
}

/// Parameters given to [`start_relay_chain_tasks`].
pub struct StartRelayChainTasksParams<'a, Block: BlockT, Client, RCInterface> {
	pub client: Arc<Client>,
	pub announce_block: Arc<dyn Fn(Block::Hash, Option<Vec<u8>>) + Send + Sync>,
	pub para_id: ParaId,
	pub relay_chain_interface: RCInterface,
	pub task_manager: &'a mut TaskManager,
	pub da_recovery_profile: DARecoveryProfile,
	pub import_queue: Box<dyn ImportQueueService<Block>>,
	pub relay_chain_slot_duration: Duration,
	pub recovery_handle: Box<dyn RecoveryHandle>,
	pub sync_service: Arc<SyncingService<Block>>,
	pub prometheus_registry: Option<&'a Registry>,
}

/// Start necessary consensus tasks related to the relay chain.
///
/// Teyrchain nodes need to track the state of the relay chain and use the
/// relay chain's data availability service to fetch blocks if they don't
/// arrive via the normal p2p layer (i.e. when authors withhold their blocks deliberately).
///
/// This function spawns work for those side tasks.
///
/// It also spawns a teyrchain informant task that will log the relay chain state and some metrics.
pub fn start_relay_chain_tasks<Block, Client, Backend, RCInterface>(
	StartRelayChainTasksParams {
		client,
		announce_block,
		para_id,
		task_manager,
		da_recovery_profile,
		relay_chain_interface,
		import_queue,
		relay_chain_slot_duration,
		recovery_handle,
		sync_service,
		prometheus_registry,
	}: StartRelayChainTasksParams<Block, Client, RCInterface>,
) -> pezsc_service::error::Result<()>
where
	Block: BlockT,
	Client: Finalizer<Block, Backend>
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ Send
		+ Sync
		+ BlockBackend<Block>
		+ BlockchainEvents<Block>
		+ 'static,
	for<'a> &'a Client: BlockImport<Block>,
	Backend: BackendT<Block> + 'static,
	RCInterface: RelayChainInterface + Clone + 'static,
{
	let (recovery_chan_tx, recovery_chan_rx) = mpsc::channel(RECOVERY_CHAN_SIZE);

	pezcumulus_client_consensus_common::spawn_teyrchain_consensus_tasks(
		para_id,
		client.clone(),
		relay_chain_interface.clone(),
		announce_block.clone(),
		Some(recovery_chan_tx),
		task_manager.spawn_essential_handle(),
	);

	let da_recovery_profile = match da_recovery_profile {
		DARecoveryProfile::Collator => {
			// We want that collators wait at maximum the relay chain slot duration before starting
			// to recover blocks. Additionally, we wait at least half the slot time to give the
			// relay chain the chance to increase availability.
			RecoveryDelayRange {
				min: relay_chain_slot_duration / 2,
				max: relay_chain_slot_duration,
			}
		},
		DARecoveryProfile::FullNode => {
			// Full nodes should at least wait 2.5 minutes (assuming 6 seconds slot duration) and
			// in maximum 5 minutes before starting to recover blocks. Collators should already
			// start the recovery way before full nodes try to recover a certain block and then
			// share the block with the network using "the normal way". Full nodes are just the
			// "last resort" for block recovery.
			RecoveryDelayRange {
				min: relay_chain_slot_duration * 25,
				max: relay_chain_slot_duration * 50,
			}
		},
		DARecoveryProfile::Other(profile) => profile,
	};

	let pov_recovery = PoVRecovery::new(
		recovery_handle,
		da_recovery_profile,
		client.clone(),
		import_queue,
		relay_chain_interface.clone(),
		para_id,
		recovery_chan_rx,
		sync_service.clone(),
	);

	task_manager.spawn_essential_handle().spawn(
		"pezcumulus-pov-recovery",
		None,
		pov_recovery.run(),
	);

	let teyrchain_informant = teyrchain_informant::<Block, _>(
		para_id,
		relay_chain_interface.clone(),
		client.clone(),
		prometheus_registry.map(TeyrchainInformantMetrics::new).transpose()?,
	);
	task_manager
		.spawn_handle()
		.spawn("teyrchain-informant", None, teyrchain_informant);

	Ok(())
}

/// Prepare the teyrchain's node configuration
///
/// This function will:
/// * Disable the default announcement of Bizinikiwi for the teyrchain in favor of the one of
///   Pezcumulus.
/// * Set peers needed to start warp sync to 1.
pub fn prepare_node_config(mut teyrchain_config: Configuration) -> Configuration {
	teyrchain_config.announce_block = false;
	// Teyrchains only need 1 peer to start warp sync, because the target block is fetched from the
	// relay chain.
	teyrchain_config.network.min_peers_to_start_warp_sync = Some(1);

	teyrchain_config
}

/// Build a relay chain interface.
/// Will return a minimal relay chain node with RPC
/// client or an inprocess node, based on the [`CollatorOptions`] passed in.
pub async fn build_relay_chain_interface(
	relay_chain_config: Configuration,
	teyrchain_config: &Configuration,
	telemetry_worker_handle: Option<TelemetryWorkerHandle>,
	task_manager: &mut TaskManager,
	collator_options: CollatorOptions,
	hwbench: Option<pezsc_sysinfo::HwBench>,
) -> RelayChainResult<(
	Arc<dyn RelayChainInterface + 'static>,
	Option<CollatorPair>,
	Arc<dyn NetworkService>,
	async_channel::Receiver<IncomingRequest>,
)> {
	match collator_options.relay_chain_mode {
		pezcumulus_client_cli::RelayChainMode::Embedded => build_inprocess_relay_chain(
			relay_chain_config,
			teyrchain_config,
			telemetry_worker_handle,
			task_manager,
			hwbench,
		),
		pezcumulus_client_cli::RelayChainMode::ExternalRpc(rpc_target_urls) => {
			build_minimal_relay_chain_node_with_rpc(
				relay_chain_config,
				teyrchain_config.prometheus_registry(),
				task_manager,
				rpc_target_urls,
			)
			.await
		},
	}
}

/// The expected level of collator sybil-resistance on the network. This is used to
/// configure the type of metadata passed alongside block announcements on the network.
pub enum CollatorSybilResistance {
	/// There is a collator-selection protocol which provides sybil-resistance,
	/// such as Aura. Sybil-resistant collator-selection protocols are able to
	/// operate more efficiently.
	Resistant,
	/// There is no collator-selection protocol providing sybil-resistance.
	/// In situations such as "free-for-all" collators, the network is unresistant
	/// and needs to attach more metadata to block announcements, relying on relay-chain
	/// validators to avoid handling unbounded numbers of blocks.
	Unresistant,
}

/// Parameters given to [`build_network`].
pub struct BuildNetworkParams<
	'a,
	Block: BlockT,
	Client: ProvideRuntimeApi<Block>
		+ BlockBackend<Block>
		+ HeaderMetadata<Block, Error = pezsp_blockchain::Error>
		+ HeaderBackend<Block>
		+ BlockIdTo<Block>
		+ 'static,
	Network: NetworkBackend<Block, <Block as BlockT>::Hash>,
	RCInterface,
	IQ,
> where
	Client::Api: pezsp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>,
{
	pub teyrchain_config: &'a Configuration,
	pub net_config:
		pezsc_network::config::FullNetworkConfiguration<Block, <Block as BlockT>::Hash, Network>,
	pub client: Arc<Client>,
	pub transaction_pool: Arc<pezsc_transaction_pool::TransactionPoolHandle<Block, Client>>,
	pub para_id: ParaId,
	pub relay_chain_interface: RCInterface,
	pub spawn_handle: SpawnTaskHandle,
	pub import_queue: IQ,
	pub sybil_resistance_level: CollatorSybilResistance,
	pub metrics: pezsc_network::NotificationMetrics,
}

/// Build the network service, the network status sinks and an RPC sender.
pub async fn build_network<'a, Block, Client, RCInterface, IQ, Network>(
	BuildNetworkParams {
		teyrchain_config,
		net_config,
		client,
		transaction_pool,
		para_id,
		spawn_handle,
		relay_chain_interface,
		import_queue,
		sybil_resistance_level,
		metrics,
	}: BuildNetworkParams<'a, Block, Client, Network, RCInterface, IQ>,
) -> pezsc_service::error::Result<(
	Arc<dyn NetworkService>,
	TracingUnboundedSender<pezsc_rpc::system::Request<Block>>,
	TransactionsHandlerController<Block::Hash>,
	Arc<SyncingService<Block>>,
)>
where
	Block: BlockT,
	Client: UsageProvider<Block>
		+ HeaderBackend<Block>
		+ pezsp_consensus::block_validation::Chain<Block>
		+ Send
		+ Sync
		+ BlockBackend<Block>
		+ BlockchainEvents<Block>
		+ ProvideRuntimeApi<Block>
		+ HeaderMetadata<Block, Error = pezsp_blockchain::Error>
		+ BlockIdTo<Block, Error = pezsp_blockchain::Error>
		+ ProofProvider<Block>
		+ 'static,
	Client::Api: CollectCollationInfo<Block>
		+ pezsp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>,
	for<'b> &'b Client: BlockImport<Block>,
	RCInterface: RelayChainInterface + Clone + 'static,
	IQ: ImportQueue<Block> + 'static,
	Network: NetworkBackend<Block, <Block as BlockT>::Hash>,
{
	let warp_sync_config = match teyrchain_config.network.sync_mode {
		SyncMode::Warp => {
			log::debug!(target: LOG_TARGET_SYNC, "waiting for announce block...");

			let target_block =
				wait_for_finalized_para_head::<Block, _>(para_id, relay_chain_interface.clone())
					.await
					.inspect_err(|e| {
						log::error!(
							target: LOG_TARGET_SYNC,
							"Unable to determine teyrchain target block {:?}",
							e
						);
					})?;
			Some(WarpSyncConfig::WithTarget(target_block))
		},
		_ => None,
	};

	let block_announce_validator = match sybil_resistance_level {
		CollatorSybilResistance::Resistant => {
			let block_announce_validator = AssumeSybilResistance::allow_seconded_messages();
			Box::new(block_announce_validator) as Box<_>
		},
		CollatorSybilResistance::Unresistant => {
			let block_announce_validator =
				RequireSecondedInBlockAnnounce::new(relay_chain_interface, para_id);
			Box::new(block_announce_validator) as Box<_>
		},
	};

	pezsc_service::build_network(pezsc_service::BuildNetworkParams {
		config: teyrchain_config,
		net_config,
		client,
		transaction_pool,
		spawn_handle,
		import_queue,
		block_announce_validator_builder: Some(Box::new(move |_| block_announce_validator)),
		warp_sync_config,
		block_relay: None,
		metrics,
	})
}

/// Waits for the relay chain to have finished syncing and then gets the teyrchain header that
/// corresponds to the last finalized relay chain block.
async fn wait_for_finalized_para_head<B, RCInterface>(
	para_id: ParaId,
	relay_chain_interface: RCInterface,
) -> pezsc_service::error::Result<<B as BlockT>::Header>
where
	B: BlockT + 'static,
	RCInterface: RelayChainInterface + Send + 'static,
{
	let mut imported_blocks = relay_chain_interface
		.import_notification_stream()
		.await
		.map_err(|error| {
			pezsc_service::Error::Other(format!(
				"Relay chain import notification stream error when waiting for teyrchain head: \
				{error}"
			))
		})?
		.fuse();
	while imported_blocks.next().await.is_some() {
		let is_syncing = relay_chain_interface
			.is_major_syncing()
			.await
			.map_err(|e| format!("Unable to determine sync status: {e}"))?;

		if !is_syncing {
			let relay_chain_best_hash = relay_chain_interface
				.finalized_block_hash()
				.await
				.map_err(|e| Box::new(e) as Box<_>)?;

			let validation_data = relay_chain_interface
				.persisted_validation_data(
					relay_chain_best_hash,
					para_id,
					OccupiedCoreAssumption::TimedOut,
				)
				.await
				.map_err(|e| format!("{e:?}"))?
				.ok_or("Could not find teyrchain head in relay chain")?;

			let finalized_header = B::Header::decode(&mut &validation_data.parent_head.0[..])
				.map_err(|e| format!("Failed to decode teyrchain head: {e}"))?;

			log::info!(
				"🎉 Received target teyrchain header #{} ({}) from the relay chain.",
				finalized_header.number(),
				finalized_header.hash()
			);
			return Ok(finalized_header);
		}
	}

	Err("Stopping following imported blocks. Could not determine teyrchain target block".into())
}

/// Task for logging candidate events and some related metrics.
async fn teyrchain_informant<Block: BlockT, Client>(
	para_id: ParaId,
	relay_chain_interface: impl RelayChainInterface + Clone,
	client: Arc<Client>,
	metrics: Option<TeyrchainInformantMetrics>,
) where
	Client: HeaderBackend<Block> + Send + Sync + 'static,
{
	let mut import_notifications = match relay_chain_interface.import_notification_stream().await {
		Ok(import_notifications) => import_notifications,
		Err(e) => {
			log::error!("Failed to get import notification stream: {e:?}. Teyrchain informant will not run!");
			return;
		},
	};
	let mut last_backed_block_time: Option<Instant> = None;
	while let Some(n) = import_notifications.next().await {
		let candidate_events = match relay_chain_interface.candidate_events(n.hash()).await {
			Ok(candidate_events) => candidate_events,
			Err(e) => {
				log::warn!("Failed to get candidate events for block {}: {e:?}", n.hash());
				continue;
			},
		};
		let mut backed_candidates = Vec::new();
		let mut included_candidates = Vec::new();
		let mut timed_out_candidates = Vec::new();
		for event in candidate_events {
			match event {
				CandidateEvent::CandidateBacked(receipt, head, _, _) => {
					if receipt.descriptor.para_id() != para_id {
						continue;
					}
					let backed_block = match Block::Header::decode(&mut &head.0[..]) {
						Ok(header) => header,
						Err(e) => {
							log::warn!(
								"Failed to decode teyrchain header from backed block: {e:?}"
							);
							continue;
						},
					};
					let backed_block_time = Instant::now();
					if let Some(last_backed_block_time) = &last_backed_block_time {
						let duration = backed_block_time.duration_since(*last_backed_block_time);
						if let Some(metrics) = &metrics {
							metrics.teyrchain_block_backed_duration.observe(duration.as_secs_f64());
						}
					}
					last_backed_block_time = Some(backed_block_time);
					backed_candidates.push(backed_block);
				},
				CandidateEvent::CandidateIncluded(receipt, head, _, _) => {
					if receipt.descriptor.para_id() != para_id {
						continue;
					}
					let included_block = match Block::Header::decode(&mut &head.0[..]) {
						Ok(header) => header,
						Err(e) => {
							log::warn!(
								"Failed to decode teyrchain header from included block: {e:?}"
							);
							continue;
						},
					};
					let unincluded_segment_size =
						client.info().best_number.saturating_sub(*included_block.number());
					let unincluded_segment_size: u32 = unincluded_segment_size.saturated_into();
					if let Some(metrics) = &metrics {
						metrics.unincluded_segment_size.observe(unincluded_segment_size.into());
					}
					included_candidates.push(included_block);
				},
				CandidateEvent::CandidateTimedOut(receipt, head, _) => {
					if receipt.descriptor.para_id() != para_id {
						continue;
					}
					let timed_out_block = match Block::Header::decode(&mut &head.0[..]) {
						Ok(header) => header,
						Err(e) => {
							log::warn!(
								"Failed to decode teyrchain header from timed out block: {e:?}"
							);
							continue;
						},
					};
					timed_out_candidates.push(timed_out_block);
				},
			}
		}
		let mut log_parts = Vec::new();
		if !backed_candidates.is_empty() {
			let backed_candidates = backed_candidates
				.into_iter()
				.map(|c| format!("#{} ({})", c.number(), c.hash()))
				.collect::<Vec<_>>()
				.join(", ");
			log_parts.push(format!("backed: {}", backed_candidates));
		};
		if !included_candidates.is_empty() {
			let included_candidates = included_candidates
				.into_iter()
				.map(|c| format!("#{} ({})", c.number(), c.hash()))
				.collect::<Vec<_>>()
				.join(", ");
			log_parts.push(format!("included: {}", included_candidates));
		};
		if !timed_out_candidates.is_empty() {
			let timed_out_candidates = timed_out_candidates
				.into_iter()
				.map(|c| format!("#{} ({})", c.number(), c.hash()))
				.collect::<Vec<_>>()
				.join(", ");
			log_parts.push(format!("timed out: {}", timed_out_candidates));
		};
		if !log_parts.is_empty() {
			log::info!(
				"Update at relay chain block #{} ({}) - {}",
				n.number(),
				n.hash(),
				log_parts.join(", ")
			);
		}
	}
}

struct TeyrchainInformantMetrics {
	/// Time between teyrchain blocks getting backed by the relaychain.
	teyrchain_block_backed_duration: Histogram,
	/// Number of blocks between best block and last included block.
	unincluded_segment_size: Histogram,
}

impl TeyrchainInformantMetrics {
	fn new(prometheus_registry: &Registry) -> prometheus::Result<Self> {
		let teyrchain_block_authorship_duration = Histogram::with_opts(HistogramOpts::new(
			"teyrchain_block_backed_duration",
			"Time between teyrchain blocks getting backed by the relaychain",
		))?;
		prometheus_registry.register(Box::new(teyrchain_block_authorship_duration.clone()))?;

		let unincluded_segment_size = Histogram::with_opts(
			HistogramOpts::new(
				"teyrchain_unincluded_segment_size",
				"Number of blocks between best block and last included block",
			)
			.buckets((0..=24).into_iter().map(|i| i as f64).collect()),
		)?;
		prometheus_registry.register(Box::new(unincluded_segment_size.clone()))?;

		Ok(Self {
			teyrchain_block_backed_duration: teyrchain_block_authorship_duration,
			unincluded_segment_size,
		})
	}
}

/// Implementation of [`TracingExecuteBlock`] for teyrchains.
///
/// Ensures that all the required extensions required by teyrchain runtimes are registered and
/// available.
pub struct TeyrchainTracingExecuteBlock<Client> {
	client: Arc<Client>,
}

impl<Client> TeyrchainTracingExecuteBlock<Client> {
	/// Creates a new instance of `self`.
	pub fn new(client: Arc<Client>) -> Self {
		Self { client }
	}
}

impl<Block, Client> TracingExecuteBlock<Block> for TeyrchainTracingExecuteBlock<Client>
where
	Block: BlockT,
	Client: ProvideRuntimeApi<Block> + Send + Sync,
	Client::Api: Core<Block>,
{
	fn execute_block(&self, _: Block::Hash, block: Block) -> pezsp_blockchain::Result<()> {
		let mut runtime_api = self.client.runtime_api();
		let storage_proof_recorder = ProofRecorder::<Block>::default();
		runtime_api.register_extension(ProofSizeExt::new(storage_proof_recorder.clone()));
		runtime_api.record_proof_with_recorder(storage_proof_recorder);

		runtime_api
			.execute_block(*block.header().parent_hash(), block.into())
			.map_err(Into::into)
	}
}
