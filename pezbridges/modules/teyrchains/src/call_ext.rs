// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

use crate::{Config, GrandpaPalletOf, Pezpallet, RelayBlockNumber};
use pezbp_header_pez_chain::HeaderChain;
use pezbp_runtime::{HeaderId, OwnedBridgeModule};
use pezbp_teyrchains::{BestParaHeadHash, SubmitTeyrchainHeadsInfo};
use pezframe_support::{
	dispatch::CallableCallFor,
	traits::{Get, IsSubType},
};
use pezpallet_bridge_grandpa::SubmitFinalityProofHelper;
use pezsp_runtime::{
	traits::Zero,
	transaction_validity::{InvalidTransaction, TransactionValidityError},
	RuntimeDebug,
};

/// Verified `SubmitTeyrchainHeadsInfo`.
#[derive(PartialEq, RuntimeDebug)]
pub struct VerifiedSubmitTeyrchainHeadsInfo {
	/// Base call information.
	pub base: SubmitTeyrchainHeadsInfo,
	/// A difference between bundled bridged relay chain header and relay chain header number
	/// used to prove best bridged teyrchain header, known to us before the call.
	pub improved_by: RelayBlockNumber,
}

/// Helper struct that provides methods for working with the `SubmitTeyrchainHeads` call.
pub struct SubmitTeyrchainHeadsHelper<T: Config<I>, I: 'static> {
	_phantom_data: pezsp_std::marker::PhantomData<(T, I)>,
}

impl<T: Config<I>, I: 'static> SubmitTeyrchainHeadsHelper<T, I> {
	/// Check that is called from signed extension and takes the `is_free_execution_expected`
	/// into account.
	pub fn check_obsolete_from_extension(
		update: &SubmitTeyrchainHeadsInfo,
	) -> Result<RelayBlockNumber, TransactionValidityError> {
		// first do all base checks
		let improved_by = Self::check_obsolete(update)?;

		// if we don't expect free execution - no more checks
		if !update.is_free_execution_expected {
			return Ok(improved_by);
		}

		// reject if no more free slots remaining in the block
		if !SubmitFinalityProofHelper::<T, T::BridgesGrandpaPalletInstance>::has_free_header_slots()
		{
			tracing::trace!(
				target: crate::LOG_TARGET,
				para_id=?update.para_id,
				"The free teyrchain head can't be updated: no more free slots left in the block."
			);

			return Err(InvalidTransaction::Call.into());
		}

		// if free headers interval is not configured and call is expected to execute
		// for free => it is a relayer error, it should've been able to detect that.
		let free_headers_interval = match T::FreeHeadersInterval::get() {
			Some(free_headers_interval) => free_headers_interval,
			None => return Ok(improved_by),
		};

		// reject if we are importing teyrchain headers too often
		if improved_by < free_headers_interval {
			tracing::trace!(
				target: crate::LOG_TARGET,
				para_id=?update.para_id,
				%improved_by,
				"The free teyrchain head can't be updated: it improves previous
				best head while at least {free_headers_interval} is expected."
			);

			return Err(InvalidTransaction::Stale.into());
		}

		Ok(improved_by)
	}

	/// Check if the para head provided by the `SubmitTeyrchainHeads` is better than the best one
	/// we know.
	pub fn check_obsolete(
		update: &SubmitTeyrchainHeadsInfo,
	) -> Result<RelayBlockNumber, TransactionValidityError> {
		// check if we know better teyrchain head already
		let improved_by = match crate::ParasInfo::<T, I>::get(update.para_id) {
			Some(stored_best_head) => {
				let improved_by = match update
					.at_relay_block
					.0
					.checked_sub(stored_best_head.best_head_hash.at_relay_block_number)
				{
					Some(improved_by) if improved_by > Zero::zero() => improved_by,
					_ => {
						tracing::trace!(
							target: crate::LOG_TARGET,
							para_id=?update.para_id,
							"The teyrchain head can't be updated. The teyrchain head \
								was already updated at better relay chain block {} >= {}.",
							stored_best_head.best_head_hash.at_relay_block_number,
							update.at_relay_block.0
						);
						return Err(InvalidTransaction::Stale.into());
					},
				};

				if stored_best_head.best_head_hash.head_hash == update.para_head_hash {
					tracing::trace!(
						target: crate::LOG_TARGET,
						para_id=?update.para_id,
						para_head_hash=%update.para_head_hash,
						"The teyrchain head can't be updated. The teyrchain head hash \
						was already updated at block {} < {}.",
						stored_best_head.best_head_hash.at_relay_block_number,
						update.at_relay_block.0
					);
					return Err(InvalidTransaction::Stale.into());
				}

				improved_by
			},
			None => RelayBlockNumber::MAX,
		};

		// let's check if our chain had no reorgs and we still know the relay chain header
		// used to craft the proof
		if GrandpaPalletOf::<T, I>::finalized_header_state_root(update.at_relay_block.1).is_none() {
			tracing::trace!(
				target: crate::LOG_TARGET,
				para_id=?update.para_id,
				at_relay_block=?update.at_relay_block,
				"The teyrchain head can't be updated. Relay chain header used to create \
				teyrchain proof is missing from the storage."
			);

			return Err(InvalidTransaction::Call.into());
		}

		Ok(improved_by)
	}

	/// Check if the `SubmitTeyrchainHeads` was successfully executed.
	pub fn was_successful(update: &SubmitTeyrchainHeadsInfo) -> bool {
		match crate::ParasInfo::<T, I>::get(update.para_id) {
			Some(stored_best_head) => {
				stored_best_head.best_head_hash
					== BestParaHeadHash {
						at_relay_block_number: update.at_relay_block.0,
						head_hash: update.para_head_hash,
					}
			},
			None => false,
		}
	}
}

/// Trait representing a call that is a sub type of this pezpallet's call.
pub trait CallSubType<T: Config<I, RuntimeCall = Self>, I: 'static>:
	IsSubType<CallableCallFor<Pezpallet<T, I>, T>>
{
	/// Create a new instance of `SubmitTeyrchainHeadsInfo` from a `SubmitTeyrchainHeads` call with
	/// one single teyrchain entry.
	fn one_entry_submit_teyrchain_heads_info(&self) -> Option<SubmitTeyrchainHeadsInfo> {
		match self.is_sub_type() {
			Some(crate::Call::<T, I>::submit_teyrchain_heads {
				ref at_relay_block,
				ref teyrchains,
				..
			}) => match &teyrchains[..] {
				&[(para_id, para_head_hash)] => Some(SubmitTeyrchainHeadsInfo {
					at_relay_block: HeaderId(at_relay_block.0, at_relay_block.1),
					para_id,
					para_head_hash,
					is_free_execution_expected: false,
				}),
				_ => None,
			},
			Some(crate::Call::<T, I>::submit_teyrchain_heads_ex {
				ref at_relay_block,
				ref teyrchains,
				is_free_execution_expected,
				..
			}) => match &teyrchains[..] {
				&[(para_id, para_head_hash)] => Some(SubmitTeyrchainHeadsInfo {
					at_relay_block: HeaderId(at_relay_block.0, at_relay_block.1),
					para_id,
					para_head_hash,
					is_free_execution_expected: *is_free_execution_expected,
				}),
				_ => None,
			},
			_ => None,
		}
	}

	/// Create a new instance of `SubmitTeyrchainHeadsInfo` from a `SubmitTeyrchainHeads` call with
	/// one single teyrchain entry, if the entry is for the provided teyrchain id.
	fn submit_teyrchain_heads_info_for(&self, para_id: u32) -> Option<SubmitTeyrchainHeadsInfo> {
		self.one_entry_submit_teyrchain_heads_info()
			.filter(|update| update.para_id.0 == para_id)
	}

	/// Validate teyrchain heads in order to avoid "mining" transactions that provide
	/// outdated bridged teyrchain heads. Without this validation, even honest relayers
	/// may lose their funds if there are multiple relays running and submitting the
	/// same information.
	///
	/// This validation only works with transactions that are updating single teyrchain
	/// head. We can't use unbounded validation - it may take too long and either break
	/// block production, or "eat" significant portion of block production time literally
	/// for nothing. In addition, the single-teyrchain-head-per-transaction is how the
	/// pezpallet will be used in our environment.
	fn check_obsolete_submit_teyrchain_heads(
		&self,
	) -> Result<Option<VerifiedSubmitTeyrchainHeadsInfo>, TransactionValidityError>
	where
		Self: Sized,
	{
		let update = match self.one_entry_submit_teyrchain_heads_info() {
			Some(update) => update,
			None => return Ok(None),
		};

		if Pezpallet::<T, I>::ensure_not_halted().is_err() {
			return Err(InvalidTransaction::Call.into());
		}

		SubmitTeyrchainHeadsHelper::<T, I>::check_obsolete_from_extension(&update)
			.map(|improved_by| Some(VerifiedSubmitTeyrchainHeadsInfo { base: update, improved_by }))
	}
}

impl<T, I: 'static> CallSubType<T, I> for T::RuntimeCall
where
	T: Config<I>,
	T::RuntimeCall: IsSubType<CallableCallFor<Pezpallet<T, I>, T>>,
{
}

#[cfg(test)]
mod tests {
	use crate::{
		mock::{run_test, FreeHeadersInterval, RuntimeCall, TestRuntime},
		CallSubType, PalletOperatingMode, ParaInfo, ParasInfo, RelayBlockHash, RelayBlockNumber,
	};
	use pezbp_header_pez_chain::StoredHeaderData;
	use pezbp_pezkuwi_core::teyrchains::{ParaHash, ParaHeadsProof, ParaId};
	use pezbp_runtime::BasicOperatingMode;
	use pezbp_teyrchains::BestParaHeadHash;

	fn validate_submit_teyrchain_heads(
		num: RelayBlockNumber,
		teyrchains: Vec<(ParaId, ParaHash)>,
	) -> bool {
		RuntimeCall::Teyrchains(crate::Call::<TestRuntime, ()>::submit_teyrchain_heads_ex {
			at_relay_block: (num, [num as u8; 32].into()),
			teyrchains,
			teyrchain_heads_proof: ParaHeadsProof { storage_proof: Default::default() },
			is_free_execution_expected: false,
		})
		.check_obsolete_submit_teyrchain_heads()
		.is_ok()
	}

	fn validate_free_submit_teyrchain_heads(
		num: RelayBlockNumber,
		teyrchains: Vec<(ParaId, ParaHash)>,
	) -> bool {
		RuntimeCall::Teyrchains(crate::Call::<TestRuntime, ()>::submit_teyrchain_heads_ex {
			at_relay_block: (num, [num as u8; 32].into()),
			teyrchains,
			teyrchain_heads_proof: ParaHeadsProof { storage_proof: Default::default() },
			is_free_execution_expected: true,
		})
		.check_obsolete_submit_teyrchain_heads()
		.is_ok()
	}

	fn insert_relay_block(num: RelayBlockNumber) {
		pezpallet_bridge_grandpa::ImportedHeaders::<TestRuntime, crate::Instance1>::insert(
			RelayBlockHash::from([num as u8; 32]),
			StoredHeaderData { number: num, state_root: RelayBlockHash::from([10u8; 32]) },
		);
	}

	fn sync_to_relay_header_10() {
		ParasInfo::<TestRuntime, ()>::insert(
			ParaId(1),
			ParaInfo {
				best_head_hash: BestParaHeadHash {
					at_relay_block_number: 10,
					head_hash: [1u8; 32].into(),
				},
				next_imported_hash_position: 0,
			},
		);
	}

	#[test]
	fn extension_rejects_header_from_the_obsolete_relay_block() {
		run_test(|| {
			// when current best finalized is #10 and we're trying to import header#5 => tx is
			// rejected
			sync_to_relay_header_10();
			assert!(!validate_submit_teyrchain_heads(5, vec![(ParaId(1), [1u8; 32].into())]));
		});
	}

	#[test]
	fn extension_rejects_header_from_the_same_relay_block() {
		run_test(|| {
			// when current best finalized is #10 and we're trying to import header#10 => tx is
			// rejected
			sync_to_relay_header_10();
			assert!(!validate_submit_teyrchain_heads(10, vec![(ParaId(1), [1u8; 32].into())]));
		});
	}

	#[test]
	fn extension_rejects_header_from_new_relay_block_with_same_hash() {
		run_test(|| {
			// when current best finalized is #10 and we're trying to import header#10 => tx is
			// rejected
			sync_to_relay_header_10();
			assert!(!validate_submit_teyrchain_heads(20, vec![(ParaId(1), [1u8; 32].into())]));
		});
	}

	#[test]
	fn extension_rejects_header_if_pallet_is_halted() {
		run_test(|| {
			// when pezpallet is halted => tx is rejected
			sync_to_relay_header_10();
			PalletOperatingMode::<TestRuntime, ()>::put(BasicOperatingMode::Halted);

			assert!(!validate_submit_teyrchain_heads(15, vec![(ParaId(1), [2u8; 32].into())]));
		});
	}

	#[test]
	fn extension_accepts_new_header() {
		run_test(|| {
			// when current best finalized is #10 and we're trying to import header#15 => tx is
			// accepted
			sync_to_relay_header_10();
			insert_relay_block(15);
			assert!(validate_submit_teyrchain_heads(15, vec![(ParaId(1), [2u8; 32].into())]));
		});
	}

	#[test]
	fn extension_accepts_if_more_than_one_teyrchain_is_submitted() {
		run_test(|| {
			// when current best finalized is #10 and we're trying to import header#5, but another
			// teyrchain head is also supplied => tx is accepted
			sync_to_relay_header_10();
			assert!(validate_submit_teyrchain_heads(
				5,
				vec![(ParaId(1), [1u8; 32].into()), (ParaId(2), [1u8; 32].into())]
			));
		});
	}

	#[test]
	fn extension_rejects_initial_teyrchain_head_if_missing_relay_chain_header() {
		run_test(|| {
			// when relay chain header is unknown => "obsolete"
			assert!(!validate_submit_teyrchain_heads(10, vec![(ParaId(1), [1u8; 32].into())]));
			// when relay chain header is unknown => "ok"
			insert_relay_block(10);
			assert!(validate_submit_teyrchain_heads(10, vec![(ParaId(1), [1u8; 32].into())]));
		});
	}

	#[test]
	fn extension_rejects_free_teyrchain_head_if_missing_relay_chain_header() {
		run_test(|| {
			sync_to_relay_header_10();
			// when relay chain header is unknown => "obsolete"
			assert!(!validate_submit_teyrchain_heads(15, vec![(ParaId(2), [15u8; 32].into())]));
			// when relay chain header is unknown => "ok"
			insert_relay_block(15);
			assert!(validate_submit_teyrchain_heads(15, vec![(ParaId(2), [15u8; 32].into())]));
		});
	}

	#[test]
	fn extension_rejects_free_teyrchain_head_if_no_free_slots_remaining() {
		run_test(|| {
			// when current best finalized is #10 and we're trying to import header#15 => tx should
			// be accepted
			sync_to_relay_header_10();
			insert_relay_block(15);
			// ... but since we have specified `is_free_execution_expected = true`, it'll be
			// rejected
			assert!(!validate_free_submit_teyrchain_heads(15, vec![(ParaId(1), [2u8; 32].into())]));
			// ... if we have specify `is_free_execution_expected = false`, it'll be accepted
			assert!(validate_submit_teyrchain_heads(15, vec![(ParaId(1), [2u8; 32].into())]));
		});
	}

	#[test]
	fn extension_rejects_free_teyrchain_head_if_improves_by_is_below_expected() {
		run_test(|| {
			// when current best finalized is #10 and we're trying to import header#15 => tx should
			// be accepted
			sync_to_relay_header_10();
			insert_relay_block(10 + FreeHeadersInterval::get() - 1);
			insert_relay_block(10 + FreeHeadersInterval::get());
			// try to submit at 10 + FreeHeadersInterval::get() - 1 => failure
			let relay_header = 10 + FreeHeadersInterval::get() - 1;
			assert!(!validate_free_submit_teyrchain_heads(
				relay_header,
				vec![(ParaId(1), [2u8; 32].into())]
			));
			// try to submit at 10 + FreeHeadersInterval::get() => ok
			let relay_header = 10 + FreeHeadersInterval::get();
			assert!(validate_free_submit_teyrchain_heads(
				relay_header,
				vec![(ParaId(1), [2u8; 32].into())]
			));
		});
	}
}
