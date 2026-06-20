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

//! A fake runtime struct that allows us to instantiate a client.
//! Has all the required runtime APIs implemented to satisfy trait bounds,
//! but the methods are never called since we use WASM exclusively.

use pezsp_core::OpaqueMetadata;
use pezsp_runtime::{
	generic,
	traits::{BlakeTwo256, Block as BlockT},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, OpaqueExtrinsic,
};

/// Block number
#[allow(dead_code)]
type BlockNumber = u32;
/// Opaque block header type.
#[allow(dead_code)]
type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Opaque block type.
#[allow(dead_code)]
type Block = generic::Block<Header, OpaqueExtrinsic>;

#[allow(unused)]
pub struct Runtime;

pezsp_api::impl_runtime_apis! {
	impl pezsp_api::Core<Block> for Runtime {
		fn version() -> pezsp_version::RuntimeVersion {
			unimplemented!()
		}

		fn execute_block(_: <Block as BlockT>::LazyBlock) {
			unimplemented!()
		}

		fn initialize_block(_: &<Block as BlockT>::Header) -> pezsp_runtime::ExtrinsicInclusionMode {
			unimplemented!()
		}
	}

	impl pezsp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			unimplemented!()
		}

		fn metadata_at_version(_: u32) -> Option<OpaqueMetadata> {
			unimplemented!()
		}

		fn metadata_versions() -> Vec<u32> {
			unimplemented!()
		}
	}
	impl pezsp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(_: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			unimplemented!()
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			unimplemented!()
		}

		fn inherent_extrinsics(_: pezsp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			unimplemented!()
		}

		fn check_inherents(_: <Block as BlockT>::LazyBlock, _: pezsp_inherents::InherentData) -> pezsp_inherents::CheckInherentsResult {
			unimplemented!()
		}
	}

	impl pezsp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			_: TransactionSource,
			_: <Block as BlockT>::Extrinsic,
			_: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			unimplemented!()
		}
	}

	impl pezsp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(_: Vec<u8>) -> pezsp_genesis_builder::Result {
			unimplemented!()
		}

		fn get_preset(_id: &Option<pezsp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			unimplemented!()
		}

		fn preset_names() -> Vec<pezsp_genesis_builder::PresetId> {
			unimplemented!()
		}
	}
}
