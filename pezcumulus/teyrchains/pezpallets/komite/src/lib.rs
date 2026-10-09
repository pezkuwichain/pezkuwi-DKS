// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! # Komite
//!
//! The committee the People chain seated, as the Asset Hub sees it: each member with the trust
//! score People recorded for them. The Asset Hub pays the committee, and pays by trust × work;
//! trust lives on People, so People sends it here every era. A missing message keeps the last
//! snapshot -- the Asset Hub never stops paying because a message was late.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pezpallet::*;
pub mod weights;
pub use weights::WeightInfo;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// What else must follow when a committee is noted.
pub trait OnCommittee<AccountId> {
	fn on_committee(members: &[(AccountId, u128)]);
}

impl<AccountId> OnCommittee<AccountId> for () {
	fn on_committee(_: &[(AccountId, u128)]) {}
}

/// An origin `CommitteeOrigin` accepts, for benchmarks.
///
/// The runtime's origin is the People chain over XCM, and `EnsureXcm`'s own
/// `try_successful_origin()` offers `Here`, which `Equals<PeopleLocation>` rightly refuses. Only
/// the runtime knows People's location, so the runtime says it.
#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkSetup<RuntimeOrigin> {
	fn committee_origin() -> RuntimeOrigin;
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;
	use pezsp_runtime::Perbill;

	#[pezpallet::config]
	pub trait Config: pezframe_system::Config<RuntimeEvent: From<Event<Self>>> {
		/// Who may note a committee: the People chain over XCM.
		type CommitteeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// The largest committee TNPoS may seat.
		#[pezpallet::constant]
		type MaxMembers: Get<u32>;
		/// Told of every committee noted, after it is stored.
		type OnCommittee: super::OnCommittee<Self::AccountId>;
		type WeightInfo: WeightInfo;
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: super::BenchmarkSetup<Self::RuntimeOrigin>;
	}

	/// Bumped with every change to `Snapshot`'s encoding, so a migration has a version to key on.
	/// It starts at 1, not 0: 0 is a chain the pallet has not arrived on yet, and the Asset Hub
	/// keys a one-off migration on that step (`StartTheEraClockWithKomite`).
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pezpallet::pezpallet]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	/// One era's committee as People sent it.
	#[derive(
		Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct Snapshot<T: Config> {
		/// The TNPoS era People seated this committee for.
		pub era: u32,
		/// Each member with its trust score as People recorded it.
		pub members: BoundedVec<(T::AccountId, u128), T::MaxMembers>,
		/// The highest trust in `members`, kept so a lookup does not scan twice.
		pub max_trust: u128,
	}

	/// The latest committee People sent, or nothing before the first message.
	impl<T: Config> Snapshot<T> {
		/// `who`'s trust in thousandths of this snapshot's highest, or `None` if not a member.
		/// For callers that read the snapshot once and weigh many accounts by it.
		pub fn trust_permille(&self, who: &T::AccountId) -> Option<u32> {
			let (_, trust) = self.members.iter().find(|(m, _)| m == who)?;
			if self.max_trust == 0 {
				return Some(0);
			}
			Some(Perbill::from_rational(*trust, self.max_trust).mul_floor(1000u32))
		}
	}

	#[pezpallet::storage]
	pub type Committee<T: Config> = StorageValue<_, Snapshot<T>, OptionQuery>;

	/// The committee as it stood when the election for the next era was planned: the one its
	/// targets were drawn from.
	#[pezpallet::storage]
	pub type PlannedCommittee<T: Config> = StorageValue<_, Snapshot<T>, OptionQuery>;

	/// The committee the active era was elected under. Work in the era is weighed by its trust,
	/// so a member removed mid-era keeps its own weight to the era's end.
	#[pezpallet::storage]
	pub type ActiveCommittee<T: Config> = StorageValue<_, Snapshot<T>, OptionQuery>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// People's committee for `era` replaced the one held.
		CommitteeNoted { era: u32, members: u32 },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// A snapshot for an older era than the one held: a late message must not win.
		StaleSnapshot,
	}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Replace the held committee with People's committee for `era`.
		///
		/// The same era may be noted again (a resent message); an older one may not.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(T::WeightInfo::set_committee(members.len() as u32))]
		pub fn set_committee(
			origin: OriginFor<T>,
			era: u32,
			members: BoundedVec<(T::AccountId, u128), T::MaxMembers>,
		) -> DispatchResult {
			T::CommitteeOrigin::ensure_origin(origin)?;
			if let Some(held) = Committee::<T>::get() {
				ensure!(era >= held.era, Error::<T>::StaleSnapshot);
			}
			let max_trust = members.iter().map(|(_, t)| *t).max().unwrap_or(0);
			let count = members.len() as u32;
			T::OnCommittee::on_committee(&members);
			Committee::<T>::put(Snapshot { era, members, max_trust });
			Self::deposit_event(Event::CommitteeNoted { era, members: count });
			Ok(())
		}
	}

	impl<T: Config> Pezpallet<T> {
		/// `who`'s trust in thousandths of the committee's highest: the highest is 1000, a trust
		/// of zero is 0. `None` if `who` is not in the held committee, or nothing is held.
		pub fn trust_permille(who: &T::AccountId) -> Option<u32> {
			Committee::<T>::get()?.trust_permille(who)
		}

		pub fn is_member(who: &T::AccountId) -> bool {
			Committee::<T>::get().map_or(false, |s| s.members.iter().any(|(m, _)| m == who))
		}

		pub fn member_count() -> u32 {
			Committee::<T>::get().map_or(0, |s| s.members.len() as u32)
		}

		pub fn has_snapshot() -> bool {
			Committee::<T>::exists()
		}

		/// An era's election was planned: freeze the committee its targets come from.
		pub fn note_era_planned() {
			match Committee::<T>::get() {
				Some(s) => PlannedCommittee::<T>::put(s),
				None => PlannedCommittee::<T>::kill(),
			}
		}

		/// The planned era started: the committee it was elected under is now the active one.
		pub fn note_era_activated() {
			match PlannedCommittee::<T>::take() {
				Some(s) => ActiveCommittee::<T>::put(s),
				None => ActiveCommittee::<T>::kill(),
			}
		}

		/// The committee to weigh the active era's work by: the one it was elected under, or --
		/// before any era has been elected under one -- the latest.
		pub fn weighing_snapshot() -> Option<Snapshot<T>> {
			ActiveCommittee::<T>::get().or_else(Committee::<T>::get)
		}
	}
}
