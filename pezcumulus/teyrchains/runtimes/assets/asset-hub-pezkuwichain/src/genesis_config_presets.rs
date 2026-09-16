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
/// - `pez_presale_custody`: Account holding the PEZ presale allocation. The sale happens on
///   the exchange, not on chain, so this allocation is moved by whoever holds it rather than
///   released by a pallet -- and "whoever holds it" is a board rather than a person, so no
///   single key can move it. Where, how much and for how long the PEZ presale runs is that
///   board's to decide. HEZ's presale is not here at all -- it is a pot on this chain that
///   only Parliament can release.
/// - `foreign_assets`: Foreign assets to create at genesis
/// - `foreign_assets_endowed_accounts`: Initial balances for foreign assets
fn asset_hub_pezkuwichain_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	endowment: Balance,
	id: ParaId,
	asset_owner: AccountId,
	founder_account: AccountId,
	pez_presale_custody: AccountId,
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

	// The presale pot's 100M HEZ -- half the supply -- minted here for the same reason and by
	// the same means, but answering to Parliament rather than to the two offices above. It
	// used to be a plain balance on a single key called `Presale_1`; a key holding half the
	// supply is not a presale mechanism, it is a person who can end one.
	let presale_pot: AccountId = PresalePotPalletId::get().into_account_truncating();
	const PRESALE_ALLOCATION: Balance = 100_000_000 * UNITS;

	// The treasury's 40M HEZ, minted into the account `pezpallet_treasury` pays from.
	//
	// It used to be minted on the relay, onto a key called `Treasury_1` -- and the relay has no
	// treasury pallet, so that balance had no governance path at all: the pot with the authority
	// held nothing and the money with no authority held everything. The five spender tracks that
	// decide these payments are here, so the money is here.
	//
	// Less the validators' initial stashes, which are carved out of this share and minted on the
	// relay because the accounts that need them are there. The relay subtracts the same constant
	// from what it escrows, and `hez_allocations_sum_to_200m` asserts both halves.
	let treasury_pot: AccountId = TreasuryPalletId::get().into_account_truncating();
	const TREASURY_ALLOCATION: Balance =
		40_000_000 * UNITS - pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING;

	// The XCM checking account, holding everything that does *not* live here.
	//
	// This chain sets `TeleportTracking` to `MintLocation::Local`, which means an arriving
	// teleport is paid out of this account rather than minted freely -- the ledger entry that
	// makes the invariant "no more returns than was sent out" enforceable. Without a seed the
	// account does not exist, every inbound teleport fails with `NotWithdrawable`, and the
	// sender's balance is gone on the other side while its extrinsic reported success. That
	// is what the Zagros launch did on 2026-09-11: 10 HEZ left the relay and never arrived.
	//
	// The rule is `total supply - what this chain holds`, not a figure: the relay's preset
	// writes the mirror of it, and the two must not be able to drift apart. Today it is
	// 20,001,000 HEZ, the relay's whole share, because that is all the HEZ there is outside
	// this chain -- the People chain mints none, so nothing can reach here except by way of
	// the relay's holdings. It does not need to cover this chain's own pots: sending those
	// out accrues to this same account and raises the ceiling for their return.
	const TOTAL_SUPPLY: Balance = 200_000_000 * UNITS;
	const CHECKING_ACCOUNT_SEED: Balance =
		TOTAL_SUPPLY - (AIRDROP_ALLOCATION + PRESALE_ALLOCATION + TREASURY_ALLOCATION);
	let checking_account: AccountId = crate::PezkuwiXcm::check_account();

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, endowment))
				.chain(core::iter::once((airdrop_pot, AIRDROP_ALLOCATION)))
				.chain(core::iter::once((presale_pot, PRESALE_ALLOCATION)))
				.chain(core::iter::once((treasury_pot, TREASURY_ALLOCATION)))
				.chain(core::iter::once((checking_account, CHECKING_ACCOUNT_SEED)))
				.collect(),
		},
		// The account the founder's pot pays when the gate fires -- whatever this preset
		// chose above, so a development chain keeps its development founder.
		pez_treasury: PezTreasuryConfig { founder: Some(founder_account.clone()) },
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
				// The founder's PEZ waits in a keyless pot, not in the founder's account. It
				// leaves when the population gate fires and the citizens' payments start --
				// the same latch, not a parallel schedule. See `pezpallet_pez_treasury`'s
				// `FounderPotId` and `do_initialize_treasury`.
				(
					PEZ_ASSET_ID,
					PezFounderPotId::get().into_account_truncating(),
					PEZ_FOUNDER_ALLOCATION
				),
				// Presale allocation: 1.875% = 93,750,000 PEZ. Sold on the exchange, so it
				// is held by an account that can move it, not by a pallet -- and that account
				// answers to a board rather than to a key.
				(PEZ_ASSET_ID, pez_presale_custody.clone(), PEZ_PRESALE_ALLOCATION),
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
			// MAINNET ACCOUNTS - generated 2026-09-15 for the genesis reset.
			//
			// New keys, not the January set: the reset replaces the ledger, and leaving the
			// old accounts in place would mean it did not replace who holds it. Recorded in
			// `res/genesis/mainnet/mainnet-wallets.json`, which stays out of this repository.
			//
			// Administrator of wHEZ and wUSDT only -- both are minted and burned by live
			// systems, so they need a team that can act. PEZ is deliberately not among
			// them; see `PezAssetTeamId`.
			// SS58: 5FRp6DBpM24irn5mrDeAAUB2ypB3ozu7kjLRvqhJ3JK1rpGn
			let asset_owner: AccountId =
				hex!("94cdd66f332e0c7759fee3e49b3706b3a8cf63b948a642c57f0c8bf0eb16f202").into();
			// SS58: 5DPA5ctyUhFZcLoqNj11w1xEn3QqtDSmUjk4L6YxQNBWiDxS -- the same founder the
			// relay endows; one person, one account, two chains.
			let founder_account: AccountId =
				hex!("3a4eed1ba224f6d76dec6f24da10b850248dc8db5e8de7effcaf25bea977fe7f").into();
			// Custody for the PEZ presale share. No single key holds it: a three-of-five
			// multisig, derived from the signatory set rather than chosen, so the address is a
			// consequence of who signs and cannot drift from it. Who those five are is recorded
			// off-repository -- see `check-chain-key-overlap.py` for why that is not written
			// beside the address.
			let pez_presale_custody: AccountId =
				hex!("ab445602ed2049270de70fcb1d52cb445e5580de63edd8b215f0b2039cb36f1f").into();

			asset_hub_pezkuwichain_genesis(
				// initial collators - 2 Asset Hub collators - Generated 2026-01-29
				vec![
					// Azad (5CoxwDrivErLrWGh2wUBwto4kACphxeqMDsvgUVTBgzwiBdR)
					(
						hex!("20fe4fa9e8289dae29099651f6f525845c5f379eda42c63dad86fe458d16916e")
							.into(),
						hex!("20fe4fa9e8289dae29099651f6f525845c5f379eda42c63dad86fe458d16916e")
							.unchecked_into(),
					),
					// Beritan (5DAgtoFmatt8MVWBRfk2u6JGgiwRXquavjfXKBSb3x2eQky4)
					(
						hex!("30cc7692d41b1119dd5f67b1e04d21e8a46c1aa76aa3cd237db8b8b9aa742b22")
							.into(),
						hex!("30cc7692d41b1119dd5f67b1e04d21e8a46c1aa76aa3cd237db8b8b9aa742b22")
							.unchecked_into(),
					),
				],
				Vec::new(),
				ASSET_HUB_PEZKUWICHAIN_ED * 524_288,
				1000.into(),
				asset_owner,
				founder_account,
				pez_presale_custody,
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
			let pez_presale_custody = Sr25519Keyring::Alice.to_account_id();

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
				pez_presale_custody,
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
			let pez_presale_custody = Sr25519Keyring::Alice.to_account_id();

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
				pez_presale_custody,
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

/// The Asset Hub mints its own share and the escrow behind the relay's, and no third thing.
///
/// The mirror of the relay's `the_relay_mints_exactly_its_share`. It exists because the relay's
/// half was written and this half was not: the relay seeded its escrow, this chain seeded none,
/// and `TeleportTracking` here is `MintLocation::Local` -- so every inbound teleport failed with
/// `NotWithdrawable` and the sender's balance was gone on the far side while its extrinsic
/// reported success. Zagros launched in that state on 2026-09-11 and lost the first teleport sent
/// through it; this chain had `TeleportTracking` off entirely, which hid the same gap behind a
/// different symptom -- teleports worked and nothing was accounted for.
///
/// Summing alone would not have caught it, and does not catch it now: a sum is equally happy if
/// the escrow is handed to the wrong account. So the last assertion names the account.
#[test]
fn the_asset_hub_mints_exactly_its_share() {
	let bytes = get_preset(&PresetId::from(preset_names::PRESET_GENESIS))
		.expect("the genesis preset exists");
	let genesis: serde_json::Value =
		serde_json::from_slice(&bytes).expect("the preset is valid json");
	let entries = genesis["balances"]["balances"]
		.as_array()
		.expect("the balances patch is an array of (account, amount)");

	let amount = |entry: &serde_json::Value| -> u128 {
		entry[1].as_u64().map(u128::from).unwrap_or_else(|| {
			// Anything past u64 arrives as a JSON number too large for `as_u64`; parse
			// rather than silently skip it.
			entry[1].to_string().parse().expect("a balance is a number")
		})
	};
	let total: u128 = entries.iter().map(amount).sum();

	// Held here: the three pots. Everything else in the supply lives on the relay.
	let held = 40_000_000 * UNITS
		+ 100_000_000 * UNITS
		+ (40_000_000 * UNITS - pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING);
	let escrow = 200_000_000 * UNITS - held;

	assert_eq!(
		total,
		held + escrow,
		"the Asset Hub mints two things and no third: the pots it holds, and the escrow \
		 behind what the relay holds"
	);
	assert_eq!(
		held + escrow,
		200_000_000 * UNITS,
		"and those two are the whole supply -- the relay's twenty million is this escrow \
		 seen from the other side, not a second twenty million"
	);

	let checking =
		serde_json::to_value(crate::PezkuwiXcm::check_account()).expect("an account id serialises");
	let seeded = entries.iter().find(|entry| entry[0] == checking).map(amount).expect(
		"the XCM checking account must be seeded at genesis: `TeleportTracking` is \
			 `MintLocation::Local` here, so an arriving teleport is paid out of it, and an \
			 account that does not exist pays nothing -- every inbound teleport fails with \
			 `NotWithdrawable` and the sender's funds are lost",
	);
	assert_eq!(
		seeded, escrow,
		"the escrow is the supply that does not live here, so that no more can arrive than \
		 exists elsewhere"
	);
}

#[cfg(test)]
mod genesis_ledger {
	use super::*;

	/// Every pot named, and the amount in it -- not just the totals.
	///
	/// `PEZ_TOTAL_SUPPLY` and the debug_assert above it add the constants and are right about
	/// them, but constants are not what a chain mints. A share that moved from one pot to
	/// another, or to a key, passes every constant check untouched: the sum is the same and the
	/// owner is not. That is the failure this pins, because the owner is the whole point --
	/// three of these accounts are keyless by design, and "keyless" is a property of the
	/// address, not of the number beside it.
	#[test]
	fn the_asset_hub_mints_into_the_accounts_it_is_supposed_to() {
		let raw = get_preset(&PresetId::from(preset_names::PRESET_GENESIS))
			.expect("the genesis preset exists");
		let g: serde_json::Value = serde_json::from_slice(&raw).expect("valid json");
		let n = |e: &serde_json::Value, i: usize| -> u128 {
			e[i].as_u64()
				.map(u128::from)
				.unwrap_or_else(|| e[i].to_string().parse().unwrap())
		};

		// A `modl`-prefixed account is a pallet account: no seed produces it, so nothing can
		// sign for it. Checking the prefix is checking that claim, rather than trusting a name.
		let keyless = |addr: &str| -> bool {
			use pezsp_core::crypto::Ss58Codec;
			AccountId::from_ss58check(addr)
				.map(|a| <[u8; 32]>::from(a).starts_with(b"modl"))
				.unwrap_or(false)
		};

		let hez: alloc::collections::BTreeMap<String, u128> = g["balances"]["balances"]
			.as_array()
			.expect("balances")
			.iter()
			.map(|e| (e[0].as_str().unwrap().to_string(), n(e, 1)))
			.collect();

		let airdrop = AirdropPotPalletId::get().into_account_truncating();
		let presale = PresalePotPalletId::get().into_account_truncating();
		let treasury: AccountId = TreasuryPalletId::get().into_account_truncating();
		let checking = crate::PezkuwiXcm::check_account();
		let ss58 = |a: &AccountId| -> String {
			use pezsp_core::crypto::Ss58Codec;
			a.to_ss58check()
		};

		// Written out rather than read back from the constants that built this. A test that
		// recomputes from the same constant proves the code is consistent with itself and
		// nothing about whether the number is the agreed one; changing an allocation has to
		// cost an edit here, which is the whole friction a genesis number deserves.
		//
		// The treasury's 39,999,000 is not a typo: `HEZ_VALIDATOR_FUNDING` (1,000 HEZ) is carved
		// out of its share and minted onto the relay's validator stashes, so the four still sum
		// to two hundred million.
		for (name, acc, want) in [
			("airdrop", &airdrop, 40_000_000 * UNITS),
			("presale", &presale, 100_000_000 * UNITS),
			("treasury", &treasury, 39_999_000 * UNITS),
			("checking", &checking, 20_001_000 * UNITS),
		] {
			let addr = ss58(acc);
			assert_eq!(
				hez.get(&addr).copied(),
				Some(want),
				"the {name} pot must hold exactly its allocation"
			);
			assert!(
				keyless(&addr),
				"the {name} pot must be keyless -- {addr} is not a modl account"
			);
		}
		assert_eq!(hez.len(), 4, "the Asset Hub mints HEZ into four accounts and no fifth");
		assert_eq!(
			hez.values().sum::<u128>(),
			200_000_000 * UNITS,
			"and they are the whole supply, mirrored"
		);

		// PEZ. The founder's share is the one that must NOT be on a key at genesis: it is held
		// in a keyless pot and leaves only through `activate_distribution`, in the same atomic
		// call that starts the citizens' payments. Minting it straight to the founder would
		// silently untie that knot while every total still added up.
		let mut pez: alloc::vec::Vec<(String, u128)> = g["assets"]["accounts"]
			.as_array()
			.expect("asset accounts")
			.iter()
			.filter(|e| n(e, 0) == PEZ_ASSET_ID as u128)
			.map(|e| (e[1].as_str().unwrap().to_string(), n(e, 2)))
			.collect();
		pez.sort_by_key(|(_, v)| core::cmp::Reverse(*v));

		assert_eq!(pez.len(), 3, "PEZ is minted into three accounts");
		assert_eq!(
			pez[0],
			(ss58(&pez_treasury_pot()), 4_812_500_000 * UNITS),
			"the treasury pot holds the treasury allocation and the rewards pool together"
		);
		assert!(keyless(&pez[0].0), "the PEZ treasury pot must be keyless");

		let founder_pot: AccountId = PezFounderPotId::get().into_account_truncating();
		let founder_line = pez.iter().find(|(a, _)| *a == ss58(&founder_pot));
		assert_eq!(
			founder_line.map(|(_, v)| *v),
			Some(93_750_000 * UNITS),
			"the founder's PEZ starts in its pot, not on the founder's key -- it is released by \
			 `activate_distribution`, the same call that starts paying citizens"
		);
		assert!(keyless(&ss58(&founder_pot)), "the founder pot must be keyless");

		assert_eq!(
			pez.iter().map(|(_, v)| *v).sum::<u128>(),
			5_000_000_000 * UNITS,
			"PEZ genesis must mint exactly five billion"
		);
	}
}
