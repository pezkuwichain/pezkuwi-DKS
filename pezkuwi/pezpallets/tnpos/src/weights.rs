// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Weights for pezpallet-tnpos.
//!
//! The zero implementation exists so tests can run before the benchmark pass; it is not a
//! weight anyone may ship. Task 13 replaces it with measured values from CI.

use pezframe_support::weights::{constants::RocksDbWeight, Weight};

pub trait WeightInfo {
	fn join() -> Weight;
	fn leave() -> Weight;
	fn set_strata() -> Weight;
	fn report_offence() -> Weight;
	fn commit_seed() -> Weight;
	fn reveal_seed() -> Weight;
	/// `p` is the pool size: seating iterates `PoolMembers` once per seated stratum, so the
	/// cost is linear in it and a constant here would be a lie the block budget pays for.
	fn seat_committee(p: u32) -> Weight;
}

/// Not measured, and deliberately not zero -- and measured now says these are too low.
///
/// This impl is the fallback for a runtime that has not generated weights. It returned
/// `Weight::zero()` for all seven calls until 2026-08-30, which made every TNPoS extrinsic
/// free on both People chains. The figures below replaced the zeros as a stand-in, and the
/// benchmark run that followed showed the stand-in undercharging five of the seven: `join` by
/// 1.8x, `report_offence` by 9x, and `seat_committee` by 107x -- its measured base alone is
/// 21.5 billion ref_time, one per cent of a block, because it draws from nine strata.
///
/// They are left as they are rather than raised to match, because both production runtimes now
/// bind the generated file and nothing reads this. What it must never be again is zero. A
/// runtime that binds `()` is a runtime whose weights were never generated, and this should
/// cost it visibly rather than silently.
///
/// This returned `Weight::zero()` for all seven calls, and both People runtimes bound
/// `WeightInfo = ()` -- so every TNPoS extrinsic was free, `join` and `commit_seed` included,
/// each of which writes storage. A free call is a free block.
///
/// The figures below are a ceiling, not a measurement: one read and one write of a bounded
/// item at the reference machine's own DbWeight, plus a flat execution allowance. They
/// overcharge, which costs a caller a little; zero undercharges by everything. The pallet is
/// in both People runtimes' `define_benchmarks!` now, so the next weights run replaces this
/// whole impl with real numbers.
impl WeightInfo for () {
	fn join() -> Weight {
		// Reads the five register scores and the key register; writes pool membership.
		Weight::from_parts(50_000_000, 4_000)
			.saturating_add(RocksDbWeight::get().reads(7))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	fn leave() -> Weight {
		Weight::from_parts(30_000_000, 3_000)
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	fn set_strata() -> Weight {
		Weight::from_parts(30_000_000, 3_000)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	fn report_offence() -> Weight {
		Weight::from_parts(30_000_000, 3_000)
			.saturating_add(RocksDbWeight::get().reads(3))
			.saturating_add(RocksDbWeight::get().writes(4))
	}
	fn commit_seed() -> Weight {
		Weight::from_parts(30_000_000, 3_000)
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	fn reveal_seed() -> Weight {
		Weight::from_parts(30_000_000, 3_000)
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	fn seat_committee(p: u32) -> Weight {
		// Draws from every stratum and writes the committee; scales with the pool.
		Weight::from_parts(200_000_000, 20_000)
			.saturating_add(Weight::from_parts(500_000, 0).saturating_mul(p.into()))
			.saturating_add(RocksDbWeight::get().reads(12))
			.saturating_add(RocksDbWeight::get().writes(4))
	}
}
