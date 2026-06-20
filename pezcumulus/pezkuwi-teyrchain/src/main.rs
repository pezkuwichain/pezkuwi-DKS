// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

//! Pezkuwi teyrchain node.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod chain_spec;

use pezkuwi_omni_node_lib::{run, CliConfig as CliConfigT, RunConfig, NODE_VERSION};

struct CliConfig;

impl CliConfigT for CliConfig {
	fn impl_version() -> String {
		let commit_hash = env!("BIZINIKIWI_CLI_COMMIT_HASH");
		format!("{}-{commit_hash}", NODE_VERSION)
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/pezkuwichain/pezkuwi-sdk/issues/new".into()
	}

	fn copyright_start_year() -> u16 {
		2017
	}
}

fn main() -> color_eyre::eyre::Result<()> {
	color_eyre::install()?;
	let config = RunConfig::new(
		Box::new(chain_spec::RuntimeResolver),
		Box::new(chain_spec::ChainSpecLoader),
	);
	// This enables pezkuwi-teyrchain to support additional subcommands like `export-chain-spec`.
	// To add more, extend the `ExtraSubcommands` enum in
	// `pezcumulus/pezkuwi-omni-node/lib/src/extra_subcommand` and handle them in
	// `ExtraSubcommands::handle`.
	Ok(run::<CliConfig>(config)?)
}
