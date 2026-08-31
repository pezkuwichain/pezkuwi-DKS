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

//! New governance configurations for the Pezkuwichain runtime.

use super::*;
use pezframe_support::{
	parameter_types,
	traits::{ConstU16, EitherOf, Get},
};
use pezframe_system::EnsureRootWithSuccess;

mod origins;
pub use origins::{
	pezpallet_custom_origins, AuctionAdmin, Fellows, FellowshipAdmin, FellowshipExperts,
	FellowshipInitiates, FellowshipMasters, GeneralAdmin, LeaseAdmin, ReferendumCanceller,
	ReferendumKiller, StakingAdmin, WhitelistedCaller,
};
mod tracks;
pub use tracks::TracksInfo;

parameter_types! {
	pub const VoteLockingPeriod: BlockNumber = 7 * DAYS;
}

/// Turnout is measured against what can actually vote here.
///
/// The XCM checking account holds the supply that lives on the other chains: an arriving teleport
/// is checked in against it, so its balance is the counterpart of what left. It cannot vote, yet
/// `ActiveIssuanceOf` counts it, and every support threshold is a fraction of that figure. The
/// fast track's support floor never decays below five percent, so a checking account seeded to
/// cover real cross-chain flow would have put that path permanently out of reach — the emergency
/// route failing arithmetically, and only discovered in an emergency.
///
/// `MaxTurnout`'s own documentation asks for exactly this: reduce it to account for funds which
/// are unable to vote. Excluding the checking account also frees the genesis allocation, which
/// otherwise had to keep the seed below the circulating supply to leave the floor reachable.
pub struct VotableIssuance;
impl Get<Balance> for VotableIssuance {
	fn get() -> Balance {
		use pezframe_support::traits::fungible::Inspect;
		Balances::active_issuance().saturating_sub(Balances::balance(&XcmPallet::check_account()))
	}
}

impl pezpallet_conviction_voting::Config for Runtime {
	type WeightInfo = weights::pezpallet_conviction_voting::WeightInfo<Self>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type VoteLockingPeriod = VoteLockingPeriod;
	type MaxVotes = ConstU32<512>;
	type MaxTurnout = VotableIssuance;
	type Polls = Referenda;
	type BlockNumberProvider = System;
	type VotingHooks = ();
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	/// What it costs to put a referendum on the table. Was `1 * 3 * CENTS`, a ten-thousandth of
	/// a unit, so submitting was free and nothing stood between the queue and a flood.
	pub const SubmissionDeposit: Balance = 1 * DOLLARS;
	pub const UndecidingTimeout: BlockNumber = 14 * DAYS;
}

parameter_types! {
	pub const MaxBalance: Balance = Balance::max_value();
}

impl origins::pezpallet_custom_origins::Config for Runtime {}

impl pezpallet_whitelist::Config for Runtime {
	type WeightInfo = weights::pezpallet_whitelist::WeightInfo<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	// Root, and Root alone. `Fellows` stood beside it and no track on this chain produces that
	// origin -- there is no Fellowship collective here -- so the alternative was never an
	// alternative. What Root means changed underneath it: with the root track gone, Root here is
	// the register's referendum arriving over XCM. So the people whitelist and this chain's fast
	// track enacts, which is the two-key shape the arrangement wanted all along.
	type WhitelistOrigin = EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>;
	type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<Self::AccountId>, WhitelistedCaller>;
	type Preimages = Preimage;
}

impl pezpallet_referenda::Config for Runtime {
	type WeightInfo = weights::pezpallet_referenda_referenda::WeightInfo<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin = pezframe_system::EnsureSigned<AccountId>;
	type CancelOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumCanceller>;
	type KillOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumKiller>;
	// Referendum deposits that are killed rather than refunded. They go where every other
	// penalty on this chain goes: accumulated here and forwarded to the Asset Hub's
	// treasury. Not `()`, which would destroy them.
	type Slash = crate::PenaltiesToTreasury;
	type Votes = pezpallet_conviction_voting::VotesOf<Runtime>;
	type Tally = pezpallet_conviction_voting::TallyOf<Runtime>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}
