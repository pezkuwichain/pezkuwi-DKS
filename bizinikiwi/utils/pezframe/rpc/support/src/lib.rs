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

//! Combines [pezsc_rpc_api::state::StateApiClient] with [pezframe_support::storage::generator]
//! traits to provide strongly typed chain state queries over rpc.

#![warn(missing_docs)]

use codec::{DecodeAll, FullCodec, FullEncode};
use core::marker::PhantomData;
use jsonrpsee::core::ClientError as RpcError;
use pezframe_support::storage::generator::{StorageDoubleMap, StorageMap, StorageValue};
use pezsc_rpc_api::state::StateApiClient;
use pezsp_storage::{StorageData, StorageKey};
use serde::{de::DeserializeOwned, Serialize};

/// A typed query on chain state usable from an RPC client.
///
/// ```no_run
/// # use jsonrpsee::core::ClientError as RpcError;
/// # use jsonrpsee::ws_client::WsClientBuilder;
/// # use codec::Encode;
/// # use pezframe_support::{construct_runtime, derive_impl, traits::ConstU32};
/// # use bizinikiwi_frame_rpc_support::StorageQuery;
/// # use pezsc_rpc_api::state::StateApiClient;
/// # use pezsp_runtime::{traits::{BlakeTwo256, IdentityLookup}, testing::Header};
/// #
/// # construct_runtime!(
/// # 	pub enum TestRuntime
/// # 	{
/// # 		System: pezframe_system,
/// # 		Test: pezpallet_test,
/// # 	}
/// # );
/// #
/// # type Hash = pezsp_core::H256;
/// #
/// # #[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
/// # impl pezframe_system::Config for TestRuntime {
/// # 	type BaseCallFilter = ();
/// # 	type BlockWeights = ();
/// # 	type BlockLength = ();
/// # 	type RuntimeOrigin = RuntimeOrigin;
/// # 	type RuntimeCall = RuntimeCall;
/// # 	type Nonce = u64;
/// # 	type Hash = Hash;
/// # 	type Hashing = BlakeTwo256;
/// # 	type AccountId = u64;
/// # 	type Lookup = IdentityLookup<Self::AccountId>;
/// # 	type Block = pezframe_system::mocking::MockBlock<TestRuntime>;
/// # 	type RuntimeEvent = RuntimeEvent;
/// # 	type RuntimeTask = RuntimeTask;
/// # 	type BlockHashCount = ();
/// # 	type DbWeight = ();
/// # 	type Version = ();
/// # 	type PalletInfo = PalletInfo;
/// # 	type AccountData = ();
/// # 	type OnNewAccount = ();
/// # 	type OnKilledAccount = ();
/// # 	type SystemWeightInfo = ();
/// # 	type SS58Prefix = ();
/// # 	type OnSetCode = ();
/// # 	type MaxConsumers = ConstU32<16>;
/// # }
/// #
/// # impl pezpallet_test::Config for TestRuntime {}
/// #
///
/// pub type Loc = (i64, i64, i64);
/// pub type Block = u8;
///
/// // Note that all fields are marked pub.
/// pub use self::pezpallet_test::*;
///
/// #[pezframe_support::pezpallet]
/// mod pezpallet_test {
/// 	use super::*;
/// 	use pezframe_support::pezpallet_prelude::*;
///
/// 	#[pezpallet::pezpallet]
/// 	pub struct Pezpallet<T>(_);
///
/// 	#[pezpallet::config]
/// 	pub trait Config: pezframe_system::Config {}
///
/// 	#[pezpallet::storage]
/// 	pub type LastActionId<T> = StorageValue<_, u64, ValueQuery>;
///
/// 	#[pezpallet::storage]
/// 	pub type Voxels<T> = StorageMap<_, Blake2_128Concat, Loc, Block>;
///
/// 	#[pezpallet::storage]
/// 	pub type Actions<T> = StorageMap<_, Blake2_128Concat, u64, Loc>;
///
/// 	#[pezpallet::storage]
/// 	pub type Prefab<T> = StorageDoubleMap<
/// 		_,
/// 		Blake2_128Concat, u128,
/// 		Blake2_128Concat, (i8, i8, i8), Block
/// 	>;
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), RpcError> {
///     let cl = WsClientBuilder::default().build("ws://[::1]:9944").await?;
///
///     let q = StorageQuery::value::<LastActionId<TestRuntime>>();
///     let hash = None::<Hash>;
///     let _: Option<u64> = q.get(&cl, hash).await?;
///
///     let q = StorageQuery::map::<Voxels<TestRuntime>, _>((0, 0, 0));
///     let _: Option<Block> = q.get(&cl, hash).await?;
///
///     let q = StorageQuery::map::<Actions<TestRuntime>, _>(12);
///     let _: Option<Loc> = q.get(&cl, hash).await?;
///
///     let q = StorageQuery::double_map::<Prefab<TestRuntime>, _, _>(3, (0, 0, 0));
///     let _: Option<Block> = q.get(&cl, hash).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct StorageQuery<V> {
	key: StorageKey,
	_spook: PhantomData<V>,
}

impl<V: FullCodec> StorageQuery<V> {
	/// Create a storage query for a StorageValue.
	pub fn value<St: StorageValue<V>>() -> Self {
		Self { key: StorageKey(St::storage_value_final_key().to_vec()), _spook: PhantomData }
	}

	/// Create a storage query for a value in a StorageMap.
	pub fn map<St: StorageMap<K, V>, K: FullEncode>(key: K) -> Self {
		Self { key: StorageKey(St::storage_map_final_key(key)), _spook: PhantomData }
	}

	/// Create a storage query for a value in a StorageDoubleMap.
	pub fn double_map<St: StorageDoubleMap<K1, K2, V>, K1: FullEncode, K2: FullEncode>(
		key1: K1,
		key2: K2,
	) -> Self {
		Self { key: StorageKey(St::storage_double_map_final_key(key1, key2)), _spook: PhantomData }
	}

	/// Send this query over RPC, await the typed result.
	///
	/// Hash should be `<YourRuntime as pezframe_system::Config>::Hash`.
	///
	/// # Arguments
	///
	/// state_client represents a connection to the RPC server.
	///
	/// block_index indicates the block for which state will be queried. A value of None indicates
	/// the latest block.
	pub async fn get<Hash, StateClient>(
		self,
		state_client: &StateClient,
		block_index: Option<Hash>,
	) -> Result<Option<V>, RpcError>
	where
		Hash: Send + Sync + 'static + DeserializeOwned + Serialize,
		StateClient: StateApiClient<Hash> + Sync,
	{
		let opt: Option<StorageData> = state_client.storage(self.key, block_index).await?;
		opt.map(|encoded| V::decode_all(&mut &encoded.0[..]))
			.transpose()
			.map_err(|decode_err| RpcError::Custom(decode_err.to_string()))
	}
}
