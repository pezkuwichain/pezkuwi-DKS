// Copyright 2019-2026 Dijital Kurdistan Tech Institute
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

//! Default platform for WASM environments.
//! When both 'native' and 'web' features are enabled, 'native' takes priority.

#[cfg(all(feature = "web", not(feature = "native")))]
mod wasm_helpers;
#[cfg(all(feature = "web", not(feature = "native")))]
mod wasm_platform;
#[cfg(all(feature = "web", not(feature = "native")))]
mod wasm_socket;

#[cfg(any(feature = "native", feature = "web"))]
pub use helpers::{build_platform, DefaultPlatform};

#[cfg(feature = "native")]
mod helpers {
	use smoldot_light::platform::default::DefaultPlatform as Platform;
	use std::sync::Arc;

	pub type DefaultPlatform = Arc<Platform>;

	pub fn build_platform() -> DefaultPlatform {
		Platform::new("subxt-light-client".into(), env!("CARGO_PKG_VERSION").into())
	}
}

#[cfg(all(feature = "web", not(feature = "native")))]
mod helpers {
	use super::wasm_platform::SubxtPlatform as Platform;

	pub type DefaultPlatform = Platform;

	pub fn build_platform() -> DefaultPlatform {
		Platform::new()
	}
}
