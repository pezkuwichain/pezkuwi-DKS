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

/// An overarching CLI command definition.
#[derive(Debug, clap::Parser)]
pub struct Cli {
	/// Possible subcommand with parameters.
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub run: pezsc_cli::RunCmd,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub mixnet_params: pezsc_cli::MixnetParams,

	/// Disable automatic hardware benchmarks.
	///
	/// By default these benchmarks are automatically ran at startup and measure
	/// the CPU speed, the memory bandwidth and the disk speed.
	///
	/// The results are then printed out in the logs, and also sent as part of
	/// telemetry, if telemetry is enabled.
	#[arg(long)]
	pub no_hardware_benchmarks: bool,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub storage_monitor: pezsc_storage_monitor::StorageMonitorParams,
}

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// The custom inspect subcommand for decoding blocks and extrinsics.
	#[command(
		name = "inspect",
		about = "Decode given block or extrinsic using current native runtime."
	)]
	Inspect(node_inspect::cli::InspectCmd),

	/// Sub-commands concerned with benchmarking.
	///
	/// The pezpallet benchmarking moved to the `pezpallet` sub-command.
	#[command(subcommand)]
	Benchmark(pezframe_benchmarking_cli::BenchmarkCmd),

	/// Key management cli utilities
	#[command(subcommand)]
	Key(pezsc_cli::KeySubcommand),

	/// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
	Verify(pezsc_cli::VerifyCmd),

	/// Generate a seed that provides a vanity address.
	Vanity(pezsc_cli::VanityCmd),

	/// Sign a message, with a given (secret) key.
	Sign(pezsc_cli::SignCmd),

	/// Build a chain specification.
	/// DEPRECATED: `build-spec` command will be removed after 1/04/2026. Use `export-chain-spec`
	/// command instead.
	#[deprecated(
		note = "build-spec command will be removed after 1/04/2026. Use export-chain-spec command instead"
	)]
	BuildSpec(pezsc_cli::BuildSpecCmd),

	/// Export the chain specification.
	ExportChainSpec(pezsc_cli::ExportChainSpecCmd),

	/// Validate blocks.
	CheckBlock(pezsc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(pezsc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(pezsc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(pezsc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(pezsc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(pezsc_cli::RevertCmd),

	/// Db meta columns information.
	ChainInfo(pezsc_cli::ChainInfoCmd),
}
