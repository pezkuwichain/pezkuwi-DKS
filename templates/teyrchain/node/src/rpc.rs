//! A collection of node-specific RPC methods.
//! Bizinikiwi provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Bizinikiwi nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use teyrchain_template_runtime::{opaque::Block, AccountId, Balance, Nonce};

use pezkuwi_sdk::*;

use pezsc_transaction_pool_api::TransactionPool;
use pezsp_api::ProvideRuntimeApi;
use pezsp_block_builder::BlockBuilder;
use pezsp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies
pub struct FullDeps<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P>(
	deps: FullDeps<C, P>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Send
		+ Sync
		+ 'static,
	C::Api: pezpallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: bizinikiwi_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + Sync + Send + 'static,
{
	use bizinikiwi_frame_rpc_system::{System, SystemApiServer};
	use pezpallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};

	let mut module = RpcExtension::new(());
	let FullDeps { client, pool } = deps;

	module.merge(System::new(client.clone(), pool).into_rpc())?;
	module.merge(TransactionPayment::new(client).into_rpc())?;
	Ok(module)
}
