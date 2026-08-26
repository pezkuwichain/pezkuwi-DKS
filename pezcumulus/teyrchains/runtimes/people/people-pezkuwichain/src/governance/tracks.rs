// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Tracks for the state register's referenda.
//!
//! The four tracks are the ones the relay already runs for these origins, carried over with
//! their periods and deposits unchanged -- the same offices, deciding the same things, now on
//! the chain that holds the register.
//!
//! The curves are not carried over unchanged, and cannot be. On the relay the tally is
//! token-weighted and `support` measures turnout against total issuance, where the standard
//! shape lets the support requirement fall to zero: after four weeks with nobody objecting, a
//! handful of tokens carries it. Here `support` is measured against the roll and every citizen
//! counts once, so a floor of zero would mean one voter out of the whole register can decide a
//! constitutional question by being the only one who turned up. `CitizenTally` refuses to read
//! an empty roll as consent; a curve that decays to nothing would hand that back.
//!
//! So each support curve keeps a floor. What the floor should be is the question of how much
//! of the register has to show up before a decision binds everyone, and it is set here at a
//! deliberately conservative starting point rather than at zero.

use super::origins;
use crate::{Balance, BlockNumber, RuntimeOrigin};

use alloc::borrow::Cow;
use pezpallet_referenda::Curve;
use pezsp_runtime::{str_array as s, FixedI64};
use teyrchains_common::{DAYS, HOURS, MINUTES};
use testnet_teyrchains_constants::pezkuwichain::currency::UNITS;

const fn percent(x: i32) -> FixedI64 {
	FixedI64::from_rational(x as u128, 100)
}

/// Root: the constitution and the chain itself. Approval falls from unanimity towards a
/// simple majority over the decision period; support never falls below a fifth of the roll.
const APP_ROOT: Curve = Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_ROOT: Curve = Curve::make_linear(28, 28, percent(20), percent(50));

/// Elections: a single question, decided once, by the largest share of the roll asked for
/// anywhere here.
const APP_WELATI_ELECTION: Curve = APP_ROOT;
const SUP_WELATI_ELECTION: Curve = Curve::make_linear(14, 14, percent(20), percent(50));

/// Appointments and tiki grants: routine, frequent, reversible by the same track.
const APP_WELATI_ADMIN: Curve =
	Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_WELATI_ADMIN: Curve = Curve::make_reciprocal(7, 28, percent(10), percent(5), percent(50));

/// Citizenship and trust: it takes a citizen's standing away, so it asks for more than the
/// admin track and takes twice as long.
const APP_CITIZENSHIP_ADMIN: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
const SUP_CITIZENSHIP_ADMIN: Curve =
	Curve::make_reciprocal(12, 28, percent(10), percent(10), percent(50));

const TRACKS_DATA: [pezpallet_referenda::Track<u16, Balance, BlockNumber>; 4] = [
	pezpallet_referenda::Track {
		id: 0,
		info: pezpallet_referenda::TrackInfo {
			name: s("root"),
			max_deciding: 5,
			decision_deposit: 100 * UNITS,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 24 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_ROOT,
			min_support: SUP_ROOT,
		},
	},
	pezpallet_referenda::Track {
		id: 40,
		info: pezpallet_referenda::TrackInfo {
			name: s("welati_election"),
			max_deciding: 1,
			decision_deposit: 50 * UNITS,
			prepare_period: 2 * HOURS,
			decision_period: 14 * DAYS,
			confirm_period: 12 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_WELATI_ELECTION,
			min_support: SUP_WELATI_ELECTION,
		},
	},
	pezpallet_referenda::Track {
		id: 41,
		info: pezpallet_referenda::TrackInfo {
			name: s("welati_admin"),
			max_deciding: 10,
			decision_deposit: 10 * UNITS,
			prepare_period: 2 * HOURS,
			decision_period: 7 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_WELATI_ADMIN,
			min_support: SUP_WELATI_ADMIN,
		},
	},
	pezpallet_referenda::Track {
		id: 42,
		info: pezpallet_referenda::TrackInfo {
			name: s("citizenship_admin"),
			max_deciding: 10,
			decision_deposit: 20 * UNITS,
			prepare_period: 2 * HOURS,
			decision_period: 14 * DAYS,
			confirm_period: 6 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_CITIZENSHIP_ADMIN,
			min_support: SUP_CITIZENSHIP_ADMIN,
		},
	},
];

pub struct TracksInfo;
impl pezpallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = u16;
	type RuntimeOrigin = <RuntimeOrigin as pezframe_support::traits::OriginTrait>::PalletsOrigin;

	fn tracks(
	) -> impl Iterator<Item = Cow<'static, pezpallet_referenda::Track<Self::Id, Balance, BlockNumber>>>
	{
		TRACKS_DATA.iter().map(Cow::Borrowed)
	}

	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		if let Ok(system_origin) = pezframe_system::RawOrigin::try_from(id.clone()) {
			match system_origin {
				pezframe_system::RawOrigin::Root => Ok(0),
				_ => Err(()),
			}
		} else if let Ok(custom_origin) = origins::Origin::try_from(id.clone()) {
			match custom_origin {
				origins::Origin::WelatiElection => Ok(40),
				origins::Origin::WelatiAdmin => Ok(41),
				origins::Origin::CitizenshipAdmin => Ok(42),
				// These two act on referenda themselves and are handed out by the tracks
				// above rather than being tracks of their own.
				origins::Origin::ReferendumCanceller | origins::Origin::ReferendumKiller =>
					Err(()),
			}
		} else {
			Err(())
		}
	}
}
