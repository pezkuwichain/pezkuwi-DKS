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
//! Pezkuwi-specific JSON-RPC methods.

use crate::*;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use pezsp_runtime::Weight;

#[rpc(server, client)]
pub trait PolkadotRpc {
	/// Get the post dispatch weight for a given transaction hash.
	#[method(name = "pezkuwi_postDispatchWeight")]
	async fn post_dispatch_weight(&self, transaction_hash: H256) -> RpcResult<Option<Weight>>;
}

pub struct PolkadotRpcServerImpl {
	client: client::Client,
}

impl PolkadotRpcServerImpl {
	pub fn new(client: client::Client) -> Self {
		Self { client }
	}
}

#[async_trait]
impl PolkadotRpcServer for PolkadotRpcServerImpl {
	async fn post_dispatch_weight(&self, transaction_hash: H256) -> RpcResult<Option<Weight>> {
		let weight = self.client.post_dispatch_weight(&transaction_hash).await;
		Ok(weight)
	}
}
