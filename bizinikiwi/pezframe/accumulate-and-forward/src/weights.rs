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

//! Placeholder weights for `pezpallet_accumulate_and_forward`.
//!
//! These weights are not benchmarked. Replace with actual benchmarked weights
//! via `pezframe-omni-bencher` before deploying to production.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use pezframe_support::weights::{constants::RocksDbWeight, Weight};

/// Weight functions needed for `pezpallet_accumulate_and_forward`.
pub trait WeightInfo {
	fn send_native() -> Weight;
}

/// Default weights (not benchmarked).
/// Not a call weight -- a budget guard, and zero disabled it.
///
/// `send_native()` is read inside `on_initialize` as `meter.try_consume(...)`, so the pallet
/// can decline to forward when the block has no room left. At `Weight::zero()` the consume
/// always succeeds and the guard can never refuse: the XCM send happens whatever the block
/// has left. That is worse than a free extrinsic, which at least only costs the caller
/// nothing; this one spends a budget it did not check.
///
/// The figure below is a stand-in and is deliberately generous, because the failure it
/// prevents is a block that overruns. The pallet is benchmarked; the measurement replaces it.
impl WeightInfo for () {
	fn send_native() -> Weight {
		// One teleport: read the accumulation account, burn locally, send the message.
		Weight::from_parts(500_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(4))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
}
