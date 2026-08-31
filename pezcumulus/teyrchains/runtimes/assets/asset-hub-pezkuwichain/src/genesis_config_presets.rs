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

//! # Asset Hub Pezkuwichain Runtime genesis config presets
//!
//! This module contains genesis configuration for:
//! - TrustBackedAssets (Instance1): PEZ (ID:1) and wHEZ (ID:2)
//! - ForeignAssets (Instance2): Bridged assets from other chains
//! - Initial token distributions

use crate::{
	xcm_config::{bridging::to_zagros::ZagrosNetwork, UniversalLocation},
	*,
};
use alloc::{vec, vec::Vec};
use hex_literal::hex;
use pezcumulus_primitives_core::ParaId;
use pezframe_support::build_struct_json_patch;
use pezsp_core::crypto::UncheckedInto;
use pezsp_genesis_builder::PresetId;
use pezsp_keyring::Sr25519Keyring;
use pezsp_runtime::traits::AccountIdConversion;
use testnet_teyrchains_constants::pezkuwichain::{currency::UNITS, xcm_version::SAFE_XCM_VERSION};
use teyrchains_common::{AccountId, AssetIdForTrustBackedAssets, AuraId};
use xcm::latest::prelude::*;
use xcm_builder::GlobalConsensusConvertsFor;
use xcm_executor::traits::ConvertLocation;

const ASSET_HUB_PEZKUWICHAIN_ED: Balance = ExistentialDeposit::get();

/// The treasury's own account, derived from `PezTreasuryPalletId`.
///
/// No seed produces this address, which is the whole point: the state's PEZ is not held by
/// anyone who could be persuaded, compelled or compromised into moving it. It leaves only
/// through `pezpallet-pez-treasury`'s monthly release, on the schedule written into the
/// runtime.
fn pez_treasury_pot() -> AccountId {
	PezTreasuryPalletId::get().into_account_truncating()
}

/// The account that owns the PEZ asset -- see `PezAssetTeamId` for why it is keyless.
fn pez_asset_team() -> AccountId {
	PezAssetTeamId::get().into_account_truncating()
}

// ============================================================================
// PEZ TOKEN CONSTANTS
// ============================================================================

/// PEZ Token Asset ID - Governance token with fixed 5B supply
pub const PEZ_ASSET_ID: AssetIdForTrustBackedAssets = 1;

/// Wrapped HEZ (wHEZ) Asset ID - Used by TokenWrapper pezpallet
pub const WHEZ_ASSET_ID: AssetIdForTrustBackedAssets = 2;

/// wUSDT Asset ID - Wrapped USDT (1:1 backed by Polkadot USDT or TRC20 USDT)
/// Using 1000 to match chains.json configuration in pezWallet
pub const WUSDT_ASSET_ID: AssetIdForTrustBackedAssets = 1000;

/// Where permissionless asset creation starts. Everything below is reserved for the assets
/// genesis creates above. `pezpallet_assets` enforces this as the *only* id `force_create`
/// will accept while it is set, so the benchmark helper has to hand out this same value.
pub const FIRST_AUTO_ASSET_ID: AssetIdForTrustBackedAssets = 1001;

/// PEZ Token decimals (same as HEZ)
pub const PEZ_DECIMALS: u8 = 12;

/// wUSDT decimals (USDT standard: 6 decimals)
pub const WUSDT_DECIMALS: u8 = 6;

/// Treasury allocation: 20.25% = 1,012,500,000 PEZ
pub const PEZ_TREASURY_ALLOCATION: Balance = 1_012_500_000 * UNITS;

/// Founder allocation: 1.875% = 93,750,000 PEZ
pub const PEZ_FOUNDER_ALLOCATION: Balance = 93_750_000 * UNITS;

/// Presale allocation: 1.875% = 93,750,000 PEZ
pub const PEZ_PRESALE_ALLOCATION: Balance = 93_750_000 * UNITS;

/// Rewards pool: 76% = 3,800,000,000 PEZ (distributed via sentetik halving)
pub const PEZ_REWARDS_POOL: Balance = 3_800_000_000 * UNITS;

/// Total PEZ supply: 5 Billion (derived from allocations)
pub const PEZ_TOTAL_SUPPLY: Balance =
	PEZ_TREASURY_ALLOCATION + PEZ_FOUNDER_ALLOCATION + PEZ_PRESALE_ALLOCATION + PEZ_REWARDS_POOL;

// Compile-time verification that total equals expected 5 billion
const _: () = assert!(
	PEZ_TOTAL_SUPPLY == 5_000_000_000 * UNITS,
	"PEZ allocations must sum to exactly 5 billion tokens"
);

/// Genesis configuration for Asset Hub Pezkuwichain
///
/// # Parameters
/// - `invulnerables`: Initial collators with their Aura keys
/// - `endowed_accounts`: Accounts to receive initial HEZ balance
/// - `endowment`: HEZ amount for each endowed account
/// - `id`: Teyrchain ID
/// - `asset_owner`: Account that administers the wHEZ and wUSDT assets, both of which are
///   minted and burned by live systems and so need a team that can act. PEZ is deliberately
///   not among them -- see `pez_asset_team()`.
/// - `founder_account`: Account holding the Founder PEZ allocation. This one is genuinely
///   owned -- it is property, and what happens to it is the founder's to decide.
/// - `presale_account`: Account holding the presale allocation. Owned, like the founder's:
///   the sale happens on the exchange, not on chain, so this allocation is moved by whoever
///   holds it rather than released by a pallet. Once the sale is done the account has no
///   further role.
/// - `foreign_assets`: Foreign assets to create at genesis
/// - `foreign_assets_endowed_accounts`: Initial balances for foreign assets
fn asset_hub_pezkuwichain_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	endowment: Balance,
	id: ParaId,
	asset_owner: AccountId,
	founder_account: AccountId,
	presale_account: AccountId,
	foreign_assets: Vec<(Location, AccountId, Balance)>,
	foreign_assets_endowed_accounts: Vec<(Location, AccountId, Balance)>,
	dev_stakers: Option<(u32, u32)>,
) -> serde_json::Value {
	// Verify total PEZ minted at genesis equals PEZ_TOTAL_SUPPLY (5 billion)
	debug_assert_eq!(
		PEZ_TREASURY_ALLOCATION
			+ PEZ_REWARDS_POOL
			+ PEZ_FOUNDER_ALLOCATION
			+ PEZ_PRESALE_ALLOCATION,
		PEZ_TOTAL_SUPPLY,
		"PEZ genesis allocations must equal total supply"
	);

	// The airdrop pot's 40M HEZ, minted here rather than on the relay.
	//
	// The pot is `pezpallet_treasury`'s second instance and its account is derived from a
	// pallet id, so nobody holds a key to it -- the balance can only leave through an
	// approved spend, which the People chain authorises with two signatures under a million
	// HEZ and three above. Minting it straight here means no key ever holds the amount and no
	// post-launch transfer has to be remembered; the relay's `hez_allocations_sum_to_200m`
	// asserts the other side of this split.
	let airdrop_pot: AccountId = AirdropPotPalletId::get().into_account_truncating();
	const AIRDROP_ALLOCATION: Balance = 40_000_000 * UNITS;

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, endowment))
				.chain(core::iter::once((airdrop_pot, AIRDROP_ALLOCATION)))
				.collect(),
		},
		teyrchain_info: TeyrchainInfoConfig { teyrchain_id: id },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ASSET_HUB_PEZKUWICHAIN_ED * 16,
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
		// Prevent automatic election before validators are staked.
		// After staking setup, trigger manually with force_new_era().
		staking: StakingConfig {
			force_era: pezpallet_staking_async::Forcing::ForceNone,
			// Synthetic stakers, for the presets that ask for them. The multi-block election
			// benchmarks assert that a snapshot page is FULL -- `TargetSnapshotPerBlock` is
			// `MaxValidatorSet`, i.e. 1000 -- and a genesis with no stakers makes that
			// assertion fail before any weight is taken, which is why
			// `pezpallet_election_provider_multi_block` and its verifier had no measured
			// weights at all. Only the dev and local presets set this; the real genesis is a
			// deliberate design and does not get 25k invented nominators.
			dev_stakers,
			..Default::default()
		},

		// ====================================================================
		// TrustBackedAssets (Instance1) - PEZ, wHEZ, and wUSDT tokens
		// ====================================================================
		assets: AssetsConfig {
			// Asset definitions: (id, owner, is_sufficient, min_balance)
			assets: vec![
				// PEZ Token - Governance token with 5B fixed supply.
				// The asset team is keyless by design -- see `PezAssetTeamId`.
				(PEZ_ASSET_ID, pez_asset_team(), true, 1),
				// wHEZ Token - Wrapped HEZ for DeFi operations
				(WHEZ_ASSET_ID, asset_owner.clone(), true, 1),
				// wUSDT - Wrapped USDT (1:1 backed by Polkadot USDT or TRC20 USDT)
				// Min balance: 10_000 (0.01 USDT with 6 decimals)
				(WUSDT_ASSET_ID, asset_owner.clone(), true, 10_000),
			],
			// Asset metadata: (id, name, symbol, decimals)
			metadata: vec![
				(PEZ_ASSET_ID, b"Pez Token".to_vec(), b"PEZ".to_vec(), PEZ_DECIMALS),
				(WHEZ_ASSET_ID, b"Wrapped HEZ".to_vec(), b"wHEZ".to_vec(), PEZ_DECIMALS),
				(WUSDT_ASSET_ID, b"Wrapped USDT".to_vec(), b"wUSDT".to_vec(), WUSDT_DECIMALS),
			],
			// Initial balances: (asset_id, account, balance)
			accounts: vec![
				// Treasury: 20.25% + 76% rewards pool = 4,812,500,000 PEZ, held by the
				// treasury pallet's own keyless account. Released monthly, halving every 48
				// releases; nothing else can move it.
				(PEZ_ASSET_ID, pez_treasury_pot(), PEZ_TREASURY_ALLOCATION + PEZ_REWARDS_POOL),
				// Founder allocation: 1.875% = 93,750,000 PEZ. Property, not treasury.
				(PEZ_ASSET_ID, founder_account.clone(), PEZ_FOUNDER_ALLOCATION),
				// Presale allocation: 1.875% = 93,750,000 PEZ. Sold on the exchange, so it
				// is held by an account that can move it, not by a pallet.
				(PEZ_ASSET_ID, presale_account.clone(), PEZ_PRESALE_ALLOCATION),
				// wHEZ starts with 0 balance - only created via TokenWrapper
				// wUSDT starts with 0 balance - minted via Custodial Bridge
			],
			// Next asset ID after PEZ (1), wHEZ (2), and wUSDT (1000)
			next_asset_id: Some(FIRST_AUTO_ASSET_ID),
			..Default::default()
		},

		// ====================================================================
		// ForeignAssets (Instance2) - Bridged assets from other chains
		// ====================================================================
		foreign_assets: ForeignAssetsConfig {
			assets: foreign_assets
				.into_iter()
				.map(|asset| (asset.0.try_into().unwrap(), asset.1, false, asset.2))
				.collect(),
			accounts: foreign_assets_endowed_accounts
				.into_iter()
				.map(|asset| (asset.0.try_into().unwrap(), asset.1, asset.2))
				.collect(),
			..Default::default()
		},
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
		// Uses hardcoded hex keys for collators, and the wallets generated on 2026-01-29
		// for the founder and the presale holder. The treasury needs no address: it is a
		// keyless pallet account, and so is the PEZ asset team.
		// ====================================================================
		PRESET_GENESIS => {
			// MAINNET ACCOUNTS - NEW SECURE WALLETS (2026-01-29)
			// Administrator of wHEZ and wUSDT only -- both are minted and burned by live
			// systems, so they need a team that can act. PEZ is deliberately not among
			// them; see `PezAssetTeamId`.
			// Asset_Admin_1: 5EhCpn82QtdU53MF6PoNFrKHgSrsfcAxFTMwrn3JYf9dioQw
			let asset_owner: AccountId =
				hex!("744ed0812d6096827376b4625fe4f840d4950d5aef0ab12902e64c444c8e9d29").into();
			// Founder_Satoshi_Qazi_Muhammed: 5CyuFfbF95rzBxru7c9yEsX4XmQXUxpLUcbj9RLg9K1cGiiF
			let founder_account: AccountId =
				hex!("28925ed8b4c0c95402b31563251fd318414351114b1c7797ee788666d27d6305").into();
			// Presale_1: 5Fs1VXbPVvmHAaQ8a7bKcdJ8h8c1mgJKLJ6Pwce69fSqhLJ5
			// Holds the whole presale allocation. Presale_2 and Presale_3 exist in the same
			// wallet set if the allocation is ever to be split across several holders.
			let presale_account: AccountId =
				hex!("a8055af9df1db60bea4277f7e91157246a6245123564bff10435f461f284bf55").into();

			asset_hub_pezkuwichain_genesis(
				// initial collators - 2 Asset Hub collators - Generated 2026-01-29
				vec![
					// Azad (5Et1WgtNjUdMxyvHjAKGN8Nq1ivhUyANYjwKpCL8a46D8mCp)
					(
						hex!("7c8c6f463d124a601fbc7d425daad82651193f35730957982519dbcff6d55f71")
							.into(),
						hex!("7c8c6f463d124a601fbc7d425daad82651193f35730957982519dbcff6d55f71")
							.unchecked_into(),
					),
					// Beritan (5F4GeiJE2oBcPdxfeYfWL4bu4iJfduzJk4aHhttemwhpscpQ)
					(
						hex!("845fd9541c46c3dc4325ddcbae06596382771d943f49d9659bdbbed4abd4eb09")
							.into(),
						hex!("845fd9541c46c3dc4325ddcbae06596382771d943f49d9659bdbbed4abd4eb09")
							.unchecked_into(),
					),
				],
				Vec::new(),
				ASSET_HUB_PEZKUWICHAIN_ED * 524_288,
				1000.into(),
				asset_owner,
				founder_account,
				presale_account,
				vec![],
				vec![],
				None,
			)
		},

		// ====================================================================
		// LOCAL TESTNET PRESET - For local multi-node testing (Alice + Bob)
		// ====================================================================
		pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => {
			// For local testnet, Alice acts as treasury/founder/presale
			let asset_owner = Sr25519Keyring::Alice.to_account_id();
			let founder_account = Sr25519Keyring::Alice.to_account_id();
			let presale_account = Sr25519Keyring::Alice.to_account_id();

			asset_hub_pezkuwichain_genesis(
				// initial collators.
				vec![
					(Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
					(Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
				],
				Sr25519Keyring::well_known().map(|x| x.to_account_id()).collect(),
				testnet_teyrchains_constants::pezkuwichain::currency::UNITS * 1_000_000,
				1000.into(),
				asset_owner,
				founder_account,
				presale_account,
				vec![
					// bridged ZGR
					(
						Location::new(2, [GlobalConsensus(ZagrosNetwork::get())]),
						GlobalConsensusConvertsFor::<UniversalLocation, AccountId>::convert_location(
							&Location { parents: 2, interior: [GlobalConsensus(ZagrosNetwork::get())].into() },
						)
						.unwrap(),
						10_000_000,
					),
				],
				vec![
					// bridged ZGR to Bob
					(
						Location::new(2, [GlobalConsensus(ZagrosNetwork::get())]),
						Sr25519Keyring::Bob.to_account_id(),
						10_000_000 * 4096 * 4096,
					),
				],
				Some((1000, 25_000)),
			)
		},

		// ====================================================================
		// DEV PRESET - For single-node development (Alice only)
		// ====================================================================
		pezsp_genesis_builder::DEV_RUNTIME_PRESET => {
			// For dev, Alice acts as all special accounts
			let asset_owner = Sr25519Keyring::Alice.to_account_id();
			let founder_account = Sr25519Keyring::Alice.to_account_id();
			let presale_account = Sr25519Keyring::Alice.to_account_id();

			asset_hub_pezkuwichain_genesis(
				// initial collators.
				vec![(
					Sr25519Keyring::Alice.to_account_id(),
					Sr25519Keyring::Alice.public().into(),
				)],
				vec![
					Sr25519Keyring::Alice.to_account_id(),
					Sr25519Keyring::Bob.to_account_id(),
					Sr25519Keyring::AliceStash.to_account_id(),
					Sr25519Keyring::BobStash.to_account_id(),
				],
				UNITS * 1_000_000,
				1000.into(),
				asset_owner,
				founder_account,
				presale_account,
				vec![],
				vec![],
				Some((1000, 25_000)),
			)
		},

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
