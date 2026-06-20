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

//! Mock setup for tests.

#![cfg(any(test, feature = "runtime-benchmarks"))]

use crate as pezpallet_meta_tx;
use crate::*;
use pezframe_support::{
	construct_runtime, derive_impl,
	weights::{FixedFee, NoFee},
};
use pezsp_core::ConstU8;
use pezsp_keystore::{testing::MemoryKeystore, KeystoreExt};
use pezsp_runtime::{
	traits::{IdentifyAccount, IdentityLookup, Verify},
	MultiSignature,
};

pub type Balance = u64;

pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

#[cfg(feature = "runtime-benchmarks")]
pub type MetaTxExtension = crate::benchmarking::types::WeightlessExtension<Runtime>;

#[cfg(not(feature = "runtime-benchmarks"))]
pub use tx_ext::*;

#[cfg(not(feature = "runtime-benchmarks"))]
mod tx_ext {
	use super::*;

	pub type UncheckedExtrinsic =
		pezsp_runtime::generic::UncheckedExtrinsic<AccountId, RuntimeCall, Signature, TxExtension>;

	/// Transaction extension.
	pub type TxExtension = (pezpallet_verify_signature::VerifySignature<Runtime>, TxBareExtension);

	/// Transaction extension without signature information.
	///
	/// Helper type used to decode the part of the extension which should be signed.
	pub type TxBareExtension = (
		pezframe_system::CheckNonZeroSender<Runtime>,
		pezframe_system::CheckSpecVersion<Runtime>,
		pezframe_system::CheckTxVersion<Runtime>,
		pezframe_system::CheckGenesis<Runtime>,
		pezframe_system::CheckMortality<Runtime>,
		pezframe_system::CheckNonce<Runtime>,
		pezframe_system::CheckWeight<Runtime>,
		pezpallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	);

	pub const META_EXTENSION_VERSION: ExtensionVersion = 0;

	/// Meta transaction extension.
	pub type MetaTxExtension =
		(pezpallet_verify_signature::VerifySignature<Runtime>, MetaTxBareExtension);

	/// Meta transaction extension without signature information.
	///
	/// Helper type used to decode the part of the extension which should be signed.
	pub type MetaTxBareExtension = (
		MetaTxMarker<Runtime>,
		pezframe_system::CheckNonZeroSender<Runtime>,
		pezframe_system::CheckSpecVersion<Runtime>,
		pezframe_system::CheckTxVersion<Runtime>,
		pezframe_system::CheckGenesis<Runtime>,
		pezframe_system::CheckMortality<Runtime>,
		pezframe_system::CheckNonce<Runtime>,
	);
}

impl Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Extension = MetaTxExtension;
}

impl pezpallet_verify_signature::Config for Runtime {
	type Signature = MultiSignature;
	type AccountIdentifier = <Signature as Verify>::Signer;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = pezframe_system::mocking::MockBlock<Runtime>;
	type AccountData =
		pezpallet_balances::AccountData<<Self as pezpallet_balances::Config>::Balance>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Runtime {
	type ReserveIdentifier = [u8; 8];
	type AccountStore = System;
}

pub const TX_FEE: u32 = 10;

impl pezpallet_transaction_payment::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pezpallet_transaction_payment::FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<1>;
	type WeightToFee = FixedFee<TX_FEE, Balance>;
	type LengthToFee = NoFee<Balance>;
	type FeeMultiplierUpdate = ();
}

construct_runtime!(
	pub enum Runtime {
		System: pezframe_system,
		Balances: pezpallet_balances,
		MetaTx: pezpallet_meta_tx,
		TxPayment: pezpallet_transaction_payment,
		VerifySignature: pezpallet_verify_signature,
	}
);

pub(crate) fn new_test_ext() -> pezsp_io::TestExternalities {
	let mut ext = pezsp_io::TestExternalities::new(Default::default());
	ext.execute_with(|| {
		pezframe_system::GenesisConfig::<Runtime>::default().build();
		System::set_block_number(1);
	});
	ext.register_extension(KeystoreExt::new(MemoryKeystore::new()));
	ext
}
