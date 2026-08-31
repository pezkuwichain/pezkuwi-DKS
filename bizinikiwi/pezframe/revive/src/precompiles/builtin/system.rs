// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
	Config, H160,
	address::AddressMapper,
	precompiles::{BuiltinAddressMatcher, BuiltinPrecompile, Error, Ext},
	vm::RuntimeCosts,
};
use alloc::vec::Vec;
use alloy_core::sol_types::SolValue;
use codec::Encode;
use core::{marker::PhantomData, num::NonZero};
use pezpallet_revive_uapi::precompiles::system::ISystem;
use pezsp_core::hexdisplay::AsBytesRef;

pub struct System<T>(PhantomData<T>);

impl<T: Config> BuiltinPrecompile for System<T> {
	type T = T;
	type Interface = ISystem::ISystemCalls;
	const MATCHER: BuiltinAddressMatcher =
		BuiltinAddressMatcher::Fixed(NonZero::new(0x900).unwrap());
	const HAS_CONTRACT_INFO: bool = false;

	fn call(
		_address: &[u8; 20],
		input: &Self::Interface,
		env: &mut impl Ext<T = Self::T>,
	) -> Result<Vec<u8>, Error> {
		use ISystem::ISystemCalls;
		match input {
			ISystemCalls::terminate(_) if env.is_read_only() => {
				Err(crate::Error::<T>::StateChangeDenied.into())
			},
			ISystemCalls::hashBlake256(ISystem::hashBlake256Call { input }) => {
				env.pezframe_meter_mut()
					.charge_weight_token(RuntimeCosts::HashBlake256(input.len() as u32))?;
				let output = pezsp_io::hashing::blake2_256(input.as_bytes_ref());
				Ok(output.abi_encode())
			},
			ISystemCalls::hashBlake128(ISystem::hashBlake128Call { input }) => {
				env.pezframe_meter_mut()
					.charge_weight_token(RuntimeCosts::HashBlake128(input.len() as u32))?;
				let output = pezsp_io::hashing::blake2_128(input.as_bytes_ref());
				Ok(output.abi_encode())
			},
			ISystemCalls::toAccountId(ISystem::toAccountIdCall { input }) => {
				env.pezframe_meter_mut().charge_weight_token(RuntimeCosts::ToAccountId)?;
				let account_id = env.to_account_id(&H160::from_slice(input.as_slice()));
				Ok(account_id.encode().abi_encode())
			},
			ISystemCalls::callerIsOrigin(ISystem::callerIsOriginCall {}) => {
				env.pezframe_meter_mut().charge_weight_token(RuntimeCosts::CallerIsOrigin)?;
				let is_origin = env.caller_is_origin(true);
				Ok(is_origin.abi_encode())
			},
			ISystemCalls::callerIsRoot(ISystem::callerIsRootCall {}) => {
				env.pezframe_meter_mut().charge_weight_token(RuntimeCosts::CallerIsRoot)?;
				let is_root = env.caller_is_root(true);
				Ok(is_root.abi_encode())
			},
			ISystemCalls::ownCodeHash(ISystem::ownCodeHashCall {}) => {
				env.pezframe_meter_mut().charge_weight_token(RuntimeCosts::OwnCodeHash)?;
				let caller = env.caller();
				let addr = T::AddressMapper::to_address(caller.account_id()?);
				let output = env.code_hash(&addr.into()).0.abi_encode();
				Ok(output)
			},
			ISystemCalls::minimumBalance(ISystem::minimumBalanceCall {}) => {
				env.pezframe_meter_mut().charge_weight_token(RuntimeCosts::MinimumBalance)?;
				let minimum_balance = env.minimum_balance();
				Ok(minimum_balance.to_big_endian().abi_encode())
			},
			ISystemCalls::weightLeft(ISystem::weightLeftCall {}) => {
				env.pezframe_meter_mut().charge_weight_token(RuntimeCosts::WeightLeft)?;
				let ref_time = env.pezframe_meter().weight_left().unwrap_or_default().ref_time();
				let proof_size =
					env.pezframe_meter().weight_left().unwrap_or_default().proof_size();
				let res = (ref_time, proof_size);
				Ok(res.abi_encode())
			},
			ISystemCalls::terminate(ISystem::terminateCall { beneficiary }) => {
				// no need to adjust gas because this always deletes code
				env.pezframe_meter_mut()
					.charge_weight_token(RuntimeCosts::Terminate { code_removed: true })?;
				let h160 = H160::from_slice(beneficiary.as_slice());
				env.terminate_caller(&h160).map_err(Error::try_to_revert::<T>)?;
				Ok(Vec::new())
			},
			ISystemCalls::sr25519Verify(ISystem::sr25519VerifyCall {
				signature,
				message,
				publicKey,
			}) => {
				env.pezframe_meter_mut()
					.charge_weight_token(RuntimeCosts::Sr25519Verify(message.len() as _))?;
				let ok = env.sr25519_verify(signature, message, publicKey);
				Ok(ok.abi_encode())
			},
			ISystemCalls::ecdsaToEthAddress(ISystem::ecdsaToEthAddressCall { publicKey }) => {
				env.pezframe_meter_mut().charge_weight_token(RuntimeCosts::EcdsaToEthAddress)?;
				let address =
					env.ecdsa_to_eth_address(publicKey).map_err(Error::try_to_revert::<T>)?;
				Ok(address.abi_encode())
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		address::AddressMapper,
		call_builder::{CallSetup, caller_funding},
		metering::Token,
		pezpallet,
		precompiles::{
			BuiltinPrecompile,
			alloy::sol_types::{SolType, sol_data::Bytes},
			tests::run_test_vectors,
		},
		test_utils::ALICE,
		tests::{ExtBuilder, Test},
		vm::RuntimeCosts,
	};

	use alloy_core::primitives::FixedBytes;
	use codec::Decode;
	use pezframe_support::traits::fungible::Mutate;

	#[test]
	fn test_system_precompile() {
		run_test_vectors::<System<Test>>(include_str!("testdata/900-blake2_256.json"));
		run_test_vectors::<System<Test>>(include_str!("testdata/900-blake2_128.json"));
		run_test_vectors::<System<Test>>(include_str!("testdata/900-to_account_id.json"));
	}

	#[test]
	fn test_system_precompile_unmapped_account() {
		ExtBuilder::default().build().execute_with(|| {
			// given
			let mut call_setup = CallSetup::<Test>::default();
			let (mut ext, _) = call_setup.ext();
			let unmapped_address = H160::zero();

			// when
			let input = ISystem::ISystemCalls::toAccountId(ISystem::toAccountIdCall {
				input: unmapped_address.0.into(),
			});
			let raw_data =
				<System<Test>>::call(&<System<Test>>::MATCHER.base_address(), &input, &mut ext)
					.unwrap();

			// then
			let expected_fallback_account_id =
				Bytes::abi_decode(&raw_data).expect("decoding failed");
			assert_eq!(
				expected_fallback_account_id.0.as_ref()[20..32],
				[0xEE; 12],
				"no fallback suffix found where one should be"
			);
		})
	}

	#[test]
	fn test_system_precompile_mapped_account() {
		use crate::test_utils::EVE;
		ExtBuilder::default().build().execute_with(|| {
			// given
			let mapped_address = {
				<Test as pezpallet::Config>::Currency::set_balance(&EVE, caller_funding::<Test>());
				let _ = <Test as pezpallet::Config>::AddressMapper::map(&EVE);
				<Test as pezpallet::Config>::AddressMapper::to_address(&EVE)
			};

			let mut call_setup = CallSetup::<Test>::default();
			let (mut ext, _) = call_setup.ext();

			// when
			let input = ISystem::ISystemCalls::toAccountId(ISystem::toAccountIdCall {
				input: mapped_address.0.into(),
			});
			let raw_data =
				<System<Test>>::call(&<System<Test>>::MATCHER.base_address(), &input, &mut ext)
					.unwrap();

			// then
			let data = Bytes::abi_decode(&raw_data).expect("decoding failed");
			assert_ne!(
				data.0.as_ref()[20..32],
				[0xEE; 12],
				"fallback suffix found where none should be"
			);
			assert_eq!(
				<Test as pezframe_system::Config>::AccountId::decode(&mut data.as_ref()),
				Ok(EVE),
			);
		})
	}

	#[test]
	fn sr25519_verify() {
		use crate::precompiles::alloy::sol_types::sol_data::Bool;
		ExtBuilder::default().build().execute_with(|| {
			let _ = <Test as Config>::Currency::set_balance(&ALICE, 100_000_000_000);

			let mut call_setup = CallSetup::<Test>::default();
			let (mut ext, _) = call_setup.ext();

			// Signed here rather than pasted in. A hard-coded signature is a wire-format
			// constant: it verifies only under the context it was produced with, and this
			// chain's signing context is deliberately not upstream's. The pasted pair
			// verified under `b"substrate"` and this test began failing the moment the
			// sovereign context was put back.
			let pair =
				<pezsp_core::sr25519::Pair as pezsp_core::Pair>::from_string("//Alice", None)
					.expect("//Alice is a valid seed; qed");
			let public_key: [u8; 32] =
				<pezsp_core::sr25519::Pair as pezsp_core::Pair>::public(&pair).0;

			let mut call_with = |message: &[u8; 11], signed: &[u8; 11]| {
				let signature: [u8; 64] =
					<pezsp_core::sr25519::Pair as pezsp_core::Pair>::sign(&pair, signed).0;

				let weight_before = ext.pezframe_meter().weight_consumed();

				let input = ISystem::ISystemCalls::sr25519Verify(ISystem::sr25519VerifyCall {
					signature,
					message: (*message).into(),
					publicKey: public_key.into(),
				});
				let result =
					<System<Test>>::call(&<System<Test>>::MATCHER.base_address(), &input, &mut ext)
						.unwrap();

				let weight_used = ext.pezframe_meter().weight_consumed() - weight_before;
				assert!(weight_used.ref_time() > 0, "sr25519_verify should charge weight");
				assert_eq!(
					weight_used,
					Token::<Test>::weight(&RuntimeCosts::Sr25519Verify(message.len() as u32)),
					"sr25519_verify should charge the expected weight"
				);
				result
			};
			// The signature covers the message being checked.
			let result = Bool::abi_decode(&call_with(b"hello world", b"hello world"))
				.expect("decoding failed");
			assert!(result);
			// A signature over a different message must not verify.
			let result = Bool::abi_decode(&call_with(b"hello worlD", b"hello world"))
				.expect("decoding failed");
			assert!(!result);
		});
	}

	#[test]
	fn ecdsa_to_eth_address() {
		ExtBuilder::default().build().execute_with(|| {
			let _ = <Test as Config>::Currency::set_balance(&ALICE, 100_000_000_000);

			let mut call_setup = CallSetup::<Test>::default();
			let (mut ext, _) = call_setup.ext();

			let pubkey_compressed = array_bytes::hex2array_unchecked(
				"028db55b05db86c0b1786ca49f095d76344c9e6056b2f02701a7e7f3c20aabfd91",
			);

			let weight_before = ext.pezframe_meter().weight_consumed();

			let input = ISystem::ISystemCalls::ecdsaToEthAddress(ISystem::ecdsaToEthAddressCall {
				publicKey: pubkey_compressed,
			});
			let result =
				<System<Test>>::call(&<System<Test>>::MATCHER.base_address(), &input, &mut ext)
					.unwrap();

			let expected: FixedBytes<20> = array_bytes::hex2array_unchecked::<_, 20>(
				"09231da7b19A016f9e576d23B16277062F4d46A8",
			)
			.into();
			assert_eq!(result, expected.abi_encode());

			let weight_used = ext.pezframe_meter().weight_consumed() - weight_before;
			assert!(weight_used.ref_time() > 0, "ecdsa_to_eth_address should charge weight");
			assert_eq!(
				weight_used,
				Token::<Test>::weight(&RuntimeCosts::EcdsaToEthAddress),
				"ecdsa_to_eth_address should charge the expected weight"
			);
		});
	}
}
