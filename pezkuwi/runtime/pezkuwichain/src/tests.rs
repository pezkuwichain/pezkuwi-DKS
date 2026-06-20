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
fn check_treasury_pallet_id() {
	assert_eq!(
		<Treasury as pezframe_support::traits::PalletInfoAccess>::index() as u8,
		pezkuwichain_runtime_constants::TREASURY_PALLET_ID
	);
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
	assert_eq!(count, 18, "Expected 18 tracks (15 standard + 3 welati), got {count}");
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
		(0, "root", 2 * HOURS, 28 * DAYS, 24 * HOURS, 24 * HOURS),
		(1, "whitelisted_caller", 30 * MINUTES, 28 * DAYS, 10 * MINUTES, 10 * MINUTES),
		(10, "staking_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(11, "treasurer", 2 * HOURS, 28 * DAYS, 3 * HOURS, 24 * HOURS),
		(12, "lease_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(13, "fellowship_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(14, "general_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(15, "auction_admin", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(20, "referendum_canceller", 2 * HOURS, 7 * DAYS, 3 * HOURS, 10 * MINUTES),
		(21, "referendum_killer", 2 * HOURS, 14 * DAYS, 3 * HOURS, 10 * MINUTES),
		(30, "small_tipper", 1 * MINUTES, 7 * DAYS, 10 * MINUTES, 1 * MINUTES),
		(31, "big_tipper", 10 * MINUTES, 7 * DAYS, 1 * HOURS, 10 * MINUTES),
		(32, "small_spender", 4 * HOURS, 28 * DAYS, 12 * HOURS, 24 * HOURS),
		(33, "medium_spender", 4 * HOURS, 28 * DAYS, 24 * HOURS, 24 * HOURS),
		(34, "big_spender", 4 * HOURS, 28 * DAYS, 48 * HOURS, 24 * HOURS),
		(40, "welati_election", 2 * HOURS, 14 * DAYS, 12 * HOURS, 24 * HOURS),
		(41, "welati_admin", 2 * HOURS, 7 * DAYS, 3 * HOURS, 10 * MINUTES),
		(42, "citizenship_admin", 2 * HOURS, 14 * DAYS, 6 * HOURS, 24 * HOURS),
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

#[test]
fn governance_welati_tracks_exist() {
	let tracks: HashMap<u16, _> = <TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::tracks()
		.map(|t| (t.id, t.into_owned()))
		.collect();

	// welati_election
	let t40 = tracks.get(&40).expect("welati_election track (id=40) missing");
	assert_eq!(t40.info.max_deciding, 1, "welati_election should allow only 1 deciding");

	// welati_admin
	let t41 = tracks.get(&41).expect("welati_admin track (id=41) missing");
	assert_eq!(t41.info.max_deciding, 10);

	// citizenship_admin
	let t42 = tracks.get(&42).expect("citizenship_admin track (id=42) missing");
	assert_eq!(t42.info.max_deciding, 10);
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
		(Origin::Treasurer, 11),
		(Origin::LeaseAdmin, 12),
		(Origin::FellowshipAdmin, 13),
		(Origin::GeneralAdmin, 14),
		(Origin::AuctionAdmin, 15),
		(Origin::ReferendumCanceller, 20),
		(Origin::ReferendumKiller, 21),
		(Origin::SmallTipper, 30),
		(Origin::BigTipper, 31),
		(Origin::SmallSpender, 32),
		(Origin::MediumSpender, 33),
		(Origin::BigSpender, 34),
		(Origin::WelatiElection, 40),
		(Origin::WelatiAdmin, 41),
		(Origin::CitizenshipAdmin, 42),
	];

	for (origin, expected_id) in origin_to_track {
		let pallet_origin: <RuntimeOrigin as pezframe_support::traits::OriginTrait>::PalletsOrigin =
			origin.clone().into();
		let result =
			<TracksInfo as TracksInfoTrait<Balance, BlockNumber>>::track_for(&pallet_origin);
		assert_eq!(
			result,
			Ok(expected_id),
			"Origin {:?} should map to track {expected_id}",
			origin
		);
	}
}
