// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

//! A BEEFY+MMR pezpallet combo.
//!
//! While both BEEFY and Merkle Mountain Range (MMR) can be used separately,
//! these tools were designed to work together in unison.
//!
//! The pezpallet provides a standardized MMR Leaf format that can be used
//! to bridge BEEFY+MMR-based networks (both standalone and Pezkuwi-like).
//!
//! The MMR leaf contains:
//! 1. Block number and parent block hash.
//! 2. Merkle Tree Root Hash of next BEEFY validator set.
//! 3. Arbitrary extra leaf data to be used by downstream pallets to include custom data.
//!
//! and thanks to versioning can be easily updated in the future.

extern crate alloc;

use pezsp_runtime::{
	generic::OpaqueDigestItemId,
	traits::{Convert, Header, Member},
	SaturatedConversion,
};

use alloc::vec::Vec;
use codec::Decode;
use pezpallet_mmr::{primitives::AncestryProof, LeafDataProvider, NodesUtils, ParentNumberAndHash};
use pezsp_consensus_beefy::{
	known_payloads,
	mmr::{BeefyAuthoritySet, BeefyDataProvider, BeefyNextAuthoritySet, MmrLeaf, MmrLeafVersion},
	AncestryHelper, AncestryHelperWeightInfo, Commitment, ConsensusLog,
	ValidatorSet as BeefyValidatorSet,
};

use pezframe_support::{crypto::ecdsa::ECDSAExt, pezpallet_prelude::Weight, traits::Get};
use pezframe_system::pezpallet_prelude::{BlockNumberFor, HeaderFor};

pub use pezpallet::*;
pub use weights::WeightInfo;

mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
mod weights;

/// A BEEFY consensus digest item with MMR root hash.
pub struct DepositBeefyDigest<T>(core::marker::PhantomData<T>);

impl<T> pezpallet_mmr::primitives::OnNewRoot<pezsp_consensus_beefy::MmrRootHash>
	for DepositBeefyDigest<T>
where
	T: pezpallet_mmr::Config<Hashing = pezsp_consensus_beefy::MmrHashing>,
	T: pezpallet_beefy::Config,
{
	fn on_new_root(root: &pezsp_consensus_beefy::MmrRootHash) {
		let digest = pezsp_runtime::generic::DigestItem::Consensus(
			pezsp_consensus_beefy::BEEFY_ENGINE_ID,
			codec::Encode::encode(&pezsp_consensus_beefy::ConsensusLog::<
				<T as pezpallet_beefy::Config>::BeefyId,
			>::MmrRoot(*root)),
		);
		pezframe_system::Pezpallet::<T>::deposit_log(digest);
	}
}

/// Convert BEEFY secp256k1 public keys into Ethereum addresses
pub struct BeefyEcdsaToEthereum;
impl Convert<pezsp_consensus_beefy::ecdsa_crypto::AuthorityId, Vec<u8>> for BeefyEcdsaToEthereum {
	fn convert(beefy_id: pezsp_consensus_beefy::ecdsa_crypto::AuthorityId) -> Vec<u8> {
		pezsp_core::ecdsa::Public::from(beefy_id)
			.to_eth_address()
			.map(|v| v.to_vec())
			.map_err(|_| {
				log::debug!(target: "runtime::beefy", "Failed to convert BEEFY PublicKey to ETH address!");
			})
			.unwrap_or_default()
	}
}

type MerkleRootOf<T> =
	<<T as pezpallet_mmr::Config>::Hashing as pezsp_runtime::traits::Hash>::Output;

#[pezframe_support::pezpallet]
pub mod pezpallet {
	#![allow(missing_docs)]

	use super::*;
	use pezframe_support::pezpallet_prelude::*;

	/// BEEFY-MMR pezpallet.
	#[pezpallet::pezpallet]
	pub struct Pezpallet<T>(_);

	/// The module's configuration trait.
	#[pezpallet::config]
	#[pezpallet::disable_pezframe_system_supertrait_check]
	pub trait Config: pezpallet_mmr::Config + pezpallet_beefy::Config {
		/// Current leaf version.
		///
		/// Specifies the version number added to every leaf that get's appended to the MMR.
		/// Read more in [`MmrLeafVersion`] docs about versioning leaves.
		type LeafVersion: Get<MmrLeafVersion>;

		/// Convert BEEFY AuthorityId to a form that would end up in the Merkle Tree.
		///
		/// For instance for ECDSA (secp256k1) we want to store uncompressed public keys (65 bytes)
		/// and later to Ethereum Addresses (160 bits) to simplify using them on Ethereum chain,
		/// but the rest of the Bizinikiwi codebase is storing them compressed (33 bytes) for
		/// efficiency reasons.
		type BeefyAuthorityToMerkleLeaf: Convert<
			<Self as pezpallet_beefy::Config>::BeefyId,
			Vec<u8>,
		>;

		/// The type expected for the leaf extra data
		type LeafExtra: Member + codec::FullCodec;

		/// Retrieve arbitrary data that should be added to the mmr leaf
		type BeefyDataProvider: BeefyDataProvider<Self::LeafExtra>;

		type WeightInfo: WeightInfo;
	}

	/// Details of current BEEFY authority set.
	#[pezpallet::storage]
	pub type BeefyAuthorities<T: Config> =
		StorageValue<_, BeefyAuthoritySet<MerkleRootOf<T>>, ValueQuery>;

	/// Details of next BEEFY authority set.
	///
	/// This storage entry is used as cache for calls to `update_beefy_next_authority_set`.
	#[pezpallet::storage]
	pub type BeefyNextAuthorities<T: Config> =
		StorageValue<_, BeefyNextAuthoritySet<MerkleRootOf<T>>, ValueQuery>;
}

impl<T: Config> LeafDataProvider for Pezpallet<T> {
	type LeafData = MmrLeaf<
		BlockNumberFor<T>,
		<T as pezframe_system::Config>::Hash,
		MerkleRootOf<T>,
		T::LeafExtra,
	>;

	fn leaf_data() -> Self::LeafData {
		MmrLeaf {
			version: T::LeafVersion::get(),
			parent_number_and_hash: ParentNumberAndHash::<T>::leaf_data(),
			leaf_extra: T::BeefyDataProvider::extra_data(),
			beefy_next_authority_set: BeefyNextAuthorities::<T>::get(),
		}
	}
}

impl<T> pezsp_consensus_beefy::OnNewValidatorSet<<T as pezpallet_beefy::Config>::BeefyId>
	for Pezpallet<T>
where
	T: pezpallet::Config,
{
	/// Compute and cache BEEFY authority sets based on updated BEEFY validator sets.
	fn on_new_validator_set(
		current_set: &BeefyValidatorSet<<T as pezpallet_beefy::Config>::BeefyId>,
		next_set: &BeefyValidatorSet<<T as pezpallet_beefy::Config>::BeefyId>,
	) {
		let current = Pezpallet::<T>::compute_authority_set(current_set);
		let next = Pezpallet::<T>::compute_authority_set(next_set);
		// cache the result
		BeefyAuthorities::<T>::put(&current);
		BeefyNextAuthorities::<T>::put(&next);
	}
}

impl<T: Config> AncestryHelper<HeaderFor<T>> for Pezpallet<T>
where
	T: pezpallet_mmr::Config<Hashing = pezsp_consensus_beefy::MmrHashing>,
{
	type Proof = AncestryProof<MerkleRootOf<T>>;
	type ValidationContext = MerkleRootOf<T>;

	fn is_proof_optimal(proof: &Self::Proof) -> bool {
		let is_proof_optimal = pezpallet_mmr::Pezpallet::<T>::is_ancestry_proof_optimal(proof);

		// We don't check the proof size when running benchmarks, since we use mock proofs
		// which would cause the test to fail.
		if cfg!(feature = "runtime-benchmarks") {
			return true;
		}

		is_proof_optimal
	}

	fn extract_validation_context(header: HeaderFor<T>) -> Option<Self::ValidationContext> {
		// Check if the provided header is canonical.
		let expected_hash = pezframe_system::Pezpallet::<T>::block_hash(header.number());
		if expected_hash != header.hash() {
			return None;
		}

		// Extract the MMR root from the header digest
		header.digest().convert_first(|l| {
			l.try_to(OpaqueDigestItemId::Consensus(&pezsp_consensus_beefy::BEEFY_ENGINE_ID))
				.and_then(|log: ConsensusLog<<T as pezpallet_beefy::Config>::BeefyId>| match log {
					ConsensusLog::MmrRoot(mmr_root) => Some(mmr_root),
					_ => None,
				})
		})
	}

	fn is_non_canonical(
		commitment: &Commitment<BlockNumberFor<T>>,
		proof: Self::Proof,
		context: Self::ValidationContext,
	) -> bool {
		let commitment_leaf_count =
			match pezpallet_mmr::Pezpallet::<T>::block_num_to_leaf_count(commitment.block_number) {
				Ok(commitment_leaf_count) => commitment_leaf_count,
				Err(_) => {
					// We can't prove that the commitment is non-canonical if the
					// `commitment.block_number` is invalid.
					return false;
				},
			};
		if commitment_leaf_count != proof.prev_leaf_count {
			// Can't prove that the commitment is non-canonical if the `commitment.block_number`
			// doesn't match the ancestry proof.
			return false;
		}

		let canonical_mmr_root = context;
		let canonical_prev_root =
			match pezpallet_mmr::Pezpallet::<T>::verify_ancestry_proof(canonical_mmr_root, proof) {
				Ok(canonical_prev_root) => canonical_prev_root,
				Err(_) => {
					// Can't prove that the commitment is non-canonical if the proof
					// is invalid.
					return false;
				},
			};

		let mut found_commitment_root = false;
		let commitment_roots = commitment
			.payload
			.get_all_decoded::<MerkleRootOf<T>>(&known_payloads::MMR_ROOT_ID);
		for maybe_commitment_root in commitment_roots {
			match maybe_commitment_root {
				Some(commitment_root) => {
					found_commitment_root = true;
					if canonical_prev_root != commitment_root {
						// If the commitment contains an MMR root, that is not equal to
						// `canonical_prev_root`, the commitment is invalid
						return true;
					}
				},
				None => {
					// If the commitment contains an MMR root, that can't be decoded, it is invalid.
					return true;
				},
			}
		}
		if !found_commitment_root {
			// If the commitment doesn't contain any MMR root, while the proof is valid,
			// the commitment is invalid
			return true;
		}

		false
	}
}

impl<T: Config> AncestryHelperWeightInfo<HeaderFor<T>> for Pezpallet<T>
where
	T: pezpallet_mmr::Config<Hashing = pezsp_consensus_beefy::MmrHashing>,
{
	fn is_proof_optimal(proof: &<Self as AncestryHelper<HeaderFor<T>>>::Proof) -> Weight {
		<T as Config>::WeightInfo::n_leafs_proof_is_optimal(proof.leaf_count.saturated_into())
	}

	fn extract_validation_context() -> Weight {
		<T as Config>::WeightInfo::extract_validation_context()
	}

	fn is_non_canonical(proof: &<Self as AncestryHelper<HeaderFor<T>>>::Proof) -> Weight {
		let mmr_utils = NodesUtils::new(proof.leaf_count);
		let num_peaks = mmr_utils.number_of_peaks();

		// The approximated cost of verifying an ancestry proof with `n` nodes.
		// We add the previous peaks to the total number of nodes,
		// since they have to be processed as well.
		<T as Config>::WeightInfo::n_items_proof_is_non_canonical(
			proof.items.len().saturating_add(proof.prev_peaks.len()).saturated_into(),
		)
		// `n_items_proof_is_non_canonical()` uses inflated proofs that contain all the leafs,
		// where no peak needs to be read. So we need to also add the cost of reading the peaks.
		.saturating_add(<T as Config>::WeightInfo::read_peak().saturating_mul(num_peaks))
	}
}

impl<T: Config> Pezpallet<T> {
	/// Return the currently active BEEFY authority set proof.
	pub fn authority_set_proof() -> BeefyAuthoritySet<MerkleRootOf<T>> {
		BeefyAuthorities::<T>::get()
	}

	/// Return the next/queued BEEFY authority set proof.
	pub fn next_authority_set_proof() -> BeefyNextAuthoritySet<MerkleRootOf<T>> {
		BeefyNextAuthorities::<T>::get()
	}

	/// Returns details of a BEEFY authority set.
	///
	/// Details contain authority set id, authority set length and a merkle root,
	/// constructed from uncompressed secp256k1 public keys converted to Ethereum addresses
	/// of the next BEEFY authority set.
	fn compute_authority_set(
		validator_set: &BeefyValidatorSet<<T as pezpallet_beefy::Config>::BeefyId>,
	) -> BeefyAuthoritySet<MerkleRootOf<T>> {
		let id = validator_set.id();
		let beefy_addresses = validator_set
			.validators()
			.into_iter()
			.cloned()
			.map(T::BeefyAuthorityToMerkleLeaf::convert)
			.collect::<Vec<_>>();
		let default_eth_addr = [0u8; 20];
		let len = beefy_addresses.len() as u32;
		let uninitialized_addresses = beefy_addresses
			.iter()
			.filter(|&addr| addr.as_slice().eq(&default_eth_addr))
			.count();
		if uninitialized_addresses > 0 {
			log::error!(
				target: "runtime::beefy",
				"Failed to convert {} out of {} BEEFY PublicKeys to ETH addresses!",
				uninitialized_addresses,
				len,
			);
		}
		let keyset_commitment = pez_binary_merkle_tree::merkle_root::<
			<T as pezpallet_mmr::Config>::Hashing,
			_,
		>(beefy_addresses)
		.into();
		BeefyAuthoritySet { id, len, keyset_commitment }
	}
}

pezsp_api::decl_runtime_apis! {
	/// API useful for BEEFY light clients.
	pub trait BeefyMmrApi<H>
	where
		BeefyAuthoritySet<H>: Decode,
	{
		/// Return the currently active BEEFY authority set proof.
		fn authority_set_proof() -> BeefyAuthoritySet<H>;

		/// Return the next/queued BEEFY authority set proof.
		fn next_authority_set_proof() -> BeefyNextAuthoritySet<H>;
	}
}
