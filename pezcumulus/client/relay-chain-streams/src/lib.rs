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

//! Common utilities for transforming relay chain streams.

use std::sync::Arc;

use futures::{Stream, StreamExt};
use pezcumulus_relay_chain_interface::{RelayChainInterface, RelayChainResult};
use pezkuwi_node_subsystem::messages::RuntimeApiRequest;
use pezkuwi_primitives::{
	CommittedCandidateReceiptV2 as CommittedCandidateReceipt, Hash as PHash, Id as ParaId,
	OccupiedCoreAssumption, SessionIndex,
};
use pezsp_api::RuntimeApiInfo;
use pezsp_consensus::SyncOracle;

const LOG_TARGET: &str = "pezcumulus-relay-chain-streams";

pub type RelayHeader = pezkuwi_primitives::Header;

/// Returns a stream over pending candidates for the teyrchain corresponding to `para_id`.
pub async fn pending_candidates(
	relay_chain_client: impl RelayChainInterface + Clone,
	para_id: ParaId,
	sync_service: Arc<dyn SyncOracle + Sync + Send>,
) -> RelayChainResult<impl Stream<Item = (Vec<CommittedCandidateReceipt>, SessionIndex, RelayHeader)>>
{
	let import_notification_stream = relay_chain_client.import_notification_stream().await?;

	let filtered_stream = import_notification_stream.filter_map(move |n| {
		let client = relay_chain_client.clone();
		let sync_oracle = sync_service.clone();
		async move {
			let hash = n.hash();
			if sync_oracle.is_major_syncing() {
				tracing::debug!(
					target: LOG_TARGET,
					relay_hash = ?hash,
					"Skipping candidate due to sync.",
				);
				return None;
			}

			let runtime_api_version = client
				.version(hash)
				.await
				.map_err(|e| {
					tracing::error!(
						target: LOG_TARGET,
						error = ?e,
						"Failed to fetch relay chain runtime version.",
					)
				})
				.ok()?;
			let teyrchain_host_runtime_api_version =
				runtime_api_version
					.api_version(
						&<dyn pezkuwi_primitives::runtime_api::TeyrchainHost<
							pezkuwi_primitives::Block,
						>>::ID,
					)
					.unwrap_or_default();

			// If the relay chain runtime does not support the new runtime API, fallback to the
			// deprecated one.
			let pending_availability_result = if teyrchain_host_runtime_api_version
				< RuntimeApiRequest::CANDIDATES_PENDING_AVAILABILITY_RUNTIME_REQUIREMENT
			{
				#[allow(deprecated)]
				client
					.candidate_pending_availability(hash, para_id)
					.await
					.map_err(|e| {
						tracing::error!(
							target: LOG_TARGET,
							error = ?e,
							"Failed to fetch pending candidates.",
						)
					})
					.map(|candidate| candidate.into_iter().collect::<Vec<_>>())
			} else {
				client.candidates_pending_availability(hash, para_id).await.map_err(|e| {
					tracing::error!(
						target: LOG_TARGET,
						error = ?e,
						"Failed to fetch pending candidates.",
					)
				})
			};

			let session_index_result = client.session_index_for_child(hash).await.map_err(|e| {
				tracing::error!(
					target: LOG_TARGET,
					error = ?e,
					"Failed to fetch session index.",
				)
			});

			if let Ok(candidates) = pending_availability_result {
				session_index_result.map(|session_index| (candidates, session_index, n)).ok()
			} else {
				None
			}
		}
	});
	Ok(filtered_stream)
}

/// Returns a stream that will yield best heads for the given `para_id`.
pub async fn new_best_heads(
	relay_chain: impl RelayChainInterface + Clone,
	para_id: ParaId,
) -> RelayChainResult<impl Stream<Item = Vec<u8>>> {
	let new_best_notification_stream =
		relay_chain.new_best_notification_stream().await?.filter_map(move |n| {
			let relay_chain = relay_chain.clone();
			async move { teyrchain_head_at(&relay_chain, n.hash(), para_id).await.ok().flatten() }
		});

	Ok(new_best_notification_stream)
}

/// Returns a stream that will yield finalized heads for the given `para_id`.
pub async fn finalized_heads(
	relay_chain: impl RelayChainInterface + Clone,
	para_id: ParaId,
) -> RelayChainResult<impl Stream<Item = (Vec<u8>, RelayHeader)>> {
	let finality_notification_stream =
		relay_chain.finality_notification_stream().await?.filter_map(move |n| {
			let relay_chain = relay_chain.clone();
			async move {
				teyrchain_head_at(&relay_chain, n.hash(), para_id)
					.await
					.ok()
					.flatten()
					.map(|h| (h, n))
			}
		});

	Ok(finality_notification_stream)
}

/// Returns head of the teyrchain at the given relay chain block.
async fn teyrchain_head_at(
	relay_chain: &impl RelayChainInterface,
	at: PHash,
	para_id: ParaId,
) -> RelayChainResult<Option<Vec<u8>>> {
	relay_chain
		.persisted_validation_data(at, para_id, OccupiedCoreAssumption::TimedOut)
		.await
		.map(|s| s.map(|s| s.parent_head.0))
}
