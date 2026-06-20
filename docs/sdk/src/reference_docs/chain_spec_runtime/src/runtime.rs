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

//! A minimal runtime that shows runtime genesis state.

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use crate::{
	pallets::{pezpallet_bar, pezpallet_foo},
	presets::*,
};
use alloc::{vec, vec::Vec};
use pezframe::{
	deps::pezframe_support::genesis_builder_helper::{build_state, get_preset},
	prelude::*,
	runtime::prelude::*,
};
use pezsp_api::impl_runtime_apis;
use pezsp_genesis_builder::PresetId;
use pezsp_runtime::traits::Block as BlockT;

/// The runtime version.
#[runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("pez-minimal-template-runtime"),
	impl_name: alloc::borrow::Cow::Borrowed("pez-minimal-template-runtime"),
	authoring_version: 1,
	spec_version: 0,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

/// The signed extensions that are added to the runtime.
type SignedExtra = ();

// Composes the runtime by adding all the used pallets and deriving necessary types.
#[frame_construct_runtime]
mod runtime {
	/// The main runtime type.
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Runtime;

	/// Mandatory system pezpallet that should always be included in a FRAME runtime.
	#[runtime::pezpallet_index(0)]
	pub type System = pezframe_system::Pezpallet<Runtime>;

	/// Sample pezpallet 1
	#[runtime::pezpallet_index(1)]
	pub type Bar = pezpallet_bar::Pezpallet<Runtime>;

	/// Sample pezpallet 2
	#[runtime::pezpallet_index(2)]
	pub type Foo = pezpallet_foo::Pezpallet<Runtime>;
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
}

/// Implements the types required for the system pezpallet.
#[derive_impl(pezframe_system::config_preludes::SolochainDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type Block = Block;
	type Version = Version;
}

impl pezpallet_bar::Config for Runtime {}
impl pezpallet_foo::Config for Runtime {}

type Block = pezframe::runtime::types_common::BlockOf<Runtime, SignedExtra>;
type _Header = HeaderFor<Runtime>;

#[docify::export(runtime_impl)]
impl_runtime_apis! {
	impl pezsp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> pezsp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<pezsp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, get_builtin_preset)
		}

		fn preset_names() -> Vec<pezsp_genesis_builder::PresetId> {
			vec![
				PresetId::from(PRESET_1),
				PresetId::from(PRESET_2),
				PresetId::from(PRESET_3),
				PresetId::from(PRESET_4),
				PresetId::from(PRESET_INVALID)
			]
		}
	}

	impl pezsp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion { VERSION }
		fn execute_block(_: <Block as BlockT>::LazyBlock) { }
		fn initialize_block(_: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode { ExtrinsicInclusionMode::default() }
	}
}
