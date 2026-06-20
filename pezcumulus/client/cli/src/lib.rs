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

//! Pezcumulus CLI library.

#![warn(missing_docs)]

use std::{
	fs,
	io::{self, Write},
	path::PathBuf,
	sync::Arc,
};

use codec::Encode;
use pezsc_chain_spec::ChainSpec;
use pezsc_cli::RpcEndpoint;
use pezsc_client_api::HeaderBackend;
use pezsc_service::{
	config::{PrometheusConfig, RpcBatchRequestConfig, TelemetryEndpoints},
	BasePath, TransactionPoolOptions,
};
use pezsp_core::hexdisplay::HexDisplay;
use pezsp_runtime::traits::{Block as BlockT, Zero};
use url::Url;

/// The `purge-chain` command used to remove the whole chain: the teyrchain and the relay chain.
#[derive(Debug, clap::Parser)]
#[group(skip)]
pub struct PurgeChainCmd {
	/// The base struct of the purge-chain command.
	#[command(flatten)]
	pub base: pezsc_cli::PurgeChainCmd,

	/// Only delete the para chain database
	#[arg(long, aliases = &["para"])]
	pub teyrchain: bool,

	/// Only delete the relay chain database
	#[arg(long, aliases = &["relay"])]
	pub relaychain: bool,
}

impl PurgeChainCmd {
	/// Run the purge command
	pub fn run(
		&self,
		para_config: pezsc_service::Configuration,
		relay_config: pezsc_service::Configuration,
	) -> pezsc_cli::Result<()> {
		let databases = match (self.teyrchain, self.relaychain) {
			(true, true) | (false, false) => {
				vec![("teyrchain", para_config.database), ("relaychain", relay_config.database)]
			},
			(true, false) => vec![("teyrchain", para_config.database)],
			(false, true) => vec![("relaychain", relay_config.database)],
		};

		let db_paths = databases
			.iter()
			.map(|(chain_label, database)| {
				database.path().ok_or_else(|| {
					pezsc_cli::Error::Input(format!(
						"Cannot purge custom database implementation of: {}",
						chain_label,
					))
				})
			})
			.collect::<pezsc_cli::Result<Vec<_>>>()?;

		if !self.base.yes {
			for db_path in &db_paths {
				println!("{}", db_path.display());
			}
			print!("Are you sure to remove? [y/N]: ");
			io::stdout().flush().expect("failed to flush stdout");

			let mut input = String::new();
			io::stdin().read_line(&mut input)?;
			let input = input.trim();

			match input.chars().next() {
				Some('y') | Some('Y') => {},
				_ => {
					println!("Aborted");
					return Ok(());
				},
			}
		}

		for db_path in &db_paths {
			match fs::remove_dir_all(db_path) {
				Ok(_) => {
					println!("{:?} removed.", &db_path);
				},
				Err(ref err) if err.kind() == io::ErrorKind::NotFound => {
					eprintln!("{:?} did not exist.", &db_path);
				},
				Err(err) => return Err(err.into()),
			}
		}

		Ok(())
	}
}

impl pezsc_cli::CliConfiguration for PurgeChainCmd {
	fn shared_params(&self) -> &pezsc_cli::SharedParams {
		&self.base.shared_params
	}

	fn database_params(&self) -> Option<&pezsc_cli::DatabaseParams> {
		Some(&self.base.database_params)
	}
}

/// Get the SCALE encoded genesis header of the teyrchain.
pub fn get_raw_genesis_header<B, C>(client: Arc<C>) -> pezsc_cli::Result<Vec<u8>>
where
	B: BlockT,
	C: HeaderBackend<B> + 'static,
{
	let genesis_hash = client.hash(Zero::zero())?.ok_or(pezsc_cli::Error::Client(
		pezsp_blockchain::Error::Backend(
			"Failed to lookup genesis block hash when exporting genesis head data.".into(),
		),
	))?;
	let genesis_header = client.header(genesis_hash)?.ok_or(pezsc_cli::Error::Client(
		pezsp_blockchain::Error::Backend(
			"Failed to lookup genesis header by hash when exporting genesis head data.".into(),
		),
	))?;

	Ok(genesis_header.encode())
}

/// Command for exporting the genesis head data of the teyrchain
#[derive(Debug, clap::Parser)]
pub struct ExportGenesisHeadCommand {
	/// Output file name or stdout if unspecified.
	#[arg()]
	pub output: Option<PathBuf>,

	/// Write output in binary. Default is to write in hex.
	#[arg(short, long)]
	pub raw: bool,

	#[allow(missing_docs)]
	#[command(flatten)]
	pub shared_params: pezsc_cli::SharedParams,
}

impl ExportGenesisHeadCommand {
	/// Run the export-genesis-head command
	pub fn run<B, C>(&self, client: Arc<C>) -> pezsc_cli::Result<()>
	where
		B: BlockT,
		C: HeaderBackend<B> + 'static,
	{
		let raw_header = get_raw_genesis_header(client)?;
		let output_buf = if self.raw {
			raw_header
		} else {
			format!("0x{:?}", HexDisplay::from(&raw_header)).into_bytes()
		};

		if let Some(output) = &self.output {
			fs::write(output, output_buf)?;
		} else {
			io::stdout().write_all(&output_buf)?;
		}

		Ok(())
	}
}

impl pezsc_cli::CliConfiguration for ExportGenesisHeadCommand {
	fn shared_params(&self) -> &pezsc_cli::SharedParams {
		&self.shared_params
	}

	fn base_path(&self) -> pezsc_cli::Result<Option<BasePath>> {
		// As we are just exporting the genesis wasm a tmp database is enough.
		//
		// As otherwise we may "pollute" the global base path.
		Ok(Some(BasePath::new_temp_dir()?))
	}
}

/// Command for exporting the genesis wasm file.
#[derive(Debug, clap::Parser)]
pub struct ExportGenesisWasmCommand {
	/// Output file name or stdout if unspecified.
	#[arg()]
	pub output: Option<PathBuf>,

	/// Write output in binary. Default is to write in hex.
	#[arg(short, long)]
	pub raw: bool,

	#[allow(missing_docs)]
	#[command(flatten)]
	pub shared_params: pezsc_cli::SharedParams,
}

impl ExportGenesisWasmCommand {
	/// Run the export-genesis-wasm command
	pub fn run(&self, chain_spec: &dyn ChainSpec) -> pezsc_cli::Result<()> {
		let raw_wasm_blob = extract_genesis_wasm(chain_spec)?;
		let output_buf = if self.raw {
			raw_wasm_blob
		} else {
			format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
		};

		if let Some(output) = &self.output {
			fs::write(output, output_buf)?;
		} else {
			io::stdout().write_all(&output_buf)?;
		}

		Ok(())
	}
}

/// Extract the genesis code from a given ChainSpec.
pub fn extract_genesis_wasm(chain_spec: &dyn ChainSpec) -> pezsc_cli::Result<Vec<u8>> {
	let mut storage = chain_spec.build_storage()?;
	storage
		.top
		.remove(pezsp_core::storage::well_known_keys::CODE)
		.ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

impl pezsc_cli::CliConfiguration for ExportGenesisWasmCommand {
	fn shared_params(&self) -> &pezsc_cli::SharedParams {
		&self.shared_params
	}

	fn base_path(&self) -> pezsc_cli::Result<Option<BasePath>> {
		// As we are just exporting the genesis wasm a tmp database is enough.
		//
		// As otherwise we may "pollute" the global base path.
		Ok(Some(BasePath::new_temp_dir()?))
	}
}

fn validate_relay_chain_url(arg: &str) -> Result<Url, String> {
	let url = Url::parse(arg).map_err(|e| e.to_string())?;

	let scheme = url.scheme();
	if scheme == "ws" || scheme == "wss" {
		Ok(url)
	} else {
		Err(format!(
			"'{}' URL scheme not supported. Only websocket RPC is currently supported",
			url.scheme()
		))
	}
}

/// The `run` command used to run a node.
#[derive(Debug, clap::Parser)]
#[group(skip)]
pub struct RunCmd {
	/// The pezcumulus RunCmd inherents from pezsc_cli's
	#[command(flatten)]
	pub base: pezsc_cli::RunCmd,

	/// Run node as collator.
	///
	/// Note that this is the same as running with `--validator`.
	#[arg(long, conflicts_with = "validator")]
	pub collator: bool,

	/// Creates a less resource-hungry node that retrieves relay chain data from an RPC endpoint.
	///
	/// The provided URLs should point to RPC endpoints of the relay chain.
	/// This node connects to the remote nodes following the order they were specified in. If the
	/// connection fails, it attempts to connect to the next endpoint in the list.
	///
	/// Note: This option doesn't stop the node from connecting to the relay chain network but
	/// reduces bandwidth use.
	#[arg(
		long,
		value_parser = validate_relay_chain_url,
		num_args = 0..,
		alias = "relay-chain-rpc-url"
	)]
	pub relay_chain_rpc_urls: Vec<Url>,

	/// EXPERIMENTAL: This is meant to be used only if collator is overshooting the PoV size, and
	/// building blocks that do not fit in the max_pov_size. It is a percentage of the max_pov_size
	/// configuration of the relay-chain.
	///
	/// It will be removed once <https://github.com/pezkuwichain/pezkuwi-sdk/issues/193> is fixed.
	#[arg(long)]
	pub experimental_max_pov_percentage: Option<u32>,

	/// Disable embedded DHT bootnode.
	///
	/// Do not advertise the node as a teyrchain bootnode on the relay chain DHT.
	#[arg(long)]
	pub no_dht_bootnode: bool,

	/// Disable DHT bootnode discovery.
	///
	/// Disable discovery of the teyrchain bootnodes via the relay chain DHT.
	#[arg(long)]
	pub no_dht_bootnode_discovery: bool,
}

impl RunCmd {
	/// Create a [`NormalizedRunCmd`] which merges the `collator` cli argument into `validator` to
	/// have only one.
	pub fn normalize(&self) -> NormalizedRunCmd {
		let mut new_base = self.base.clone();

		new_base.validator = self.base.validator || self.collator;

		NormalizedRunCmd { base: new_base }
	}

	/// Create [`CollatorOptions`] representing options only relevant to teyrchain collator nodes
	pub fn collator_options(&self) -> CollatorOptions {
		let relay_chain_mode = if self.relay_chain_rpc_urls.is_empty() {
			RelayChainMode::Embedded
		} else {
			RelayChainMode::ExternalRpc(self.relay_chain_rpc_urls.clone())
		};

		CollatorOptions {
			relay_chain_mode,
			embedded_dht_bootnode: !self.no_dht_bootnode,
			dht_bootnode_discovery: !self.no_dht_bootnode_discovery,
		}
	}
}

/// Possible modes for the relay chain to operate in.
#[derive(Clone, Debug)]
pub enum RelayChainMode {
	/// Spawn embedded relay chain node
	Embedded,
	/// Connect to remote relay chain node via websocket RPC
	ExternalRpc(Vec<Url>),
}

/// Options only relevant for collator/teyrchain nodes
#[derive(Clone, Debug)]
pub struct CollatorOptions {
	/// How this collator retrieves relay chain information
	pub relay_chain_mode: RelayChainMode,
	/// Enable embedded DHT bootnode.
	pub embedded_dht_bootnode: bool,
	/// Enable DHT bootnode discovery.
	pub dht_bootnode_discovery: bool,
}

/// A non-redundant version of the `RunCmd` that sets the `validator` field when the
/// original `RunCmd` had the `collator` field.
/// This is how we make `--collator` imply `--validator`.
pub struct NormalizedRunCmd {
	/// The pezcumulus RunCmd inherents from pezsc_cli's
	pub base: pezsc_cli::RunCmd,
}

impl pezsc_cli::CliConfiguration for NormalizedRunCmd {
	fn shared_params(&self) -> &pezsc_cli::SharedParams {
		self.base.shared_params()
	}

	fn import_params(&self) -> Option<&pezsc_cli::ImportParams> {
		self.base.import_params()
	}

	fn network_params(&self) -> Option<&pezsc_cli::NetworkParams> {
		self.base.network_params()
	}

	fn keystore_params(&self) -> Option<&pezsc_cli::KeystoreParams> {
		self.base.keystore_params()
	}

	fn offchain_worker_params(&self) -> Option<&pezsc_cli::OffchainWorkerParams> {
		self.base.offchain_worker_params()
	}

	fn node_name(&self) -> pezsc_cli::Result<String> {
		self.base.node_name()
	}

	fn dev_key_seed(&self, is_dev: bool) -> pezsc_cli::Result<Option<String>> {
		self.base.dev_key_seed(is_dev)
	}

	fn telemetry_endpoints(
		&self,
		chain_spec: &Box<dyn pezsc_cli::ChainSpec>,
	) -> pezsc_cli::Result<Option<TelemetryEndpoints>> {
		self.base.telemetry_endpoints(chain_spec)
	}

	fn role(&self, is_dev: bool) -> pezsc_cli::Result<pezsc_cli::Role> {
		self.base.role(is_dev)
	}

	fn force_authoring(&self) -> pezsc_cli::Result<bool> {
		self.base.force_authoring()
	}

	fn prometheus_config(
		&self,
		default_listen_port: u16,
		chain_spec: &Box<dyn pezsc_cli::ChainSpec>,
	) -> pezsc_cli::Result<Option<PrometheusConfig>> {
		self.base.prometheus_config(default_listen_port, chain_spec)
	}

	fn disable_grandpa(&self) -> pezsc_cli::Result<bool> {
		self.base.disable_grandpa()
	}

	fn rpc_max_connections(&self) -> pezsc_cli::Result<u32> {
		self.base.rpc_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> pezsc_cli::Result<Option<Vec<String>>> {
		self.base.rpc_cors(is_dev)
	}

	fn rpc_addr(&self, default_listen_port: u16) -> pezsc_cli::Result<Option<Vec<RpcEndpoint>>> {
		self.base.rpc_addr(default_listen_port)
	}

	fn rpc_methods(&self) -> pezsc_cli::Result<pezsc_service::config::RpcMethods> {
		self.base.rpc_methods()
	}

	fn rpc_rate_limit(&self) -> pezsc_cli::Result<Option<std::num::NonZeroU32>> {
		Ok(self.base.rpc_params.rpc_rate_limit)
	}

	fn rpc_rate_limit_whitelisted_ips(
		&self,
	) -> pezsc_cli::Result<Vec<pezsc_service::config::IpNetwork>> {
		Ok(self.base.rpc_params.rpc_rate_limit_whitelisted_ips.clone())
	}

	fn rpc_rate_limit_trust_proxy_headers(&self) -> pezsc_cli::Result<bool> {
		Ok(self.base.rpc_params.rpc_rate_limit_trust_proxy_headers)
	}

	fn rpc_max_request_size(&self) -> pezsc_cli::Result<u32> {
		self.base.rpc_max_request_size()
	}

	fn rpc_max_response_size(&self) -> pezsc_cli::Result<u32> {
		self.base.rpc_max_response_size()
	}

	fn rpc_max_subscriptions_per_connection(&self) -> pezsc_cli::Result<u32> {
		self.base.rpc_max_subscriptions_per_connection()
	}

	fn rpc_buffer_capacity_per_connection(&self) -> pezsc_cli::Result<u32> {
		Ok(self.base.rpc_params.rpc_message_buffer_capacity_per_connection)
	}

	fn rpc_batch_config(&self) -> pezsc_cli::Result<RpcBatchRequestConfig> {
		self.base.rpc_batch_config()
	}

	fn transaction_pool(&self, is_dev: bool) -> pezsc_cli::Result<TransactionPoolOptions> {
		self.base.transaction_pool(is_dev)
	}

	fn max_runtime_instances(&self) -> pezsc_cli::Result<Option<usize>> {
		self.base.max_runtime_instances()
	}

	fn runtime_cache_size(&self) -> pezsc_cli::Result<u8> {
		self.base.runtime_cache_size()
	}

	fn base_path(&self) -> pezsc_cli::Result<Option<BasePath>> {
		self.base.base_path()
	}
}
