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

use crate::cli::{Cli, Subcommand, NODE_VERSION};
use futures::future::TryFutureExt;
use pezframe_benchmarking_cli::{
	BenchmarkCmd, BizinikiwiRemarkBuilder, ExtrinsicFactory, BIZINIKIWI_REFERENCE_HARDWARE,
};
use pezkuwi_service::{
	self,
	benchmarking::{benchmark_inherent_data, TransferKeepAliveBuilder},
	HeaderBackend, IdentifyVariant,
};
use pezsc_cli::BizinikiwiCli;
use pezsc_network_types::PeerId;
use pezsp_core::crypto::Ss58AddressFormatRegistry;
use pezsp_keyring::Sr25519Keyring;
#[cfg(feature = "pyroscope")]
use pyroscope_pprofrs::{pprof_backend, PprofConfig};

pub use crate::error::Error;
#[cfg(feature = "pyroscope")]
use std::net::ToSocketAddrs;
use std::{collections::HashSet, time::Duration};

type Result<T> = std::result::Result<T, Error>;

fn get_exec_name() -> Option<String> {
	std::env::current_exe()
		.ok()
		.and_then(|pb| pb.file_name().map(|s| s.to_os_string()))
		.and_then(|s| s.into_string().ok())
}

/// Peer ids of the Asset Hub collators the collator protocol treats as invulnerable.
///
/// This is a per-network list of operator-run collators, so it cannot be inherited from
/// anywhere: the only correct value is our own collators' peer ids, and those are fixed when
/// the network's genesis is. Empty until then -- an empty set costs the collator protocol its
/// preferential treatment, whereas a foreign set would reserve our slots for other people's
/// nodes.
fn get_invulnerable_ah_collators() -> HashSet<PeerId> {
	HashSet::new()
}

impl BizinikiwiCli for Cli {
	fn impl_name() -> String {
		"Parity Pezkuwi".into()
	}

	fn impl_version() -> String {
		let commit_hash = env!("BIZINIKIWI_CLI_COMMIT_HASH");
		format!("{}-{commit_hash}", NODE_VERSION)
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/pezkuwichain/pezkuwi-sdk/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn executable_name() -> String {
		"pezkuwi".into()
	}

	fn load_spec(
		&self,
		id: &str,
	) -> std::result::Result<Box<dyn pezsc_service::ChainSpec>, String> {
		let id = if id == "" {
			let n = get_exec_name().unwrap_or_default();
			// Both networks this node serves. `pezkuwichain` is last-resort rather than a
			// guess: running the mainnet spec by accident is loud and harmless, whereas
			// defaulting to a testnet would have a validator quietly author on the wrong chain.
			["zagros", "pezkuwichain"]
				.iter()
				.cloned()
				.find(|&chain| n.starts_with(chain))
				.unwrap_or("pezkuwichain")
		} else {
			id
		};
		Ok(match id {
			"pezkuwichain" => Box::new(pezkuwi_service::chain_spec::pezkuwichain_config()?),
			#[cfg(feature = "pezkuwichain-native")]
			"pezkuwichain-mainnet" => Box::new(pezkuwi_service::chain_spec::pezkuwichain_mainnet_config()?),
			#[cfg(feature = "pezkuwichain-native")]
			"dev" | "pezkuwichain-dev" => {
				Box::new(pezkuwi_service::chain_spec::pezkuwichain_development_config()?)
			},
			#[cfg(feature = "pezkuwichain-native")]
			"pezkuwichain-local" => {
				Box::new(pezkuwi_service::chain_spec::pezkuwichain_local_testnet_config()?)
			},
			#[cfg(feature = "pezkuwichain-native")]
			"pezkuwichain-staging" => {
				Box::new(pezkuwi_service::chain_spec::pezkuwichain_staging_testnet_config()?)
			},
			#[cfg(feature = "pezkuwichain-native")]
			"mainnet-sim" | "mainnet-simulation" => {
				Box::new(pezkuwi_service::chain_spec::pezkuwichain_mainnet_simulation_config()?)
			},
			#[cfg(not(feature = "pezkuwichain-native"))]
			name if name.starts_with("pezkuwichain-") && !name.ends_with(".json")
				|| name == "dev" =>
			{
				Err(format!(
					"`{}` only supported with `pezkuwichain-native` feature enabled.",
					name
				))?
			},
			"zagros" => Box::new(pezkuwi_service::chain_spec::zagros_config()?),
			#[cfg(feature = "zagros-native")]
			"zagros-dev" => Box::new(pezkuwi_service::chain_spec::zagros_development_config()?),
			#[cfg(feature = "zagros-native")]
			"zagros-local" => Box::new(pezkuwi_service::chain_spec::zagros_local_testnet_config()?),
			#[cfg(feature = "zagros-native")]
			"zagros-staging" => Box::new(pezkuwi_service::chain_spec::zagros_staging_testnet_config()?),
			path => {
				let path = std::path::PathBuf::from(path);

				let chain_spec =
					Box::new(pezkuwi_service::GenericChainSpec::from_json_file(path.clone())?)
						as Box<dyn pezkuwi_service::ChainSpec>;

				// When `force_*` is given or the file name starts with the name of one of the known
				// chains, we use the chain spec for the specific chain.
				if self.run.force_pezkuwichain || chain_spec.is_pezkuwichain() {
					Box::new(pezkuwi_service::PezkuwichainChainSpec::from_json_file(path)?)
				} else if self.run.force_zagros || chain_spec.is_zagros() {
					Box::new(pezkuwi_service::ZagrosChainSpec::from_json_file(path)?)
				} else {
					chain_spec
				}
			},
		})
	}
}

fn set_default_ss58_version(spec: &Box<dyn pezkuwi_service::ChainSpec>) {
	let ss58_version = if spec.is_zagros() {
		Ss58AddressFormatRegistry::ZagrosAccount
	} else {
		Ss58AddressFormatRegistry::PezkuwichainAccount
	}
	.into();

	pezsp_core::crypto::set_default_ss58_version(ss58_version);
}

/// Launch a node, accepting arguments just like a regular node,
/// accepts an alternative overseer generator, to adjust behavior
/// for integration tests as needed.
/// `malus_finality_delay` restrict finality votes of this node
/// to be at most `best_block - malus_finality_delay` height.
#[cfg(feature = "malus")]
pub fn run_node(
	run: Cli,
	overseer_gen: impl pezkuwi_service::OverseerGen,
	malus_finality_delay: Option<u32>,
) -> Result<()> {
	run_node_inner(run, overseer_gen, malus_finality_delay, |_logger_builder, _config| {})
}

fn run_node_inner<F>(
	cli: Cli,
	overseer_gen: impl pezkuwi_service::OverseerGen,
	maybe_malus_finality_delay: Option<u32>,
	logger_hook: F,
) -> Result<()>
where
	F: FnOnce(&mut pezsc_cli::LoggerBuilder, &pezsc_service::Configuration),
{
	let runner = cli
		.create_runner_with_logger_hook::<_, _, F>(&cli.run.base, logger_hook)
		.map_err(Error::from)?;
	let chain_spec = &runner.config().chain_spec;

	// By default, enable BEEFY on all networks, unless explicitly disabled through CLI.
	let enable_beefy = !cli.run.no_beefy;

	set_default_ss58_version(chain_spec);

	let node_version =
		if cli.run.disable_worker_version_check { None } else { Some(NODE_VERSION.to_string()) };

	let secure_validator_mode = cli.run.base.validator && !cli.run.insecure_validator;

	// Parse collator protocol hold off value and get the list of the invlunerable collators.
	let collator_protocol_hold_off = cli.run.collator_protocol_hold_off.map(Duration::from_millis);
	let invulnerable_ah_collators = get_invulnerable_ah_collators();
	let experimental_collator_protocol = cli.run.experimental_collator_protocol;

	runner.run_node_until_exit(move |config| async move {
		let hwbench = (!cli.run.no_hardware_benchmarks)
			.then(|| {
				config.database.path().map(|database_path| {
					let _ = std::fs::create_dir_all(&database_path);
					pezsc_sysinfo::gather_hwbench(
						Some(database_path),
						&BIZINIKIWI_REFERENCE_HARDWARE,
					)
				})
			})
			.flatten();

		let database_source = config.database.clone();
		let task_manager = pezkuwi_service::build_full(
			config,
			pezkuwi_service::NewFullParams {
				is_teyrchain_node: pezkuwi_service::IsTeyrchainNode::No,
				enable_beefy,
				force_authoring_backoff: cli.run.force_authoring_backoff,
				telemetry_worker_handle: None,
				node_version,
				secure_validator_mode,
				workers_path: cli.run.workers_path,
				workers_names: None,
				overseer_gen,
				overseer_message_channel_capacity_override: cli
					.run
					.overseer_channel_capacity_override,
				malus_finality_delay: maybe_malus_finality_delay,
				hwbench,
				execute_workers_max_num: cli.run.execute_workers_max_num,
				prepare_workers_hard_max_num: cli.run.prepare_workers_hard_max_num,
				prepare_workers_soft_max_num: cli.run.prepare_workers_soft_max_num,
				keep_finalized_for: cli.run.keep_finalized_for,
				invulnerable_ah_collators,
				collator_protocol_hold_off,
				experimental_collator_protocol,
				collator_reputation_persist_interval: cli
					.run
					.collator_reputation_persist_interval
					.map(std::time::Duration::from_secs),
			},
		)
		.map(|full| full.task_manager)?;

		if let Some(path) = database_source.path() {
			pezsc_storage_monitor::StorageMonitorService::try_spawn(
				cli.storage_monitor,
				path.to_path_buf(),
				&task_manager.spawn_essential_handle(),
			)?;
		}

		Ok(task_manager)
	})
}

/// Parses pezkuwi specific CLI arguments and run the service.
pub fn run() -> Result<()> {
	let cli: Cli = Cli::from_args();

	#[cfg(feature = "pyroscope")]
	let mut pyroscope_agent_maybe = if let Some(ref agent_addr) = cli.run.pyroscope_server {
		let address = agent_addr
			.to_socket_addrs()
			.map_err(Error::AddressResolutionFailure)?
			.next()
			.ok_or_else(|| Error::AddressResolutionMissing)?;
		// The pyroscope agent requires a `http://` prefix, so we just do that.
		let agent = pyroscope::PyroscopeAgent::builder(
			"http://".to_owned() + address.to_string().as_str(),
			"pezkuwi".to_owned(),
		)
		.backend(pprof_backend(PprofConfig::new().sample_rate(113)))
		.build()?;
		Some(agent.start()?)
	} else {
		None
	};

	#[cfg(not(feature = "pyroscope"))]
	if cli.run.pyroscope_server.is_some() {
		return Err(Error::PyroscopeNotCompiledIn);
	}

	match &cli.subcommand {
		None => run_node_inner(
			cli,
			pezkuwi_service::ValidatorOverseerGen,
			None,
			pezkuwi_node_metrics::logger_hook(),
		),
		#[allow(deprecated)]
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run(config.chain_spec, config.network))?)
		},
		Some(Subcommand::ExportChainSpec(cmd)) => {
			// Directly load the embedded chain spec using the CLI’s load_spec method.
			let spec = cli.load_spec(&cmd.chain)?;
			cmd.run(spec).map_err(Into::into)
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd).map_err(Error::BizinikiwiCli)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) =
					pezkuwi_service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue).map_err(Error::BizinikiwiCli), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) =
					pezkuwi_service::new_chain_ops(&mut config).map_err(Error::PezkuwiService)?;
				Ok((cmd.run(client, config.database).map_err(Error::BizinikiwiCli), task_manager))
			})?)
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, _, task_manager) = pezkuwi_service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, config.chain_spec).map_err(Error::BizinikiwiCli), task_manager))
			})?)
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager) =
					pezkuwi_service::new_chain_ops(&mut config)?;
				Ok((cmd.run(client, import_queue).map_err(Error::BizinikiwiCli), task_manager))
			})?)
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run(config.database))?)
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			Ok(runner.async_run(|mut config| {
				let (client, backend, _, task_manager) =
					pezkuwi_service::new_chain_ops(&mut config)?;
				let task_handle = task_manager.spawn_handle();
				let aux_revert = Box::new(|client, backend, blocks| {
					pezkuwi_service::revert_backend(client, backend, blocks, config, task_handle)
						.map_err(|err| {
							match err {
								pezkuwi_service::Error::Blockchain(err) => err.into(),
								// Generic application-specific error.
								err => pezsc_cli::Error::Application(err.into()),
							}
						})
				});
				Ok((
					cmd.run(client, backend, Some(aux_revert)).map_err(Error::BizinikiwiCli),
					task_manager,
				))
			})?)
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			match cmd {
				// Storage benchmarks are only available with pezframe-benchmarking-cli's
				// `storage-benchmark` feature enabled, which requires test runtime crate.
				// #[cfg(not(feature = "runtime-benchmarks"))]
				// BenchmarkCmd::Storage(_) =>
				// 	return Err(pezsc_cli::Error::Input(
				// 		"Compile with --features=runtime-benchmarks \
				// 		to enable storage benchmarks."
				// 			.into(),
				// 	)
				// 	.into()),
				// #[cfg(feature = "runtime-benchmarks")]
				// BenchmarkCmd::Storage(cmd) => runner.sync_run(|mut config| {
				// 	let (client, backend, _, _) = pezkuwi_service::new_chain_ops(&mut config)?;
				// 	let db = backend.expose_db();
				// 	let storage = backend.expose_storage();
				// 	let shared_trie_cache = backend.expose_shared_trie_cache();
				//
				// 	cmd.run(config, client.clone(), db, storage, shared_trie_cache).map_err(Error::BizinikiwiCli)
				// }),
				BenchmarkCmd::Block(cmd) => runner.sync_run(|mut config| {
					// Keep `_import_queue`/`_task_manager` bound (not `_`) so they — and the
					// babe-worker/transaction-pool essential tasks `_task_manager` anchors via
					// `keep_alive` in `new_partial`, plus the import queue's own
					// `basic-block-import-worker` — stay alive for the whole closure, not just
					// until this statement ends. See the comment on `task_manager.keep_alive(...)`
					// in node/service/src/builder/partial.rs.
					let (client, _, _import_queue, _task_manager) =
						pezkuwi_service::new_chain_ops(&mut config)?;

					cmd.run(client.clone()).map_err(Error::BizinikiwiCli)
				}),
				BenchmarkCmd::Overhead(cmd) => runner.sync_run(|config| {
					if cmd.params.runtime.is_some() {
						return Err(pezsc_cli::Error::Input(
							"Pezkuwi binary does not support `--runtime` flag for `benchmark overhead`. Please provide a chain spec or use the `frame-omni-bencher`."
								.into(),
						)
						.into())
					}

					cmd.run_with_default_builder_and_spec::<pezkuwi_service::Block, ()>(
						Some(config.chain_spec),
					)
					.map_err(Error::BizinikiwiCli)
				}),
				BenchmarkCmd::Extrinsic(cmd) => runner.sync_run(|mut config| {
					// Keep `_import_queue`/`_task_manager` bound (not `_`) so they stay alive for
					// the whole closure, not just until this statement ends. Without this, the
					// import queue's `basic-block-import-worker` task (from the discarded
					// `_import_queue`) and the babe-worker/transaction-pool essential tasks
					// `task_manager` anchors via `keep_alive` in `new_partial` all complete early
					// (their last live handle drops right here) and get logged+treated as
					// essential-task failures, which fails the whole `benchmark extrinsic`
					// command even though the benchmark itself succeeds. See the comment on
					// `task_manager.keep_alive(...)` in node/service/src/builder/partial.rs.
					let (client, _, _import_queue, _task_manager) =
						pezkuwi_service::new_chain_ops(&mut config)?;
					let header = client.header(client.info().genesis_hash).unwrap().unwrap();
					let inherent_data = benchmark_inherent_data(header)
						.map_err(|e| format!("generating inherent data: {:?}", e))?;

					let remark_builder = BizinikiwiRemarkBuilder::new_from_client(client.clone())?;

					let tka_builder = TransferKeepAliveBuilder::new(
						client.clone(),
						Sr25519Keyring::Alice.to_account_id(),
						config.chain_spec.identify_chain(),
					);

					let ext_factory =
						ExtrinsicFactory(vec![Box::new(remark_builder), Box::new(tka_builder)]);

					cmd.run(client.clone(), inherent_data, Vec::new(), &ext_factory)
						.map_err(Error::BizinikiwiCli)
				}),
				BenchmarkCmd::Pezpallet(cmd) => {
					set_default_ss58_version(chain_spec);

					if cfg!(feature = "runtime-benchmarks") {
						runner.sync_run(|config| {
							cmd.run_with_spec::<pezsp_runtime::traits::HashingFor<pezkuwi_service::Block>, ()>(
								Some(config.chain_spec),
							)
							.map_err(|e| Error::BizinikiwiCli(e))
						})
					} else {
						Err(pezsc_cli::Error::Input(
							"Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
								.into(),
						)
						.into())
					}
				},
				BenchmarkCmd::Machine(cmd) => runner.sync_run(|config| {
					cmd.run(&config, BIZINIKIWI_REFERENCE_HARDWARE.clone())
						.map_err(Error::BizinikiwiCli)
				}),
				// NOTE: this allows the Pezkuwi client to leniently implement
				// new benchmark commands.
				#[allow(unreachable_patterns)]
				_ => Err(Error::CommandNotImplemented),
			}
		},
		Some(Subcommand::Key(cmd)) => Ok(cmd.run(&cli)?),
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			Ok(runner.sync_run(|config| cmd.run::<pezkuwi_service::Block>(&config))?)
		},
	}?;

	#[cfg(feature = "pyroscope")]
	if let Some(pyroscope_agent) = pyroscope_agent_maybe.take() {
		let agent = pyroscope_agent.stop()?;
		agent.shutdown();
	}
	Ok(())
}
