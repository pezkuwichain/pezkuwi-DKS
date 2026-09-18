// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Tracks for the state register's referenda.
//!
//! The four tracks are the ones the relay already runs for these origins, carried over with
//! their periods and deposits unchanged -- the same offices, deciding the same things, now on
//! the chain that holds the register.
//!
//! The curves need one change, and it is smaller than it first looks.
//!
//! A turnout requirement is the wrong instrument. Measured as a share of the roll, it turns
//! abstention into a vote against: anyone opposed does better staying home than voting no,
//! and a minority that boycotts gets more say than one that turns up. That is why the standard
//! shape lets the support requirement decay -- legitimacy is meant to come from the notice and
//! the length of the window, not from a quorum. A question that stood open for weeks and drew
//! nobody has been answered by everybody who did not come.
//!
//! That reasoning holds all the way down to one voter, and stops being true there. "Nobody
//! came" and "one person came" are not the same event, and a curve reaching exactly zero makes
//! them the same: a single citizen out of the whole register settles a constitutional question
//! by being the only one present. So each curve keeps a floor, and the floor is small on
//! purpose -- low enough that genuine interest clears it without thinking, too low for a
//! boycott to stay under.
//!
//! The other half of that bargain is notice, and it has to be real. `prepare_period` is how
//! long a question stands before it can start being decided; the relay's two hours is fine for
//! a routine appointment and is not notice of a constitutional change. The two serious tracks
//! carry a week.

use super::origins;
use crate::{Balance, BlockNumber, RuntimeOrigin};

use alloc::borrow::Cow;
use pezkuwi_runtime_common::rehearsal_period;
use pezpallet_referenda::Curve;
use pezsp_runtime::{str_array as s, FixedI64};
use testnet_teyrchains_constants::zagros::currency::UNITS;
use testnet_teyrchains_constants::zagros::time::{DAYS, HOURS, MINUTES};

const fn percent(x: i32) -> FixedI64 {
	FixedI64::from_rational(x as u128, 100)
}

/// Root: the constitution and the chain itself. Approval falls from unanimity towards a
/// simple majority over four weeks; support decays too, but not to nothing.
const APP_ROOT: Curve = Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_ROOT: Curve = Curve::make_linear(28, 28, percent(2), percent(50));

/// Elections: a single question, decided once, by the largest share of the roll asked for
/// anywhere here.
const APP_WELATI_ELECTION: Curve = APP_ROOT;
const SUP_WELATI_ELECTION: Curve = Curve::make_linear(14, 14, percent(2), percent(50));

/// Appointments and tiki grants: routine, frequent, reversible by the same track.
const APP_WELATI_ADMIN: Curve =
	Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_WELATI_ADMIN: Curve = Curve::make_reciprocal(7, 28, percent(10), percent(1), percent(50));

/// Citizenship and trust: it takes a citizen's standing away, so it asks for more than the
/// admin track and takes twice as long.
const APP_CITIZENSHIP_ADMIN: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
const SUP_CITIZENSHIP_ADMIN: Curve =
	Curve::make_reciprocal(12, 28, percent(10), percent(1), percent(50));

/// The register's rules: root's demands, stretched over three months.
///
/// The shape is deliberately root's rather than a new one -- what changes here is the *time*,
/// not how convincing the case has to be. Ninety days to prepare and ninety to decide is what
/// makes a rule hard to move in a mood; the approval and support a proposal must reach are
/// already argued for on the constitution track and there is no reason for this one to differ.
/// The curves are the number a future Serok is most likely to want to revisit; the periods are
/// the ones the decision fixed.
const APP_QEYD_RULES: Curve =
	Curve::make_reciprocal(13, 90, percent(80), percent(50), percent(100));
const SUP_QEYD_RULES: Curve = Curve::make_linear(90, 90, percent(2), percent(50));

const TRACKS_DATA: [pezpallet_referenda::Track<u16, Balance, BlockNumber>; 5] = [
	pezpallet_referenda::Track {
		id: 0,
		info: pezpallet_referenda::TrackInfo {
			name: s("root"),
			max_deciding: 5,
			decision_deposit: 100 * UNITS,
			prepare_period: rehearsal_period!(7 * DAYS, DAYS),
			decision_period: rehearsal_period!(28 * DAYS, DAYS),
			confirm_period: rehearsal_period!(24 * HOURS, DAYS),
			min_enactment_period: rehearsal_period!(24 * HOURS, DAYS),
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
			prepare_period: rehearsal_period!(7 * DAYS, DAYS),
			decision_period: rehearsal_period!(14 * DAYS, DAYS),
			confirm_period: rehearsal_period!(12 * HOURS, DAYS),
			min_enactment_period: rehearsal_period!(24 * HOURS, DAYS),
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
			prepare_period: rehearsal_period!(2 * HOURS, DAYS),
			decision_period: rehearsal_period!(7 * DAYS, DAYS),
			confirm_period: rehearsal_period!(3 * HOURS, DAYS),
			min_enactment_period: rehearsal_period!(10 * MINUTES, DAYS),
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
			prepare_period: rehearsal_period!(2 * DAYS, DAYS),
			decision_period: rehearsal_period!(14 * DAYS, DAYS),
			confirm_period: rehearsal_period!(6 * HOURS, DAYS),
			min_enactment_period: rehearsal_period!(24 * HOURS, DAYS),
			min_approval: APP_CITIZENSHIP_ADMIN,
			min_support: SUP_CITIZENSHIP_ADMIN,
		},
	},
	pezpallet_referenda::Track {
		id: 43,
		info: pezpallet_referenda::TrackInfo {
			name: s("qeyd_rules"),
			// One at a time. Two simultaneous proposals to reshape the register would be
			// decided against each other's assumptions, and both would enact.
			max_deciding: 1,
			decision_deposit: 100 * UNITS,
			prepare_period: rehearsal_period!(90 * DAYS, DAYS),
			decision_period: rehearsal_period!(90 * DAYS, DAYS),
			confirm_period: rehearsal_period!(7 * DAYS, DAYS),
			min_enactment_period: rehearsal_period!(30 * DAYS, DAYS),
			min_approval: APP_QEYD_RULES,
			min_support: SUP_QEYD_RULES,
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
				origins::Origin::QeydRules => Ok(43),
				// These two act on referenda themselves and are handed out by the tracks
				// above rather than being tracks of their own.
				origins::Origin::ReferendumCanceller | origins::Origin::ReferendumKiller => Err(()),
			}
		} else {
			Err(())
		}
	}
}

#[cfg(test)]
mod rehearsal_tests {
	use super::TRACKS_DATA;

	fn track(
		name: &str,
	) -> &'static pezpallet_referenda::Track<u16, crate::Balance, crate::BlockNumber> {
		TRACKS_DATA
			.iter()
			// `str_array` zero-pads to a fixed width, so the stored name carries trailing nulls
			// and a direct comparison against the literal never matches.
			.find(|t| {
				let stored: &[u8] = t.info.name.as_ref();
				stored.iter().position(|b| *b == 0).map_or(stored, |n| &stored[..n])
					== name.as_bytes()
			})
			.expect("the track exists")
	}

	/// A deployed runtime carries the periods as the constitution states them.
	///
	/// `qeyd_rules` is the one to check because it is the slowest and therefore the one a
	/// compression would most obviously help with -- which is exactly why it must not be
	/// compressed by accident. Every runtime this project deploys is built without
	/// `fast-runtime`, so this is the branch the live chains take.
	#[test]
	#[cfg(not(feature = "fast-runtime"))]
	fn deployed_tracks_keep_the_constitution_periods() {
		let t = &track("qeyd_rules").info;
		assert_eq!(t.prepare_period, 90 * super::DAYS, "ninety days of preparation");
		assert_eq!(t.decision_period, 90 * super::DAYS, "ninety days to decide");
		assert_eq!(t.confirm_period, 7 * super::DAYS, "a week to confirm");
		assert_eq!(t.min_enactment_period, 30 * super::DAYS, "a month before it takes effect");
	}

	/// A rehearsal runtime compresses them, and the order the constitution gives them survives.
	///
	/// The order is the part worth asserting: a compression that made every track the same
	/// length would let a rehearsal pass arrangements the real chain would refuse, because a
	/// track meant to deliberate longer than another no longer does.
	#[test]
	#[cfg(feature = "fast-runtime")]
	fn rehearsal_tracks_compress_and_stay_ordered() {
		let slow = &track("qeyd_rules").info;
		let fast = &track("welati_admin").info;
		assert_eq!(slow.prepare_period, 90, "one block per day");
		assert_eq!(slow.decision_period, 90);
		assert_eq!(slow.confirm_period, 7);
		assert_eq!(slow.min_enactment_period, 30);
		// Below a day the floor applies rather than zero.
		assert_eq!(fast.prepare_period, 2, "two hours floors at two blocks, not zero");
		assert_eq!(fast.min_enactment_period, 2, "ten minutes floors at two blocks, not zero");
		assert!(
			slow.decision_period > fast.decision_period,
			"the constitutional track still deliberates longer than the administrative one"
		);
	}
}
