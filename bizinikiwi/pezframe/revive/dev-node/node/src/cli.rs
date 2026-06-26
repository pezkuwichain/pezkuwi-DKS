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

use pezkuwi_sdk::{pezsc_cli::RunCmd, *};

#[derive(Debug, Clone, Copy)]
pub enum Consensus {
	ManualSeal(u64),
	InstantSeal,
	None,
}

impl std::str::FromStr for Consensus {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(if s == "instant-seal" {
			Consensus::InstantSeal
		} else if let Some(block_time) = s.strip_prefix("manual-seal-") {
			Consensus::ManualSeal(block_time.parse().map_err(|_| "invalid block time")?)
		} else if s.to_lowercase() == "none" {
			Consensus::None
		} else {
			return Err("incorrect consensus identifier".into());
		})
	}
}

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(long, default_value = "instant-seal")]
	pub consensus: Consensus,

	#[clap(flatten)]
	pub run: RunCmd,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Key management cli utilities
	#[command(subcommand)]
	Key(pezsc_cli::KeySubcommand),

	/// Build a chain specification.
	BuildSpec(pezsc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(pezsc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(pezsc_cli::ExportBlocksCmd),

	/// Export the chain specification.
	ExportChainSpec(pezsc_cli::ExportChainSpecCmd),

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
