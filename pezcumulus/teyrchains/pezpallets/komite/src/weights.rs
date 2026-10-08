// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Weights for `pezpallet_komite`.
//!
//! The fallbacks, at the larger of the two Asset Hubs' measurements (weights run 37817572382,
//! reference box, 2026-10-08): the snapshot written once, and each member checked against
//! `Validators` for the count. The runtimes use their own measured files.

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
		Weight::from_parts(95_871_486, 4_579)
			.saturating_add(Weight::from_parts(11_008_655, 2_520).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(1 + m as u64))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}

impl WeightInfo for () {
	fn set_committee(m: u32) -> Weight {
		Weight::from_parts(95_871_486, 4_579)
			.saturating_add(Weight::from_parts(11_008_655, 2_520).saturating_mul(m.into()))
			.saturating_add(RocksDbWeight::get().reads(1 + m as u64))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
}
