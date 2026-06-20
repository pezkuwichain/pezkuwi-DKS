// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! Pezpallet to handle teyrchain registration and related fund management.
//! In essence this is a simple wrapper around `paras`.

pub mod migration;

use alloc::{vec, vec::Vec};
use core::result;
use pezframe_support::{
	dispatch::DispatchResult,
	ensure,
	pezpallet_prelude::Weight,
	traits::{Currency, Get, ReservableCurrency},
};
use pezframe_system::{self, ensure_root, ensure_signed};
use pezkuwi_primitives::{HeadData, Id as ParaId, ValidationCode, LOWEST_PUBLIC_ID, MIN_CODE_SIZE};
use pezkuwi_runtime_teyrchains::{
	configuration, ensure_teyrchain,
	paras::{self, ParaGenesisArgs, UpgradeStrategy},
	Origin, ParaLifecycle,
};

use crate::traits::{OnSwap, Registrar};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pezkuwi_runtime_teyrchains::paras::{OnNewHead, ParaKind};
pub use pezpallet::*;
use pezsp_runtime::{
	traits::{CheckedSub, Saturating},
	RuntimeDebug,
};
use scale_info::TypeInfo;

#[derive(
	Encode,
	Decode,
	Clone,
	PartialEq,
	Eq,
	Default,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	DecodeWithMemTracking,
)]
pub struct ParaInfo<Account, Balance> {
	/// The account that has placed a deposit for registering this para.
	pub manager: Account,
	/// The amount reserved by the `manager` account for the registration.
	pub deposit: Balance,
	/// Whether the para registration should be locked from being controlled by the manager.
	/// None means the lock had not been explicitly set, and should be treated as false.
	pub locked: Option<bool>,
}

impl<Account, Balance> ParaInfo<Account, Balance> {
	/// Returns if the para is locked.
	pub fn is_locked(&self) -> bool {
		self.locked.unwrap_or(false)
	}
}

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as pezframe_system::Config>::AccountId>>::Balance;

pub trait WeightInfo {
	fn reserve() -> Weight;
	fn register() -> Weight;
	fn force_register() -> Weight;
	fn deregister() -> Weight;
	fn swap() -> Weight;
	fn schedule_code_upgrade(b: u32) -> Weight;
	fn set_current_head(b: u32) -> Weight;
}

pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
	fn reserve() -> Weight {
		Weight::zero()
	}
	fn register() -> Weight {
		Weight::zero()
	}
	fn force_register() -> Weight {
		Weight::zero()
	}
	fn deregister() -> Weight {
		Weight::zero()
	}
	fn swap() -> Weight {
		Weight::zero()
	}
	fn schedule_code_upgrade(_b: u32) -> Weight {
		Weight::zero()
	}
	fn set_current_head(_b: u32) -> Weight {
		Weight::zero()
	}
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	/// The in-code storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pezpallet::pezpallet]
	#[pezpallet::without_storage_info]
	#[pezpallet::storage_version(STORAGE_VERSION)]
	pub struct Pezpallet<T>(_);

	#[pezpallet::config]
	#[pezpallet::disable_pezframe_system_supertrait_check]
	pub trait Config: configuration::Config + paras::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;

		/// The aggregated origin type must support the `teyrchains` origin. We require that we can
		/// infallibly convert between this origin and the system origin, but in reality, they're
		/// the same type, we just can't express that to the Rust type system without writing a
		/// `where` clause everywhere.
		type RuntimeOrigin: From<<Self as pezframe_system::Config>::RuntimeOrigin>
			+ Into<result::Result<Origin, <Self as Config>::RuntimeOrigin>>;

		/// The system's currency for on-demand teyrchain payment.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Runtime hook for when a lease holding teyrchain and on-demand teyrchain swap.
		type OnSwap: crate::traits::OnSwap;

		/// The deposit to be paid to run a on-demand teyrchain.
		/// This should include the cost for storing the genesis head and validation code.
		#[pezpallet::constant]
		type ParaDeposit: Get<BalanceOf<Self>>;

		/// The deposit to be paid per byte stored on chain.
		#[pezpallet::constant]
		type DataDepositPerByte: Get<BalanceOf<Self>>;

		/// Weight Information for the Extrinsics in the Pezpallet
		type WeightInfo: WeightInfo;
	}

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Registered { para_id: ParaId, manager: T::AccountId },
		Deregistered { para_id: ParaId },
		Reserved { para_id: ParaId, who: T::AccountId },
		Swapped { para_id: ParaId, other_id: ParaId },
	}

	#[pezpallet::error]
	pub enum Error<T> {
		/// The ID is not registered.
		NotRegistered,
		/// The ID is already registered.
		AlreadyRegistered,
		/// The caller is not the owner of this Id.
		NotOwner,
		/// Invalid para code size.
		CodeTooLarge,
		/// Invalid para head data size.
		HeadDataTooLarge,
		/// Para is not a Teyrchain.
		NotTeyrchain,
		/// Para is not a Parathread (on-demand teyrchain).
		NotParathread,
		/// Cannot deregister para
		CannotDeregister,
		/// Cannot schedule downgrade of lease holding teyrchain to on-demand teyrchain
		CannotDowngrade,
		/// Cannot schedule upgrade of on-demand teyrchain to lease holding teyrchain
		CannotUpgrade,
		/// Para is locked from manipulation by the manager. Must use teyrchain or relay chain
		/// governance.
		ParaLocked,
		/// The ID given for registration has not been reserved.
		NotReserved,
		/// The validation code is invalid.
		InvalidCode,
		/// Cannot perform a teyrchain slot / lifecycle swap. Check that the state of both paras
		/// are correct for the swap to work.
		CannotSwap,
	}

	/// Pending swap operations.
	#[pezpallet::storage]
	pub(super) type PendingSwap<T> = StorageMap<_, Twox64Concat, ParaId, ParaId>;

	/// Amount held on deposit for each para and the original depositor.
	///
	/// The given account ID is responsible for registering the code and initial head data, but may
	/// only do so if it isn't yet registered. (After that, it's up to governance to do so.)
	#[pezpallet::storage]
	pub type Paras<T: Config> =
		StorageMap<_, Twox64Concat, ParaId, ParaInfo<T::AccountId, BalanceOf<T>>>;

	/// The next free `ParaId`.
	#[pezpallet::storage]
	pub type NextFreeParaId<T> = StorageValue<_, ParaId, ValueQuery>;

	#[pezpallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[serde(skip)]
		pub _config: core::marker::PhantomData<T>,
		pub next_free_para_id: ParaId,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { next_free_para_id: LOWEST_PUBLIC_ID, _config: Default::default() }
		}
	}

	#[pezpallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			NextFreeParaId::<T>::put(self.next_free_para_id);
		}
	}

	#[pezpallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pezpallet<T> {}

	#[pezpallet::call]
	impl<T: Config> Pezpallet<T> {
		/// Register head data and validation code for a reserved Para Id.
		///
		/// ## Arguments
		/// - `origin`: Must be called by a `Signed` origin.
		/// - `id`: The para ID. Must be owned/managed by the `origin` signing account.
		/// - `genesis_head`: The genesis head data of the teyrchain/thread.
		/// - `validation_code`: The initial validation code of the teyrchain/thread.
		///
		/// ## Deposits/Fees
		/// The account with the originating signature must reserve a deposit.
		///
		/// The deposit is required to cover the costs associated with storing the genesis head
		/// data and the validation code.
		/// This accounts for the potential to store validation code of a size up to the
		/// `max_code_size`, as defined in the configuration pezpallet
		///
		/// Anything already reserved previously for this para ID is accounted for.
		///
		/// ## Events
		/// The `Registered` event is emitted in case of success.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(<T as Config>::WeightInfo::register())]
		pub fn register(
			origin: OriginFor<T>,
			id: ParaId,
			genesis_head: HeadData,
			validation_code: ValidationCode,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_register(who, None, id, genesis_head, validation_code, true)?;
			Ok(())
		}

		/// Force the registration of a Para Id on the relay chain.
		///
		/// This function must be called by a Root origin.
		///
		/// The deposit taken can be specified for this registration. Any `ParaId`
		/// can be registered, including sub-1000 IDs which are System Teyrchains.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight(<T as Config>::WeightInfo::force_register())]
		pub fn force_register(
			origin: OriginFor<T>,
			who: T::AccountId,
			deposit: BalanceOf<T>,
			id: ParaId,
			genesis_head: HeadData,
			validation_code: ValidationCode,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::do_register(who, Some(deposit), id, genesis_head, validation_code, false)
		}

		/// Deregister a Para Id, freeing all data and returning any deposit.
		///
		/// The caller must be Root, the `para` owner, or the `para` itself. The para must be an
		/// on-demand teyrchain.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight(<T as Config>::WeightInfo::deregister())]
		pub fn deregister(origin: OriginFor<T>, id: ParaId) -> DispatchResult {
			Self::ensure_root_para_or_owner(origin, id)?;
			Self::do_deregister(id)
		}

		/// Swap a lease holding teyrchain with another teyrchain, either on-demand or lease
		/// holding.
		///
		/// The origin must be Root, the `para` owner, or the `para` itself.
		///
		/// The swap will happen only if there is already an opposite swap pending. If there is not,
		/// the swap will be stored in the pending swaps map, ready for a later confirmatory swap.
		///
		/// The `ParaId`s remain mapped to the same head data and code so external code can rely on
		/// `ParaId` to be a long-term identifier of a notional "teyrchain". However, their
		/// scheduling info (i.e. whether they're an on-demand teyrchain or lease holding
		/// teyrchain), auction information and the auction deposit are switched.
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(<T as Config>::WeightInfo::swap())]
		pub fn swap(origin: OriginFor<T>, id: ParaId, other: ParaId) -> DispatchResult {
			Self::ensure_root_para_or_owner(origin, id)?;

			// If `id` and `other` is the same id, we treat this as a "clear" function, and exit
			// early, since swapping the same id would otherwise be a noop.
			if id == other {
				PendingSwap::<T>::remove(id);
				return Ok(());
			}

			// Sanity check that `id` is even a para.
			let id_lifecycle =
				paras::Pezpallet::<T>::lifecycle(id).ok_or(Error::<T>::NotRegistered)?;

			if PendingSwap::<T>::get(other) == Some(id) {
				let other_lifecycle =
					paras::Pezpallet::<T>::lifecycle(other).ok_or(Error::<T>::NotRegistered)?;
				// identify which is a lease holding teyrchain and which is a parathread (on-demand
				// teyrchain)
				if id_lifecycle == ParaLifecycle::Teyrchain
					&& other_lifecycle == ParaLifecycle::Parathread
				{
					Self::do_thread_and_chain_swap(id, other);
				} else if id_lifecycle == ParaLifecycle::Parathread
					&& other_lifecycle == ParaLifecycle::Teyrchain
				{
					Self::do_thread_and_chain_swap(other, id);
				} else if id_lifecycle == ParaLifecycle::Teyrchain
					&& other_lifecycle == ParaLifecycle::Teyrchain
				{
					// If both chains are currently teyrchains, there is nothing funny we
					// need to do for their lifecycle management, just swap the underlying
					// data.
					T::OnSwap::on_swap(id, other);
				} else {
					return Err(Error::<T>::CannotSwap.into());
				}
				Self::deposit_event(Event::<T>::Swapped { para_id: id, other_id: other });
				PendingSwap::<T>::remove(other);
			} else {
				PendingSwap::<T>::insert(id, other);
			}

			Ok(())
		}

		/// Remove a manager lock from a para. This will allow the manager of a
		/// previously locked para to deregister or swap a para without using governance.
		///
		/// Can only be called by the Root origin or the teyrchain.
		#[pezpallet::call_index(4)]
		#[pezpallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn remove_lock(origin: OriginFor<T>, para: ParaId) -> DispatchResult {
			Self::ensure_root_or_para(origin, para)?;
			<Self as Registrar>::remove_lock(para);
			Ok(())
		}

		/// Reserve a Para Id on the relay chain.
		///
		/// This function will reserve a new Para Id to be owned/managed by the origin account.
		/// The origin account is able to register head data and validation code using `register` to
		/// create an on-demand teyrchain. Using the Slots pezpallet, an on-demand teyrchain can
		/// then be upgraded to a lease holding teyrchain.
		///
		/// ## Arguments
		/// - `origin`: Must be called by a `Signed` origin. Becomes the manager/owner of the new
		///   para ID.
		///
		/// ## Deposits/Fees
		/// The origin must reserve a deposit of `ParaDeposit` for the registration.
		///
		/// ## Events
		/// The `Reserved` event is emitted in case of success, which provides the ID reserved for
		/// use.
		#[pezpallet::call_index(5)]
		#[pezpallet::weight(<T as Config>::WeightInfo::reserve())]
		pub fn reserve(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let id = NextFreeParaId::<T>::get().max(LOWEST_PUBLIC_ID);
			Self::do_reserve(who, None, id)?;
			NextFreeParaId::<T>::set(id + 1);
			Ok(())
		}

		/// Add a manager lock from a para. This will prevent the manager of a
		/// para to deregister or swap a para.
		///
		/// Can be called by Root, the teyrchain, or the teyrchain manager if the teyrchain is
		/// unlocked.
		#[pezpallet::call_index(6)]
		#[pezpallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn add_lock(origin: OriginFor<T>, para: ParaId) -> DispatchResult {
			Self::ensure_root_para_or_owner(origin, para)?;
			<Self as Registrar>::apply_lock(para);
			Ok(())
		}

		/// Schedule a teyrchain upgrade.
		///
		/// This will kick off a check of `new_code` by all validators. After the majority of the
		/// validators have reported on the validity of the code, the code will either be enacted
		/// or the upgrade will be rejected. If the code will be enacted, the current code of the
		/// teyrchain will be overwritten directly. This means that any PoV will be checked by this
		/// new code. The teyrchain itself will not be informed explicitly that the validation code
		/// has changed.
		///
		/// Can be called by Root, the teyrchain, or the teyrchain manager if the teyrchain is
		/// unlocked.
		#[pezpallet::call_index(7)]
		#[pezpallet::weight(<T as Config>::WeightInfo::schedule_code_upgrade(new_code.0.len() as u32))]
		pub fn schedule_code_upgrade(
			origin: OriginFor<T>,
			para: ParaId,
			new_code: ValidationCode,
		) -> DispatchResult {
			Self::ensure_root_para_or_owner(origin, para)?;
			pezkuwi_runtime_teyrchains::schedule_code_upgrade::<T>(
				para,
				new_code,
				UpgradeStrategy::ApplyAtExpectedBlock,
			)?;
			Ok(())
		}

		/// Set the teyrchain's current head.
		///
		/// Can be called by Root, the teyrchain, or the teyrchain manager if the teyrchain is
		/// unlocked.
		#[pezpallet::call_index(8)]
		#[pezpallet::weight(<T as Config>::WeightInfo::set_current_head(new_head.0.len() as u32))]
		pub fn set_current_head(
			origin: OriginFor<T>,
			para: ParaId,
			new_head: HeadData,
		) -> DispatchResult {
			Self::ensure_root_para_or_owner(origin, para)?;
			pezkuwi_runtime_teyrchains::set_current_head::<T>(para, new_head);
			Ok(())
		}
	}
}

impl<T: Config> Registrar for Pezpallet<T> {
	type AccountId = T::AccountId;

	/// Return the manager `AccountId` of a para if one exists.
	fn manager_of(id: ParaId) -> Option<T::AccountId> {
		Some(Paras::<T>::get(id)?.manager)
	}

	// All lease holding teyrchains. Ordered ascending by ParaId. On-demand teyrchains are not
	// included.
	fn teyrchains() -> Vec<ParaId> {
		paras::Teyrchains::<T>::get()
	}

	// Return if a para is a parathread (on-demand teyrchain)
	fn is_parathread(id: ParaId) -> bool {
		paras::Pezpallet::<T>::is_parathread(id)
	}

	// Return if a para is a lease holding teyrchain
	fn is_teyrchain(id: ParaId) -> bool {
		paras::Pezpallet::<T>::is_teyrchain(id)
	}

	// Apply a lock to the teyrchain.
	fn apply_lock(id: ParaId) {
		Paras::<T>::mutate(id, |x| x.as_mut().map(|info| info.locked = Some(true)));
	}

	// Remove a lock from the teyrchain.
	fn remove_lock(id: ParaId) {
		Paras::<T>::mutate(id, |x| x.as_mut().map(|info| info.locked = Some(false)));
	}

	// Register a Para ID under control of `manager`.
	//
	// Note this is a backend registration API, so verification of ParaId
	// is not done here to prevent.
	fn register(
		manager: T::AccountId,
		id: ParaId,
		genesis_head: HeadData,
		validation_code: ValidationCode,
	) -> DispatchResult {
		Self::do_register(manager, None, id, genesis_head, validation_code, false)
	}

	// Deregister a Para ID, free any data, and return any deposits.
	fn deregister(id: ParaId) -> DispatchResult {
		Self::do_deregister(id)
	}

	// Upgrade a registered on-demand teyrchain into a lease holding teyrchain.
	fn make_teyrchain(id: ParaId) -> DispatchResult {
		// Para backend should think this is an on-demand teyrchain...
		ensure!(
			paras::Pezpallet::<T>::lifecycle(id) == Some(ParaLifecycle::Parathread),
			Error::<T>::NotParathread
		);
		pezkuwi_runtime_teyrchains::schedule_parathread_upgrade::<T>(id)
			.map_err(|_| Error::<T>::CannotUpgrade)?;

		Ok(())
	}

	// Downgrade a registered para into a parathread (on-demand teyrchain).
	fn make_parathread(id: ParaId) -> DispatchResult {
		// Para backend should think this is a teyrchain...
		ensure!(
			paras::Pezpallet::<T>::lifecycle(id) == Some(ParaLifecycle::Teyrchain),
			Error::<T>::NotTeyrchain
		);
		pezkuwi_runtime_teyrchains::schedule_teyrchain_downgrade::<T>(id)
			.map_err(|_| Error::<T>::CannotDowngrade)?;
		Ok(())
	}

	#[cfg(any(feature = "runtime-benchmarks", test))]
	fn worst_head_data() -> HeadData {
		let max_head_size = configuration::ActiveConfig::<T>::get().max_head_data_size;
		assert!(max_head_size > 0, "max_head_data can't be zero for generating worst head data.");
		vec![0u8; max_head_size as usize].into()
	}

	#[cfg(any(feature = "runtime-benchmarks", test))]
	fn worst_validation_code() -> ValidationCode {
		let max_code_size = configuration::ActiveConfig::<T>::get().max_code_size;
		assert!(max_code_size > 0, "max_code_size can't be zero for generating worst code data.");
		let validation_code = vec![0u8; max_code_size as usize];
		validation_code.into()
	}

	#[cfg(any(feature = "runtime-benchmarks", test))]
	fn execute_pending_transitions() {
		use pezkuwi_runtime_teyrchains::shared;
		shared::Pezpallet::<T>::set_session_index(shared::Pezpallet::<T>::scheduled_session());
		paras::Pezpallet::<T>::test_on_new_session();
	}
}

impl<T: Config> Pezpallet<T> {
	/// Ensure the origin is one of Root, the `para` owner, or the `para` itself.
	/// If the origin is the `para` owner, the `para` must be unlocked.
	fn ensure_root_para_or_owner(
		origin: <T as pezframe_system::Config>::RuntimeOrigin,
		id: ParaId,
	) -> DispatchResult {
		if let Ok(who) = ensure_signed(origin.clone()) {
			let para_info = Paras::<T>::get(id).ok_or(Error::<T>::NotRegistered)?;

			if para_info.manager == who {
				ensure!(!para_info.is_locked(), Error::<T>::ParaLocked);
				return Ok(());
			}
		}

		Self::ensure_root_or_para(origin, id)
	}

	/// Ensure the origin is one of Root or the `para` itself.
	fn ensure_root_or_para(
		origin: <T as pezframe_system::Config>::RuntimeOrigin,
		id: ParaId,
	) -> DispatchResult {
		if ensure_root(origin.clone()).is_ok() {
			return Ok(());
		}

		let caller_id = ensure_teyrchain(<T as Config>::RuntimeOrigin::from(origin))?;
		// Check if matching para id...
		ensure!(caller_id == id, Error::<T>::NotOwner);

		Ok(())
	}

	fn do_reserve(
		who: T::AccountId,
		deposit_override: Option<BalanceOf<T>>,
		id: ParaId,
	) -> DispatchResult {
		ensure!(!Paras::<T>::contains_key(id), Error::<T>::AlreadyRegistered);
		ensure!(paras::Pezpallet::<T>::lifecycle(id).is_none(), Error::<T>::AlreadyRegistered);

		let deposit = deposit_override.unwrap_or_else(T::ParaDeposit::get);
		<T as Config>::Currency::reserve(&who, deposit)?;
		let info = ParaInfo { manager: who.clone(), deposit, locked: None };

		Paras::<T>::insert(id, info);
		Self::deposit_event(Event::<T>::Reserved { para_id: id, who });
		Ok(())
	}

	/// Attempt to register a new Para Id under management of `who` in the
	/// system with the given information.
	fn do_register(
		who: T::AccountId,
		deposit_override: Option<BalanceOf<T>>,
		id: ParaId,
		genesis_head: HeadData,
		validation_code: ValidationCode,
		ensure_reserved: bool,
	) -> DispatchResult {
		let deposited = if let Some(para_data) = Paras::<T>::get(id) {
			ensure!(para_data.manager == who, Error::<T>::NotOwner);
			ensure!(!para_data.is_locked(), Error::<T>::ParaLocked);
			para_data.deposit
		} else {
			ensure!(!ensure_reserved, Error::<T>::NotReserved);
			Default::default()
		};
		ensure!(paras::Pezpallet::<T>::lifecycle(id).is_none(), Error::<T>::AlreadyRegistered);
		let (genesis, deposit) =
			Self::validate_onboarding_data(genesis_head, validation_code, ParaKind::Parathread)?;
		let deposit = deposit_override.unwrap_or(deposit);

		if let Some(additional) = deposit.checked_sub(&deposited) {
			<T as Config>::Currency::reserve(&who, additional)?;
		} else if let Some(rebate) = deposited.checked_sub(&deposit) {
			<T as Config>::Currency::unreserve(&who, rebate);
		};
		let info = ParaInfo { manager: who.clone(), deposit, locked: None };

		Paras::<T>::insert(id, info);
		// We check above that para has no lifecycle, so this should not fail.
		let res = pezkuwi_runtime_teyrchains::schedule_para_initialize::<T>(id, genesis);
		debug_assert!(res.is_ok());
		Self::deposit_event(Event::<T>::Registered { para_id: id, manager: who });
		Ok(())
	}

	/// Deregister a Para Id, freeing all data returning any deposit.
	fn do_deregister(id: ParaId) -> DispatchResult {
		match paras::Pezpallet::<T>::lifecycle(id) {
			// Para must be a parathread (on-demand teyrchain), or not exist at all.
			Some(ParaLifecycle::Parathread) | None => {},
			_ => return Err(Error::<T>::NotParathread.into()),
		}
		pezkuwi_runtime_teyrchains::schedule_para_cleanup::<T>(id)
			.map_err(|_| Error::<T>::CannotDeregister)?;

		if let Some(info) = Paras::<T>::take(&id) {
			<T as Config>::Currency::unreserve(&info.manager, info.deposit);
		}

		PendingSwap::<T>::remove(id);
		Self::deposit_event(Event::<T>::Deregistered { para_id: id });
		Ok(())
	}

	/// Verifies the onboarding data is valid for a para.
	///
	/// Returns `ParaGenesisArgs` and the deposit needed for the data.
	fn validate_onboarding_data(
		genesis_head: HeadData,
		validation_code: ValidationCode,
		para_kind: ParaKind,
	) -> Result<(ParaGenesisArgs, BalanceOf<T>), pezsp_runtime::DispatchError> {
		let config = configuration::ActiveConfig::<T>::get();
		ensure!(validation_code.0.len() >= MIN_CODE_SIZE as usize, Error::<T>::InvalidCode);
		ensure!(validation_code.0.len() <= config.max_code_size as usize, Error::<T>::CodeTooLarge);
		ensure!(
			genesis_head.0.len() <= config.max_head_data_size as usize,
			Error::<T>::HeadDataTooLarge
		);

		let per_byte_fee = T::DataDepositPerByte::get();
		let deposit = T::ParaDeposit::get()
			.saturating_add(per_byte_fee.saturating_mul((genesis_head.0.len() as u32).into()))
			.saturating_add(per_byte_fee.saturating_mul(config.max_code_size.into()));

		Ok((ParaGenesisArgs { genesis_head, validation_code, para_kind }, deposit))
	}

	/// Swap a lease holding teyrchain and parathread (on-demand teyrchain), which involves
	/// scheduling an appropriate lifecycle update.
	fn do_thread_and_chain_swap(to_downgrade: ParaId, to_upgrade: ParaId) {
		let res1 = pezkuwi_runtime_teyrchains::schedule_teyrchain_downgrade::<T>(to_downgrade);
		debug_assert!(res1.is_ok());
		let res2 = pezkuwi_runtime_teyrchains::schedule_parathread_upgrade::<T>(to_upgrade);
		debug_assert!(res2.is_ok());
		T::OnSwap::on_swap(to_upgrade, to_downgrade);
	}
}

impl<T: Config> OnNewHead for Pezpallet<T> {
	fn on_new_head(id: ParaId, _head: &HeadData) -> Weight {
		// mark the teyrchain locked if the locked value is not already set
		let mut writes = 0;
		if let Some(mut info) = Paras::<T>::get(id) {
			if info.locked.is_none() {
				info.locked = Some(true);
				Paras::<T>::insert(id, info);
				writes += 1;
			}
		}
		T::DbWeight::get().reads_writes(1, writes)
	}
}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
