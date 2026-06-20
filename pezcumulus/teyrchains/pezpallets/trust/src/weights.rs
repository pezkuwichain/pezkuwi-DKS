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


//! Weights for `pezpallet_trust`
//!
//! Originally auto-generated using BIZINIKIWI BENCHMARK CLI VERSION 32.0.0
//! DATE: 2025-12-08 (original), 2026-02-16 (manually adjusted)
//!
//! ADJUSTED: Added +2 reads per account for CachedStakingDetails iter_prefix
//! (staking score calculation now aggregates from StorageDoubleMap instead of noop).
//! TODO: Run proper benchmarks to replace these adjusted estimates.

// Executed Command:
// ./target/release/frame-omni-bencher
// v1
// benchmark
// pezpallet
// --runtime
// target/release/wbuild/people-pezkuwichain-runtime/people_pezkuwichain_runtime.compact.compressed.wasm
// --pallets
// pezpallet_trust
// -e
// all
// --steps
// 50
// --repeat
// 20
// --output
// pezcumulus/teyrchains/pezpallets/trust/src/weights.rs
// --template
// bizinikiwi/.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]
#![allow(dead_code)]

use pezframe_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pezpallet_trust`.
pub trait WeightInfo {
	fn force_recalculate_trust_score() -> Weight;
	fn update_all_trust_scores() -> Weight;
	fn periodic_trust_score_update() -> Weight;
}

/// Weights for `pezpallet_trust` using the Bizinikiwi node and recommended hardware.
pub struct BizinikiwiWeight<T>(PhantomData<T>);
impl<T: pezframe_system::Config> WeightInfo for BizinikiwiWeight<T> {
	/// Storage: `Trust::TrustScores` (r:1 w:1)
	/// Storage: `StakingScore::StakingStartBlock` (r:1 w:0)
	/// Storage: `StakingScore::CachedStakingDetails` (r:2 w:0) -- iter_prefix for score calc
	/// Storage: `Referral::ReferralCount` (r:1 w:0)
	/// Storage: `Tiki::UserTikis` (r:1 w:0)
	/// Storage: `Trust::TotalActiveTrustScore` (r:1 w:1)
	/// Storage: `IdentityKyc::KycStatuses` (r:1 w:0) -- citizenship check
	///
	/// Total: r:8 w:2 (adjusted +3 reads from original benchmark)
	fn force_recalculate_trust_score() -> Weight {
		Weight::from_parts(65_000_000, 5000)
			.saturating_add(T::DbWeight::get().reads(8_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: `Trust::LastProcessedAccount` (r:1 w:1)
	/// Storage: `IdentityKyc::KycStatuses` (r:2 w:0)
	/// Storage: `Trust::TrustScores` (r:1 w:1)
	/// Storage: `StakingScore::StakingStartBlock` (r:1 w:0)
	/// Storage: `StakingScore::CachedStakingDetails` (r:2 w:0) -- iter_prefix for score calc
	/// Storage: `Referral::ReferralCount` (r:1 w:0)
	/// Storage: `Tiki::UserTikis` (r:1 w:0)
	/// Storage: `Trust::TotalActiveTrustScore` (r:1 w:1)
	/// Storage: `Trust::BatchUpdateInProgress` (r:0 w:1)
	///
	/// Total: r:10 w:4 (adjusted +2 reads from original benchmark)
	fn update_all_trust_scores() -> Weight {
		Weight::from_parts(85_000_000, 7000)
			.saturating_add(T::DbWeight::get().reads(10_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	/// Storage: `Trust::BatchUpdateInProgress` (r:1 w:1)
	/// Storage: `Trust::LastProcessedAccount` (r:1 w:1)
	/// Storage: `IdentityKyc::KycStatuses` (r:2 w:0)
	/// Storage: `Trust::TrustScores` (r:1 w:1)
	/// Storage: `StakingScore::StakingStartBlock` (r:1 w:0)
	/// Storage: `StakingScore::CachedStakingDetails` (r:2 w:0) -- iter_prefix for score calc
	/// Storage: `Referral::ReferralCount` (r:1 w:0)
	/// Storage: `Tiki::UserTikis` (r:1 w:0)
	/// Storage: `Trust::TotalActiveTrustScore` (r:1 w:1)
	///
	/// Total: r:11 w:4 (adjusted +2 reads from original benchmark)
	fn periodic_trust_score_update() -> Weight {
		Weight::from_parts(100_000_000, 7000)
			.saturating_add(T::DbWeight::get().reads(11_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	fn force_recalculate_trust_score() -> Weight {
		Weight::from_parts(65_000_000, 5000)
			.saturating_add(RocksDbWeight::get().reads(8_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}

	fn update_all_trust_scores() -> Weight {
		Weight::from_parts(85_000_000, 7000)
			.saturating_add(RocksDbWeight::get().reads(10_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}

	fn periodic_trust_score_update() -> Weight {
		Weight::from_parts(100_000_000, 7000)
			.saturating_add(RocksDbWeight::get().reads(11_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
}
