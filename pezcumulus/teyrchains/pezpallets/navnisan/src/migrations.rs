//! Migrations for pezpallet-navnisan.

pub mod v1 {
	use crate::{BalanceOf, Config, Names, OfficeOf, Pezpallet, PriceBands};
	use alloc::vec::Vec;
	use pezframe_support::{
		migrations::VersionedMigration,
		traits::{Get, UncheckedOnRuntimeUpgrade},
		weights::Weight,
	};

	/// What the runtime seeds: the same set its genesis preset writes.
	pub trait SeedSet<T: Config> {
		fn zones() -> Vec<Vec<u8>>;
		fn gov_subnames() -> Vec<(Vec<u8>, Option<OfficeOf<T>>)>;
		fn locked() -> Vec<(Vec<u8>, u8)>;
		fn prices() -> PriceBands<BalanceOf<T>>;
	}

	pub struct Seed<T, S>(core::marker::PhantomData<(T, S)>);
	impl<T: Config, S: SeedSet<T>> UncheckedOnRuntimeUpgrade for Seed<T, S> {
		fn on_runtime_upgrade() -> Weight {
			// Only an empty register is seeded: a chain whose genesis already wrote it is at
			// version 1 and never gets here, and nothing written by hand is overwritten.
			if Names::<T>::iter_keys().next().is_some() {
				return T::DbWeight::get().reads(1);
			}
			let zones = S::zones();
			let subs = S::gov_subnames();
			let locked = S::locked();
			let n = (zones.len() + subs.len() + locked.len()) as u64;
			if let Err(e) = Pezpallet::<T>::seed(&zones, &subs, &locked, S::prices()) {
				pezframe_support::defensive!("navnisan seed failed", e);
			}
			// Each entry: a few reads (the parent, the old record) and one or two writes.
			T::DbWeight::get().reads_writes(3 * n + 2, 2 * n + 2)
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_: Vec<u8>) -> Result<(), pezsp_runtime::TryRuntimeError> {
			Pezpallet::<T>::do_try_state()
		}
	}

	/// The genesis set, once, on a chain that gained the pallet by upgrade.
	pub type SeedNavnisan<T, S> = VersionedMigration<
		0,
		1,
		Seed<T, S>,
		Pezpallet<T>,
		<T as pezframe_system::Config>::DbWeight,
	>;
}
