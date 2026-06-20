// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! Pezkuwi-specific RPCs implementation.

#![warn(missing_docs)]

use std::sync::Arc;

use jsonrpsee::RpcModule;
use pezkuwi_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Nonce};
use pezsc_client_api::AuxStore;
use pezsc_consensus_beefy::communication::notification::{
	BeefyBestBlockStream, BeefyVersionedFinalityProofStream,
};
use pezsc_consensus_grandpa::FinalityProofProvider;
pub use pezsc_rpc::SubscriptionTaskExecutor;
use pezsc_transaction_pool_api::TransactionPool;
use pezsp_api::ProvideRuntimeApi;
use pezsp_application_crypto::RuntimeAppPublic;
use pezsp_block_builder::BlockBuilder;
use pezsp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use pezsp_consensus::SelectChain;
use pezsp_consensus_babe::BabeApi;
use pezsp_consensus_beefy::AuthorityIdBound;
use pezsp_keystore::KeystorePtr;

/// A type representing all RPC extensions.
pub type RpcExtension = RpcModule<()>;

/// Extra dependencies for BABE.
pub struct BabeDeps {
	/// A handle to the BABE worker for issuing requests.
	pub babe_worker_handle: pezsc_consensus_babe::BabeWorkerHandle<Block>,
	/// The keystore that manages the keys of the node.
	pub keystore: KeystorePtr,
}

/// Dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: pezsc_consensus_grandpa::SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: pezsc_consensus_grandpa::SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: pezsc_consensus_grandpa::GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: pezsc_rpc::SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Dependencies for BEEFY
pub struct BeefyDeps<AuthorityId: AuthorityIdBound> {
	/// Receives notifications about finality proof events from BEEFY.
	pub beefy_finality_proof_stream: BeefyVersionedFinalityProofStream<Block, AuthorityId>,
	/// Receives notifications about best block events from BEEFY.
	pub beefy_best_block_stream: BeefyBestBlockStream<Block>,
	/// Executor to drive the subscription manager in the BEEFY RPC handler.
	pub subscription_executor: pezsc_rpc::SubscriptionTaskExecutor,
}

/// Full client dependencies
pub struct FullDeps<C, P, SC, B, AuthorityId: AuthorityIdBound> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// The [`SelectChain`] Strategy
	pub select_chain: SC,
	/// A copy of the chain spec.
	pub chain_spec: Box<dyn pezsc_chain_spec::ChainSpec>,
	/// BABE specific dependencies.
	pub babe: BabeDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	/// BEEFY specific dependencies.
	pub beefy: BeefyDeps<AuthorityId>,
	/// Backend used by the node.
	pub backend: Arc<B>,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, SC, B, AuthorityId>(
	FullDeps { client, pool, select_chain, chain_spec, babe, grandpa, beefy, backend }: FullDeps<
		C,
		P,
		SC,
		B,
		AuthorityId,
	>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Send
		+ Sync
		+ 'static,
	C::Api: bizinikiwi_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pezmmr_rpc::MmrRuntimeApi<
		Block,
		<Block as pezsp_runtime::traits::Block>::Hash,
		BlockNumber,
	>,
	C::Api: pezpallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BabeApi<Block>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + Sync + Send + 'static,
	SC: SelectChain<Block> + 'static,
	B: pezsc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: pezsc_client_api::StateBackend<pezsp_runtime::traits::HashingFor<Block>>,
	AuthorityId: AuthorityIdBound,
	<AuthorityId as RuntimeAppPublic>::Signature: Send + Sync,
{
	use bizinikiwi_frame_rpc_system::{System, SystemApiServer};
	use bizinikiwi_state_trie_migration_rpc::{StateMigration, StateMigrationApiServer};
	use pezmmr_rpc::{Mmr, MmrApiServer};
	use pezpallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use pezsc_consensus_babe_rpc::{Babe, BabeApiServer};
	use pezsc_consensus_beefy_rpc::{Beefy, BeefyApiServer};
	use pezsc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use pezsc_sync_state_rpc::{SyncState, SyncStateApiServer};

	let mut io = RpcModule::new(());
	let BabeDeps { babe_worker_handle, keystore } = babe;
	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	io.merge(StateMigration::new(client.clone(), backend.clone()).into_rpc())?;
	io.merge(System::new(client.clone(), pool.clone()).into_rpc())?;
	io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	io.merge(
		Mmr::new(
			client.clone(),
			backend
				.offchain_storage()
				.ok_or("Backend doesn't provide the required offchain storage")?,
		)
		.into_rpc(),
	)?;
	io.merge(
		Babe::new(client.clone(), babe_worker_handle.clone(), keystore, select_chain).into_rpc(),
	)?;
	io.merge(
		Grandpa::new(
			subscription_executor,
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;
	io.merge(
		SyncState::new(chain_spec, client, shared_authority_set, babe_worker_handle)?.into_rpc(),
	)?;

	io.merge(
		Beefy::<Block, AuthorityId>::new(
			beefy.beefy_finality_proof_stream,
			beefy.beefy_best_block_stream,
			beefy.subscription_executor,
		)?
		.into_rpc(),
	)?;

	Ok(io)
}
