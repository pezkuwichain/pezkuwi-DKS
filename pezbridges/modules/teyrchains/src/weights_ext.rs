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

//! Weight-related utilities.

use crate::weights::{BridgeWeight, WeightInfo};

use pezbp_runtime::Size;
use pezframe_support::weights::{RuntimeDbWeight, Weight};

/// Size of the regular teyrchain head.
///
/// It's not that we are expecting all teyrchain heads to share the same size or that we would
/// reject all heads that have larger/lesser size. It is about head size that we use in benchmarks.
/// Relayer would need to pay additional fee for extra bytes.
///
/// 384 is a bit larger (1.3 times) than the size of the randomly chosen Pezkuwi block.
pub const DEFAULT_TEYRCHAIN_HEAD_SIZE: u32 = 384;

/// Number of extra bytes (excluding size of storage value itself) of storage proof, built at
/// some generic chain.
pub const EXTRA_STORAGE_PROOF_SIZE: u32 = 1024;

/// Extended weight info.
pub trait WeightInfoExt: WeightInfo {
	// Our configuration assumes that the runtime has special signed extensions used to:
	//
	// 1) boost priority of `submit_teyrchain_heads` transactions;
	//
	// 2) slash relayer if he submits an invalid transaction.
	//
	// We read and update storage values of other pallets (`pezpallet-bridge-relayers` and
	// balances/assets pezpallet). So we need to add this weight to the weight of our call.
	// Hence two following methods.

	/// Extra weight that is added to the `submit_finality_proof` call weight by signed extensions
	/// that are declared at runtime level.
	fn submit_teyrchain_heads_overhead_from_runtime() -> Weight;

	/// Storage proof overhead, that is included in every storage proof.
	///
	/// The relayer would pay some extra fee for additional proof bytes, since they mean
	/// more hashing operations.
	fn expected_extra_storage_proof_size() -> u32;

	/// Weight of the teyrchain heads delivery extrinsic.
	fn submit_teyrchain_heads_weight(
		db_weight: RuntimeDbWeight,
		proof: &impl Size,
		teyrchains_count: u32,
	) -> Weight {
		// weight of the `submit_teyrchain_heads` with exactly `teyrchains_count` teyrchain
		// heads of the default size (`DEFAULT_TEYRCHAIN_HEAD_SIZE`)
		let base_weight = Self::submit_teyrchain_heads_with_n_teyrchains(teyrchains_count);

		// overhead because of extra storage proof bytes
		let expected_proof_size = teyrchains_count
			.saturating_mul(DEFAULT_TEYRCHAIN_HEAD_SIZE)
			.saturating_add(Self::expected_extra_storage_proof_size());
		let actual_proof_size = proof.size();
		let proof_size_overhead = Self::storage_proof_size_overhead(
			actual_proof_size.saturating_sub(expected_proof_size),
		);

		// potential pruning weight (refunded if hasn't happened)
		let pruning_weight =
			Self::teyrchain_head_pruning_weight(db_weight).saturating_mul(teyrchains_count as u64);

		base_weight
			.saturating_add(proof_size_overhead)
			.saturating_add(pruning_weight)
			.saturating_add(Self::submit_teyrchain_heads_overhead_from_runtime())
	}

	/// Returns weight of single teyrchain head storage update.
	///
	/// This weight only includes db write operations that happens if teyrchain head is actually
	/// updated. All extra weights (weight of storage proof validation, additional checks, ...) is
	/// not included.
	fn teyrchain_head_storage_write_weight(db_weight: RuntimeDbWeight) -> Weight {
		// it's just a couple of operations - we need to write the hash (`ImportedParaHashes`) and
		// the head itself (`ImportedParaHeads`. Pruning is not included here
		db_weight.writes(2)
	}

	/// Returns weight of single teyrchain head pruning.
	fn teyrchain_head_pruning_weight(db_weight: RuntimeDbWeight) -> Weight {
		// it's just one write operation, we don't want any benchmarks for that
		db_weight.writes(1)
	}

	/// Returns weight that needs to be accounted when storage proof of given size is received.
	fn storage_proof_size_overhead(extra_proof_bytes: u32) -> Weight {
		let extra_byte_weight = (Self::submit_teyrchain_heads_with_16kb_proof()
			- Self::submit_teyrchain_heads_with_1kb_proof())
			/ (15 * 1024);
		extra_byte_weight.saturating_mul(extra_proof_bytes as u64)
	}
}

impl WeightInfoExt for () {
	fn submit_teyrchain_heads_overhead_from_runtime() -> Weight {
		Weight::zero()
	}

	fn expected_extra_storage_proof_size() -> u32 {
		EXTRA_STORAGE_PROOF_SIZE
	}
}

impl<T: pezframe_system::Config> WeightInfoExt for BridgeWeight<T> {
	fn submit_teyrchain_heads_overhead_from_runtime() -> Weight {
		Weight::zero()
	}

	fn expected_extra_storage_proof_size() -> u32 {
		EXTRA_STORAGE_PROOF_SIZE
	}
}
