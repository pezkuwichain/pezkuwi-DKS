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

//! Runtime used in `your_first_runtime`.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::{vec, vec::Vec};
use first_pezpallet::pezpallet_v2 as our_first_pallet;
use pezframe::{deps::pezsp_genesis_builder::DEV_RUNTIME_PRESET, prelude::*, runtime::prelude::*};
use pezpallet_transaction_payment_rpc_runtime_api::{FeeDetails, RuntimeDispatchInfo};
use pezsp_keyring::Sr25519Keyring;
use pezsp_runtime::{
	traits::Block as BlockT,
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};

#[docify::export]
#[runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("first-runtime"),
	impl_name: alloc::borrow::Cow::Borrowed("first-runtime"),
	authoring_version: 1,
	spec_version: 0,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

#[docify::export(cr)]
construct_runtime!(
	pub struct Runtime {
		// Mandatory for all runtimes
		System: pezframe_system,

		// A number of other pallets from FRAME.
		Timestamp: pezpallet_timestamp,
		Balances: pezpallet_balances,
		Sudo: pezpallet_sudo,
		TransactionPayment: pezpallet_transaction_payment,

		// Our local pezpallet
		FirstPallet: our_first_pallet,
	}
);

#[docify::export_content]
mod runtime_types {
	use super::*;
	pub(super) type SignedExtra = (
		// `frame` already provides all the signed extensions from `pezframe-system`. We just add
		// the one related to tx-payment here.
		pezframe::runtime::types_common::SystemTransactionExtensionsOf<Runtime>,
		pezpallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	);

	pub(super) type Block = pezframe::runtime::types_common::BlockOf<Runtime, SignedExtra>;
	pub(super) type _Header = HeaderFor<Runtime>;

	pub(super) type RuntimeExecutive = Executive<
		Runtime,
		Block,
		pezframe_system::ChainContext<Runtime>,
		Runtime,
		AllPalletsWithSystem,
	>;
}
use runtime_types::*;

#[docify::export_content]
mod config_impls {
	use super::*;

	parameter_types! {
		pub const Version: RuntimeVersion = VERSION;
	}

	#[derive_impl(pezframe_system::config_preludes::SolochainDefaultConfig)]
	impl pezframe_system::Config for Runtime {
		type Block = Block;
		type Version = Version;
		type AccountData =
			pezpallet_balances::AccountData<<Runtime as pezpallet_balances::Config>::Balance>;
	}

	#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
	impl pezpallet_balances::Config for Runtime {
		type AccountStore = System;
	}

	#[derive_impl(pezpallet_sudo::config_preludes::TestDefaultConfig)]
	impl pezpallet_sudo::Config for Runtime {}

	#[derive_impl(pezpallet_timestamp::config_preludes::TestDefaultConfig)]
	impl pezpallet_timestamp::Config for Runtime {}

	#[derive_impl(pezpallet_transaction_payment::config_preludes::TestDefaultConfig)]
	impl pezpallet_transaction_payment::Config for Runtime {
		type OnChargeTransaction = pezpallet_transaction_payment::FungibleAdapter<Balances, ()>;
		// We specify a fixed length to fee here, which essentially means all transactions charge
		// exactly 1 unit of fee.
		type LengthToFee = FixedFee<1, <Self as pezpallet_balances::Config>::Balance>;
		type WeightToFee = NoFee<<Self as pezpallet_balances::Config>::Balance>;
	}
}

#[docify::export(our_config_impl)]
impl our_first_pallet::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

/// Provides getters for genesis configuration presets.
pub mod genesis_config_presets {
	use super::*;
	use crate::{
		interface::{Balance, MinimumBalance},
		BalancesConfig, RuntimeGenesisConfig, SudoConfig,
	};
	use pezframe::deps::pezframe_support::build_struct_json_patch;
	use serde_json::Value;

	/// Returns a development genesis config preset.
	#[docify::export]
	pub fn development_config_genesis() -> Value {
		let endowment = <MinimumBalance as Get<Balance>>::get().max(1) * 1000;
		build_struct_json_patch!(RuntimeGenesisConfig {
			balances: BalancesConfig {
				balances: Sr25519Keyring::iter()
					.map(|a| (a.to_account_id(), endowment))
					.collect::<Vec<_>>(),
			},
			sudo: SudoConfig { key: Some(Sr25519Keyring::Alice.to_account_id()) },
		})
	}

	/// Get the set of the available genesis config presets.
	#[docify::export]
	pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
		let patch = match id.as_ref() {
			DEV_RUNTIME_PRESET => development_config_genesis(),
			_ => return None,
		};
		Some(
			serde_json::to_string(&patch)
				.expect("serialization to json is expected to work. qed.")
				.into_bytes(),
		)
	}

	/// List of supported presets.
	#[docify::export]
	pub fn preset_names() -> Vec<PresetId> {
		vec![PresetId::from(DEV_RUNTIME_PRESET)]
	}
}

impl_runtime_apis! {
	impl pezsp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: <Block as BlockT>::LazyBlock) {
			RuntimeExecutive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode {
			RuntimeExecutive::initialize_block(header)
		}
	}

	impl pezsp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl pezsp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			RuntimeExecutive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			RuntimeExecutive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: <Block as BlockT>::LazyBlock,
			data: InherentData,
		) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl pezsp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			RuntimeExecutive::validate_transaction(source, tx, block_hash)
		}
	}

	impl pezsp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			RuntimeExecutive::offchain_worker(header)
		}
	}

	impl pezsp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(_seed: Option<Vec<u8>>) -> Vec<u8> {
			Default::default()
		}

		fn decode_session_keys(
			_encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, pezsp_core::crypto::KeyTypeId)>> {
			Default::default()
		}
	}

	impl pezframe_system_rpc_runtime_api::AccountNonceApi<Block, interface::AccountId, interface::Nonce> for Runtime {
		fn account_nonce(account: interface::AccountId) -> interface::Nonce {
			System::account_nonce(account)
		}
	}

	impl pezsp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> GenesisBuilderResult {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, self::genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<PresetId> {
			crate::genesis_config_presets::preset_names()
		}
	}

	impl pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		interface::Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<interface::Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<interface::Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> interface::Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> interface::Balance {
			TransactionPayment::length_to_fee(length)
		}
	}
}

/// Just a handy re-definition of some types based on what is already provided to the pezpallet
/// configs.
pub mod interface {
	use super::Runtime;
	use pezframe::prelude::pezframe_system;

	pub type AccountId = <Runtime as pezframe_system::Config>::AccountId;
	pub type Nonce = <Runtime as pezframe_system::Config>::Nonce;
	pub type Hash = <Runtime as pezframe_system::Config>::Hash;
	pub type Balance = <Runtime as pezpallet_balances::Config>::Balance;
	pub type MinimumBalance = <Runtime as pezpallet_balances::Config>::ExistentialDeposit;
}
