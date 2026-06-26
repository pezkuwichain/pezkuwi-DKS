use pez_solochain_template_runtime::WASM_BINARY;
use pezsc_service::ChainType;

/// Specialized `ChainSpec`. This is a specialization of the general Bizinikiwi ChainSpec type.
pub type ChainSpec = pezsc_service::GenericChainSpec;

pub fn development_chain_spec() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_preset_name(pezsp_genesis_builder::DEV_RUNTIME_PRESET)
	.build())
}

pub fn local_chain_spec() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.build())
}
