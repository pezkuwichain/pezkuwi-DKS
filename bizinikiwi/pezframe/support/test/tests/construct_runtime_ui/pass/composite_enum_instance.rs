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

#![cfg_attr(not(feature = "std"), no_std)]

use pezframe_support::derive_impl;

pub use pezpallet::*;

#[pezframe_support::pezpallet(dev_mode)]
pub mod pezpallet {
	use pezframe_support::pezpallet_prelude::*;

	// The struct on which we build all of our Pezpallet logic.
	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(PhantomData<(T, I)>);

	// Your Pezpallet's configuration trait, representing custom external types and interfaces.
	#[pezpallet::config]
	pub trait Config<I: 'static = ()>: pezframe_system::Config {}
	
	#[pezpallet::composite_enum]
	pub enum HoldReason<I: 'static = ()> {
		SomeHoldReason
	}

	#[pezpallet::composite_enum]
	pub enum FreezeReason<I: 'static = ()> {
		SomeFreezeReason
	}

	#[pezpallet::composite_enum]
	pub enum SlashReason<I: 'static = ()> {
		SomeSlashReason
	}

	#[pezpallet::composite_enum]
	pub enum LockId<I: 'static = ()> {
		SomeLockId
	}
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type Block = Block;
}

pub type Header = pezsp_runtime::generic::Header<u64, pezsp_runtime::traits::BlakeTwo256>;
pub type UncheckedExtrinsic = pezsp_runtime::generic::UncheckedExtrinsic<u64, RuntimeCall, (), ()>;
pub type Block = pezsp_runtime::generic::Block<Header, UncheckedExtrinsic>;

pezframe_support::construct_runtime!(
	pub enum Runtime
	{
		// Exclude part `Storage` in order not to check its metadata in tests.
		System: pezframe_system,
		Pallet1: pezpallet,
		Pallet2: pezpallet::<Instance2>,
	}
);

impl pezpallet::Config for Runtime {}

impl pezpallet::Config<pezpallet::Instance2> for Runtime {}

fn main() {}
