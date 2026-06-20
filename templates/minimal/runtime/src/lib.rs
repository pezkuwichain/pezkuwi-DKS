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

//! A minimal runtime that includes the template [`pezpallet`](`pezpallet_minimal_template`).

#![cfg_attr(not(feature = "std"), no_std)]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

extern crate alloc;

use alloc::vec::Vec;
use pezkuwi_sdk::{
	pezframe::{
		deps::pezsp_genesis_builder,
		runtime::{apis, prelude::*},
	},
	*,
};
use pezpallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};

/// Provides getters for genesis configuration presets.
pub mod genesis_config_presets {
	use super::*;
	use crate::{
		interface::{Balance, MinimumBalance},
		pezsp_keyring::Sr25519Keyring,
		BalancesConfig, RuntimeGenesisConfig, SudoConfig,
	};

	use alloc::{vec, vec::Vec};
	use serde_json::Value;

	/// Returns a development genesis config preset.
	pub fn development_config_genesis() -> Value {
		let endowment = <MinimumBalance as Get<Balance>>::get().max(1) * 1000;
		pezframe_support::build_struct_json_patch!(RuntimeGenesisConfig {
			balances: BalancesConfig {
				balances: Sr25519Keyring::iter()
					.map(|a| (a.to_account_id(), endowment))
					.collect::<Vec<_>>(),
			},
			sudo: SudoConfig { key: Some(Sr25519Keyring::Alice.to_account_id()) },
		})
	}

	/// Get the set of the available genesis config presets.
	pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
		let patch = match id.as_ref() {
			pezsp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
			_ => return None,
		};
		Some(
			serde_json::to_string(&patch)
				.expect("serialization to json is expected to work. qed.")
				.into_bytes(),
		)
	}

	/// List of supported presets.
	pub fn preset_names() -> Vec<PresetId> {
		vec![PresetId::from(pezsp_genesis_builder::DEV_RUNTIME_PRESET)]
	}
}

/// The runtime version.
#[runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("pez-minimal-template-runtime"),
	impl_name: alloc::borrow::Cow::Borrowed("pez-minimal-template-runtime"),
	authoring_version: 1,
	spec_version: 0,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// The transaction extensions that are added to the runtime.
type TxExtension = (
	// Authorize calls that validate themselves.
	pezframe_system::AuthorizeCall<Runtime>,
	// Checks that the sender is not the zero address.
	pezframe_system::CheckNonZeroSender<Runtime>,
	// Checks that the runtime version is correct.
	pezframe_system::CheckSpecVersion<Runtime>,
	// Checks that the transaction version is correct.
	pezframe_system::CheckTxVersion<Runtime>,
	// Checks that the genesis hash is correct.
	pezframe_system::CheckGenesis<Runtime>,
	// Checks that the era is valid.
	pezframe_system::CheckEra<Runtime>,
	// Checks that the nonce is valid.
	pezframe_system::CheckNonce<Runtime>,
	// Checks that the weight is valid.
	pezframe_system::CheckWeight<Runtime>,
	// Ensures that the sender has enough funds to pay for the transaction
	// and deducts the fee from the sender's account.
	pezpallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	// Reclaim the unused weight from the block using post dispatch information.
	// It must be last in the pipeline in order to catch the refund in previous transaction
	// extensions
	pezframe_system::WeightReclaim<Runtime>,
);

// Composes the runtime by adding all the used pallets and deriving necessary types.
#[frame_construct_runtime]
mod runtime {
	/// The main runtime type.
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Runtime;

	/// Mandatory system pezpallet that should always be included in a FRAME runtime.
	#[runtime::pezpallet_index(0)]
	pub type System = pezframe_system::Pezpallet<Runtime>;

	/// Provides a way for consensus systems to set and check the onchain time.
	#[runtime::pezpallet_index(1)]
	pub type Timestamp = pezpallet_timestamp::Pezpallet<Runtime>;

	/// Provides the ability to keep track of balances.
	#[runtime::pezpallet_index(2)]
	pub type Balances = pezpallet_balances::Pezpallet<Runtime>;

	/// Provides a way to execute privileged functions.
	#[runtime::pezpallet_index(3)]
	pub type Sudo = pezpallet_sudo::Pezpallet<Runtime>;

	/// Provides the ability to charge for extrinsic execution.
	#[runtime::pezpallet_index(4)]
	pub type TransactionPayment = pezpallet_transaction_payment::Pezpallet<Runtime>;

	/// A minimal pezpallet template.
	#[runtime::pezpallet_index(5)]
	pub type Template = pezpallet_minimal_template::Pezpallet<Runtime>;
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
}

/// Implements the types required for the system pezpallet.
#[derive_impl(pezframe_system::config_preludes::SolochainDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type Block = Block;
	type Version = Version;
	// Use the account data from the balances pezpallet
	type AccountData =
		pezpallet_balances::AccountData<<Runtime as pezpallet_balances::Config>::Balance>;
}

// Implements the types required for the balances pezpallet.
#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Runtime {
	type AccountStore = System;
}

// Implements the types required for the sudo pezpallet.
#[derive_impl(pezpallet_sudo::config_preludes::TestDefaultConfig)]
impl pezpallet_sudo::Config for Runtime {}

// Implements the types required for the sudo pezpallet.
#[derive_impl(pezpallet_timestamp::config_preludes::TestDefaultConfig)]
impl pezpallet_timestamp::Config for Runtime {}

// Implements the types required for the transaction payment pezpallet.
#[derive_impl(pezpallet_transaction_payment::config_preludes::TestDefaultConfig)]
impl pezpallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = pezpallet_transaction_payment::FungibleAdapter<Balances, ()>;
	// Setting fee as independent of the weight of the extrinsic for demo purposes
	type WeightToFee = NoFee<<Self as pezpallet_balances::Config>::Balance>;
	// Setting fee as fixed for any length of the call data for demo purposes
	type LengthToFee = FixedFee<1, <Self as pezpallet_balances::Config>::Balance>;
}

// Implements the types required for the template pezpallet.
impl pezpallet_minimal_template::Config for Runtime {}

type Block = pezframe::runtime::types_common::BlockOf<Runtime, TxExtension>;
type Header = HeaderFor<Runtime>;

type RuntimeExecutive = Executive<
	Runtime,
	Block,
	pezframe_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

impl_runtime_apis! {
	impl apis::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: <Block as pezframe::traits::Block>::LazyBlock) {
			RuntimeExecutive::execute_block(block)
		}

		fn initialize_block(header: &Header) -> ExtrinsicInclusionMode {
			RuntimeExecutive::initialize_block(header)
		}
	}
	impl apis::Metadata<Block> for Runtime {
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

	impl apis::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: ExtrinsicFor<Runtime>) -> ApplyExtrinsicResult {
			RuntimeExecutive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> HeaderFor<Runtime> {
			RuntimeExecutive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<ExtrinsicFor<Runtime>> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: <Block as pezframe::traits::Block>::LazyBlock,
			data: InherentData,
		) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl apis::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: ExtrinsicFor<Runtime>,
			block_hash: <Runtime as pezframe_system::Config>::Hash,
		) -> TransactionValidity {
			RuntimeExecutive::validate_transaction(source, tx, block_hash)
		}
	}

	impl apis::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &HeaderFor<Runtime>) {
			RuntimeExecutive::offchain_worker(header)
		}
	}

	impl apis::SessionKeys<Block> for Runtime {
		fn generate_session_keys(_seed: Option<Vec<u8>>) -> Vec<u8> {
			Default::default()
		}

		fn decode_session_keys(
			_encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, apis::KeyTypeId)>> {
			Default::default()
		}
	}

	impl apis::AccountNonceApi<Block, interface::AccountId, interface::Nonce> for Runtime {
		fn account_nonce(account: interface::AccountId) -> interface::Nonce {
			System::account_nonce(account)
		}
	}

	impl pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		interface::Balance,
	> for Runtime {
		fn query_info(uxt: ExtrinsicFor<Runtime>, len: u32) -> RuntimeDispatchInfo<interface::Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: ExtrinsicFor<Runtime>, len: u32) -> FeeDetails<interface::Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> interface::Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> interface::Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl apis::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> pezsp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, self::genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<PresetId> {
			self::genesis_config_presets::preset_names()
		}
	}
}

/// Some re-exports that the node side code needs to know. Some are useful in this context as well.
///
/// Other types should preferably be private.
// TODO: this should be standardized in some way, see:
// https://github.com/pezkuwichain/pezkuwi-sdk/issues/3
pub mod interface {
	use super::Runtime;
	use pezkuwi_sdk::*;

	pub type Block = super::Block;
	pub use pezframe::runtime::types_common::OpaqueBlock;
	pub type AccountId = <Runtime as pezframe_system::Config>::AccountId;
	pub type Nonce = <Runtime as pezframe_system::Config>::Nonce;
	pub type Hash = <Runtime as pezframe_system::Config>::Hash;
	pub type Balance = <Runtime as pezpallet_balances::Config>::Balance;
	pub type MinimumBalance = <Runtime as pezpallet_balances::Config>::ExistentialDeposit;
}
