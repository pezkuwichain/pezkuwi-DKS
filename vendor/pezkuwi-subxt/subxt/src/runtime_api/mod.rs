// Copyright 2019-2026 Dijital Kurdistan Tech Institute
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

//! Types associated with executing runtime API calls.

mod runtime_client;
mod runtime_types;

pub use pezkuwi_subxt_core::runtime_api::payload::{
	dynamic, DynamicPayload, Payload, StaticPayload,
};
pub use runtime_client::RuntimeApiClient;
pub use runtime_types::RuntimeApi;
