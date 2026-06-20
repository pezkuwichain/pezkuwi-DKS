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

use crate::CheckMetadataHash;
use bizinikiwi_test_runtime_client::{
	prelude::*,
	runtime::{self, ExtrinsicBuilder},
	DefaultTestClientBuilderExt, TestClientBuilder,
};
use codec::{Decode, Encode};
use frame_metadata::RuntimeMetadataPrefixed;
use merkleized_metadata::{generate_metadata_digest, ExtraInfo};
use pezframe_support::{
	derive_impl,
	pezpallet_prelude::{InvalidTransaction, TransactionValidityError},
};
use pezsp_api::{Metadata, ProvideRuntimeApi};
use pezsp_runtime::{
	traits::{ExtrinsicLike, TransactionExtension},
	transaction_validity::{TransactionSource, UnknownTransaction},
};
use pezsp_transaction_pool::runtime_api::TaggedTransactionQueue;

type Block = pezframe_system::mocking::MockBlock<Test>;

pezframe_support::construct_runtime! {
	pub enum Test {
		System: pezframe_system,
	}
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type Block = Block;
}

#[test]
fn rejects_when_no_metadata_hash_was_passed() {
	let ext = CheckMetadataHash::<Test>::decode(&mut &1u8.encode()[..]).unwrap();
	assert_eq!(Err(UnknownTransaction::CannotLookup.into()), ext.implicit());
}

#[test]
fn rejects_unknown_mode() {
	assert!(CheckMetadataHash::<Test>::decode(&mut &50u8.encode()[..]).is_err());
}

/// Generate the metadata hash for the `test-runtime`.
fn generate_metadata_hash(metadata: RuntimeMetadataPrefixed) -> [u8; 32] {
	let runtime_version = runtime::VERSION;
	let base58_prefix = 0;

	let extra_info = ExtraInfo {
		spec_version: runtime_version.spec_version,
		spec_name: runtime_version.spec_name.into(),
		base58_prefix,
		decimals: 10,
		token_symbol: "TOKEN".into(),
	};

	generate_metadata_digest(&metadata.1, extra_info).unwrap().hash()
}

#[test]
fn ensure_check_metadata_works_on_real_extrinsics() {
	pezsp_tracing::try_init_simple();

	let client = TestClientBuilder::new().build();
	let runtime_api = client.runtime_api();
	let best_hash = client.chain_info().best_hash;

	let metadata = RuntimeMetadataPrefixed::decode(
		&mut &runtime_api.metadata_at_version(best_hash, 15).unwrap().unwrap()[..],
	)
	.unwrap();

	let valid_transaction = ExtrinsicBuilder::new_include_data(vec![1, 2, 3])
		.metadata_hash(generate_metadata_hash(metadata))
		.build();
	// Ensure that the transaction is signed.
	assert!(!valid_transaction.is_bare());

	runtime_api
		.validate_transaction(best_hash, TransactionSource::External, valid_transaction, best_hash)
		.unwrap()
		.unwrap();

	// Including some random metadata hash should make the transaction invalid.
	let invalid_transaction = ExtrinsicBuilder::new_include_data(vec![1, 2, 3])
		.metadata_hash([10u8; 32])
		.build();
	// Ensure that the transaction is signed.
	assert!(!invalid_transaction.is_bare());

	assert_eq!(
		TransactionValidityError::from(InvalidTransaction::BadProof),
		runtime_api
			.validate_transaction(
				best_hash,
				TransactionSource::External,
				invalid_transaction,
				best_hash
			)
			.unwrap()
			.unwrap_err()
	);
}

#[allow(unused)]
mod docs {
	use super::*;

	#[docify::export]
	mod add_metadata_hash_extension {
		pezframe_support::construct_runtime! {
			pub enum Runtime {
				System: pezframe_system,
			}
		}

		/// The `TransactionExtension` to the basic transaction logic.
		pub type TxExtension = (
			pezframe_system::AuthorizeCall<Runtime>,
			pezframe_system::CheckNonZeroSender<Runtime>,
			pezframe_system::CheckSpecVersion<Runtime>,
			pezframe_system::CheckTxVersion<Runtime>,
			pezframe_system::CheckGenesis<Runtime>,
			pezframe_system::CheckMortality<Runtime>,
			pezframe_system::CheckNonce<Runtime>,
			pezframe_system::CheckWeight<Runtime>,
			// Add the `CheckMetadataHash` extension.
			// The position in this list is not important, so we could also add it to beginning.
			pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
			pezframe_system::WeightReclaim<Runtime>,
		);

		/// In your runtime this will be your real address type.
		type Address = ();
		/// In your runtime this will be your real signature type.
		type Signature = ();

		/// Unchecked extrinsic type as expected by this runtime.
		pub type UncheckedExtrinsic = pezsp_runtime::generic::UncheckedExtrinsic<
			Address,
			RuntimeCall,
			Signature,
			TxExtension,
		>;
	}

	// Put here to not have it in the docs as well.
	#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
	impl pezframe_system::Config for add_metadata_hash_extension::Runtime {
		type Block = Block;
		type RuntimeEvent = add_metadata_hash_extension::RuntimeEvent;
		type RuntimeOrigin = add_metadata_hash_extension::RuntimeOrigin;
		type RuntimeCall = add_metadata_hash_extension::RuntimeCall;
		type PalletInfo = add_metadata_hash_extension::PalletInfo;
	}

	/// This function demonstrates how to enable metadata hash in WasmBuilder.
	/// It is only available when the `metadata-hash` feature is enabled on `bizinikiwi-wasm-builder`.
	#[cfg(feature = "metadata-hash")]
	#[docify::export]
	fn enable_metadata_hash_in_wasm_builder() {
		bizinikiwi_wasm_builder::WasmBuilder::init_with_defaults()
			// Requires the `metadata-hash` feature to be activated.
			// You need to pass the main token symbol and its number of decimals.
			.enable_metadata_hash("TOKEN", 12)
			// The runtime will be build twice and the second time the `RUNTIME_METADATA_HASH`
			// environment variable will be set for the `CheckMetadataHash` extension.
			.build()
	}
}
