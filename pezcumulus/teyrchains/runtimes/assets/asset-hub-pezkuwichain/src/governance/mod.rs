// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The economic franchise: what the money decides, and how.

mod origins;
mod tracks;

pub use tracks::TracksInfo;

pub use origins::{
	pezpallet_custom_origins, ReferendumCanceller, ReferendumKiller, Spender, Treasurer,
};
