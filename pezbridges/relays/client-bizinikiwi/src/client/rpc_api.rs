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

//! The most generic Bizinikiwi node RPC interface.

use crate::{Chain, ChainWithGrandpa, TransactionStatusOf};

use jsonrpsee::proc_macros::rpc;
use pezpallet_transaction_payment_rpc_runtime_api::FeeDetails;
use pezsc_rpc_api::{state::ReadProof, system::Health};
use pezsp_core::{
	storage::{StorageData, StorageKey},
	Bytes,
};
use pezsp_rpc::number::NumberOrHex;
use pezsp_version::RuntimeVersion;

/// RPC methods of Bizinikiwi `system` namespace, that we are using.
#[rpc(client, client_bounds(C: Chain), namespace = "system")]
pub(crate) trait BizinikiwiSystem<C> {
	/// Return node health.
	#[method(name = "health")]
	async fn health(&self) -> RpcResult<Health>;
	/// Return system properties.
	#[method(name = "properties")]
	async fn properties(&self) -> RpcResult<pezsc_chain_spec::Properties>;
}

/// RPC methods of Bizinikiwi `chain` namespace, that we are using.
#[rpc(client, client_bounds(C: Chain), namespace = "chain")]
pub(crate) trait BizinikiwiChain<C> {
	/// Get block hash by its number.
	#[method(name = "getBlockHash")]
	async fn block_hash(&self, block_number: Option<C::BlockNumber>) -> RpcResult<C::Hash>;
	/// Return block header by its hash.
	#[method(name = "getHeader")]
	async fn header(&self, block_hash: Option<C::Hash>) -> RpcResult<C::Header>;
	/// Return best finalized block hash.
	#[method(name = "getFinalizedHead")]
	async fn finalized_head(&self) -> RpcResult<C::Hash>;
	/// Return signed block (with justifications) by its hash.
	#[method(name = "getBlock")]
	async fn block(&self, block_hash: Option<C::Hash>) -> RpcResult<C::SignedBlock>;
	/// Subscribe to best headers.
	#[subscription(
		name = "subscribeNewHeads" => "newHead",
		unsubscribe = "unsubscribeNewHeads",
		item = C::Header
	)]
	async fn subscribe_new_heads(&self);
	/// Subscribe to finalized headers.
	#[subscription(
		name = "subscribeFinalizedHeads" => "finalizedHead",
		unsubscribe = "unsubscribeFinalizedHeads",
		item = C::Header
	)]
	async fn subscribe_finalized_heads(&self);
}

/// RPC methods of Bizinikiwi `author` namespace, that we are using.
#[rpc(client, client_bounds(C: Chain), namespace = "author")]
pub(crate) trait BizinikiwiAuthor<C> {
	/// Submit extrinsic to the transaction pool.
	#[method(name = "submitExtrinsic")]
	async fn submit_extrinsic(&self, extrinsic: Bytes) -> RpcResult<C::Hash>;
	/// Return vector of pending extrinsics from the transaction pool.
	#[method(name = "pendingExtrinsics")]
	async fn pending_extrinsics(&self) -> RpcResult<Vec<Bytes>>;
	/// Submit and watch for extrinsic state.
	#[subscription(name = "submitAndWatchExtrinsic", unsubscribe = "unwatchExtrinsic", item = TransactionStatusOf<C>)]
	async fn submit_and_watch_extrinsic(&self, extrinsic: Bytes);
}

/// RPC methods of Bizinikiwi `state` namespace, that we are using.
#[rpc(client, client_bounds(C: Chain), namespace = "state")]
pub(crate) trait BizinikiwiState<C> {
	/// Get current runtime version.
	#[method(name = "getRuntimeVersion")]
	async fn runtime_version(&self) -> RpcResult<RuntimeVersion>;
	/// Call given runtime method.
	#[method(name = "call")]
	async fn call(
		&self,
		method: String,
		data: Bytes,
		at_block: Option<C::Hash>,
	) -> RpcResult<Bytes>;
	/// Get value of the runtime storage.
	#[method(name = "getStorage")]
	async fn storage(
		&self,
		key: StorageKey,
		at_block: Option<C::Hash>,
	) -> RpcResult<Option<StorageData>>;
	/// Get proof of the runtime storage value.
	#[method(name = "getReadProof")]
	async fn prove_storage(
		&self,
		keys: Vec<StorageKey>,
		hash: Option<C::Hash>,
	) -> RpcResult<ReadProof<C::Hash>>;
}

/// RPC methods of Bizinikiwi `grandpa` namespace, that we are using.
#[rpc(client, client_bounds(C: ChainWithGrandpa), namespace = "grandpa")]
pub(crate) trait BizinikiwiGrandpa<C> {
	/// Subscribe to GRANDPA justifications.
	#[subscription(name = "subscribeJustifications", unsubscribe = "unsubscribeJustifications", item = Bytes)]
	async fn subscribe_justifications(&self);
}

// TODO: Use `ChainWithBeefy` instead of `Chain` after #1606 is merged
/// RPC methods of Bizinikiwi `beefy` namespace, that we are using.
#[rpc(client, client_bounds(C: Chain), namespace = "beefy")]
pub(crate) trait BizinikiwiBeefy<C> {
	/// Subscribe to BEEFY justifications.
	#[subscription(name = "subscribeJustifications", unsubscribe = "unsubscribeJustifications", item = Bytes)]
	async fn subscribe_justifications(&self);
}

/// RPC methods of Bizinikiwi `system` frame pezpallet, that we are using.
#[rpc(client, client_bounds(C: Chain), namespace = "system")]
pub(crate) trait BizinikiwiFrameSystem<C> {
	/// Return index of next account transaction.
	#[method(name = "accountNextIndex")]
	async fn account_next_index(&self, account_id: C::AccountId) -> RpcResult<C::Nonce>;
}

/// RPC methods of Bizinikiwi `pezpallet_transaction_payment` frame pezpallet, that we are using.
#[rpc(client, client_bounds(C: Chain), namespace = "payment")]
pub(crate) trait BizinikiwiTransactionPayment<C> {
	/// Query transaction fee details.
	#[method(name = "queryFeeDetails")]
	async fn fee_details(
		&self,
		extrinsic: Bytes,
		at_block: Option<C::Hash>,
	) -> RpcResult<FeeDetails<NumberOrHex>>;
}
