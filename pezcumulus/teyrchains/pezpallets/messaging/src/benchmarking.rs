// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Benchmarking setup for pezpallet-messaging
//!
//! Run benchmarks with:
//! ```text
//! ./target/release/frame-omni-bencher v1 benchmark pezpallet \
//!   --runtime target/release/wbuild/people-pezkuwichain-runtime/people_pezkuwichain_runtime.compact.compressed.wasm \
//!   --pallets pezpallet_messaging -e all --steps 50 --repeat 20 \
//!   --output pezcumulus/teyrchains/pezpallets/messaging/src/weights.rs \
//!   --template bizinikiwi/.maintain/frame-weight-template.hbs
//! ```

use super::*;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register_encryption_key() {
		let caller: T::AccountId = whitelisted_caller();
		// NOTE: In real benchmarks, caller must be mocked as citizen.
		// This requires BenchmarkHelper trait integration (future work).
		let key = [1u8; 32];

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), key);

		assert!(EncryptionKeys::<T>::contains_key(&caller));
	}

	#[benchmark]
	fn send_message(l: Linear<1, 512>) {
		let sender: T::AccountId = whitelisted_caller();
		let recipient: T::AccountId = account("recipient", 0, 0);
		let key = [2u8; 32];
		let ephemeral = [3u8; 32];
		let nonce = [4u8; 24];
		let ciphertext = alloc::vec![0xAB; l as usize];

		// Pre-setup: register encryption keys
		EncryptionKeys::<T>::insert(&sender, [1u8; 32]);
		EncryptionKeys::<T>::insert(&recipient, key);

		#[extrinsic_call]
		_(RawOrigin::Signed(sender), recipient.clone(), ephemeral, nonce, ciphertext);
	}

	#[benchmark]
	fn acknowledge_messages() {
		let caller: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller));
	}

	#[benchmark]
	fn cleanup_era(n: Linear<1, 100>) {
		// Pre-populate storage with n entries for era 0
		let era: u32 = 0;
		for i in 0..n {
			let account: T::AccountId = account("user", i, 0);
			Inbox::<T>::insert(era, &account, BoundedVec::default());
		}

		#[block]
		{
			let _ = Inbox::<T>::clear_prefix(era, n, None);
		}
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test,);
}
