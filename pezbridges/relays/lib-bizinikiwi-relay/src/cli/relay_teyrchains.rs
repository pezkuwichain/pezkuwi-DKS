// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! Primitives for exposing the teyrchains finality relaying functionality in the CLI.

use async_std::sync::Mutex;
use async_trait::async_trait;
use clap::Parser;
use pezbp_pezkuwi_core::BlockNumber as RelayBlockNumber;
use pezbp_runtime::HeaderIdProvider;
use relay_bizinikiwi_client::{Client, Teyrchain};
use relay_utils::metrics::{GlobalMetrics, StandaloneMetric};
use std::sync::Arc;
use teyrchains_relay::teyrchains_loop::{AvailableHeader, SourceClient, TargetClient};

use crate::{
	cli::{
		bridge::{CliBridgeBase, TeyrchainToRelayHeadersCliBridge},
		chain_schema::*,
		DefaultClient, PrometheusParams,
	},
	finality::BizinikiwiFinalitySyncPipeline,
	teyrchains::{source::TeyrchainsSource, target::TeyrchainsTarget, TeyrchainsPipelineAdapter},
	TransactionParams,
};

/// Teyrchains heads relaying params.
#[derive(Parser)]
pub struct RelayTeyrchainsParams {
	#[command(flatten)]
	source: SourceConnectionParams,
	#[command(flatten)]
	target: TargetConnectionParams,
	#[command(flatten)]
	target_sign: TargetSigningParams,
	/// If passed, only free headers (those, available at "free" relay chain headers)
	/// are relayed.
	#[arg(long)]
	only_free_headers: bool,
	#[command(flatten)]
	prometheus_params: PrometheusParams,
}

/// Single teyrchains head relaying params.
#[derive(Parser)]
pub struct RelayTeyrchainHeadParams {
	#[command(flatten)]
	source: SourceConnectionParams,
	#[command(flatten)]
	target: TargetConnectionParams,
	#[command(flatten)]
	target_sign: TargetSigningParams,
	/// Prove teyrchain head at that relay block number. This relay header must be previously
	/// proved to the target chain.
	#[arg(long)]
	at_relay_block: RelayBlockNumber,
}

/// Trait used for relaying teyrchains finality between 2 chains.
#[async_trait]
pub trait TeyrchainsRelayer: TeyrchainToRelayHeadersCliBridge
where
	TeyrchainsSource<Self::TeyrchainFinality, DefaultClient<Self::SourceRelay>>:
		SourceClient<TeyrchainsPipelineAdapter<Self::TeyrchainFinality>>,
	TeyrchainsTarget<
		Self::TeyrchainFinality,
		DefaultClient<Self::SourceRelay>,
		DefaultClient<Self::Target>,
	>: TargetClient<TeyrchainsPipelineAdapter<Self::TeyrchainFinality>>,
	<Self as CliBridgeBase>::Source: Teyrchain,
{
	/// Start relaying teyrchains finality.
	async fn relay_teyrchains(data: RelayTeyrchainsParams) -> anyhow::Result<()> {
		let source_chain_client = data.source.into_client::<Self::SourceRelay>().await?;
		let source_client = TeyrchainsSource::<Self::TeyrchainFinality, _>::new(
			source_chain_client.clone(),
			Arc::new(Mutex::new(AvailableHeader::Missing)),
		);

		let target_transaction_params = TransactionParams {
			signer: data.target_sign.to_keypair::<Self::Target>()?,
			mortality: data.target_sign.target_transactions_mortality,
		};
		let target_chain_client = data.target.into_client::<Self::Target>().await?;
		let target_client = TeyrchainsTarget::<Self::TeyrchainFinality, _, _>::new(
			source_chain_client,
			target_chain_client,
			target_transaction_params,
		);

		let metrics_params: relay_utils::metrics::MetricsParams =
			data.prometheus_params.into_metrics_params()?;
		GlobalMetrics::new()?.register_and_spawn(&metrics_params.registry)?;

		Self::RelayFinality::start_relay_guards(
			target_client.target_client(),
			target_client.target_client().can_start_version_guard(),
		)
		.await?;

		teyrchains_relay::teyrchains_loop::run(
			source_client,
			target_client,
			metrics_params,
			data.only_free_headers,
			futures::future::pending(),
		)
		.await
		.map_err(|e| anyhow::format_err!("{}", e))
	}

	/// Relay single teyrchain head. No checks are made to ensure that transaction will succeed.
	async fn relay_teyrchain_head(data: RelayTeyrchainHeadParams) -> anyhow::Result<()> {
		let source_chain_client = data.source.into_client::<Self::SourceRelay>().await?;
		let at_relay_block = source_chain_client
			.header_by_number(data.at_relay_block)
			.await
			.map_err(|e| anyhow::format_err!("{}", e))?
			.id();

		let source_client = TeyrchainsSource::<Self::TeyrchainFinality, _>::new(
			source_chain_client.clone(),
			Arc::new(Mutex::new(AvailableHeader::Missing)),
		);

		let target_transaction_params = TransactionParams {
			signer: data.target_sign.to_keypair::<Self::Target>()?,
			mortality: data.target_sign.target_transactions_mortality,
		};
		let target_chain_client = data.target.into_client::<Self::Target>().await?;
		let target_client = TeyrchainsTarget::<Self::TeyrchainFinality, _, _>::new(
			source_chain_client,
			target_chain_client,
			target_transaction_params,
		);

		teyrchains_relay::teyrchains_loop::relay_single_head(
			source_client,
			target_client,
			at_relay_block,
		)
		.await
		.map_err(|_| anyhow::format_err!("The command has failed"))
	}
}
