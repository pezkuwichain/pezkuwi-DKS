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

//! The Pezcumulus [`ProposerInterface`] is an extension of the Bizinikiwi [`ProposerFactory`]
//! for creating new teyrchain blocks.
//!
//! This utility is designed to be composed within any collator consensus algorithm.

use async_trait::async_trait;
use pezcumulus_primitives_teyrchain_inherent::TeyrchainInherentData;
use pezsc_basic_authorship::{ProposeArgs, ProposerFactory};
use pezsc_block_builder::BlockBuilderApi;
use pezsc_transaction_pool_api::TransactionPool;
use pezsp_api::{ApiExt, CallApiAt, ProvideRuntimeApi};
use pezsp_blockchain::HeaderBackend;
use pezsp_consensus::{EnableProofRecording, Environment, Proposal};
use pezsp_inherents::{InherentData, InherentDataProvider};
use pezsp_runtime::{traits::Block as BlockT, Digest};
use pezsp_state_machine::StorageProof;
use std::{fmt::Debug, time::Duration};

/// Errors that can occur when proposing a teyrchain block.
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct Error {
	inner: anyhow::Error,
}

impl Error {
	/// Create an error tied to the creation of a proposer.
	pub fn proposer_creation(err: impl Into<anyhow::Error>) -> Self {
		Error { inner: err.into().context("Proposer Creation") }
	}

	/// Create an error tied to the proposing logic itself.
	pub fn proposing(err: impl Into<anyhow::Error>) -> Self {
		Error { inner: err.into().context("Proposing") }
	}
}

/// A type alias for easily referring to the type of a proposal produced by a specific
/// [`ProposerInterface`].
pub type ProposalOf<B> = Proposal<B, StorageProof>;

/// An interface for proposers.
#[async_trait]
pub trait ProposerInterface<Block: BlockT> {
	/// Propose a collation using the supplied `InherentData` and the provided
	/// `TeyrchainInherentData`.
	///
	/// Also specify any required inherent digests, the maximum proposal duration,
	/// and the block size limit in bytes. See the documentation on
	/// [`pezsp_consensus::Proposer::propose`] for more details on how to interpret these
	/// parameters.
	///
	/// The `InherentData` and `Digest` are left deliberately general in order to accommodate
	/// all possible collator selection algorithms or inherent creation mechanisms,
	/// while the `TeyrchainInherentData` is made explicit so it will be constructed appropriately.
	///
	/// If the `InherentData` passed into this function already has a `TeyrchainInherentData`,
	/// this should throw an error.
	async fn propose(
		&mut self,
		parent_header: &Block::Header,
		paras_inherent_data: &TeyrchainInherentData,
		other_inherent_data: InherentData,
		inherent_digests: Digest,
		max_duration: Duration,
		block_size_limit: Option<usize>,
	) -> Result<Option<Proposal<Block, StorageProof>>, Error>;
}

#[async_trait]
impl<Block, A, C> ProposerInterface<Block> for ProposerFactory<A, C, EnableProofRecording>
where
	A: TransactionPool<Block = Block> + 'static,
	C: HeaderBackend<Block> + ProvideRuntimeApi<Block> + CallApiAt<Block> + Send + Sync + 'static,
	C::Api: ApiExt<Block> + BlockBuilderApi<Block>,
	Block: pezsp_runtime::traits::Block,
{
	async fn propose(
		&mut self,
		parent_header: &Block::Header,
		paras_inherent_data: &TeyrchainInherentData,
		other_inherent_data: InherentData,
		inherent_digests: Digest,
		max_duration: Duration,
		block_size_limit: Option<usize>,
	) -> Result<Option<Proposal<Block, StorageProof>>, Error> {
		let proposer = self
			.init(parent_header)
			.await
			.map_err(|e| Error::proposer_creation(anyhow::Error::new(e)))?;

		let mut inherent_data = other_inherent_data;
		paras_inherent_data
			.provide_inherent_data(&mut inherent_data)
			.await
			.map_err(|e| Error::proposing(anyhow::Error::new(e)))?;

		proposer
			.propose_block(ProposeArgs {
				inherent_data,
				inherent_digests,
				max_duration,
				block_size_limit,
				ignored_nodes_by_proof_recording: None,
			})
			.await
			.map(Some)
			.map_err(|e| Error::proposing(anyhow::Error::new(e)).into())
	}
}
