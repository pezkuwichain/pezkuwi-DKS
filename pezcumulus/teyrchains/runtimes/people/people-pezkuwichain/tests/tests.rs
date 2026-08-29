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

use people_pezkuwichain_runtime::{
	xcm_config::{GovernanceLocation, LocationToAccountId},
	Block, Runtime, RuntimeCall, RuntimeOrigin,
};
use pezframe_support::{assert_err, assert_ok};
use pezsp_core::crypto::Ss58Codec;
use pezsp_runtime::Either;
use testnet_teyrchains_constants::pezkuwichain::fee::WeightToFee;
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

/// Who may tell this chain to accept a new runtime.
///
/// The mainnet twin of the test people-zagros carries. It was missing here entirely, which is
/// how a difference between the two chains' XCM barriers could have gone unnoticed: the twins
/// are supposed to answer this identically, and only one of them was ever asked.
///
/// The answer is the relay chain and its governance, and nothing else. Asset Hub holds the
/// state's money but not its authority -- every power it exercises arrives as an instruction
/// from People, never the reverse -- so a message from there is refused at the barrier.
#[test]
fn governance_authorize_upgrade_works() {
	use pezkuwichain_runtime_constants::system_teyrchain::ASSET_HUB_ID;

	// no - a teyrchain nobody granted anything to
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Teyrchain(12334)))),
		Either::Right(InstructionError { index: 0, error: XcmError::Barrier })
	);

	// no - Asset Hub. Upstream allows this because its governance has moved there; ours has
	// not, and `GovernanceLocation` below is the proof.
	//
	// Refused at the origin rather than at the barrier, and the difference is deliberate. The
	// rewards pallet here is funded from the Asset Hub, so the barrier has to let a message
	// from there arrive at all -- it decides who may speak, not what they may say. What they
	// may say is the origin converter's question, and it converts a sibling into nothing
	// privileged. Two doors rather than one, and the second is the one that belongs here:
	// were this asserted at the barrier, funding could not reach us either.
	assert_err!(
		teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
			Runtime,
			RuntimeOrigin,
		>(GovernanceOrigin::Location(Location::new(1, Teyrchain(ASSET_HUB_ID)))),
		Either::Right(InstructionError { index: 1, error: XcmError::BadOrigin })
	);

	// ok - the relay chain
	assert_ok!(teyrchains_runtimes_test_utils::test_cases::can_governance_authorize_upgrade::<
		Runtime,
		RuntimeOrigin,
	>(GovernanceOrigin::Location(Location::parent())));

	// ok - the governance location this runtime declares
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
	use people_pezkuwichain_runtime::{DiwanCollective, Welati};
	use pezsp_runtime::BuildStorage;

	pub(crate) const FOUNDER: [u8; 32] = [9u8; 32];
	const JURIST: [u8; 32] = [10u8; 32];
	const ENGINEER: [u8; 32] = [11u8; 32];

	pub(crate) fn account(raw: [u8; 32]) -> AccountId {
		raw.into()
	}

	/// Genesis with a citizen register that exists and two people qualified for the bench.
	pub(crate) fn new_test_ext() -> pezsp_io::TestExternalities {
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

	/// Seat a judge the way the state does: the President's own signature, and nothing else.
	///
	/// Root stands in for the President while sudo lives. These tests are about whether the
	/// two registers stay one list, not about who signed.
	fn seat_judge(who: AccountId) -> pezsp_runtime::DispatchResult {
		Welati::appoint_diwan_member(RuntimeOrigin::root(), who.into())
	}

	#[test]
	fn seating_a_judge_puts_them_on_both_registers() {
		new_test_ext().execute_with(|| {
			assert!(bench().is_empty());
			assert!(collective().is_empty());

			assert_ok!(seat_judge(account(JURIST)));

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
			assert_ok!(seat_judge(account(JURIST)));
			assert_ok!(seat_judge(account(ENGINEER)));

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
			assert_ok!(seat_judge(account(JURIST)));

			// The founder is a citizen and nothing more, so the qualification gate refuses.
			assert!(seat_judge(account(FOUNDER)).is_err());

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
				assert_ok!(seat_judge(account(who)));
			}
			assert_eq!(collective().len(), 2);
		});
	}
}

/// No message from another chain may write the register.
///
/// `ParentAsSuperuser` hands the relay Root here, and Root is what every register call asks
/// for -- so the relay's token-weighted electorate could revoke a citizenship. The rule is a
/// call filter rather than a narrower origin because FRAME's Root bypasses origin filters.
mod the_register_is_not_writable_from_abroad {
	use super::*;
	use people_pezkuwichain_runtime::xcm_config::TheRegisterIsNotWritableFromAbroad as Filter;
	use pezframe_support::traits::Contains;

	#[test]
	fn who_is_a_person_who_holds_office_and_who_sits_are_all_refused() {
		let calls = vec![
			RuntimeCall::IdentityKyc(pezpallet_identity_kyc::Call::revoke_citizenship {
				who: ALICE.into(),
			}),
			RuntimeCall::Welati(pezpallet_welati::Call::confirm_prime_minister {}),
			RuntimeCall::Tiki(pezpallet_tiki::Call::grant_honorary_citizenship {
				dest: AccountId::from(ALICE).into(),
			}),
		];

		for call in calls {
			assert!(!Filter::contains(&call), "an off-chain message reached the register");
		}
	}

	#[test]
	fn the_relay_keeps_this_chains_code() {
		let upgrade = RuntimeCall::System(pezframe_system::Call::authorize_upgrade {
			code_hash: Default::default(),
		});
		assert!(Filter::contains(&upgrade), "the upgrade path must stay open");
	}
}

/// The register's scores are read locally, and the oracle's is the only one that can be stale.
///
/// This is the measurement behind putting TNPoS on this chain rather than the relay. Four of
/// the five scores are computed here from state written here, so they are current by
/// construction. The fifth comes from off-chain and carries the observation height the oracle
/// itself reported -- which is what lets `value_if_fresh` turn a stalled oracle into an absent
/// score instead of a confident old one.
///
/// A relay-side TNPoS would have had all five arrive over XCM, and the failure that matters is
/// not all five going stale at once -- that seats nobody and leaves the authorities alone. It
/// is three of five going stale, which seats a committee whose stratum proportions are wrong
/// while storage still says they are right.
mod register_scores {
	use super::*;
	// Borrowed rather than duplicated: two `new_test_ext`s in one file drift, and the second
	// one is always the one that stops matching the runtime.
	use crate::the_court_roster::{account, new_test_ext, FOUNDER};
	use people_pezkuwichain_runtime::people::RegisterScores;
	use pezkuwi_tnpos_primitives::scores::ScoreProvider;

	#[test]
	fn four_scores_are_current_and_the_oracle_carries_its_own_height() {
		new_test_ext().execute_with(|| {
			let who: AccountId = account(FOUNDER);
			pezframe_system::Pezpallet::<Runtime>::set_block_number(500);

			// Computed on this chain, so their height is this block -- not a remembered one.
			for got in [
				RegisterScores::trust_of(&who),
				RegisterScores::tiki_of(&who),
				RegisterScores::perwerde_of(&who),
				RegisterScores::referral_of(&who),
			] {
				assert_eq!(got.last_updated, 500, "a local score claimed a height it did not have");
				assert!(got.value_if_fresh(500, 1).is_some(), "a local score read as stale");
			}

			// The oracle carries its own height: nothing observed means height zero, not this
			// block. Once the chain is older than the window that reads as absent, which is
			// what the type exists for -- a stalled oracle must not hand back a confident old
			// number.
			let staking = RegisterScores::staking_of(&who);
			assert_eq!(staking.last_updated, 0, "the oracle claimed a height it never reported");
			assert!(
				staking.value_if_fresh(500, 4 * 600).is_some(),
				"before the window has elapsed nothing can be stale -- staleness is measured \
				 as an absolute distance, and at block 500 no distance exceeds 2400"
			);

			pezframe_system::Pezpallet::<Runtime>::set_block_number(100_000);
			assert!(
				RegisterScores::staking_of(&who).value_if_fresh(100_000, 4 * 600).is_none(),
				"an unobserved stake must read absent once the window has passed"
			);

			// And the four local ones follow the block, so they never go stale by sitting.
			assert_eq!(RegisterScores::trust_of(&who).last_updated, 100_000);
		});
	}
}

/// The register's rules moved from the binary to storage. Two things must stay true.
mod register_parameters {
	use super::*;
	use pezframe_support::{assert_noop, traits::Get};
	use pezsp_runtime::{traits::BadOrigin, BuildStorage};
	use testnet_teyrchains_constants::pezkuwichain::time::DAYS;

	fn new_test_ext() -> pezsp_io::TestExternalities {
		pezframe_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into()
	}

	/// Genesis behaves exactly as it did when these were `const`.
	///
	/// A default that drifted from the constant it replaced would change the register's growth
	/// rules on the day of the move, and nothing else in the tree would say so.
	#[test]
	fn the_defaults_are_what_the_constants_were() {
		use people_pezkuwichain_runtime::dynamic_params::qeyd;

		new_test_ext().execute_with(|| {
			assert_eq!(qeyd::VouchingWaitingPeriod::get(), DAYS);
			assert_eq!(qeyd::InitialVouchingCapacity::get(), 5);
			assert_eq!(qeyd::SettledVouchesPerPlace::get(), 3);
			assert_eq!(qeyd::MaxVouchingCapacity::get(), 50);
			assert_eq!(qeyd::SuspensionRevocationFloor::get(), 3);
			assert_eq!(qeyd::SuspensionRevocationPercent::get(), 20);
			assert_eq!(qeyd::PenaltyPerRevocation::get(), 10);
		});
	}

	/// Moving them to storage must not have made them easier to change than a runtime upgrade.
	///
	/// This is the whole reason `AdminOrigin` is Root and not the court: a body that both
	/// writes the register's entries and sets the rules for writing them has no rule above it.
	/// When the register-rules track lands, this test is what has to be edited deliberately.
	#[test]
	fn only_root_turns_them() {
		use people_pezkuwichain_runtime::{dynamic_params::qeyd, Parameters, RuntimeParameters};

		let widen = || {
			RuntimeParameters::Qeyd(qeyd::Parameters::MaxVouchingCapacity(
				qeyd::MaxVouchingCapacity,
				Some(50_000),
			))
		};

		new_test_ext().execute_with(|| {
			// The court writes the register; it does not write the register's rules.
			assert_noop!(
				Parameters::set_parameter(
					RuntimeOrigin::signed(AccountId::from([1u8; 32])),
					widen()
				),
				BadOrigin
			);
			assert_ok!(Parameters::set_parameter(RuntimeOrigin::root(), widen()));
			assert_eq!(qeyd::MaxVouchingCapacity::get(), 50_000);
		});
	}
}
