// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Tracks for the economic franchise's referenda.
//!
//! Carried over from the relay, where these five spending tiers and the treasury origin above
//! them already had their periods, deposits and concurrency settled. The same questions,
//! decided the same way, on the chain that holds the money.
//!
//! The curves come over unchanged, and here that is right. This tally is token-weighted:
//! `support` measures turnout against total issuance, and the standard shape lets it decay to
//! nothing because a spending question that stood open for four weeks unopposed has been
//! answered by everyone who did not object. The state's ballot needed a floor because one
//! citizen out of a register is not the register; a holding is not a head, and the same
//! reasoning does not carry across.

use super::origins;
use crate::{Balance, BlockNumber, RuntimeOrigin};

use alloc::borrow::Cow;
use pezpallet_referenda::Curve;
use pezsp_runtime::{str_array as s, FixedI64};
use testnet_teyrchains_constants::pezkuwichain::currency::UNITS;
use teyrchains_common::{DAYS, HOURS, MINUTES};

const fn percent(x: i32) -> FixedI64 {
	FixedI64::from_rational(x as u128, 100)
}

const APP_TREASURER: Curve = Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_TREASURER: Curve = Curve::make_linear(28, 28, percent(0), percent(50));
const APP_REFERENDUM_CANCELLER: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
const SUP_REFERENDUM_CANCELLER: Curve =
	Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
const APP_REFERENDUM_KILLER: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
const SUP_REFERENDUM_KILLER: Curve =
	Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
const APP_SMALL_TIPPER: Curve = Curve::make_linear(10, 28, percent(50), percent(100));
const SUP_SMALL_TIPPER: Curve = Curve::make_reciprocal(1, 28, percent(4), percent(0), percent(50));
const APP_BIG_TIPPER: Curve = Curve::make_linear(10, 28, percent(50), percent(100));
const SUP_BIG_TIPPER: Curve = Curve::make_reciprocal(8, 28, percent(1), percent(0), percent(50));
const APP_SMALL_SPENDER: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
const SUP_SMALL_SPENDER: Curve =
	Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
const APP_MEDIUM_SPENDER: Curve = Curve::make_linear(23, 28, percent(50), percent(100));
const SUP_MEDIUM_SPENDER: Curve =
	Curve::make_reciprocal(16, 28, percent(1), percent(0), percent(50));
const APP_BIG_SPENDER: Curve = Curve::make_linear(28, 28, percent(50), percent(100));
const SUP_BIG_SPENDER: Curve = Curve::make_reciprocal(20, 28, percent(1), percent(0), percent(50));

// A `root` track used to head this list, and it is gone. Root here reaches
// `System::authorize_upgrade`, and this chain's electorate is holdings: HEZ, weighted by
// conviction. An upgrade is the constitution, and the constitution is not decided by how much
// of the token someone holds -- the People chain's root track counts heads, and that is the
// one that decides code. This chain's franchise keeps what it is for: the treasury, the
// spenders, and its own referenda.
//
// It was also priced as if it were not root at all. `min_approval` and `min_support` were
// `APP_TREASURER` and `SUP_TREASURER` -- a spending track's thresholds on a track that could
// rewrite the runtime.
const TRACKS_DATA: [pezpallet_referenda::Track<u16, Balance, BlockNumber>; 9] = [
	pezpallet_referenda::Track {
		id: 11,
		info: pezpallet_referenda::TrackInfo {
			name: s("treasurer"),
			max_deciding: 10,
			decision_deposit: 100 * UNITS,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_TREASURER,
			min_support: SUP_TREASURER,
		},
	},
	// The emission knobs. Slower and dearer than any spend, because a spend is one payment
	// and a rate is every payment after it -- and it is the holders' own dilution, so it is
	// theirs to set. The ceiling it cannot pass is in the code, not here.
	pezpallet_referenda::Track {
		id: 12,
		info: pezpallet_referenda::TrackInfo {
			name: s("economic_admin"),
			max_deciding: 2,
			decision_deposit: 100 * UNITS,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 24 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_TREASURER,
			min_support: SUP_TREASURER,
		},
	},
	pezpallet_referenda::Track {
		id: 20,
		info: pezpallet_referenda::TrackInfo {
			name: s("referendum_canceller"),
			max_deciding: 1_000,
			decision_deposit: 10 * UNITS,
			prepare_period: 2 * HOURS,
			decision_period: 7 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_REFERENDUM_CANCELLER,
			min_support: SUP_REFERENDUM_CANCELLER,
		},
	},
	pezpallet_referenda::Track {
		id: 21,
		info: pezpallet_referenda::TrackInfo {
			name: s("referendum_killer"),
			max_deciding: 1_000,
			decision_deposit: 50 * UNITS,
			prepare_period: 2 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_REFERENDUM_KILLER,
			min_support: SUP_REFERENDUM_KILLER,
		},
	},
	pezpallet_referenda::Track {
		id: 30,
		info: pezpallet_referenda::TrackInfo {
			name: s("small_tipper"),
			max_deciding: 200,
			decision_deposit: 1 * UNITS,
			prepare_period: 1 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 10 * MINUTES,
			min_enactment_period: 1 * MINUTES,
			min_approval: APP_SMALL_TIPPER,
			min_support: SUP_SMALL_TIPPER,
		},
	},
	pezpallet_referenda::Track {
		id: 31,
		info: pezpallet_referenda::TrackInfo {
			name: s("big_tipper"),
			max_deciding: 100,
			decision_deposit: 10 * UNITS,
			prepare_period: 10 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 1 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_BIG_TIPPER,
			min_support: SUP_BIG_TIPPER,
		},
	},
	pezpallet_referenda::Track {
		id: 32,
		info: pezpallet_referenda::TrackInfo {
			name: s("small_spender"),
			max_deciding: 50,
			decision_deposit: 100 * UNITS,
			prepare_period: 4 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 12 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_SMALL_SPENDER,
			min_support: SUP_SMALL_SPENDER,
		},
	},
	pezpallet_referenda::Track {
		id: 33,
		info: pezpallet_referenda::TrackInfo {
			name: s("medium_spender"),
			max_deciding: 50,
			decision_deposit: 200 * UNITS,
			prepare_period: 4 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 24 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_MEDIUM_SPENDER,
			min_support: SUP_MEDIUM_SPENDER,
		},
	},
	pezpallet_referenda::Track {
		id: 34,
		info: pezpallet_referenda::TrackInfo {
			name: s("big_spender"),
			max_deciding: 50,
			decision_deposit: 400 * UNITS,
			prepare_period: 4 * HOURS,
			decision_period: 28 * DAYS,
			confirm_period: 48 * HOURS,
			min_enactment_period: 24 * HOURS,
			min_approval: APP_BIG_SPENDER,
			min_support: SUP_BIG_SPENDER,
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
		if pezframe_system::RawOrigin::try_from(id.clone()).is_ok() {
			// No system origin, Root included, has a track on this chain any more.
			Err(())
		} else if let Ok(custom_origin) = origins::Origin::try_from(id.clone()) {
			match custom_origin {
				origins::Origin::Treasurer => Ok(11),
				origins::Origin::EconomicAdmin => Ok(12),
				origins::Origin::ReferendumCanceller => Ok(20),
				origins::Origin::ReferendumKiller => Ok(21),
				origins::Origin::SmallTipper => Ok(30),
				origins::Origin::BigTipper => Ok(31),
				origins::Origin::SmallSpender => Ok(32),
				origins::Origin::MediumSpender => Ok(33),
				origins::Origin::BigSpender => Ok(34),
			}
		} else {
			Err(())
		}
	}
}
