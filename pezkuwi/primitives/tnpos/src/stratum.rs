// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The nine strata a TNPoS committee is drawn from.
//!
//! Each stratum's gate belongs to a different source of authority. That is the hidden
//! condition of the security arithmetic: two strata gated by the same institution count
//! as one stratum, not two.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// One of the nine independent gates a committee seat can be drawn through.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
pub enum StratumId {
	/// Bonded HEZ; ranked internally by the existing Phragmen election on Asset Hub.
	#[codec(index = 0)]
	Stake,
	/// Elected members of the assembly.
	#[codec(index = 1)]
	Meclis,
	/// Members of the court.
	#[codec(index = 2)]
	Divan,
	/// Holders of accredited education credentials.
	#[codec(index = 3)]
	Perwerde,
	/// Community-granted tikis. Office tikis are excluded on purpose: they would tie this
	/// stratum back to the assembly and collapse two gates into one.
	#[codec(index = 4)]
	Tiki,
	/// Drawn by lot from every citizen; gated only by citizenship.
	#[codec(index = 5)]
	WelatiLottery,
	/// Attested residence outside the region.
	#[codec(index = 6)]
	Geography,
	/// Uninterrupted, offence-free pool membership. Time cannot be bought or granted.
	#[codec(index = 7)]
	Tenure,
	/// Measured operating record on attested, independent infrastructure.
	#[codec(index = 8)]
	Infrastructure,
}

impl StratumId {
	/// Every stratum, in index order.
	pub const ALL: [StratumId; 9] = [
		StratumId::Stake,
		StratumId::Meclis,
		StratumId::Divan,
		StratumId::Perwerde,
		StratumId::Tiki,
		StratumId::WelatiLottery,
		StratumId::Geography,
		StratumId::Tenure,
		StratumId::Infrastructure,
	];
}

/// How many seats a stratum carries and how many eligible members it needs to carry them.
///
/// `min_eligible` is the runtime's whole security check. The probability argument behind
/// the number lives in `analysis`, which never reaches the runtime.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
pub struct StratumConfig {
	pub id: StratumId,
	pub seats: u32,
	pub min_eligible: u32,
}

#[cfg(test)]
mod tests {
	use super::*;
	use scale_info::{TypeDef, TypeInfo};

	#[test]
	fn all_lists_every_stratum_once() {
		let mut seen = StratumId::ALL.to_vec();
		seen.sort();
		seen.dedup();
		assert_eq!(seen.len(), 9, "StratumId::ALL must list all nine strata exactly once");
	}

	// A variant's index is what the chain wrote into storage; move it and old bytes decode
	// as a different stratum -- no error, a different answer.
	#[test]
	fn stratum_indices_are_pinned() {
		let info = <StratumId as TypeInfo>::type_info();
		let TypeDef::Variant(v) = info.type_def else { panic!("StratumId is not an enum") };
		let got: Vec<(String, u8)> =
			v.variants.iter().map(|x| (x.name.to_string(), x.index)).collect();
		let want = [
			("Stake", 0u8),
			("Meclis", 1),
			("Divan", 2),
			("Perwerde", 3),
			("Tiki", 4),
			("WelatiLottery", 5),
			("Geography", 6),
			("Tenure", 7),
			("Infrastructure", 8),
		];
		assert_eq!(got.len(), want.len(), "a new stratum needs its own index and a line here");
		for (i, (n, ix)) in want.iter().enumerate() {
			assert_eq!((got[i].0.as_str(), got[i].1), (*n, *ix), "stratum {i} moved");
		}
	}
}
