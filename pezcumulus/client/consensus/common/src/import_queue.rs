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

//! (unstable) Composable utilities for constructing import queues for teyrchains.
//!
//! Unlike standalone chains, teyrchains have the requirement that all consensus logic
//! must be checked within the runtime. This property means that work which is normally
//! done in the import queue per-block, such as checking signatures, quorums, and whether
//! inherent extrinsics were constructed faithfully do not need to be done, per se.
//!
//! It may seem that it would be beneficial for the client to do these checks regardless,
//! but in practice this means that clients would just reject blocks which are _valid_ according
//! to their Teyrchain Validation Function, which is the ultimate source of consensus truth.
//!
//! However, teyrchain runtimes expose two different access points for executing blocks
//! in full nodes versus executing those blocks in the teyrchain validation environment.
//! At the time of writing, the inherent and consensus checks in most Pezcumulus runtimes
//! are only performed during teyrchain validation, not full node block execution.
//!
//! See <https://github.com/pezkuwichain/pezkuwi-sdk/issues/238> for details.

use pezsp_consensus::error::Error as ConsensusError;
use pezsp_runtime::traits::Block as BlockT;

use pezsc_consensus::{
	block_import::{BlockImport, BlockImportParams},
	import_queue::{BasicQueue, Verifier},
};

use crate::TeyrchainBlockImportMarker;

/// A [`Verifier`] for blocks which verifies absolutely nothing.
///
/// This should only be used when the runtime is responsible for checking block seals and inherents.
pub struct VerifyNothing;

#[async_trait::async_trait]
impl<Block: BlockT> Verifier<Block> for VerifyNothing {
	async fn verify(
		&self,
		params: BlockImportParams<Block>,
	) -> Result<BlockImportParams<Block>, String> {
		Ok(params)
	}
}

/// An import queue which does no verification.
///
/// This should only be used when the runtime is responsible for checking block seals and inherents.
pub fn verify_nothing_import_queue<Block: BlockT, I>(
	block_import: I,
	spawner: &impl pezsp_core::traits::SpawnEssentialNamed,
	registry: Option<&prometheus_endpoint::Registry>,
) -> BasicQueue<Block>
where
	I: BlockImport<Block, Error = ConsensusError>
		+ TeyrchainBlockImportMarker
		+ Send
		+ Sync
		+ 'static,
{
	BasicQueue::new(VerifyNothing, Box::new(block_import), None, spawner, registry)
}
