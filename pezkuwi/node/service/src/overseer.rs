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

use super::{Error, IsTeyrchainNode, Registry};
use pezkuwi_node_subsystem_types::{ChainApiBackend, RuntimeApiSubsystemClient};
use pezkuwi_overseer::{DummySubsystem, InitializedOverseerBuilder, SubsystemError};
use pezsp_core::traits::SpawnNamed;

use pezkuwi_availability_distribution::IncomingRequestReceivers;
use pezkuwi_node_core_approval_voting::Config as ApprovalVotingConfig;
use pezkuwi_node_core_av_store::Config as AvailabilityConfig;
use pezkuwi_node_core_candidate_validation::Config as CandidateValidationConfig;
use pezkuwi_node_core_chain_selection::Config as ChainSelectionConfig;
use pezkuwi_node_core_dispute_coordinator::Config as DisputeCoordinatorConfig;
use pezkuwi_node_network_protocol::{
	peer_set::{PeerSet, PeerSetProtocolNames},
	request_response::{
		v1 as request_v1, v2 as request_v2, IncomingRequestReceiver, ReqProtocolNames,
	},
};
#[cfg(any(feature = "malus", test))]
pub use pezkuwi_overseer::{dummy::dummy_overseer_builder, HeadSupportsTeyrchains};
use pezkuwi_overseer::{
	metrics::Metrics as OverseerMetrics, MetricsTrait, Overseer, OverseerConnector, OverseerHandle,
	SpawnGlue,
};

use parking_lot::Mutex;
use pezsc_authority_discovery::Service as AuthorityDiscoveryService;
use pezsc_client_api::AuxStore;
use pezsc_keystore::LocalKeystore;
use pezsc_network::{NetworkStateInfo, NotificationService};
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
	time::Duration,
};

pub use pezkuwi_approval_distribution::ApprovalDistribution as ApprovalDistributionSubsystem;
pub use pezkuwi_availability_bitfield_distribution::BitfieldDistribution as BitfieldDistributionSubsystem;
pub use pezkuwi_availability_distribution::AvailabilityDistributionSubsystem;
pub use pezkuwi_availability_recovery::AvailabilityRecoverySubsystem;
pub use pezkuwi_collator_protocol::{CollatorProtocolSubsystem, ProtocolSide};
pub use pezkuwi_dispute_distribution::DisputeDistributionSubsystem;
pub use pezkuwi_gossip_support::GossipSupport as GossipSupportSubsystem;
pub use pezkuwi_network_bridge::{
	Metrics as NetworkBridgeMetrics, NetworkBridgeRx as NetworkBridgeRxSubsystem,
	NetworkBridgeTx as NetworkBridgeTxSubsystem,
};
pub use pezkuwi_node_collation_generation::CollationGenerationSubsystem;
pub use pezkuwi_node_core_approval_voting::ApprovalVotingSubsystem;
pub use pezkuwi_node_core_approval_voting_parallel::{
	ApprovalVotingParallelSubsystem, Metrics as ApprovalVotingParallelMetrics,
};
pub use pezkuwi_node_core_av_store::AvailabilityStoreSubsystem;
pub use pezkuwi_node_core_backing::CandidateBackingSubsystem;
pub use pezkuwi_node_core_bitfield_signing::BitfieldSigningSubsystem;
pub use pezkuwi_node_core_candidate_validation::CandidateValidationSubsystem;
pub use pezkuwi_node_core_chain_api::ChainApiSubsystem;
pub use pezkuwi_node_core_chain_selection::ChainSelectionSubsystem;
pub use pezkuwi_node_core_dispute_coordinator::DisputeCoordinatorSubsystem;
pub use pezkuwi_node_core_prospective_teyrchains::ProspectiveTeyrchainsSubsystem;
pub use pezkuwi_node_core_provisioner::ProvisionerSubsystem;
pub use pezkuwi_node_core_pvf_checker::PvfCheckerSubsystem;
pub use pezkuwi_node_core_runtime_api::RuntimeApiSubsystem;
pub use pezkuwi_statement_distribution::StatementDistributionSubsystem;

/// Arguments passed for overseer construction.
pub struct OverseerGenArgs<'a, Spawner, RuntimeClient>
where
	Spawner: 'static + SpawnNamed + Clone + Unpin,
{
	/// Runtime client generic, providing the `ProvideRuntimeApi` trait besides others.
	pub runtime_client: Arc<RuntimeClient>,
	/// Underlying network service implementation.
	pub network_service: Arc<dyn pezsc_network::service::traits::NetworkService>,
	/// Underlying syncing service implementation.
	pub sync_service: Arc<dyn pezsp_consensus::SyncOracle + Send + Sync>,
	/// Underlying authority discovery service.
	pub authority_discovery_service: AuthorityDiscoveryService,
	/// Collations request receiver for network protocol v1.
	pub collation_req_v1_receiver: IncomingRequestReceiver<request_v1::CollationFetchingRequest>,
	/// Collations request receiver for network protocol v2.
	pub collation_req_v2_receiver: IncomingRequestReceiver<request_v2::CollationFetchingRequest>,
	/// Receiver for available data requests.
	pub available_data_req_receiver:
		IncomingRequestReceiver<request_v1::AvailableDataFetchingRequest>,
	/// Prometheus registry, commonly used for production systems, less so for test.
	pub registry: Option<&'a Registry>,
	/// Task spawner to be used throughout the overseer and the APIs it provides.
	pub spawner: Spawner,
	/// Determines the behavior of the collator.
	pub is_teyrchain_node: IsTeyrchainNode,
	/// Overseer channel capacity override.
	pub overseer_message_channel_capacity_override: Option<usize>,
	/// Request-response protocol names source.
	pub req_protocol_names: ReqProtocolNames,
	/// `PeerSet` protocol names to protocols mapping.
	pub peerset_protocol_names: PeerSetProtocolNames,
	/// Notification services for validation/collation protocols.
	pub notification_services: HashMap<PeerSet, Box<dyn NotificationService>>,
}

pub struct ExtendedOverseerGenArgs {
	/// The keystore to use for i.e. validator keys.
	pub keystore: Arc<LocalKeystore>,
	/// The underlying key value store for the teyrchains.
	pub teyrchains_db: Arc<dyn pezkuwi_node_subsystem_util::database::Database>,
	/// Configuration for the candidate validation subsystem.
	pub candidate_validation_config: Option<CandidateValidationConfig>,
	/// Configuration for the availability store subsystem.
	pub availability_config: AvailabilityConfig,
	/// POV request receiver.
	pub pov_req_receiver: IncomingRequestReceiver<request_v1::PoVFetchingRequest>,
	/// Erasure chunk request v1 receiver.
	pub chunk_req_v1_receiver: IncomingRequestReceiver<request_v1::ChunkFetchingRequest>,
	/// Erasure chunk request v2 receiver.
	pub chunk_req_v2_receiver: IncomingRequestReceiver<request_v2::ChunkFetchingRequest>,
	/// Receiver for incoming candidate requests.
	pub candidate_req_v2_receiver: IncomingRequestReceiver<request_v2::AttestedCandidateRequest>,
	/// Configuration for the approval voting subsystem.
	pub approval_voting_config: ApprovalVotingConfig,
	/// Receiver for incoming disputes.
	pub dispute_req_receiver: IncomingRequestReceiver<request_v1::DisputeRequest>,
	/// Configuration for the dispute coordinator subsystem.
	pub dispute_coordinator_config: DisputeCoordinatorConfig,
	/// Configuration for the chain selection subsystem.
	pub chain_selection_config: ChainSelectionConfig,
	/// Optional availability recovery fetch chunks threshold. If PoV size size is lower
	/// than the value put in here we always try to recovery availability from backers.
	/// The presence of this parameter here is needed to have different values per chain.
	pub fetch_chunks_threshold: Option<usize>,
	/// Set of invulnerable AH collator `PeerId`s
	pub invulnerable_ah_collators: HashSet<pezkuwi_node_network_protocol::PeerId>,
	/// Override for `HOLD_OFF_DURATION` constant .
	pub collator_protocol_hold_off: Option<Duration>,
}

/// Obtain a prepared validator `Overseer`, that is initialized with all default values.
pub fn validator_overseer_builder<Spawner, RuntimeClient>(
	OverseerGenArgs {
		runtime_client,
		network_service,
		sync_service,
		authority_discovery_service,
		collation_req_v1_receiver: _,
		collation_req_v2_receiver: _,
		available_data_req_receiver,
		registry,
		spawner,
		is_teyrchain_node,
		overseer_message_channel_capacity_override,
		req_protocol_names,
		peerset_protocol_names,
		notification_services,
	}: OverseerGenArgs<Spawner, RuntimeClient>,
	ExtendedOverseerGenArgs {
		keystore,
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
	}: ExtendedOverseerGenArgs,
) -> Result<
	InitializedOverseerBuilder<
		SpawnGlue<Spawner>,
		Arc<RuntimeClient>,
		CandidateValidationSubsystem,
		PvfCheckerSubsystem,
		CandidateBackingSubsystem,
		StatementDistributionSubsystem,
		AvailabilityDistributionSubsystem,
		AvailabilityRecoverySubsystem,
		BitfieldSigningSubsystem,
		BitfieldDistributionSubsystem,
		ProvisionerSubsystem,
		RuntimeApiSubsystem<RuntimeClient>,
		AvailabilityStoreSubsystem,
		NetworkBridgeRxSubsystem<
			Arc<dyn pezsc_network::service::traits::NetworkService>,
			AuthorityDiscoveryService,
		>,
		NetworkBridgeTxSubsystem<
			Arc<dyn pezsc_network::service::traits::NetworkService>,
			AuthorityDiscoveryService,
		>,
		ChainApiSubsystem<RuntimeClient>,
		DummySubsystem,
		CollatorProtocolSubsystem,
		DummySubsystem,
		DummySubsystem,
		ApprovalVotingParallelSubsystem,
		GossipSupportSubsystem<AuthorityDiscoveryService>,
		DisputeCoordinatorSubsystem,
		DisputeDistributionSubsystem<AuthorityDiscoveryService>,
		ChainSelectionSubsystem,
		ProspectiveTeyrchainsSubsystem,
	>,
	Error,
>
where
	RuntimeClient: RuntimeApiSubsystemClient + ChainApiBackend + AuxStore + 'static,
	Spawner: 'static + SpawnNamed + Clone + Unpin,
{
	use pezkuwi_node_subsystem_util::metrics::Metrics;

	let metrics = <OverseerMetrics as MetricsTrait>::register(registry)?;
	let notification_sinks = Arc::new(Mutex::new(HashMap::new()));

	let spawner = SpawnGlue(spawner);

	let network_bridge_metrics: NetworkBridgeMetrics = Metrics::register(registry)?;
	let approval_voting_parallel_metrics: ApprovalVotingParallelMetrics =
		Metrics::register(registry)?;
	let builder = Overseer::builder()
		.network_bridge_tx(NetworkBridgeTxSubsystem::new(
			network_service.clone(),
			authority_discovery_service.clone(),
			network_bridge_metrics.clone(),
			req_protocol_names.clone(),
			peerset_protocol_names.clone(),
			notification_sinks.clone(),
		))
		.network_bridge_rx(NetworkBridgeRxSubsystem::new(
			network_service.clone(),
			authority_discovery_service.clone(),
			Box::new(sync_service.clone()),
			network_bridge_metrics,
			peerset_protocol_names,
			notification_services,
			notification_sinks,
		))
		.availability_distribution(AvailabilityDistributionSubsystem::new(
			keystore.clone(),
			IncomingRequestReceivers {
				pov_req_receiver,
				chunk_req_v1_receiver,
				chunk_req_v2_receiver,
			},
			req_protocol_names.clone(),
			Metrics::register(registry)?,
		))
		.availability_recovery(AvailabilityRecoverySubsystem::for_validator(
			fetch_chunks_threshold,
			available_data_req_receiver,
			&req_protocol_names,
			Metrics::register(registry)?,
		))
		.availability_store(AvailabilityStoreSubsystem::new(
			teyrchains_db.clone(),
			availability_config,
			Box::new(sync_service.clone()),
			Metrics::register(registry)?,
		))
		.bitfield_distribution(BitfieldDistributionSubsystem::new(Metrics::register(registry)?))
		.bitfield_signing(BitfieldSigningSubsystem::new(
			keystore.clone(),
			Metrics::register(registry)?,
		))
		.candidate_backing(CandidateBackingSubsystem::new(
			keystore.clone(),
			Metrics::register(registry)?,
		))
		.candidate_validation(CandidateValidationSubsystem::with_config(
			candidate_validation_config,
			keystore.clone(),
			Metrics::register(registry)?, // candidate-validation metrics
			Metrics::register(registry)?, // validation host metrics
		))
		.pvf_checker(PvfCheckerSubsystem::new(keystore.clone(), Metrics::register(registry)?))
		.chain_api(ChainApiSubsystem::new(runtime_client.clone(), Metrics::register(registry)?))
		.collation_generation(DummySubsystem)
		.collator_protocol({
			let side = match is_teyrchain_node {
				IsTeyrchainNode::Collator(_) | IsTeyrchainNode::FullNode => {
					return Err(Error::Overseer(SubsystemError::Context(
						"build validator overseer for teyrchain node".to_owned(),
					)))
				},
				IsTeyrchainNode::No => ProtocolSide::Validator {
					keystore: keystore.clone(),
					eviction_policy: Default::default(),
					metrics: Metrics::register(registry)?,
					invulnerables: invulnerable_ah_collators,
					collator_protocol_hold_off,
				},
			};
			CollatorProtocolSubsystem::new(side)
		})
		.provisioner(ProvisionerSubsystem::new(Metrics::register(registry)?))
		.runtime_api(RuntimeApiSubsystem::new(
			runtime_client.clone(),
			Metrics::register(registry)?,
			spawner.clone(),
		))
		.statement_distribution(StatementDistributionSubsystem::new(
			keystore.clone(),
			candidate_req_v2_receiver,
			Metrics::register(registry)?,
		))
		.approval_distribution(DummySubsystem)
		.approval_voting(DummySubsystem)
		.approval_voting_parallel(ApprovalVotingParallelSubsystem::with_config(
			approval_voting_config,
			teyrchains_db.clone(),
			keystore.clone(),
			Box::new(sync_service.clone()),
			approval_voting_parallel_metrics,
			spawner.clone(),
			overseer_message_channel_capacity_override,
		))
		.gossip_support(GossipSupportSubsystem::new(
			keystore.clone(),
			authority_discovery_service.clone(),
			Metrics::register(registry)?,
		))
		.dispute_coordinator(DisputeCoordinatorSubsystem::new(
			teyrchains_db.clone(),
			dispute_coordinator_config,
			keystore.clone(),
			Metrics::register(registry)?,
		))
		.dispute_distribution(DisputeDistributionSubsystem::new(
			keystore.clone(),
			dispute_req_receiver,
			authority_discovery_service.clone(),
			Metrics::register(registry)?,
		))
		.chain_selection(ChainSelectionSubsystem::new(chain_selection_config, teyrchains_db))
		.prospective_teyrchains(ProspectiveTeyrchainsSubsystem::new(Metrics::register(registry)?))
		.activation_external_listeners(Default::default())
		.active_leaves(Default::default())
		.supports_teyrchains(runtime_client)
		.metrics(metrics)
		.spawner(spawner);

	let builder = if let Some(capacity) = overseer_message_channel_capacity_override {
		builder.message_channel_capacity(capacity)
	} else {
		builder
	};
	Ok(builder)
}

/// Obtain a prepared collator `Overseer`, that is initialized with all default values.
pub fn collator_overseer_builder<Spawner, RuntimeClient>(
	OverseerGenArgs {
		runtime_client,
		network_service,
		sync_service,
		authority_discovery_service,
		collation_req_v1_receiver: _,
		collation_req_v2_receiver,
		available_data_req_receiver,
		registry,
		spawner,
		is_teyrchain_node,
		overseer_message_channel_capacity_override,
		req_protocol_names,
		peerset_protocol_names,
		notification_services,
	}: OverseerGenArgs<Spawner, RuntimeClient>,
) -> Result<
	InitializedOverseerBuilder<
		SpawnGlue<Spawner>,
		Arc<RuntimeClient>,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
		AvailabilityRecoverySubsystem,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
		RuntimeApiSubsystem<RuntimeClient>,
		DummySubsystem,
		NetworkBridgeRxSubsystem<
			Arc<dyn pezsc_network::service::traits::NetworkService>,
			AuthorityDiscoveryService,
		>,
		NetworkBridgeTxSubsystem<
			Arc<dyn pezsc_network::service::traits::NetworkService>,
			AuthorityDiscoveryService,
		>,
		ChainApiSubsystem<RuntimeClient>,
		CollationGenerationSubsystem,
		CollatorProtocolSubsystem,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
		DummySubsystem,
	>,
	Error,
>
where
	Spawner: 'static + SpawnNamed + Clone + Unpin,
	RuntimeClient: RuntimeApiSubsystemClient + ChainApiBackend + AuxStore + 'static,
{
	use pezkuwi_node_subsystem_util::metrics::Metrics;

	let notification_sinks = Arc::new(Mutex::new(HashMap::new()));

	let spawner = SpawnGlue(spawner);

	let network_bridge_metrics: NetworkBridgeMetrics = Metrics::register(registry)?;

	let builder = Overseer::builder()
		.network_bridge_tx(NetworkBridgeTxSubsystem::new(
			network_service.clone(),
			authority_discovery_service.clone(),
			network_bridge_metrics.clone(),
			req_protocol_names.clone(),
			peerset_protocol_names.clone(),
			notification_sinks.clone(),
		))
		.network_bridge_rx(NetworkBridgeRxSubsystem::new(
			network_service.clone(),
			authority_discovery_service.clone(),
			Box::new(sync_service.clone()),
			network_bridge_metrics,
			peerset_protocol_names,
			notification_services,
			notification_sinks,
		))
		.availability_distribution(DummySubsystem)
		.availability_recovery(AvailabilityRecoverySubsystem::for_collator(
			None,
			available_data_req_receiver,
			&req_protocol_names,
			Metrics::register(registry)?,
		))
		.availability_store(DummySubsystem)
		.bitfield_distribution(DummySubsystem)
		.bitfield_signing(DummySubsystem)
		.candidate_backing(DummySubsystem)
		.candidate_validation(DummySubsystem)
		.pvf_checker(DummySubsystem)
		.chain_api(ChainApiSubsystem::new(runtime_client.clone(), Metrics::register(registry)?))
		.collation_generation(CollationGenerationSubsystem::new(Metrics::register(registry)?))
		.collator_protocol({
			let side = match is_teyrchain_node {
				IsTeyrchainNode::No => {
					return Err(Error::Overseer(SubsystemError::Context(
						"build teyrchain node overseer for validator".to_owned(),
					)))
				},
				IsTeyrchainNode::Collator(collator_pair) => ProtocolSide::Collator {
					peer_id: network_service.local_peer_id(),
					collator_pair,
					request_receiver_v2: collation_req_v2_receiver,
					metrics: Metrics::register(registry)?,
				},
				IsTeyrchainNode::FullNode => ProtocolSide::None,
			};
			CollatorProtocolSubsystem::new(side)
		})
		.provisioner(DummySubsystem)
		.runtime_api(RuntimeApiSubsystem::new(
			runtime_client.clone(),
			Metrics::register(registry)?,
			spawner.clone(),
		))
		.statement_distribution(DummySubsystem)
		.approval_distribution(DummySubsystem)
		.approval_voting(DummySubsystem)
		.approval_voting_parallel(DummySubsystem)
		.gossip_support(DummySubsystem)
		.dispute_coordinator(DummySubsystem)
		.dispute_distribution(DummySubsystem)
		.chain_selection(DummySubsystem)
		.prospective_teyrchains(DummySubsystem)
		.activation_external_listeners(Default::default())
		.active_leaves(Default::default())
		.supports_teyrchains(runtime_client)
		.metrics(Metrics::register(registry)?)
		.spawner(spawner);

	let builder = if let Some(capacity) = overseer_message_channel_capacity_override {
		builder.message_channel_capacity(capacity)
	} else {
		builder
	};
	Ok(builder)
}

/// Trait for the `fn` generating the overseer.
pub trait OverseerGen {
	/// Overwrite the full generation of the overseer, including the subsystems.
	fn generate<Spawner, RuntimeClient>(
		&self,
		connector: OverseerConnector,
		args: OverseerGenArgs<Spawner, RuntimeClient>,
		ext_args: Option<ExtendedOverseerGenArgs>,
	) -> Result<(Overseer<SpawnGlue<Spawner>, Arc<RuntimeClient>>, OverseerHandle), Error>
	where
		RuntimeClient: RuntimeApiSubsystemClient + ChainApiBackend + AuxStore + 'static,
		Spawner: 'static + SpawnNamed + Clone + Unpin;

	// It would be nice to make `create_subsystems` part of this trait,
	// but the amount of generic arguments that would be required as
	// as consequence make this rather annoying to implement and use.
}

/// The regular set of subsystems.
pub struct ValidatorOverseerGen;

impl OverseerGen for ValidatorOverseerGen {
	fn generate<Spawner, RuntimeClient>(
		&self,
		connector: OverseerConnector,
		args: OverseerGenArgs<Spawner, RuntimeClient>,
		ext_args: Option<ExtendedOverseerGenArgs>,
	) -> Result<(Overseer<SpawnGlue<Spawner>, Arc<RuntimeClient>>, OverseerHandle), Error>
	where
		RuntimeClient: RuntimeApiSubsystemClient + ChainApiBackend + AuxStore + 'static,
		Spawner: 'static + SpawnNamed + Clone + Unpin,
	{
		let ext_args = ext_args.ok_or(Error::Overseer(SubsystemError::Context(
			"create validator overseer as mandatory extended arguments were not provided"
				.to_owned(),
		)))?;
		validator_overseer_builder(args, ext_args)?
			.build_with_connector(connector)
			.map_err(|e| e.into())
	}
}

/// Reduced set of subsystems, to use in collator and collator's full node.
pub struct CollatorOverseerGen;

impl OverseerGen for CollatorOverseerGen {
	fn generate<Spawner, RuntimeClient>(
		&self,
		connector: OverseerConnector,
		args: OverseerGenArgs<Spawner, RuntimeClient>,
		_ext_args: Option<ExtendedOverseerGenArgs>,
	) -> Result<(Overseer<SpawnGlue<Spawner>, Arc<RuntimeClient>>, OverseerHandle), Error>
	where
		RuntimeClient: RuntimeApiSubsystemClient + ChainApiBackend + AuxStore + 'static,
		Spawner: 'static + SpawnNamed + Clone + Unpin,
	{
		collator_overseer_builder(args)?
			.build_with_connector(connector)
			.map_err(|e| e.into())
	}
}
