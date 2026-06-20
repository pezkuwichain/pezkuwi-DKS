// Copyright 2019-2023 Parity Technologies (UK) Ltd.
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

//! Default generic implementation of equivocation source for basic Bizinikiwi client.

use crate::{
	equivocation::{
		BizinikiwiEquivocationDetectionPipeline, EquivocationDetectionPipelineAdapter,
		EquivocationProofOf, ReportEquivocationCallBuilder,
	},
	finality_base::{engine::Engine, finality_proofs, BizinikiwiFinalityProofsStream},
	TransactionParams,
};

use async_trait::async_trait;
use pez_equivocation_detector::SourceClient;
use pez_finality_relay::SourceClientBase;
use pezbp_runtime::{HashOf, TransactionEra};
use relay_bizinikiwi_client::{
	AccountKeyPairOf, Client, Error, TransactionTracker, UnsignedTransaction,
};
use relay_utils::relay_loop::Client as RelayClient;

/// Bizinikiwi node as equivocation source.
pub struct BizinikiwiEquivocationSource<P: BizinikiwiEquivocationDetectionPipeline, SourceClnt> {
	client: SourceClnt,
	transaction_params: TransactionParams<AccountKeyPairOf<P::SourceChain>>,
}

impl<P: BizinikiwiEquivocationDetectionPipeline, SourceClnt: Client<P::SourceChain>>
	BizinikiwiEquivocationSource<P, SourceClnt>
{
	/// Create new instance of `BizinikiwiEquivocationSource`.
	pub fn new(
		client: SourceClnt,
		transaction_params: TransactionParams<AccountKeyPairOf<P::SourceChain>>,
	) -> Self {
		Self { client, transaction_params }
	}
}

impl<P: BizinikiwiEquivocationDetectionPipeline, SourceClnt: Client<P::SourceChain>> Clone
	for BizinikiwiEquivocationSource<P, SourceClnt>
{
	fn clone(&self) -> Self {
		Self { client: self.client.clone(), transaction_params: self.transaction_params.clone() }
	}
}

#[async_trait]
impl<P: BizinikiwiEquivocationDetectionPipeline, SourceClnt: Client<P::SourceChain>> RelayClient
	for BizinikiwiEquivocationSource<P, SourceClnt>
{
	type Error = Error;

	async fn reconnect(&mut self) -> Result<(), Error> {
		self.client.reconnect().await
	}
}

#[async_trait]
impl<P: BizinikiwiEquivocationDetectionPipeline, SourceClnt: Client<P::SourceChain>>
	SourceClientBase<EquivocationDetectionPipelineAdapter<P>>
	for BizinikiwiEquivocationSource<P, SourceClnt>
{
	type FinalityProofsStream = BizinikiwiFinalityProofsStream<P>;

	async fn finality_proofs(&self) -> Result<Self::FinalityProofsStream, Error> {
		finality_proofs::<P>(&self.client).await
	}
}

#[async_trait]
impl<P: BizinikiwiEquivocationDetectionPipeline, SourceClnt: Client<P::SourceChain>>
	SourceClient<EquivocationDetectionPipelineAdapter<P>>
	for BizinikiwiEquivocationSource<P, SourceClnt>
{
	type TransactionTracker = TransactionTracker<P::SourceChain, SourceClnt>;

	async fn report_equivocation(
		&self,
		at: HashOf<P::SourceChain>,
		equivocation: EquivocationProofOf<P>,
	) -> Result<Self::TransactionTracker, Self::Error> {
		let key_owner_proof =
			P::FinalityEngine::generate_source_key_ownership_proof(&self.client, at, &equivocation)
				.await?;

		let mortality = self.transaction_params.mortality;
		let call = P::ReportEquivocationCallBuilder::build_report_equivocation_call(
			equivocation,
			key_owner_proof,
		);
		self.client
			.submit_and_watch_signed_extrinsic(
				&self.transaction_params.signer,
				move |best_block_id, transaction_nonce| {
					Ok(UnsignedTransaction::new(call.into(), transaction_nonce)
						.era(TransactionEra::new(best_block_id, mortality)))
				},
			)
			.await
	}
}
