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
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

//! Pezkuwi chain configurations.

#[cfg(feature = "pezkuwichain-native")]
use pezkuwichain_runtime as pezkuwichain;
use pezsc_chain_spec::ChainSpecExtension;
#[cfg(any(feature = "zagros-native", feature = "pezkuwichain-native"))]
use pezsc_chain_spec::ChainType;
#[cfg(any(feature = "zagros-native", feature = "pezkuwichain-native"))]
use pezsc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
#[cfg(feature = "zagros-native")]
use zagros_runtime as zagros;

#[cfg(feature = "zagros-native")]
const ZAGROS_STAGING_TELEMETRY_URL: &str = "wss://telemetry.pezkuwichain.io/submit/";
#[cfg(feature = "pezkuwichain-native")]
const PEZKUWICHAIN_STAGING_TELEMETRY_URL: &str = "wss://telemetry.pezkuwichain.io/submit/";
#[cfg(feature = "pezkuwichain-native")]
const VERSI_STAGING_TELEMETRY_URL: &str = "wss://telemetry.pezkuwichain.io/submit/";
#[cfg(any(feature = "zagros-native", feature = "pezkuwichain-native"))]
const DEFAULT_PROTOCOL_ID: &str = "hez";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Bizinikiwi core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: pezsc_client_api::ForkBlocks<pezkuwi_primitives::Block>,
	/// Known bad block hashes.
	pub bad_blocks: pezsc_client_api::BadBlocks<pezkuwi_primitives::Block>,
	/// The light sync state.
	///
	/// This value will be set by the `sync-state rpc` implementation.
	pub light_sync_state: pezsc_sync_state_rpc::LightSyncStateExtension,
}

// Generic chain spec, in case when we don't have the native runtime.
pub type GenericChainSpec = pezsc_service::GenericChainSpec<Extensions>;

/// The `ChainSpec` parameterized for the zagros runtime.
#[cfg(feature = "zagros-native")]
pub type ZagrosChainSpec = pezsc_service::GenericChainSpec<Extensions>;

/// The `ChainSpec` parameterized for the zagros runtime.
// Dummy chain spec, but that is fine when we don't have the native runtime.
#[cfg(not(feature = "zagros-native"))]
pub type ZagrosChainSpec = GenericChainSpec;

/// The `ChainSpec` parameterized for the pezkuwichain runtime.
#[cfg(feature = "pezkuwichain-native")]
pub type PezkuwichainChainSpec = pezsc_service::GenericChainSpec<Extensions>;

/// The `ChainSpec` parameterized for the pezkuwichain runtime.
// Dummy chain spec, but that is fine when we don't have the native runtime.
#[cfg(not(feature = "pezkuwichain-native"))]
pub type PezkuwichainChainSpec = GenericChainSpec;

pub fn pezkuwi_config() -> Result<GenericChainSpec, String> {
	GenericChainSpec::from_json_bytes(&include_bytes!("../chain-specs/pezkuwi.json")[..])
}

pub fn dicle_config() -> Result<GenericChainSpec, String> {
	GenericChainSpec::from_json_bytes(&include_bytes!("../chain-specs/dicle.json")[..])
}

pub fn zagros_config() -> Result<ZagrosChainSpec, String> {
	ZagrosChainSpec::from_json_bytes(&include_bytes!("../chain-specs/zagros.json")[..])
}

pub fn paseo_config() -> Result<GenericChainSpec, String> {
	GenericChainSpec::from_json_bytes(&include_bytes!("../chain-specs/paseo.json")[..])
}

pub fn pezkuwichain_config() -> Result<PezkuwichainChainSpec, String> {
	PezkuwichainChainSpec::from_json_bytes(&include_bytes!("../chain-specs/pezkuwichain.json")[..])
}

/// PezkuwiChain Mainnet config with real validators and HEZ token distribution
#[cfg(feature = "pezkuwichain-native")]
pub fn pezkuwichain_mainnet_config() -> Result<PezkuwichainChainSpec, String> {
	Ok(PezkuwichainChainSpec::builder(
		pezkuwichain::WASM_BINARY.ok_or("Pezkuwichain WASM not available")?,
		Default::default(),
	)
	.with_name("PezkuwiChain Mainnet")
	.with_id("pezkuwichain_mainnet")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_preset_name("genesis")
	.with_telemetry_endpoints(
		TelemetryEndpoints::new(vec![(PEZKUWICHAIN_STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Pezkuwichain Mainnet telemetry url is valid; qed"),
	)
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.with_properties(pezkuwichain_chain_spec_properties())
	.build())
}

/// Zagros staging testnet config.
#[cfg(feature = "zagros-native")]
pub fn zagros_staging_testnet_config() -> Result<ZagrosChainSpec, String> {
	Ok(ZagrosChainSpec::builder(
		zagros::WASM_BINARY.ok_or("Zagros development wasm not available")?,
		Default::default(),
	)
	.with_name("Zagros Staging Testnet")
	.with_id("zagros_staging_testnet")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_preset_name("pezstaging_testnet")
	.with_telemetry_endpoints(
		TelemetryEndpoints::new(vec![(ZAGROS_STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Zagros Staging telemetry url is valid; qed"),
	)
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build())
}

/// Pezkuwichain staging testnet config.
#[cfg(feature = "pezkuwichain-native")]
pub fn pezkuwichain_staging_testnet_config() -> Result<PezkuwichainChainSpec, String> {
	Ok(PezkuwichainChainSpec::builder(
		pezkuwichain::WASM_BINARY.ok_or("Pezkuwichain development wasm not available")?,
		Default::default(),
	)
	.with_name("Pezkuwichain Staging Testnet")
	.with_id("pezkuwichain_staging_testnet")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_preset_name("pezstaging_testnet")
	.with_telemetry_endpoints(
		TelemetryEndpoints::new(vec![(PEZKUWICHAIN_STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Pezkuwichain Staging telemetry url is valid; qed"),
	)
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build())
}

pub fn versi_chain_spec_properties() -> serde_json::map::Map<String, serde_json::Value> {
	serde_json::json!({
		"ss58Format": 42,
		"tokenDecimals": 12,
		"tokenSymbol": "VRS",
	})
	.as_object()
	.expect("Map given; qed")
	.clone()
}

/// PezkuwiChain mainnet chain spec properties (HEZ token)
/// Note: tokenDecimals must be 12 to match UNITS = 10^12 in runtime constants
pub fn pezkuwichain_chain_spec_properties() -> serde_json::map::Map<String, serde_json::Value> {
	serde_json::json!({
		"ss58Format": 42,
		"tokenDecimals": 12,
		"tokenSymbol": "HEZ",
	})
	.as_object()
	.expect("Map given; qed")
	.clone()
}

/// Versi staging testnet config.
#[cfg(feature = "pezkuwichain-native")]
pub fn versi_staging_testnet_config() -> Result<PezkuwichainChainSpec, String> {
	Ok(PezkuwichainChainSpec::builder(
		pezkuwichain::WASM_BINARY.ok_or("Versi development wasm not available")?,
		Default::default(),
	)
	.with_name("Versi Staging Testnet")
	.with_id("versi_staging_testnet")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_preset_name("pezstaging_testnet")
	.with_telemetry_endpoints(
		TelemetryEndpoints::new(vec![(VERSI_STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Versi Staging telemetry url is valid; qed"),
	)
	.with_protocol_id("versi")
	.with_properties(versi_chain_spec_properties())
	.build())
}

/// Mainnet simulation config (2 validators + real sudo key, for local upgrade testing)
#[cfg(feature = "pezkuwichain-native")]
pub fn pezkuwichain_mainnet_simulation_config() -> Result<PezkuwichainChainSpec, String> {
	Ok(PezkuwichainChainSpec::builder(
		pezkuwichain::WASM_BINARY.ok_or("Pezkuwichain WASM not available")?,
		Default::default(),
	)
	.with_name("PezkuwiChain Mainnet Simulation")
	.with_id("pezkuwichain_mainnet_simulation")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name("mainnet_simulation")
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.with_properties(pezkuwichain_chain_spec_properties())
	.build())
}

/// Zagros development config (single validator Alice)
#[cfg(feature = "zagros-native")]
pub fn zagros_development_config() -> Result<ZagrosChainSpec, String> {
	Ok(ZagrosChainSpec::builder(
		zagros::WASM_BINARY.ok_or("Zagros development wasm not available")?,
		Default::default(),
	)
	.with_name("Development")
	.with_id("zagros_dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_preset_name(pezsp_genesis_builder::DEV_RUNTIME_PRESET)
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build())
}

/// Pezkuwichain development config (single validator Alice)
#[cfg(feature = "pezkuwichain-native")]
pub fn pezkuwichain_development_config() -> Result<PezkuwichainChainSpec, String> {
	Ok(PezkuwichainChainSpec::builder(
		pezkuwichain::WASM_BINARY.ok_or("Pezkuwichain development wasm not available")?,
		Default::default(),
	)
	.with_name("Development")
	.with_id("pezkuwichain_dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_preset_name(pezsp_genesis_builder::DEV_RUNTIME_PRESET)
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build())
}

/// `Versi` development config (single validator Alice)
#[cfg(feature = "pezkuwichain-native")]
pub fn versi_development_config() -> Result<PezkuwichainChainSpec, String> {
	Ok(PezkuwichainChainSpec::builder(
		pezkuwichain::WASM_BINARY.ok_or("Versi development wasm not available")?,
		Default::default(),
	)
	.with_name("Development")
	.with_id("versi_dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_preset_name(pezsp_genesis_builder::DEV_RUNTIME_PRESET)
	.with_protocol_id("versi")
	.build())
}

/// Zagros local testnet config (multivalidator Alice + Bob)
#[cfg(feature = "zagros-native")]
pub fn zagros_local_testnet_config() -> Result<ZagrosChainSpec, String> {
	Ok(ZagrosChainSpec::builder(
		zagros::fast_runtime_binary::WASM_BINARY.ok_or("Zagros development wasm not available")?,
		Default::default(),
	)
	.with_name("Zagros Local Testnet")
	.with_id("zagros_local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build())
}

/// Pezkuwichain local testnet config (multivalidator Alice + Bob)
#[cfg(feature = "pezkuwichain-native")]
pub fn pezkuwichain_local_testnet_config() -> Result<PezkuwichainChainSpec, String> {
	Ok(PezkuwichainChainSpec::builder(
		pezkuwichain::fast_runtime_binary::WASM_BINARY
			.ok_or("Pezkuwichain development wasm not available")?,
		Default::default(),
	)
	.with_name("Pezkuwichain Local Testnet")
	.with_id("pezkuwichain_local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build())
}

/// `Versi` local testnet config (multivalidator Alice + Bob + Charlie + Dave)
#[cfg(feature = "pezkuwichain-native")]
pub fn versi_local_testnet_config() -> Result<PezkuwichainChainSpec, String> {
	Ok(PezkuwichainChainSpec::builder(
		pezkuwichain::WASM_BINARY
			.ok_or("Pezkuwichain development wasm (used for versi) not available")?,
		Default::default(),
	)
	.with_name("Versi Local Testnet")
	.with_id("versi_local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name("versi_local_testnet")
	.with_protocol_id("versi")
	.build())
}
