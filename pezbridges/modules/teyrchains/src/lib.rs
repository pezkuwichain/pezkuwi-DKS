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

//! Teyrchains finality module.
//!
//! This module needs to be deployed with GRANDPA module, which is syncing relay
//! chain blocks. The main entry point of this module is `submit_teyrchain_heads`, which
//! accepts storage proof of some teyrchain `Heads` entries from bridged relay chain.
//! It requires corresponding relay headers to be already synced.

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use weights::WeightInfo;
pub use weights_ext::WeightInfoExt;

use pezbp_header_pez_chain::{HeaderChain, HeaderChainError};
use pezbp_pezkuwi_core::teyrchains::{ParaHash, ParaHead, ParaHeadsProof, ParaId};
use pezbp_runtime::{Chain, HashOf, HeaderId, HeaderIdOf, Teyrchain};
use pezbp_teyrchains::{
	ParaInfo, ParaStoredHeaderData, RelayBlockHash, RelayBlockHasher, RelayBlockNumber,
	SubmitTeyrchainHeadsInfo,
};
use pezframe_support::{dispatch::PostDispatchInfo, DefaultNoBound};
use pezpallet_bridge_grandpa::SubmitFinalityProofHelper;
use pezsp_std::{marker::PhantomData, vec::Vec};
use proofs::{StorageProofAdapter, TeyrchainsStorageProofAdapter};

#[cfg(feature = "runtime-benchmarks")]
use codec::Encode;
#[cfg(feature = "runtime-benchmarks")]
use pezbp_runtime::HeaderOf;
#[cfg(feature = "runtime-benchmarks")]
use pezbp_teyrchains::ParaStoredHeaderDataBuilder;

// Re-export in crate namespace for `construct_runtime!`.
pub use call_ext::*;
pub use pezpallet::*;

pub mod weights;
pub mod weights_ext;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

mod call_ext;
#[cfg(test)]
mod mock;
mod proofs;

/// The target that will be used when publishing logs related to this pezpallet.
pub const LOG_TARGET: &str = "runtime::bridge-teyrchains";

/// Artifacts of the teyrchains head update.
struct UpdateTeyrchainHeadArtifacts {
	/// New best head of the teyrchain.
	pub best_head: ParaInfo,
	/// If `true`, some old teyrchain head has been pruned during update.
	pub prune_happened: bool,
}

#[pezframe_support::pezpallet]
pub mod pezpallet {
	use super::*;
	use pezbp_runtime::{
		BasicOperatingMode, BoundedStorageValue, OwnedBridgeModule, StorageDoubleMapKeyProvider,
		StorageMapKeyProvider,
	};
	use pezbp_teyrchains::{
		BestParaHeadHash, ImportedParaHeadsKeyProvider, OnNewHead, ParaStoredHeaderDataBuilder,
		ParasInfoKeyProvider,
	};
	use pezframe_support::pezpallet_prelude::*;
	use pezframe_system::pezpallet_prelude::*;

	/// Stored teyrchain head data of given teyrchains pezpallet.
	pub type StoredParaHeadDataOf<T, I> =
		BoundedStorageValue<<T as Config<I>>::MaxParaHeadDataSize, ParaStoredHeaderData>;
	/// Weight info of the given teyrchains pezpallet.
	pub type WeightInfoOf<T, I> = <T as Config<I>>::WeightInfo;
	/// Bridge GRANDPA pezpallet that is used to verify teyrchain proofs.
	pub type GrandpaPalletOf<T, I> =
		pezpallet_bridge_grandpa::Pezpallet<T, <T as Config<I>>::BridgesGrandpaPalletInstance>;

	#[pezpallet::event]
	#[pezpallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// The caller has provided head of teyrchain that the pezpallet is not configured to
		/// track.
		UntrackedTeyrchainRejected {
			/// Identifier of the teyrchain that is not tracked by the pezpallet.
			teyrchain: ParaId,
		},
		/// The caller has declared that he has provided given teyrchain head, but it is missing
		/// from the storage proof.
		MissingTeyrchainHead {
			/// Identifier of the teyrchain with missing head.
			teyrchain: ParaId,
		},
		/// The caller has provided teyrchain head hash that is not matching the hash read from the
		/// storage proof.
		IncorrectTeyrchainHeadHash {
			/// Identifier of the teyrchain with incorrect head hast.
			teyrchain: ParaId,
			/// Specified teyrchain head hash.
			teyrchain_head_hash: ParaHash,
			/// Actual teyrchain head hash.
			actual_teyrchain_head_hash: ParaHash,
		},
		/// The caller has provided obsolete teyrchain head, which is already known to the
		/// pezpallet.
		RejectedObsoleteTeyrchainHead {
			/// Identifier of the teyrchain with obsolete head.
			teyrchain: ParaId,
			/// Obsolete teyrchain head hash.
			teyrchain_head_hash: ParaHash,
		},
		/// The caller has provided teyrchain head that exceeds the maximal configured head size.
		RejectedLargeTeyrchainHead {
			/// Identifier of the teyrchain with rejected head.
			teyrchain: ParaId,
			/// Teyrchain head hash.
			teyrchain_head_hash: ParaHash,
			/// Teyrchain head size.
			teyrchain_head_size: u32,
		},
		/// Teyrchain head has been updated.
		UpdatedTeyrchainHead {
			/// Identifier of the teyrchain that has been updated.
			teyrchain: ParaId,
			/// Teyrchain head hash.
			teyrchain_head_hash: ParaHash,
		},
	}

	#[pezpallet::error]
	pub enum Error<T, I = ()> {
		/// Relay chain block hash is unknown to us.
		UnknownRelayChainBlock,
		/// The number of stored relay block is different from what the relayer has provided.
		InvalidRelayChainBlockNumber,
		/// Teyrchain heads storage proof is invalid.
		HeaderChainStorageProof(HeaderChainError),
		/// Error generated by the `OwnedBridgeModule` trait.
		BridgeModule(pezbp_runtime::OwnedBridgeModuleError),
	}

	/// Convenience trait for defining `BridgedChain` bounds.
	pub trait BoundedBridgeGrandpaConfig<I: 'static>:
		pezpallet_bridge_grandpa::Config<I, BridgedChain = Self::BridgedRelayChain>
	{
		/// Type of the bridged relay chain.
		type BridgedRelayChain: Chain<
			BlockNumber = RelayBlockNumber,
			Hash = RelayBlockHash,
			Hasher = RelayBlockHasher,
		>;
	}

	impl<T, I: 'static> BoundedBridgeGrandpaConfig<I> for T
	where
		T: pezpallet_bridge_grandpa::Config<I>,
		T::BridgedChain:
			Chain<BlockNumber = RelayBlockNumber, Hash = RelayBlockHash, Hasher = RelayBlockHasher>,
	{
		type BridgedRelayChain = T::BridgedChain;
	}

	#[pezpallet::config]
	#[pezpallet::disable_pezframe_system_supertrait_check]
	pub trait Config<I: 'static = ()>:
		BoundedBridgeGrandpaConfig<Self::BridgesGrandpaPalletInstance>
	{
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as pezframe_system::Config>::RuntimeEvent>;
		/// Benchmarks results from runtime we're plugged into.
		type WeightInfo: WeightInfoExt;

		/// Instance of bridges GRANDPA pezpallet (within this runtime) that this pezpallet is
		/// linked to.
		///
		/// The GRANDPA pezpallet instance must be configured to import headers of relay chain that
		/// we're interested in.
		///
		/// The associated GRANDPA pezpallet is also used to configure free teyrchain heads
		/// submissions. The teyrchain head submission will be free if:
		///
		/// 1) the submission contains exactly one teyrchain head update that succeeds;
		///
		/// 2) the difference between relay chain block numbers, used to prove new teyrchain head
		///    and previous best teyrchain head is larger than the `FreeHeadersInterval`, configured
		///    at the associated GRANDPA pezpallet;
		///
		/// 3) there are slots for free submissions, remaining at the block. This is also configured
		///    at the associated GRANDPA pezpallet using `MaxFreeHeadersPerBlock` parameter.
		///
		/// First teyrchain head submission is also free for the submitted, if free submissions
		/// are yet accepted to this block.
		type BridgesGrandpaPalletInstance: 'static;

		/// Name of the original `paras` pezpallet in the `construct_runtime!()` call at the bridged
		/// chain.
		///
		/// Please keep in mind that this should be the name of the `runtime_teyrchains::paras`
		/// pezpallet from pezkuwi repository, not the `pezpallet-bridge-teyrchains`.
		#[pezpallet::constant]
		type ParasPalletName: Get<&'static str>;

		/// Teyrchain head data builder.
		///
		/// We never store teyrchain heads here, since they may be too big (e.g. because of large
		/// digest items). Instead we're using the same approach as `pezpallet-bridge-grandpa`
		/// pezpallet - we are only storing `pezbp_messages::StoredHeaderData` (number and state root),
		/// which is enough for our applications. However, we work with different teyrchains here
		/// and they can use different primitives (for block numbers and hash). So we can't store
		/// it directly. Instead, we're storing `pezbp_messages::StoredHeaderData` in SCALE-encoded
		/// form, wrapping it into `pezbp_teyrchains::ParaStoredHeaderData`.
		///
		/// This builder helps to convert from `HeadData` to `pezbp_teyrchains::ParaStoredHeaderData`.
		type ParaStoredHeaderDataBuilder: ParaStoredHeaderDataBuilder;

		/// Maximal number of single teyrchain heads to keep in the storage.
		///
		/// The setting is there to prevent growing the on-chain state indefinitely. Note
		/// the setting does not relate to teyrchain block numbers - we will simply keep as much
		/// items in the storage, so it doesn't guarantee any fixed timeframe for heads.
		///
		/// Incautious change of this constant may lead to orphan entries in the runtime storage.
		#[pezpallet::constant]
		type HeadsToKeep: Get<u32>;

		/// Maximal size (in bytes) of the SCALE-encoded teyrchain head data
		/// (`pezbp_teyrchains::ParaStoredHeaderData`).
		///
		/// Keep in mind that the size of any tracked teyrchain header data must not exceed this
		/// value. So if you're going to track multiple teyrchains, one of which is using large
		/// hashes, you shall choose this maximal value.
		///
		/// There's no mandatory headers in this pezpallet, so it can't stall if there's some header
		/// that exceeds this bound.
		#[pezpallet::constant]
		type MaxParaHeadDataSize: Get<u32>;

		/// Runtime hook for when a teyrchain head is updated.
		type OnNewHead: OnNewHead;
	}

	/// Optional pezpallet owner.
	///
	/// Pezpallet owner has a right to halt all pezpallet operations and then resume them. If it is
	/// `None`, then there are no direct ways to halt/resume pezpallet operations, but other
	/// runtime methods may still be used to do that (i.e. democracy::referendum to update halt
	/// flag directly or call the `set_operating_mode`).
	#[pezpallet::storage]
	pub type PalletOwner<T: Config<I>, I: 'static = ()> =
		StorageValue<_, T::AccountId, OptionQuery>;

	/// The current operating mode of the pezpallet.
	///
	/// Depending on the mode either all, or no transactions will be allowed.
	#[pezpallet::storage]
	pub type PalletOperatingMode<T: Config<I>, I: 'static = ()> =
		StorageValue<_, BasicOperatingMode, ValueQuery>;

	/// Teyrchains info.
	///
	/// Contains the following info:
	/// - best teyrchain head hash
	/// - the head of the `ImportedParaHashes` ring buffer
	#[pezpallet::storage]
	pub type ParasInfo<T: Config<I>, I: 'static = ()> = StorageMap<
		Hasher = <ParasInfoKeyProvider as StorageMapKeyProvider>::Hasher,
		Key = <ParasInfoKeyProvider as StorageMapKeyProvider>::Key,
		Value = <ParasInfoKeyProvider as StorageMapKeyProvider>::Value,
		QueryKind = OptionQuery,
		OnEmpty = GetDefault,
		MaxValues = MaybeMaxTeyrchains<T, I>,
	>;

	/// State roots of teyrchain heads which have been imported into the pezpallet.
	#[pezpallet::storage]
	pub type ImportedParaHeads<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		Hasher1 = <ImportedParaHeadsKeyProvider as StorageDoubleMapKeyProvider>::Hasher1,
		Key1 = <ImportedParaHeadsKeyProvider as StorageDoubleMapKeyProvider>::Key1,
		Hasher2 = <ImportedParaHeadsKeyProvider as StorageDoubleMapKeyProvider>::Hasher2,
		Key2 = <ImportedParaHeadsKeyProvider as StorageDoubleMapKeyProvider>::Key2,
		Value = StoredParaHeadDataOf<T, I>,
		QueryKind = OptionQuery,
		OnEmpty = GetDefault,
		MaxValues = MaybeMaxTotalTeyrchainHashes<T, I>,
	>;

	/// A ring buffer of imported teyrchain head hashes. Ordered by the insertion time.
	#[pezpallet::storage]
	pub(super) type ImportedParaHashes<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		Hasher1 = Blake2_128Concat,
		Key1 = ParaId,
		Hasher2 = Twox64Concat,
		Key2 = u32,
		Value = ParaHash,
		QueryKind = OptionQuery,
		OnEmpty = GetDefault,
		MaxValues = MaybeMaxTotalTeyrchainHashes<T, I>,
	>;

	#[pezpallet::pezpallet]
	pub struct Pezpallet<T, I = ()>(PhantomData<(T, I)>);

	impl<T: Config<I>, I: 'static> OwnedBridgeModule<T> for Pezpallet<T, I> {
		const LOG_TARGET: &'static str = LOG_TARGET;
		type OwnerStorage = PalletOwner<T, I>;
		type OperatingMode = BasicOperatingMode;
		type OperatingModeStorage = PalletOperatingMode<T, I>;
	}

	#[pezpallet::call]
	impl<T: Config<I>, I: 'static> Pezpallet<T, I> {
		/// Submit proof of one or several teyrchain heads.
		///
		/// The proof is supposed to be proof of some `Heads` entries from the
		/// `pezkuwi-runtime-teyrchains::paras` pezpallet instance, deployed at the bridged chain.
		/// The proof is supposed to be crafted at the `relay_header_hash` that must already be
		/// imported by corresponding GRANDPA pezpallet at this chain.
		///
		/// The call fails if:
		///
		/// - the pezpallet is halted;
		///
		/// - the relay chain block `at_relay_block` is not imported by the associated bridge
		///   GRANDPA pezpallet.
		///
		/// The call may succeed, but some heads may not be updated e.g. because pezpallet knows
		/// better head or it isn't tracked by the pezpallet.
		#[pezpallet::call_index(0)]
		#[pezpallet::weight(WeightInfoOf::<T, I>::submit_teyrchain_heads_weight(
			T::DbWeight::get(),
			teyrchain_heads_proof,
			teyrchains.len() as _,
		))]
		pub fn submit_teyrchain_heads(
			origin: OriginFor<T>,
			at_relay_block: (RelayBlockNumber, RelayBlockHash),
			teyrchains: Vec<(ParaId, ParaHash)>,
			teyrchain_heads_proof: ParaHeadsProof,
		) -> DispatchResultWithPostInfo {
			Self::submit_teyrchain_heads_ex(
				origin,
				at_relay_block,
				teyrchains,
				teyrchain_heads_proof,
				false,
			)
		}

		/// Change `PalletOwner`.
		///
		/// May only be called either by root, or by `PalletOwner`.
		#[pezpallet::call_index(1)]
		#[pezpallet::weight((T::DbWeight::get().reads_writes(1, 1), DispatchClass::Operational))]
		pub fn set_owner(origin: OriginFor<T>, new_owner: Option<T::AccountId>) -> DispatchResult {
			<Self as OwnedBridgeModule<_>>::set_owner(origin, new_owner)
		}

		/// Halt or resume all pezpallet operations.
		///
		/// May only be called either by root, or by `PalletOwner`.
		#[pezpallet::call_index(2)]
		#[pezpallet::weight((T::DbWeight::get().reads_writes(1, 1), DispatchClass::Operational))]
		pub fn set_operating_mode(
			origin: OriginFor<T>,
			operating_mode: BasicOperatingMode,
		) -> DispatchResult {
			<Self as OwnedBridgeModule<_>>::set_operating_mode(origin, operating_mode)
		}

		/// Submit proof of one or several teyrchain heads.
		///
		/// The proof is supposed to be proof of some `Heads` entries from the
		/// `pezkuwi-runtime-teyrchains::paras` pezpallet instance, deployed at the bridged chain.
		/// The proof is supposed to be crafted at the `relay_header_hash` that must already be
		/// imported by corresponding GRANDPA pezpallet at this chain.
		///
		/// The call fails if:
		///
		/// - the pezpallet is halted;
		///
		/// - the relay chain block `at_relay_block` is not imported by the associated bridge
		///   GRANDPA pezpallet.
		///
		/// The call may succeed, but some heads may not be updated e.g. because pezpallet knows
		/// better head or it isn't tracked by the pezpallet.
		///
		/// The `is_free_execution_expected` parameter is not really used inside the call. It is
		/// used by the transaction extension, which should be registered at the runtime level. If
		/// this parameter is `true`, the transaction will be treated as invalid, if the call won't
		/// be executed for free. If transaction extension is not used by the runtime, this
		/// parameter is not used at all.
		#[pezpallet::call_index(3)]
		#[pezpallet::weight(WeightInfoOf::<T, I>::submit_teyrchain_heads_weight(
			T::DbWeight::get(),
			teyrchain_heads_proof,
			teyrchains.len() as _,
		))]
		pub fn submit_teyrchain_heads_ex(
			origin: OriginFor<T>,
			at_relay_block: (RelayBlockNumber, RelayBlockHash),
			teyrchains: Vec<(ParaId, ParaHash)>,
			teyrchain_heads_proof: ParaHeadsProof,
			_is_free_execution_expected: bool,
		) -> DispatchResultWithPostInfo {
			Self::ensure_not_halted().map_err(Error::<T, I>::BridgeModule)?;
			ensure_signed(origin)?;

			let total_teyrchains = teyrchains.len();
			let free_headers_interval =
				T::FreeHeadersInterval::get().unwrap_or(RelayBlockNumber::MAX);
			// the pezpallet allows two kind of free submissions
			// 1) if distance between all teyrchain heads is gte than the [`T::FreeHeadersInterval`]
			// 2) if all heads are the first heads of their teyrchains
			let mut free_teyrchain_heads = 0;

			// we'll need relay chain header to verify that teyrchains heads are always increasing.
			let (relay_block_number, relay_block_hash) = at_relay_block;
			let relay_block = pezpallet_bridge_grandpa::ImportedHeaders::<
				T,
				T::BridgesGrandpaPalletInstance,
			>::get(relay_block_hash)
			.ok_or(Error::<T, I>::UnknownRelayChainBlock)?;
			ensure!(
				relay_block.number == relay_block_number,
				Error::<T, I>::InvalidRelayChainBlockNumber,
			);

			// now parse storage proof and read teyrchain heads
			let mut actual_weight = WeightInfoOf::<T, I>::submit_teyrchain_heads_weight(
				T::DbWeight::get(),
				&teyrchain_heads_proof,
				teyrchains.len() as _,
			);

			let mut storage: TeyrchainsStorageProofAdapter<T, I> =
				TeyrchainsStorageProofAdapter::try_new_with_verified_storage_proof(
					relay_block_hash,
					teyrchain_heads_proof.storage_proof,
				)
				.map_err(Error::<T, I>::HeaderChainStorageProof)?;

			for (teyrchain, teyrchain_head_hash) in teyrchains {
				let teyrchain_head = match storage.read_teyrchain_head(teyrchain) {
					Ok(Some(teyrchain_head)) => teyrchain_head,
					Ok(None) => {
						tracing::trace!(
							target: LOG_TARGET,
							?teyrchain,
							"The head of teyrchain is None. {}",
							if ParasInfo::<T, I>::contains_key(teyrchain) {
								"Looks like it is not yet registered at the source relay chain"
							} else {
								"Looks like it has been deregistered from the source relay chain"
							},
						);
						Self::deposit_event(Event::MissingTeyrchainHead { teyrchain });
						continue;
					},
					Err(e) => {
						tracing::trace!(
							target: LOG_TARGET,
							error=?e,
							?teyrchain,
							"The read of head of teyrchain has failed"
						);
						Self::deposit_event(Event::MissingTeyrchainHead { teyrchain });
						continue;
					},
				};

				// if relayer has specified invalid teyrchain head hash, ignore the head
				// (this isn't strictly necessary, but better safe than sorry)
				let actual_teyrchain_head_hash = teyrchain_head.hash();
				if teyrchain_head_hash != actual_teyrchain_head_hash {
					tracing::trace!(
						target: LOG_TARGET,
						?teyrchain,
						?teyrchain_head_hash,
						?actual_teyrchain_head_hash,
						"The submitter has specified invalid teyrchain head hash"
					);
					Self::deposit_event(Event::IncorrectTeyrchainHeadHash {
						teyrchain,
						teyrchain_head_hash,
						actual_teyrchain_head_hash,
					});
					continue;
				}

				// convert from teyrchain head into stored teyrchain head data
				let teyrchain_head_size = teyrchain_head.0.len();
				let teyrchain_head_data =
					match T::ParaStoredHeaderDataBuilder::try_build(teyrchain, &teyrchain_head) {
						Some(teyrchain_head_data) => teyrchain_head_data,
						None => {
							tracing::trace!(
								target: LOG_TARGET,
								?teyrchain,
								"The head of teyrchain has been provided, but it is not tracked by the pezpallet"
							);
							Self::deposit_event(Event::UntrackedTeyrchainRejected { teyrchain });
							continue;
						},
					};

				let update_result: Result<_, ()> =
					ParasInfo::<T, I>::try_mutate(teyrchain, |stored_best_head| {
						let is_free = teyrchain_head_size
							< T::ParaStoredHeaderDataBuilder::max_free_head_size() as usize
							&& match stored_best_head {
								Some(ref best_head)
									if at_relay_block.0.saturating_sub(
										best_head.best_head_hash.at_relay_block_number,
									) >= free_headers_interval =>
								{
									true
								},
								Some(_) => false,
								None => true,
							};
						let artifacts = Pezpallet::<T, I>::update_teyrchain_head(
							teyrchain,
							stored_best_head.take(),
							HeaderId(relay_block_number, relay_block_hash),
							teyrchain_head_data,
							teyrchain_head_hash,
							teyrchain_head,
						)?;

						if is_free {
							free_teyrchain_heads = free_teyrchain_heads + 1;
						}

						*stored_best_head = Some(artifacts.best_head);
						Ok(artifacts.prune_happened)
					});

				// we're refunding weight if update has not happened and if pruning has not happened
				let is_update_happened = update_result.is_ok();
				if !is_update_happened {
					actual_weight = actual_weight.saturating_sub(
						WeightInfoOf::<T, I>::teyrchain_head_storage_write_weight(
							T::DbWeight::get(),
						),
					);
				}
				let is_prune_happened = matches!(update_result, Ok(true));
				if !is_prune_happened {
					actual_weight = actual_weight.saturating_sub(
						WeightInfoOf::<T, I>::teyrchain_head_pruning_weight(T::DbWeight::get()),
					);
				}
			}

			// even though we may have accepted some teyrchain heads, we can't allow relayers to
			// submit proof with unused trie nodes
			// => treat this as an error
			//
			// (we can throw error here, because now all our calls are transactional)
			storage.ensure_no_unused_keys().map_err(|e| {
				Error::<T, I>::HeaderChainStorageProof(HeaderChainError::StorageProof(e))
			})?;

			// check if we allow this submission for free
			let is_free = total_teyrchains == 1
				&& free_teyrchain_heads == total_teyrchains
				&& SubmitFinalityProofHelper::<T, T::BridgesGrandpaPalletInstance>::has_free_header_slots();
			let pays_fee = if is_free {
				tracing::trace!(target: LOG_TARGET, "Teyrchain heads update transaction is free");
				pezpallet_bridge_grandpa::on_free_header_imported::<
					T,
					T::BridgesGrandpaPalletInstance,
				>();
				Pays::No
			} else {
				tracing::trace!(target: LOG_TARGET, "Teyrchain heads update transaction is paid");
				Pays::Yes
			};

			Ok(PostDispatchInfo { actual_weight: Some(actual_weight), pays_fee })
		}
	}

	impl<T: Config<I>, I: 'static> Pezpallet<T, I> {
		/// Get stored teyrchain info.
		pub fn best_teyrchain_info(teyrchain: ParaId) -> Option<ParaInfo> {
			ParasInfo::<T, I>::get(teyrchain)
		}

		/// Get best finalized head data of the given teyrchain.
		pub fn best_teyrchain_head(teyrchain: ParaId) -> Option<ParaStoredHeaderData> {
			let best_para_head_hash = ParasInfo::<T, I>::get(teyrchain)?.best_head_hash.head_hash;
			ImportedParaHeads::<T, I>::get(teyrchain, best_para_head_hash).map(|h| h.into_inner())
		}

		/// Get best finalized head hash of the given teyrchain.
		pub fn best_teyrchain_head_hash(teyrchain: ParaId) -> Option<ParaHash> {
			Some(ParasInfo::<T, I>::get(teyrchain)?.best_head_hash.head_hash)
		}

		/// Get best finalized head id of the given teyrchain.
		pub fn best_teyrchain_head_id<C: Chain<Hash = ParaHash> + Teyrchain>(
		) -> Result<Option<HeaderIdOf<C>>, codec::Error> {
			let teyrchain = ParaId(C::TEYRCHAIN_ID);
			let best_head_hash = match Self::best_teyrchain_head_hash(teyrchain) {
				Some(best_head_hash) => best_head_hash,
				None => return Ok(None),
			};
			let encoded_head = match Self::teyrchain_head(teyrchain, best_head_hash) {
				Some(encoded_head) => encoded_head,
				None => return Ok(None),
			};
			encoded_head
				.decode_teyrchain_head_data::<C>()
				.map(|data| Some(HeaderId(data.number, best_head_hash)))
		}

		/// Get teyrchain head data with given hash.
		pub fn teyrchain_head(teyrchain: ParaId, hash: ParaHash) -> Option<ParaStoredHeaderData> {
			ImportedParaHeads::<T, I>::get(teyrchain, hash).map(|h| h.into_inner())
		}

		/// Try to update teyrchain head.
		pub(super) fn update_teyrchain_head(
			teyrchain: ParaId,
			stored_best_head: Option<ParaInfo>,
			new_at_relay_block: HeaderId<RelayBlockHash, RelayBlockNumber>,
			new_head_data: ParaStoredHeaderData,
			new_head_hash: ParaHash,
			new_head: ParaHead,
		) -> Result<UpdateTeyrchainHeadArtifacts, ()> {
			// check if head has been already updated at better relay chain block. Without this
			// check, we may import heads in random order
			let update = SubmitTeyrchainHeadsInfo {
				at_relay_block: new_at_relay_block,
				para_id: teyrchain,
				para_head_hash: new_head_hash,
				// doesn't actually matter here
				is_free_execution_expected: false,
			};
			if SubmitTeyrchainHeadsHelper::<T, I>::check_obsolete(&update).is_err() {
				Self::deposit_event(Event::RejectedObsoleteTeyrchainHead {
					teyrchain,
					teyrchain_head_hash: new_head_hash,
				});
				return Err(());
			}

			// verify that the teyrchain head data size is <= `MaxParaHeadDataSize`
			let updated_head_data = match StoredParaHeadDataOf::<T, I>::try_from_inner(
				new_head_data,
			) {
				Ok(updated_head_data) => updated_head_data,
				Err(e) => {
					tracing::trace!(
						target: LOG_TARGET,
						error=?e,
						?teyrchain,
						"The teyrchain head can't be updated. The teyrchain head data size exceeds maximal configured size."
					);

					Self::deposit_event(Event::RejectedLargeTeyrchainHead {
						teyrchain,
						teyrchain_head_hash: new_head_hash,
						teyrchain_head_size: e.value_size as _,
					});

					return Err(());
				},
			};

			let next_imported_hash_position = stored_best_head
				.map_or(0, |stored_best_head| stored_best_head.next_imported_hash_position);

			// insert updated best teyrchain head
			let head_hash_to_prune =
				ImportedParaHashes::<T, I>::try_get(teyrchain, next_imported_hash_position);
			let updated_best_para_head = ParaInfo {
				best_head_hash: BestParaHeadHash {
					at_relay_block_number: new_at_relay_block.0,
					head_hash: new_head_hash,
				},
				next_imported_hash_position: (next_imported_hash_position + 1)
					% T::HeadsToKeep::get(),
			};
			ImportedParaHashes::<T, I>::insert(
				teyrchain,
				next_imported_hash_position,
				new_head_hash,
			);
			ImportedParaHeads::<T, I>::insert(teyrchain, new_head_hash, &updated_head_data);
			tracing::trace!(
				target: LOG_TARGET,
				?teyrchain,
				%new_head_hash,
				at_relay_block=%new_at_relay_block.0,
				"Updated head of teyrchain"
			);

			// trigger callback
			T::OnNewHead::on_new_head(teyrchain, &new_head);

			// remove old head
			let prune_happened = head_hash_to_prune.is_ok();
			if let Ok(head_hash_to_prune) = head_hash_to_prune {
				tracing::trace!(
					target: LOG_TARGET,
					?teyrchain,
					%head_hash_to_prune,
					"Pruning old head of teyrchain"
				);
				ImportedParaHeads::<T, I>::remove(teyrchain, head_hash_to_prune);
			}
			Self::deposit_event(Event::UpdatedTeyrchainHead {
				teyrchain,
				teyrchain_head_hash: new_head_hash,
			});

			Ok(UpdateTeyrchainHeadArtifacts { best_head: updated_best_para_head, prune_happened })
		}
	}

	#[pezpallet::genesis_config]
	#[derive(DefaultNoBound)]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		/// Initial pezpallet operating mode.
		pub operating_mode: BasicOperatingMode,
		/// Initial pezpallet owner.
		pub owner: Option<T::AccountId>,
		/// Dummy marker.
		#[serde(skip)]
		pub _phantom: pezsp_std::marker::PhantomData<I>,
	}

	#[pezpallet::genesis_build]
	impl<T: Config<I>, I: 'static> BuildGenesisConfig for GenesisConfig<T, I> {
		fn build(&self) {
			PalletOperatingMode::<T, I>::put(self.operating_mode);
			if let Some(ref owner) = self.owner {
				PalletOwner::<T, I>::put(owner);
			}
		}
	}

	/// Returns maximal number of teyrchains, supported by the pezpallet.
	pub struct MaybeMaxTeyrchains<T, I>(PhantomData<(T, I)>);

	impl<T: Config<I>, I: 'static> Get<Option<u32>> for MaybeMaxTeyrchains<T, I> {
		fn get() -> Option<u32> {
			Some(T::ParaStoredHeaderDataBuilder::supported_teyrchains())
		}
	}

	/// Returns total number of all teyrchains hashes/heads, stored by the pezpallet.
	pub struct MaybeMaxTotalTeyrchainHashes<T, I>(PhantomData<(T, I)>);

	impl<T: Config<I>, I: 'static> Get<Option<u32>> for MaybeMaxTotalTeyrchainHashes<T, I> {
		fn get() -> Option<u32> {
			Some(
				T::ParaStoredHeaderDataBuilder::supported_teyrchains()
					.saturating_mul(T::HeadsToKeep::get()),
			)
		}
	}
}

/// Single teyrchain header chain adapter.
pub struct TeyrchainHeaders<T, I, C>(PhantomData<(T, I, C)>);

impl<T: Config<I>, I: 'static, C: Teyrchain<Hash = ParaHash>> HeaderChain<C>
	for TeyrchainHeaders<T, I, C>
{
	fn finalized_header_state_root(hash: HashOf<C>) -> Option<HashOf<C>> {
		Pezpallet::<T, I>::teyrchain_head(ParaId(C::TEYRCHAIN_ID), hash)
			.and_then(|head| head.decode_teyrchain_head_data::<C>().ok())
			.map(|h| h.state_root)
	}
}

/// (Re)initialize pezpallet with given header for using it in `pezpallet-bridge-messages`
/// benchmarks.
#[cfg(feature = "runtime-benchmarks")]
pub fn initialize_for_benchmarks<T: Config<I>, I: 'static, PC: Teyrchain<Hash = ParaHash>>(
	header: HeaderOf<PC>,
) {
	use pezbp_pezkuwi_core::teyrchains::ParaHead;
	use pezbp_runtime::HeaderIdProvider;
	use pezsp_runtime::traits::Header;

	let relay_head =
		pezpallet_bridge_grandpa::BridgedHeader::<T, T::BridgesGrandpaPalletInstance>::new(
			0,
			Default::default(),
			Default::default(),
			Default::default(),
			Default::default(),
		);
	let teyrchain = ParaId(PC::TEYRCHAIN_ID);
	let teyrchain_head = ParaHead(header.encode());
	let updated_head_data = T::ParaStoredHeaderDataBuilder::try_build(teyrchain, &teyrchain_head)
		.expect("failed to build stored teyrchain head in benchmarks");
	pezpallet_bridge_grandpa::initialize_for_benchmarks::<T, T::BridgesGrandpaPalletInstance>(
		relay_head.clone(),
	);
	Pezpallet::<T, I>::update_teyrchain_head(
		teyrchain,
		None,
		relay_head.id(),
		updated_head_data,
		teyrchain_head.hash(),
		teyrchain_head,
	)
	.expect("failed to insert teyrchain head in benchmarks");
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use crate::mock::{
		run_test, test_relay_header, BigTeyrchain, BigTeyrchainHeader, FreeHeadersInterval,
		RegularTeyrchainHasher, RegularTeyrchainHeader, RelayBlockHeader,
		RuntimeEvent as TestEvent, RuntimeOrigin, TestRuntime, UNTRACKED_TEYRCHAIN_ID,
	};
	use codec::Encode;
	use pezbp_test_utils::prepare_teyrchain_heads_proof;

	use pezbp_header_pez_chain::{justification::GrandpaJustification, StoredHeaderGrandpaInfo};
	use pezbp_pezkuwi_core::teyrchains::ParaHead;
	use pezbp_runtime::{
		BasicOperatingMode, OwnedBridgeModuleError, StorageDoubleMapKeyProvider,
		StorageMapKeyProvider, StorageProofError,
	};
	use pezbp_test_utils::{
		authority_list, generate_owned_bridge_module_tests, make_default_justification,
		TEST_GRANDPA_SET_ID,
	};
	use pezbp_teyrchains::{
		BestParaHeadHash, BridgeTeyrchainCall, ImportedParaHeadsKeyProvider, ParasInfoKeyProvider,
	};
	use pezframe_support::{
		assert_noop, assert_ok,
		dispatch::DispatchResultWithPostInfo,
		pezpallet_prelude::Pays,
		storage::generator::{StorageDoubleMap, StorageMap},
		traits::Get,
		weights::Weight,
	};
	use pezframe_system::{EventRecord, Pezpallet as System, Phase};
	use pezsp_core::Hasher;
	use pezsp_runtime::{traits::Header as HeaderT, DispatchError};

	type BridgesGrandpaPalletInstance = pezpallet_bridge_grandpa::Instance1;
	type WeightInfo = <TestRuntime as Config>::WeightInfo;
	type DbWeight = <TestRuntime as pezframe_system::Config>::DbWeight;

	pub(crate) fn initialize(state_root: RelayBlockHash) -> RelayBlockHash {
		pezpallet_bridge_grandpa::FreeHeadersRemaining::<TestRuntime, BridgesGrandpaPalletInstance>::set(Some(100));
		pezpallet_bridge_grandpa::Pezpallet::<TestRuntime, BridgesGrandpaPalletInstance>::initialize(
			RuntimeOrigin::root(),
			pezbp_header_pez_chain::InitializationData {
				header: Box::new(test_relay_header(0, state_root)),
				authority_list: authority_list(),
				set_id: 1,
				operating_mode: BasicOperatingMode::Normal,
			},
		)
		.unwrap();

		System::<TestRuntime>::set_block_number(1);
		System::<TestRuntime>::reset_events();

		test_relay_header(0, state_root).hash()
	}

	fn proceed(
		num: RelayBlockNumber,
		state_root: RelayBlockHash,
	) -> (ParaHash, GrandpaJustification<RelayBlockHeader>) {
		let header = test_relay_header(num, state_root);
		let hash = header.hash();
		let justification = make_default_justification(&header);
		assert_ok!(
			pezpallet_bridge_grandpa::Pezpallet::<TestRuntime, BridgesGrandpaPalletInstance>::submit_finality_proof_ex(
				RuntimeOrigin::signed(1),
				Box::new(header),
				justification.clone(),
				TEST_GRANDPA_SET_ID,
				false,
			)
		);

		(hash, justification)
	}

	fn initial_best_head(teyrchain: u32) -> ParaInfo {
		ParaInfo {
			best_head_hash: BestParaHeadHash {
				at_relay_block_number: 0,
				head_hash: head_data(teyrchain, 0).hash(),
			},
			next_imported_hash_position: 1,
		}
	}

	pub(crate) fn head_data(teyrchain: u32, head_number: u32) -> ParaHead {
		ParaHead(
			RegularTeyrchainHeader::new(
				head_number as _,
				Default::default(),
				RegularTeyrchainHasher::hash(&(teyrchain, head_number).encode()),
				Default::default(),
				Default::default(),
			)
			.encode(),
		)
	}

	fn stored_head_data(teyrchain: u32, head_number: u32) -> ParaStoredHeaderData {
		ParaStoredHeaderData(
			(head_number as u64, RegularTeyrchainHasher::hash(&(teyrchain, head_number).encode()))
				.encode(),
		)
	}

	fn big_head_data(teyrchain: u32, head_number: u32) -> ParaHead {
		ParaHead(
			BigTeyrchainHeader::new(
				head_number as _,
				Default::default(),
				RegularTeyrchainHasher::hash(&(teyrchain, head_number).encode()),
				Default::default(),
				Default::default(),
			)
			.encode(),
		)
	}

	fn big_stored_head_data(teyrchain: u32, head_number: u32) -> ParaStoredHeaderData {
		ParaStoredHeaderData(
			(head_number as u128, RegularTeyrchainHasher::hash(&(teyrchain, head_number).encode()))
				.encode(),
		)
	}

	fn head_hash(teyrchain: u32, head_number: u32) -> ParaHash {
		head_data(teyrchain, head_number).hash()
	}

	fn import_teyrchain_1_head(
		relay_chain_block: RelayBlockNumber,
		relay_state_root: RelayBlockHash,
		teyrchains: Vec<(ParaId, ParaHash)>,
		proof: ParaHeadsProof,
	) -> DispatchResultWithPostInfo {
		Pezpallet::<TestRuntime>::submit_teyrchain_heads(
			RuntimeOrigin::signed(1),
			(relay_chain_block, test_relay_header(relay_chain_block, relay_state_root).hash()),
			teyrchains,
			proof,
		)
	}

	fn weight_of_import_teyrchain_1_head(proof: &ParaHeadsProof, prune_expected: bool) -> Weight {
		let db_weight = <TestRuntime as pezframe_system::Config>::DbWeight::get();
		WeightInfoOf::<TestRuntime, ()>::submit_teyrchain_heads_weight(db_weight, proof, 1)
			.saturating_sub(if prune_expected {
				Weight::zero()
			} else {
				WeightInfoOf::<TestRuntime, ()>::teyrchain_head_pruning_weight(db_weight)
			})
	}

	#[test]
	fn submit_teyrchain_heads_checks_operating_mode() {
		let (state_root, proof, teyrchains) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 0))]);

		run_test(|| {
			initialize(state_root);

			// `submit_teyrchain_heads()` should fail when the pezpallet is halted.
			PalletOperatingMode::<TestRuntime>::put(BasicOperatingMode::Halted);
			assert_noop!(
				Pezpallet::<TestRuntime>::submit_teyrchain_heads(
					RuntimeOrigin::signed(1),
					(0, test_relay_header(0, state_root).hash()),
					teyrchains.clone(),
					proof.clone(),
				),
				Error::<TestRuntime>::BridgeModule(OwnedBridgeModuleError::Halted)
			);

			// `submit_teyrchain_heads()` should succeed now that the pezpallet is resumed.
			PalletOperatingMode::<TestRuntime>::put(BasicOperatingMode::Normal);
			assert_ok!(Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(0, test_relay_header(0, state_root).hash()),
				teyrchains,
				proof,
			),);
		});
	}

	#[test]
	fn imports_initial_teyrchain_heads() {
		let (state_root, proof, teyrchains) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![
				(1, head_data(1, 0)),
				(3, head_data(3, 10)),
			]);
		run_test(|| {
			initialize(state_root);

			// we're trying to update heads of teyrchains 1 and 3
			let expected_weight =
				WeightInfo::submit_teyrchain_heads_weight(DbWeight::get(), &proof, 2);
			let result = Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(0, test_relay_header(0, state_root).hash()),
				teyrchains,
				proof,
			);
			assert_ok!(result);
			assert_eq!(result.expect("checked above").pays_fee, Pays::Yes);
			assert_eq!(result.expect("checked above").actual_weight, Some(expected_weight));

			// 1 and 3 are updated, because proof is missing head of teyrchain#2
			assert_eq!(ParasInfo::<TestRuntime>::get(ParaId(1)), Some(initial_best_head(1)));
			assert_eq!(ParasInfo::<TestRuntime>::get(ParaId(2)), None);
			assert_eq!(
				ParasInfo::<TestRuntime>::get(ParaId(3)),
				Some(ParaInfo {
					best_head_hash: BestParaHeadHash {
						at_relay_block_number: 0,
						head_hash: head_data(3, 10).hash()
					},
					next_imported_hash_position: 1,
				})
			);

			assert_eq!(
				ImportedParaHeads::<TestRuntime>::get(
					ParaId(1),
					initial_best_head(1).best_head_hash.head_hash
				)
				.map(|h| h.into_inner()),
				Some(stored_head_data(1, 0))
			);
			assert_eq!(
				ImportedParaHeads::<TestRuntime>::get(
					ParaId(2),
					initial_best_head(2).best_head_hash.head_hash
				)
				.map(|h| h.into_inner()),
				None
			);
			assert_eq!(
				ImportedParaHeads::<TestRuntime>::get(ParaId(3), head_hash(3, 10))
					.map(|h| h.into_inner()),
				Some(stored_head_data(3, 10))
			);

			assert_eq!(
				System::<TestRuntime>::events(),
				vec![
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: initial_best_head(1).best_head_hash.head_hash,
						}),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(3),
							teyrchain_head_hash: head_data(3, 10).hash(),
						}),
						topics: vec![],
					}
				],
			);
		});
	}

	#[test]
	fn imports_teyrchain_heads_is_able_to_progress() {
		let (state_root_5, proof_5, teyrchains_5) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 5))]);
		let (state_root_10, proof_10, teyrchains_10) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 10))]);
		run_test(|| {
			// start with relay block #0 and import head#5 of teyrchain#1
			initialize(state_root_5);
			let result = import_teyrchain_1_head(0, state_root_5, teyrchains_5, proof_5);
			// first teyrchain head is imported for free
			assert_eq!(result.unwrap().pays_fee, Pays::No);
			assert_eq!(
				ParasInfo::<TestRuntime>::get(ParaId(1)),
				Some(ParaInfo {
					best_head_hash: BestParaHeadHash {
						at_relay_block_number: 0,
						head_hash: head_data(1, 5).hash()
					},
					next_imported_hash_position: 1,
				})
			);
			assert_eq!(
				ImportedParaHeads::<TestRuntime>::get(ParaId(1), head_data(1, 5).hash())
					.map(|h| h.into_inner()),
				Some(stored_head_data(1, 5))
			);
			assert_eq!(
				ImportedParaHeads::<TestRuntime>::get(ParaId(1), head_data(1, 10).hash())
					.map(|h| h.into_inner()),
				None
			);
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![EventRecord {
					phase: Phase::Initialization,
					event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
						teyrchain: ParaId(1),
						teyrchain_head_hash: head_data(1, 5).hash(),
					}),
					topics: vec![],
				}],
			);

			// import head#10 of teyrchain#1 at relay block #1
			let (relay_1_hash, justification) = proceed(1, state_root_10);
			let result = import_teyrchain_1_head(1, state_root_10, teyrchains_10, proof_10);
			// second teyrchain head is imported for fee
			assert_eq!(result.unwrap().pays_fee, Pays::Yes);
			assert_eq!(
				ParasInfo::<TestRuntime>::get(ParaId(1)),
				Some(ParaInfo {
					best_head_hash: BestParaHeadHash {
						at_relay_block_number: 1,
						head_hash: head_data(1, 10).hash()
					},
					next_imported_hash_position: 2,
				})
			);
			assert_eq!(
				ImportedParaHeads::<TestRuntime>::get(ParaId(1), head_data(1, 5).hash())
					.map(|h| h.into_inner()),
				Some(stored_head_data(1, 5))
			);
			assert_eq!(
				ImportedParaHeads::<TestRuntime>::get(ParaId(1), head_data(1, 10).hash())
					.map(|h| h.into_inner()),
				Some(stored_head_data(1, 10))
			);
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: head_data(1, 5).hash(),
						}),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Grandpa1(
							pezpallet_bridge_grandpa::Event::UpdatedBestFinalizedHeader {
								number: 1,
								hash: relay_1_hash,
								grandpa_info: StoredHeaderGrandpaInfo {
									finality_proof: justification,
									new_verification_context: None,
								},
							}
						),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: head_data(1, 10).hash(),
						}),
						topics: vec![],
					}
				],
			);
		});
	}

	#[test]
	fn ignores_untracked_teyrchain() {
		let (state_root, proof, teyrchains) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![
				(1, head_data(1, 5)),
				(UNTRACKED_TEYRCHAIN_ID, head_data(1, 5)),
				(2, head_data(1, 5)),
			]);
		run_test(|| {
			// start with relay block #0 and try to import head#5 of teyrchain#1 and untracked
			// teyrchain
			let expected_weight =
				WeightInfo::submit_teyrchain_heads_weight(DbWeight::get(), &proof, 3)
					.saturating_sub(WeightInfo::teyrchain_head_storage_write_weight(
						DbWeight::get(),
					));
			initialize(state_root);
			let result = Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(0, test_relay_header(0, state_root).hash()),
				teyrchains,
				proof,
			);
			assert_ok!(result);
			assert_eq!(result.expect("checked above").actual_weight, Some(expected_weight));
			assert_eq!(
				ParasInfo::<TestRuntime>::get(ParaId(1)),
				Some(ParaInfo {
					best_head_hash: BestParaHeadHash {
						at_relay_block_number: 0,
						head_hash: head_data(1, 5).hash()
					},
					next_imported_hash_position: 1,
				})
			);
			assert_eq!(ParasInfo::<TestRuntime>::get(ParaId(UNTRACKED_TEYRCHAIN_ID)), None,);
			assert_eq!(
				ParasInfo::<TestRuntime>::get(ParaId(2)),
				Some(ParaInfo {
					best_head_hash: BestParaHeadHash {
						at_relay_block_number: 0,
						head_hash: head_data(1, 5).hash()
					},
					next_imported_hash_position: 1,
				})
			);
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: head_data(1, 5).hash(),
						}),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UntrackedTeyrchainRejected {
							teyrchain: ParaId(UNTRACKED_TEYRCHAIN_ID),
						}),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(2),
							teyrchain_head_hash: head_data(1, 5).hash(),
						}),
						topics: vec![],
					}
				],
			);
		});
	}

	#[test]
	fn does_nothing_when_already_imported_this_head_at_previous_relay_header() {
		let (state_root, proof, teyrchains) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 0))]);
		run_test(|| {
			// import head#0 of teyrchain#1 at relay block#0
			initialize(state_root);
			assert_ok!(import_teyrchain_1_head(0, state_root, teyrchains.clone(), proof.clone()));
			assert_eq!(ParasInfo::<TestRuntime>::get(ParaId(1)), Some(initial_best_head(1)));
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![EventRecord {
					phase: Phase::Initialization,
					event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
						teyrchain: ParaId(1),
						teyrchain_head_hash: initial_best_head(1).best_head_hash.head_hash,
					}),
					topics: vec![],
				}],
			);

			// try to import head#0 of teyrchain#1 at relay block#1
			// => call succeeds, but nothing is changed
			let (relay_1_hash, justification) = proceed(1, state_root);
			assert_ok!(import_teyrchain_1_head(1, state_root, teyrchains, proof));
			assert_eq!(ParasInfo::<TestRuntime>::get(ParaId(1)), Some(initial_best_head(1)));
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: initial_best_head(1).best_head_hash.head_hash,
						}),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Grandpa1(
							pezpallet_bridge_grandpa::Event::UpdatedBestFinalizedHeader {
								number: 1,
								hash: relay_1_hash,
								grandpa_info: StoredHeaderGrandpaInfo {
									finality_proof: justification,
									new_verification_context: None,
								}
							}
						),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::RejectedObsoleteTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: initial_best_head(1).best_head_hash.head_hash,
						}),
						topics: vec![],
					}
				],
			);
		});
	}

	#[test]
	fn does_nothing_when_already_imported_head_at_better_relay_header() {
		let (state_root_5, proof_5, teyrchains_5) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 5))]);
		let (state_root_10, proof_10, teyrchains_10) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 10))]);
		run_test(|| {
			// start with relay block #0
			initialize(state_root_5);

			// head#10 of teyrchain#1 at relay block#1
			let (relay_1_hash, justification) = proceed(1, state_root_10);
			assert_ok!(import_teyrchain_1_head(1, state_root_10, teyrchains_10, proof_10));
			assert_eq!(
				ParasInfo::<TestRuntime>::get(ParaId(1)),
				Some(ParaInfo {
					best_head_hash: BestParaHeadHash {
						at_relay_block_number: 1,
						head_hash: head_data(1, 10).hash()
					},
					next_imported_hash_position: 1,
				})
			);
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Grandpa1(
							pezpallet_bridge_grandpa::Event::UpdatedBestFinalizedHeader {
								number: 1,
								hash: relay_1_hash,
								grandpa_info: StoredHeaderGrandpaInfo {
									finality_proof: justification.clone(),
									new_verification_context: None,
								}
							}
						),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: head_data(1, 10).hash(),
						}),
						topics: vec![],
					}
				],
			);

			// now try to import head#5 at relay block#0
			// => nothing is changed, because better head has already been imported
			assert_ok!(import_teyrchain_1_head(0, state_root_5, teyrchains_5, proof_5));
			assert_eq!(
				ParasInfo::<TestRuntime>::get(ParaId(1)),
				Some(ParaInfo {
					best_head_hash: BestParaHeadHash {
						at_relay_block_number: 1,
						head_hash: head_data(1, 10).hash()
					},
					next_imported_hash_position: 1,
				})
			);
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Grandpa1(
							pezpallet_bridge_grandpa::Event::UpdatedBestFinalizedHeader {
								number: 1,
								hash: relay_1_hash,
								grandpa_info: StoredHeaderGrandpaInfo {
									finality_proof: justification,
									new_verification_context: None,
								}
							}
						),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: head_data(1, 10).hash(),
						}),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::RejectedObsoleteTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: head_data(1, 5).hash(),
						}),
						topics: vec![],
					}
				],
			);
		});
	}

	#[test]
	fn does_nothing_when_teyrchain_head_is_too_large() {
		let (state_root, proof, teyrchains) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![
				(1, head_data(1, 5)),
				(4, big_head_data(1, 5)),
			]);
		run_test(|| {
			// start with relay block #0 and try to import head#5 of teyrchain#1 and big teyrchain
			initialize(state_root);
			let result = Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(0, test_relay_header(0, state_root).hash()),
				teyrchains,
				proof,
			);
			assert_ok!(result);
			assert_eq!(
				ParasInfo::<TestRuntime>::get(ParaId(1)),
				Some(ParaInfo {
					best_head_hash: BestParaHeadHash {
						at_relay_block_number: 0,
						head_hash: head_data(1, 5).hash()
					},
					next_imported_hash_position: 1,
				})
			);
			assert_eq!(ParasInfo::<TestRuntime>::get(ParaId(4)), None);
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::UpdatedTeyrchainHead {
							teyrchain: ParaId(1),
							teyrchain_head_hash: head_data(1, 5).hash(),
						}),
						topics: vec![],
					},
					EventRecord {
						phase: Phase::Initialization,
						event: TestEvent::Teyrchains(Event::RejectedLargeTeyrchainHead {
							teyrchain: ParaId(4),
							teyrchain_head_hash: big_head_data(1, 5).hash(),
							teyrchain_head_size: big_stored_head_data(1, 5).encoded_size() as u32,
						}),
						topics: vec![],
					},
				],
			);
		});
	}

	#[test]
	fn prunes_old_heads() {
		run_test(|| {
			let heads_to_keep = crate::mock::HeadsToKeep::get();

			// import exactly `HeadsToKeep` headers
			for i in 0..heads_to_keep {
				let (state_root, proof, teyrchains) = prepare_teyrchain_heads_proof::<
					RegularTeyrchainHeader,
				>(vec![(1, head_data(1, i))]);
				if i == 0 {
					initialize(state_root);
				} else {
					proceed(i, state_root);
				}

				let expected_weight = weight_of_import_teyrchain_1_head(&proof, false);
				let result = import_teyrchain_1_head(i, state_root, teyrchains, proof);
				assert_ok!(result);
				assert_eq!(result.expect("checked above").actual_weight, Some(expected_weight));
			}

			// nothing is pruned yet
			for i in 0..heads_to_keep {
				assert!(ImportedParaHeads::<TestRuntime>::get(ParaId(1), head_data(1, i).hash())
					.is_some());
			}

			// import next relay chain header and next teyrchain head
			let (state_root, proof, teyrchains) = prepare_teyrchain_heads_proof::<
				RegularTeyrchainHeader,
			>(vec![(1, head_data(1, heads_to_keep))]);
			proceed(heads_to_keep, state_root);
			let expected_weight = weight_of_import_teyrchain_1_head(&proof, true);
			let result = import_teyrchain_1_head(heads_to_keep, state_root, teyrchains, proof);
			assert_ok!(result);
			assert_eq!(result.expect("checked above").actual_weight, Some(expected_weight));

			// and the head#0 is pruned
			assert!(
				ImportedParaHeads::<TestRuntime>::get(ParaId(1), head_data(1, 0).hash()).is_none()
			);
			for i in 1..=heads_to_keep {
				assert!(ImportedParaHeads::<TestRuntime>::get(ParaId(1), head_data(1, i).hash())
					.is_some());
			}
		});
	}

	#[test]
	fn fails_on_unknown_relay_chain_block() {
		let (state_root, proof, teyrchains) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 5))]);
		run_test(|| {
			// start with relay block #0
			initialize(state_root);

			// try to import head#5 of teyrchain#1 at unknown relay chain block #1
			assert_noop!(
				import_teyrchain_1_head(1, state_root, teyrchains, proof),
				Error::<TestRuntime>::UnknownRelayChainBlock
			);
		});
	}

	#[test]
	fn fails_on_invalid_storage_proof() {
		let (_state_root, proof, teyrchains) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 5))]);
		run_test(|| {
			// start with relay block #0
			initialize(Default::default());

			// try to import head#5 of teyrchain#1 at relay chain block #0
			assert_noop!(
				import_teyrchain_1_head(0, Default::default(), teyrchains, proof),
				Error::<TestRuntime>::HeaderChainStorageProof(HeaderChainError::StorageProof(
					StorageProofError::StorageRootMismatch
				))
			);
		});
	}

	#[test]
	fn is_not_rewriting_existing_head_if_failed_to_read_updated_head() {
		let (state_root_5, proof_5, teyrchains_5) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 5))]);
		let (state_root_10_at_20, proof_10_at_20, teyrchains_10_at_20) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(2, head_data(2, 10))]);
		let (state_root_10_at_30, proof_10_at_30, teyrchains_10_at_30) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 10))]);
		run_test(|| {
			// we've already imported head#5 of teyrchain#1 at relay block#10
			initialize(state_root_5);
			import_teyrchain_1_head(0, state_root_5, teyrchains_5, proof_5).expect("ok");
			assert_eq!(
				Pezpallet::<TestRuntime>::best_teyrchain_head(ParaId(1)),
				Some(stored_head_data(1, 5))
			);

			// then if someone is pretending to provide updated head#10 of teyrchain#1 at relay
			// block#20, but fails to do that
			//
			// => we'll leave previous value
			proceed(20, state_root_10_at_20);
			assert_ok!(Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(20, test_relay_header(20, state_root_10_at_20).hash()),
				teyrchains_10_at_20,
				proof_10_at_20,
			),);
			assert_eq!(
				Pezpallet::<TestRuntime>::best_teyrchain_head(ParaId(1)),
				Some(stored_head_data(1, 5))
			);

			// then if someone is pretending to provide updated head#10 of teyrchain#1 at relay
			// block#30, and actually provides it
			//
			// => we'll update value
			proceed(30, state_root_10_at_30);
			assert_ok!(Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(30, test_relay_header(30, state_root_10_at_30).hash()),
				teyrchains_10_at_30,
				proof_10_at_30,
			),);
			assert_eq!(
				Pezpallet::<TestRuntime>::best_teyrchain_head(ParaId(1)),
				Some(stored_head_data(1, 10))
			);
		});
	}

	#[test]
	fn storage_keys_computed_properly() {
		assert_eq!(
			ParasInfo::<TestRuntime>::storage_map_final_key(ParaId(42)).to_vec(),
			ParasInfoKeyProvider::final_key("Teyrchains", &ParaId(42)).0
		);

		assert_eq!(
			ImportedParaHeads::<TestRuntime>::storage_double_map_final_key(
				ParaId(42),
				ParaHash::from([21u8; 32])
			)
			.to_vec(),
			ImportedParaHeadsKeyProvider::final_key(
				"Teyrchains",
				&ParaId(42),
				&ParaHash::from([21u8; 32])
			)
			.0,
		);
	}

	#[test]
	fn ignores_teyrchain_head_if_it_is_missing_from_storage_proof() {
		let (state_root, proof, _) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![]);
		let teyrchains = vec![(ParaId(2), Default::default())];
		run_test(|| {
			initialize(state_root);
			assert_ok!(Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(0, test_relay_header(0, state_root).hash()),
				teyrchains,
				proof,
			));
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![EventRecord {
					phase: Phase::Initialization,
					event: TestEvent::Teyrchains(Event::MissingTeyrchainHead {
						teyrchain: ParaId(2),
					}),
					topics: vec![],
				}],
			);
		});
	}

	#[test]
	fn ignores_teyrchain_head_if_teyrchain_head_hash_is_wrong() {
		let (state_root, proof, _) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 0))]);
		let teyrchains = vec![(ParaId(1), head_data(1, 10).hash())];
		run_test(|| {
			initialize(state_root);
			assert_ok!(Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(0, test_relay_header(0, state_root).hash()),
				teyrchains,
				proof,
			));
			assert_eq!(
				System::<TestRuntime>::events(),
				vec![EventRecord {
					phase: Phase::Initialization,
					event: TestEvent::Teyrchains(Event::IncorrectTeyrchainHeadHash {
						teyrchain: ParaId(1),
						teyrchain_head_hash: head_data(1, 10).hash(),
						actual_teyrchain_head_hash: head_data(1, 0).hash(),
					}),
					topics: vec![],
				}],
			);
		});
	}

	#[test]
	fn test_bridge_teyrchain_call_is_correctly_defined() {
		let (state_root, proof, _) =
			prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 0))]);
		let teyrchains = vec![(ParaId(2), Default::default())];
		let relay_header_id = (0, test_relay_header(0, state_root).hash());

		let direct_submit_teyrchain_heads_call = Call::<TestRuntime>::submit_teyrchain_heads {
			at_relay_block: relay_header_id,
			teyrchains: teyrchains.clone(),
			teyrchain_heads_proof: proof.clone(),
		};
		let indirect_submit_teyrchain_heads_call = BridgeTeyrchainCall::submit_teyrchain_heads {
			at_relay_block: relay_header_id,
			teyrchains,
			teyrchain_heads_proof: proof,
		};
		assert_eq!(
			direct_submit_teyrchain_heads_call.encode(),
			indirect_submit_teyrchain_heads_call.encode()
		);
	}

	generate_owned_bridge_module_tests!(BasicOperatingMode::Normal, BasicOperatingMode::Halted);

	#[test]
	fn maybe_max_teyrchains_returns_correct_value() {
		assert_eq!(MaybeMaxTeyrchains::<TestRuntime, ()>::get(), Some(mock::TOTAL_TEYRCHAINS));
	}

	#[test]
	fn maybe_max_total_teyrchain_hashes_returns_correct_value() {
		assert_eq!(
			MaybeMaxTotalTeyrchainHashes::<TestRuntime, ()>::get(),
			Some(mock::TOTAL_TEYRCHAINS * mock::HeadsToKeep::get()),
		);
	}

	#[test]
	fn submit_finality_proof_requires_signed_origin() {
		run_test(|| {
			let (state_root, proof, teyrchains) =
				prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(1, head_data(1, 0))]);

			initialize(state_root);

			// `submit_teyrchain_heads()` should fail when the pezpallet is halted.
			assert_noop!(
				Pezpallet::<TestRuntime>::submit_teyrchain_heads(
					RuntimeOrigin::root(),
					(0, test_relay_header(0, state_root).hash()),
					teyrchains,
					proof,
				),
				DispatchError::BadOrigin
			);
		})
	}

	#[test]
	fn may_be_free_for_submitting_filtered_heads() {
		run_test(|| {
			let (state_root, proof, teyrchains) =
				prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(2, head_data(2, 5))]);
			// start with relay block #0 and import head#5 of teyrchain#2
			initialize(state_root);
			// first submission is free
			let result = Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(0, test_relay_header(0, state_root).hash()),
				teyrchains.clone(),
				proof.clone(),
			);
			assert_eq!(result.unwrap().pays_fee, Pays::No);
			// next submission is NOT free, because we haven't updated anything
			let result = Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(0, test_relay_header(0, state_root).hash()),
				teyrchains,
				proof,
			);
			assert_eq!(result.unwrap().pays_fee, Pays::Yes);
			// then we submit new head, proved at relay block `FreeHeadersInterval - 1` => Pays::Yes
			let (state_root, proof, teyrchains) = prepare_teyrchain_heads_proof::<
				RegularTeyrchainHeader,
			>(vec![(2, head_data(2, 50))]);
			let relay_block_number = FreeHeadersInterval::get() - 1;
			proceed(relay_block_number, state_root);
			let result = Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(relay_block_number, test_relay_header(relay_block_number, state_root).hash()),
				teyrchains,
				proof,
			);
			assert_eq!(result.unwrap().pays_fee, Pays::Yes);
			// then we submit new head, proved after `FreeHeadersInterval` => Pays::No
			let (state_root, proof, teyrchains) = prepare_teyrchain_heads_proof::<
				RegularTeyrchainHeader,
			>(vec![(2, head_data(2, 100))]);
			let relay_block_number = relay_block_number + FreeHeadersInterval::get();
			proceed(relay_block_number, state_root);
			let result = Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(relay_block_number, test_relay_header(relay_block_number, state_root).hash()),
				teyrchains,
				proof,
			);
			assert_eq!(result.unwrap().pays_fee, Pays::No);
			// then we submit new BIG head, proved after `FreeHeadersInterval` => Pays::Yes
			// then we submit new head, proved after `FreeHeadersInterval` => Pays::No
			let mut large_head = head_data(2, 100);
			large_head.0.extend(&[42u8; BigTeyrchain::MAX_HEADER_SIZE as _]);
			let (state_root, proof, teyrchains) =
				prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(2, large_head)]);
			let relay_block_number = relay_block_number + FreeHeadersInterval::get();
			proceed(relay_block_number, state_root);
			let result = Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(relay_block_number, test_relay_header(relay_block_number, state_root).hash()),
				teyrchains,
				proof,
			);
			assert_eq!(result.unwrap().pays_fee, Pays::Yes);
		})
	}

	#[test]
	fn grandpa_and_teyrchain_pallets_share_free_headers_counter() {
		run_test(|| {
			initialize(Default::default());
			// set free headers limit to `4`
			let mut free_headers_remaining = 4;
			pezpallet_bridge_grandpa::FreeHeadersRemaining::<
				TestRuntime,
				BridgesGrandpaPalletInstance,
			>::set(Some(free_headers_remaining));
			// import free GRANDPA and teyrchain headers
			let mut relay_block_number = 0;
			for i in 0..2 {
				// import free GRANDPA header
				let (state_root, proof, teyrchains) = prepare_teyrchain_heads_proof::<
					RegularTeyrchainHeader,
				>(vec![(2, head_data(2, 5 + i))]);
				relay_block_number = relay_block_number + FreeHeadersInterval::get();
				proceed(relay_block_number, state_root);
				assert_eq!(
					pezpallet_bridge_grandpa::FreeHeadersRemaining::<
						TestRuntime,
						BridgesGrandpaPalletInstance,
					>::get(),
					Some(free_headers_remaining - 1),
				);
				free_headers_remaining = free_headers_remaining - 1;
				// import free teyrchain header
				assert_ok!(Pezpallet::<TestRuntime>::submit_teyrchain_heads(
					RuntimeOrigin::signed(1),
					(relay_block_number, test_relay_header(relay_block_number, state_root).hash()),
					teyrchains,
					proof,
				),);
				assert_eq!(
					pezpallet_bridge_grandpa::FreeHeadersRemaining::<
						TestRuntime,
						BridgesGrandpaPalletInstance,
					>::get(),
					Some(free_headers_remaining - 1),
				);
				free_headers_remaining = free_headers_remaining - 1;
			}
			// try to import free GRANDPA header => non-free execution
			let (state_root, proof, teyrchains) =
				prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(vec![(2, head_data(2, 7))]);
			relay_block_number = relay_block_number + FreeHeadersInterval::get();
			let result = pezpallet_bridge_grandpa::Pezpallet::<
				TestRuntime,
				BridgesGrandpaPalletInstance,
			>::submit_finality_proof_ex(
				RuntimeOrigin::signed(1),
				Box::new(test_relay_header(relay_block_number, state_root)),
				make_default_justification(&test_relay_header(relay_block_number, state_root)),
				TEST_GRANDPA_SET_ID,
				false,
			);
			assert_eq!(result.unwrap().pays_fee, Pays::Yes);
			// try to import free teyrchain header => non-free execution
			let result = Pezpallet::<TestRuntime>::submit_teyrchain_heads(
				RuntimeOrigin::signed(1),
				(relay_block_number, test_relay_header(relay_block_number, state_root).hash()),
				teyrchains,
				proof,
			);
			assert_eq!(result.unwrap().pays_fee, Pays::Yes);
			assert_eq!(
				pezpallet_bridge_grandpa::FreeHeadersRemaining::<
					TestRuntime,
					BridgesGrandpaPalletInstance,
				>::get(),
				Some(0),
			);
		});
	}
}
