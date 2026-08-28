// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Weights for pezpallet-tnpos.
//!
//! The zero implementation exists so tests can run before the benchmark pass; it is not a
//! weight anyone may ship. Task 13 replaces it with measured values from CI.

use pezframe_support::weights::Weight;

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

impl WeightInfo for () {
	fn join() -> Weight {
		Weight::zero()
	}
	fn leave() -> Weight {
		Weight::zero()
	}
	fn set_strata() -> Weight {
		Weight::zero()
	}
	fn report_offence() -> Weight {
		Weight::zero()
	}
	fn commit_seed() -> Weight {
		Weight::zero()
	}
	fn reveal_seed() -> Weight {
		Weight::zero()
	}
	fn seat_committee(_p: u32) -> Weight {
		Weight::zero()
	}
}
