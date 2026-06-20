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

//! Mocking utilities for testing.

use crate::traits::Registrar;
use codec::{Decode, Encode};
use pezframe_support::{dispatch::DispatchResult, weights::Weight};
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezkuwi_primitives::{HeadData, Id as ParaId, PvfCheckStatement, SessionIndex, ValidationCode};
use pezkuwi_runtime_teyrchains::paras;
use pezsp_keyring::Sr25519Keyring;
use pezsp_runtime::{traits::SaturatedConversion, DispatchError, Permill};
use std::{cell::RefCell, collections::HashMap};

thread_local! {
	static OPERATIONS: RefCell<Vec<(ParaId, u32, bool)>> = RefCell::new(Vec::new());
	static TEYRCHAINS: RefCell<Vec<ParaId>> = RefCell::new(Vec::new());
	// On-demand teyrchains
	static PARATHREADS: RefCell<Vec<ParaId>> = RefCell::new(Vec::new());
	static LOCKS: RefCell<HashMap<ParaId, bool>> = RefCell::new(HashMap::new());
	static MANAGERS: RefCell<HashMap<ParaId, Vec<u8>>> = RefCell::new(HashMap::new());
}

pub struct TestRegistrar<T>(core::marker::PhantomData<T>);

impl<T: pezframe_system::Config> Registrar for TestRegistrar<T> {
	type AccountId = T::AccountId;

	fn manager_of(id: ParaId) -> Option<Self::AccountId> {
		MANAGERS.with(|x| x.borrow().get(&id).and_then(|v| T::AccountId::decode(&mut &v[..]).ok()))
	}

	fn teyrchains() -> Vec<ParaId> {
		TEYRCHAINS.with(|x| x.borrow().clone())
	}

	// Is on-demand teyrchain
	fn is_parathread(id: ParaId) -> bool {
		PARATHREADS.with(|x| x.borrow().binary_search(&id).is_ok())
	}

	fn apply_lock(id: ParaId) {
		LOCKS.with(|x| x.borrow_mut().insert(id, true));
	}

	fn remove_lock(id: ParaId) {
		LOCKS.with(|x| x.borrow_mut().insert(id, false));
	}

	fn register(
		manager: Self::AccountId,
		id: ParaId,
		_genesis_head: HeadData,
		_validation_code: ValidationCode,
	) -> DispatchResult {
		// Should not be teyrchain.
		TEYRCHAINS.with(|x| {
			let teyrchains = x.borrow_mut();
			match teyrchains.binary_search(&id) {
				Ok(_) => Err(DispatchError::Other("Already Teyrchain")),
				Err(_) => Ok(()),
			}
		})?;
		// Should not be parathread (on-demand teyrchain), then make it.
		PARATHREADS.with(|x| {
			let mut parathreads = x.borrow_mut();
			match parathreads.binary_search(&id) {
				Ok(_) => Err(DispatchError::Other("Already Parathread")),
				Err(i) => {
					parathreads.insert(i, id);
					Ok(())
				},
			}
		})?;
		MANAGERS.with(|x| x.borrow_mut().insert(id, manager.encode()));
		Ok(())
	}

	fn deregister(id: ParaId) -> DispatchResult {
		// Should not be teyrchain.
		TEYRCHAINS.with(|x| {
			let teyrchains = x.borrow_mut();
			match teyrchains.binary_search(&id) {
				Ok(_) => Err(DispatchError::Other("cannot deregister teyrchain")),
				Err(_) => Ok(()),
			}
		})?;
		// Remove from parathreads (on-demand teyrchains).
		PARATHREADS.with(|x| {
			let mut parathreads = x.borrow_mut();
			match parathreads.binary_search(&id) {
				Ok(i) => {
					parathreads.remove(i);
					Ok(())
				},
				Err(_) => Err(DispatchError::Other("not parathread, so cannot `deregister`")),
			}
		})?;
		MANAGERS.with(|x| x.borrow_mut().remove(&id));
		Ok(())
	}

	/// If the ParaId corresponds to a parathread (on-demand teyrchain),
	/// then upgrade it to a lease holding teyrchain
	fn make_teyrchain(id: ParaId) -> DispatchResult {
		PARATHREADS.with(|x| {
			let mut parathreads = x.borrow_mut();
			match parathreads.binary_search(&id) {
				Ok(i) => {
					parathreads.remove(i);
					Ok(())
				},
				Err(_) => Err(DispatchError::Other("not parathread, so cannot `make_teyrchain`")),
			}
		})?;
		TEYRCHAINS.with(|x| {
			let mut teyrchains = x.borrow_mut();
			match teyrchains.binary_search(&id) {
				Ok(_) => Err(DispatchError::Other("already teyrchain, so cannot `make_teyrchain`")),
				Err(i) => {
					teyrchains.insert(i, id);
					Ok(())
				},
			}
		})?;
		OPERATIONS.with(|x| {
			x.borrow_mut().push((
				id,
				pezframe_system::Pezpallet::<T>::block_number().saturated_into(),
				true,
			))
		});
		Ok(())
	}

	/// If the ParaId corresponds to a lease holding teyrchain, then downgrade it to a
	/// parathread (on-demand teyrchain)
	fn make_parathread(id: ParaId) -> DispatchResult {
		TEYRCHAINS.with(|x| {
			let mut teyrchains = x.borrow_mut();
			match teyrchains.binary_search(&id) {
				Ok(i) => {
					teyrchains.remove(i);
					Ok(())
				},
				Err(_) => Err(DispatchError::Other("not teyrchain, so cannot `make_parathread`")),
			}
		})?;
		PARATHREADS.with(|x| {
			let mut parathreads = x.borrow_mut();
			match parathreads.binary_search(&id) {
				Ok(_) => {
					Err(DispatchError::Other("already parathread, so cannot `make_parathread`"))
				},
				Err(i) => {
					parathreads.insert(i, id);
					Ok(())
				},
			}
		})?;
		OPERATIONS.with(|x| {
			x.borrow_mut().push((
				id,
				pezframe_system::Pezpallet::<T>::block_number().saturated_into(),
				false,
			))
		});
		Ok(())
	}

	#[cfg(test)]
	fn worst_head_data() -> HeadData {
		vec![0u8; 1000].into()
	}

	#[cfg(test)]
	fn worst_validation_code() -> ValidationCode {
		let validation_code = vec![0u8; 1000];
		validation_code.into()
	}

	#[cfg(test)]
	fn execute_pending_transitions() {}
}

impl<T: pezframe_system::Config> TestRegistrar<T> {
	pub fn operations() -> Vec<(ParaId, BlockNumberFor<T>, bool)> {
		OPERATIONS
			.with(|x| x.borrow().iter().map(|(p, b, c)| (*p, (*b).into(), *c)).collect::<Vec<_>>())
	}

	#[allow(dead_code)]
	pub fn teyrchains() -> Vec<ParaId> {
		TEYRCHAINS.with(|x| x.borrow().clone())
	}

	#[allow(dead_code)]
	pub fn parathreads() -> Vec<ParaId> {
		PARATHREADS.with(|x| x.borrow().clone())
	}

	#[allow(dead_code)]
	pub fn clear_storage() {
		OPERATIONS.with(|x| x.borrow_mut().clear());
		TEYRCHAINS.with(|x| x.borrow_mut().clear());
		PARATHREADS.with(|x| x.borrow_mut().clear());
		MANAGERS.with(|x| x.borrow_mut().clear());
	}
}

/// A very dumb implementation of `EstimateNextSessionRotation`. At the moment of writing, this
/// is more to satisfy type requirements rather than to test anything.
pub struct TestNextSessionRotation;

impl pezframe_support::traits::EstimateNextSessionRotation<u32> for TestNextSessionRotation {
	fn average_session_length() -> u32 {
		10
	}

	fn estimate_current_session_progress(_now: u32) -> (Option<Permill>, Weight) {
		(None, Weight::zero())
	}

	fn estimate_next_session_rotation(_now: u32) -> (Option<u32>, Weight) {
		(None, Weight::zero())
	}
}

pub fn validators_public_keys(
	validators: &[Sr25519Keyring],
) -> Vec<pezkuwi_primitives::ValidatorId> {
	validators.iter().map(|v| v.public().into()).collect()
}

pub fn conclude_pvf_checking<T: paras::Config>(
	validation_code: &ValidationCode,
	validators: &[Sr25519Keyring],
	session_index: SessionIndex,
) {
	let num_required = pezkuwi_primitives::supermajority_threshold(validators.len());
	validators.iter().enumerate().take(num_required).for_each(|(idx, key)| {
		let validator_index = idx as u32;
		let statement = PvfCheckStatement {
			accept: true,
			subject: validation_code.hash(),
			session_index,
			validator_index: validator_index.into(),
		};
		let signature = key.sign(&statement.signing_payload());
		let _ = paras::Pezpallet::<T>::include_pvf_check_statement(
			pezframe_system::Origin::<T>::None.into(),
			statement,
			signature.into(),
		);
	});
}
