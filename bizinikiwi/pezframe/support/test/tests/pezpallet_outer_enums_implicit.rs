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

use pezframe_support::derive_impl;

mod common;

pub type Header = pezsp_runtime::generic::Header<u32, pezsp_runtime::traits::BlakeTwo256>;
pub type Block = pezsp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = pezsp_runtime::generic::UncheckedExtrinsic<u32, RuntimeCall, (), ()>;

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type Block = Block;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletInfo = PalletInfo;
	type OnSetCode = ();
}

impl common::outer_enums::pezpallet::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}
impl common::outer_enums::pezpallet::Config<common::outer_enums::pezpallet::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
}
impl common::outer_enums::pallet2::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}
impl common::outer_enums::pallet2::Config<common::outer_enums::pezpallet::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
}
impl common::outer_enums::pallet3::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}
impl common::outer_enums::pallet3::Config<common::outer_enums::pezpallet::Instance1> for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

pezframe_support::construct_runtime!(
	pub struct Runtime
	{
		// Exclude part `Storage` in order not to check its metadata in tests.
		System: pezframe_system exclude_parts { Storage },

		// Pezpallet exposes `Error` implicitly.
		Example: common::outer_enums::pezpallet,
		Instance1Example: common::outer_enums::pezpallet::<Instance1>,

		// Pezpallet exposes `Error` implicitly.
		Example2: common::outer_enums::pallet2,
		Instance1Example2: common::outer_enums::pallet2::<Instance1>,

		// Pezpallet does not implement error.
		Example3: common::outer_enums::pallet3,
		Instance1Example3: common::outer_enums::pallet3::<Instance1>,
	}
);

#[cfg(feature = "experimental")]
#[test]
fn module_error_outer_enum_expand_implicit() {
	use common::outer_enums::{pallet2, pezpallet};
	// The Runtime has *all* parts implicitly defined.

	// Check that all error types are propagated
	match RuntimeError::Example(pezpallet::Error::InsufficientProposersBalance) {
		// Error passed implicitly to the pezpallet system.
		RuntimeError::System(system) => match system {
			pezframe_system::Error::InvalidSpecName => (),
			pezframe_system::Error::SpecVersionNeedsToIncrease => (),
			pezframe_system::Error::FailedToExtractRuntimeVersion => (),
			pezframe_system::Error::NonDefaultComposite => (),
			pezframe_system::Error::NonZeroRefCount => (),
			pezframe_system::Error::CallFiltered => (),
			pezframe_system::Error::MultiBlockMigrationsOngoing => (),
			pezframe_system::Error::InvalidTask => (),
			pezframe_system::Error::FailedTask => (),
			pezframe_system::Error::NothingAuthorized => (),
			pezframe_system::Error::Unauthorized => (),
			pezframe_system::Error::__Ignore(_, _) => (),
		},

		// Error declared explicitly.
		RuntimeError::Example(example) => match example {
			pezpallet::Error::InsufficientProposersBalance => (),
			pezpallet::Error::NonExistentStorageValue => (),
			pezpallet::Error::__Ignore(_, _) => (),
		},
		// Error declared explicitly.
		RuntimeError::Instance1Example(example) => match example {
			pezpallet::Error::InsufficientProposersBalance => (),
			pezpallet::Error::NonExistentStorageValue => (),
			pezpallet::Error::__Ignore(_, _) => (),
		},

		// Error must propagate even if not defined explicitly as pezpallet part.
		RuntimeError::Example2(example) => match example {
			pallet2::Error::OtherInsufficientProposersBalance => (),
			pallet2::Error::OtherNonExistentStorageValue => (),
			pallet2::Error::__Ignore(_, _) => (),
		},
		// Error must propagate even if not defined explicitly as pezpallet part.
		RuntimeError::Instance1Example2(example) => match example {
			pallet2::Error::OtherInsufficientProposersBalance => (),
			pallet2::Error::OtherNonExistentStorageValue => (),
			pallet2::Error::__Ignore(_, _) => (),
		},
	};
}
