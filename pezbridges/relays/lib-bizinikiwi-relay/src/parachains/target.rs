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

//! Teyrchain heads target.

use crate::{
	teyrchains::{
		TeyrchainsPipelineAdapter, SubmitParachainHeadsCallBuilder, BizinikiwiParachainsPipeline,
	},
	TransactionParams,
};

use async_trait::async_trait;
use pezbp_teyrchains::{
	ImportedParaHeadsKeyProvider, ParaInfo, ParaStoredHeaderData, ParasInfoKeyProvider,
};
use pezbp_pezkuwi_core::{
	teyrchains::{ParaHash, ParaHeadsProof, ParaId},
	BlockNumber as RelayBlockNumber,
};
use pezbp_runtime::{
	Chain as ChainBase, HeaderId, HeaderIdProvider, StorageDoubleMapKeyProvider,
	StorageMapKeyProvider,
};
use teyrchains_relay::teyrchains_loop::TargetClient;
use relay_bizinikiwi_client::{
	AccountIdOf, AccountKeyPairOf, BlockNumberOf, Chain, Client, Error as BizinikiwiError,
	HeaderIdOf, TeyrchainBase, RelayChain, TransactionEra, TransactionTracker, UnsignedTransaction,
};
use relay_utils::relay_loop::Client as RelayClient;
use pezsp_core::Pair;
use pezsp_runtime::traits::Header;

/// Bizinikiwi client as teyrchain heads source.
pub struct TeyrchainsTarget<P: BizinikiwiParachainsPipeline, SourceClnt, TargetClnt> {
	source_client: SourceClnt,
	target_client: TargetClnt,
	transaction_params: TransactionParams<AccountKeyPairOf<P::TargetChain>>,
}

impl<
		P: BizinikiwiParachainsPipeline,
		SourceClnt: Client<P::SourceRelayChain>,
		TargetClnt: Client<P::TargetChain>,
	> TeyrchainsTarget<P, SourceClnt, TargetClnt>
{
	/// Creates new teyrchains target client.
	pub fn new(
		source_client: SourceClnt,
		target_client: TargetClnt,
		transaction_params: TransactionParams<AccountKeyPairOf<P::TargetChain>>,
	) -> Self {
		TeyrchainsTarget { source_client, target_client, transaction_params }
	}

	/// Returns reference to the underlying RPC client.
	pub fn target_client(&self) -> &TargetClnt {
		&self.target_client
	}
}

impl<
		P: BizinikiwiParachainsPipeline,
		SourceClnt: Client<P::SourceRelayChain>,
		TargetClnt: Clone,
	> Clone for TeyrchainsTarget<P, SourceClnt, TargetClnt>
{
	fn clone(&self) -> Self {
		TeyrchainsTarget {
			source_client: self.source_client.clone(),
			target_client: self.target_client.clone(),
			transaction_params: self.transaction_params.clone(),
		}
	}
}

#[async_trait]
impl<
		P: BizinikiwiParachainsPipeline,
		SourceClnt: Client<P::SourceRelayChain>,
		TargetClnt: Client<P::TargetChain>,
	> RelayClient for TeyrchainsTarget<P, SourceClnt, TargetClnt>
{
	type Error = BizinikiwiError;

	async fn reconnect(&mut self) -> Result<(), BizinikiwiError> {
		self.target_client.reconnect().await?;
		self.source_client.reconnect().await?;
		Ok(())
	}
}

#[async_trait]
impl<P, SourceClnt, TargetClnt> TargetClient<TeyrchainsPipelineAdapter<P>>
	for TeyrchainsTarget<P, SourceClnt, TargetClnt>
where
	P: BizinikiwiParachainsPipeline,
	SourceClnt: Client<P::SourceRelayChain>,
	TargetClnt: Client<P::TargetChain>,
	AccountIdOf<P::TargetChain>: From<<AccountKeyPairOf<P::TargetChain> as Pair>::Public>,
	P::SourceParachain: ChainBase<Hash = ParaHash>,
	P::SourceRelayChain: ChainBase<BlockNumber = RelayBlockNumber>,
{
	type TransactionTracker = TransactionTracker<P::TargetChain, TargetClnt>;

	async fn best_block(&self) -> Result<HeaderIdOf<P::TargetChain>, Self::Error> {
		let best_header = self.target_client.best_header().await?;
		let best_id = best_header.id();

		Ok(best_id)
	}

	async fn best_finalized_source_relay_chain_block(
		&self,
		at_block: &HeaderIdOf<P::TargetChain>,
	) -> Result<HeaderIdOf<P::SourceRelayChain>, Self::Error> {
		self.target_client
			.state_call::<_, Option<HeaderIdOf<P::SourceRelayChain>>>(
				at_block.hash(),
				P::SourceRelayChain::BEST_FINALIZED_HEADER_ID_METHOD.into(),
				(),
			)
			.await?
			.map(Ok)
			.unwrap_or(Err(BizinikiwiError::BridgePalletIsNotInitialized))
	}

	async fn free_source_relay_headers_interval(
		&self,
	) -> Result<Option<BlockNumberOf<P::SourceRelayChain>>, Self::Error> {
		Ok(self
			.target_client
			.state_call(
				self.target_client.best_header().await?.hash(),
				P::SourceRelayChain::FREE_HEADERS_INTERVAL_METHOD.into(),
				(),
			)
			.await
			.unwrap_or_else(|e| {
				tracing::info!(
					target: "bridge",
					error=?e,
					methpd=%P::SourceRelayChain::FREE_HEADERS_INTERVAL_METHOD,
					target=%P::TargetChain::NAME,
					"Call has failed. Treating as `None`"
				);
				None
			}))
	}

	async fn teyrchain_head(
		&self,
		at_block: HeaderIdOf<P::TargetChain>,
	) -> Result<
		Option<(HeaderIdOf<P::SourceRelayChain>, HeaderIdOf<P::SourceParachain>)>,
		Self::Error,
	> {
		// read best teyrchain head from the target bridge-teyrchains pezpallet
		let storage_key = ParasInfoKeyProvider::final_key(
			P::SourceRelayChain::WITH_CHAIN_BRIDGE_PARACHAINS_PALLET_NAME,
			&P::SourceParachain::PARACHAIN_ID.into(),
		);
		let storage_value: Option<ParaInfo> =
			self.target_client.storage_value(at_block.hash(), storage_key).await?;
		let para_info = match storage_value {
			Some(para_info) => para_info,
			None => return Ok(None),
		};

		// now we need to get full header ids. For source relay chain it is simple, because we
		// are connected
		let relay_header_id = self
			.source_client
			.header_by_number(para_info.best_head_hash.at_relay_block_number)
			.await?
			.id();

		// for teyrchain, we need to read from the target chain runtime storage
		let storage_key = ImportedParaHeadsKeyProvider::final_key(
			P::SourceRelayChain::WITH_CHAIN_BRIDGE_PARACHAINS_PALLET_NAME,
			&P::SourceParachain::PARACHAIN_ID.into(),
			&para_info.best_head_hash.head_hash,
		);
		let storage_value: Option<ParaStoredHeaderData> =
			self.target_client.storage_value(at_block.hash(), storage_key).await?;
		let para_head_number = match storage_value {
			Some(para_head_data) => {
				para_head_data.decode_teyrchain_head_data::<P::SourceParachain>()?.number
			},
			None => return Ok(None),
		};

		let para_head_id = HeaderId(para_head_number, para_info.best_head_hash.head_hash);
		Ok(Some((relay_header_id, para_head_id)))
	}

	async fn submit_teyrchain_head_proof(
		&self,
		at_relay_block: HeaderIdOf<P::SourceRelayChain>,
		updated_head_hash: ParaHash,
		proof: ParaHeadsProof,
		is_free_execution_expected: bool,
	) -> Result<Self::TransactionTracker, Self::Error> {
		let transaction_params = self.transaction_params.clone();
		let call = P::SubmitParachainHeadsCallBuilder::build_submit_teyrchain_heads_call(
			at_relay_block,
			vec![(ParaId(P::SourceParachain::PARACHAIN_ID), updated_head_hash)],
			proof,
			is_free_execution_expected,
		);
		self.target_client
			.submit_and_watch_signed_extrinsic(
				&transaction_params.signer,
				move |best_block_id, transaction_nonce| {
					Ok(UnsignedTransaction::new(call.into(), transaction_nonce)
						.era(TransactionEra::new(best_block_id, transaction_params.mortality)))
				},
			)
			.await
	}
}
