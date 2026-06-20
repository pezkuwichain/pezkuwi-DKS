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
#![allow(useless_deprecated)]

use std::collections::BTreeMap;

use pezframe_support::{
	derive_impl,
	dispatch::Parameter,
	parameter_types,
	traits::{ConstU32, StorageVersion},
	OrdNoBound, PartialOrdNoBound,
};
use scale_info::TypeInfo;

parameter_types! {
	/// Used to control if the storage version should be updated.
	storage UpdateStorageVersion: bool = false;
}

pub struct SomeType1;
impl From<SomeType1> for u64 {
	fn from(_t: SomeType1) -> Self {
		0u64
	}
}

pub trait SomeAssociation1 {
	type _1: Parameter + codec::MaxEncodedLen + TypeInfo;
}
impl SomeAssociation1 for u64 {
	type _1 = u64;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;

	pub(crate) const STORAGE_VERSION: StorageVersion = StorageVersion::new(10);

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config
	where
		<Self as pezframe_system::Config>::AccountId: From<SomeType1> + SomeAssociation1,
	{
		type Balance: Parameter + Default + TypeInfo;

		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
	}

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::error]
	#[derive(PartialEq, Eq)]
	pub enum Error<T> {
		/// error doc comment put in metadata
		InsufficientProposersBalance,
		NonExistentStorageValue,
		Code(u8),
		#[codec(skip)]
		Skipped(u128),
		CompactU8(#[codec(compact)] u8),
	}

	#[pezpallet::event]
	pub enum Event<T: Config>
	where
		T::AccountId: SomeAssociation1 + From<SomeType1>,
	{
		#[deprecated = "second"]
		#[codec(index = 1)]
		A,
		#[deprecated = "first"]
		#[codec(index = 0)]
		B,
	}

	#[pezpallet::origin]
	#[derive(
		EqNoBound,
		RuntimeDebugNoBound,
		CloneNoBound,
		PartialEqNoBound,
		PartialOrdNoBound,
		OrdNoBound,
		Encode,
		Decode,
		DecodeWithMemTracking,
		TypeInfo,
		MaxEncodedLen,
	)]
	pub struct Origin<T>(PhantomData<T>);
}

pezframe_support::parameter_types!(
	pub const MyGetParam3: u32 = 12;
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = pezsp_runtime::testing::H256;
	type Hashing = pezsp_runtime::traits::BlakeTwo256;
	type AccountId = u64;
	type Lookup = pezsp_runtime::traits::IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type MaxConsumers = ConstU32<16>;
}
impl pezpallet::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u64;
}

pub type Header = pezsp_runtime::generic::Header<u32, pezsp_runtime::traits::BlakeTwo256>;
pub type Block = pezsp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = pezsp_runtime::generic::UncheckedExtrinsic<
	u64,
	RuntimeCall,
	pezsp_runtime::testing::UintAuthorityId,
	pezframe_system::CheckNonZeroSender<Runtime>,
>;

pezframe_support::construct_runtime!(
	pub struct Runtime {
		// Exclude part `Storage` in order not to check its metadata in tests.
		System: pezframe_system exclude_parts { Pezpallet, Storage },
		Example: pezpallet,

	}
);

#[test]
fn pezpallet_metadata() {
	use pezsp_metadata_ir::{EnumDeprecationInfoIR, VariantDeprecationInfoIR};
	let pallets = Runtime::metadata_ir().pallets;
	let example = pallets[0].clone();
	{
		// Example pezpallet events are partially and fully deprecated
		let meta = example.event.unwrap();
		assert_eq!(
			EnumDeprecationInfoIR(BTreeMap::from([
				(0, VariantDeprecationInfoIR::Deprecated { note: "first", since: None }),
				(1, VariantDeprecationInfoIR::Deprecated { note: "second", since: None })
			])),
			meta.deprecation_info
		);
	}
}
