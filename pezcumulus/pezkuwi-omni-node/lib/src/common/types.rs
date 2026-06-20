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

use pezcumulus_client_consensus_common::TeyrchainBlockImport as TTeyrchainBlockImport;
use pezcumulus_primitives_core::relay_chain::UncheckedExtrinsic;
use pezsc_consensus::DefaultImportQueue;
use pezsc_executor::WasmExecutor;
use pezsc_service::{PartialComponents, TFullBackend, TFullClient};
use pezsc_telemetry::{Telemetry, TelemetryWorkerHandle};
use pezsc_transaction_pool::TransactionPoolHandle;
use pezsp_runtime::{generic, traits::BlakeTwo256};

pub use teyrchains_common::{AccountId, Balance, Hash, Nonce};

type Header<BlockNumber> = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block<BlockNumber> = generic::Block<Header<BlockNumber>, UncheckedExtrinsic>;

#[cfg(not(feature = "runtime-benchmarks"))]
pub type TeyrchainHostFunctions = (
	pezcumulus_client_service::TeyrchainHostFunctions,
	pezsp_statement_store::runtime_api::HostFunctions,
);
#[cfg(feature = "runtime-benchmarks")]
pub type TeyrchainHostFunctions = (
	pezcumulus_client_service::TeyrchainHostFunctions,
	pezsp_statement_store::runtime_api::HostFunctions,
	pezframe_benchmarking::benchmarking::HostFunctions,
);

pub type TeyrchainClient<Block, RuntimeApi> =
	TFullClient<Block, RuntimeApi, WasmExecutor<TeyrchainHostFunctions>>;

pub type TeyrchainBackend<Block> = TFullBackend<Block>;

pub type TeyrchainBlockImport<Block, BI> =
	TTeyrchainBlockImport<Block, BI, TeyrchainBackend<Block>>;

/// Assembly of PartialComponents (enough to run chain ops subcommands)
pub type TeyrchainService<Block, RuntimeApi, BI, BIExtraReturnValue> = PartialComponents<
	TeyrchainClient<Block, RuntimeApi>,
	TeyrchainBackend<Block>,
	(),
	DefaultImportQueue<Block>,
	TransactionPoolHandle<Block, TeyrchainClient<Block, RuntimeApi>>,
	(
		TeyrchainBlockImport<Block, BI>,
		Option<Telemetry>,
		Option<TelemetryWorkerHandle>,
		BIExtraReturnValue,
	),
>;
