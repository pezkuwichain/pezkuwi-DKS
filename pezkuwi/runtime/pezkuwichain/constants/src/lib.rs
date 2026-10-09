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

	/// What each genesis validator starts with on the Asset Hub (spec K5): the validator bond,
	/// bonded, and a thousand beside it for fees.
	pub const HEZ_GENESIS_VALIDATOR_STAKE: Balance = 11_000 * UNITS;
	/// Of `HEZ_GENESIS_VALIDATOR_STAKE`, the part bonded at genesis -- the validator floor.
	pub const HEZ_GENESIS_VALIDATOR_BOND: Balance = 10_000 * UNITS;
	/// All of it, carved out of the founder's allocation on the relay and minted on the Asset
	/// Hub, so the genesis total stays at two hundred million to the planck.
	pub const HEZ_GENESIS_STAKE_CARVE_OUT: Balance =
		super::genesis::VALIDATOR_STASHES.len() as Balance * HEZ_GENESIS_VALIDATOR_STAKE;
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
/// The genesis validators' stashes, which the relay seats and the Asset Hub bonds (spec K5).
///
/// One list for both chains: the relay's genesis authorities and the Asset Hub's genesis
/// stakers must be the same accounts, and a list held twice drifts. The relay's preset test
/// `the_relay_seats_the_stashes_the_asset_hub_bonds` holds its own authorities to this one.
/// Generated from `res/genesis/mainnet/mainnet-wallets.json` (public keys only; that file
/// never enters this repository).
pub mod genesis {
	/// The twenty-seven genesis validators' stash public keys, in seating order.
	pub const VALIDATOR_STASHES: [[u8; 32]; 27] = [
		// Validator_01: 5G4e6KKUViiwUp6qPgvdqNxwtaGM7LzYB2KVr9bXMjJAfMDF
		[
			0xb0, 0xe4, 0x42, 0xbf, 0x46, 0x7e, 0xf3, 0x68, 0xa9, 0x24, 0x74, 0x74, 0xbf, 0x1a,
			0x97, 0xa1, 0xc0, 0x8a, 0xd9, 0x21, 0x36, 0x73, 0xe3, 0x18, 0x0f, 0xda, 0xb4, 0x07,
			0xa7, 0x58, 0xac, 0x3c,
		],
		// Validator_02: 5E5239nsL71Qb2MUmPwPtUM8pLb1Md4KkNBajmJ6wAHQwZP1
		[
			0x58, 0xb5, 0xab, 0x34, 0xbf, 0xef, 0x86, 0xcc, 0xd2, 0xfd, 0xa2, 0x4b, 0x4b, 0x40,
			0x34, 0xd8, 0xe7, 0x7f, 0x03, 0xe2, 0x4b, 0x19, 0xc8, 0x10, 0xb1, 0xd3, 0x02, 0x54,
			0x56, 0x03, 0x68, 0x06,
		],
		// Validator_03: 5Ck4sveGyXL8Y3Fd1cqcEnUmXwRxWRE64AyUVaKdVumnbbSm
		[
			0x1e, 0x05, 0x56, 0x05, 0xaa, 0xe1, 0x78, 0xfb, 0x4d, 0xb0, 0x58, 0xe1, 0xff, 0xa4,
			0x38, 0x81, 0x2c, 0xec, 0xd1, 0x6c, 0x45, 0xe1, 0x94, 0xed, 0x83, 0x1c, 0x27, 0x50,
			0x8b, 0xea, 0x75, 0x19,
		],
		// Validator_04: 5ECHUqJ8KczMu3ChhX2f9JD4w9Z9ZfgzKbu5KEJ7TobL1DBW
		[
			0x5e, 0x40, 0x5f, 0x23, 0x06, 0x59, 0x3d, 0x90, 0x38, 0x7b, 0xf8, 0x4c, 0x40, 0x13,
			0x20, 0x72, 0xb1, 0xac, 0xe7, 0xcd, 0x09, 0xb7, 0x79, 0xff, 0xc6, 0x21, 0xb4, 0x5f,
			0x57, 0x8a, 0xcb, 0x0c,
		],
		// Validator_05: 5DUwAkAaBme36xwFePyWyyVPAJs1fLDurtXuNopPPCQJnCnw
		[
			0x3e, 0xb6, 0xef, 0x27, 0xbf, 0xf8, 0x78, 0x4a, 0x63, 0x54, 0xdd, 0x00, 0x4b, 0x22,
			0x58, 0xd9, 0x5d, 0x2b, 0x2a, 0x65, 0x57, 0x19, 0xab, 0xac, 0x1a, 0x29, 0x7a, 0x7f,
			0x01, 0x4f, 0x32, 0x4d,
		],
		// Validator_06: 5CoMjECS3gHczmRd24NZGyATuP4iCmU3S4p97Aehinudegcp
		[
			0x20, 0x87, 0xcb, 0x67, 0x7d, 0x08, 0xec, 0x47, 0xc6, 0xa3, 0xbc, 0x36, 0xb9, 0x4e,
			0x92, 0x22, 0x6d, 0x47, 0xf3, 0x2e, 0xbf, 0x18, 0xdb, 0xe3, 0x98, 0x08, 0x01, 0x4f,
			0x3b, 0xb1, 0xf6, 0x74,
		],
		// Validator_07: 5ENhEwumArbBpLNqc258MQe3pyZhgKjNMWTL8NaeCoc3Ayyf
		[
			0x66, 0x30, 0xcf, 0x99, 0x96, 0xf3, 0x6e, 0xbd, 0x25, 0x86, 0xa0, 0x60, 0x03, 0x24,
			0xf0, 0x26, 0x8f, 0x8b, 0xc5, 0x4d, 0xc5, 0x0e, 0x56, 0xd3, 0x1e, 0xa6, 0xcf, 0xa7,
			0xf8, 0x23, 0x42, 0x19,
		],
		// Validator_08: 5Ey4cgchows4382WhyvMaoLeNENFoUpg3Zz5iHidZti7ypYy
		[
			0x80, 0x67, 0x1b, 0xca, 0x44, 0x67, 0x1b, 0x26, 0x7f, 0xe0, 0x3e, 0x28, 0xab, 0x3b,
			0x67, 0xfa, 0xc9, 0xff, 0x55, 0x8a, 0xc4, 0x6c, 0x82, 0xf3, 0x46, 0x2f, 0xdb, 0x7b,
			0xfb, 0x66, 0xf7, 0x2c,
		],
		// Validator_09: 5H8YwbdehipCdPjLHkrCh5wHgaza5iVE1bhGiavGG7reWGLR
		[
			0xe0, 0x1c, 0x23, 0x65, 0x95, 0x9e, 0x8c, 0xda, 0x2e, 0x23, 0xd2, 0x5b, 0xf1, 0x48,
			0x8c, 0xf9, 0x68, 0x00, 0x91, 0xeb, 0xe4, 0xda, 0x86, 0x5a, 0x87, 0x3b, 0x9b, 0x29,
			0x96, 0xa5, 0x6a, 0x5e,
		],
		// Validator_10: 5EhMtwRJCJ8fQ8ZyNxVL5FGExjaobwgnGbrsq3dJoxJdoJEX
		[
			0x74, 0x6d, 0x5a, 0x46, 0x41, 0xe7, 0xde, 0x16, 0x28, 0x3a, 0xe2, 0x51, 0x92, 0xd2,
			0xb8, 0x7b, 0xf3, 0x97, 0xce, 0xde, 0xec, 0xa1, 0x32, 0xb7, 0x0e, 0x24, 0x4d, 0xc4,
			0x76, 0x38, 0xfb, 0x3f,
		],
		// Validator_11: 5Fmm54cqKb7PywNjkZGro97eNHAYFrJt4vU2G3D4T8ubwvbW
		[
			0xa4, 0x04, 0x94, 0x88, 0x73, 0xbe, 0x26, 0x09, 0x8c, 0xc9, 0xd1, 0xac, 0xb1, 0xf0,
			0x8e, 0x34, 0xbc, 0x4f, 0x5e, 0x7a, 0x40, 0xca, 0x0a, 0xec, 0x87, 0xd2, 0x0b, 0x18,
			0xa5, 0xd8, 0x51, 0x58,
		],
		// Validator_12: 5CiAWLRtzhHnv6Ph18fx2Y7zDVG1iBAqrqVF62mTsTs1mhiS
		[
			0x1c, 0x91, 0xca, 0x39, 0xe8, 0x50, 0x56, 0x16, 0xa9, 0xae, 0xb5, 0xd0, 0xd2, 0x8f,
			0x1e, 0x65, 0x68, 0xbe, 0x59, 0x5a, 0xae, 0x65, 0x17, 0x87, 0xe1, 0x74, 0xf0, 0x64,
			0x90, 0x07, 0xd0, 0x64,
		],
		// Validator_13: 5C7tXxNJa9t1oKYVEJCPbpJRnsNjCsr3qUE5WSqRPSXNx1uY
		[
			0x02, 0x6d, 0xad, 0xed, 0x15, 0x8e, 0xb1, 0xba, 0xe7, 0xc1, 0x22, 0x18, 0x86, 0x94,
			0x0c, 0x95, 0x98, 0x34, 0x08, 0xb1, 0x44, 0x65, 0xa1, 0x5a, 0xa4, 0xf3, 0x37, 0x86,
			0xfe, 0xe4, 0x3d, 0x35,
		],
		// Validator_14: 5G9XFUFuZdUWuRmbfySNFYUi8tGWQCxm6J4q48TKy3bLJjbA
		[
			0xb4, 0x9d, 0x74, 0x77, 0xdc, 0x29, 0xd3, 0xf4, 0x24, 0x20, 0x10, 0x14, 0xaa, 0x0b,
			0x84, 0xdf, 0x0f, 0x68, 0x60, 0x2e, 0xa9, 0xa1, 0x34, 0x76, 0x85, 0xf6, 0x90, 0x20,
			0x8d, 0xb9, 0xfc, 0x5f,
		],
		// Validator_15: 5G9kpHYkZqnpoeyp3SWKsjzGnbxg5oAGUJTLYzoXgVrPZJ2K
		[
			0xb4, 0xcb, 0x1f, 0x17, 0x20, 0xf4, 0x8a, 0x35, 0x08, 0xbb, 0xab, 0x35, 0x02, 0x15,
			0xb7, 0x86, 0x01, 0xfc, 0x03, 0x28, 0x0c, 0xa6, 0xb1, 0xf1, 0x1a, 0x3a, 0xd2, 0xf8,
			0x21, 0xa5, 0xb5, 0x4f,
		],
		// Validator_16: 5GeP9GjAJwAArgzBnJxqAD2RfK33DnxHDsao7UKoBpsV7iRQ
		[
			0xca, 0xa0, 0x48, 0x10, 0xa3, 0x7d, 0x6b, 0x26, 0xef, 0x7d, 0x6b, 0x9a, 0xef, 0x71,
			0xac, 0x0a, 0xaa, 0x81, 0xdf, 0x9c, 0xf8, 0x66, 0x53, 0x30, 0x1f, 0x62, 0xdd, 0xf5,
			0xbd, 0x04, 0xaa, 0x7c,
		],
		// Validator_17: 5G8zDxQRUwAgF16wRbJuZF9QuvYLbLE6hNHemt3FaLvU2J7o
		[
			0xb4, 0x35, 0x03, 0x01, 0x48, 0xbc, 0xc7, 0x5a, 0x24, 0xfd, 0x81, 0x76, 0xfc, 0x42,
			0xb4, 0x35, 0x70, 0x37, 0xe3, 0x86, 0x8f, 0x0d, 0x7a, 0x2b, 0x04, 0x65, 0x84, 0x26,
			0xe8, 0x3d, 0x80, 0x16,
		],
		// Validator_18: 5Gs1YvzVN2u95rjbFDxFdmF8BvpvWeWgEcGi4p46Mxca8G24
		[
			0xd4, 0x41, 0xc9, 0x4e, 0xfc, 0x63, 0xd8, 0x5b, 0x36, 0xe5, 0xe5, 0x26, 0xcd, 0xf4,
			0x56, 0x58, 0xf9, 0xa1, 0xa4, 0x9e, 0xf9, 0x5d, 0x5f, 0xb1, 0x1b, 0x3e, 0x3a, 0xc8,
			0x81, 0xc3, 0x8e, 0x09,
		],
		// Validator_19: 5Fjt81qbrvkskeFbCgUvQ3kyYA7BjKSsghXQS5VT98ctxsdN
		[
			0xa2, 0x95, 0xd3, 0x28, 0x75, 0x75, 0x81, 0x5c, 0x83, 0xc1, 0xde, 0x28, 0xbe, 0x46,
			0xdd, 0x6b, 0x4c, 0x8e, 0xd5, 0x28, 0xc8, 0x4f, 0xd7, 0x8f, 0xa2, 0x93, 0x5d, 0xa4,
			0xd2, 0xdd, 0x66, 0x07,
		],
		// Validator_20: 5D5YRetQTSwEZwvLP6fZSxKwYkMYWvjtPqmfgm48suSjHQZt
		[
			0x2c, 0xdf, 0xba, 0xf5, 0xa4, 0xaf, 0xd2, 0xb5, 0xc2, 0xbf, 0x2d, 0x7d, 0x21, 0xc6,
			0xa0, 0x0c, 0xc4, 0xe9, 0x03, 0xb6, 0x5c, 0x20, 0x6f, 0x26, 0xa1, 0x06, 0xb6, 0x98,
			0xf4, 0xc0, 0x7a, 0x07,
		],
		// Validator_21: 5CcXoQJCfPZuTFq2iZG1RimSPs2xJHRMcjApTZHJphBMN5gU
		[
			0x18, 0x46, 0x00, 0x17, 0xf6, 0x48, 0xfc, 0x75, 0x48, 0x27, 0x1f, 0x44, 0xbf, 0x43,
			0x0b, 0xcf, 0x34, 0xa1, 0x3c, 0x5d, 0x11, 0xc2, 0xc7, 0x97, 0x3e, 0x58, 0x72, 0xfc,
			0x7f, 0x1b, 0xb6, 0x79,
		],
		// Validator_22: 5GndGGg7A1G1tVLjbZcY9M5KCCLpp7UfW4JUNjFnE6Fh47dX
		[
			0xd0, 0xe9, 0xc7, 0x77, 0x56, 0xf7, 0x75, 0xab, 0xa3, 0x82, 0x99, 0x20, 0xbe, 0x3b,
			0xc3, 0xba, 0x9d, 0xcc, 0x28, 0x12, 0x2d, 0x48, 0xa7, 0x78, 0x30, 0xff, 0xd3, 0x6b,
			0x73, 0xa8, 0x6a, 0x2e,
		],
		// Validator_23: 5EHCp5x4QPDUMgGp5aWZDJ4JvEKiwS4PJZJUa4Zodc7xHQBB
		[
			0x62, 0x00, 0xe2, 0x72, 0xfd, 0x9d, 0xda, 0xb0, 0x4f, 0xbd, 0x37, 0x56, 0x57, 0x46,
			0xe3, 0x42, 0xaa, 0x45, 0x17, 0xd4, 0x32, 0xbd, 0xb8, 0x1b, 0x2f, 0x74, 0xd3, 0x25,
			0xb5, 0x74, 0x55, 0x08,
		],
		// Validator_24: 5CnqLUZLZ4khcGx6FmWPuqMBRMC8GAkF6EWjNYT5MS2jFcTs
		[
			0x20, 0x21, 0x7c, 0x29, 0x34, 0xce, 0x0f, 0x44, 0x85, 0xd9, 0xb2, 0x93, 0xa4, 0x32,
			0xa4, 0xbe, 0x12, 0xde, 0xae, 0xaf, 0x24, 0xf5, 0x89, 0x9b, 0xea, 0x97, 0x38, 0x90,
			0x7c, 0x88, 0x5a, 0x29,
		],
		// Validator_25: 5FnxvFT2huTXQRLEBY83ebSJc6Lf8CxQoUf1fDRpRNphEqhv
		[
			0xa4, 0xef, 0xb5, 0xae, 0x7e, 0x9c, 0xef, 0xcd, 0x23, 0x14, 0x26, 0xb0, 0x93, 0x76,
			0x84, 0x64, 0x56, 0x94, 0x55, 0xd3, 0x7b, 0xe7, 0x68, 0x44, 0x41, 0x0a, 0x41, 0x0c,
			0x34, 0x9d, 0x6a, 0x1d,
		],
		// Validator_26: 5FWoqakZ1Bc4s1X5QhM72wWhx4tUiXWU6D1tkQDKAjb9sXNv
		[
			0x98, 0x9d, 0x37, 0x34, 0x35, 0x59, 0x30, 0x82, 0xd1, 0x5e, 0xb0, 0x2a, 0x1b, 0xca,
			0x8c, 0xb5, 0x7a, 0x26, 0xbc, 0xad, 0x35, 0x59, 0x19, 0x3f, 0x21, 0x98, 0x59, 0x91,
			0x25, 0x24, 0x0e, 0x12,
		],
		// Validator_27: 5GHk3DkZaK63UQ4UGGQDJHYDuXV6DdeKVRbQsMybaQrirabD
		[
			0xba, 0xe2, 0x78, 0x16, 0xcf, 0x59, 0xab, 0xdb, 0x82, 0x4d, 0xf0, 0x99, 0x2f, 0x28,
			0x42, 0x6b, 0xfc, 0xab, 0x1a, 0x51, 0xa7, 0xc4, 0xa1, 0xf2, 0x87, 0x7d, 0x07, 0xb6,
			0x37, 0x7c, 0xc4, 0x44,
		],
	];
}

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
