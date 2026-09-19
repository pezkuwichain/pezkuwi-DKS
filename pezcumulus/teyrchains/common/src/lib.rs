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

extern crate alloc;

pub mod impls;
pub mod message_queue;
pub mod pay;
pub mod xcm_config;
pub use constants::*;
pub use teyrchains_common_types::{opaque::*, *};

/// Common constants of teyrchains.
mod constants {
	use pezframe_support::{
		weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
		PalletId,
	};
	use pezsp_runtime::Perbill;
	use teyrchains_common_types::BlockNumber;

	/// This determines the average expected block time that we are targeting. Blocks will be
	/// produced at a minimum duration defined by `SLOT_DURATION`. `SLOT_DURATION` is picked up by
	/// `pezpallet_timestamp` which is in turn picked up by `pezpallet_aura` to implement `fn
	/// slot_duration()`.
	///
	/// Change this to adjust the block time.
	///
	/// **This is upstream's default and it does not govern any chain in this tree.** Every
	/// Pezkuwi teyrchain takes its `SLOT_DURATION` from `testnet_teyrchains_constants::<chain>
	/// ::consensus`, where it is 6000 -- confirmed by asking a running chain for
	/// `AuraApi_slot_duration`. The constant is left here because the crate is upstream's and
	/// the rest of it is used.
	///
	/// The `MINUTES`/`HOURS`/`DAYS` below are derived from it and are therefore **half** a
	/// Pezkuwi teyrchain's real day. Do not import them into a runtime. Doing so is not a
	/// compile error and not a test failure; it silently halves every period built from them,
	/// which is what happened to eighty-one periods across the People and Asset Hub runtimes
	/// until 2026-09-18 -- a four-year term of office that ran two years, and every governance
	/// track deciding in half the time its name promised.
	///
	/// The runtimes now import time from their own chain's constants, and each holds a test
	/// comparing its `DAYS` against its own slot duration rather than against another
	/// constant, because comparing two constants is how the wrong one gets confirmed.
	pub const MILLISECS_PER_BLOCK: u64 = 12000;
	pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

	// Time is measured by number of blocks. See the warning on `MILLISECS_PER_BLOCK`: these are
	// upstream's twelve-second day, not this tree's six-second one.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
	/// used to limit the maximal weight of a single extrinsic.
	pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
	/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
	/// Operational  extrinsics.
	pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

	/// We allow for 0.5 seconds of compute with a 6 second average block time.
	pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
		WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
		pezkuwi_primitives::MAX_POV_SIZE as u64,
	);

	/// We allow for 2 seconds of compute with a 6 second average block.
	pub const MAXIMUM_BLOCK_WEIGHT_FOR_ASYNC_BACKING: Weight = Weight::from_parts(
		WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
		pezkuwi_primitives::MAX_POV_SIZE as u64,
	);

	/// Treasury pezpallet id of the local chain, used to convert into AccountId
	pub const TREASURY_PALLET_ID: PalletId = PalletId(*b"py/trsry");
}
