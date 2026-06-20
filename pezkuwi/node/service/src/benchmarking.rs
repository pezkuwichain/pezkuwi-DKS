// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! Code related to benchmarking a node.

use pezkuwi_primitives::AccountId;
use pezsc_client_api::UsageProvider;
use pezsp_keyring::Sr25519Keyring;
use pezsp_runtime::OpaqueExtrinsic;

use crate::*;

macro_rules! identify_chain {
	(
		$chain:expr,
		$nonce:ident,
		$current_block:ident,
		$period:ident,
		$genesis:ident,
		$signer:ident,
		$generic_code:expr $(,)*
	) => {
		match $chain {
			Chain::Pezkuwi => Err("Pezkuwi runtimes are currently not supported"),
			Chain::Dicle => Err("Dicle runtimes are currently not supported"),
			Chain::Pezkuwichain => {
				#[cfg(feature = "pezkuwichain-native")]
				{
					use pezkuwichain_runtime as runtime;

					let call = $generic_code;

					Ok(pezkuwichain_sign_call(
						call,
						$nonce,
						$current_block,
						$period,
						$genesis,
						$signer,
					))
				}

				#[cfg(not(feature = "pezkuwichain-native"))]
				{
					Err("`pezkuwichain-native` feature not enabled")
				}
			},
			Chain::Zagros => {
				#[cfg(feature = "zagros-native")]
				{
					use zagros_runtime as runtime;

					let call = $generic_code;

					Ok(zagros_sign_call(call, $nonce, $current_block, $period, $genesis, $signer))
				}

				#[cfg(not(feature = "zagros-native"))]
				{
					Err("`zagros-native` feature not enabled")
				}
			},
			Chain::Unknown => {
				let _ = $nonce;
				let _ = $current_block;
				let _ = $period;
				let _ = $genesis;
				let _ = $signer;

				Err("Unknown chain")
			},
		}
	};
}

/// Generates `Balances::TransferKeepAlive` extrinsics for the benchmarks.
///
/// Note: Should only be used for benchmarking.
pub struct TransferKeepAliveBuilder {
	client: Arc<FullClient>,
	dest: AccountId,
	chain: Chain,
}

impl TransferKeepAliveBuilder {
	/// Creates a new [`Self`] from the given client and the arguments for the extrinsics.
	pub fn new(client: Arc<FullClient>, dest: AccountId, chain: Chain) -> Self {
		Self { client, dest, chain }
	}
}

impl pezframe_benchmarking_cli::ExtrinsicBuilder for TransferKeepAliveBuilder {
	fn pezpallet(&self) -> &str {
		"balances"
	}

	fn extrinsic(&self) -> &str {
		"transfer_keep_alive"
	}

	fn build(&self, nonce: u32) -> std::result::Result<OpaqueExtrinsic, &'static str> {
		let signer = Sr25519Keyring::Bob.pair();
		// We apply the extrinsic directly, so let's take some random period.
		let period = 128;
		let genesis = self.client.usage_info().chain.best_hash;
		let current_block = 0;
		let _dest = self.dest.clone();

		identify_chain! {
			self.chain,
			nonce,
			current_block,
			period,
			genesis,
			signer,
			{
				runtime::RuntimeCall::Balances(runtime::BalancesCall::transfer_keep_alive {
					dest: _dest.into(),
					value: runtime::ExistentialDeposit::get(),
				})
			},
		}
	}
}

#[cfg(feature = "zagros-native")]
fn zagros_sign_call(
	call: zagros_runtime::RuntimeCall,
	nonce: u32,
	current_block: u64,
	period: u64,
	genesis: pezsp_core::H256,
	acc: pezsp_core::sr25519::Pair,
) -> OpaqueExtrinsic {
	use codec::Encode;
	use pezsp_core::Pair;
	use zagros_runtime as runtime;

	let tx_ext: runtime::TxExtension = (
		pezframe_system::AuthorizeCall::<runtime::Runtime>::new(),
		pezframe_system::CheckNonZeroSender::<runtime::Runtime>::new(),
		pezframe_system::CheckSpecVersion::<runtime::Runtime>::new(),
		pezframe_system::CheckTxVersion::<runtime::Runtime>::new(),
		pezframe_system::CheckGenesis::<runtime::Runtime>::new(),
		pezframe_system::CheckMortality::<runtime::Runtime>::from(
			pezsp_runtime::generic::Era::mortal(period, current_block),
		),
		pezframe_system::CheckNonce::<runtime::Runtime>::from(nonce),
		pezframe_system::CheckWeight::<runtime::Runtime>::new(),
		pezpallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from(0),
		pezframe_metadata_hash_extension::CheckMetadataHash::<runtime::Runtime>::new(false),
		pezframe_system::WeightReclaim::<runtime::Runtime>::new(),
	)
		.into();

	let payload = runtime::SignedPayload::from_raw(
		call.clone(),
		tx_ext.clone(),
		(
			(),
			(),
			runtime::VERSION.spec_version,
			runtime::VERSION.transaction_version,
			genesis,
			genesis,
			(),
			(),
			(),
			None,
			(),
		),
	);

	let signature = payload.using_encoded(|p| acc.sign(p));
	runtime::UncheckedExtrinsic::new_signed(
		call,
		pezsp_runtime::AccountId32::from(acc.public()).into(),
		pezkuwi_core_primitives::Signature::Sr25519(signature),
		tx_ext,
	)
	.into()
}

#[cfg(feature = "pezkuwichain-native")]
fn pezkuwichain_sign_call(
	call: pezkuwichain_runtime::RuntimeCall,
	nonce: u32,
	current_block: u64,
	period: u64,
	genesis: pezsp_core::H256,
	acc: pezsp_core::sr25519::Pair,
) -> OpaqueExtrinsic {
	use codec::Encode;
	use pezkuwichain_runtime as runtime;
	use pezsp_core::Pair;

	let tx_ext: runtime::TxExtension = (
		pezframe_system::AuthorizeCall::<runtime::Runtime>::new(),
		pezframe_system::CheckNonZeroSender::<runtime::Runtime>::new(),
		pezframe_system::CheckSpecVersion::<runtime::Runtime>::new(),
		pezframe_system::CheckTxVersion::<runtime::Runtime>::new(),
		pezframe_system::CheckGenesis::<runtime::Runtime>::new(),
		pezframe_system::CheckMortality::<runtime::Runtime>::from(
			pezsp_runtime::generic::Era::mortal(period, current_block),
		),
		pezframe_system::CheckNonce::<runtime::Runtime>::from(nonce),
		pezframe_system::CheckWeight::<runtime::Runtime>::new(),
		pezpallet_transaction_payment::ChargeTransactionPayment::<runtime::Runtime>::from(0),
		pezframe_metadata_hash_extension::CheckMetadataHash::<runtime::Runtime>::new(false),
		pezframe_system::WeightReclaim::<runtime::Runtime>::new(),
	)
		.into();

	let payload = runtime::SignedPayload::from_raw(
		call.clone(),
		tx_ext.clone(),
		(
			(),
			(),
			runtime::VERSION.spec_version,
			runtime::VERSION.transaction_version,
			genesis,
			genesis,
			(),
			(),
			(),
			None,
			(),
		),
	);

	let signature = payload.using_encoded(|p| acc.sign(p));
	runtime::UncheckedExtrinsic::new_signed(
		call,
		pezsp_runtime::AccountId32::from(acc.public()).into(),
		pezkuwi_core_primitives::Signature::Sr25519(signature),
		tx_ext,
	)
	.into()
}

/// Generates inherent data for benchmarking Pezkuwi, Dicle, Zagros and Pezkuwichain.
///
/// Not to be used outside of benchmarking since it returns mocked values.
pub fn benchmark_inherent_data(
	header: pezkuwi_core_primitives::Header,
) -> std::result::Result<pezsp_inherents::InherentData, pezsp_inherents::Error> {
	use pezsp_inherents::InherentDataProvider;
	let mut inherent_data = pezsp_inherents::InherentData::new();

	// Assume that all runtimes have the `timestamp` pezpallet.
	let d = std::time::Duration::from_millis(0);
	let timestamp = pezsp_timestamp::InherentDataProvider::new(d.into());
	futures::executor::block_on(timestamp.provide_inherent_data(&mut inherent_data))?;

	let para_data = pezkuwi_primitives::InherentData {
		bitfields: Vec::new(),
		backed_candidates: Vec::new(),
		disputes: Vec::new(),
		parent_header: header,
	};

	inherent_data.put_data(pezkuwi_primitives::TEYRCHAINS_INHERENT_IDENTIFIER, &para_data)?;

	Ok(inherent_data)
}
