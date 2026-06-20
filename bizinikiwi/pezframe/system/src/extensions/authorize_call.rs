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

use crate::Config;
use pezframe_support::{
	dispatch::DispatchInfo,
	pezpallet_prelude::{
		Decode, DecodeWithMemTracking, DispatchResult, Encode, TransactionSource, TypeInfo, Weight,
	},
	traits::Authorize,
	CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound,
};
use pezsp_runtime::{
	traits::{
		AsTransactionAuthorizedOrigin, Dispatchable, Implication, PostDispatchInfoOf,
		TransactionExtension, ValidateResult,
	},
	transaction_validity::TransactionValidityError,
};

/// A transaction extension that authorizes some calls (i.e. dispatchable functions) to be
/// included in the block.
///
/// This transaction extension use the runtime implementation of the trait
/// [`Authorize`](pezframe_support::traits::Authorize) to set the validity of the transaction.
#[derive(
	Encode,
	Decode,
	CloneNoBound,
	EqNoBound,
	PartialEqNoBound,
	TypeInfo,
	RuntimeDebugNoBound,
	DecodeWithMemTracking,
)]
#[scale_info(skip_type_params(T))]
pub struct AuthorizeCall<T>(core::marker::PhantomData<T>);

impl<T> AuthorizeCall<T> {
	pub fn new() -> Self {
		Self(Default::default())
	}
}

impl<T: Config + Send + Sync> TransactionExtension<T::RuntimeCall> for AuthorizeCall<T>
where
	T::RuntimeCall: Dispatchable<Info = DispatchInfo>,
{
	const IDENTIFIER: &'static str = "AuthorizeCall";
	type Implicit = ();
	type Val = Weight;
	type Pre = Weight;

	fn validate(
		&self,
		origin: T::RuntimeOrigin,
		call: &T::RuntimeCall,
		_info: &DispatchInfo,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Implication,
		source: TransactionSource,
	) -> ValidateResult<Self::Val, T::RuntimeCall> {
		if !origin.is_transaction_authorized() {
			if let Some(authorize) = call.authorize(source) {
				return authorize.map(|(validity, unspent)| {
					(validity, unspent, crate::Origin::<T>::Authorized.into())
				});
			}
		}

		Ok((Default::default(), Weight::zero(), origin))
	}

	fn prepare(
		self,
		val: Self::Val,
		_origin: &T::RuntimeOrigin,
		_call: &T::RuntimeCall,
		_info: &DispatchInfo,
		_len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		Ok(val)
	}

	fn post_dispatch_details(
		pre: Self::Pre,
		_info: &DispatchInfo,
		_post_info: &PostDispatchInfoOf<T::RuntimeCall>,
		_len: usize,
		_result: &DispatchResult,
	) -> Result<Weight, TransactionValidityError> {
		Ok(pre)
	}

	fn weight(&self, call: &T::RuntimeCall) -> Weight {
		call.weight_of_authorize()
	}
}

#[cfg(test)]
mod tests {
	use crate as pezframe_system;
	use codec::Encode;
	use pezframe_support::{
		derive_impl, dispatch::GetDispatchInfo, pezpallet_prelude::TransactionSource,
		traits::OriginTrait,
	};
	use pezsp_runtime::{
		testing::UintAuthorityId,
		traits::{Applyable, Checkable, TransactionExtension as _, TxBaseImplication},
		transaction_validity::{
			InvalidTransaction, TransactionSource::External, TransactionValidityError,
		},
		BuildStorage, DispatchError,
	};

	#[pezframe_support::pezpallet]
	pub mod pallet1 {
		use crate as pezframe_system;
		use pezframe_support::pezpallet_prelude::*;
		use pezframe_system::pezpallet_prelude::*;

		pub const CALL_WEIGHT: Weight = Weight::from_all(4);
		pub const AUTH_WEIGHT: Weight = Weight::from_all(5);

		pub fn valid_transaction() -> ValidTransaction {
			ValidTransaction {
				priority: 10,
				provides: vec![1u8.encode()],
				requires: vec![],
				longevity: 1000,
				propagate: true,
			}
		}

		#[pezpallet::pezpallet]
		pub struct Pezpallet<T>(_);

		#[pezpallet::config]
		pub trait Config: pezframe_system::Config {}

		#[pezpallet::call]
		impl<T: Config> Pezpallet<T> {
			#[pezpallet::weight(CALL_WEIGHT)]
			#[pezpallet::call_index(0)]
			#[pezpallet::authorize(|_source, valid| if *valid {
				Ok((valid_transaction(), Weight::zero()))
			} else {
				Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
			})]
			#[pezpallet::weight_of_authorize(AUTH_WEIGHT)]
			pub fn call1(origin: OriginFor<T>, valid: bool) -> DispatchResult {
				crate::ensure_authorized(origin)?;
				let _ = valid;
				Ok(())
			}
		}
	}

	#[pezframe_support::runtime]
	mod runtime {
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
			RuntimeTask
		)]
		pub struct Runtime;

		#[runtime::pezpallet_index(0)]
		pub type System = pezframe_system::Pezpallet<Runtime>;

		#[runtime::pezpallet_index(1)]
		pub type Pallet1 = pallet1::Pezpallet<Runtime>;
	}

	pub type TransactionExtension = (pezframe_system::AuthorizeCall<Runtime>,);

	pub type Header = pezsp_runtime::generic::Header<u32, pezsp_runtime::traits::BlakeTwo256>;
	pub type Block = pezsp_runtime::generic::Block<Header, UncheckedExtrinsic>;
	pub type UncheckedExtrinsic = pezsp_runtime::generic::UncheckedExtrinsic<
		u64,
		RuntimeCall,
		UintAuthorityId,
		TransactionExtension,
	>;

	#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
	impl pezframe_system::Config for Runtime {
		type Block = Block;
	}

	impl pallet1::Config for Runtime {}

	pub fn new_test_ext() -> pezsp_io::TestExternalities {
		let t = RuntimeGenesisConfig { ..Default::default() }.build_storage().unwrap();
		t.into()
	}

	#[test]
	fn valid_transaction() {
		let call = RuntimeCall::Pallet1(pallet1::Call::call1 { valid: true });

		new_test_ext().execute_with(|| {
			let tx_ext = (pezframe_system::AuthorizeCall::<Runtime>::new(),);

			let tx = UncheckedExtrinsic::new_transaction(call, tx_ext);

			let info = tx.get_dispatch_info();
			let len = tx.using_encoded(|e| e.len());

			let checked =
				Checkable::check(tx, &pezframe_system::ChainContext::<Runtime>::default())
					.expect("Transaction is general so signature is good");

			let valid_tx = checked
				.validate::<Runtime>(TransactionSource::External, &info, len)
				.expect("call valid");

			let dispatch_result =
				checked.apply::<Runtime>(&info, len).expect("Transaction is valid");

			assert!(dispatch_result.is_ok());

			let post_info = dispatch_result.unwrap_or_else(|e| e.post_info);

			assert_eq!(valid_tx, pallet1::valid_transaction());
			assert_eq!(info.call_weight, pallet1::CALL_WEIGHT);
			assert_eq!(info.extension_weight, pallet1::AUTH_WEIGHT);
			assert_eq!(post_info.actual_weight, Some(pallet1::CALL_WEIGHT + pallet1::AUTH_WEIGHT));
		});
	}

	#[test]
	fn invalid_transaction_fail_authorization() {
		let call = RuntimeCall::Pallet1(pallet1::Call::call1 { valid: false });

		new_test_ext().execute_with(|| {
			let tx_ext = (pezframe_system::AuthorizeCall::<Runtime>::new(),);

			let tx = UncheckedExtrinsic::new_transaction(call, tx_ext);

			let info = tx.get_dispatch_info();
			let len = tx.using_encoded(|e| e.len());

			let checked =
				Checkable::check(tx, &pezframe_system::ChainContext::<Runtime>::default())
					.expect("Transaction is general so signature is good");

			let validate_err = checked
				.validate::<Runtime>(TransactionSource::External, &info, len)
				.expect_err("call is invalid");

			let apply_err =
				checked.apply::<Runtime>(&info, len).expect_err("Transaction is invalid");

			assert_eq!(validate_err, TransactionValidityError::Invalid(InvalidTransaction::Call));
			assert_eq!(apply_err, TransactionValidityError::Invalid(InvalidTransaction::Call));
			assert_eq!(info.call_weight, pallet1::CALL_WEIGHT);
			assert_eq!(info.extension_weight, pallet1::AUTH_WEIGHT);
		});
	}

	#[test]
	fn failing_transaction_invalid_origin() {
		let call = RuntimeCall::Pallet1(pallet1::Call::call1 { valid: true });

		new_test_ext().execute_with(|| {
			let tx_ext = (pezframe_system::AuthorizeCall::<Runtime>::new(),);

			let tx = UncheckedExtrinsic::new_signed(call, 42, 42.into(), tx_ext);

			let info = tx.get_dispatch_info();
			let len = tx.using_encoded(|e| e.len());

			let checked =
				Checkable::check(tx, &pezframe_system::ChainContext::<Runtime>::default())
					.expect("Signature is good");

			checked
				.validate::<Runtime>(TransactionSource::External, &info, len)
				.expect("Transaction is valid, tx ext doesn't deny none");

			let dispatch_res = checked
				.apply::<Runtime>(&info, len)
				.expect("Transaction is valid")
				.expect_err("Transaction is failing, because origin is wrong");

			assert_eq!(dispatch_res.error, DispatchError::BadOrigin);
			assert_eq!(info.call_weight, pallet1::CALL_WEIGHT);
			assert_eq!(info.extension_weight, pallet1::AUTH_WEIGHT);
		});
	}

	#[test]
	fn call_filter_preserved() {
		new_test_ext().execute_with(|| {
			let ext = pezframe_system::AuthorizeCall::<Runtime>::new();
			let filtered_call =
				RuntimeCall::System(pezframe_system::Call::remark { remark: vec![] });

			let origin = {
				let mut o: RuntimeOrigin = crate::Origin::<Runtime>::Signed(42).into();
				let filter_clone = filtered_call.clone();
				o.add_filter(move |call| filter_clone != *call);
				o
			};

			assert!(!origin.filter_call(&filtered_call));

			let (_, _, new_origin) = ext
				.validate(
					origin,
					&RuntimeCall::Pallet1(pallet1::Call::call1 { valid: true }),
					&crate::DispatchInfo::default(),
					Default::default(),
					(),
					&TxBaseImplication(()),
					External,
				)
				.expect("valid");

			assert!(!new_origin.filter_call(&filtered_call));
		});
	}
}
