// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Weights for pezpallet-tnpos.
//!
//! Both People runtimes bind their own generated file (`weights/pezpallet_tnpos.rs`); the
//! `()` impl below is what the mock and any runtime without generated weights get.

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
	/// `n` is the number of accounts reported.
	fn note_bonded(n: u32) -> Weight;
	fn set_relay_keys() -> Weight;
	fn purge_relay_keys() -> Weight;
	fn switch_stratum() -> Weight;
	/// `n` is the number of validators the session report names, scored and failed together.
	fn note_session_performance(n: u32) -> Weight;
}

/// Measured, not estimated: each figure is the larger of the two People twins' generated
/// weights from the reference box (weights runs 39841537690 and 39841543012, 2026-10-23,
/// `--steps 50 --repeat 20`), with the runtime's `DbWeight` replaced by `RocksDbWeight`.
///
/// This impl returned `Weight::zero()` for every call until 2026-08-30, which made every TNPoS
/// extrinsic free; a hand-written ceiling replaced it and the first measurement showed that
/// ceiling undercharging five of the seven calls, `seat_committee` by 107x. Neither a zero nor
/// a guess is a weight: when a benchmark changes, the generated files are re-measured and
/// these figures are copied from them.
impl WeightInfo for () {
	fn join() -> Weight {
		Weight::from_parts(120_064_000, 34_255)
			.saturating_add(RocksDbWeight::get().reads(16))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	fn leave() -> Weight {
		Weight::from_parts(42_391_000, 3_514)
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	fn set_strata() -> Weight {
		Weight::from_parts(22_721_000, 1_489)
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	fn report_offence() -> Weight {
		Weight::from_parts(649_581_000, 67_004)
			.saturating_add(RocksDbWeight::get().reads(39))
			.saturating_add(RocksDbWeight::get().writes(5))
	}
	fn commit_seed() -> Weight {
		Weight::from_parts(37_521_000, 3_557)
			.saturating_add(RocksDbWeight::get().reads(4))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
	fn reveal_seed() -> Weight {
		Weight::from_parts(36_031_000, 3_557)
			.saturating_add(RocksDbWeight::get().reads(4))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	fn seat_committee(p: u32) -> Weight {
		Weight::from_parts(36_616_067_000, 128_790)
			.saturating_add(Weight::from_parts(61_596_474, 3_037).saturating_mul(p.into()))
			.saturating_add(RocksDbWeight::get().reads(36))
			.saturating_add(RocksDbWeight::get().reads((4_u64).saturating_mul(p.into())))
			.saturating_add(RocksDbWeight::get().writes(4))
	}
	fn note_bonded(n: u32) -> Weight {
		Weight::from_parts(32_030_923, 34_255)
			.saturating_add(Weight::from_parts(178_519, 0).saturating_mul(n.into()))
			.saturating_add(RocksDbWeight::get().reads(2))
			.saturating_add(RocksDbWeight::get().writes(2))
	}
	fn set_relay_keys() -> Weight {
		Weight::from_parts(768_924_000, 36_701)
			.saturating_add(RocksDbWeight::get().reads(7))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	fn purge_relay_keys() -> Weight {
		Weight::from_parts(118_603_000, 4_027)
			.saturating_add(RocksDbWeight::get().reads(7))
			.saturating_add(RocksDbWeight::get().writes(5))
	}
	fn switch_stratum() -> Weight {
		Weight::from_parts(204_347_000, 34_255)
			.saturating_add(RocksDbWeight::get().reads(7))
			.saturating_add(RocksDbWeight::get().writes(3))
	}
	fn note_session_performance(n: u32) -> Weight {
		Weight::from_parts(13_550_000, 1_489)
			.saturating_add(Weight::from_parts(11_210_062, 2_527).saturating_mul(n.into()))
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(RocksDbWeight::get().writes(1))
			.saturating_add(RocksDbWeight::get().writes((1_u64).saturating_mul(n.into())))
	}
}
