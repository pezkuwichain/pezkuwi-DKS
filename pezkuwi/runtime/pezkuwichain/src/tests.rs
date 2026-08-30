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
// along with Pezkuwi. If not, see <http://www.gnu.org/licenses/>.

//! Tests for the Pezkuwichain Runtime Configuration

use crate::*;
use std::collections::HashSet;

use crate::xcm_config::LocationConverter;
use pezframe_support::traits::WhitelistedStorageKeys;
use pezsp_core::{crypto::Ss58Codec, hexdisplay::HexDisplay};
use pezsp_keyring::Sr25519Keyring::Alice;
use xcm_runtime_pezapis::conversions::LocationToAccountHelper;

#[test]
fn check_whitelist() {
	let whitelist: HashSet<String> = AllPalletsWithSystem::whitelisted_storage_keys()
		.iter()
		.map(|e| HexDisplay::from(&e.key).to_string())
		.collect();

	// Block number
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac"));
	// Total issuance
	assert!(whitelist.contains("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80"));
	// Execution phase
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a"));
	// Event count
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850"));
	// System events
	assert!(whitelist.contains("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7"));
	// XcmPallet VersionDiscoveryQueue
	assert!(whitelist.contains("1405f2411d0af5a7ff397e7c9dc68d194a222ba0333561192e474c59ed8e30e1"));
	// XcmPallet SafeXcmVersion
	assert!(whitelist.contains("1405f2411d0af5a7ff397e7c9dc68d196323ae84c43568be0d1394d5d0d522c4"));
}

#[test]
fn retired_indices_stay_retired() {
	// `Treasury` held 18 and `Council` 17 until the treasury moved to the Asset Hub. An index
	// is part of the composite `RuntimeCall` and `RuntimeEvent` encodings, so giving either
	// number to a new pallet makes old bytes decode as that pallet.
	use pezframe_support::traits::PalletsInfoAccess;
	let taken: Vec<usize> = <AllPalletsWithSystem as PalletsInfoAccess>::infos()
		.iter()
		.map(|i| i.index)
		.collect();
	for index in [17usize, 18] {
		assert!(!taken.contains(&index), "index {index} is retired and was handed to a pallet");
	}
}

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
			description: "DescribeTerminus Child",
			location: Location::new(0, [Teyrchain(1111)]),
			expected_account_id_str: "5Ec4AhP4h37t7TFsAZ4HhFq6k92usAAJDUC3ADSZ4H4Acru3",
		},
		// DescribePalletTerminal
		TestCase {
			description: "DescribePalletTerminal Child",
			location: Location::new(0, [Teyrchain(1111), PalletInstance(50)]),
			expected_account_id_str: "5FjEBrKn3STAFsZpQF4jzwxUYHNGnNgzdZqSQfTzeJ82XKp6",
		},
		// DescribeAccountId32Terminal
		TestCase {
			description: "DescribeAccountId32Terminal Child",
			location: Location::new(
				0,
				[Teyrchain(1111), AccountId32 { network: None, id: AccountId::from(Alice).into() }],
			),
			expected_account_id_str: "5EEMro9RRDpne4jn9TuD7cTB6Amv1raVZ3xspSkqb2BF3FJH",
		},
		// DescribeAccountKey20Terminal
		TestCase {
			description: "DescribeAccountKey20Terminal Child",
			location: Location::new(
				0,
				[Teyrchain(1111), AccountKey20 { network: None, key: [0u8; 20] }],
			),
			expected_account_id_str: "5HohjXdjs6afcYcgHHSstkrtGfxgfGKsnZ1jtewBpFiGu4DL",
		},
		// DescribeTreasuryVoiceTerminal
		TestCase {
			description: "DescribeTreasuryVoiceTerminal Child",
			location: Location::new(
				0,
				[Teyrchain(1111), Plurality { id: BodyId::Treasury, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5GenE4vJgHvwYVcD6b4nBvH5HNY4pzpVHWoqwFpNMFT7a2oX",
		},
		// DescribeBodyTerminal
		TestCase {
			description: "DescribeBodyTerminal Child",
			location: Location::new(
				0,
				[Teyrchain(1111), Plurality { id: BodyId::Unit, part: BodyPart::Voice }],
			),
			expected_account_id_str: "5DPgGBFTTYm1dGbtB1VWHJ3T3ScvdrskGGx6vSJZNP1WNStV",
		},
	];

	for tc in test_cases {
		let expected =
			AccountId::from_string(tc.expected_account_id_str).expect("Invalid AccountId string");

		let got = LocationToAccountHelper::<AccountId, LocationConverter>::convert_location(
			tc.location.into(),
		)
		.unwrap();

		assert_eq!(got, expected, "{}", tc.description);
	}
}

// =============================================================================
// OpenGov Track Configuration Tests
// =============================================================================

use governance::TracksInfo;
use pezkuwichain_runtime_constants::time::{DAYS, HOURS, MINUTES};
use pezpallet_referenda::TracksInfo as TracksInfoTrait;
use std::collections::HashMap;

#[test]
fn governance_tracks_total_count() {
	let count = <TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::tracks().count();
	// Eight. The treasury moved to the Asset Hub and took its six tracks with it -- the
	// treasurer and the five spenders -- the three register tracks moved to the People chain,
	// and `root` was removed: Root here is the register's referendum arriving over XCM, not a
	// ballot of this chain's holders.
	assert_eq!(count, 8, "Expected 8 relay tracks, got {count}");
}

#[test]
fn governance_track_ids_are_unique() {
	let mut seen = HashSet::new();
	for track in <TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::tracks() {
		assert!(seen.insert(track.id), "Duplicate track ID: {}", track.id);
	}
}

#[test]
fn governance_track_names_are_unique() {
	let mut seen = HashSet::new();
	for track in <TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::tracks() {
		let name = String::from_utf8_lossy(&track.info.name).to_string();
		assert!(seen.insert(name.clone()), "Duplicate track name: {name}");
	}
}

#[test]
fn governance_no_test_periods_remain() {
	// Ensure no track still uses the old test values (< 1 HOURS for decision_period).
	// All production decision periods should be at least 7 DAYS.
	for track in <TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::tracks() {
		let name = String::from_utf8_lossy(&track.info.name).to_string();
		assert!(
			track.info.decision_period >= 7 * DAYS,
			"Track '{name}' (id={}) has decision_period={} blocks, expected >= {} (7 DAYS)",
			track.id,
			track.info.decision_period,
			7 * DAYS
		);
	}
}

#[test]
fn governance_production_periods_match_spec() {
	// Build expected values: (track_id, prepare, decision, confirm, enact)
	let expected: Vec<(u16, &str, BlockNumber, BlockNumber, BlockNumber, BlockNumber)> = vec![
		(1, "whitelisted_caller", 30 * MINUTES, 28 * DAYS, 10 * MINUTES, 10 * MINUTES),
		(10, "staking_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(12, "lease_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(13, "fellowship_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(14, "general_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(15, "auction_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(20, "referendum_canceller", 2 * HOURS, 7 * DAYS, 3 * HOURS, 10 * MINUTES),
		(21, "referendum_killer", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
	];

	let tracks: HashMap<u16, _> = <TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::tracks()
		.map(|t| (t.id, t.into_owned()))
		.collect();

	for (id, name, prepare, decision, confirm, enact) in &expected {
		let track = tracks.get(id).unwrap_or_else(|| panic!("Track id={id} '{name}' not found"));
		let got_name = String::from_utf8_lossy(&track.info.name).trim_end_matches('\0').to_string();
		assert_eq!(&got_name, name, "Track id={id} name mismatch");
		assert_eq!(
			track.info.prepare_period, *prepare,
			"Track '{name}' prepare_period: got={}, expected={prepare}",
			track.info.prepare_period
		);
		assert_eq!(
			track.info.decision_period, *decision,
			"Track '{name}' decision_period: got={}, expected={decision}",
			track.info.decision_period
		);
		assert_eq!(
			track.info.confirm_period, *confirm,
			"Track '{name}' confirm_period: got={}, expected={confirm}",
			track.info.confirm_period
		);
		assert_eq!(
			track.info.min_enactment_period, *enact,
			"Track '{name}' min_enactment_period: got={}, expected={enact}",
			track.info.min_enactment_period
		);
	}

	assert_eq!(expected.len(), tracks.len(), "Track count mismatch");
}

/// Root is not on this chain's ballot.
///
/// Root here reaches every chain in the network -- `System::set_code` locally, and
/// `Paras::force_set_current_code` for each teyrchain. This chain's electorate is holdings, and
/// the root track's support curve asked for none at all by day 28, so an upgrade of the whole
/// network was a large enough position plus four weeks.
///
/// The power did not go away, it changed hands: `StateRegisterAsRoot` converts a `Superuser`
/// message from the People chain into Root here, and that chain's tally counts citizens one
/// each. The constitution is decided by the people and enacted by the relay.
#[test]
fn root_is_not_reachable_from_this_chains_ballot() {
	use governance::pezpallet_custom_origins::Origin;

	let tracks: HashMap<u16, _> = <TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::tracks()
		.map(|t| (t.id, t.into_owned()))
		.collect();

	assert!(!tracks.contains_key(&0), "the root track is back on a holdings ballot");
	for track in tracks.values() {
		let name = String::from_utf8_lossy(&track.info.name).to_string();
		assert_ne!(name, "root", "a track named root is a track that upgrades the network");
	}

	// And no system origin maps to a track, so none can start a referendum here.
	let root: <RuntimeOrigin as pezframe_support::traits::OriginTrait>::PalletsOrigin =
		pezframe_system::RawOrigin::Root.into();
	assert!(<TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::track_for(&root).is_err());

	// The custom origins that remain are this chain's own business -- none of them is Root by
	// another name, and each still has a track to run on.
	for origin in [
		Origin::StakingAdmin,
		Origin::LeaseAdmin,
		Origin::FellowshipAdmin,
		Origin::GeneralAdmin,
		Origin::AuctionAdmin,
		Origin::ReferendumCanceller,
		Origin::ReferendumKiller,
		Origin::WhitelistedCaller,
	] {
		let pallets_origin: <RuntimeOrigin as pezframe_support::traits::OriginTrait>::PalletsOrigin =
			origin.into();
		assert!(
			<TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::track_for(&pallets_origin)
				.is_ok(),
			"an origin with no track cannot be dispatched at all"
		);
	}
}

/// The register is not reachable from this chain's referenda.
///
/// Tracks 40, 41 and 42 -- `welati_election`, `welati_admin`, `citizenship_admin` -- used to
/// sit here, and this test used to assert they existed. This chain's referenda weigh tokens
/// and conviction, so those three let stake decide who is a person, who holds office and who
/// stops being a citizen. They live on the People chain now, where the tally counts heads.
#[test]
fn the_relay_has_no_track_that_reaches_the_register() {
	let tracks: HashMap<u16, _> = <TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::tracks()
		.map(|t| (t.id, t.into_owned()))
		.collect();

	for id in [40u16, 41, 42] {
		assert!(!tracks.contains_key(&id), "track {id} reaches the register from a token vote");
	}

	for track in tracks.values() {
		let name = String::from_utf8_lossy(&track.info.name).to_string();
		assert!(
			!name.contains("welati") && !name.contains("citizenship"),
			"track '{name}' names a register matter on a token-weighted chain"
		);
	}
}

#[test]
fn governance_decision_periods_are_in_days() {
	// Verify all decision periods are expressed as multiples of DAYS (not minutes)
	for track in <TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::tracks() {
		let name = String::from_utf8_lossy(&track.info.name).to_string();
		let period = track.info.decision_period;
		let days = period / DAYS;
		assert!(
			period == days * DAYS,
			"Track '{name}' decision_period ({period} blocks) is not a whole number of days"
		);
		assert!(
			days >= 7,
			"Track '{name}' decision_period is only {days} days, expected at least 7"
		);
	}
}

#[test]
fn governance_track_for_origin_mapping() {
	use governance::pezpallet_custom_origins::Origin;

	// Test that track_for() correctly maps each origin to its track ID
	let origin_to_track: Vec<(Origin, u16)> = vec![
		(Origin::WhitelistedCaller, 1),
		(Origin::StakingAdmin, 10),
		(Origin::LeaseAdmin, 12),
		(Origin::FellowshipAdmin, 13),
		(Origin::GeneralAdmin, 14),
		(Origin::AuctionAdmin, 15),
		(Origin::ReferendumCanceller, 20),
		(Origin::ReferendumKiller, 21),
	];

	for (origin, expected_id) in origin_to_track {
		let pezpallet_origin: <RuntimeOrigin as pezframe_support::traits::OriginTrait>::PalletsOrigin =
			origin.clone().into();
		let result =
			<TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::track_for(&pezpallet_origin);
		assert_eq!(
			result,
			Ok(expected_id),
			"Origin {:?} should map to track {expected_id}",
			origin
		);
	}
}

#[test]
fn votable_issuance_excludes_the_checking_account() {
	use governance::VotableIssuance;
	use pezframe_support::traits::{
		fungible::{Inspect, Mutate},
		Get,
	};

	pezsp_io::TestExternalities::new_empty().execute_with(|| {
		let check_account = XcmPallet::check_account();
		let voter = Alice.to_account_id();

		// Five million in circulation, five million standing behind the supply that lives on the
		// other chains — the allocation the genesis is built around.
		Balances::mint_into(&voter, 5_000_000 * UNITS).unwrap();
		Balances::mint_into(&check_account, 5_000_000 * UNITS).unwrap();

		assert_eq!(Balances::active_issuance(), 10_000_000 * UNITS);
		assert_eq!(
			VotableIssuance::get(),
			5_000_000 * UNITS,
			"the checking account cannot vote and must not count toward turnout"
		);
	});
}

#[test]
fn fast_track_support_floor_is_measured_against_what_can_vote() {
	use governance::VotableIssuance;
	use pezframe_support::traits::{
		fungible::{Inspect, Mutate},
		Get,
	};
	use pezsp_runtime::Perbill;

	// `SUP_WHITELISTED_CALLER` floors at five percent and never decays below it, so the fast path
	// is only ever usable if that fraction of the turnout denominator can actually be voted. With
	// the checking account counted, the floor is a fraction of supply that includes tokens no one
	// holds; the larger the seed, the further out of reach the emergency route drifts — and it
	// would only be discovered in an emergency.
	pezsp_io::TestExternalities::new_empty().execute_with(|| {
		let check_account = XcmPallet::check_account();
		let voter = Alice.to_account_id();

		Balances::mint_into(&voter, 5_000_000 * UNITS).unwrap();
		Balances::mint_into(&check_account, 5_000_000 * UNITS).unwrap();

		let floor = Perbill::from_percent(5);
		let required = floor * VotableIssuance::get();
		let circulating = Balances::active_issuance() - Balances::balance(&check_account);

		assert_eq!(required, 250_000 * UNITS);
		assert_eq!(
			floor * Balances::active_issuance(),
			500_000 * UNITS,
			"counting the checking account would double what the floor demands"
		);
		assert!(
			required * 2 <= circulating,
			"the floor has to sit well inside what is actually held: {required} of {circulating}"
		);
	});
}

/// The People chain builds the key calls by hand, and nothing fails if the bytes drift.
///
/// It cannot name this runtime's types, so `KeysToRelay` writes the address itself: pallet
/// index 67, then call index 3 or 4. If either moves, the `Transact` stops decoding -- and it
/// stops silently, because the sending side has already taken the registration. This pins the
/// bytes it has to produce, the same way the treasury call is pinned for `welati`.
#[test]
fn the_key_calls_encode_the_way_people_builds_them() {
	use codec::Encode;

	let stash: AccountId = [7u8; 32].into();
	let keys = vec![1u8, 2, 3];

	let real_set = RuntimeCall::StakingAhClient(
		pezpallet_staking_async_ah_client::Call::<Runtime>::set_keys_from_ah {
			stash: stash.clone(),
			keys: keys.clone(),
		},
	)
	.encode();
	assert_eq!(real_set, (67u8, 3u8, stash.clone(), keys).encode(), "set_keys address moved");

	let real_purge = RuntimeCall::StakingAhClient(pezpallet_staking_async_ah_client::Call::<
		Runtime,
	>::purge_keys_from_ah {
		stash: stash.clone(),
	})
	.encode();
	assert_eq!(real_purge, (67u8, 4u8, stash).encode(), "purge_keys address moved");
}

/// Exactly two chains may tell this one who validates it, and one of them is new.
///
/// The People chain draws the committee, so it has to be admitted here or every message it
/// sends is dropped at the origin check with nothing on either side saying why -- both ends
/// look healthy and the validator set simply stops changing. The Asset Hub stays because
/// `ah_client` is also how session reports and the staking bookkeeping arrive.
///
/// Every other chain is refused, and the list is pinned rather than described: widening this
/// origin is how a sibling chain would come to seat validators, and nothing else in the tree
/// would notice.
#[test]
fn only_the_two_named_chains_may_seat_validators() {
	use pezframe_support::traits::EnsureOrigin;
	use pezkuwichain_runtime_constants::system_teyrchain::{
		ASSET_HUB_ID, BRIDGE_HUB_ID, BROKER_ID, PEOPLE_ID,
	};

	let from =
		|id: u32| -> RuntimeOrigin { teyrchains_origin::Origin::Teyrchain(id.into()).into() };

	for id in [ASSET_HUB_ID, PEOPLE_ID] {
		assert!(EnsureAssetHub::try_origin(from(id)).is_ok(), "chain {id} must be admitted");
	}
	for id in [BRIDGE_HUB_ID, BROKER_ID, 2000, 4242] {
		assert!(EnsureAssetHub::try_origin(from(id)).is_err(), "chain {id} must be refused");
	}
	// And nothing that is not a chain at all.
	assert!(EnsureAssetHub::try_origin(RuntimeOrigin::root()).is_err());
}

/// The committee call, pinned the same way and for the same reason.
///
/// This one matters more than the key calls: a key that never arrives costs one validator a
/// session, while a committee that never arrives means the relay keeps validating with the
/// set it already had, era after era, while People believes it has been handing over a new
/// one. Both ends would look healthy.
#[test]
fn the_committee_call_encodes_the_way_people_builds_it() {
	use codec::Encode;

	let committee: Vec<AccountId> = vec![[1u8; 32].into(), [2u8; 32].into()];
	let report = pezpallet_staking_async_rc_client::ValidatorSetReport::new_terminal(
		committee.clone(),
		9,
		None,
	);

	let real = RuntimeCall::StakingAhClient(
		pezpallet_staking_async_ah_client::Call::<Runtime>::validator_set {
			report: report.clone(),
		},
	)
	.encode();
	assert_eq!(real, (67u8, 0u8, report).encode(), "validator_set address moved");

	// The report's own shape is half the contract: People builds it from the crate that
	// defines it, so a field added upstream stops that runtime compiling. What the compiler
	// cannot see is the order, so pin the bytes.
	let by_hand = (
		67u8,
		0u8,
		committee,
		9u32,
		Option::<u32>::None, // prune_up_to: this chain does not track relay sessions
		false,               // leftover: a committee is bounded and always fits one message
	)
		.encode();
	assert_eq!(real, by_hand, "ValidatorSetReport's field order changed");
}
