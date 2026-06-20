// Copyright 2019-2026 Dijital Kurdistan Tech Institute
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

//! Types associated with accessing constants.

mod constants_client;

pub use constants_client::ConstantsClient;
pub use pezkuwi_subxt_core::constants::address::{dynamic, Address, DynamicAddress, StaticAddress};
