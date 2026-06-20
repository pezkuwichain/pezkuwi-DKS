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

//! Pezkuwi CLI library.

use clap::Parser;
use pezsc_cli::BizinikiwiCli;
use std::path::PathBuf;

/// Sub-commands supported by the collator.
#[derive(Debug, Parser)]
pub enum Subcommand {
	/// Export the genesis state of the teyrchain.
	#[command(name = "export-genesis-state")]
	ExportGenesisState(ExportGenesisHeadCommand),

	/// Export the genesis wasm of the teyrchain.
	#[command(name = "export-genesis-wasm")]
	ExportGenesisWasm(ExportGenesisWasmCommand),
}

/// Command for exporting the genesis head data of the teyrchain
#[derive(Debug, Parser)]
pub struct ExportGenesisHeadCommand {
	/// Output file name or stdout if unspecified.
	#[arg()]
	pub output: Option<PathBuf>,
}

/// Command for exporting the genesis wasm file.
#[derive(Debug, Parser)]
pub struct ExportGenesisWasmCommand {
	/// Output file name or stdout if unspecified.
	#[arg()]
	pub output: Option<PathBuf>,
}

#[allow(missing_docs)]
#[derive(Debug, Parser)]
#[group(skip)]
pub struct RunCmd {
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub base: pezsc_cli::RunCmd,

	/// Id of the teyrchain this collator collates for.
	#[arg(long)]
	pub teyrchain_id: Option<u32>,
}

#[allow(missing_docs)]
#[derive(Debug, Parser)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: RunCmd,
}

impl BizinikiwiCli for Cli {
	fn impl_name() -> String {
		"Parity Pezkuwi".into()
	}

	fn impl_version() -> String {
		"0.0.0".into()
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
		"adder-collator".into()
	}

	fn load_spec(
		&self,
		id: &str,
	) -> std::result::Result<Box<dyn pezsc_service::ChainSpec>, String> {
		let id = if id.is_empty() { "pezkuwichain" } else { id };
		Ok(match id {
			"pezkuwichain-staging" => {
				Box::new(pezkuwi_service::chain_spec::pezkuwichain_staging_testnet_config()?)
			},
			"pezkuwichain-local" => {
				Box::new(pezkuwi_service::chain_spec::pezkuwichain_local_testnet_config()?)
			},
			"pezkuwichain" => Box::new(pezkuwi_service::chain_spec::pezkuwichain_config()?),
			path => {
				let path = std::path::PathBuf::from(path);
				Box::new(pezkuwi_service::PezkuwichainChainSpec::from_json_file(path)?)
			},
		})
	}
}
