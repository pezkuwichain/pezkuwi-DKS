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

//! Teyrchain specific wrapper for the AuRa import queue.

use codec::Codec;
use pezcumulus_client_consensus_common::TeyrchainBlockImportMarker;
use pezsc_client_api::{backend::AuxStore, BlockOf, UsageProvider};
use pezsc_consensus::{import_queue::DefaultImportQueue, BlockImport};
use pezsc_consensus_aura::{AuraVerifier, CompatibilityMode};
use pezsc_consensus_slots::InherentDataProviderExt;
use pezsc_telemetry::TelemetryHandle;
use pezsp_api::{ApiExt, ProvideRuntimeApi};
use pezsp_block_builder::BlockBuilder as BlockBuilderApi;
use pezsp_blockchain::{HeaderBackend, HeaderMetadata};
use pezsp_consensus::Error as ConsensusError;
use pezsp_consensus_aura::AuraApi;
use pezsp_core::crypto::Pair;
use pezsp_inherents::CreateInherentDataProviders;
use pezsp_runtime::traits::Block as BlockT;
use prometheus_endpoint::Registry;
use std::{fmt::Debug, sync::Arc};

/// Parameters for [`import_queue`].
pub struct ImportQueueParams<'a, I, C, CIDP, S> {
	/// The block import to use.
	pub block_import: I,
	/// The client to interact with the chain.
	pub client: Arc<C>,
	/// The inherent data providers, to create the inherent data.
	pub create_inherent_data_providers: CIDP,
	/// The spawner to spawn background tasks.
	pub spawner: &'a S,
	/// The prometheus registry.
	pub registry: Option<&'a Registry>,
	/// The telemetry handle.
	pub telemetry: Option<TelemetryHandle>,
}

/// Start an import queue for the Aura consensus algorithm.
pub fn import_queue<P, Block, I, C, S, CIDP>(
	ImportQueueParams {
		block_import,
		client,
		create_inherent_data_providers,
		spawner,
		registry,
		telemetry,
	}: ImportQueueParams<'_, I, C, CIDP, S>,
) -> Result<DefaultImportQueue<Block>, pezsp_consensus::Error>
where
	Block: BlockT,
	C::Api: BlockBuilderApi<Block> + AuraApi<Block, P::Public> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>
		+ HeaderMetadata<Block, Error = pezsp_blockchain::Error>,
	I: BlockImport<Block, Error = ConsensusError>
		+ TeyrchainBlockImportMarker
		+ Send
		+ Sync
		+ 'static,
	P: Pair + 'static,
	P::Public: Debug + Codec,
	P::Signature: Codec,
	S: pezsp_core::traits::SpawnEssentialNamed,
	CIDP: CreateInherentDataProviders<Block, ()> + Sync + Send + 'static,
	CIDP::InherentDataProviders: InherentDataProviderExt + Send + Sync,
{
	pezsc_consensus_aura::import_queue::<P, _, _, _, _, _>(
		pezsc_consensus_aura::ImportQueueParams {
			block_import,
			justification_import: None,
			client,
			create_inherent_data_providers,
			spawner,
			registry,
			check_for_equivocation: pezsc_consensus_aura::CheckForEquivocation::No,
			telemetry,
			compatibility_mode: CompatibilityMode::None,
		},
	)
}

/// Parameters of [`build_verifier`].
pub struct BuildVerifierParams<C, CIDP> {
	/// The client to interact with the chain.
	pub client: Arc<C>,
	/// The inherent data providers, to create the inherent data.
	pub create_inherent_data_providers: CIDP,
	/// The telemetry handle.
	pub telemetry: Option<TelemetryHandle>,
}

/// Build the [`AuraVerifier`].
pub fn build_verifier<P: Pair, C, CIDP, B: BlockT>(
	BuildVerifierParams { client, create_inherent_data_providers, telemetry }: BuildVerifierParams<
		C,
		CIDP,
	>,
) -> AuraVerifier<C, P, CIDP, B> {
	pezsc_consensus_aura::build_verifier(pezsc_consensus_aura::BuildVerifierParams {
		client,
		create_inherent_data_providers,
		telemetry,
		check_for_equivocation: pezsc_consensus_aura::CheckForEquivocation::No,
		compatibility_mode: CompatibilityMode::None,
	})
}
