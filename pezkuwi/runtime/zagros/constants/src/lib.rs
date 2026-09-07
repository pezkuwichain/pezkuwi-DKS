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

#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;

/// Money matters.
pub mod currency {
	use pezkuwi_primitives::Balance;

	/// The existential deposit.
	pub const EXISTENTIAL_DEPOSIT: Balance = 1 * CENTS;

	/// One ZGR, the testnet counterpart of a HEZ. Balances are carried in the smallest
	/// indivisible amount and one ZGR is 10^12 of them, the same relationship a DOT has to
	/// a planck. Chain specs pair this with `tokenDecimals: 12`; the two have to agree or
	/// every displayed balance is wrong by a power of ten.
	pub const UNITS: Balance = 1_000_000_000_000;
	pub const CENTS: Balance = UNITS / 30_000;
	/// One unit, named for what a deposit or a spend is reckoned in.
	///
	/// The governance figures inherited from upstream are written `N * 3 * CENTS`, which reads as
	/// "N dollars" only where three cents come to a dollar. Here a cent is a thirty-thousandth,
	/// so those expressions evaluated to fractions of a unit: the largest treasury track asked
	/// 0.04 to open a referendum against a `Treasurer` track asking a thousand — twenty-five
	/// thousand times apart, in the same file. Name the unit and reckon in it, so the ladder
	/// stays legible and the cent scale is not load-bearing.
	pub const DOLLARS: Balance = UNITS;
	/// A grand is a thousand of the base unit, which is what the name has always meant.
	/// Upstream spells it `CENTS * 100_000` — a hundred thousand cents — which only comes to a
	/// thousand where a cent is a hundredth. Here a cent is a thirty-thousandth, so that
	/// spelling yielded roughly 3.33 units, and every governance figure derived from it came out
	/// three hundred times too small: referendum submission and decision deposits of a few units
	/// against a supply in the hundreds of millions, and a Fellowship spend cap of about 33.
	/// Derive it from what it means rather than from the other ecosystem's cent scale, which
	/// this crate's own comments warn against mixing.
	pub const GRAND: Balance = UNITS * 1_000;
	pub const MILLICENTS: Balance = CENTS / 1_000;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 2_000 * CENTS + (bytes as Balance) * 100 * MILLICENTS
	}
}

/// Time and blocks.
pub mod time {
	use pezkuwi_runtime_common::prod_or_fast;

	use pezkuwi_primitives::{BlockNumber, Moment};
	pub const MILLISECS_PER_BLOCK: Moment = 6000;
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	pezframe_support::parameter_types! {
		pub EpochDurationInBlocks: BlockNumber =
			prod_or_fast!(1 * HOURS, 1 * MINUTES, "PEZKUWICHAIN_EPOCH_DURATION");
	}

	// These time units are defined in number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
	pub const WEEKS: BlockNumber = DAYS * 7;

	// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
	// The choice of is done in accordance to the slot duration and expected target
	// block time, for safely resisting network delays of maximum two seconds.
	// <https://research.web3.foundation/Polkadot/protocols/block-production/Babe#6-practical-results>
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}

/// Fee-related.
pub mod fee {
	use crate::weights::ExtrinsicBaseWeight;
	use pezframe_support::weights::{
		WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	};
	use pezkuwi_primitives::Balance;
	pub use pezsp_runtime::Perbill;
	use smallvec::smallvec;

	/// The block saturation level. Fees will be updates based on this value.
	pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);

	/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
	/// node's balance type.
	///
	/// This should typically create a mapping between the following ranges:
	///   - [0, `pezframe_system::MaximumBlockWeight`]
	///   - [Balance::min, Balance::max]
	///
	/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
	///   - Setting it to `0` will essentially disable the weight fee.
	///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
	pub struct WeightToFee;
	impl WeightToFeePolynomial for WeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			// in Pezkuwichain, extrinsic base weight (smallest non-zero weight) is mapped to 1/10
			// CENT:
			let p = super::currency::CENTS;
			let q = 10 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}
}

/// System Teyrchains.
pub mod system_teyrchain {
	use pezframe_support::parameter_types;
	use pezkuwi_primitives::Id as ParaId;
	use xcm_builder::IsChildSystemTeyrchain;

	parameter_types! {
		pub AssetHubParaId: ParaId = ASSET_HUB_ID.into();
		pub PeopleParaId: ParaId = PEOPLE_ID.into();
	}

	/// Network's Asset Hub teyrchain ID.
	pub const ASSET_HUB_ID: u32 = 1000;
	/// Collectives teyrchain ID. Zagros keeps a collectives chain, so the ID stays here
	/// even though mainnet has none.
	pub const COLLECTIVES_ID: u32 = 1001;
	/// People teyrchain ID.
	pub const PEOPLE_ID: u32 = 1004;
	/// BridgeHub teyrchain ID.
	pub const BRIDGE_HUB_ID: u32 = 1002;
	/// Brokerage teyrchain ID.
	/// 1005 was also declared as `ASSET_HUB_NEXT_ID`, reserved for a migration rehearsal that
	/// this network never ran. Two names for one para ID is how a location ends up pointing at
	/// the wrong chain, so the unused one is gone rather than renumbered.
	pub const BROKER_ID: u32 = 1005;

	/// All system teyrchains of Pezkuwichain.
	pub type SystemTeyrchains = IsChildSystemTeyrchain<ParaId>;

	/// Coretime constants
	pub mod coretime {
		/// Coretime timeslice period in blocks
		/// WARNING: This constant is used accross chains, so additional care should be taken
		/// when changing it.
		#[cfg(feature = "fast-runtime")]
		pub const TIMESLICE_PERIOD: u32 = 20;
		#[cfg(not(feature = "fast-runtime"))]
		pub const TIMESLICE_PERIOD: u32 = 80;
	}
}

// `TREASURY_PALLET_ID: u8 = 18` stood here, naming the relay's Treasury pallet so a
// `PalletInstance` location could address it. The treasury moved to the Asset Hub and index 18
// is retired; the constant outlived the pallet and named a number nothing answers on. Nothing
// consumed it, which is the only reason it was harmless -- a location built from it would have
// addressed an empty slot. The Asset Hub's treasury is reached by its own `PalletId`
// (`teyrchains_common::TREASURY_PALLET_ID`), which is a different thing with the same name.

#[cfg(test)]
mod tests {
	use super::{
		currency::{CENTS, MILLICENTS},
		fee::WeightToFee,
	};
	use crate::weights::ExtrinsicBaseWeight;
	use pezframe_support::weights::WeightToFee as WeightToFeeT;
	use pezkuwi_runtime_common::MAXIMUM_BLOCK_WEIGHT;

	#[test]
	// Test that the fee for `MAXIMUM_BLOCK_WEIGHT` of weight has sane bounds.
	fn full_block_fee_is_correct() {
		// A full block costs what fits in it, and what fits is a property of the reference
		// hardware rather than of the fee schedule. `WeightToFee` anchors `ExtrinsicBaseWeight`
		// at a tenth of a CENT -- that is the economic decision, and
		// `extrinsic_base_fee_is_correct` below is what holds it. This test only asks that the
		// resulting block price stay in a sane range.
		//
		// The range was [1_000, 10_000], upstream's numbers for upstream's machine: their
		// extrinsic base is around 100 microseconds, so twenty thousand extrinsics fit in two
		// seconds and a full block comes to 2_000 CENTS. Ours is 223 microseconds, measured on
		// the weakest validator class we intend to support, so 8_968 fit and a full block is
		// 897 CENTS. A slower reference machine means fewer transactions per block, not cheaper
		// transactions.
		//
		// 500 rather than 897 so the bound survives re-benchmarking: this figure moves whenever
		// the reference hardware is re-measured, and a bound pinned to today's number would go
		// red on a routine measurement rather than on a defect.
		let full_block = WeightToFee::weight_to_fee(&MAXIMUM_BLOCK_WEIGHT);
		assert!(full_block >= 500 * CENTS);
		assert!(full_block <= 10_000 * CENTS);
	}

	#[test]
	// This function tests that the fee for `ExtrinsicBaseWeight` of weight is correct
	fn extrinsic_base_fee_is_correct() {
		// `ExtrinsicBaseWeight` should cost 1/10 of a CENT
		println!("Base: {}", ExtrinsicBaseWeight::get());
		let x = WeightToFee::weight_to_fee(&ExtrinsicBaseWeight::get());
		let y = CENTS / 10;
		assert!(x.max(y) - x.min(y) < MILLICENTS);
	}
}

/// XCM protocol related constants.
pub mod xcm {
	/// Pluralistic bodies existing within the consensus.
	pub mod body {
		// Preallocated for the Root body.
		#[allow(dead_code)]
		const ROOT_INDEX: u32 = 0;
		// The bodies corresponding to the Pezkuwi OpenGov Origins.
		pub const FELLOWSHIP_ADMIN_INDEX: u32 = 1;
		#[deprecated = "Will be removed after August 2024; Use `xcm::latest::BodyId::Treasury` \
			instead"]
		pub const TREASURER_INDEX: u32 = 2;
	}
}
