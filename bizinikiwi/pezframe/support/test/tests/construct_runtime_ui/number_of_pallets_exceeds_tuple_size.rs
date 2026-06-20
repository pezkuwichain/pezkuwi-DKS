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

use pezframe_support::{construct_runtime, derive_impl};
use pezsp_core::sr25519;
use pezsp_runtime::{generic, traits::BlakeTwo256};

#[pezframe_support::pezpallet]
mod pezpallet {
	#[pezpallet::config]
	pub trait Config: pezframe_system::Config {}

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(core::marker::PhantomData<T>);
}

pub type Signature = sr25519::Signature;
pub type BlockNumber = u32;
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<u32, RuntimeCall, Signature, ()>;

impl pezpallet::Config for Runtime {}

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
	type BlockHashCount = pezframe_support::traits::ConstU32<250>;
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
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
}

construct_runtime! {
	pub struct Runtime
	{
		System: pezframe_system::{Pezpallet, Call, Storage, Config<T>, Event<T>},
		Pallet1: pezpallet::{Pezpallet},
		Pallet2: pezpallet::{Pezpallet},
		Pallet3: pezpallet::{Pezpallet},
		Pallet4: pezpallet::{Pezpallet},
		Pallet5: pezpallet::{Pezpallet},
		Pallet6: pezpallet::{Pezpallet},
		Pallet7: pezpallet::{Pezpallet},
		Pallet8: pezpallet::{Pezpallet},
		Pallet9: pezpallet::{Pezpallet},
		Pallet10: pezpallet::{Pezpallet},
		Pallet11: pezpallet::{Pezpallet},
		Pallet12: pezpallet::{Pezpallet},
		Pallet13: pezpallet::{Pezpallet},
		Pallet14: pezpallet::{Pezpallet},
		Pallet15: pezpallet::{Pezpallet},
		Pallet16: pezpallet::{Pezpallet},
		Pallet17: pezpallet::{Pezpallet},
		Pallet18: pezpallet::{Pezpallet},
		Pallet19: pezpallet::{Pezpallet},
		Pallet20: pezpallet::{Pezpallet},
		Pallet21: pezpallet::{Pezpallet},
		Pallet22: pezpallet::{Pezpallet},
		Pallet23: pezpallet::{Pezpallet},
		Pallet24: pezpallet::{Pezpallet},
		Pallet25: pezpallet::{Pezpallet},
		Pallet26: pezpallet::{Pezpallet},
		Pallet27: pezpallet::{Pezpallet},
		Pallet28: pezpallet::{Pezpallet},
		Pallet29: pezpallet::{Pezpallet},
		Pallet30: pezpallet::{Pezpallet},
		Pallet31: pezpallet::{Pezpallet},
		Pallet32: pezpallet::{Pezpallet},
		Pallet33: pezpallet::{Pezpallet},
		Pallet34: pezpallet::{Pezpallet},
		Pallet35: pezpallet::{Pezpallet},
		Pallet36: pezpallet::{Pezpallet},
		Pallet37: pezpallet::{Pezpallet},
		Pallet38: pezpallet::{Pezpallet},
		Pallet39: pezpallet::{Pezpallet},
		Pallet40: pezpallet::{Pezpallet},
		Pallet41: pezpallet::{Pezpallet},
		Pallet42: pezpallet::{Pezpallet},
		Pallet43: pezpallet::{Pezpallet},
		Pallet44: pezpallet::{Pezpallet},
		Pallet45: pezpallet::{Pezpallet},
		Pallet46: pezpallet::{Pezpallet},
		Pallet47: pezpallet::{Pezpallet},
		Pallet48: pezpallet::{Pezpallet},
		Pallet49: pezpallet::{Pezpallet},
		Pallet50: pezpallet::{Pezpallet},
		Pallet51: pezpallet::{Pezpallet},
		Pallet52: pezpallet::{Pezpallet},
		Pallet53: pezpallet::{Pezpallet},
		Pallet54: pezpallet::{Pezpallet},
		Pallet55: pezpallet::{Pezpallet},
		Pallet56: pezpallet::{Pezpallet},
		Pallet57: pezpallet::{Pezpallet},
		Pallet58: pezpallet::{Pezpallet},
		Pallet59: pezpallet::{Pezpallet},
		Pallet60: pezpallet::{Pezpallet},
		Pallet61: pezpallet::{Pezpallet},
		Pallet62: pezpallet::{Pezpallet},
		Pallet63: pezpallet::{Pezpallet},
		Pallet64: pezpallet::{Pezpallet},
		Pallet65: pezpallet::{Pezpallet},
		Pallet66: pezpallet::{Pezpallet},
		Pallet67: pezpallet::{Pezpallet},
		Pallet68: pezpallet::{Pezpallet},
		Pallet69: pezpallet::{Pezpallet},
		Pallet70: pezpallet::{Pezpallet},
		Pallet71: pezpallet::{Pezpallet},
		Pallet72: pezpallet::{Pezpallet},
		Pallet73: pezpallet::{Pezpallet},
		Pallet74: pezpallet::{Pezpallet},
		Pallet75: pezpallet::{Pezpallet},
		Pallet76: pezpallet::{Pezpallet},
		Pallet77: pezpallet::{Pezpallet},
		Pallet78: pezpallet::{Pezpallet},
		Pallet79: pezpallet::{Pezpallet},
		Pallet80: pezpallet::{Pezpallet},
		Pallet81: pezpallet::{Pezpallet},
		Pallet82: pezpallet::{Pezpallet},
		Pallet83: pezpallet::{Pezpallet},
		Pallet84: pezpallet::{Pezpallet},
		Pallet85: pezpallet::{Pezpallet},
		Pallet86: pezpallet::{Pezpallet},
		Pallet87: pezpallet::{Pezpallet},
		Pallet88: pezpallet::{Pezpallet},
		Pallet89: pezpallet::{Pezpallet},
		Pallet90: pezpallet::{Pezpallet},
		Pallet91: pezpallet::{Pezpallet},
		Pallet92: pezpallet::{Pezpallet},
		Pallet93: pezpallet::{Pezpallet},
		Pallet94: pezpallet::{Pezpallet},
		Pallet95: pezpallet::{Pezpallet},
		Pallet96: pezpallet::{Pezpallet},
		Pallet97: pezpallet::{Pezpallet},
		Pallet98: pezpallet::{Pezpallet},
		Pallet99: pezpallet::{Pezpallet},
		Pallet100: pezpallet::{Pezpallet},
		Pallet101: pezpallet::{Pezpallet},
		Pallet102: pezpallet::{Pezpallet},
		Pallet103: pezpallet::{Pezpallet},
		Pallet104: pezpallet::{Pezpallet},
		Pallet105: pezpallet::{Pezpallet},
		Pallet106: pezpallet::{Pezpallet},
		Pallet107: pezpallet::{Pezpallet},
		Pallet108: pezpallet::{Pezpallet},
		Pallet109: pezpallet::{Pezpallet},
		Pallet110: pezpallet::{Pezpallet},
	}
}

fn main() {}
