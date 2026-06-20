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

//! Types and functions intended to ease adding of new Bizinikiwi -> Bizinikiwi
//! teyrchain finality proofs synchronization pipelines.

use async_trait::async_trait;
use pezbp_pezkuwi_core::teyrchains::{ParaHash, ParaHeadsProof, ParaId};
use pezbp_teyrchains::{RelayBlockHash, RelayBlockHasher, RelayBlockNumber};
use pezpallet_bridge_teyrchains::{Call as BridgeTeyrchainsCall, Config as BridgeTeyrchainsConfig};
use relay_bizinikiwi_client::{
	CallOf, Chain, ChainWithTransactions, HeaderIdOf, RelayChain, Teyrchain,
};
use std::{fmt::Debug, marker::PhantomData};
use teyrchains_relay::TeyrchainsPipeline;

pub mod source;
pub mod target;

/// Bizinikiwi -> Bizinikiwi teyrchain finality proofs synchronization pipeline.
///
/// This is currently restricted to the single teyrchain, because it is how it
/// will be used (at least) initially.
#[async_trait]
pub trait BizinikiwiTeyrchainsPipeline: 'static + Clone + Debug + Send + Sync {
	/// Headers of this teyrchain are submitted to the `Self::TargetChain`.
	type SourceTeyrchain: Teyrchain;
	/// Relay chain that is storing headers of `Self::SourceTeyrchain`.
	type SourceRelayChain: RelayChain;
	/// Target chain where `Self::SourceTeyrchain` headers are submitted.
	type TargetChain: ChainWithTransactions;

	/// How submit teyrchains heads call is built?
	type SubmitTeyrchainHeadsCallBuilder: SubmitTeyrchainHeadsCallBuilder<Self>;
}

/// Adapter that allows all `BizinikiwiTeyrchainsPipeline` to act as `TeyrchainsPipeline`.
#[derive(Clone, Debug)]
pub struct TeyrchainsPipelineAdapter<P: BizinikiwiTeyrchainsPipeline> {
	_phantom: PhantomData<P>,
}

impl<P: BizinikiwiTeyrchainsPipeline> TeyrchainsPipeline for TeyrchainsPipelineAdapter<P> {
	type SourceTeyrchain = P::SourceTeyrchain;
	type SourceRelayChain = P::SourceRelayChain;
	type TargetChain = P::TargetChain;
}

/// Different ways of building `submit_teyrchain_heads` calls.
pub trait SubmitTeyrchainHeadsCallBuilder<P: BizinikiwiTeyrchainsPipeline>:
	'static + Send + Sync
{
	/// Given teyrchains and their heads proof, build call of `submit_teyrchain_heads`
	/// function of bridge teyrchains module at the target chain.
	fn build_submit_teyrchain_heads_call(
		at_relay_block: HeaderIdOf<P::SourceRelayChain>,
		teyrchains: Vec<(ParaId, ParaHash)>,
		teyrchain_heads_proof: ParaHeadsProof,
		is_free_execution_expected: bool,
	) -> CallOf<P::TargetChain>;
}

/// Building `submit_teyrchain_heads` call when you have direct access to the target
/// chain runtime.
pub struct DirectSubmitTeyrchainHeadsCallBuilder<P, R, I> {
	_phantom: PhantomData<(P, R, I)>,
}

impl<P, R, I> SubmitTeyrchainHeadsCallBuilder<P> for DirectSubmitTeyrchainHeadsCallBuilder<P, R, I>
where
	P: BizinikiwiTeyrchainsPipeline,
	P::SourceRelayChain: Chain<Hash = RelayBlockHash, BlockNumber = RelayBlockNumber>,
	R: BridgeTeyrchainsConfig<I> + Send + Sync,
	I: 'static + Send + Sync,
	R::BridgedChain: pezbp_runtime::Chain<
		BlockNumber = RelayBlockNumber,
		Hash = RelayBlockHash,
		Hasher = RelayBlockHasher,
	>,
	CallOf<P::TargetChain>: From<BridgeTeyrchainsCall<R, I>>,
{
	fn build_submit_teyrchain_heads_call(
		at_relay_block: HeaderIdOf<P::SourceRelayChain>,
		teyrchains: Vec<(ParaId, ParaHash)>,
		teyrchain_heads_proof: ParaHeadsProof,
		_is_free_execution_expected: bool,
	) -> CallOf<P::TargetChain> {
		BridgeTeyrchainsCall::<R, I>::submit_teyrchain_heads {
			at_relay_block: (at_relay_block.0, at_relay_block.1),
			teyrchains,
			teyrchain_heads_proof,
		}
		.into()
	}
}
