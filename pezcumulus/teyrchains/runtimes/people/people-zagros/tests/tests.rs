// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

#![cfg(test)]

use people_zagros_runtime::{
	xcm_config::{GovernanceLocation, LocationToAccountId},
	Block, Runtime, RuntimeCall, RuntimeOrigin,
};
use pezframe_support::{assert_err, assert_ok};
use pezsp_core::crypto::Ss58Codec;
use pezsp_runtime::Either;
use testnet_teyrchains_constants::zagros::fee::WeightToFee;
use teyrchains_common::AccountId;
use teyrchains_runtimes_test_utils::GovernanceOrigin;
use xcm::latest::prelude::*;
use xcm_runtime_pezapis::conversions::LocationToAccountHelper;

const ALICE: [u8; 32] = [1u8; 32];

#[test]
fn location_conversion_works() {
	// the purpose of hardcoded values is to catch an unintended location conversion logic change.
	struct TestCase {
		description: &'static str,
		location: Location,
		expected_account_id_str: &'static str,
	}

	let test_cases = vec![
		// DescribeTerminus
		TestCase {
			description: "DescribeTerminus Parent",
			location: Location::new(1, Here),
			expected_account_id_str: "5Dt6dpkWPwLaH4BBCKJwjiWrFVAGyYk3tLUabvyn4v7KtESG",
		},
		TestCase {
			description: "DescribeTerminus Sibling",
			location: Location::new(1, [Teyrchain(1111)]),
			expected_account_id_str: "5Eg2fnssmmJnF3z1iZ1NouAuzciDaaDQH7qURAy3w15jULDk",
		},
		// DescribePalletTerminal
		TestCase {
			description: "DescribePalletTerminal Parent",
			location: Location::new(1, [PalletInstance(50)]),
			expected_account_id_str: "5CnwemvaAXkWFVwibiCvf2EjqwiqBi29S5cLLydZLEaEw6jZ",
		},
		TestCase {
			description: "DescribePalletTerminal Sibling",
			location: Location::new(1, [Teyrchain(1111), PalletInstance(50)]),
			expected_account_id_str: "5GFBgPjpEQPdaxEnFirUoa51u5erVx84twYxJVuBRAT2UP2g",
		},
		// DescribeAccountId32Terminal
		TestCase {
			description: "DescribeAccountId32Terminal Parent",
			location: Location::new(
				1,
				[Junction::AccountId32 { network: None, id: AccountId::from(ALICE).into() }],
			),
			expected_account_id_str: "5DN5SGsuUG7PAqFL47J9meViwdnk9AdeSWKFkcHC45hEzVz4",
		},
		TestCase {
			description: "DescribeAccountId32Terminal Sibling",
			location: Location::new(
				1,
				[
					Teyrchain(1111),
					Junction::AccountId32 { network: None, id: AccountId::from(ALICE).into() },
				],
			),
			expected_account_id_str: "5DGRXLYwWGce7wvm14vX1Ms4Vf118FSWQbJkyQigY2pfm6bg",
		},
		// DescribeAccountKey20Terminal
		TestCase {
			description: "DescribeAccountKey20Terminal Parent",
			location: Location::new(1, [AccountKey20 { network: None, key: [0u8; 20] }]),
			expected_account_id_str: "5F5Ec11567pa919wJkX6VHtv2ZXS5W698YCW35EdEbrg14cg",
		},
		TestCase {
			description: "DescribeAccountKey20Terminal Sibling",
			location: Location::new(
				1,
				[Teyrchain(1111), AccountKey20 { network: None, key: [0u8; 20] }],
			),
			expected_account_id_str: "5CB2FbUds2qvcJNhDiTbRZwiS3trAy6ydFGMSVutmYijpPAg",
		},
		// DescribeTreasuryVoiceTerminal
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Treasury, part: BodyPart::Voice }]),
			expected_account_id_str: "5CUjnE2vgcUCuhxPwFoQ5r7p1DkhujgvMNDHaF2bLqRp4D5F",
		},
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Sibling",
			location: Location::new(
				1,
				[Teyrchain(1111), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5G6TDwaVgbWmhqRUKjBhRRnH4ry9L9cjRymUEmiRsLbSE4gB",
		},
		// DescribeBodyTerminal
		TestCase {
			description: "DescribeBodyTerminal Parent",
			location: Location::new(1, [Plurality { id: BodyId::Unit, part: BodyPart::Voice }]),
			expected_account_id_str: "5EBRMTBkDisEXsaN283SRbzx9Xf2PXwUxxFCJohSGo4jYe6B",
		},
		TestCase {
			description: "DescribeBodyTerminal Sibling",
			location: Location::new(
				1,
				[Teyrchain(1111), Plurality { id: BodyId::Unit, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5DBoExvojy8tYnHgLL97phNH975CyT45PWTZEeGoBZfAyRMH",
		},
	];

	for tc in test_cases {
		let expected =
			AccountId::from_string(tc.expected_account_id_str).expect("Invalid AccountId string");

		let got = LocationToAccountHelper::<AccountId, LocationToAccountId>::convert_location(
			tc.location.into(),
		)
		.unwrap();

		assert_eq!(got, expected, "{}", tc.description);
	}
}

#[test]
fn xcm_payment_api_works() {
	teyrchains_runtimes_test_utils::test_cases::xcm_payment_api_with_native_token_works::<
		Runtime,
		RuntimeCall,
		RuntimeOrigin,
		Block,
		WeightToFee,
	>();
}

#[test]
fn governance_authorize_upgrade_works() {
	use zagros_runtime_constants::system_teyrchain::{ASSET_HUB_ID, COLLECTIVES_ID};

	// no - random para
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Teyrchain(12334)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - AssetHub
	//
	// Upstream expects this to pass, because upstream's governance has moved to its Asset
	// Hub. Ours has not: `GovernanceLocation` is the relay on both People runtimes. Asset Hub
	// holds the money, not the authority -- every power it exercises arrives as an
	// instruction from People, never the reverse.
	//
	// Refused at the origin rather than at the barrier. The rewards pallet here is funded
	// from the Asset Hub, so a message from there has to be able to arrive: the barrier
	// decides who may speak and the origin converter decides what they may say. Asserting
	// this at the barrier would mean funding could not reach us either.
	//
	// If that ever changes, this is one of the places that has to change with it.
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Teyrchain(ASSET_HUB_ID)))),
		Either::Right(InstructionError { index: 1, error: XcmError::BadOrigin })
	);
	// no - Collectives
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Teyrchain(COLLECTIVES_ID)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);
	// no - Collectives Voice of Fellows plurality
	//
	// Refused at the barrier (instruction 0) rather than at the origin check (instruction 2).
	// Upstream lets a sibling's message through and turns it away when it asks to Transact;
	// ours never lets it in. The stricter answer is the same answer, reached sooner.
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::LocationAndDescendOrigin(
			Location::new(1, Teyrchain(COLLECTIVES_ID)),
			Plurality { id: BodyId::Technical, part: BodyPart::Voice }.into()
		)),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);

	// ok - relaychain
	assert_ok!(teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(Location::parent())));

	// ok - governance location
	assert_ok!(teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(GovernanceLocation::get())));
}

// =============================================================================
// THE COURT'S TWO REGISTERS
// =============================================================================

/// welati decides who sits on the Diwan; the collective is what lets them decide as a court.
/// That is two places holding the same fact, and the only thing keeping them together is that
/// welati writes both. This is the test of that -- at the runtime, because the pallet's own
/// mock has nowhere to relay to.
///
/// It matters because the two halves fail differently. A collective that keeps somebody
/// welati removed is a judge who still votes; one that misses somebody welati seated is a
/// judge who cannot.
mod the_court_roster {
	use super::*;
	use people_zagros_runtime::{DiwanCollective, Welati};
	use pezframe_support::assert_ok;
	use pezsp_runtime::BuildStorage;

	const FOUNDER: [u8; 32] = [9u8; 32];
	const JURIST: [u8; 32] = [10u8; 32];
	const ENGINEER: [u8; 32] = [11u8; 32];

	fn account(raw: [u8; 32]) -> AccountId {
		raw.into()
	}

	/// Genesis with a citizen register that exists and two people qualified for the bench.
	fn new_test_ext() -> pezsp_io::TestExternalities {
		let mut t = pezframe_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

		pezpallet_tiki::GenesisConfig::<Runtime> {
			founding_citizen: Some(account(FOUNDER)),
			founding_government: vec![
				(account(JURIST), pezpallet_tiki::Tiki::Hiquqnas),
				(account(ENGINEER), pezpallet_tiki::Tiki::Bernamenivîs),
			],
			..Default::default()
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = pezsp_io::TestExternalities::new(t);
		ext.execute_with(|| pezframe_system::Pezpallet::<Runtime>::set_block_number(1));
		ext
	}

	fn bench() -> Vec<AccountId> {
		Welati::diwan_members().iter().map(|member| member.account.clone()).collect()
	}

	fn collective() -> Vec<AccountId> {
		pezpallet_collective::Members::<Runtime, DiwanCollective>::get()
	}

	#[test]
	fn seating_a_judge_puts_them_on_both_registers() {
		new_test_ext().execute_with(|| {
			assert!(bench().is_empty());
			assert!(collective().is_empty());

			assert_ok!(Welati::appoint_diwan_member(RuntimeOrigin::root(), account(JURIST).into()));

			assert_eq!(bench(), vec![account(JURIST)]);
			assert_eq!(
				collective(),
				vec![account(JURIST)],
				"a judge welati seated who cannot vote is not on the court"
			);
		});
	}

	#[test]
	fn the_two_registers_stay_together_as_the_bench_grows() {
		new_test_ext().execute_with(|| {
			assert_ok!(Welati::appoint_diwan_member(RuntimeOrigin::root(), account(JURIST).into()));
			assert_ok!(Welati::appoint_diwan_member(
				RuntimeOrigin::root(),
				account(ENGINEER).into()
			));

			let mut seated = bench();
			let mut voting = collective();
			seated.sort();
			voting.sort();

			assert_eq!(seated.len(), 2);
			assert_eq!(seated, voting, "the bench and the body that votes it must be one list");
		});
	}

	#[test]
	fn a_refused_appointment_leaves_neither_register_touched() {
		// The failure that would be hardest to see: welati rejects the nominee but the
		// collective was already told. Then somebody nobody appointed can vote on the court.
		new_test_ext().execute_with(|| {
			assert_ok!(Welati::appoint_diwan_member(RuntimeOrigin::root(), account(JURIST).into()));

			// The founder is a citizen and nothing more, so the qualification gate refuses.
			assert!(Welati::appoint_diwan_member(RuntimeOrigin::root(), account(FOUNDER).into())
				.is_err());

			assert_eq!(bench(), vec![account(JURIST)]);
			assert_eq!(collective(), vec![account(JURIST)]);
		});
	}

	/// Kept honest about what the qualification pool actually is: a jurist and an engineer
	/// both belong on this bench, and a citizen with neither does not.
	#[test]
	fn the_pool_admits_law_and_the_chain_alike() {
		new_test_ext().execute_with(|| {
			for who in [JURIST, ENGINEER] {
				assert_ok!(Welati::appoint_diwan_member(
					RuntimeOrigin::root(),
					account(who).into()
				));
			}
			assert_eq!(collective().len(), 2);
		});
	}
}
