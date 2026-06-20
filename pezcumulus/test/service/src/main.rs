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

mod cli;

use std::sync::Arc;

use cli::{AuthoringPolicy, RelayChainCli, Subcommand, TestCollatorCli};
use pezcumulus_primitives_core::relay_chain::CollatorPair;
use pezcumulus_test_service::{new_partial, AnnounceBlockFn};
use pezsc_cli::{BizinikiwiCli, CliConfiguration};
use pezsp_core::Pair;

pub fn wrap_announce_block() -> Box<dyn FnOnce(AnnounceBlockFn) -> AnnounceBlockFn> {
	tracing::info!("Block announcements disabled.");
	Box::new(|_| {
		// Never announce any block
		Arc::new(|_, _| {})
	})
}

fn main() -> Result<(), pezsc_cli::Error> {
	let cli = TestCollatorCli::from_args();

	match &cli.subcommand {
		#[allow(deprecated)]
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},

		Some(Subcommand::ExportChainSpec(cmd)) => {
			let chain_spec = cli.load_spec(&cmd.chain)?;
			cmd.run(chain_spec)
		},

		Some(Subcommand::ExportGenesisHead(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|mut config| {
				let partial = new_partial(&mut config, false)?;
				cmd.run(partial.client)
			})
		},
		Some(Subcommand::ExportGenesisWasm(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(&*config.chain_spec))
		},
		None => {
			let log_filters = cli.run.normalize().log_filters();
			let mut builder = pezsc_cli::LoggerBuilder::new(log_filters.unwrap_or_default());
			builder.with_colors(false);
			let _ = builder.init();

			let collator_options = cli.run.collator_options();
			let tokio_runtime = pezsc_cli::build_runtime()?;
			let tokio_handle = tokio_runtime.handle();
			let teyrchain_config = cli
				.run
				.normalize()
				.create_configuration(&cli, tokio_handle.clone())
				.expect("Should be able to generate config");

			let relay_chain_cli = RelayChainCli::new(
				&teyrchain_config,
				[RelayChainCli::executable_name()].iter().chain(cli.relaychain_args.iter()),
			);
			let tokio_handle = teyrchain_config.tokio_handle.clone();
			let relay_chain_config = BizinikiwiCli::create_configuration(
				&relay_chain_cli,
				&relay_chain_cli,
				tokio_handle,
			)
			.map_err(|err| format!("Relay chain argument error: {}", err))?;

			tracing::info!(
				"Is collating: {}",
				if teyrchain_config.role.is_authority() { "yes" } else { "no" }
			);
			if cli.fail_pov_recovery {
				tracing::info!("PoV recovery failure enabled");
			}

			let collator_key =
				teyrchain_config.role.is_authority().then(|| CollatorPair::generate().0);

			let use_slot_based_collator = cli.authoring == AuthoringPolicy::SlotBased;
			let (mut task_manager, _, _, _, _, _) = tokio_runtime
				.block_on(async move {
					match relay_chain_config.network.network_backend {
						pezsc_network::config::NetworkBackendType::Libp2p => {
							pezcumulus_test_service::start_node_impl::<
								_,
								pezsc_network::NetworkWorker<_, _>,
							>(
								teyrchain_config,
								collator_key,
								relay_chain_config,
								cli.disable_block_announcements.then(wrap_announce_block),
								cli.fail_pov_recovery,
								|_| Ok(jsonrpsee::RpcModule::new(())),
								collator_options,
								true,
								use_slot_based_collator,
							)
							.await
						},
						pezsc_network::config::NetworkBackendType::Litep2p => {
							pezcumulus_test_service::start_node_impl::<
								_,
								pezsc_network::Litep2pNetworkBackend,
							>(
								teyrchain_config,
								collator_key,
								relay_chain_config,
								cli.disable_block_announcements.then(wrap_announce_block),
								cli.fail_pov_recovery,
								|_| Ok(jsonrpsee::RpcModule::new(())),
								collator_options,
								true,
								use_slot_based_collator,
							)
							.await
						},
					}
				})
				.expect("could not create Pezcumulus test service");

			tokio_runtime
				.block_on(task_manager.future())
				.expect("Could not run service to completion");
			Ok(())
		},
	}
}
