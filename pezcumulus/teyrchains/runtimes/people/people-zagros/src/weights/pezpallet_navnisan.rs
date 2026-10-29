// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Weights for `pezpallet_navnisan`.
//!
//! PROVISIONAL upper bounds, copied from the pallet, until the weights run measures this
//! runtime and replaces the file.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use pezframe_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pezpallet_navnisan`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: pezframe_system::Config> pezpallet_navnisan::WeightInfo for WeightInfo<T> {
	fn commit() -> Weight {
		Weight::from_parts(40_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn register(t: u32) -> Weight {
		Weight::from_parts(120_000_000, 8_000)
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().reads(9_u64))
			.saturating_add(T::DbWeight::get().writes(7_u64))
	}
	fn renew() -> Weight {
		Weight::from_parts(70_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	fn transfer(d: u32) -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn set_account(d: u32) -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn set_content(d: u32) -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn anchor(d: u32) -> Weight {
		Weight::from_parts(50_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn set_text(d: u32) -> Weight {
		Weight::from_parts(80_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	fn set_subname(d: u32, t: u32) -> Weight {
		Weight::from_parts(120_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().reads(6_u64))
			.saturating_add(T::DbWeight::get().writes(5_u64))
	}
	fn remove_subname(t: u32) -> Weight {
		Weight::from_parts(80_000_000, 8_000)
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	fn set_primary(d: u32) -> Weight {
		Weight::from_parts(50_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn clear_primary() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn grant(d: u32) -> Weight {
		Weight::from_parts(90_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	fn revoke(t: u32) -> Weight {
		Weight::from_parts(80_000_000, 8_000)
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	fn force_change() -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn cancel_change() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn enact_change() -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	fn schedule_revocation() -> Weight {
		Weight::from_parts(50_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn cancel_revocation() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn enact_revocation(t: u32) -> Weight {
		Weight::from_parts(80_000_000, 8_000)
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	fn lock() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(0_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn unlock() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn set_prices() -> Weight {
		Weight::from_parts(25_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(0_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn reap(d: u32, t: u32) -> Weight {
		Weight::from_parts(100_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	fn reap_primary(d: u32) -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn remove_commitment() -> Weight {
		Weight::from_parts(50_000_000, 8_000)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}
