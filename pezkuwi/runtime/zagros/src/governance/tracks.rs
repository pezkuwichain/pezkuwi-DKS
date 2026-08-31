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

//! Track configurations for governance.

use super::*;

use alloc::borrow::Cow;
use pezsp_runtime::str_array as s;

const fn percent(x: i32) -> pezsp_arithmetic::FixedI64 {
	pezsp_arithmetic::FixedI64::from_rational(x as u128, 100)
}
use pezpallet_referenda::Curve;
const APP_ROOT: Curve = Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_ROOT: Curve = Curve::make_linear(28, 28, percent(0), percent(50));
const APP_STAKING_ADMIN: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
const SUP_STAKING_ADMIN: Curve =
	Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
const APP_TREASURER: Curve = Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_TREASURER: Curve = Curve::make_linear(28, 28, percent(0), percent(50));
const APP_FELLOWSHIP_ADMIN: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
const SUP_FELLOWSHIP_ADMIN: Curve =
	Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
const APP_GENERAL_ADMIN: Curve =
	Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_GENERAL_ADMIN: Curve =
	Curve::make_reciprocal(7, 28, percent(10), percent(0), percent(50));
const APP_AUCTION_ADMIN: Curve =
	Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_AUCTION_ADMIN: Curve =
	Curve::make_reciprocal(7, 28, percent(10), percent(0), percent(50));
const APP_LEASE_ADMIN: Curve = Curve::make_linear(17, 28, percent(50), percent(100));
const SUP_LEASE_ADMIN: Curve = Curve::make_reciprocal(12, 28, percent(1), percent(0), percent(50));
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
const APP_WHITELISTED_CALLER: Curve =
	Curve::make_reciprocal(16, 28 * 24, percent(96), percent(50), percent(100));
const SUP_WHITELISTED_CALLER: Curve =
	Curve::make_reciprocal(1, 28, percent(20), percent(5), percent(50));

// A `root` track used to head this list, and it is gone.
//
// Root on this chain reaches every chain: `System::set_code` here, and
// `Paras::force_set_current_code` for each teyrchain. This chain's electorate is HEZ weighted by
// conviction, and its root curve asked for no support at all by day 28 -- so an upgrade of the
// whole network was, on paper, a large enough holding plus four weeks.
//
// Removing the track does not remove the power; it moves who holds it. Root here is still
// reachable, by `StateRegisterAsRoot`: a referendum on the People chain, where the tally counts
// citizens one each, arrives as `Superuser` and converts to Root. The constitution is decided by
// the people and enacted by the relay, which is the arrangement the whole design rests on.
//
// The `whitelisted_caller` track below keeps its purpose and gains a second key. `Whitelist`'s
// `WhitelistOrigin` is Root, and Root now means the register's referendum -- so the people
// approve a call hash and this chain's own fast track enacts it. Neither half alone suffices.
//
// Sudo remains for the founding period and is the one hand outside this arrangement.
const TRACKS_DATA: [pezpallet_referenda::Track<u16, Balance, BlockNumber>; 8] = [
	pezpallet_referenda::Track {
		id: 1,
		info: pezpallet_referenda::TrackInfo {
			name: s("whitelisted_caller"),
			max_deciding: 100,
			decision_deposit: 10 * GRAND,
			prepare_period: 30 * MINUTES,
			decision_period: 28 * DAYS,
			confirm_period: 10 * MINUTES,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_WHITELISTED_CALLER,
			min_support: SUP_WHITELISTED_CALLER,
		},
	},
	pezpallet_referenda::Track {
		id: 10,
		info: pezpallet_referenda::TrackInfo {
			name: s("staking_admin"),
			max_deciding: 10,
			decision_deposit: 5 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 14 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_STAKING_ADMIN,
			min_support: SUP_STAKING_ADMIN,
		},
	},
	pezpallet_referenda::Track {
		id: 12,
		info: pezpallet_referenda::TrackInfo {
			name: s("lease_admin"),
			max_deciding: 10,
			decision_deposit: 5 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 14 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_LEASE_ADMIN,
			min_support: SUP_LEASE_ADMIN,
		},
	},
	pezpallet_referenda::Track {
		id: 13,
		info: pezpallet_referenda::TrackInfo {
			name: s("fellowship_admin"),
			max_deciding: 10,
			decision_deposit: 5 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 14 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_FELLOWSHIP_ADMIN,
			min_support: SUP_FELLOWSHIP_ADMIN,
		},
	},
	pezpallet_referenda::Track {
		id: 14,
		info: pezpallet_referenda::TrackInfo {
			name: s("general_admin"),
			max_deciding: 10,
			decision_deposit: 5 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 14 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_GENERAL_ADMIN,
			min_support: SUP_GENERAL_ADMIN,
		},
	},
	pezpallet_referenda::Track {
		id: 15,
		info: pezpallet_referenda::TrackInfo {
			name: s("auction_admin"),
			max_deciding: 10,
			decision_deposit: 5 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 14 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_AUCTION_ADMIN,
			min_support: SUP_AUCTION_ADMIN,
		},
	},
	pezpallet_referenda::Track {
		id: 20,
		info: pezpallet_referenda::TrackInfo {
			name: s("referendum_canceller"),
			max_deciding: 1_000,
			decision_deposit: 10 * GRAND,
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
			decision_deposit: 50 * GRAND,
			prepare_period: 2 * HOURS,
			decision_period: 14 * DAYS,
			confirm_period: 3 * HOURS,
			min_enactment_period: 10 * MINUTES,
			min_approval: APP_REFERENDUM_KILLER,
			min_support: SUP_REFERENDUM_KILLER,
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
			// No system origin has a track here any more, Root included. Root arrives from the
			// People chain's referendum, not from a ballot of this chain's holders.
			Err(())
		} else if let Ok(custom_origin) = origins::Origin::try_from(id.clone()) {
			match custom_origin {
				origins::Origin::WhitelistedCaller => Ok(1),
				// General admin
				origins::Origin::StakingAdmin => Ok(10),
				origins::Origin::LeaseAdmin => Ok(12),
				origins::Origin::FellowshipAdmin => Ok(13),
				origins::Origin::GeneralAdmin => Ok(14),
				origins::Origin::AuctionAdmin => Ok(15),
				// Referendum admins
				origins::Origin::ReferendumCanceller => Ok(20),
				origins::Origin::ReferendumKiller => Ok(21),
				// Limited treasury spenders
				_ => Err(()),
			}
		} else {
			Err(())
		}
	}
}
