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

//! Teyrchains finality pezpallet benchmarking.

use crate::{
	weights_ext::DEFAULT_TEYRCHAIN_HEAD_SIZE, Call, RelayBlockHash, RelayBlockHasher,
	RelayBlockNumber,
};

use pezbp_pezkuwi_core::teyrchains::{ParaHash, ParaHeadsProof, ParaId};
use pezbp_runtime::UnverifiedStorageProofParams;
use pezframe_benchmarking::{account, benchmarks_instance_pallet};
use pezframe_system::RawOrigin;
use pezsp_std::prelude::*;

/// Pezpallet we're benchmarking here.
pub struct Pezpallet<T: Config<I>, I: 'static = ()>(crate::Pezpallet<T, I>);

/// Trait that must be implemented by runtime to benchmark the teyrchains finality pezpallet.
pub trait Config<I: 'static>: crate::Config<I> {
	/// Returns vector of supported teyrchains.
	fn teyrchains() -> Vec<ParaId>;
	/// Generate teyrchain heads proof and prepare environment for verifying this proof.
	fn prepare_teyrchain_heads_proof(
		teyrchains: &[ParaId],
		teyrchain_head_size: u32,
		proof_params: UnverifiedStorageProofParams,
	) -> (RelayBlockNumber, RelayBlockHash, ParaHeadsProof, Vec<(ParaId, ParaHash)>);
}

benchmarks_instance_pallet! {
	where_clause {
		where
			<T as pezpallet_bridge_grandpa::Config<T::BridgesGrandpaPalletInstance>>::BridgedChain:
				pezbp_runtime::Chain<
					BlockNumber = RelayBlockNumber,
					Hash = RelayBlockHash,
					Hasher = RelayBlockHasher,
				>,
	}

	// Benchmark `submit_teyrchain_heads` extrinsic with different number of teyrchains.
	submit_teyrchain_heads_with_n_teyrchains {
		let p in 1..(T::teyrchains().len() + 1) as u32;

		let sender = account("sender", 0, 0);
		let mut teyrchains = T::teyrchains();
		let _ = if p <= teyrchains.len() as u32 {
			teyrchains.split_off(p as usize)
		} else {
			Default::default()
		};
		tracing::trace!(target: crate::LOG_TARGET, "=== {:?}", teyrchains.len());
		let (relay_block_number, relay_block_hash, teyrchain_heads_proof, teyrchains_heads) = T::prepare_teyrchain_heads_proof(
			&teyrchains,
			DEFAULT_TEYRCHAIN_HEAD_SIZE,
			UnverifiedStorageProofParams::default(),
		);
		let at_relay_block = (relay_block_number, relay_block_hash);
	}: submit_teyrchain_heads(RawOrigin::Signed(sender), at_relay_block, teyrchains_heads, teyrchain_heads_proof)
	verify {
		for teyrchain in teyrchains {
			assert!(crate::Pezpallet::<T, I>::best_teyrchain_head(teyrchain).is_some());
		}
	}

	// Benchmark `submit_teyrchain_heads` extrinsic with 1kb proof size.
	submit_teyrchain_heads_with_1kb_proof {
		let sender = account("sender", 0, 0);
		let teyrchains = vec![T::teyrchains()[0]];
		let (relay_block_number, relay_block_hash, teyrchain_heads_proof, teyrchains_heads) = T::prepare_teyrchain_heads_proof(
			&teyrchains,
			DEFAULT_TEYRCHAIN_HEAD_SIZE,
			UnverifiedStorageProofParams::from_db_size(1024),
		);
		let at_relay_block = (relay_block_number, relay_block_hash);
	}: submit_teyrchain_heads(RawOrigin::Signed(sender), at_relay_block, teyrchains_heads, teyrchain_heads_proof)
	verify {
		for teyrchain in teyrchains {
			assert!(crate::Pezpallet::<T, I>::best_teyrchain_head(teyrchain).is_some());
		}
	}

	// Benchmark `submit_teyrchain_heads` extrinsic with 16kb proof size.
	submit_teyrchain_heads_with_16kb_proof {
		let sender = account("sender", 0, 0);
		let teyrchains = vec![T::teyrchains()[0]];
		let (relay_block_number, relay_block_hash, teyrchain_heads_proof, teyrchains_heads) = T::prepare_teyrchain_heads_proof(
			&teyrchains,
			DEFAULT_TEYRCHAIN_HEAD_SIZE,
			UnverifiedStorageProofParams::from_db_size(16 * 1024),
		);
		let at_relay_block = (relay_block_number, relay_block_hash);
	}: submit_teyrchain_heads(RawOrigin::Signed(sender), at_relay_block, teyrchains_heads, teyrchain_heads_proof)
	verify {
		for teyrchain in teyrchains {
			assert!(crate::Pezpallet::<T, I>::best_teyrchain_head(teyrchain).is_some());
		}
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(), crate::mock::TestRuntime)
}
