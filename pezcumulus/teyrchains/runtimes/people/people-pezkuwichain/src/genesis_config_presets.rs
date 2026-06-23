// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # People Pezkuwichain Runtime genesis config presets
//!
//! This module contains genesis configuration for:
//! - IdentityKyc: Founding citizens (founder account starts as Approved citizen)
//! - Collator selection and session keys
//! - Initial balance distributions

use crate::*;
use alloc::{vec, vec::Vec};
use hex_literal::hex;
use pezcumulus_primitives_core::ParaId;
use pezframe_support::build_struct_json_patch;
use pezsp_core::{crypto::UncheckedInto, H256};
use pezsp_genesis_builder::PresetId;
use pezsp_keyring::Sr25519Keyring;
use testnet_teyrchains_constants::pezkuwichain::{
	currency::UNITS as HEZ, xcm_version::SAFE_XCM_VERSION,
};
use teyrchains_common::{AccountId, AuraId};

const PEOPLE_PEZKUWICHAIN_ED: Balance = ExistentialDeposit::get();
const PEOPLE_PARA_ID: ParaId = ParaId::new(1004);

// ============================================================================
// FOUNDING CITIZEN IDENTITY HASH
// ============================================================================

/// Default identity hash for founding citizens
/// This is a placeholder hash - real citizens will update their identity through the KYC process
/// Hash format: keccak256(json_identity_data)
fn default_founding_citizen_identity_hash() -> H256 {
	// A default hash representing "Genesis Founding Citizen"
	H256::from(hex!("0000000000000000000000000000000000000000000000000000000000000001"))
}

/// Genesis configuration for People Pezkuwichain
///
/// # Parameters
/// - `invulnerables`: Initial collators with their Aura keys
/// - `endowed_accounts`: Accounts to receive initial HEZ balance
/// - `endowment`: HEZ amount for each endowed account
/// - `id`: Parachain ID
/// - `founding_citizens`: Accounts that start as Approved citizens (can accept referrals)
/// - `founding_citizen`: The account that receives NFT #0 and Collection 0 ownership
fn people_pezkuwichain_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	endowment: Balance,
	id: ParaId,
	founding_citizens: Vec<(AccountId, H256)>,
	founding_citizen: Option<AccountId>,
) -> serde_json::Value {
	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, endowment)).collect(),
		},
		teyrchain_info: TeyrchainInfoConfig { teyrchain_id: id },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: PEOPLE_PEZKUWICHAIN_ED * 16,
		},
		session: SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),          // account id
						acc,                  // validator id
						SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		pezkuwi_xcm: PezkuwiXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },

		// ====================================================================
		// IdentityKyc - Founding Citizens
		// ====================================================================
		// These accounts start with Approved status and can accept referrals immediately
		// This solves the chicken-egg problem: first citizens need to exist for others to join
		identity_kyc: IdentityKycConfig { founding_citizens, _phantom: Default::default() },

		// ====================================================================
		// Tiki - NFT Collection 0 + Founding Citizen NFT #0
		// ====================================================================
		// Creates Collection 0 in pezpallet_nfts and mints NFT #0 for the founder
		// This is required before any citizenship NFTs can be minted
		tiki: TikiConfig { founding_citizen },
	})
}

/// Encapsulates names of predefined presets.
mod preset_names {
	pub const PRESET_GENESIS: &str = "genesis";
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	use preset_names::*;
	let patch = match id.as_ref() {
		// ====================================================================
		// GENESIS PRESET - For mainnet or production use
		// Uses hardcoded hex keys for collators
		// Founder account is the founding citizen
		// ====================================================================
		PRESET_GENESIS => {
			// MAINNET FOUNDER ACCOUNT - NEW SECURE WALLET (2026-01-29)
			// Founder_Satoshi_Qazi_Muhammed: 5CyuFfbF95rzBxru7c9yEsX4XmQXUxpLUcbj9RLg9K1cGiiF
			let founder_account: AccountId =
				hex!("28925ed8b4c0c95402b31563251fd318414351114b1c7797ee788666d27d6305").into();

			people_pezkuwichain_genesis(
				// initial collators - 2 People Chain collators - Generated 2026-01-29
				vec![
					// Erin (5G4iuN7MvkhdwN4ikZd9uijBzxV78LUWQro3rc9HrfWWzeuS)
					(
						hex!("b0f474e2f94868485e7269e503d6b327af392449c0878670021365ac7e173206")
							.into(),
						hex!("b0f474e2f94868485e7269e503d6b327af392449c0878670021365ac7e173206")
							.unchecked_into(),
					),
					// Firaz (5EeXnoiPoXko3Hqggy74oSgxCFKpkNoppqTcV9MWUQAtmZHj)
					(
						hex!("7244ec68c6f873e386ef8039ad6e9436e5e97c0d28bab4499090b9443034eb04")
							.into(),
						hex!("7244ec68c6f873e386ef8039ad6e9436e5e97c0d28bab4499090b9443034eb04")
							.unchecked_into(),
					),
				],
				Vec::new(),
				PEOPLE_PEZKUWICHAIN_ED * 524_288,
				PEOPLE_PARA_ID,
				// Founding citizens: Founder starts as Approved citizen
				vec![(founder_account.clone(), default_founding_citizen_identity_hash())],
				// Founding citizen gets NFT #0 and Collection 0 ownership
				Some(founder_account),
			)
		},

		// ====================================================================
		// LOCAL TESTNET PRESET - For local multi-node testing (Alice + Bob)
		// ====================================================================
		pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => people_pezkuwichain_genesis(
			// initial collators.
			vec![
				(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
				(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
			],
			Sr25519Keyring::well_known().map(|x| x.to_account_id()).collect(),
			HEZ * 1_000_000,
			PEOPLE_PARA_ID,
			// Founding citizens: Alice and Bob are founding citizens for testing
			vec![
				(Sr25519Keyring::Alice.to_account_id(), default_founding_citizen_identity_hash()),
				(Sr25519Keyring::Bob.to_account_id(), default_founding_citizen_identity_hash()),
			],
			// Alice gets NFT #0 for testing
			Some(Sr25519Keyring::Alice.to_account_id()),
		),

		// ====================================================================
		// DEV PRESET - For single-node development (Alice only)
		// ====================================================================
		pezsp_genesis_builder::DEV_RUNTIME_PRESET => people_pezkuwichain_genesis(
			// initial collators.
			vec![(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into())],
			vec![
				Sr25519Keyring::Alice.to_account_id(),
				Sr25519Keyring::Bob.to_account_id(),
				Sr25519Keyring::AliceStash.to_account_id(),
				Sr25519Keyring::BobStash.to_account_id(),
			],
			HEZ * 1_000_000,
			PEOPLE_PARA_ID,
			// Founding citizen: Alice is the founding citizen for dev
			vec![(Sr25519Keyring::Alice.to_account_id(), default_founding_citizen_identity_hash())],
			// Alice gets NFT #0 for dev
			Some(Sr25519Keyring::Alice.to_account_id()),
		),

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
	use preset_names::*;
	vec![
		PresetId::from(PRESET_GENESIS),
		PresetId::from(pezsp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}
