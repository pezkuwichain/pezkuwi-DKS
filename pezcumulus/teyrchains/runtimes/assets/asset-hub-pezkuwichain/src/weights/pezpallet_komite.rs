// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Weights for `pezpallet_komite`.
//!
//! Placeholders sized above the work -- one read, one write, a vector of `m` members -- until
//! the pallet is benchmarked on the reference machine and this file is replaced by the result.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use pezframe_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pezpallet_komite`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: pezframe_system::Config> pezpallet_komite::WeightInfo for WeightInfo<T> {
	/// Storage: `Komite::Committee` (r:1 w:1)
	/// The range of component `m` is `[1, 64]`.
	fn set_committee(m: u32, ) -> Weight {
		Weight::from_parts(10_000_000, 4_000)
			.saturating_add(Weight::from_parts(50_000, 48).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads_writes(1, 1))
	}
}
