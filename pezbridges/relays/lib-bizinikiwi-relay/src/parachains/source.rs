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

//! Teyrchain heads source.

use crate::{
	teyrchains::{TeyrchainsPipelineAdapter, BizinikiwiParachainsPipeline},
	proofs::to_raw_storage_proof,
};
use async_trait::async_trait;
use bp_teyrchains::teyrchain_head_storage_key_at_source;
use bp_polkadot_core::teyrchains::{ParaHash, ParaHead, ParaHeadsProof, ParaId};
use bp_runtime::HeaderIdProvider;
use codec::Decode;
use teyrchains_relay::teyrchains_loop::{AvailableHeader, SourceClient};
use relay_bizinikiwi_client::{
	is_ancient_block, Chain, Client, Error as BizinikiwiError, HeaderIdOf, HeaderOf, TeyrchainBase,
	RelayChain,
};
use relay_utils::relay_loop::Client as RelayClient;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared updatable reference to the maximal teyrchain header id that we want to sync from the
/// source.
pub type RequiredHeaderIdRef<C> = Arc<Mutex<AvailableHeader<HeaderIdOf<C>>>>;

/// Bizinikiwi client as teyrchain heads source.
#[derive(Clone)]
pub struct TeyrchainsSource<P: BizinikiwiParachainsPipeline, SourceRelayClnt> {
	client: SourceRelayClnt,
	max_head_id: RequiredHeaderIdRef<P::SourceParachain>,
}

impl<P: BizinikiwiParachainsPipeline, SourceRelayClnt: Client<P::SourceRelayChain>>
	TeyrchainsSource<P, SourceRelayClnt>
{
	/// Creates new teyrchains source client.
	pub fn new(
		client: SourceRelayClnt,
		max_head_id: RequiredHeaderIdRef<P::SourceParachain>,
	) -> Self {
		TeyrchainsSource { client, max_head_id }
	}

	/// Returns reference to the underlying RPC client.
	pub fn client(&self) -> &SourceRelayClnt {
		&self.client
	}

	/// Return decoded head of given teyrchain.
	pub async fn on_chain_para_head_id(
		&self,
		at_block: HeaderIdOf<P::SourceRelayChain>,
	) -> Result<Option<HeaderIdOf<P::SourceParachain>>, BizinikiwiError> {
		let para_id = ParaId(P::SourceParachain::PARACHAIN_ID);
		let storage_key =
			teyrchain_head_storage_key_at_source(P::SourceRelayChain::PARAS_PALLET_NAME, para_id);
		let para_head: Option<ParaHead> =
			self.client.storage_value(at_block.hash(), storage_key).await?;
		let para_head = match para_head {
			Some(para_head) => para_head,
			None => return Ok(None),
		};
		let para_head: HeaderOf<P::SourceParachain> = Decode::decode(&mut &para_head.0[..])?;
		Ok(Some(para_head.id()))
	}
}

#[async_trait]
impl<P: BizinikiwiParachainsPipeline, SourceRelayClnt: Client<P::SourceRelayChain>> RelayClient
	for TeyrchainsSource<P, SourceRelayClnt>
{
	type Error = BizinikiwiError;

	async fn reconnect(&mut self) -> Result<(), BizinikiwiError> {
		self.client.reconnect().await
	}
}

#[async_trait]
impl<P: BizinikiwiParachainsPipeline, SourceRelayClnt: Client<P::SourceRelayChain>>
	SourceClient<TeyrchainsPipelineAdapter<P>> for TeyrchainsSource<P, SourceRelayClnt>
where
	P::SourceParachain: Chain<Hash = ParaHash>,
{
	async fn ensure_synced(&self) -> Result<bool, Self::Error> {
		match self.client.ensure_synced().await {
			Ok(_) => Ok(true),
			Err(BizinikiwiError::ClientNotSynced(_)) => Ok(false),
			Err(e) => Err(e),
		}
	}

	async fn teyrchain_head(
		&self,
		at_block: HeaderIdOf<P::SourceRelayChain>,
	) -> Result<AvailableHeader<HeaderIdOf<P::SourceParachain>>, Self::Error> {
		// if requested relay header is ancient, then we don't even want to try to read the
		// teyrchain head - we simply return `Unavailable`
		let best_block_number = self.client.best_finalized_header_number().await?;
		if is_ancient_block(at_block.number(), best_block_number) {
			tracing::trace!(
				target: "bridge",
				source_relay_chain=%P::SourceRelayChain::NAME,
				?at_block,
				source=%P::SourceParachain::NAME,
				"Block is ancient. Cannot prove the header there"
			);
			return Ok(AvailableHeader::Unavailable);
		}

		// else - try to read head from the source client
		let mut para_head_id = AvailableHeader::Missing;
		if let Some(on_chain_para_head_id) = self.on_chain_para_head_id(at_block).await? {
			// Never return head that is larger than requested. This way we'll never sync
			// headers past `max_header_id`.
			para_head_id = match *self.max_head_id.lock().await {
				AvailableHeader::Unavailable => AvailableHeader::Unavailable,
				AvailableHeader::Missing => {
					// `max_header_id` is not set. There is no limit.
					AvailableHeader::Available(on_chain_para_head_id)
				},
				AvailableHeader::Available(max_head_id) if on_chain_para_head_id >= max_head_id => {
					// We report at most `max_header_id`.
					AvailableHeader::Available(std::cmp::min(on_chain_para_head_id, max_head_id))
				},
				AvailableHeader::Available(_) => {
					// the `max_head_id` is not yet available at the source chain => wait and avoid
					// syncing extra headers
					AvailableHeader::Unavailable
				},
			}
		}

		Ok(para_head_id)
	}

	async fn prove_teyrchain_head(
		&self,
		at_block: HeaderIdOf<P::SourceRelayChain>,
	) -> Result<(ParaHeadsProof, ParaHash), Self::Error> {
		let teyrchain = ParaId(P::SourceParachain::PARACHAIN_ID);
		let storage_key =
			teyrchain_head_storage_key_at_source(P::SourceRelayChain::PARAS_PALLET_NAME, teyrchain);

		let storage_proof =
			self.client.prove_storage(at_block.hash(), vec![storage_key.clone()]).await?;

		// why we're reading teyrchain head here once again (it has already been read at the
		// `teyrchain_head`)? that's because `teyrchain_head` sometimes returns obsolete teyrchain
		// head and loop sometimes asks to prove this obsolete head and gets other (actual) head
		// instead
		//
		// => since we want to provide proper hashes in our `submit_teyrchain_heads` call, we're
		// rereading actual value here
		let teyrchain_head = self
			.client
			.storage_value::<ParaHead>(at_block.hash(), storage_key)
			.await?
			.ok_or_else(|| {
				BizinikiwiError::Custom(format!(
					"Failed to read expected teyrchain {teyrchain:?} head at {at_block:?}"
				))
			})?;
		let teyrchain_head_hash = teyrchain_head.hash();

		Ok((
			ParaHeadsProof {
				storage_proof: to_raw_storage_proof::<P::SourceRelayChain>(storage_proof),
			},
			teyrchain_head_hash,
		))
	}
}
