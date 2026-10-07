// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Weights for `pezpallet_komite`.
//!
//! Placeholders sized above the work -- one read, one write, a vector of `m` members -- until
//! the benchmark below is run on the reference machine and replaces them.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use pezframe_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

pub trait WeightInfo {
	fn set_committee(m: u32) -> Weight;
}

/// Weights for `pezpallet_komite` using the runtime's database weights.
pub struct BizinikiwiWeight<T>(PhantomData<T>);
impl<T: pezframe_system::Config> WeightInfo for BizinikiwiWeight<T> {
	/// Storage: `Komite::Committee` (r:1 w:1)
	fn set_committee(m: u32) -> Weight {
		Weight::from_parts(10_000_000, 4_000)
			.saturating_add(Weight::from_parts(50_000, 48).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads_writes(1, 1))
	}
}

impl WeightInfo for () {
	fn set_committee(m: u32) -> Weight {
		Weight::from_parts(10_000_000, 4_000)
			.saturating_add(Weight::from_parts(50_000, 48).saturating_mul(m.into()))
			.saturating_add(RocksDbWeight::get().reads_writes(1, 1))
	}
}
