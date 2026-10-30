//! Migrations for pezpallet-navnisan.

pub mod v1 {
	use crate::{BalanceOf, Config, Names, OfficeOf, Pezpallet, PriceBands, Prices};
	use alloc::vec::Vec;
	use pezframe_support::{
		traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
		weights::Weight,
	};

	/// What the runtime seeds: the same set its genesis preset writes.
	pub trait SeedSet<T: Config> {
		fn zones() -> Vec<Vec<u8>>;
		fn gov_subnames() -> Vec<(Vec<u8>, Option<OfficeOf<T>>)>;
		fn locked() -> Vec<(Vec<u8>, u8)>;
		fn prices() -> PriceBands<BalanceOf<T>>;
	}

	/// The genesis set, once, on a chain that gained the pallet by upgrade.
	///
	/// Keyed on the register's own state, not on the storage version: Executive stamps a newly
	/// added pallet with its in-code version before any migration runs, so a `0 -> 1`
	/// `VersionedMigration` is skipped on exactly the chain it was written for. Only a register
	/// that has never been seeded -- no prices and no names -- is written, so a chain born with
	/// the genesis preset, or one seeded by hand, is left alone. Remove it from the runtime's
	/// migrations once every live chain carrying the pallet has been measured seeded.
	pub struct SeedNavnisan<T, S>(core::marker::PhantomData<(T, S)>);
	impl<T: Config, S: SeedSet<T>> OnRuntimeUpgrade for SeedNavnisan<T, S> {
		fn on_runtime_upgrade() -> Weight {
			if Prices::<T>::exists() || Names::<T>::iter_keys().next().is_some() {
				return T::DbWeight::get().reads(2);
			}
			let zones = S::zones();
			let subs = S::gov_subnames();
			let locked = S::locked();
			let n = (zones.len() + subs.len() + locked.len()) as u64;
			if let Err(e) = Pezpallet::<T>::seed(&zones, &subs, &locked, S::prices()) {
				pezframe_support::defensive!("navnisan seed failed", e);
			}
			if Pezpallet::<T>::on_chain_storage_version() < 1 {
				StorageVersion::new(1).put::<Pezpallet<T>>();
			}
			// Each entry: a few reads (the parent, the old record) and one or two writes.
			T::DbWeight::get().reads_writes(3 * n + 2, 2 * n + 2)
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_: Vec<u8>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			Pezpallet::<T>::do_try_state()
		}
	}
}
