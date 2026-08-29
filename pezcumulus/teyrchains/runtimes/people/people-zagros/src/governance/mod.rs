// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The state franchise: its origins today, its tracks when Referenda lands.

mod origins;
mod tracks;

pub use tracks::TracksInfo;

pub use origins::{
	pezpallet_custom_origins, CitizenshipAdmin, QeydRules, ReferendumCanceller, ReferendumKiller,
	WelatiAdmin, WelatiElection,
};
