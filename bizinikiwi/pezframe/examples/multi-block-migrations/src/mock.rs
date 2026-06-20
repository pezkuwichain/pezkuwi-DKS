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

#![cfg(test)]

//! # Mock runtime for testing Multi-Block-Migrations
//!
//! This runtime is for testing only and should *never* be used in production. Please see the
//! comments on the specific config items. The core part of this runtime is the
//! [`pezpallet_migrations::Config`] implementation, where you define the migrations you want to run
//! using the [`Migrations`] type.

use pezframe_support::{
	construct_runtime, derive_impl, migrations::MultiStepMigrator, pezpallet_prelude::Weight,
};

type Block = pezframe_system::mocking::MockBlock<Runtime>;

impl crate::Config for Runtime {}

pezframe_support::parameter_types! {
	pub storage MigratorServiceWeight: Weight = Weight::from_parts(100, 100); // do not use in prod
}

#[derive_impl(pezpallet_migrations::config_preludes::TestDefaultConfig)]
impl pezpallet_migrations::Config for Runtime {
	// Here we inject the actual MBMs. Currently there is just one, but it accepts a tuple.
	//
	// # Example
	// ```ignore
	// type Migrations = (v1::Migration<Runtime>, v2::Migration<Runtime>, v3::Migration<Runtime>);
	// ```
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Migrations = (
		crate::migrations::v1::LazyMigrationV1<
			Runtime,
			crate::migrations::v1::weights::BizinikiwiWeight<Runtime>,
		>,
	);
	#[cfg(feature = "runtime-benchmarks")]
	type Migrations = pezpallet_migrations::mock_helpers::MockedMigrations;
	type MaxServiceWeight = MigratorServiceWeight;
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type Block = Block;
	type MultiBlockMigrator = Migrator;
}

// Construct the runtime using the `construct_runtime` macro, specifying the pezpallet_migrations.
construct_runtime! {
	pub struct Runtime
	{
		System: pezframe_system,
		Pezpallet: crate,
		Migrator: pezpallet_migrations,
	}
}

pub fn new_test_ext() -> pezsp_io::TestExternalities {
	pezsp_io::TestExternalities::new(Default::default())
}

#[allow(dead_code)]
pub fn run_to_block(n: u64) {
	System::run_to_block_with::<AllPalletsWithSystem>(
		n,
		pezframe_system::RunToBlockHooks::default().after_initialize(|_| {
			// Done by Executive:
			<Runtime as pezframe_system::Config>::MultiBlockMigrator::step();
		}),
	);
}
