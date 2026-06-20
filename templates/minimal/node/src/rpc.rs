// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A collection of node-specific RPC methods.
//! Bizinikiwi provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Bizinikiwi nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use jsonrpsee::RpcModule;
use pez_minimal_template_runtime::interface::{AccountId, Nonce, OpaqueBlock};
use pezkuwi_sdk::{
	pezsc_transaction_pool_api::TransactionPool,
	pezsp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata},
	*,
};
use std::sync::Arc;

/// Full client dependencies.
pub struct FullDeps<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
}

#[docify::export]
/// Instantiate all full RPC extensions.
pub fn create_full<C, P>(
	deps: FullDeps<C, P>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: Send
		+ Sync
		+ 'static
		+ pezkuwi_sdk::pezsp_api::ProvideRuntimeApi<OpaqueBlock>
		+ HeaderBackend<OpaqueBlock>
		+ HeaderMetadata<OpaqueBlock, Error = BlockChainError>
		+ 'static,
	C::Api: pezkuwi_sdk::pezsp_block_builder::BlockBuilder<OpaqueBlock>,
	C::Api: bizinikiwi_frame_rpc_system::AccountNonceApi<OpaqueBlock, AccountId, Nonce>,
	P: TransactionPool + 'static,
{
	use pezkuwi_sdk::bizinikiwi_frame_rpc_system::{System, SystemApiServer};
	let mut module = RpcModule::new(());
	let FullDeps { client, pool } = deps;

	module.merge(System::new(client.clone(), pool.clone()).into_rpc())?;

	Ok(module)
}
