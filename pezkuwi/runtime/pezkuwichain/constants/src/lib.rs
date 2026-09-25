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

	/// One HEZ, which is what `tokenSymbol` reports on the relay chain, the Asset Hub and
	/// People alike. Balances are carried in the smallest indivisible amount, the TYR, and
	/// one HEZ is 10^12 of them -- the same relationship a DOT has to a planck. Chain specs
	/// pair this with `tokenDecimals: 12`; the two have to agree or every displayed balance
	/// is wrong by a power of ten.
	pub const UNITS: Balance = 1_000_000_000_000;

	/// What the initial validators are funded with at genesis, carved out of the treasury's
	/// 40M share rather than added beside it.
	///
	/// It lives here because two chains have to agree on it and neither can derive it: the
	/// relay mints it onto the validator stashes, and the Asset Hub subtracts the same number
	/// from the treasury's share before minting the rest into the pot the spender tracks pay
	/// from. A constant rather than `authorities.len() * stash`, so seating another validator
	/// cannot silently change what either side mints.
	pub const HEZ_VALIDATOR_FUNDING: Balance = 1_000 * UNITS;

	/// What root starts with, so its first call can pay for itself.
	///
	/// Root is its own account here rather than the founder's, and the moment it stopped being
	/// the founder's it stopped being funded -- which is how Zagros launched on 2026-09-14:
	/// registering the teyrchains failed because the extrinsic could not pay its fee, and the
	/// chain came up ungovernable until an account was funded by hand.
	///
	/// Carved out of the founder's allocation rather than added on top, the same way
	/// `HEZ_VALIDATOR_FUNDING` is carved out of the treasury's, so the genesis total stays at
	/// two hundred million to the planck.
	///
	/// Sized from measurement: five sudo calls on a live chain cost 0.000641 HEZ in total. A
	/// thousand is a fee budget with a very wide margin, and small enough that the root account
	/// can never be mistaken for a treasury.
	pub const HEZ_SUDO_FUNDING: Balance = 1_000 * UNITS;

	/// What the founding office starts with, on each chain it has to act on.
	///
	/// The holder of `Tiki::Serok` is the only origin that can write the People chain's
	/// register on the founding day. `TheRegisterIsNotWritableFromAbroad` drops every register
	/// call arriving over XCM, so the relay's sudo cannot seat the founding Parliament; People
	/// has no sudo pallet of its own; and its Root track wants a referendum, which wants a roll
	/// that does not exist yet. `seat_founding_parliament` is signed by that key or the house
	/// never sits -- and a key with no balance cannot sign anything.
	///
	/// Three chains rather than one, decided 2026-09-19. The office acts on People (the
	/// register), on the Asset Hub (the pots the state spends from) and on the relay (the chain
	/// itself), and a fee budget on one of the three is a gap discovered on the day. That is not
	/// hypothetical here: Zagros launched on 2026-09-14 with an unfunded root key and came up
	/// ungovernable, which is the same failure one chain over.
	///
	/// Carved out of the founder's allocation rather than added on top, exactly as
	/// `HEZ_SUDO_FUNDING` is, so the genesis total stays at two hundred million to the planck.
	/// The size rests on the same measurement: five sudo calls on the live chain cost 0.000641
	/// HEZ altogether, so a thousand is a very wide fee budget and far too small to be mistaken
	/// for a treasury.
	pub const HEZ_FOUNDING_OFFICE_FUNDING: Balance = 1_000 * UNITS;

	/// The whole carve-out, so the founder's line is reduced once and by the right amount.
	///
	/// Written as a product rather than three subtractions at three call sites: the relay is
	/// where the founder's line is computed, and it has to know what the other two chains mint
	/// without being able to see them.
	pub const HEZ_FOUNDING_OFFICE_CARVE_OUT: Balance = 3 * HEZ_FOUNDING_OFFICE_FUNDING;

	/// What the relay's accumulation account starts with: its existential deposit.
	///
	/// Fees, dust and coretime revenue are paid into that account by `resolve`, and `resolve`
	/// refuses any deposit that would leave an account below the existential deposit. With the
	/// account unfunded every such deposit smaller than the deposit itself is refused, and the
	/// credit is dropped -- burned, with a defensive log and nothing else. Measured 2026-09-25:
	/// the account did not exist on the relay or on People, on mainnet or on Zagros, and the
	/// benchmark run logged the burn ninety-eight thousand times. The pallet's own setup notes
	/// say to fund it in genesis; nothing did.
	pub const HEZ_ACCUMULATION_RELAY: Balance = EXISTENTIAL_DEPOSIT;

	/// The same for People's accumulation account. People's existential deposit is a tenth of
	/// the relay's; the People runtime pins that the two agree, since this crate cannot see it.
	pub const HEZ_ACCUMULATION_PEOPLE: Balance = EXISTENTIAL_DEPOSIT / 10;

	/// Both, carved out of the founder's allocation as the office budgets are, so the genesis
	/// total stays at two hundred million to the planck.
	pub const HEZ_ACCUMULATION_CARVE_OUT: Balance =
		HEZ_ACCUMULATION_RELAY + HEZ_ACCUMULATION_PEOPLE;
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
	/// A grand is a thousand dollars, which is what the name has always meant. Upstream spells
	/// it `CENTS * 100_000`, which only comes to that where a cent is a hundredth; here a cent is
	/// a thirty-thousandth, so it evaluated to roughly 3.33 units and every governance figure
	/// built on it came out three hundred times too small.
	pub const GRAND: Balance = DOLLARS * 1_000;
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
	/// People teyrchain ID.
	pub const PEOPLE_ID: u32 = 1004;
	/// BridgeHub teyrchain ID.
	///
	/// 1002 was also declared as `CONTRACTS_ID`, so `Teyrchain(CONTRACTS_ID)` resolved to this
	/// chain and the relay trusted it twice under two names. There is no contracts chain here;
	/// the constant is gone rather than renumbered.
	pub const BRIDGE_HUB_ID: u32 = 1002;
	/// Brokerage teyrchain ID.
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
