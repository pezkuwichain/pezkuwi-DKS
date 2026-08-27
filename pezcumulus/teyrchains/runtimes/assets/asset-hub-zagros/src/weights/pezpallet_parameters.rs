//! FOREIGN NUMBERS. Copied from `pezpallet_parameters`' own `BizinikiwiWeight`, measured on
//! upstream's reference hardware rather than ours. Kept because zero is worse: with
//! `WeightInfo = ()` the runtime believes setting a parameter is free, and a free call is a
//! free block. It lives here rather than being imported because the pallet's `weights` module
//! is private -- reaching into it would mean diverging from the fork base for a visibility
//! keyword. Replaced by the same benchmark run as the rest of this directory.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

use core::marker::PhantomData;
use pezframe_support::{traits::Get, weights::Weight};

/// Weight functions for `pezpallet_parameters`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: pezframe_system::Config> pezpallet_parameters::WeightInfo for WeightInfo<T> {
	/// Storage: `Parameters::Parameters` (r:1 w:1)
	/// Proof: `Parameters::Parameters` (`max_values`: None, `max_size`: Some(11322), added:
	/// 13797, mode: `MaxEncodedLen`)
	fn set_parameter() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `14787`
		// Minimum execution time: 5_884_000 picoseconds.
		Weight::from_parts(6_204_000, 14787)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}
