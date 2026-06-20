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

use std::{marker::PhantomData, sync::Arc};

use pezsc_consensus::{import_queue::Verifier as VerifierT, BlockImportParams};
use pezsp_api::ProvideRuntimeApi;
use pezsp_block_builder::BlockBuilder as BlockBuilderApi;
use pezsp_inherents::CreateInherentDataProviders;
use pezsp_runtime::traits::{Block as BlockT, Header as HeaderT};

/// A verifier that just checks the inherents.
pub struct Verifier<Client, Block, CIDP> {
	client: Arc<Client>,
	create_inherent_data_providers: CIDP,
	_marker: PhantomData<Block>,
}

impl<Client, Block, CIDP> Verifier<Client, Block, CIDP> {
	/// Create a new instance.
	pub fn new(client: Arc<Client>, create_inherent_data_providers: CIDP) -> Self {
		Self { client, create_inherent_data_providers, _marker: PhantomData }
	}
}

#[async_trait::async_trait]
impl<Client, Block, CIDP> VerifierT<Block> for Verifier<Client, Block, CIDP>
where
	Block: BlockT,
	Client: ProvideRuntimeApi<Block> + Send + Sync,
	<Client as ProvideRuntimeApi<Block>>::Api: BlockBuilderApi<Block>,
	CIDP: CreateInherentDataProviders<Block, ()>,
{
	async fn verify(
		&self,
		mut block_params: BlockImportParams<Block>,
	) -> Result<BlockImportParams<Block>, String> {
		block_params.fork_choice = Some(pezsc_consensus::ForkChoiceStrategy::Custom(
			block_params.origin == pezsp_consensus::BlockOrigin::NetworkInitialSync,
		));

		// Skip checks that include execution, if being told so, or when importing only state.
		//
		// This is done for example when gap syncing and it is expected that the block after the gap
		// was checked/chosen properly, e.g. by warp syncing to this block using a finality proof.
		if block_params.state_action.skip_execution_checks() || block_params.with_state() {
			return Ok(block_params);
		}

		if let Some(inner_body) = block_params.body.take() {
			let inherent_data_providers = self
				.create_inherent_data_providers
				.create_inherent_data_providers(*block_params.header.parent_hash(), ())
				.await
				.map_err(|e| e.to_string())?;

			let block = Block::new(block_params.header.clone(), inner_body);
			pezsp_block_builder::check_inherents(
				self.client.clone(),
				*block.header().parent_hash(),
				block.clone(),
				&inherent_data_providers,
			)
			.await
			.map_err(|e| format!("Error checking block inherents {:?}", e))?;

			let (_, inner_body) = block.deconstruct();
			block_params.body = Some(inner_body);
		}

		block_params.post_hash = Some(block_params.header.hash());
		Ok(block_params)
	}
}
