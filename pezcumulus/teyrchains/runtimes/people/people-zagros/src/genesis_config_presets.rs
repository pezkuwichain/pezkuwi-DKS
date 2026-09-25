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
use pezpallet_tiki::Tiki;
use pezsp_core::{crypto::UncheckedInto, H256};
use pezsp_genesis_builder::PresetId;
use pezsp_keyring::Sr25519Keyring;
use testnet_teyrchains_constants::zagros::{currency::UNITS as HEZ, xcm_version::SAFE_XCM_VERSION};
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
/// - `id`: Teyrchain ID
/// - `founding_citizens`: Accounts that start as Approved citizens (can accept referrals)
/// - `founding_citizen`: The account that receives NFT #0 and Collection 0 ownership
fn people_pezkuwichain_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	endowment: Balance,
	id: ParaId,
	founding_citizens: Vec<(AccountId, H256)>,
	founding_citizen: Option<AccountId>,
	// The bench and the offices the state starts with. Empty is a decision, not a default:
	// the register is the court's to write, so a chain launched with no court cannot revoke a
	// citizenship until an election seats one.
	founding_government: Vec<(AccountId, Tiki)>,
) -> serde_json::Value {
	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, endowment))
				// The founding office pays for its own calls, and the account funded is read
				// out of the bench rather than passed in beside it. Two parameters that have to
				// name the same key are two parameters that can disagree, and the disagreement
				// here is silent both ways: a funded account holding no tiki signs nothing, and
				// a seated Serok with no balance is a chain that cannot be founded at all.
				//
				// `filter` on what is already endowed, because `pezpallet_balances` panics on a
				// duplicate account at genesis -- and on the local preset the Serok is Alice,
				// who is endowed by the line above.
				.chain(
					founding_government
						.iter()
						.filter(|(who, tiki)| {
							*tiki == Tiki::Serok && !endowed_accounts.contains(who)
						})
						.map(|(who, _)| {
							(
								who.clone(),
								pezkuwichain_runtime_constants::currency::HEZ_FOUNDING_OFFICE_FUNDING,
							)
						}),
				)
				// The accumulation account at its existential deposit. Fees and dust reach it
				// through `resolve`, which refuses a deposit that would leave an account below
				// the deposit -- unfunded, every small fee is turned away and burned. Carved out
				// of the founder's line on the relay and escrowed there, like the office budget.
				.chain(core::iter::once((
					pezpallet_accumulate_and_forward::Pezpallet::<Runtime>::accumulation_account(),
					pezkuwichain_runtime_constants::currency::HEZ_ACCUMULATION_PEOPLE,
				)))
				.collect(),
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
		tiki: TikiConfig { founding_citizen, founding_government },
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
			// Zagros founder: 5Fhjq3KmYHgChQ7mfaRGz3hotzC1XTSsGXK8HChaid5sUrNS
			// Zagros's own, from the Zagros wallet set. The Pezkuwichain hub's value here
			// belongs to mainnet; sharing it would put one key on two chains.
			// The founding hand: holder of `Tiki::Serok`, and the only origin that can
			// write this chain's register on day one.
			//
			// `TheRegisterIsNotWritableFromAbroad` drops every register call arriving over
			// XCM, so the relay's sudo cannot seat the founding Parliament; this runtime has
			// no sudo pallet of its own; and its Root track wants a referendum, which wants a
			// roll that does not exist yet. What is left is `ensure_root_or_serok`.
			//
			// Deliberately not the founder above. That account holds HEZ and citizen NFT #0;
			// this one holds the executive. One key for both would make one compromise take
			// the allocation and the register together.
			//
			// SS58: 5FP4tMpDaoURCktetrc7LeCm8cncHdE78L5L3CNDVX5hZrJ2
			// Path: //zagros//office//serok in res/genesis/zagros/zagros-wallets.json
			let serok_account: AccountId =
				hex!("92b5e877ed42d604c921154f456dd2effe85991d07c1577b9709beb7dee41119").into();

			let founder_account: AccountId =
				hex!("a0f36b1ed6006a5ed8e492a1a5c5820cec6cb6feba17282f0bd41faacc1f8c12").into();

			people_pezkuwichain_genesis(
				// initial collators - 2 People Chain collators - Generated 2026-01-29
				vec![
					// Zagros people collator 1 (5Cr51WBkgnrDtDVNmp19JHLfZ9nnSJbTPqRpU4w1cVXXvDFB)
					(
						hex!("22993f9027fe8bbdfaa5a3815922144539b03590d3e8aee15503ba19211ee42c")
							.into(),
						hex!("22993f9027fe8bbdfaa5a3815922144539b03590d3e8aee15503ba19211ee42c")
							.unchecked_into(),
					),
					// Zagros people collator 2 (5GYcxRhUGpnBkc4NQBtxDR9jEThcRrBrQqyAJFbRUyMvCaar)
					(
						hex!("c63b4ecd42df29151b511f4d9fcc7e2cd4a872b3ae81bd3e206fe0ae5901f719")
							.into(),
						hex!("c63b4ecd42df29151b511f4d9fcc7e2cd4a872b3ae81bd3e206fe0ae5901f719")
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
				// The founding government: one office, and the chain cannot be founded
				// without it. The Serok named above signs `seat_founding_parliament`, and
				// nothing else on this chain can.
				//
				// The bench itself stays empty, and that is still a decision rather than an
				// omission: two hundred and one members and eleven judges are people, not
				// keys, and they are seated by extrinsic once the chain is running. Until
				// that call lands the register cannot be corrected -- the cost of not
				// pretending an unelected court exists.
				vec![(serok_account, Tiki::Serok)],
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
			// The founding register of the local chain.
			//
			// Five rather than two, and the reason is the rehearsal: a citizen is the unit of
			// almost everything this chain does. Trust is only computed for one
			// (`calculate_trust_score` refuses a non-citizen outright), and without trust there
			// is no vote, so a bench seated from non-citizens can hold a seat and still not
			// answer a referendum. Measured 2026-09-18: the rehearsal seated five, three of
			// them were not on the register, and the runs died waiting for a trust score that
			// could never be computed.
			//
			// This is the *local* preset -- a fixture whose job is to make the rehearsal
			// possible. The genesis preset seats the founder alone and is untouched.
			vec![
				(Sr25519Keyring::Alice.to_account_id(), default_founding_citizen_identity_hash()),
				(Sr25519Keyring::Bob.to_account_id(), default_founding_citizen_identity_hash()),
				(Sr25519Keyring::Charlie.to_account_id(), default_founding_citizen_identity_hash()),
				(Sr25519Keyring::Dave.to_account_id(), default_founding_citizen_identity_hash()),
				(Sr25519Keyring::Eve.to_account_id(), default_founding_citizen_identity_hash()),
			],
			// Alice gets NFT #0 for testing
			Some(Sr25519Keyring::Alice.to_account_id()),
			// The founding hand, and the local chain cannot be founded without it.
			//
			// `seat_founding_parliament` takes `ensure_root_or_serok`, and neither origin
			// exists here by default: this chain has no sudo pallet, and Root arriving from
			// the relay is dropped by `TheRegisterIsNotWritableFromAbroad` before its origin
			// is even resolved. Its own Root track needs a referendum, which needs a roll.
			// So the first Serok is named here or the register is never written at all.
			//
			// Measured 2026-09-19: the rehearsal drove the whole founding sequence from the
			// relay's sudo and every call was dropped, because three comments in this tree
			// said that was the founding hand and none of them had been checked against the
			// filter.
			vec![(Sr25519Keyring::Alice.to_account_id(), Tiki::Serok)],
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
			vec![],
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

/// The hand the genesis names can also pay.
///
/// Seating an office and funding it are two edits in two places, and the failure when they
/// disagree is the quiet kind: the chain comes up, the register reads as correctly configured,
/// and the first founding call is rejected for a fee. That is not hypothetical -- Zagros
/// launched on 2026-09-14 with an unfunded root key and came up ungovernable until an account
/// was funded by hand, which is not a repair available on a chain whose register is closed to
/// outside help.
///
/// Read out of the production preset. The local preset endows the whole dev keyring, so the
/// same assertion there would pass with the office funding deleted outright.
///
/// The two halves are read from different places on purpose: the seated account comes from
/// `tiki.foundingGovernment` and the balance from `balances.balances`, so the test fails if
/// they ever name different keys.
#[test]
fn the_founding_hand_can_pay_for_the_founding_call() {
	let preset = get_preset(&PresetId::from(preset_names::PRESET_GENESIS))
		.expect("the genesis preset exists");
	let genesis: serde_json::Value =
		serde_json::from_slice(&preset).expect("the preset is valid json");

	let seated = genesis["tiki"]["foundingGovernment"]
		.as_array()
		.expect("the genesis seats a founding government")
		.iter()
		.find(|entry| entry[1] == "Serok")
		.map(|entry| entry[0].clone())
		.expect("the genesis seats a Serok");

	let funded = genesis["balances"]["balances"]
		.as_array()
		.expect("the balances patch is an array of (account, amount)")
		.iter()
		.find(|entry| entry[0] == seated)
		.map(|entry| {
			entry[1]
				.as_u64()
				.map(u128::from)
				.unwrap_or_else(|| entry[1].to_string().parse().expect("a balance is a number"))
		})
		.unwrap_or(0);

	// Asserted against the *Zagros* relay's constant, while the line above is minted from the
	// mainnet one. That is not a slip: this runtime mirrors the mainnet People chain and takes
	// its constants from there (see `Cargo.toml`), but the chain that carves the money out of
	// the founder's share and escrows the two off-relay budgets is the Zagros relay. Two
	// constants that must agree and are reached from different crates is exactly the kind of
	// pair that drifts in silence -- so the test reads one and the code reads the other, and
	// the day they stop matching this goes red instead of the escrow going short.
	assert_eq!(
		funded,
		zagros_runtime_constants::currency::HEZ_FOUNDING_OFFICE_FUNDING,
		"the account this genesis seats as Serok holds {funded} here, so the founding call \
		 it is the only origin for cannot pay its fee"
	);
}

/// The accumulation account starts at this chain's existential deposit, and the relay escrows
/// exactly that.
///
/// Two facts, pinned together because each alone is wrong in a quiet way. Unfunded, the account
/// refuses every deposit smaller than the existential deposit and the credit is burned
/// (measured 2026-09-25: the account did not exist on People on either network). And the
/// relay carves `HEZ_ACCUMULATION_PEOPLE` out of the founder and escrows it for this line
/// without being able to see this chain's existential deposit: if the two ever differ, either
/// the account starts short or the relay's escrow does not match what was minted here.
#[test]
fn the_accumulation_account_starts_at_its_existential_deposit() {
	let ed = <Runtime as pezpallet_balances::Config>::ExistentialDeposit::get();
	assert_eq!(
		pezkuwichain_runtime_constants::currency::HEZ_ACCUMULATION_PEOPLE,
		ed,
		"the relay escrows what it believes this chain's existential deposit is"
	);

	let preset = get_preset(&PresetId::from(preset_names::PRESET_GENESIS))
		.expect("the genesis preset exists");
	let genesis: serde_json::Value =
		serde_json::from_slice(&preset).expect("the preset is valid json");
	let account = serde_json::to_value(
		pezpallet_accumulate_and_forward::Pezpallet::<Runtime>::accumulation_account(),
	)
	.expect("an account id serialises");
	let funded: u128 = genesis["balances"]["balances"]
		.as_array()
		.expect("the balances patch is an array of (account, amount)")
		.iter()
		.filter(|entry| entry[0] == account)
		.map(|entry| {
			entry[1]
				.as_u64()
				.map(u128::from)
				.unwrap_or_else(|| entry[1].to_string().parse().expect("a balance is a number"))
		})
		.sum();
	assert_eq!(funded, ed, "the accumulation account must start at the existential deposit");
}
