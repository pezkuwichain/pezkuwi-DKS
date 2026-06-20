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

//! Primitives of pezkuwi-like chains, that are related to teyrchains functionality.
//!
//! Even though this (bridges) repository references pezkuwi repository, we can't
//! reference pezkuwi crates from pallets. That's because bridges repository is
//! included in the Pezcumulus repository and included pallets are used by Pezcumulus
//! teyrchains. Having pallets that are referencing pezkuwi, would mean that there may
//! be two versions of pezkuwi crates included in the runtime. Which is bad.

use codec::{CompactAs, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pezbp_runtime::{raw_storage_proof_size, RawStorageProof, Size};
use pezsp_core::Hasher;
use pezsp_runtime::RuntimeDebug;
use pezsp_std::vec::Vec;
use scale_info::TypeInfo;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Teyrchain id.
///
/// This is an equivalent of the `pezkuwi_teyrchain_primitives::Id`, which is a compact-encoded
/// `u32`.
#[derive(
	Clone,
	CompactAs,
	Copy,
	Decode,
	DecodeWithMemTracking,
	Default,
	Encode,
	Eq,
	Hash,
	MaxEncodedLen,
	Ord,
	PartialEq,
	PartialOrd,
	RuntimeDebug,
	TypeInfo,
)]
pub struct ParaId(pub u32);

impl From<u32> for ParaId {
	fn from(id: u32) -> Self {
		ParaId(id)
	}
}

/// Teyrchain head.
///
/// This is an equivalent of the `pezkuwi_teyrchain_primitives::HeadData`.
///
/// The teyrchain head means (at least in Pezcumulus) a SCALE-encoded teyrchain header.
#[derive(
	PartialEq,
	Eq,
	Clone,
	PartialOrd,
	Ord,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	Default,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Hash))]
pub struct ParaHead(pub Vec<u8>);

impl ParaHead {
	/// Returns the hash of this head data.
	pub fn hash(&self) -> crate::Hash {
		pezsp_runtime::traits::BlakeTwo256::hash(&self.0)
	}
}

/// Teyrchain head hash.
pub type ParaHash = crate::Hash;

/// Teyrchain head hasher.
pub type ParaHasher = crate::Hasher;

/// Raw storage proof of teyrchain heads, stored in pezkuwi-like chain runtime.
#[derive(Clone, Decode, DecodeWithMemTracking, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct ParaHeadsProof {
	/// Unverified storage proof of finalized teyrchain heads.
	pub storage_proof: RawStorageProof,
}

impl Size for ParaHeadsProof {
	fn size(&self) -> u32 {
		use pezframe_support::pezsp_runtime::SaturatedConversion;
		raw_storage_proof_size(&self.storage_proof).saturated_into()
	}
}
