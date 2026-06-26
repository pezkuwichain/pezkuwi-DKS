// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Utilities to build a `TestClient` for `pez-kitchensink-runtime`.

/// Re-export test-client utilities.
pub use bizinikiwi_test_client::*;
use pezsp_runtime::BuildStorage;

/// Call executor for `pez-kitchensink-runtime` `TestClient`.
use node_cli::service::RuntimeExecutor;

/// Default backend type.
pub type Backend = pezsc_client_db::Backend<pez_node_primitives::Block>;

/// Test client type.
pub type Client = client::Client<
	Backend,
	client::LocalCallExecutor<pez_node_primitives::Block, Backend, RuntimeExecutor>,
	pez_node_primitives::Block,
	pez_kitchensink_runtime::RuntimeApi,
>;

/// Genesis configuration parameters for `TestClient`.
#[derive(Default)]
pub struct GenesisParameters;

impl bizinikiwi_test_client::GenesisInit for GenesisParameters {
	fn genesis_storage(&self) -> Storage {
		let mut storage = crate::genesis::config().build_storage().unwrap();
		storage.top.insert(
			pezsp_core::storage::well_known_keys::CODE.to_vec(),
			pez_kitchensink_runtime::wasm_binary_unwrap().into(),
		);
		storage
	}
}

/// A `test-runtime` extensions to `TestClientBuilder`.
pub trait TestClientBuilderExt: Sized {
	/// Create test client builder.
	fn new() -> Self;

	/// Build the test client.
	fn build(self) -> Client;
}

impl TestClientBuilderExt
	for bizinikiwi_test_client::TestClientBuilder<
		pez_node_primitives::Block,
		client::LocalCallExecutor<pez_node_primitives::Block, Backend, RuntimeExecutor>,
		Backend,
		GenesisParameters,
	>
{
	fn new() -> Self {
		Self::default()
	}
	fn build(self) -> Client {
		let executor = RuntimeExecutor::builder().build();
		use pezsc_service::client::LocalCallExecutor;
		use std::sync::Arc;
		let executor = LocalCallExecutor::new(
			self.backend().clone(),
			executor.clone(),
			Default::default(),
			ExecutionExtensions::new(None, Arc::new(executor)),
		)
		.expect("Creates LocalCallExecutor");
		self.build_with_executor(executor).0
	}
}
