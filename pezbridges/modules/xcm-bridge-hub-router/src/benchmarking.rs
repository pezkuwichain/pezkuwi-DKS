// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! XCM bridge hub router pezpallet benchmarks.

#![cfg(feature = "runtime-benchmarks")]

use crate::{Bridge, BridgeState, Call};
use pezframe_benchmarking::{benchmarks_instance_pallet, BenchmarkError};
use pezframe_support::traits::{EnsureOrigin, Get, Hooks, UnfilteredDispatchable};
use pezkuwi_runtime_teyrchains::FeeTracker;
use pezsp_runtime::{traits::Zero, Saturating};
use xcm::prelude::*;

/// Pezpallet we're benchmarking here.
pub struct Pezpallet<T: Config<I>, I: 'static = ()>(crate::Pezpallet<T, I>);

/// Trait that must be implemented by runtime to be able to benchmark pezpallet properly.
pub trait Config<I: 'static>: crate::Config<I> {
	/// Fill up queue so it becomes congested.
	fn make_congested();

	/// Returns destination which is valid for this router instance.
	/// (Needs to pass `T::Bridges`)
	/// Make sure that `SendXcm` will pass.
	fn ensure_bridged_target_destination() -> Result<Location, BenchmarkError> {
		Ok(Location::new(
			Self::UniversalLocation::get().len() as u8,
			[GlobalConsensus(Self::BridgedNetworkId::get().unwrap())],
		))
	}
}

benchmarks_instance_pallet! {
	on_initialize_when_non_congested {
		Bridge::<T, I>::put(BridgeState {
			is_congested: false,
			delivery_fee_factor: crate::Pezpallet::<T, I>::MIN_FEE_FACTOR.saturating_mul(2.into()),
		});
	}: {
		crate::Pezpallet::<T, I>::on_initialize(Zero::zero())
	}

	on_initialize_when_congested {
		Bridge::<T, I>::put(BridgeState {
			is_congested: false,
			delivery_fee_factor: crate::Pezpallet::<T, I>::MIN_FEE_FACTOR.saturating_mul(2.into()),
		});
		let _ = T::ensure_bridged_target_destination()?;
		T::make_congested();
	}: {
		crate::Pezpallet::<T, I>::on_initialize(Zero::zero())
	}

	report_bridge_status {
		Bridge::<T, I>::put(BridgeState::default());

		let origin: T::RuntimeOrigin = T::BridgeHubOrigin::try_successful_origin().expect("expected valid BridgeHubOrigin");
		let bridge_id = Default::default();
		let is_congested = true;

		let call = Call::<T, I>::report_bridge_status { bridge_id, is_congested };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert!(Bridge::<T, I>::get().is_congested);
	}
}
