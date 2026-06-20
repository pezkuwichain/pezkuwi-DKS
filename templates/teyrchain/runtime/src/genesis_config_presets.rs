use crate::{
	AccountId, BalancesConfig, CollatorSelectionConfig, PezkuwiXcmConfig, RuntimeGenesisConfig,
	SessionConfig, SessionKeys, SudoConfig, TeyrchainInfoConfig, EXISTENTIAL_DEPOSIT,
};

use alloc::{vec, vec::Vec};

use pezkuwi_sdk::{pezstaging_xcm as xcm, *};

use pezcumulus_primitives_core::ParaId;
use pezframe_support::build_struct_json_patch;
use pezsp_genesis_builder::PresetId;
use pezsp_keyring::Sr25519Keyring;
use serde_json::Value;
use teyrchains_common::AuraId;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
/// Teyrchain id used for genesis config presets of teyrchain template.
#[docify::export_content]
pub const TEYRCHAIN_ID: u32 = 1000;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> SessionKeys {
	SessionKeys { aura: keys }
}

fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	root: AccountId,
	id: ParaId,
) -> Value {
	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1u128 << 60))
				.collect::<Vec<_>>(),
		},
		teyrchain_info: TeyrchainInfoConfig { teyrchain_id: id },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect::<Vec<_>>(),
			candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
		},
		session: SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura): (AccountId, AuraId)| {
					(
						acc.clone(),                 // account id
						acc,                         // validator id
						template_session_keys(aura), // session keys
					)
				})
				.collect::<Vec<_>>(),
		},
		pezkuwi_xcm: PezkuwiXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },
		sudo: SudoConfig { key: Some(root) },
	})
}

fn local_testnet_genesis() -> Value {
	testnet_genesis(
		// initial collators.
		vec![
			(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
			(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
		],
		Sr25519Keyring::well_known()
			.map(|k: Sr25519Keyring| k.to_account_id())
			.collect(),
		Sr25519Keyring::Alice.to_account_id(),
		TEYRCHAIN_ID.into(),
	)
}

fn development_config_genesis() -> Value {
	testnet_genesis(
		// initial collators.
		vec![
			(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
			(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
		],
		Sr25519Keyring::well_known()
			.map(|k: Sr25519Keyring| k.to_account_id())
			.collect(),
		Sr25519Keyring::Alice.to_account_id(),
		TEYRCHAIN_ID.into(),
	)
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<vec::Vec<u8>> {
	let patch = match id.as_ref() {
		pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => local_testnet_genesis(),
		pezsp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
	vec![
		PresetId::from(pezsp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}
