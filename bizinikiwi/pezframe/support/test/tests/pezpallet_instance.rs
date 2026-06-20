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

use core::any::TypeId;
use pezframe_support::{
	derive_impl,
	dispatch::{DispatchClass, DispatchInfo, GetDispatchInfo, Pays},
	parameter_types,
	pezpallet_prelude::ValueQuery,
	storage::unhashed,
	traits::{
		ConstU32, GetCallName, OnFinalize, OnGenesis, OnInitialize, OnRuntimeUpgrade,
		UnfilteredDispatchable,
	},
	weights::Weight,
	OrdNoBound, PartialOrdNoBound,
};
use pezsp_io::{
	hashing::{blake2_128, twox_128, twox_64},
	TestExternalities,
};
use pezsp_runtime::{DispatchError, ModuleError};

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	type BalanceOf<T, I> = <T as Config<I>>::Balance;

	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config {
		#[pezpallet::constant]
		type MyGetParam: Get<u32>;
		type Balance: Parameter + Default + scale_info::StaticTypeInfo;
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(PhantomData<(T, I)>);

	#[pezpallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pezpallet<T, I> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			if TypeId::of::<I>() == TypeId::of::<()>() {
				Self::deposit_event(Event::Something(10));
				Weight::from_parts(10, 0)
			} else {
				Self::deposit_event(Event::Something(11));
				Weight::from_parts(11, 0)
			}
		}
		fn on_finalize(_: BlockNumberFor<T>) {
			if TypeId::of::<I>() == TypeId::of::<()>() {
				Self::deposit_event(Event::Something(20));
			} else {
				Self::deposit_event(Event::Something(21));
			}
		}
		fn on_runtime_upgrade() -> Weight {
			if TypeId::of::<I>() == TypeId::of::<()>() {
				Self::deposit_event(Event::Something(30));
				Weight::from_parts(30, 0)
			} else {
				Self::deposit_event(Event::Something(31));
				Weight::from_parts(31, 0)
			}
		}
		fn integrity_test() {}
	}

	#[pezpallet::call]
	impl<T: Config<I>, I: 'static> Pezpallet<T, I> {
		/// Doc comment put in metadata
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(Weight::from_parts(*foo as u64, 0))]
		pub fn foo(
			origin: OriginFor<T>,
			#[pezpallet::compact] foo: u32,
		) -> DispatchResultWithPostInfo {
			let _ = origin;
			let _ = foo;
			Self::deposit_event(Event::Something(3));
			Ok(().into())
		}

		/// Doc comment put in metadata
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(1)]
		pub fn foo_storage_layer(
			origin: OriginFor<T>,
			#[pezpallet::compact] _foo: u32,
		) -> DispatchResultWithPostInfo {
			let _ = origin;
			Ok(().into())
		}
	}

	#[pezpallet::error]
	#[derive(PartialEq, Eq)]
	pub enum Error<T, I = ()> {
		/// doc comment put into metadata
		InsufficientProposersBalance,
		NonExistentStorageValue,
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// doc comment put in metadata
		Proposed(<T as pezframe_system::Config>::AccountId),
		/// doc
		Spending(BalanceOf<T, I>),
		Something(u32),
	}

	#[pezpallet::storage]
	pub type Value<T, I = ()> = StorageValue<_, u32>;

	#[pezpallet::storage]
	pub type Map<T, I = ()> = StorageMap<_, Blake2_128Concat, u8, u16>;

	#[pezpallet::storage]
	pub type Map2<T, I = ()> = StorageMap<_, Twox64Concat, u16, u32>;

	parameter_types! {
		pub const Map3Default<T, I>: Result<u64, Error<T, I>> = Ok(1337);
	}

	#[pezpallet::storage]
	pub type Map3<T, I = ()> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		u64,
		ResultQuery<Error<T, I>::NonExistentStorageValue>,
		Map3Default<T, I>,
	>;

	#[pezpallet::storage]
	pub type DoubleMap<T, I = ()> =
		StorageDoubleMap<_, Blake2_128Concat, u8, Twox64Concat, u16, u32>;

	#[pezpallet::storage]
	pub type DoubleMap2<T, I = ()> =
		StorageDoubleMap<_, Twox64Concat, u16, Blake2_128Concat, u32, u64>;

	#[pezpallet::storage]
	pub type DoubleMap3<T, I = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Twox64Concat,
		u64,
		u128,
		ResultQuery<Error<T, I>::NonExistentStorageValue>,
	>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn nmap)]
	pub type NMap<T, I = ()> = StorageNMap<_, storage::Key<Blake2_128Concat, u8>, u32>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn nmap2)]
	pub type NMap2<T, I = ()> =
		StorageNMap<_, (storage::Key<Twox64Concat, u16>, storage::Key<Blake2_128Concat, u32>), u64>;

	#[pezpallet::storage]
	#[pezpallet::getter(fn nmap3)]
	pub type NMap3<T, I = ()> = StorageNMap<
		_,
		(NMapKey<Blake2_128Concat, u8>, NMapKey<Twox64Concat, u16>),
		u128,
		ResultQuery<Error<T, I>::NonExistentStorageValue>,
	>;

	#[pezpallet::genesis_config]
	#[derive(pezframe_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		#[serde(skip)]
		_config: core::marker::PhantomData<(T, I)>,
		_myfield: u32,
	}

	#[pezpallet::genesis_build]
	impl<T: Config<I>, I: 'static> BuildGenesisConfig for GenesisConfig<T, I> {
		fn build(&self) {}
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
		scale_info::TypeInfo,
		MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T, I))]
	pub struct Origin<T, I = ()>(PhantomData<(T, I)>);

	#[pezpallet::validate_unsigned]
	impl<T: Config<I>, I: 'static> ValidateUnsigned for Pezpallet<T, I> {
		type Call = Call<T, I>;
		fn validate_unsigned(
			_source: TransactionSource,
			_call: &Self::Call,
		) -> TransactionValidity {
			Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
		}
	}

	#[pezpallet::inherent]
	impl<T: Config<I>, I: 'static> ProvideInherent for Pezpallet<T, I> {
		type Call = Call<T, I>;
		type Error = InherentError;

		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(_data: &InherentData) -> Option<Self::Call> {
			unimplemented!();
		}

		fn is_inherent(_call: &Self::Call) -> bool {
			unimplemented!();
		}
	}

	#[derive(codec::Encode, pezsp_runtime::RuntimeDebug)]
	#[cfg_attr(feature = "std", derive(codec::Decode))]
	pub enum InherentError {}

	impl pezframe_support::inherent::IsFatalError for InherentError {
		fn is_fatal_error(&self) -> bool {
			unimplemented!();
		}
	}

	pub const INHERENT_IDENTIFIER: pezframe_support::inherent::InherentIdentifier = *b"testpall";
}

// Test that a instantiable pezpallet with a generic genesis_config is correctly handled
#[pezframe_support::pezpallet]
pub mod pallet2 {
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config {
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
	}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(PhantomData<(T, I)>);

	#[pezpallet::event]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Something
		Something(u32),
	}

	#[pezpallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		phantom: PhantomData<(T, I)>,
	}

	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			GenesisConfig { phantom: Default::default() }
		}
	}

	#[pezpallet::genesis_build]
	impl<T: Config<I>, I: 'static> BuildGenesisConfig for GenesisConfig<T, I> {
		fn build(&self) {}
	}
}

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
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}
impl pezpallet::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MyGetParam = ConstU32<10>;
	type Balance = u64;
}
impl pezpallet::Config<pezpallet::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MyGetParam = ConstU32<10>;
	type Balance = u64;
}
impl pallet2::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}
impl pallet2::Config<pezpallet::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

pub type Header = pezsp_runtime::generic::Header<u32, pezsp_runtime::traits::BlakeTwo256>;
pub type Block = pezsp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = pezsp_runtime::generic::UncheckedExtrinsic<u32, RuntimeCall, (), ()>;

pezframe_support::construct_runtime!(
	pub struct Runtime
	{
		// Exclude part `Storage` in order not to check its metadata in tests.
		System: pezframe_system exclude_parts { Storage },
		Example: pezpallet,
		Instance1Example: pezpallet::<Instance1>,
		Example2: pallet2,
		Instance1Example2: pallet2::<Instance1>,
	}
);

#[test]
fn call_expand() {
	let call_foo = pezpallet::Call::<Runtime>::foo { foo: 3 };
	assert_eq!(
		call_foo.get_dispatch_info(),
		DispatchInfo {
			call_weight: Weight::from_parts(3, 0),
			extension_weight: Default::default(),
			class: DispatchClass::Normal,
			pays_fee: Pays::Yes
		}
	);
	assert_eq!(call_foo.get_call_name(), "foo");
	assert_eq!(pezpallet::Call::<Runtime>::get_call_names(), &["foo", "foo_storage_layer"]);

	let call_foo = pezpallet::Call::<Runtime, pezpallet::Instance1>::foo { foo: 3 };
	assert_eq!(
		call_foo.get_dispatch_info(),
		DispatchInfo {
			call_weight: Weight::from_parts(3, 0),
			extension_weight: Default::default(),
			class: DispatchClass::Normal,
			pays_fee: Pays::Yes
		}
	);
	assert_eq!(call_foo.get_call_name(), "foo");
	assert_eq!(
		pezpallet::Call::<Runtime, pezpallet::Instance1>::get_call_names(),
		&["foo", "foo_storage_layer"],
	);
}

#[test]
fn error_expand() {
	assert_eq!(
		format!("{:?}", pezpallet::Error::<Runtime>::InsufficientProposersBalance),
		String::from("InsufficientProposersBalance"),
	);
	assert_eq!(
		<&'static str>::from(pezpallet::Error::<Runtime>::InsufficientProposersBalance),
		"InsufficientProposersBalance",
	);
	assert_eq!(
		DispatchError::from(pezpallet::Error::<Runtime>::InsufficientProposersBalance),
		DispatchError::Module(ModuleError {
			index: 1,
			error: [0; 4],
			message: Some("InsufficientProposersBalance")
		}),
	);

	assert_eq!(
		format!(
			"{:?}",
			pezpallet::Error::<Runtime, pezpallet::Instance1>::InsufficientProposersBalance
		),
		String::from("InsufficientProposersBalance"),
	);
	assert_eq!(
		<&'static str>::from(
			pezpallet::Error::<Runtime, pezpallet::Instance1>::InsufficientProposersBalance
		),
		"InsufficientProposersBalance",
	);
	assert_eq!(
		DispatchError::from(
			pezpallet::Error::<Runtime, pezpallet::Instance1>::InsufficientProposersBalance
		),
		DispatchError::Module(ModuleError {
			index: 2,
			error: [0; 4],
			message: Some("InsufficientProposersBalance")
		}),
	);
}

#[test]
fn module_error_outer_enum_expand() {
	// assert that all variants of the Example pezpallet are included into the
	// RuntimeError definition.
	match RuntimeError::Example(pezpallet::Error::InsufficientProposersBalance) {
		RuntimeError::Example(example) => match example {
			pezpallet::Error::InsufficientProposersBalance => (),
			pezpallet::Error::NonExistentStorageValue => (),
			// Extra pattern added by `construct_runtime`.
			pezpallet::Error::__Ignore(_, _) => (),
		},
		_ => (),
	};
}

#[test]
fn module_error_from_dispatch_error() {
	let dispatch_err = DispatchError::Module(ModuleError {
		index: 1,
		error: [0; 4],
		message: Some("InsufficientProposersBalance"),
	});
	let err = RuntimeError::from_dispatch_error(dispatch_err).unwrap();

	match err {
		RuntimeError::Example(pezpallet::Error::InsufficientProposersBalance) => (),
		_ => panic!("Module error constructed incorrectly"),
	};

	// Only `ModuleError` is converted.
	assert!(RuntimeError::from_dispatch_error(DispatchError::BadOrigin).is_none());
}

#[test]
fn instance_expand() {
	// assert same type
	let _: pezpallet::__InherentHiddenInstance = ();
}

#[test]
fn pezpallet_expand_deposit_event() {
	TestExternalities::default().execute_with(|| {
		pezframe_system::Pezpallet::<Runtime>::set_block_number(1);
		pezpallet::Call::<Runtime>::foo { foo: 3 }
			.dispatch_bypass_filter(None.into())
			.unwrap();
		assert_eq!(
			pezframe_system::Pezpallet::<Runtime>::events()[0].event,
			RuntimeEvent::Example(pezpallet::Event::Something(3)),
		);
	});

	TestExternalities::default().execute_with(|| {
		pezframe_system::Pezpallet::<Runtime>::set_block_number(1);
		pezpallet::Call::<Runtime, pezpallet::Instance1>::foo { foo: 3 }
			.dispatch_bypass_filter(None.into())
			.unwrap();
		assert_eq!(
			pezframe_system::Pezpallet::<Runtime>::events()[0].event,
			RuntimeEvent::Instance1Example(pezpallet::Event::Something(3)),
		);
	});
}

#[test]
fn storage_expand() {
	use pezframe_support::{pezpallet_prelude::*, storage::StoragePrefixedMap};

	fn twox_64_concat(d: &[u8]) -> Vec<u8> {
		let mut v = twox_64(d).to_vec();
		v.extend_from_slice(d);
		v
	}

	fn blake2_128_concat(d: &[u8]) -> Vec<u8> {
		let mut v = blake2_128(d).to_vec();
		v.extend_from_slice(d);
		v
	}

	TestExternalities::default().execute_with(|| {
		<pezpallet::Value<Runtime>>::put(1);
		let k = [twox_128(b"Example"), twox_128(b"Value")].concat();
		assert_eq!(unhashed::get::<u32>(&k), Some(1u32));

		<pezpallet::Map<Runtime>>::insert(1, 2);
		let mut k = [twox_128(b"Example"), twox_128(b"Map")].concat();
		k.extend(1u8.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u16>(&k), Some(2u16));
		assert_eq!(&k[..32], &<pezpallet::Map<Runtime>>::final_prefix());

		<pezpallet::Map2<Runtime>>::insert(1, 2);
		let mut k = [twox_128(b"Example"), twox_128(b"Map2")].concat();
		k.extend(1u16.using_encoded(twox_64_concat));
		assert_eq!(unhashed::get::<u32>(&k), Some(2u32));
		assert_eq!(&k[..32], &<pezpallet::Map2<Runtime>>::final_prefix());

		<pezpallet::Map3<Runtime>>::insert(1, 2);
		let mut k = [twox_128(b"Example"), twox_128(b"Map3")].concat();
		k.extend(1u32.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u64>(&k), Some(2u64));
		assert_eq!(&k[..32], &<pezpallet::Map3<Runtime>>::final_prefix());
		assert_eq!(<pezpallet::Map3<Runtime>>::get(2), Ok(1337));

		<pezpallet::DoubleMap<Runtime>>::insert(&1, &2, &3);
		let mut k = [twox_128(b"Example"), twox_128(b"DoubleMap")].concat();
		k.extend(1u8.using_encoded(blake2_128_concat));
		k.extend(2u16.using_encoded(twox_64_concat));
		assert_eq!(unhashed::get::<u32>(&k), Some(3u32));
		assert_eq!(&k[..32], &<pezpallet::DoubleMap<Runtime>>::final_prefix());

		<pezpallet::DoubleMap2<Runtime>>::insert(&1, &2, &3);
		let mut k = [twox_128(b"Example"), twox_128(b"DoubleMap2")].concat();
		k.extend(1u16.using_encoded(twox_64_concat));
		k.extend(2u32.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u64>(&k), Some(3u64));
		assert_eq!(&k[..32], &<pezpallet::DoubleMap2<Runtime>>::final_prefix());

		<pezpallet::DoubleMap3<Runtime>>::insert(&1, &2, &3);
		let mut k = [twox_128(b"Example"), twox_128(b"DoubleMap3")].concat();
		k.extend(1u32.using_encoded(blake2_128_concat));
		k.extend(2u64.using_encoded(twox_64_concat));
		assert_eq!(unhashed::get::<u128>(&k), Some(3u128));
		assert_eq!(&k[..32], &<pezpallet::DoubleMap3<Runtime>>::final_prefix());
		assert_eq!(
			<pezpallet::DoubleMap3<Runtime>>::get(2, 3),
			Err(pezpallet::Error::<Runtime>::NonExistentStorageValue),
		);

		<pezpallet::NMap<Runtime>>::insert((&1,), &3);
		let mut k = [twox_128(b"Example"), twox_128(b"NMap")].concat();
		k.extend(1u8.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u32>(&k), Some(3u32));
		assert_eq!(&k[..32], &<pezpallet::NMap<Runtime>>::final_prefix());

		<pezpallet::NMap2<Runtime>>::insert((&1, &2), &3);
		let mut k = [twox_128(b"Example"), twox_128(b"NMap2")].concat();
		k.extend(1u16.using_encoded(twox_64_concat));
		k.extend(2u32.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u64>(&k), Some(3u64));
		assert_eq!(&k[..32], &<pezpallet::NMap2<Runtime>>::final_prefix());

		<pezpallet::NMap3<Runtime>>::insert((&1, &2), &3);
		let mut k = [twox_128(b"Example"), twox_128(b"NMap3")].concat();
		k.extend(1u8.using_encoded(blake2_128_concat));
		k.extend(2u16.using_encoded(twox_64_concat));
		assert_eq!(unhashed::get::<u128>(&k), Some(3u128));
		assert_eq!(&k[..32], &<pezpallet::NMap3<Runtime>>::final_prefix());
		assert_eq!(
			<pezpallet::NMap3<Runtime>>::get((2, 3)),
			Err(pezpallet::Error::<Runtime>::NonExistentStorageValue),
		);
	});

	TestExternalities::default().execute_with(|| {
		<pezpallet::Value<Runtime, pezpallet::Instance1>>::put(1);
		let k = [twox_128(b"Instance1Example"), twox_128(b"Value")].concat();
		assert_eq!(unhashed::get::<u32>(&k), Some(1u32));

		<pezpallet::Map<Runtime, pezpallet::Instance1>>::insert(1, 2);
		let mut k = [twox_128(b"Instance1Example"), twox_128(b"Map")].concat();
		k.extend(1u8.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u16>(&k), Some(2u16));
		assert_eq!(&k[..32], &<pezpallet::Map<Runtime, pezpallet::Instance1>>::final_prefix());

		<pezpallet::Map2<Runtime, pezpallet::Instance1>>::insert(1, 2);
		let mut k = [twox_128(b"Instance1Example"), twox_128(b"Map2")].concat();
		k.extend(1u16.using_encoded(twox_64_concat));
		assert_eq!(unhashed::get::<u32>(&k), Some(2u32));
		assert_eq!(&k[..32], &<pezpallet::Map2<Runtime, pezpallet::Instance1>>::final_prefix());

		<pezpallet::Map3<Runtime, pezpallet::Instance1>>::insert(1, 2);
		let mut k = [twox_128(b"Instance1Example"), twox_128(b"Map3")].concat();
		k.extend(1u32.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u64>(&k), Some(2u64));
		assert_eq!(&k[..32], &<pezpallet::Map3<Runtime, pezpallet::Instance1>>::final_prefix());
		assert_eq!(<pezpallet::Map3<Runtime, pezpallet::Instance1>>::get(2), Ok(1337));

		<pezpallet::DoubleMap<Runtime, pezpallet::Instance1>>::insert(&1, &2, &3);
		let mut k = [twox_128(b"Instance1Example"), twox_128(b"DoubleMap")].concat();
		k.extend(1u8.using_encoded(blake2_128_concat));
		k.extend(2u16.using_encoded(twox_64_concat));
		assert_eq!(unhashed::get::<u32>(&k), Some(3u32));
		assert_eq!(
			&k[..32],
			&<pezpallet::DoubleMap<Runtime, pezpallet::Instance1>>::final_prefix()
		);

		<pezpallet::DoubleMap2<Runtime, pezpallet::Instance1>>::insert(&1, &2, &3);
		let mut k = [twox_128(b"Instance1Example"), twox_128(b"DoubleMap2")].concat();
		k.extend(1u16.using_encoded(twox_64_concat));
		k.extend(2u32.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u64>(&k), Some(3u64));
		assert_eq!(
			&k[..32],
			&<pezpallet::DoubleMap2<Runtime, pezpallet::Instance1>>::final_prefix()
		);

		<pezpallet::DoubleMap3<Runtime, pezpallet::Instance1>>::insert(&1, &2, &3);
		let mut k = [twox_128(b"Instance1Example"), twox_128(b"DoubleMap3")].concat();
		k.extend(1u32.using_encoded(blake2_128_concat));
		k.extend(2u64.using_encoded(twox_64_concat));
		assert_eq!(unhashed::get::<u128>(&k), Some(3u128));
		assert_eq!(
			&k[..32],
			&<pezpallet::DoubleMap3<Runtime, pezpallet::Instance1>>::final_prefix()
		);
		assert_eq!(
			<pezpallet::DoubleMap3<Runtime, pezpallet::Instance1>>::get(2, 3),
			Err(pezpallet::Error::<Runtime, pezpallet::Instance1>::NonExistentStorageValue),
		);

		<pezpallet::NMap<Runtime, pezpallet::Instance1>>::insert((&1,), &3);
		let mut k = [twox_128(b"Instance1Example"), twox_128(b"NMap")].concat();
		k.extend(1u8.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u32>(&k), Some(3u32));
		assert_eq!(&k[..32], &<pezpallet::NMap<Runtime, pezpallet::Instance1>>::final_prefix());

		<pezpallet::NMap2<Runtime, pezpallet::Instance1>>::insert((&1, &2), &3);
		let mut k = [twox_128(b"Instance1Example"), twox_128(b"NMap2")].concat();
		k.extend(1u16.using_encoded(twox_64_concat));
		k.extend(2u32.using_encoded(blake2_128_concat));
		assert_eq!(unhashed::get::<u64>(&k), Some(3u64));
		assert_eq!(&k[..32], &<pezpallet::NMap2<Runtime, pezpallet::Instance1>>::final_prefix());

		<pezpallet::NMap3<Runtime, pezpallet::Instance1>>::insert((&1, &2), &3);
		let mut k = [twox_128(b"Instance1Example"), twox_128(b"NMap3")].concat();
		k.extend(1u8.using_encoded(blake2_128_concat));
		k.extend(2u16.using_encoded(twox_64_concat));
		assert_eq!(unhashed::get::<u128>(&k), Some(3u128));
		assert_eq!(&k[..32], &<pezpallet::NMap3<Runtime, pezpallet::Instance1>>::final_prefix());
		assert_eq!(
			<pezpallet::NMap3<Runtime, pezpallet::Instance1>>::get((2, 3)),
			Err(pezpallet::Error::<Runtime, pezpallet::Instance1>::NonExistentStorageValue),
		);
	});
}

#[test]
fn pezpallet_metadata_expands() {
	use pezframe_support::traits::PalletsInfoAccess;
	let mut infos = AllPalletsWithSystem::infos();
	infos.sort_by_key(|x| x.index);

	assert_eq!(infos[0].index, 0);
	assert_eq!(infos[0].name, "System");
	assert_eq!(infos[0].module_name, "pezframe_system");

	assert_eq!(infos[1].index, 1);
	assert_eq!(infos[1].name, "Example");
	assert_eq!(infos[1].module_name, "pezpallet");

	assert_eq!(infos[2].index, 2);
	assert_eq!(infos[2].name, "Instance1Example");
	assert_eq!(infos[2].module_name, "pezpallet");

	assert_eq!(infos[3].index, 3);
	assert_eq!(infos[3].name, "Example2");
	assert_eq!(infos[3].module_name, "pallet2");

	assert_eq!(infos[4].index, 4);
	assert_eq!(infos[4].name, "Instance1Example2");
	assert_eq!(infos[4].module_name, "pallet2");
}

#[test]
fn pezpallet_hooks_expand() {
	TestExternalities::default().execute_with(|| {
		pezframe_system::Pezpallet::<Runtime>::set_block_number(1);

		assert_eq!(AllPalletsWithoutSystem::on_initialize(1), Weight::from_parts(21, 0));
		AllPalletsWithoutSystem::on_finalize(1);

		assert_eq!(AllPalletsWithoutSystem::on_runtime_upgrade(), Weight::from_parts(61, 0));

		assert_eq!(
			pezframe_system::Pezpallet::<Runtime>::events()[0].event,
			RuntimeEvent::Example(pezpallet::Event::Something(10)),
		);
		assert_eq!(
			pezframe_system::Pezpallet::<Runtime>::events()[1].event,
			RuntimeEvent::Instance1Example(pezpallet::Event::Something(11)),
		);
		assert_eq!(
			pezframe_system::Pezpallet::<Runtime>::events()[2].event,
			RuntimeEvent::Example(pezpallet::Event::Something(20)),
		);
		assert_eq!(
			pezframe_system::Pezpallet::<Runtime>::events()[3].event,
			RuntimeEvent::Instance1Example(pezpallet::Event::Something(21)),
		);
		assert_eq!(
			pezframe_system::Pezpallet::<Runtime>::events()[4].event,
			RuntimeEvent::Example(pezpallet::Event::Something(30)),
		);
		assert_eq!(
			pezframe_system::Pezpallet::<Runtime>::events()[5].event,
			RuntimeEvent::Instance1Example(pezpallet::Event::Something(31)),
		);
	})
}

#[test]
fn pezpallet_on_genesis() {
	TestExternalities::default().execute_with(|| {
		pezpallet::Pezpallet::<Runtime>::on_genesis();

		pezpallet::Pezpallet::<Runtime, pezpallet::Instance1>::on_genesis();
	})
}

#[test]
fn metadata() {
	use frame_metadata::{v14::*, *};

	let system_pallet_metadata = PalletMetadata {
		index: 0,
		name: "System",
		storage: None, // The storage metadatas have been excluded.
		calls: Some(scale_info::meta_type::<pezframe_system::Call<Runtime>>().into()),
		event: Some(PalletEventMetadata {
			ty: scale_info::meta_type::<pezframe_system::Event<Runtime>>(),
		}),
		constants: vec![
			PalletConstantMetadata {
				name: "BlockWeights",
				ty: scale_info::meta_type::<pezframe_system::limits::BlockWeights>(),
				value: vec![],
				docs: vec![],
			},
			PalletConstantMetadata {
				name: "BlockLength",
				ty: scale_info::meta_type::<pezframe_system::limits::BlockLength>(),
				value: vec![],
				docs: vec![],
			},
			PalletConstantMetadata {
				name: "BlockHashCount",
				ty: scale_info::meta_type::<u32>(),
				value: vec![],
				docs: vec![],
			},
			PalletConstantMetadata {
				name: "DbWeight",
				ty: scale_info::meta_type::<pezframe_support::weights::RuntimeDbWeight>(),
				value: vec![],
				docs: vec![],
			},
			PalletConstantMetadata {
				name: "Version",
				ty: scale_info::meta_type::<pezsp_version::RuntimeVersion>(),
				value: vec![],
				docs: vec![],
			},
			PalletConstantMetadata {
				name: "SS58Prefix",
				ty: scale_info::meta_type::<u16>(),
				value: vec![],
				docs: vec![],
			},
		],
		error: Some(PalletErrorMetadata {
			ty: scale_info::meta_type::<pezframe_system::Error<Runtime>>(),
		}),
	};

	let example_pallet_metadata = PalletMetadata {
		index: 1,
		name: "Example",
		storage: Some(PalletStorageMetadata {
			prefix: "Example",
			entries: vec![
				StorageEntryMetadata {
					name: "Value",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Plain(scale_info::meta_type::<u32>()),
					default: vec![0],
					docs: vec![],
				},
				StorageEntryMetadata {
					name: "Map",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Map {
						key: scale_info::meta_type::<u8>(),
						value: scale_info::meta_type::<u16>(),
						hashers: vec![StorageHasher::Blake2_128Concat],
					},
					default: vec![0],
					docs: vec![],
				},
				StorageEntryMetadata {
					name: "Map2",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Map {
						key: scale_info::meta_type::<u16>(),
						value: scale_info::meta_type::<u32>(),
						hashers: vec![StorageHasher::Twox64Concat],
					},
					default: vec![0],
					docs: vec![],
				},
				StorageEntryMetadata {
					name: "Map3",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Map {
						key: scale_info::meta_type::<u32>(),
						value: scale_info::meta_type::<u64>(),
						hashers: vec![StorageHasher::Blake2_128Concat],
					},
					default: vec![0, 57, 5, 0, 0, 0, 0, 0, 0],
					docs: vec![],
				},
				StorageEntryMetadata {
					name: "DoubleMap",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Map {
						value: scale_info::meta_type::<u32>(),
						key: scale_info::meta_type::<(u8, u16)>(),
						hashers: vec![StorageHasher::Blake2_128Concat, StorageHasher::Twox64Concat],
					},
					default: vec![0],
					docs: vec![],
				},
				StorageEntryMetadata {
					name: "DoubleMap2",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Map {
						value: scale_info::meta_type::<u64>(),
						key: scale_info::meta_type::<(u16, u32)>(),
						hashers: vec![StorageHasher::Twox64Concat, StorageHasher::Blake2_128Concat],
					},
					default: vec![0],
					docs: vec![],
				},
				StorageEntryMetadata {
					name: "DoubleMap3",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Map {
						value: scale_info::meta_type::<u128>(),
						key: scale_info::meta_type::<(u32, u64)>(),
						hashers: vec![StorageHasher::Blake2_128Concat, StorageHasher::Twox64Concat],
					},
					default: vec![1, 1],
					docs: vec![],
				},
				StorageEntryMetadata {
					name: "NMap",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Map {
						key: scale_info::meta_type::<u8>(),
						hashers: vec![StorageHasher::Blake2_128Concat],
						value: scale_info::meta_type::<u32>(),
					},
					default: vec![0],
					docs: vec![],
				},
				StorageEntryMetadata {
					name: "NMap2",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Map {
						key: scale_info::meta_type::<(u16, u32)>(),
						hashers: vec![StorageHasher::Twox64Concat, StorageHasher::Blake2_128Concat],
						value: scale_info::meta_type::<u64>(),
					},
					default: vec![0],
					docs: vec![],
				},
				StorageEntryMetadata {
					name: "NMap3",
					modifier: StorageEntryModifier::Optional,
					ty: StorageEntryType::Map {
						key: scale_info::meta_type::<(u8, u16)>(),
						hashers: vec![StorageHasher::Blake2_128Concat, StorageHasher::Twox64Concat],
						value: scale_info::meta_type::<u128>(),
					},
					default: vec![1, 1],
					docs: vec![],
				},
			],
		}),
		calls: Some(scale_info::meta_type::<pezpallet::Call<Runtime>>().into()),
		event: Some(PalletEventMetadata {
			ty: scale_info::meta_type::<pezpallet::Event<Runtime>>(),
		}),
		constants: vec![PalletConstantMetadata {
			name: "MyGetParam",
			ty: scale_info::meta_type::<u32>(),
			value: vec![10, 0, 0, 0],
			docs: vec![],
		}],
		error: Some(PalletErrorMetadata {
			ty: scale_info::meta_type::<pezpallet::Error<Runtime>>(),
		}),
	};

	let mut example_pallet_instance1_metadata = example_pallet_metadata.clone();
	example_pallet_instance1_metadata.name = "Instance1Example";
	example_pallet_instance1_metadata.index = 2;
	match example_pallet_instance1_metadata.calls {
		Some(ref mut calls_meta) => {
			calls_meta.ty =
				scale_info::meta_type::<pezpallet::Call<Runtime, pezpallet::Instance1>>();
		},
		_ => unreachable!(),
	}
	match example_pallet_instance1_metadata.event {
		Some(ref mut event_meta) => {
			event_meta.ty =
				scale_info::meta_type::<pezpallet::Event<Runtime, pezpallet::Instance1>>();
		},
		_ => unreachable!(),
	}
	match example_pallet_instance1_metadata.error {
		Some(ref mut error_meta) => {
			error_meta.ty =
				scale_info::meta_type::<pezpallet::Error<Runtime, pezpallet::Instance1>>();
		},
		_ => unreachable!(),
	}
	match example_pallet_instance1_metadata.storage {
		Some(ref mut storage_meta) => {
			storage_meta.prefix = "Instance1Example";
		},
		_ => unreachable!(),
	}

	let pallets =
		vec![system_pallet_metadata, example_pallet_metadata, example_pallet_instance1_metadata];

	let extrinsic = ExtrinsicMetadata {
		ty: scale_info::meta_type::<UncheckedExtrinsic>(),
		version: 5,
		signed_extensions: vec![SignedExtensionMetadata {
			identifier: "UnitTransactionExtension",
			ty: scale_info::meta_type::<()>(),
			additional_signed: scale_info::meta_type::<()>(),
		}],
	};

	let expected_metadata: RuntimeMetadataPrefixed =
		RuntimeMetadataLastVersion::new(pallets, extrinsic, scale_info::meta_type::<Runtime>())
			.into();
	let expected_metadata = match expected_metadata.1 {
		RuntimeMetadata::V14(metadata) => metadata,
		_ => panic!("metadata has been bumped, test needs to be updated"),
	};

	let actual_metadata = match Runtime::metadata().1 {
		RuntimeMetadata::V14(metadata) => metadata,
		_ => panic!("metadata has been bumped, test needs to be updated"),
	};

	pretty_assertions::assert_eq!(actual_metadata.pallets[1], expected_metadata.pallets[1]);
	pretty_assertions::assert_eq!(actual_metadata.pallets[2], expected_metadata.pallets[2]);
}

#[test]
fn test_pallet_info_access() {
	assert_eq!(<System as pezframe_support::traits::PalletInfoAccess>::name(), "System");
	assert_eq!(<Example as pezframe_support::traits::PalletInfoAccess>::name(), "Example");
	assert_eq!(
		<Instance1Example as pezframe_support::traits::PalletInfoAccess>::name(),
		"Instance1Example"
	);
	assert_eq!(<Example2 as pezframe_support::traits::PalletInfoAccess>::name(), "Example2");
	assert_eq!(
		<Instance1Example2 as pezframe_support::traits::PalletInfoAccess>::name(),
		"Instance1Example2"
	);

	assert_eq!(<System as pezframe_support::traits::PalletInfoAccess>::index(), 0);
	assert_eq!(<Example as pezframe_support::traits::PalletInfoAccess>::index(), 1);
	assert_eq!(<Instance1Example as pezframe_support::traits::PalletInfoAccess>::index(), 2);
	assert_eq!(<Example2 as pezframe_support::traits::PalletInfoAccess>::index(), 3);
	assert_eq!(<Instance1Example2 as pezframe_support::traits::PalletInfoAccess>::index(), 4);
}

#[test]
fn test_storage_alias() {
	#[pezframe_support::storage_alias]
	type Value<T: pezpallet::Config<I>, I: 'static> =
		StorageValue<pezpallet::Pezpallet<T, I>, u32, ValueQuery>;

	TestExternalities::default().execute_with(|| {
		pezpallet::Value::<Runtime, pezpallet::Instance1>::put(10);
		assert_eq!(10, Value::<Runtime, pezpallet::Instance1>::get());
	})
}
