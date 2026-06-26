use pezkuwi_sdk::*;

use pezsc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use pezsc_service::ChainType;
use serde::{Deserialize, Serialize};
use teyrchain_template_runtime as runtime;

/// Specialized `ChainSpec` for the normal teyrchain runtime.
pub type ChainSpec = pezsc_service::GenericChainSpec<Extensions>;
/// The relay chain that you want to configure this teyrchain to connect to.
pub const RELAY_CHAIN: &str = "pezkuwichain-local";

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
pub struct Extensions {
	/// The relay chain of the Teyrchain.
	#[serde(alias = "relayChain", alias = "RelayChain")]
	pub relay_chain: String,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn pezsc_service::ChainSpec) -> Option<&Self> {
		pezsc_chain_spec::get_extension(chain_spec.extensions())
	}
}

pub fn development_chain_spec() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "UNIT".into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties.insert("ss58Format".into(), 42.into());

	ChainSpec::builder(
		runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: RELAY_CHAIN.into() },
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_preset_name(pezsp_genesis_builder::DEV_RUNTIME_PRESET)
	.with_properties(properties)
	.build()
}

pub fn local_chain_spec() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = pezsc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "UNIT".into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties.insert("ss58Format".into(), 42.into());

	ChainSpec::builder(
		runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: RELAY_CHAIN.into() },
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsc_chain_spec::LOCAL_TESTNET_RUNTIME_PRESET)
	.with_protocol_id("template-local")
	.with_properties(properties)
	.build()
}
