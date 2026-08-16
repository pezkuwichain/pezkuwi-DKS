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
//! The generated subxt client.
//! Generated against a substrate chain configured with [`pezpallet_revive`] using:
//! subxt metadata  --url ws://localhost:9944 -o rpc/revive_chain.scale
pub use pezkuwi_subxt::config::PezkuwiConfig as SrcChainConfig;

#[pezkuwi_subxt::subxt(
	runtime_metadata_path = "$OUT_DIR/revive_chain.scale",
	// TODO remove once subxt use the same U256 type
	substitute_type(
		path = "primitive_types::U256",
		with = "::pezkuwi_subxt::utils::Static<::pezsp_core::U256>"
	),

	substitute_type(
		path = "pezsp_runtime::generic::block::Block<A, B, C, D, E>",
		with = "::pezkuwi_subxt::utils::Static<::pezsp_runtime::generic::Block<
		::pezsp_runtime::generic::Header<u32, pezsp_runtime::traits::BlakeTwo256>,
		::pezsp_runtime::OpaqueExtrinsic
		>>"
	),
	substitute_type(
		path = "pezpallet_revive::evm::api::debug_rpc_types::Trace",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::evm::Trace>"
	),
	substitute_type(
		path = "pezpallet_revive::evm::api::debug_rpc_types::TracerType",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::evm::TracerType>"
	),

	substitute_type(
		path = "pezpallet_revive::evm::api::rpc_types_gen::GenericTransaction",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::evm::GenericTransaction>"
	),
	substitute_type(
		path = "pezpallet_revive::evm::api::rpc_types::DryRunConfig<M>",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::evm::DryRunConfig<M>>"
	),
	substitute_type(
		path = "pezpallet_revive::evm::api::rpc_types::TracingConfig",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::evm::TracingConfig>"
	),
	substitute_type(
		path = "pezpallet_revive::primitives::EthTransactInfo<B>",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::EthTransactInfo<B>>"
	),
	substitute_type(
		path = "pezpallet_revive::primitives::EthTransactError",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::EthTransactError>"
	),
	substitute_type(
		path = "pezpallet_revive::primitives::ExecReturnValue",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::ExecReturnValue>"
	),
	substitute_type(
		path = "pezsp_weights::weight_v2::Weight",
		with = "::pezkuwi_subxt::utils::Static<::pezsp_weights::Weight>"
	),
	substitute_type(
		path = "pezpallet_revive::evm::api::rpc_types_gen::Block",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::evm::Block>"
	),
	substitute_type(
		path = "pezpallet_revive::evm::block_hash::ReceiptGasInfo",
		with = "::pezkuwi_subxt::utils::Static<::pezpallet_revive::evm::ReceiptGasInfo>"
	),
	derive_for_all_types = "codec::Encode, codec::Decode"
)]
mod src_chain {}
pub use src_chain::*;
