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

use crate::{
	ClientError, H160,
	pezkuwi_subxt_client::{
		self, SrcChainConfig,
		runtime_types::pezpallet_revive::storage::{AccountType, ContractInfo},
	},
};
use pezkuwi_subxt::{OnlineClient, error::StorageError, storage::StorageClientAt};

/// A wrapper around the Bizinikiwi Storage API.
#[derive(Clone)]
pub struct StorageApi(StorageClientAt<SrcChainConfig, OnlineClient<SrcChainConfig>>);

impl StorageApi {
	/// Create a new instance of the StorageApi.
	pub fn new(api: StorageClientAt<SrcChainConfig, OnlineClient<SrcChainConfig>>) -> Self {
		Self(api)
	}

	/// Get the contract info for the given contract address.
	pub async fn get_contract_info(
		&self,
		contract_address: &H160,
	) -> Result<ContractInfo, ClientError> {
		// TODO: remove once subxt is updated
		let contract_address: pezkuwi_subxt::utils::H160 = contract_address.0.into();

		// The key moved out of the address and into `fetch`, and a missing entry is now an
		// error variant rather than `None`.
		let query = pezkuwi_subxt_client::storage().revive().account_info_of().unvalidated();
		let info = match self.0.entry(query)?.fetch((contract_address,)).await {
			Ok(value) => value.decode()?,
			Err(StorageError::NoValueFound) => return Err(ClientError::ContractNotFound),
			Err(err) => return Err(err.into()),
		};

		let AccountType::Contract(contract_info) = info.account_type else {
			return Err(ClientError::ContractNotFound);
		};

		Ok(contract_info)
	}

	/// Get the contract trie id for the given contract address.
	pub async fn get_contract_trie_id(&self, address: &H160) -> Result<Vec<u8>, ClientError> {
		let ContractInfo { trie_id, .. } = self.get_contract_info(address).await?;
		Ok(trie_id.0)
	}
}
