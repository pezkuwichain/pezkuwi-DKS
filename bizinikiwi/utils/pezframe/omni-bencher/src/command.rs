// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use clap::Parser;
use pezframe_benchmarking_cli::{BenchmarkCmd, OpaqueBlock};
use pezsc_cli::Result;
use pezsp_runtime::traits::BlakeTwo256;

/// # Pezkuwi Omni Benchmarking CLI
///
/// The Pezkuwi Omni benchmarker allows to benchmark the extrinsics of any Pezkuwi runtime. It is
/// meant to replace the current manual integration of the `benchmark pezpallet` into every
/// teyrchain node. This reduces duplicate code and makes maintenance for builders easier. The CLI
/// is currently only able to benchmark extrinsics. In the future it is planned to extend this to
/// some other areas.
///
/// General FRAME runtimes could also be used with this benchmarker, as long as they don't utilize
/// any host functions that are not part of the Pezkuwi host specification.
///
/// ## Installation
///
/// Directly via crates.io:
///
/// ```sh
/// cargo install frame-omni-bencher --profile=production
/// ```
///
/// from GitHub:
///
/// ```sh
/// cargo install --git https://github.com/pezkuwichain/pezkuwi-sdk frame-omni-bencher --profile=production
/// ```
///
/// or locally from the sources:
///
/// ```sh
/// cargo install --path bizinikiwi/utils/pezframe/omni-bencher --profile=production
/// ```
///
/// Check the installed version and print the docs:
///
/// ```sh
/// frame-omni-bencher --help
/// ```
///
/// ## Usage
///
/// First we need to ensure that there is a runtime available. As example we will build the Zagros
/// runtime:
///
/// ```sh
/// cargo build -p zagros-runtime --profile production --features runtime-benchmarks
/// ```
///
/// Now as an example, we benchmark the `balances` pezpallet:
///
/// ```sh
/// frame-omni-bencher v1 benchmark pezpallet \
///     --runtime target/release/wbuild/zagros-runtime/zagros-runtime.compact.compressed.wasm \
///     --pezpallet "pezpallet_balances" --extrinsic ""
/// ```
///
/// For the exact arguments of the `pezpallet` command, please refer to the `pezpallet` sub-module.
///
/// ## Backwards Compatibility
///
/// The exposed pezpallet sub-command is identical as the node-integrated CLI. The only difference
/// is that it needs to be prefixed with a `v1` to ensure drop-in compatibility.
#[derive(Parser, Debug)]
#[clap(author, version, about, verbatim_doc_comment)]
pub struct Command {
	#[command(subcommand)]
	sub: SubCommand,
}

/// Root-level subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum SubCommand {
	/// Compatibility syntax with the old benchmark runner.
	V1(V1Command),
	// NOTE: Here we can add new commands in a forward-compatible way. For example when
	// transforming the CLI from a monolithic design to a data driven pipeline, there could be
	// commands like `measure`, `analyze` and `render`.
}

/// A command that conforms to the legacy `benchmark` argument syntax.
#[derive(Parser, Debug)]
pub struct V1Command {
	#[command(subcommand)]
	sub: V1SubCommand,
}

/// The `v1 benchmark` subcommand.
#[derive(Debug, clap::Subcommand)]
pub enum V1SubCommand {
	Benchmark(V1BenchmarkCommand),
}

/// Subcommands for `v1 benchmark`.
#[derive(Parser, Debug)]
pub struct V1BenchmarkCommand {
	#[command(subcommand)]
	sub: BenchmarkCmd,
}

type HostFunctions = (
	pezsp_statement_store::runtime_api::HostFunctions,
	pezcumulus_primitives_proof_size_hostfunction::storage_proof_size::HostFunctions,
);

impl Command {
	pub fn run(self) -> Result<()> {
		match self.sub {
			SubCommand::V1(V1Command { sub }) => sub.run(),
		}
	}
}
impl V1SubCommand {
	pub fn run(self) -> Result<()> {
		match self {
			V1SubCommand::Benchmark(V1BenchmarkCommand { sub }) => match sub {
				BenchmarkCmd::Pezpallet(pezpallet) => {
					pezpallet.run_with_spec::<BlakeTwo256, HostFunctions>(None)
				},
				BenchmarkCmd::Overhead(overhead_cmd) =>
					overhead_cmd.run_with_default_builder_and_spec::<OpaqueBlock, HostFunctions>(None),
				_ =>
					return Err(
						"Only the `v1 benchmark pezpallet` and `v1 benchmark overhead` command is currently supported".into()
					),
			},
		}
	}
}
