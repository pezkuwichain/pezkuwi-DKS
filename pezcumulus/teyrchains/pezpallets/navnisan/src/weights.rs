//! Weights for `pezpallet_navnisan`.
//!
//! PROVISIONAL: written by hand, upper bounds, until the weights run replaces this file with
//! measured values (`.github/workflows` weights job). Every call is bounded by `d` (ancestors
//! read, at most `MAX_DEPTH`) and `t` (text records cleared, at most `MAX_TEXTS`).

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]
#![allow(dead_code)]

use pezframe_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pezpallet_navnisan`.
pub trait WeightInfo {
	fn commit() -> Weight;
	fn register(t: u32) -> Weight;
	fn renew() -> Weight;
	fn transfer(d: u32) -> Weight;
	fn set_account(d: u32) -> Weight;
	fn set_content(d: u32) -> Weight;
	fn anchor(d: u32) -> Weight;
	fn set_text(d: u32) -> Weight;
	fn set_subname(d: u32, t: u32) -> Weight;
	fn remove_subname(t: u32) -> Weight;
	fn set_primary(d: u32) -> Weight;
	fn clear_primary() -> Weight;
	fn grant(d: u32) -> Weight;
	fn revoke(t: u32) -> Weight;
	fn force_change() -> Weight;
	fn cancel_change() -> Weight;
	fn enact_change() -> Weight;
	fn schedule_revocation() -> Weight;
	fn cancel_revocation() -> Weight;
	fn enact_revocation(t: u32) -> Weight;
	fn lock() -> Weight;
	fn unlock() -> Weight;
	fn set_prices() -> Weight;
	fn reap(d: u32, t: u32) -> Weight;
	fn reap_primary(d: u32) -> Weight;
	fn remove_commitment() -> Weight;
}

pub struct BizinikiwiWeight<T>(PhantomData<T>);
impl<T: pezframe_system::Config> WeightInfo for BizinikiwiWeight<T> {
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

impl WeightInfo for () {
	fn commit() -> Weight {
		Weight::from_parts(40_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn register(t: u32) -> Weight {
		Weight::from_parts(120_000_000, 8_000)
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().reads(9_u64))
			.saturating_add(RocksDbWeight::get().writes(7_u64))
	}
	fn renew() -> Weight {
		Weight::from_parts(70_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	fn transfer(d: u32) -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn set_account(d: u32) -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn set_content(d: u32) -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn anchor(d: u32) -> Weight {
		Weight::from_parts(50_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn set_text(d: u32) -> Weight {
		Weight::from_parts(80_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
	fn set_subname(d: u32, t: u32) -> Weight {
		Weight::from_parts(120_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().reads(6_u64))
			.saturating_add(RocksDbWeight::get().writes(5_u64))
	}
	fn remove_subname(t: u32) -> Weight {
		Weight::from_parts(80_000_000, 8_000)
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	fn set_primary(d: u32) -> Weight {
		Weight::from_parts(50_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn clear_primary() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn grant(d: u32) -> Weight {
		Weight::from_parts(90_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	fn revoke(t: u32) -> Weight {
		Weight::from_parts(80_000_000, 8_000)
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	fn force_change() -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn cancel_change() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn enact_change() -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	fn schedule_revocation() -> Weight {
		Weight::from_parts(50_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn cancel_revocation() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn enact_revocation(t: u32) -> Weight {
		Weight::from_parts(80_000_000, 8_000)
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	fn lock() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(0_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn unlock() -> Weight {
		Weight::from_parts(30_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn set_prices() -> Weight {
		Weight::from_parts(25_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(0_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn reap(d: u32, t: u32) -> Weight {
		Weight::from_parts(100_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(8_000_000, 3_000).saturating_mul(t.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().writes(1_u64.saturating_mul(t.into())))
			.saturating_add(RocksDbWeight::get().reads(5_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	fn reap_primary(d: u32) -> Weight {
		Weight::from_parts(60_000_000, 8_000)
			.saturating_add(Weight::from_parts(6_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64.saturating_mul(d.into())))
			.saturating_add(RocksDbWeight::get().reads(5_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn remove_commitment() -> Weight {
		Weight::from_parts(50_000_000, 8_000)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
}
