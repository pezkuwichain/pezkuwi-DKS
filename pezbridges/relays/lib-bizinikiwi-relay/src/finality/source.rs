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

//! Default generic implementation of finality source for basic Bizinikiwi client.

use crate::{
	finality::{BizinikiwiFinalitySyncPipeline, FinalitySyncPipelineAdapter},
	finality_base::{
		engine::Engine, finality_proofs, BizinikiwiFinalityProof, BizinikiwiFinalityProofsStream,
	},
};

use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use codec::Decode;
use futures::{
	select,
	stream::{try_unfold, Stream, StreamExt, TryStreamExt},
};
use num_traits::One;
use pez_finality_relay::{SourceClient, SourceClientBase};
use pezbp_header_pez_chain::FinalityProof;
use relay_bizinikiwi_client::{BlockNumberOf, BlockWithJustification, Client, Error, HeaderOf};
use relay_utils::{relay_loop::Client as RelayClient, UniqueSaturatedInto};

/// Shared updatable reference to the maximal header number that we want to sync from the source.
pub type RequiredHeaderNumberRef<C> = Arc<Mutex<<C as pezbp_runtime::Chain>::BlockNumber>>;

/// Bizinikiwi node as finality source.
pub struct BizinikiwiFinalitySource<P: BizinikiwiFinalitySyncPipeline, SourceClnt> {
	client: SourceClnt,
	maximal_header_number: Option<RequiredHeaderNumberRef<P::SourceChain>>,
}

impl<P: BizinikiwiFinalitySyncPipeline, SourceClnt: Client<P::SourceChain>>
	BizinikiwiFinalitySource<P, SourceClnt>
{
	/// Create new headers source using given client.
	pub fn new(
		client: SourceClnt,
		maximal_header_number: Option<RequiredHeaderNumberRef<P::SourceChain>>,
	) -> Self {
		BizinikiwiFinalitySource { client, maximal_header_number }
	}

	/// Returns reference to the underlying RPC client.
	pub fn client(&self) -> &SourceClnt {
		&self.client
	}

	/// Returns best finalized block number.
	pub async fn on_chain_best_finalized_block_number(
		&self,
	) -> Result<BlockNumberOf<P::SourceChain>, Error> {
		// we **CAN** continue to relay finality proofs if source node is out of sync, because
		// target node may be missing proofs that are already available at the source
		self.client.best_finalized_header_number().await
	}

	/// Return header and its justification of the given block or its descendant that
	/// has a GRANDPA justification.
	///
	/// This method is optimized for cases when `block_number` is close to the best finalized
	/// chain block.
	pub async fn prove_block_finality(
		&self,
		block_number: BlockNumberOf<P::SourceChain>,
	) -> Result<
		(relay_bizinikiwi_client::SyncHeader<HeaderOf<P::SourceChain>>, BizinikiwiFinalityProof<P>),
		Error,
	> {
		// first, subscribe to proofs
		let next_persistent_proof =
			self.persistent_proofs_stream(block_number + One::one()).await?.fuse();
		let next_ephemeral_proof = self.ephemeral_proofs_stream(block_number).await?.fuse();

		// in perfect world we'll need to return justfication for the requested `block_number`
		let (header, maybe_proof) = self.header_and_finality_proof(block_number).await?;
		if let Some(proof) = maybe_proof {
			return Ok((header, proof));
		}

		// otherwise we don't care which header to return, so let's select first
		futures::pin_mut!(next_persistent_proof, next_ephemeral_proof);
		loop {
			select! {
				maybe_header_and_proof = next_persistent_proof.next() => match maybe_header_and_proof {
					Some(header_and_proof) => return header_and_proof,
					None => continue,
				},
				maybe_header_and_proof = next_ephemeral_proof.next() => match maybe_header_and_proof {
					Some(header_and_proof) => return header_and_proof,
					None => continue,
				},
				complete => return Err(Error::FinalityProofNotFound(block_number.unique_saturated_into()))
			}
		}
	}

	/// Returns stream of headers and their persistent proofs, starting from given block.
	async fn persistent_proofs_stream(
		&self,
		block_number: BlockNumberOf<P::SourceChain>,
	) -> Result<
		impl Stream<
			Item = Result<
				(
					relay_bizinikiwi_client::SyncHeader<HeaderOf<P::SourceChain>>,
					BizinikiwiFinalityProof<P>,
				),
				Error,
			>,
		>,
		Error,
	> {
		let client = self.client.clone();
		let best_finalized_block_number = client.best_finalized_header_number().await?;
		Ok(try_unfold((client, block_number), move |(client, current_block_number)| async move {
			// if we've passed the `best_finalized_block_number`, we no longer need persistent
			// justifications
			if current_block_number > best_finalized_block_number {
				return Ok(None);
			}

			let (header, maybe_proof) =
				header_and_finality_proof::<P>(&client, current_block_number).await?;
			let next_block_number = current_block_number + One::one();
			let next_state = (client, next_block_number);

			Ok(Some((maybe_proof.map(|proof| (header, proof)), next_state)))
		})
		.try_filter_map(|maybe_result| async { Ok(maybe_result) }))
	}

	/// Returns stream of headers and their ephemeral proofs, starting from given block.
	async fn ephemeral_proofs_stream(
		&self,
		block_number: BlockNumberOf<P::SourceChain>,
	) -> Result<
		impl Stream<
			Item = Result<
				(
					relay_bizinikiwi_client::SyncHeader<HeaderOf<P::SourceChain>>,
					BizinikiwiFinalityProof<P>,
				),
				Error,
			>,
		>,
		Error,
	> {
		let client = self.client.clone();
		Ok(self.finality_proofs().await?.map(Ok).try_filter_map(move |proof| {
			let client = client.clone();
			async move {
				if proof.target_header_number() < block_number {
					return Ok(None);
				}

				let header = client.header_by_number(proof.target_header_number()).await?;
				Ok(Some((header.into(), proof)))
			}
		}))
	}
}

impl<P: BizinikiwiFinalitySyncPipeline, SourceClnt: Clone> Clone
	for BizinikiwiFinalitySource<P, SourceClnt>
{
	fn clone(&self) -> Self {
		BizinikiwiFinalitySource {
			client: self.client.clone(),
			maximal_header_number: self.maximal_header_number.clone(),
		}
	}
}

#[async_trait]
impl<P: BizinikiwiFinalitySyncPipeline, SourceClnt: Client<P::SourceChain>> RelayClient
	for BizinikiwiFinalitySource<P, SourceClnt>
{
	type Error = Error;

	async fn reconnect(&mut self) -> Result<(), Error> {
		self.client.reconnect().await
	}
}

#[async_trait]
impl<P: BizinikiwiFinalitySyncPipeline, SourceClnt: Client<P::SourceChain>>
	SourceClientBase<FinalitySyncPipelineAdapter<P>> for BizinikiwiFinalitySource<P, SourceClnt>
{
	type FinalityProofsStream = BizinikiwiFinalityProofsStream<P>;

	async fn finality_proofs(&self) -> Result<Self::FinalityProofsStream, Error> {
		finality_proofs::<P>(&self.client).await
	}
}

#[async_trait]
impl<P: BizinikiwiFinalitySyncPipeline, SourceClnt: Client<P::SourceChain>>
	SourceClient<FinalitySyncPipelineAdapter<P>> for BizinikiwiFinalitySource<P, SourceClnt>
{
	async fn best_finalized_block_number(&self) -> Result<BlockNumberOf<P::SourceChain>, Error> {
		let mut finalized_header_number = self.on_chain_best_finalized_block_number().await?;
		// never return block number larger than requested. This way we'll never sync headers
		// past `maximal_header_number`
		if let Some(ref maximal_header_number) = self.maximal_header_number {
			let maximal_header_number = *maximal_header_number.lock().await;
			if finalized_header_number > maximal_header_number {
				finalized_header_number = maximal_header_number;
			}
		}
		Ok(finalized_header_number)
	}

	async fn header_and_finality_proof(
		&self,
		number: BlockNumberOf<P::SourceChain>,
	) -> Result<
		(
			relay_bizinikiwi_client::SyncHeader<HeaderOf<P::SourceChain>>,
			Option<BizinikiwiFinalityProof<P>>,
		),
		Error,
	> {
		header_and_finality_proof::<P>(&self.client, number).await
	}
}

async fn header_and_finality_proof<P: BizinikiwiFinalitySyncPipeline>(
	client: &impl Client<P::SourceChain>,
	number: BlockNumberOf<P::SourceChain>,
) -> Result<
	(
		relay_bizinikiwi_client::SyncHeader<HeaderOf<P::SourceChain>>,
		Option<BizinikiwiFinalityProof<P>>,
	),
	Error,
> {
	let header_hash = client.header_hash_by_number(number).await?;
	let signed_block = client.block_by_hash(header_hash).await?;

	let justification = signed_block
		.justification(P::FinalityEngine::ID)
		.map(|raw_justification| {
			BizinikiwiFinalityProof::<P>::decode(&mut raw_justification.as_slice())
		})
		.transpose()
		.map_err(Error::ResponseParseFailed)?;

	Ok((signed_block.header().into(), justification))
}
