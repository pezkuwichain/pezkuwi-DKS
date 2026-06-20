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

use pezframe_support::construct_runtime;
use pezsp_runtime::{generic, traits::BlakeTwo256};
use pezsp_core::sr25519;

#[pezframe_support::pezpallet]
mod pezpallet {
	use pezframe_support::pezpallet_prelude::*;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	#[pezpallet::storage]
	type Foo<T> = StorageValue<Value=u8>;
}

pub type Signature = sr25519::Signature;
pub type BlockNumber = u64;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<u32, RuntimeCall, Signature, ()>;

impl pezpallet::Config for Runtime {}

construct_runtime! {
	pub struct Runtime
	{
		System: system::{Pezpallet, Call, Storage, Config<T>, Event<T>},
		Pezpallet: pezpallet use_parts { Call },
	}
}

fn main() {}
