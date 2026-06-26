// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: MIT-0

// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is furnished to do
// so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::*;
pub(crate) use example_runtime::*;
use extensions::AuthorizeCoownership;
use pezframe_support::derive_impl;
use pezframe_system::{CheckEra, CheckGenesis, CheckNonce, CheckTxVersion};
use pezpallet_verify_signature::VerifySignature;
use pezsp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature, MultiSigner,
};

#[docify::export]
mod example_runtime {
	use super::*;

	/// Our `TransactionExtension` fit for general transactions.
	pub type TxExtension = (
		// Validate the signature of regular account transactions (substitutes the old signed
		// transaction).
		VerifySignature<Runtime>,
		// Nonce check (and increment) for the caller.
		CheckNonce<Runtime>,
		// If activated, will mutate the origin to a `pezpallet_coownership` origin of 2 accounts
		// that own something.
		AuthorizeCoownership<Runtime, MultiSigner, MultiSignature>,
		// Some other extensions that we want to run for every possible origin and we want captured
		// in any and all signature and authorization schemes (such as the traditional account
		// signature or the double signature in `pezpallet_coownership`).
		CheckGenesis<Runtime>,
		CheckTxVersion<Runtime>,
		CheckEra<Runtime>,
	);
	/// Convenience type to more easily construct the signature to be signed in case
	/// `AuthorizeCoownership` is activated.
	pub type InnerTxExtension = (CheckGenesis<Runtime>, CheckTxVersion<Runtime>, CheckEra<Runtime>);
	pub type UncheckedExtrinsic =
		generic::UncheckedExtrinsic<AccountId, RuntimeCall, Signature, TxExtension>;
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
	pub type Signature = MultiSignature;
	pub type BlockNumber = u32;

	// For testing the pezpallet, we construct a mock runtime.
	pezframe_support::construct_runtime!(
		pub enum Runtime
		{
			System: pezframe_system,
			VerifySignaturePallet: pezpallet_verify_signature,

			Assets: pezpallet_assets,
			Coownership: pezpallet_coownership,
		}
	);

	#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
	impl pezframe_system::Config for Runtime {
		type AccountId = AccountId;
		type Block = Block;
		type Lookup = IdentityLookup<Self::AccountId>;
	}

	impl pezpallet_verify_signature::Config for Runtime {
		type Signature = MultiSignature;
		type AccountIdentifier = MultiSigner;
		type WeightInfo = ();
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper = ();
	}

	/// Type that enables any pezpallet to ask for a coowner origin.
	pub struct EnsureCoowner;
	impl EnsureOrigin<RuntimeOrigin> for EnsureCoowner {
		type Success = (AccountId, AccountId);

		fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
			match o.clone().into() {
				Ok(pezpallet_coownership::Origin::<Runtime>::Coowners(first, second)) => {
					Ok((first, second))
				},
				_ => Err(o),
			}
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
			unimplemented!()
		}
	}

	impl pezpallet_assets::Config for Runtime {
		type CoownerOrigin = EnsureCoowner;
	}

	impl pezpallet_coownership::Config for Runtime {
		type RuntimeOrigin = RuntimeOrigin;
		type PalletsOrigin = OriginCaller;
	}
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let t = RuntimeGenesisConfig {
		// We use default for brevity, but you can configure as desired if needed.
		system: Default::default(),
	}
	.build_storage()
	.unwrap();
	t.into()
}
