// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

//! Teyrchain-specific RPCs implementation.

#![warn(missing_docs)]

use crate::common::{
	types::{AccountId, Balance, Nonce, TeyrchainBackend, TeyrchainClient},
	ConstructNodeRuntimeApi,
};
use bizinikiwi_frame_rpc_system::{System, SystemApiServer};
use bizinikiwi_state_trie_migration_rpc::{StateMigration, StateMigrationApiServer};
use pezpallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
use pezsc_rpc::{
	dev::{Dev, DevApiServer},
	statement::{StatementApiServer, StatementStore},
};
use pezsp_runtime::traits::Block as BlockT;
use std::{marker::PhantomData, sync::Arc};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

pub(crate) trait BuildRpcExtensions<Client, Backend, Pool, StatementStore> {
	fn build_rpc_extensions(
		client: Arc<Client>,
		backend: Arc<Backend>,
		pool: Arc<Pool>,
		statement_store: Option<Arc<StatementStore>>,
	) -> pezsc_service::error::Result<RpcExtension>;
}

pub(crate) struct BuildTeyrchainRpcExtensions<Block, RuntimeApi>(PhantomData<(Block, RuntimeApi)>);

impl<Block: BlockT, RuntimeApi>
	BuildRpcExtensions<
		TeyrchainClient<Block, RuntimeApi>,
		TeyrchainBackend<Block>,
		pezsc_transaction_pool::TransactionPoolHandle<Block, TeyrchainClient<Block, RuntimeApi>>,
		pezsc_statement_store::Store,
	> for BuildTeyrchainRpcExtensions<Block, RuntimeApi>
where
	RuntimeApi:
		ConstructNodeRuntimeApi<Block, TeyrchainClient<Block, RuntimeApi>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: pezpallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
		+ bizinikiwi_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
{
	fn build_rpc_extensions(
		client: Arc<TeyrchainClient<Block, RuntimeApi>>,
		backend: Arc<TeyrchainBackend<Block>>,
		pool: Arc<
			pezsc_transaction_pool::TransactionPoolHandle<
				Block,
				TeyrchainClient<Block, RuntimeApi>,
			>,
		>,
		statement_store: Option<Arc<pezsc_statement_store::Store>>,
	) -> pezsc_service::error::Result<RpcExtension> {
		let build = || -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>> {
			let mut module = RpcExtension::new(());

			module.merge(System::new(client.clone(), pool).into_rpc())?;
			module.merge(TransactionPayment::new(client.clone()).into_rpc())?;
			module.merge(StateMigration::new(client.clone(), backend).into_rpc())?;
			if let Some(statement_store) = statement_store {
				module.merge(StatementStore::new(statement_store).into_rpc())?;
			}
			module.merge(Dev::new(client).into_rpc())?;

			Ok(module)
		};
		build().map_err(Into::into)
	}
}
