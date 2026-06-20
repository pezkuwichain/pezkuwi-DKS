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

//! Defines structures related to calls of the `pezpallet-bridge-teyrchains` pezpallet.

use crate::{ParaHash, ParaId, RelayBlockHash, RelayBlockNumber};

use codec::{Decode, Encode};
use pezbp_pezkuwi_core::teyrchains::ParaHeadsProof;
use pezbp_runtime::HeaderId;
use pezsp_runtime::RuntimeDebug;
use pezsp_std::vec::Vec;
use scale_info::TypeInfo;

/// A minimized version of `pezpallet-bridge-teyrchains::Call` that can be used without a runtime.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
#[allow(non_camel_case_types)]
pub enum BridgeTeyrchainCall {
	/// `pezpallet-bridge-teyrchains::Call::submit_teyrchain_heads`
	#[codec(index = 0)]
	submit_teyrchain_heads {
		/// Relay chain block, for which we have submitted the `teyrchain_heads_proof`.
		at_relay_block: (RelayBlockNumber, RelayBlockHash),
		/// Teyrchain identifiers and their head hashes.
		teyrchains: Vec<(ParaId, ParaHash)>,
		/// Teyrchain heads proof.
		teyrchain_heads_proof: ParaHeadsProof,
	},
}

/// Info about a `SubmitTeyrchainHeads` call which tries to update a single teyrchain.
///
/// The pezpallet supports updating multiple teyrchain heads at once,
#[derive(PartialEq, RuntimeDebug)]
pub struct SubmitTeyrchainHeadsInfo {
	/// Number and hash of the finalized relay block that has been used to prove teyrchain
	/// finality.
	pub at_relay_block: HeaderId<RelayBlockHash, RelayBlockNumber>,
	/// Teyrchain identifier.
	pub para_id: ParaId,
	/// Hash of the bundled teyrchain head.
	pub para_head_hash: ParaHash,
	/// If `true`, then the call must be free (assuming that everything else is valid) to
	/// be treated as valid.
	pub is_free_execution_expected: bool,
}
