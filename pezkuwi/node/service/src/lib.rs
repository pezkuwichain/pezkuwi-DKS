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

//! Pezkuwi service. Specialized wrapper over bizinikiwi service.

#![deny(unused_results)]

pub mod benchmarking;
pub mod chain_spec;
mod fake_runtime_api;
mod grandpa_support;
mod relay_chain_selection;
mod teyrchains_db;

#[cfg(feature = "full-node")]
pub mod builder;
#[cfg(feature = "full-node")]
pub mod overseer;
#[cfg(feature = "full-node")]
pub mod workers;

#[cfg(feature = "full-node")]
pub use crate::builder::{new_full, NewFull, NewFullParams};

#[cfg(feature = "full-node")]
pub use self::overseer::{
	CollatorOverseerGen, ExtendedOverseerGenArgs, OverseerGen, OverseerGenArgs,
	ValidatorOverseerGen,
};

#[cfg(test)]
mod tests;

#[cfg(feature = "full-node")]
use crate::builder::{new_partial, new_partial_basics};

#[cfg(feature = "full-node")]
use {
	pezkuwi_node_core_approval_voting as approval_voting_subsystem,
	pezkuwi_node_core_av_store::Error as AvailabilityError,
	pezkuwi_node_core_chain_selection as chain_selection_subsystem,
};

use pezkuwi_node_subsystem_util::database::Database;
use pezkuwi_overseer::SpawnGlue;

#[cfg(feature = "full-node")]
pub use {
	pezkuwi_overseer::{Handle, Overseer, OverseerConnector, OverseerHandle},
	pezkuwi_primitives::runtime_api::TeyrchainHost,
	pezsc_client_api::AuxStore,
	pezsp_authority_discovery::AuthorityDiscoveryApi,
	pezsp_blockchain::{HeaderBackend, HeaderMetadata},
	pezsp_consensus_babe::BabeApi,
	relay_chain_selection::SelectRelayChain,
};

use std::{path::PathBuf, sync::Arc};

use pezsc_service::SpawnTaskHandle;
use prometheus_endpoint::Registry;

pub use chain_spec::{GenericChainSpec, PezkuwichainChainSpec, ZagrosChainSpec};
pub use pezkuwi_primitives::{Block, BlockId, BlockNumber, CollatorPair, Hash, Id as ParaId};
pub use pezsc_client_api::{Backend, CallExecutor};
pub use pezsc_consensus::{BlockImport, LongestChain};
pub use pezsc_executor::NativeExecutionDispatch;
use pezsc_executor::WasmExecutor;
pub use pezsc_service::{
	config::{DatabaseSource, PrometheusConfig},
	ChainSpec, Configuration, Error as BizinikiwiServiceError, PruningMode, Role, TFullBackend,
	TFullCallExecutor, TFullClient, TaskManager, TransactionPoolOptions,
};
pub use pezsp_api::{ApiRef, ConstructRuntimeApi, Core as CoreApi, ProvideRuntimeApi};
pub use pezsp_consensus::{Proposal, SelectChain};
pub use pezsp_runtime::{
	generic,
	traits::{self as runtime_traits, BlakeTwo256, Block as BlockT, Header as HeaderT, NumberFor},
};

#[cfg(feature = "pezkuwichain-native")]
pub use {pezkuwichain_runtime, pezkuwichain_runtime_constants};
#[cfg(feature = "zagros-native")]
pub use {zagros_runtime, zagros_runtime_constants};

pub use fake_runtime_api::{GetLastTimestamp, RuntimeApi};

#[cfg(feature = "full-node")]
pub type FullBackend = pezsc_service::TFullBackend<Block>;

#[cfg(feature = "full-node")]
pub type FullClient = pezsc_service::TFullClient<
	Block,
	RuntimeApi,
	WasmExecutor<(
		pezsp_io::BizinikiwiHostFunctions,
		pezframe_benchmarking::benchmarking::HostFunctions,
	)>,
>;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

/// The number of hours to keep finalized data in the availability store for live networks.
const KEEP_FINALIZED_FOR_LIVE_NETWORKS: u32 = 25;

/// Provides the header and block number for a hash.
///
/// Decouples `pezsc_client_api::Backend` and `pezsp_blockchain::HeaderBackend`.
pub trait HeaderProvider<Block, Error = pezsp_blockchain::Error>: Send + Sync + 'static
where
	Block: BlockT,
	Error: std::fmt::Debug + Send + Sync + 'static,
{
	/// Obtain the header for a hash.
	fn header(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> Result<Option<<Block as BlockT>::Header>, Error>;
	/// Obtain the block number for a hash.
	fn number(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> Result<Option<<<Block as BlockT>::Header as HeaderT>::Number>, Error>;
}

impl<Block, T> HeaderProvider<Block> for T
where
	Block: BlockT,
	T: pezsp_blockchain::HeaderBackend<Block> + 'static,
{
	fn header(
		&self,
		hash: Block::Hash,
	) -> pezsp_blockchain::Result<Option<<Block as BlockT>::Header>> {
		<Self as pezsp_blockchain::HeaderBackend<Block>>::header(self, hash)
	}
	fn number(
		&self,
		hash: Block::Hash,
	) -> pezsp_blockchain::Result<Option<<<Block as BlockT>::Header as HeaderT>::Number>> {
		<Self as pezsp_blockchain::HeaderBackend<Block>>::number(self, hash)
	}
}

/// Decoupling the provider.
///
/// Mandated since `trait HeaderProvider` can only be
/// implemented once for a generic `T`.
pub trait HeaderProviderProvider<Block>: Send + Sync + 'static
where
	Block: BlockT,
{
	type Provider: HeaderProvider<Block> + 'static;

	fn header_provider(&self) -> &Self::Provider;
}

impl<Block, T> HeaderProviderProvider<Block> for T
where
	Block: BlockT,
	T: pezsc_client_api::Backend<Block> + 'static,
{
	type Provider = <T as pezsc_client_api::Backend<Block>>::Blockchain;

	fn header_provider(&self) -> &Self::Provider {
		self.blockchain()
	}
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	Io(#[from] std::io::Error),

	#[error(transparent)]
	AddrFormatInvalid(#[from] std::net::AddrParseError),

	#[error(transparent)]
	Sub(#[from] BizinikiwiServiceError),

	#[error(transparent)]
	Blockchain(#[from] pezsp_blockchain::Error),

	#[error(transparent)]
	Consensus(#[from] pezsp_consensus::Error),

	#[error("Failed to create an overseer")]
	Overseer(#[from] pezkuwi_overseer::SubsystemError),

	#[error(transparent)]
	Prometheus(#[from] prometheus_endpoint::PrometheusError),

	#[error(transparent)]
	Telemetry(#[from] pezsc_telemetry::Error),

	#[cfg(feature = "full-node")]
	#[error(transparent)]
	Availability(#[from] AvailabilityError),

	#[error("Authorities require the real overseer implementation")]
	AuthoritiesRequireRealOverseer,

	#[cfg(feature = "full-node")]
	#[error("Creating a custom database is required for validators")]
	DatabasePathRequired,

	#[cfg(feature = "full-node")]
	#[error("Expected at least one of pezkuwi, dicle, zagros or pezkuwichain runtime feature")]
	NoRuntime,

	#[cfg(feature = "full-node")]
	#[error("Worker binaries not executable, prepare binary: {prep_worker_path:?}, execute binary: {exec_worker_path:?}")]
	InvalidWorkerBinaries { prep_worker_path: PathBuf, exec_worker_path: PathBuf },

	#[cfg(feature = "full-node")]
	#[error("Worker binaries could not be found, make sure pezkuwi was built and installed correctly. Please see the readme for the latest instructions (https://github.com/pezkuwichain/pezkuwi-sdk/tree/main/pezkuwi). If you ran with `cargo run`, please run `cargo build` first. Searched given workers path ({given_workers_path:?}), pezkuwi binary path ({current_exe_path:?}), and lib path (/usr/lib/pezkuwi), workers names: {workers_names:?}")]
	MissingWorkerBinaries {
		given_workers_path: Option<PathBuf>,
		current_exe_path: PathBuf,
		workers_names: Option<(String, String)>,
	},

	#[cfg(feature = "full-node")]
	#[error("Version of worker binary ({worker_version}) is different from node version ({node_version}), worker_path: {worker_path}. If you ran with `cargo run`, please run `cargo build` first, otherwise try to `cargo clean`. TESTING ONLY: this check can be disabled with --disable-worker-version-check")]
	WorkerBinaryVersionMismatch {
		worker_version: String,
		node_version: String,
		worker_path: PathBuf,
	},
}

/// Identifies the variant of the chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Chain {
	/// Pezkuwi.
	Pezkuwi,
	/// Dicle.
	Dicle,
	/// Pezkuwichain or one of its derivations.
	Pezkuwichain,
	/// Zagros.
	Zagros,
	/// Unknown chain?
	Unknown,
}

/// Can be called for a `Configuration` to identify which network the configuration targets.
pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `Pezkuwi` network.
	fn is_pezkuwi(&self) -> bool;

	/// Returns if this is a configuration for the `Dicle` network.
	fn is_dicle(&self) -> bool;

	/// Returns if this is a configuration for the `Zagros` network.
	fn is_zagros(&self) -> bool;

	/// Returns if this is a configuration for the `Pezkuwichain` network.
	fn is_pezkuwichain(&self) -> bool;

	/// Returns if this is a configuration for the `Versi` test network.
	fn is_versi(&self) -> bool;

	/// Returns true if this configuration is for a development network.
	fn is_dev(&self) -> bool;

	/// Identifies the variant of the chain.
	fn identify_chain(&self) -> Chain;
}

impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_pezkuwi(&self) -> bool {
		self.id().starts_with("pezkuwi") || self.id().starts_with("hez")
	}
	fn is_dicle(&self) -> bool {
		self.id().starts_with("dicle") || self.id().starts_with("dcl")
	}
	fn is_zagros(&self) -> bool {
		self.id().starts_with("zagros") || self.id().starts_with("wnd")
	}
	fn is_pezkuwichain(&self) -> bool {
		self.id().starts_with("pezkuwichain") || self.id().starts_with("rco")
	}
	fn is_versi(&self) -> bool {
		self.id().starts_with("versi") || self.id().starts_with("vrs")
	}
	fn is_dev(&self) -> bool {
		self.id().ends_with("dev")
	}
	fn identify_chain(&self) -> Chain {
		if self.is_pezkuwi() {
			Chain::Pezkuwi
		} else if self.is_dicle() {
			Chain::Dicle
		} else if self.is_zagros() {
			Chain::Zagros
		} else if self.is_pezkuwichain() || self.is_versi() {
			Chain::Pezkuwichain
		} else {
			Chain::Unknown
		}
	}
}

#[cfg(feature = "full-node")]
pub fn open_database(db_source: &DatabaseSource) -> Result<Arc<dyn Database>, Error> {
	let teyrchains_db = match db_source {
		DatabaseSource::RocksDb { path, .. } => teyrchains_db::open_creating_rocksdb(
			path.clone(),
			teyrchains_db::CacheSizes::default(),
		)?,
		DatabaseSource::ParityDb { path, .. } => teyrchains_db::open_creating_paritydb(
			path.parent().ok_or(Error::DatabasePathRequired)?.into(),
			teyrchains_db::CacheSizes::default(),
		)?,
		DatabaseSource::Auto { paritydb_path, rocksdb_path, .. } => {
			if paritydb_path.is_dir() && paritydb_path.exists() {
				teyrchains_db::open_creating_paritydb(
					paritydb_path.parent().ok_or(Error::DatabasePathRequired)?.into(),
					teyrchains_db::CacheSizes::default(),
				)?
			} else {
				teyrchains_db::open_creating_rocksdb(
					rocksdb_path.clone(),
					teyrchains_db::CacheSizes::default(),
				)?
			}
		},
		DatabaseSource::Custom { .. } => {
			unimplemented!("No pezkuwi subsystem db for custom source.");
		},
	};
	Ok(teyrchains_db)
}

/// Is this node running as in-process node for a teyrchain node?
#[cfg(feature = "full-node")]
#[derive(Clone)]
pub enum IsTeyrchainNode {
	/// This node is running as in-process node for a teyrchain collator.
	Collator(CollatorPair),
	/// This node is running as in-process node for a teyrchain full node.
	FullNode,
	/// This node is not running as in-process node for a teyrchain node, aka a normal relay chain
	/// node.
	No,
}

#[cfg(feature = "full-node")]
impl std::fmt::Debug for IsTeyrchainNode {
	fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
		use pezsp_core::Pair;
		match self {
			IsTeyrchainNode::Collator(pair) => write!(fmt, "Collator({})", pair.public()),
			IsTeyrchainNode::FullNode => write!(fmt, "FullNode"),
			IsTeyrchainNode::No => write!(fmt, "No"),
		}
	}
}

#[cfg(feature = "full-node")]
impl IsTeyrchainNode {
	/// Is this running alongside a collator?
	fn is_collator(&self) -> bool {
		matches!(self, Self::Collator(_))
	}

	/// Is this running alongside a full node?
	fn is_full_node(&self) -> bool {
		matches!(self, Self::FullNode)
	}

	/// Is this node running alongside a relay chain node?
	fn is_running_alongside_teyrchain_node(&self) -> bool {
		self.is_collator() || self.is_full_node()
	}
}

#[cfg(feature = "full-node")]
macro_rules! chain_ops {
	($config:expr, $telemetry_worker_handle:expr) => {{
		let telemetry_worker_handle = $telemetry_worker_handle;
		let mut config = $config;
		let basics = new_partial_basics(config, telemetry_worker_handle)?;

		use ::pezsc_consensus::LongestChain;
		// use the longest chain selection, since there is no overseer available
		let chain_selection = LongestChain::new(basics.backend.clone());

		let pezsc_service::PartialComponents {
			client, backend, import_queue, task_manager, ..
		} = new_partial::<LongestChain<_, Block>>(&mut config, basics, chain_selection)?;
		Ok((client, backend, import_queue, task_manager))
	}};
}

/// Builds a new object suitable for chain operations.
#[cfg(feature = "full-node")]
pub fn new_chain_ops(
	config: &mut Configuration,
) -> Result<
	(Arc<FullClient>, Arc<FullBackend>, pezsc_consensus::BasicQueue<Block>, TaskManager),
	Error,
> {
	config.keystore = pezsc_service::config::KeystoreConfig::InMemory;

	if config.chain_spec.is_pezkuwichain() || config.chain_spec.is_versi() {
		chain_ops!(config, None)
	} else if config.chain_spec.is_dicle() {
		chain_ops!(config, None)
	} else if config.chain_spec.is_zagros() {
		return chain_ops!(config, None);
	} else {
		chain_ops!(config, None)
	}
}

/// Build a full node.
///
/// The actual "flavor", aka if it will use `Pezkuwi`, `Pezkuwichain` or `Dicle` is determined
/// based on [`IdentifyVariant`] using the chain spec.
#[cfg(feature = "full-node")]
pub fn build_full<OverseerGenerator: OverseerGen>(
	config: Configuration,
	mut params: NewFullParams<OverseerGenerator>,
) -> Result<NewFull, Error> {
	let is_pezkuwi = config.chain_spec.is_pezkuwi();

	params.overseer_message_channel_capacity_override =
		params.overseer_message_channel_capacity_override.map(move |capacity| {
			if is_pezkuwi {
				gum::warn!("Channel capacity should _never_ be tampered with on pezkuwi!");
			}
			capacity
		});

	match config.network.network_backend {
		pezsc_network::config::NetworkBackendType::Libp2p => {
			new_full::<_, pezsc_network::NetworkWorker<Block, Hash>>(config, params)
		},
		pezsc_network::config::NetworkBackendType::Litep2p => {
			new_full::<_, pezsc_network::Litep2pNetworkBackend>(config, params)
		},
	}
}

/// Reverts the node state down to at most the last finalized block.
///
/// In particular this reverts:
/// - `ApprovalVotingSubsystem` data in the teyrchains-db;
/// - `ChainSelectionSubsystem` data in the teyrchains-db;
/// - Low level Babe and Grandpa consensus data.
#[cfg(feature = "full-node")]
pub fn revert_backend(
	client: Arc<FullClient>,
	backend: Arc<FullBackend>,
	blocks: BlockNumber,
	config: Configuration,
	task_handle: SpawnTaskHandle,
) -> Result<(), Error> {
	let best_number = client.info().best_number;
	let finalized = client.info().finalized_number;
	let revertible = blocks.min(best_number - finalized);

	if revertible == 0 {
		return Ok(());
	}

	let number = best_number - revertible;
	let hash = client.block_hash_from_id(&BlockId::Number(number))?.ok_or(
		pezsp_blockchain::Error::Backend(format!(
			"Unexpected hash lookup failure for block number: {}",
			number
		)),
	)?;

	let teyrchains_db = open_database(&config.database)
		.map_err(|err| pezsp_blockchain::Error::Backend(err.to_string()))?;

	revert_approval_voting(teyrchains_db.clone(), hash, task_handle)?;
	revert_chain_selection(teyrchains_db, hash)?;
	// Revert Bizinikiwi consensus related components
	pezsc_consensus_babe::revert(client.clone(), backend, blocks)?;
	pezsc_consensus_grandpa::revert(client, blocks)?;

	Ok(())
}

fn revert_chain_selection(db: Arc<dyn Database>, hash: Hash) -> pezsp_blockchain::Result<()> {
	let config = chain_selection_subsystem::Config {
		col_data: teyrchains_db::REAL_COLUMNS.col_chain_selection_data,
		stagnant_check_interval: chain_selection_subsystem::StagnantCheckInterval::never(),
		stagnant_check_mode: chain_selection_subsystem::StagnantCheckMode::PruneOnly,
	};

	let chain_selection = chain_selection_subsystem::ChainSelectionSubsystem::new(config, db);

	chain_selection
		.revert_to(hash)
		.map_err(|err| pezsp_blockchain::Error::Backend(err.to_string()))
}

fn revert_approval_voting(
	db: Arc<dyn Database>,
	hash: Hash,
	task_handle: SpawnTaskHandle,
) -> pezsp_blockchain::Result<()> {
	let config = approval_voting_subsystem::Config {
		col_approval_data: teyrchains_db::REAL_COLUMNS.col_approval_data,
		slot_duration_millis: Default::default(),
	};

	let approval_voting = approval_voting_subsystem::ApprovalVotingSubsystem::with_config(
		config,
		db,
		Arc::new(pezsc_keystore::LocalKeystore::in_memory()),
		Box::new(pezsp_consensus::NoNetwork),
		approval_voting_subsystem::Metrics::default(),
		Arc::new(SpawnGlue(task_handle)),
	);

	approval_voting
		.revert_to(hash)
		.map_err(|err| pezsp_blockchain::Error::Backend(err.to_string()))
}
