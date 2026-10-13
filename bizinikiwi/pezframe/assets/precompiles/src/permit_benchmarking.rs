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

//! Benchmarks for `pezpallet_assets_precompiles::permit`.
//!
//! Registered under `permit::Pezpallet` and bounded only by the permit pallet's own `Config`,
//! so a runtime registers them as `[pezpallet_assets_precompiles::permit, ...]` and the run
//! writes `permit_weights.rs`. They do not need the foreign-asset mapping pallet.

#![cfg(feature = "runtime-benchmarks")]

use crate::permit::pezpallet::{Config, Pezpallet};
use pezframe_benchmarking::v2::*;
use pezpallet_revive::precompiles::H160;
use pezsp_core::U256;

/// Test owner address (Hardhat account #0: 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266)
const TEST_OWNER: [u8; 20] = [
	0xf3, 0x9f, 0xd6, 0xe5, 0x1a, 0xad, 0x88, 0xf6, 0xf4, 0xce, 0x6a, 0xb8, 0x82, 0x72, 0x79, 0xcf,
	0xff, 0xb9, 0x22, 0x66,
];

fn test_verifying_contract() -> H160 {
	H160::from_low_u64_be(0x1234_5678)
}

fn test_owner() -> H160 {
	H160::from_slice(&TEST_OWNER)
}

/// Test token name for EIP-712 domain separator.
const TEST_TOKEN_NAME: &[u8] = b"Asset Permit";

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn nonces() {
		let verifying_contract = test_verifying_contract();
		let owner = test_owner();
		crate::permit::Nonces::<T>::insert(&verifying_contract, &owner, U256::from(42));

		let result;
		#[block]
		{
			result = Pezpallet::<T>::nonce(&verifying_contract, &owner);
		}
		assert_eq!(result, U256::from(42));
	}

	#[benchmark]
	fn domain_separator() {
		let verifying_contract = test_verifying_contract();
		let name = TEST_TOKEN_NAME;

		let result;
		#[block]
		{
			result = Pezpallet::<T>::compute_domain_separator(&verifying_contract, name);
		}
		assert_ne!(result, pezsp_core::H256::zero());
	}

	/// Benchmark for `use_permit` — the EIP-2612 signature verification and nonce
	/// increment that is called inside the `permit` precompile.
	///
	/// Measures: domain separator computation, struct hash, ECDSA recovery, nonce
	/// read + write. This is the weight charged for the cryptographic portion of
	/// a permit call (the asset approval weights are tracked separately).
	#[benchmark]
	fn use_permit() {
		let owner = test_owner();
		let spender = H160::from_low_u64_be(0x9876_5432);
		let verifying_contract = test_verifying_contract();
		let value: [u8; 32] = {
			let mut buf = [0u8; 32];
			buf[31] = 100; // value = 100
			buf
		};
		let deadline: [u8; 32] = {
			let mut buf = [0u8; 32];
			// Set deadline far in the future (max u64)
			buf[24..32].copy_from_slice(&u64::MAX.to_be_bytes());
			buf
		};

		// Set timestamp so deadline check passes.
		// Write directly to storage instead of calling `set_timestamp()`, which
		// triggers `OnTimestampSet` callbacks (e.g. pezpallet_babe slot validation)
		// that require additional setup not relevant to this benchmark.
		let timestamp: <T as pezpallet_timestamp::Config>::Moment =
			1_704_067_200_000u64.try_into().unwrap_or_default();
		pezpallet_timestamp::Now::<T>::put(timestamp);

		// Compute EIP-712 digest using runtime's chain_id.
		let nonce = U256::zero();
		let digest = Pezpallet::<T>::permit_digest(
			&verifying_contract,
			TEST_TOKEN_NAME,
			&owner,
			&spender,
			&value,
			&nonce,
			&deadline,
		);

		// Sign with Hardhat account #0 private key via pezsp_io host function.
		// This works in both native and WASM benchmark environments, unlike
		// using k256 directly which may not work in the WASM sandbox.
		let key_type = pezsp_core::crypto::KeyTypeId(*b"prmt");
		let pub_key = pezsp_io::crypto::ecdsa_generate(
			key_type,
			Some(b"0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_vec()),
		);
		let sig = pezsp_io::crypto::ecdsa_sign_prehashed(key_type, &pub_key, &digest)
			.expect("signing with Hardhat #0 key must succeed; qed");
		let sig_bytes: &[u8; 65] = sig.as_ref();
		let r: [u8; 32] = sig_bytes[0..32].try_into().expect("r is 32 bytes");
		let s: [u8; 32] = sig_bytes[32..64].try_into().expect("s is 32 bytes");
		let v: u8 = sig_bytes[64] + 27;

		#[block]
		{
			Pezpallet::<T>::use_permit(
				&verifying_contract,
				TEST_TOKEN_NAME,
				&owner,
				&spender,
				&value,
				&deadline,
				v,
				&r,
				&s,
			)
			.expect("use_permit should succeed");
		}

		// Verify nonce was incremented, confirming the full flow ran.
		assert_eq!(Pezpallet::<T>::nonce(&verifying_contract, &owner), U256::one());
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
