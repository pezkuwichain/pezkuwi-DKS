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

pub(crate) mod imports {
	pub use pezcumulus_primitives_core::ParaId;
	pub use pezsp_core::{crypto::KeyTypeId, OpaqueMetadata};
	pub use pezsp_runtime::{
		traits::Block as BlockT,
		transaction_validity::{TransactionSource, TransactionValidity},
		ApplyExtrinsicResult,
	};
	pub use pezsp_weights::Weight;
	pub use teyrchains_common::{AccountId, Balance, Nonce};
}

macro_rules! impl_node_runtime_apis {
	($runtime: ty, $block: tt, $aura_id: ty) => {
		pezsp_api::impl_runtime_apis! {
			impl pezsp_api::Core<$block> for $runtime {
				fn version() -> pezsp_version::RuntimeVersion {
					unimplemented!()
				}

				fn execute_block(_: <$block as BlockT>::LazyBlock) {
					unimplemented!()
				}

				fn initialize_block(
					_: &<$block as BlockT>::Header
				) -> pezsp_runtime::ExtrinsicInclusionMode {
					unimplemented!()
				}
			}

			impl pezsp_api::Metadata<$block> for $runtime {
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

			impl pezcumulus_primitives_core::RelayParentOffsetApi<$block> for $runtime {
				fn relay_parent_offset() -> u32 {
					unimplemented!()
				}
			}

			impl pezsp_consensus_aura::AuraApi<$block, $aura_id> for $runtime {
				fn slot_duration() -> pezsp_consensus_aura::SlotDuration {
					unimplemented!()
				}

				fn authorities() -> Vec<$aura_id> {
					unimplemented!()
				}
			}

			impl pezcumulus_primitives_aura::AuraUnincludedSegmentApi<$block> for $runtime {
				fn can_build_upon(
					_: <$block as BlockT>::Hash,
					_: pezcumulus_primitives_aura::Slot,
				) -> bool {
					unimplemented!()
				}
			}

			impl pezsp_block_builder::BlockBuilder<$block> for $runtime {
				fn apply_extrinsic(_: <$block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
					unimplemented!()
				}

				fn finalize_block() -> <$block as BlockT>::Header {
					unimplemented!()
				}

				fn inherent_extrinsics(
					_: pezsp_inherents::InherentData
				) -> Vec<<$block as BlockT>::Extrinsic> {
					unimplemented!()
				}

				fn check_inherents(
					_: <$block as BlockT>::LazyBlock,
					_: pezsp_inherents::InherentData
				) -> pezsp_inherents::CheckInherentsResult {
					unimplemented!()
				}
			}

			impl pezsp_transaction_pool::runtime_api::TaggedTransactionQueue<$block> for $runtime {
				fn validate_transaction(
					_: TransactionSource,
					_: <$block as BlockT>::Extrinsic,
					_: <$block as BlockT>::Hash,
				) -> TransactionValidity {
					unimplemented!()
				}
			}

			impl pezsp_offchain::OffchainWorkerApi<$block> for $runtime {
				fn offchain_worker(_: &<$block as BlockT>::Header) {
					unimplemented!()
				}
			}

			impl pezsp_session::SessionKeys<$block> for $runtime {
				fn generate_session_keys(_: Option<Vec<u8>>) -> Vec<u8> {
					unimplemented!()
				}

				fn decode_session_keys(
					_: Vec<u8>,
				) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
					unimplemented!()
				}
			}

			impl
				pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
					$block,
					Balance,
				> for $runtime
			{
				fn query_info(
					_: <$block as BlockT>::Extrinsic,
					_: u32,
				) -> pezpallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
					unimplemented!()
				}
				fn query_fee_details(
					_: <$block as BlockT>::Extrinsic,
					_: u32,
				) -> pezpallet_transaction_payment::FeeDetails<Balance> {
					unimplemented!()
				}
				fn query_weight_to_fee(_: Weight) -> Balance {
					unimplemented!()
				}
				fn query_length_to_fee(_: u32) -> Balance {
					unimplemented!()
				}
			}

			impl pezcumulus_primitives_core::CollectCollationInfo<$block> for $runtime {
				fn collect_collation_info(
					_: &<$block as BlockT>::Header
				) -> pezcumulus_primitives_core::CollationInfo {
					unimplemented!()
				}
			}

			impl pezcumulus_primitives_core::GetTeyrchainInfo<$block> for $runtime {
				fn teyrchain_id() -> ParaId {
					unimplemented!()
				}
			}
			#[cfg(feature = "try-runtime")]
			impl pezframe_try_runtime::TryRuntime<$block> for $runtime {
				fn on_runtime_upgrade(
					_: pezframe_try_runtime::UpgradeCheckSelect
				) -> (Weight, Weight) {
					unimplemented!()
				}

				fn execute_block(
					_: <$block as BlockT>::LazyBlock,
					_: bool,
					_: bool,
					_: pezframe_try_runtime::TryStateSelect,
				) -> Weight {
					unimplemented!()
				}
			}

			impl pezframe_system_rpc_runtime_api::AccountNonceApi<
				$block,
				AccountId,
				Nonce
			> for $runtime {
				fn account_nonce(_: AccountId) -> Nonce {
					unimplemented!()
				}
			}

			#[cfg(feature = "runtime-benchmarks")]
			impl pezframe_benchmarking::Benchmark<$block> for $runtime {
				fn benchmark_metadata(_: bool) -> (
					Vec<pezframe_benchmarking::BenchmarkList>,
					Vec<pezframe_support::traits::StorageInfo>,
				) {
					unimplemented!()
				}

				#[allow(non_local_definitions)]
				fn dispatch_benchmark(
					_: pezframe_benchmarking::BenchmarkConfig
				) -> Result<Vec<pezframe_benchmarking::BenchmarkBatch>, String> {
					unimplemented!()
				}
			}

			impl pezsp_genesis_builder::GenesisBuilder<$block> for $runtime {
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

			impl pezsp_statement_store::runtime_api::ValidateStatement<$block> for $runtime {
				fn validate_statement(
					_source: pezsp_statement_store::runtime_api::StatementSource,
					_statement: pezsp_statement_store::Statement,
				) -> Result<pezsp_statement_store::runtime_api::ValidStatement, pezsp_statement_store::runtime_api::InvalidStatement> {
					unimplemented!()
				}
			}

			impl pezcumulus_primitives_core::TargetBlockRate<$block> for $runtime {
				fn target_block_rate() -> u32 {
					unimplemented!()
				}
			}
		}
	};
}

pub(crate) use impl_node_runtime_apis;
