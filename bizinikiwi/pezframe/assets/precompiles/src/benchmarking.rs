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

//! Benchmarks for `pezpallet_assets_precompiles`.
//!
//! Registered under `foreign_assets::Pezpallet`; the run writes `weights.rs`. The permit
//! benchmarks live in `permit_benchmarking.rs` under their own pallet, because a runtime that
//! has the permit pallet but not the foreign-asset mapping pallet still has to measure them.

#![cfg(feature = "runtime-benchmarks")]

use crate::{
	foreign_assets::pezpallet::{Config, Pezpallet},
	migration::MigrateForeignAssetPrecompileMappings,
};
use pezframe_benchmarking::v2::*;
use pezframe_support::{migrations::SteppedMigration, weights::WeightMeter};
use pezsp_runtime::traits::StaticLookup;

#[benchmarks(
	where
		// Migration bounds
		T: pezpallet_assets::Config<T::AssetsInstance, AssetId = <T as Config>::ForeignAssetId>,
		<T as pezpallet_assets::Config<T::AssetsInstance>>::Balance: From<u32>,
		<T as pezpallet_assets::Config<T::AssetsInstance>>::AssetIdParameter: From<<T as pezpallet_assets::Config<T::AssetsInstance>>::AssetId>,
)]
mod benchmarks {
	use super::*;
	use pezpallet_assets::BenchmarkHelper;

	// ==================== Migration benchmarks ====================

	/// Benchmark one complete `step()` invocation of the
	/// [`MigrateForeignAssetPrecompileMappings`] stepped migration.
	#[benchmark]
	fn migrate_foreign_asset_step() {
		// Clear any pre-existing assets from genesis so that only our
		// benchmark asset is present during the migration step.
		let _ = pezpallet_assets::Asset::<T, T::AssetsInstance>::clear(u32::MAX, None);

		// Create one asset in pezpallet_assets storage.
		let caller: T::AccountId = whitelisted_caller();
		let caller_lookup = <T as pezframe_system::Config>::Lookup::unlookup(caller);
		let asset_id_param = <T as pezpallet_assets::Config<T::AssetsInstance>>::BenchmarkHelper::create_asset_id_parameter(42);
		let asset_id: <T as pezpallet_assets::Config<T::AssetsInstance>>::AssetId =
			asset_id_param.clone().into();

		pezpallet_assets::Pezpallet::<T, T::AssetsInstance>::force_create(
			pezframe_system::RawOrigin::Root.into(),
			asset_id_param,
			caller_lookup,
			true,
			1u32.into(),
		)
		.unwrap();

		// Remove the mapping that was auto-created by the AssetsCallback hook
		Pezpallet::<T>::remove_asset_mapping(&asset_id);

		// Verify no precompile mapping exists yet.
		assert!(Pezpallet::<T>::asset_index_of(&asset_id).is_none());

		let mut meter = WeightMeter::new();

		#[block]
		{
			MigrateForeignAssetPrecompileMappings::<T, T::AssetsInstance, ()>::step(
				None, &mut meter,
			)
			.unwrap();
		}

		// Verify the asset was migrated.
		assert!(Pezpallet::<T>::asset_index_of(&asset_id).is_some());
		// The step consumes the weight twice: once for migrating the asset and once for
		// discovering that there are no more assets to migrate.
		assert_eq!(
			meter.consumed(),
			<() as crate::weights::WeightInfo>::migrate_foreign_asset_step() * 2
		);
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::Test);
}
