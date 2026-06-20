// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
//! Helpers for implementing runtime api

use crate::Config;
use pezsp_core::H256;
use xcm::{prelude::*, VersionedLocation};

pub fn agent_id<Runtime>(location: VersionedLocation) -> Option<H256>
where
	Runtime: Config,
{
	let location: Location = location.try_into().ok()?;
	crate::Pezpallet::<Runtime>::location_to_message_origin(location).ok()
}
