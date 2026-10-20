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

//! Genesis configs presets for the Pezkuwichain runtime
//!
//! This module contains genesis configuration for:
//! - HEZ token initial distribution (200M genesis supply)
//! - Validator session keys
//! - Initial balance distributions
//!
//! ## HEZ Genesis Distribution (200M Total)
//! - 10% Founder: 20,000,000 HEZ
//! - 50% Presale: 100,000,000 HEZ
//! - 20% Kurdistan Treasury: 40,000,000 HEZ
//! - 20% Airdrop: 40,000,000 HEZ

use crate::{
	BabeConfig, BalancesConfig, ConfigurationConfig, RegistrarConfig, RuntimeGenesisConfig,
	SessionConfig, SessionKeys, StakingAhClientConfig, SudoConfig, BABE_GENESIS_EPOCH_CONFIG,
};
#[cfg(not(feature = "std"))]
use alloc::format;
use alloc::{vec, vec::Vec};
use pezframe_support::build_struct_json_patch;
use pezkuwi_primitives::{
	// HostConfiguration carries the vstaging shape, which added on_demand_queue_max_size;
	// the root re-export still points at v9 and no longer matches the field it fills.
	vstaging::SchedulerParams,
	AccountId,
	AssignmentId,
	ValidatorId,
};
use pezkuwichain_runtime_constants::currency::UNITS as HEZ;
use pezsp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use pezsp_consensus_babe::AuthorityId as BabeId;
use pezsp_consensus_beefy::ecdsa_crypto::AuthorityId as BeefyId;
use pezsp_consensus_grandpa::AuthorityId as GrandpaId;
use pezsp_core::{crypto::get_public_from_string_or_panic, sr25519};
use pezsp_genesis_builder::PresetId;
use pezsp_keyring::Sr25519Keyring;

// ============================================================================
// HEZ TOKEN GENESIS CONSTANTS (Total Supply: 200 Million HEZ)
// ============================================================================

/// Founder allocation: 10% = 20,000,000 HEZ
pub const HEZ_FOUNDER_ALLOCATION: u128 = 20_000_000 * HEZ;

/// Presale allocation: 50% = 100,000,000 HEZ.
///
/// **Minted on the Asset Hub, not here** -- into `PresalePot`, a keyless treasury instance
/// that only Parliament can release. It used to be a plain balance on `Presale_1`, a single
/// key holding half the supply.
pub const HEZ_PRESALE_ALLOCATION: u128 = 100_000_000 * HEZ;

/// Kurdistan Treasury allocation: 20% = 40,000,000 HEZ.
///
/// **Minted on the Asset Hub, not here** -- into the account `pezpallet_treasury` pays from,
/// which is derived from a pallet id and holds no key. It used to be minted on the relay onto
/// `Treasury_1`, and the relay has no treasury pallet: the pot with the authority held nothing
/// and the balance with no authority held everything, reachable by a key or by root and by no
/// vote. The five spender tracks that decide these payments are on the Asset Hub, so the money
/// is now where the authority is. Same reasoning as the airdrop and presale pots above.
pub const HEZ_TREASURY_ALLOCATION: u128 = 40_000_000 * HEZ;

/// Airdrop allocation: 20% = 40,000,000 HEZ.
///
/// **Minted on the Asset Hub, not here** -- into `AirdropPot`, a keyless treasury instance
/// spendable only by the People chain. The number lives in this file because this is where
/// the 200M is split and where anyone changing one share will look; the chain that holds it
/// is a separate question from the share it holds.
pub const HEZ_AIRDROP_ALLOCATION: u128 = 40_000_000 * HEZ;

// ===========================================================================
// COMPILE-TIME VALIDATION: the four shares still sum to 200M.
//
// Unchanged by the airdrop and the presale moving to the Asset Hub: what moved is where a
// share is minted, not how the supply is divided. `hez_allocations_sum_to_200m` below checks
// the other half of that -- which chain mints which share -- because this assert cannot see
// it, and a share that moved chain while keeping its number would pass here in silence.
// ===========================================================================
const _: () = assert!(
	HEZ_FOUNDER_ALLOCATION
		+ HEZ_PRESALE_ALLOCATION
		+ HEZ_TREASURY_ALLOCATION
		+ HEZ_AIRDROP_ALLOCATION
		== 200_000_000 * HEZ,
	"HEZ allocations MUST sum to genesis supply (200M)"
);

/// Helper function to generate stash, controller and session key from seed
fn get_authority_keys_from_seed(
	seed: &str,
) -> (
	AccountId,
	AccountId,
	BabeId,
	GrandpaId,
	ValidatorId,
	AssignmentId,
	AuthorityDiscoveryId,
	BeefyId,
) {
	let keys = get_authority_keys_from_seed_no_beefy(seed);
	(
		keys.0,
		keys.1,
		keys.2,
		keys.3,
		keys.4,
		keys.5,
		keys.6,
		get_public_from_string_or_panic::<BeefyId>(seed),
	)
}

/// Helper function to generate stash, controller and session key from seed
fn get_authority_keys_from_seed_no_beefy(
	seed: &str,
) -> (AccountId, AccountId, BabeId, GrandpaId, ValidatorId, AssignmentId, AuthorityDiscoveryId) {
	(
		get_public_from_string_or_panic::<sr25519::Public>(&format!("{}//stash", seed)).into(),
		get_public_from_string_or_panic::<sr25519::Public>(seed).into(),
		get_public_from_string_or_panic::<BabeId>(seed),
		get_public_from_string_or_panic::<GrandpaId>(seed),
		get_public_from_string_or_panic::<ValidatorId>(seed),
		get_public_from_string_or_panic::<AssignmentId>(seed),
		get_public_from_string_or_panic::<AuthorityDiscoveryId>(seed),
	)
}

fn testnet_accounts() -> Vec<AccountId> {
	Sr25519Keyring::well_known().map(|x| x.to_account_id()).collect()
}

fn pezkuwichain_session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	para_validator: ValidatorId,
	para_assignment: AssignmentId,
	authority_discovery: AuthorityDiscoveryId,
	beefy: BeefyId,
) -> SessionKeys {
	SessionKeys { babe, grandpa, para_validator, para_assignment, authority_discovery, beefy }
}

fn default_teyrchains_host_configuration(
) -> pezkuwi_runtime_teyrchains::configuration::HostConfiguration<pezkuwi_primitives::BlockNumber> {
	use pezkuwi_primitives::{
		node_features::FeatureIndex, AsyncBackingParams, MAX_CODE_SIZE, MAX_POV_SIZE,
	};

	pezkuwi_runtime_teyrchains::configuration::HostConfiguration {
		validation_upgrade_cooldown: 2u32,
		validation_upgrade_delay: 2,
		code_retention_period: 1200,
		max_code_size: MAX_CODE_SIZE,
		max_pov_size: MAX_POV_SIZE,
		max_head_data_size: 32 * 1024,
		max_upward_queue_count: 8,
		max_upward_queue_size: 1024 * 1024,
		max_downward_message_size: 1024 * 1024,
		max_upward_message_size: 50 * 1024,
		max_upward_message_num_per_candidate: 5,
		hrmp_sender_deposit: 0,
		hrmp_recipient_deposit: 0,
		hrmp_channel_max_capacity: 8,
		hrmp_channel_max_total_size: 8 * 1024,
		hrmp_max_teyrchain_inbound_channels: 4,
		hrmp_channel_max_message_size: 1024 * 1024,
		hrmp_max_teyrchain_outbound_channels: 4,
		hrmp_max_message_num_per_candidate: 5,
		dispute_period: 6,
		no_show_slots: 2,
		n_delay_tranches: 25,
		needed_approvals: 2,
		relay_vrf_modulo_samples: 2,
		zeroth_delay_tranche_width: 0,
		minimum_validation_upgrade_delay: 5,
		async_backing_params: AsyncBackingParams {
			max_candidate_depth: 0,
			allowed_ancestry_len: 0,
		},
		node_features: bitvec::vec::BitVec::from_element(
			(1u8 << (FeatureIndex::ElasticScalingMVP as usize))
				| (1u8 << (FeatureIndex::EnableAssignmentsV2 as usize))
				| (1u8 << (FeatureIndex::CandidateReceiptV2 as usize)),
		),
		scheduler_params: SchedulerParams {
			lookahead: 3,
			group_rotation_frequency: 20,
			paras_availability_period: 4,
			// Two cores, one for each system teyrchain: the Asset Hub and People.
			num_cores: 2,
			..Default::default()
		},
		..Default::default()
	}
}

#[test]
fn default_teyrchains_host_configuration_is_consistent() {
	default_teyrchains_host_configuration().panic_if_not_consistent();
}

/// The four allocations still sum to 200M, across two chains rather than one.
///
/// Two are minted here; the airdrop's 40M and the presale's 100M are minted into the Asset
/// Hub's `AirdropPot` and `PresalePot`. The constants stay in this file because this is where
/// the split is decided and where anybody changing one share will look -- moving them to the
/// chain that holds the money would leave half the arithmetic here and half somewhere else.
///
/// One stays whole: the founder's, which is property and has an owner. The other three answer
/// to a body rather than to a key, and the pots that hold them are on the Asset Hub. The
/// treasury was the last to move -- it was minted here, onto a key, while the pallet that
/// spends it has always been there.
///
/// What still starts on the relay out of the treasury's share is the validators' funding, and
/// only because the accounts that need it are here.
#[test]
fn hez_allocations_sum_to_200m() {
	// Two of the founding office's three budgets are minted on the other two chains, so they
	// leave the relay's side of the ledger even though they come out of the founder's share.
	// The genesis validators' stakes (spec K5) come out of it too, and are minted on the
	// Asset Hub where they are bonded.
	let off_relay = 2 * pezkuwichain_runtime_constants::currency::HEZ_FOUNDING_OFFICE_FUNDING
		+ pezkuwichain_runtime_constants::currency::HEZ_GENESIS_STAKE_CARVE_OUT;
	let here = HEZ_FOUNDER_ALLOCATION - off_relay
		+ pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING;
	let elsewhere = HEZ_AIRDROP_ALLOCATION + HEZ_PRESALE_ALLOCATION + HEZ_TREASURY_ALLOCATION
		- pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING
		+ off_relay;
	assert_eq!(
		here,
		19_702_000 * HEZ,
		"the relay mints the founder's 20M less the two office budgets held on the Asset Hub \
		 and on People and the validators' 297,000 staked there, plus the validators' funding \
		 out of the treasury"
	);
	assert_eq!(here + elsewhere, 200_000_000 * HEZ, "HEZ total supply must equal 200M");
}

/// Root is the multisig its signers can actually sign for.
///
/// The root key is written down as a constant and the signers beside it; nothing on chain ties
/// them together. If a signer is replaced and the key is not, the chain launches with a root
/// nobody can sign for, and there is no way back from that. So the key is derived here the way
/// the multisig pallet derives it, from the signers and the threshold, and must match.
#[test]
fn the_root_key_is_the_signers_multisig() {
	let mut signers: Vec<AccountId> = pezkuwichain_runtime_constants::genesis::SUDO_SIGNERS
		.iter()
		.map(|s| AccountId::from(*s))
		.collect();
	signers.sort();
	let derived = pezpallet_multisig::Pezpallet::<crate::Runtime>::multi_account_id(
		&signers,
		pezkuwichain_runtime_constants::genesis::SUDO_THRESHOLD,
	);
	assert_eq!(
		derived,
		AccountId::from(pezkuwichain_runtime_constants::genesis::SUDO_MULTISIG),
		"SUDO_MULTISIG is not the multisig of SUDO_SIGNERS at SUDO_THRESHOLD"
	);
	let genesis = pezkuwichain_genesis_config();
	let root = genesis["sudo"]["key"].as_str().expect("the genesis names a root key");
	assert_eq!(serde_json::to_value(&derived).expect("an account serialises").as_str(), Some(root));
}

/// Root can pay for its own first call.
///
/// The two supply tests above both stay green whether root is funded or not, because the
/// funding is carved out of the founder's share -- the total never moves. That is the right
/// shape for the supply and the wrong shape for a guard, so the guard is here: it reads the
/// key out of `SudoConfig` and asks what the genesis actually mints to whoever signs for it.
/// Root is a multisig, so that is its three signers, and the multisig itself starts empty.
///
/// Measured 2026-09-14 on Zagros, whose root is a separate key for the same reason: root held nothing, `sudo_schedule_para_initialize`
/// could not pay its fee, and the teyrchains could not be registered until an account was funded
/// by hand. A chain that is up but cannot be governed looks healthy from every angle except this
/// one.
#[test]
fn sudo_starts_with_a_fee_budget() {
	let genesis = pezkuwichain_genesis_config();
	let root = genesis["sudo"]["key"].as_str().expect("the genesis names a root key");

	let amount = |entry: &serde_json::Value| -> u128 {
		entry[1]
			.as_u64()
			.map(u128::from)
			.unwrap_or_else(|| entry[1].to_string().parse().expect("a balance is a number"))
	};
	let balances = genesis["balances"]["balances"]
		.as_array()
		.expect("the balances patch is an array of (account, amount)");
	let minted_to = |who: &str| -> u128 {
		balances.iter().filter(|e| e[0].as_str() == Some(who)).map(amount).sum()
	};

	assert_eq!(minted_to(root), 0, "the multisig pays nothing itself; its signers do");
	let mut to_signers = 0;
	for signer in pezkuwichain_runtime_constants::genesis::SUDO_SIGNERS {
		let signer = serde_json::to_value(AccountId::from(signer)).expect("an account serialises");
		let got = minted_to(signer.as_str().expect("an account serialises to a string"));
		assert_eq!(
			got,
			pezkuwichain_runtime_constants::currency::HEZ_SUDO_SIGNER_FUNDING,
			"every signer must launch with its fee budget -- the one who submits pays"
		);
		to_signers += got;
	}
	assert_eq!(to_signers, pezkuwichain_runtime_constants::currency::HEZ_SUDO_FUNDING);

	// Carved, not added. If someone later funds root by minting extra, the supply tests catch
	// it; if they fund it by taking from somewhere that is not the founder, only this does.
	let founder_line: u128 = balances
		.iter()
		.map(amount)
		.find(|&a| {
			a == HEZ_FOUNDER_ALLOCATION
				- pezkuwichain_runtime_constants::currency::HEZ_SUDO_FUNDING
				- pezkuwichain_runtime_constants::currency::HEZ_FOUNDING_OFFICE_CARVE_OUT
				- pezkuwichain_runtime_constants::currency::HEZ_ACCUMULATION_CARVE_OUT
				- pezkuwichain_runtime_constants::currency::HEZ_COLLATOR_POT_CARVE_OUT
				- pezkuwichain_runtime_constants::currency::HEZ_GENESIS_STAKE_CARVE_OUT
		})
		.unwrap_or(0);
	assert_eq!(
		founder_line,
		HEZ_FOUNDER_ALLOCATION
			- pezkuwichain_runtime_constants::currency::HEZ_SUDO_FUNDING
			- pezkuwichain_runtime_constants::currency::HEZ_FOUNDING_OFFICE_CARVE_OUT
			- pezkuwichain_runtime_constants::currency::HEZ_ACCUMULATION_CARVE_OUT
			- pezkuwichain_runtime_constants::currency::HEZ_COLLATOR_POT_CARVE_OUT
			- pezkuwichain_runtime_constants::currency::HEZ_GENESIS_STAKE_CARVE_OUT,
		"root's budget, the founding office's, the accumulation accounts', the collators' pots and \
		 the genesis validators' stakes come out of the founder's allocation, not on top of it"
	);
}

/// The accumulation account starts at its existential deposit.
///
/// Every fee, every bit of dust and all coretime revenue reach it through `resolve`, which
/// refuses a deposit that would leave the account below the existential deposit. Unfunded,
/// the account turns each such deposit away and the credit is burned -- measured 2026-09-25 on
/// both live chains, where the account did not exist. This reads the genesis the chain is
/// built from, not the constant, so a line dropped from the balances list turns it red.
#[test]
fn the_accumulation_account_starts_at_its_existential_deposit() {
	let genesis = pezkuwichain_genesis_config();
	let account = serde_json::to_value(
		pezpallet_accumulate_and_forward::Pezpallet::<crate::Runtime>::accumulation_account(),
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
	assert_eq!(
		funded,
		<crate::Runtime as pezpallet_balances::Config>::ExistentialDeposit::get(),
		"the accumulation account must start at the existential deposit, or small fees are burned"
	);
}

/// The relay's genesis mints exactly the share it keeps -- to the planck.
///
/// `hez_allocations_sum_to_200m` adds four constants and is right about them, but constants are
/// not what a chain mints. This builds the genesis and adds up what is in it, which is the only
/// way to see the thing that was there before: every validator was funded `STASH * 2` on top of
/// the allocations, so the relay minted 200,000,800 HEZ while asserting 200,000,000.
///
/// It also catches a share that moved chain without its line being deleted -- a hundred million
/// minted here and again on the Asset Hub would pass the constant check untouched.
#[test]
fn the_relay_mints_exactly_its_share() {
	let genesis = pezkuwichain_genesis_config();
	let total: u128 = genesis["balances"]["balances"]
		.as_array()
		.expect("the balances patch is an array of (account, amount)")
		.iter()
		.map(|entry| {
			entry[1].as_u64().map(u128::from).unwrap_or_else(|| {
				// Anything past u64 arrives as a JSON number too large for `as_u64`; parse
				// rather than silently skip it.
				entry[1].to_string().parse().expect("a balance is a number")
			})
		})
		.sum();

	// Owned balances: the founder's, and the validators' funding carved out of the treasury's
	// share. The rest of the treasury is not here -- it is minted into the pot on the Asset Hub
	// that the spender tracks pay from.
	// Less the two office budgets that are minted on the Asset Hub and on People: they are
	// carved from the founder's share but they are not held here.
	// And less the genesis validators' stakes, minted and bonded on the Asset Hub.
	let owned = HEZ_FOUNDER_ALLOCATION
		- 2 * pezkuwichain_runtime_constants::currency::HEZ_FOUNDING_OFFICE_FUNDING
		- pezkuwichain_runtime_constants::currency::HEZ_GENESIS_STAKE_CARVE_OUT
		+ pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING;
	// Escrow: the mirror of what the Asset Hub holds, so a teleport back has something to
	// release. Not new supply -- the same HEZ, represented there and held here.
	let escrow = HEZ_AIRDROP_ALLOCATION + HEZ_PRESALE_ALLOCATION + HEZ_TREASURY_ALLOCATION
		- pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING
		+ 2 * pezkuwichain_runtime_constants::currency::HEZ_FOUNDING_OFFICE_FUNDING
		+ pezkuwichain_runtime_constants::currency::HEZ_GENESIS_STAKE_CARVE_OUT;

	assert_eq!(
		total,
		owned + escrow,
		"the relay mints two things and no third: what it owns, and the escrow behind what the \
		 Asset Hub holds"
	);
	assert_eq!(
		owned + escrow,
		200_000_000 * HEZ,
		"and those two are the whole supply -- the Asset Hub's hundred and forty million is \
		 this escrow seen from the other side, not a second hundred and forty million"
	);
}

fn pezkuwichain_testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
		BeefyId,
	)>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> serde_json::Value {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);

	const ENDOWMENT: u128 = 1_000_000 * HEZ;

	// The XCM checking account, seeded here for the same reason the production preset seeds it.
	//
	// `TeleportTracking` is `MintLocation::Local`, so an arriving teleport is paid out of this
	// account rather than minted. Without a seed the account does not exist, every inbound
	// teleport fails with `NotWithdrawable`, and the sender's balance is gone on the other side
	// while its extrinsic reported success. That is what the Zagros launch did on 2026-09-11 --
	// and a local or dev chain built from this preset could not reproduce it, because the
	// account was missing here too. A defect the testnet cannot show is a defect the testnet
	// cannot catch.
	//
	// The rule is the production one: total supply less what this chain holds. Written as an
	// expression rather than a figure so it cannot drift from the endowments above.
	const TOTAL_SUPPLY: u128 = 200_000_000 * HEZ;
	let owned: u128 = ENDOWMENT.saturating_mul(endowed_accounts.len() as u128);
	let checking_account_seed = TOTAL_SUPPLY.saturating_sub(owned);
	let checking_account: AccountId = crate::XcmPallet::check_account();

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.map(|k| (k.clone(), ENDOWMENT))
				.chain(core::iter::once((checking_account, checking_account_seed)))
				.collect::<Vec<_>>(),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						pezkuwichain_session_keys(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
							x.6.clone(),
							x.7.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
		},
		babe: BabeConfig { epoch_config: BABE_GENESIS_EPOCH_CONFIG },
		sudo: SudoConfig { key: Some(root_key.clone()) },
		configuration: ConfigurationConfig {
			config: pezkuwi_runtime_teyrchains::configuration::HostConfiguration {
				scheduler_params: SchedulerParams {
					max_validators_per_core: Some(1),
					..default_teyrchains_host_configuration().scheduler_params
				},
				..default_teyrchains_host_configuration()
			},
		},
		registrar: RegistrarConfig { next_free_para_id: pezkuwi_primitives::LOWEST_PUBLIC_ID },
		staking_ah_client: StakingAhClientConfig {
			operating_mode: pezpallet_staking_async_ah_client::OperatingMode::Active,
			..Default::default()
		},
	})
}

// pezstaging_testnet
fn pezkuwichain_staging_testnet_config_genesis() -> serde_json::Value {
	use hex_literal::hex;
	use pezsp_core::crypto::UncheckedInto;

	// pez_subkey inspect "$SECRET"
	let endowed_accounts = Vec::from([
		// 5DwBmEFPXRESyEam5SsQF1zbWSCn2kCjyLW51hJHXe9vW4xs
		hex!["52bc71c1eca5353749542dfdf0af97bf764f9c2f44e860cd485f1cd86400f649"].into(),
	]);

	// ./scripts/prepare-test-net.sh 8
	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
		BeefyId,
	)> = Vec::from([
		(
			//5EHZkbp22djdbuMFH9qt1DVzSCvqi3zWpj6DAYfANa828oei
			hex!["62475fe5406a7cb6a64c51d0af9d3ab5c2151bcae982fb812f7a76b706914d6a"].into(),
			//5FeSEpi9UYYaWwXXb3tV88qtZkmSdB3mvgj3pXkxKyYLGhcd
			hex!["9e6e781a76810fe93187af44c79272c290c2b9e2b8b92ee11466cd79d8023f50"].into(),
			//5Fh6rDpMDhM363o1Z3Y9twtaCPfizGQWCi55BSykTQjGbP7H
			hex!["a076ef1280d768051f21d060623da3ab5b56944d681d303ed2d4bf658c5bed35"]
				.unchecked_into(),
			//5CPd3zoV9Aaah4xWucuDivMHJ2nEEmpdi864nPTiyRZp4t87
			hex!["0e6d7d1afbcc6547b92995a394ba0daed07a2420be08220a5a1336c6731f0bfa"]
				.unchecked_into(),
			//5CP6oGfwqbEfML8efqm1tCZsUgRsJztp9L8ZkEUxA16W8PPz
			hex!["0e07a51d3213842f8e9363ce8e444255990a225f87e80a3d651db7841e1a0205"]
				.unchecked_into(),
			//5HQdwiDh8Qtd5dSNWajNYpwDvoyNWWA16Y43aEkCNactFc2b
			hex!["ec60e71fe4a567ef9fef99d4bbf37ffae70564b41aa6f94ef0317c13e0a5477b"]
				.unchecked_into(),
			//5HbSgM72xVuscsopsdeG3sCSCYdAeM1Tay9p79N6ky6vwDGq
			hex!["f49eae66a0ac9f610316906ec8f1a0928e20d7059d76a5ca53cbcb5a9b50dd3c"]
				.unchecked_into(),
			//5DPSWdgw38Spu315r6LSvYCggeeieBAJtP5A1qzuzKhqmjVu
			hex!["034f68c5661a41930c82f26a662276bf89f33467e1c850f2fb8ef687fe43d62276"]
				.unchecked_into(),
		),
		(
			//5DvH8oEjQPYhzCoQVo7WDU91qmQfLZvxe9wJcrojmJKebCmG
			hex!["520b48452969f6ddf263b664de0adb0c729d0e0ad3b0e5f3cb636c541bc9022a"].into(),
			//5ENZvCRzyXJJYup8bM6yEzb2kQHEb1NDpY2ZEyVGBkCfRdj3
			hex!["6618289af7ae8621981ffab34591e7a6486e12745dfa3fd3b0f7e6a3994c7b5b"].into(),
			//5DLjSUfqZVNAADbwYLgRvHvdzXypiV1DAEaDMjcESKTcqMoM
			hex!["38757d0de00a0c739e7d7984ef4bc01161bd61e198b7c01b618425c16bb5bd5f"]
				.unchecked_into(),
			//5HnDVBN9mD6mXyx8oryhDbJtezwNSj1VRXgLoYCBA6uEkiao
			hex!["fcd5f87a6fd5707a25122a01b4dac0a8482259df7d42a9a096606df1320df08d"]
				.unchecked_into(),
			//5EPEWRecy2ApL5n18n3aHyU1956zXTRqaJpzDa9DoqiggNwF
			hex!["669a10892119453e9feb4e3f1ee8e028916cc3240022920ad643846fbdbee816"]
				.unchecked_into(),
			//5ES3fw5X4bndSgLNmtPfSbM2J1kLqApVB2CCLS4CBpM1UxUZ
			hex!["68bf52c482630a8d1511f2edd14f34127a7d7082219cccf7fd4c6ecdb535f80d"]
				.unchecked_into(),
			//5HeXbwb5PxtcRoopPZTp5CQun38atn2UudQ8p2AxR5BzoaXw
			hex!["f6f8fe475130d21165446a02fb1dbce3a7bf36412e5d98f4f0473aed9252f349"]
				.unchecked_into(),
			//5F7nTtN8MyJV4UsXpjg7tHSnfANXZ5KRPJmkASc1ZSH2Xoa5
			hex!["03a90c2bb6d3b7000020f6152fe2e5002fa970fd1f42aafb6c8edda8dacc2ea77e"]
				.unchecked_into(),
		),
		(
			//5FPMzsezo1PRxYbVpJMWK7HNbR2kUxidsAAxH4BosHa4wd6S
			hex!["92ef83665b39d7a565e11bf8d18d41d45a8011601c339e57a8ea88c8ff7bba6f"].into(),
			//5G6NQidFG7YiXsvV7hQTLGArir9tsYqD4JDxByhgxKvSKwRx
			hex!["b235f57244230589523271c27b8a490922ffd7dccc83b044feaf22273c1dc735"].into(),
			//5GpZhzAVg7SAtzLvaAC777pjquPEcNy1FbNUAG2nZvhmd6eY
			hex!["d2644c1ab2c63a3ad8d40ad70d4b260969e3abfe6d7e6665f50dc9f6365c9d2a"]
				.unchecked_into(),
			//5HAes2RQYPbYKbLBfKb88f4zoXv6pPA6Ke8CjN7dob3GpmSP
			hex!["e1b68fbd84333e31486c08e6153d9a1415b2e7e71b413702b7d64e9b631184a1"]
				.unchecked_into(),
			//5FtAGDZYJKXkhVhAxCQrXmaP7EE2mGbBMfmKDHjfYDgq2BiU
			hex!["a8e61ffacafaf546283dc92d14d7cc70ea0151a5dd81fdf73ff5a2951f2b6037"]
				.unchecked_into(),
			//5CtK7JHv3h6UQZ44y54skxdwSVBRtuxwPE1FYm7UZVhg8rJV
			hex!["244f3421b310c68646e99cdbf4963e02067601f57756b072a4b19431448c186e"]
				.unchecked_into(),
			//5D4r6YaB6F7A7nvMRHNFNF6zrR9g39bqDJFenrcaFmTCRwfa
			hex!["2c57f81fd311c1ab53813c6817fe67f8947f8d39258252663b3384ab4195494d"]
				.unchecked_into(),
			//5EPoHj8uV4fFKQHYThc6Z9fDkU7B6ih2ncVzQuDdNFb8UyhF
			hex!["039d065fe4f9234f0a4f13cc3ae585f2691e9c25afa469618abb6645111f607a53"]
				.unchecked_into(),
		),
		(
			//5DMNx7RoX6d7JQ38NEM7DWRcW2THu92LBYZEWvBRhJeqcWgR
			hex!["38f3c2f38f6d47f161e98c697bbe3ca0e47c033460afda0dda314ab4222a0404"].into(),
			//5GGdKNDr9P47dpVnmtq3m8Tvowwf1ot1abw6tPsTYYFoKm2v
			hex!["ba0898c1964196474c0be08d364cdf4e9e1d47088287f5235f70b0590dfe1704"].into(),
			//5EjkyPCzR2SjhDZq8f7ufsw6TfkvgNRepjCRQFc4TcdXdaB1
			hex!["764186bc30fd5a02477f19948dc723d6d57ab174debd4f80ed6038ec960bfe21"]
				.unchecked_into(),
			//5DJV3zCBTJBLGNDCcdWrYxWDacSz84goGTa4pFeKVvehEBte
			hex!["36be9069cdb4a8a07ecd51f257875150f0a8a1be44a10d9d98dabf10a030aef4"]
				.unchecked_into(),
			//5F9FsRjpecP9GonktmtFL3kjqNAMKjHVFjyjRdTPa4hbQRZA
			hex!["882d72965e642677583b333b2d173ac94b5fd6c405c76184bb14293be748a13b"]
				.unchecked_into(),
			//5F1FZWZSj3JyTLs8sRBxU6QWyGLSL9BMRtmSKDmVEoiKFxSP
			hex!["821271c99c958b9220f1771d9f5e29af969edfa865631dba31e1ab7bc0582b75"]
				.unchecked_into(),
			//5CtgRR74VypK4h154s369abs78hDUxZSJqcbWsfXvsjcHJNA
			hex!["2496f28d887d84705c6dae98aee8bf90fc5ad10bb5545eca1de6b68425b70f7c"]
				.unchecked_into(),
			//5CPx6dsr11SCJHKFkcAQ9jpparS7FwXQBrrMznRo4Hqv1PXz
			hex!["0307d29bbf6a5c4061c2157b44fda33b7bb4ec52a5a0305668c74688cedf288d58"]
				.unchecked_into(),
		),
		(
			//5C8AL1Zb4bVazgT3EgDxFgcow1L4SJjVu44XcLC9CrYqFN4N
			hex!["02a2d8cfcf75dda85fafc04ace3bcb73160034ed1964c43098fb1fe831de1b16"].into(),
			//5FLYy3YKsAnooqE4hCudttAsoGKbVG3hYYBtVzwMjJQrevPa
			hex!["90cab33f0bb501727faa8319f0845faef7d31008f178b65054b6629fe531b772"].into(),
			//5Et3tfbVf1ByFThNAuUq5pBssdaPPskip5yob5GNyUFojXC7
			hex!["7c94715e5dd8ab54221b1b6b2bfa5666f593f28a92a18e28052531de1bd80813"]
				.unchecked_into(),
			//5EX1JBghGbQqWohTPU6msR9qZ2nYPhK9r3RTQ2oD1K8TCxaG
			hex!["6c878e33b83c20324238d22240f735457b6fba544b383e70bb62a27b57380c81"]
				.unchecked_into(),
			//5EUNaBpX9mJgcmLQHyG5Pkms6tbDiKuLbeTEJS924Js9cA1N
			hex!["6a8570b9c6408e54bacf123cc2bb1b0f087f9c149147d0005badba63a5a4ac01"]
				.unchecked_into(),
			//5CaZuueRVpMATZG4hkcrgDoF4WGixuz7zu83jeBdY3bgWGaG
			hex!["16c69ea8d595e80b6736f44be1eaeeef2ac9c04a803cc4fd944364cb0d617a33"]
				.unchecked_into(),
			//5DABsdQCDUGuhzVGWe5xXzYQ9rtrVxRygW7RXf9Tsjsw1aGJ
			hex!["306ac5c772fe858942f92b6e28bd82fb7dd8cdd25f9a4626c1b0eee075fcb531"]
				.unchecked_into(),
			//5H91T5mHhoCw9JJG4NjghDdQyhC6L7XcSuBWKD3q3TAhEVvQ
			hex!["02fb0330356e63a35dd930bc74525edf28b3bf5eb44aab9e9e4962c8309aaba6a6"]
				.unchecked_into(),
		),
		(
			//5C8XbDXdMNKJrZSrQURwVCxdNdk8AzG6xgLggbzuA399bBBF
			hex!["02ea6bfa8b23b92fe4b5db1063a1f9475e3acd0ab61e6b4f454ed6ba00b5f864"].into(),
			//5GsyzFP8qtF8tXPSsjhjxAeU1v7D1PZofuQKN9TdCc7Dp1JM
			hex!["d4ffc4c05b47d1115ad200f7f86e307b20b46c50e1b72a912ec4f6f7db46b616"].into(),
			//5GHWB8ZDzegLcMW7Gdd1BS6WHVwDdStfkkE4G7KjPjZNJBtD
			hex!["bab3cccdcc34401e9b3971b96a662686cf755aa869a5c4b762199ce531b12c5b"]
				.unchecked_into(),
			//5GzDPGbUM9uH52ZEwydasTj8edokGUJ7vEpoFWp9FE1YNuFB
			hex!["d9c056c98ca0e6b4eb7f5c58c007c1db7be0fe1f3776108f797dd4990d1ccc33"]
				.unchecked_into(),
			//5CmLCFeSurRXXtwMmLcVo7sdJ9EqDguvJbuCYDcHkr3cpqyE
			hex!["1efc23c0b51ad609ab670ecf45807e31acbd8e7e5cb7c07cf49ee42992d2867c"]
				.unchecked_into(),
			//5DnsSy8a8pfE2aFjKBDtKw7WM1V4nfE5sLzP15MNTka53GqS
			hex!["4c64d3f06d28adeb36a892fdaccecace150bec891f04694448a60b74fa469c22"]
				.unchecked_into(),
			//5CZdFnyzZvKetZTeUwj5APAYskVJe4QFiTezo5dQNsrnehGd
			hex!["160ea09c5717270e958a3da42673fa011613a9539b2e4ebcad8626bc117ca04a"]
				.unchecked_into(),
			//5HgoR9JJkdBusxKrrs3zgd3ToppgNoGj1rDyAJp4e7eZiYyT
			hex!["020019a8bb188f8145d02fa855e9c36e9914457d37c500e03634b5223aa5702474"]
				.unchecked_into(),
		),
		(
			//5HinEonzr8MywkqedcpsmwpxKje2jqr9miEwuzyFXEBCvVXM
			hex!["fa373e25a1c4fe19c7148acde13bc3db1811cf656dc086820f3dda736b9c4a00"].into(),
			//5EHJbj6Td6ks5HDnyfN4ttTSi57osxcQsQexm7XpazdeqtV7
			hex!["62145d721967bd88622d08625f0f5681463c0f1b8bcd97eb3c2c53f7660fd513"].into(),
			//5EeCsC58XgJ1DFaoYA1WktEpP27jvwGpKdxPMFjicpLeYu96
			hex!["720537e2c1c554654d73b3889c3ef4c3c2f95a65dd3f7c185ebe4afebed78372"]
				.unchecked_into(),
			//5DnEySxbnppWEyN8cCLqvGjAorGdLRg2VmkY96dbJ1LHFK8N
			hex!["4bea0b37e0cce9bddd80835fa2bfd5606f5dcfb8388bbb10b10c483f0856cf14"]
				.unchecked_into(),
			//5CAC278tFCHAeHYqE51FTWYxHmeLcENSS1RG77EFRTvPZMJT
			hex!["042f07fc5268f13c026bbe199d63e6ac77a0c2a780f71cda05cee5a6f1b3f11f"]
				.unchecked_into(),
			//5HjRTLWcQjZzN3JDvaj1UzjNSayg5ZD9ZGWMstaL7Ab2jjAa
			hex!["fab485e87ed1537d089df521edf983a777c57065a702d7ed2b6a2926f31da74f"]
				.unchecked_into(),
			//5ELv74v7QcsS6FdzvG4vL2NnYDGWmRnJUSMKYwdyJD7Xcdi7
			hex!["64d59feddb3d00316a55906953fb3db8985797472bd2e6c7ea1ab730cc339d7f"]
				.unchecked_into(),
			//5FaUcPt4fPz93vBhcrCJqmDkjYZ7jCbzAF56QJoCmvPaKrmx
			hex!["033f1a6d47fe86f88934e4b83b9fae903b92b5dcf4fec97d5e3e8bf4f39df03685"]
				.unchecked_into(),
		),
		(
			//5Ey3NQ3dfabaDc16NUv7wRLsFCMDFJSqZFzKVycAsWuUC6Di
			hex!["8062e9c21f1d92926103119f7e8153cebdb1e5ab3e52d6f395be80bb193eab47"].into(),
			//5HiWsuSBqt8nS9pnggexXuHageUifVPKPHDE2arTKqhTp1dV
			hex!["fa0388fa88f3f0cb43d583e2571fbc0edad57dff3a6fd89775451dd2c2b8ea00"].into(),
			//5H168nKX2Yrfo3bxj7rkcg25326Uv3CCCnKUGK6uHdKMdPt8
			hex!["da6b2df18f0f9001a6dcf1d301b92534fe9b1f3ccfa10c49449fee93adaa8349"]
				.unchecked_into(),
			//5DrA2fZdzmNqT5j6DXNwVxPBjDV9jhkAqvjt6Us3bQHKy3cF
			hex!["4ee66173993dd0db5d628c4c9cb61a27b76611ad3c3925947f0d0011ee2c5dcc"]
				.unchecked_into(),
			//5Gx6YeNhynqn8qkda9QKpc9S7oDr4sBrfAu516d3sPpEt26F
			hex!["d822d4088b20dca29a580a577a97d6f024bb24c9550bebdfd7d2d18e946a1c7d"]
				.unchecked_into(),
			//5DhDcHqwxoes5s89AyudGMjtZXx1nEgrk5P45X88oSTR3iyx
			hex!["481538f8c2c011a76d7d57db11c2789a5e83b0f9680dc6d26211d2f9c021ae4c"]
				.unchecked_into(),
			//5DqAvikdpfRdk5rR35ZobZhqaC5bJXZcEuvzGtexAZP1hU3T
			hex!["4e262811acdfe94528bfc3c65036080426a0e1301b9ada8d687a70ffcae99c26"]
				.unchecked_into(),
			//5E41Znrr2YtZu8bZp3nvRuLVHg3jFksfQ3tXuviLku4wsao7
			hex!["025e84e95ed043e387ddb8668176b42f8e2773ddd84f7f58a6d9bf436a4b527986"]
				.unchecked_into(),
		),
	]);

	const ENDOWMENT: u128 = 1_000_000 * HEZ;
	const STASH: u128 = 100 * HEZ;

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.map(|k: &AccountId| (k.clone(), ENDOWMENT))
				.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
				.collect::<Vec<_>>(),
		},
		session: SessionConfig {
			keys: initial_authorities
				.into_iter()
				.map(|x| (
					x.0.clone(),
					x.0,
					pezkuwichain_session_keys(x.2, x.3, x.4, x.5, x.6, x.7)
				))
				.collect::<Vec<_>>(),
		},
		babe: BabeConfig { epoch_config: BABE_GENESIS_EPOCH_CONFIG },
		sudo: SudoConfig { key: Some(endowed_accounts[0].clone()) },
		configuration: ConfigurationConfig { config: default_teyrchains_host_configuration() },
		registrar: RegistrarConfig { next_free_para_id: pezkuwi_primitives::LOWEST_PUBLIC_ID },
		staking_ah_client: StakingAhClientConfig {
			operating_mode: pezpallet_staking_async_ah_client::OperatingMode::Active,
			..Default::default()
		},
	})
}

//development
fn pezkuwichain_development_config_genesis() -> serde_json::Value {
	pezkuwichain_testnet_genesis(
		Vec::from([get_authority_keys_from_seed("Alice")]),
		Sr25519Keyring::Alice.to_account_id(),
		None,
	)
}

//local_testnet
fn pezkuwichain_local_testnet_genesis() -> serde_json::Value {
	pezkuwichain_testnet_genesis(
		Vec::from([get_authority_keys_from_seed("Alice"), get_authority_keys_from_seed("Bob")]),
		Sr25519Keyring::Alice.to_account_id(),
		None,
	)
}

/// `Versi` is a temporary testnet that uses the same runtime as pezkuwichain.
// versi_local_testnet
fn versi_local_testnet_genesis() -> serde_json::Value {
	pezkuwichain_testnet_genesis(
		Vec::from([
			get_authority_keys_from_seed("Alice"),
			get_authority_keys_from_seed("Bob"),
			get_authority_keys_from_seed("Charlie"),
			get_authority_keys_from_seed("Dave"),
		]),
		Sr25519Keyring::Alice.to_account_id(),
		None,
	)
}

/// Encapsulates names of predefined presets.
mod preset_names {
	pub const PRESET_GENESIS: &str = "genesis";
}

// ============================================================================
// PEZKUWICHAIN GENESIS MESSAGE
// ============================================================================
//
// Satoshi Qazi Muhammed:
// {
//   "block_height": 0,
//   "timestamp": "1947-03-31T00:00:00Z",
//   "message": "Heger hûn min darve bikin, an jî parçe parçe bikin, Kurdistan yek e û nabe çar!",
//   "philosophy": "Collective Sovereignty through Proof of Unity",
//   "encoded_vow": "0xdfdfbaff585a988e269606bf7595b6899b521192a628cef55b1ef54044571efd"
// }
//
// In memory of Qazi Muhammad (1893-1947), President of the Republic of Mahabad,
// executed on March 31, 1947. His final words before the gallows:
// "Even if you hang me or tear me to pieces, Kurdistan is one and will not become four!"
//
// This blockchain is built on the principle that no force can divide a people
// who choose unity through technology, trust, and collective sovereignty.
// ============================================================================

/// Genesis configuration for mainnet with HEZ distribution
/// Accounts from Founder_treasury_presale_wallets.json
fn pezkuwichain_genesis_config() -> serde_json::Value {
	use hex_literal::hex;
	use pezsp_core::crypto::UncheckedInto;

	// ==========================================================================
	// MAINNET ACCOUNTS - Generated 2026-09-15 for the genesis reset
	//
	// Every account below is new. The set the chain launched with in January belongs to the
	// chain being replaced, and carrying it across would mean the reset changed the ledger but
	// not who holds it. Generated by `res/genesis/keygen` from one master phrase, recorded in
	// `res/genesis/mainnet/mainnet-wallets.json` -- which never enters this repository.
	//
	// No address here appears in Zagros's genesis; `check-chain-key-overlap.py` holds that.
	// ==========================================================================

	// Founder account - receives 10% (20M HEZ), less root's fee budget carved out below.
	// SS58: 5DPA5ctyUhFZcLoqNj11w1xEn3QqtDSmUjk4L6YxQNBWiDxS
	let founder_account: AccountId =
		hex!("3a4eed1ba224f6d76dec6f24da10b850248dc8db5e8de7effcaf25bea977fe7f").into();

	// The chain's root key -- its own account, not the founder's, and not one key at all.
	//
	// It used to be `founder_account`, which put root authority and personal property on one
	// key: losing or leaking it would not have cost the founder money, it would have handed
	// over the chain. Then it was one separate key, which still put the chain one theft or one
	// loss away. Now it is the 2-of-3 multisig of `genesis::SUDO_SIGNERS` (decided 2026-10-20),
	// derived rather than held; `the_root_key_is_the_signers_multisig` derives it again. Root is
	// also the one authority meant to be retired, and retiring an account is cleaner than
	// retiring a person's wallet.
	let sudo_account: AccountId = pezkuwichain_runtime_constants::genesis::SUDO_MULTISIG.into();
	let sudo_signers: Vec<AccountId> = pezkuwichain_runtime_constants::genesis::SUDO_SIGNERS
		.iter()
		.map(|s| AccountId::from(*s))
		.collect();

	// The founding office: holder of `Tiki::Serok` on the People chain, funded on all three.
	//
	// It is declared on the relay because the relay is where the founder's line is computed,
	// and the carve-out has to be subtracted exactly once. The People chain names the same
	// account in `TikiConfig::founding_government`; the two are the same key by the address
	// below, and `//pezkuwichain//office//serok` in res/genesis/mainnet/mainnet-wallets.json is where it comes from.
	// SS58: 5CZyVmWocD8mQ3YDHEyjPGqZqeooS6u91CujcTEEszSCZ7ge
	let serok_account: AccountId =
		hex!("1652c3b477df81cc819fd3813faee7667dc62ed0a29a2a3522f4c6186e955802").into();

	// Presale account - receives 50% (100M HEZ)
	// There is no airdrop account here any more, and that is the fix rather than an omission.
	// It used to hold 40M HEZ that nothing in the tree ever read: `Claims` is wired but its
	// genesis list is empty, and Claims pays Ethereum-signed claims out of newly minted funds
	// anyway, so it never touched this balance. The only way to distribute it was for whoever
	// held the key to send transfers by hand.
	//
	// The 40M is minted into the Asset Hub's `AirdropPot` instead -- a keyless treasury
	// instance, spendable only by the People chain, two signatures under a million HEZ and
	// three above it. No key holds it, so none can be lost or leaked, and no post-launch
	// transfer has to be remembered.

	// ==========================================================================
	// INITIAL VALIDATORS - twenty-seven
	//
	// Stash and controller come from the master phrase and never leave cold storage. The six
	// session keys do not: a validator has to hold them in its keystore, and a keystore holds
	// whatever it was given, so a key derived from the master by path would put the master --
	// root, the founder allocation, the custody board -- on every validator's disk. Each session
	// key here is its own phrase with nothing else derived from it; a server that is taken
	// gives up its own signing keys and no more. An outside operator's slot carries the keys
	// they generated on their own node. Written by `res/genesis/mainnet/emit_relay_authorities.py`,
	// which refuses a session key the master derives.
	// ==========================================================================
	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
		BeefyId,
	)> = Vec::from([
		(
			// Validator 01 (5G4e6KKUViiwUp6qPgvdqNxwtaGM7LzYB2KVr9bXMjJAfMDF)
			hex!("b0e442bf467ef368a9247474bf1a97a1c08ad9213673e3180fdab407a758ac3c").into(),
			hex!("10aec0daa42782e62b375845a5a496d6943a3b28c0d8803c12df0be24a641d22").into(),
			hex!("4e261f75a97625fe1d4258ae7671c1aeb77d29022282c235f8b25ff721f6de47")
				.unchecked_into(),
			hex!("12359b7c659f93473a8cdc9f236a4c9dab90dce478ef5a21d4a8be9587552565")
				.unchecked_into(),
			hex!("94f3bd94986d886db66743988c679be0f6d134bc10ec99809e577f788a490543")
				.unchecked_into(),
			hex!("3aa9fdea783b84df2f3a5b1024720de6b781c2df5e2bf65aa6794271c9a22874")
				.unchecked_into(),
			hex!("985b961a9fec7c232e90a609f908addb6129b3883da5b6b281b92bfe1cbbbf16")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("02507df6ce8f663ea68a3ea29da7ac1fda8dd467461f07d1afefa67b7969f1a59f")
				.unchecked_into(),
		),
		(
			// Validator 02 (5E5239nsL71Qb2MUmPwPtUM8pLb1Md4KkNBajmJ6wAHQwZP1)
			hex!("58b5ab34bfef86ccd2fda24b4b4034d8e77f03e24b19c810b1d3025456036806").into(),
			hex!("9cb78ac63399b9257d8ccb5a4fd0160e251f971b0ca5d4a8fb98d196e4b7721a").into(),
			hex!("8a1d0c705d1dd6e08bd3f75e4da1567a03b53baf26286b3db033f11cce4de327")
				.unchecked_into(),
			hex!("be0362dbef8ed5a21686a324ddf78575e09fe619944fa26fa79664888631034e")
				.unchecked_into(),
			hex!("84141c4692dda61075de310540ec4bde2d59508b8c58d6f1218442540ff88704")
				.unchecked_into(),
			hex!("e4226d2d23e1ace8ba9b4204a2fa3e9d416bd40349991f3c51aa92f3e54c9466")
				.unchecked_into(),
			hex!("8216ddb36f2229edc194f29fd87dee7777f24b09d749dce381f54fa9d2f2b30a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("026e1a3e151b57fe4cffb9967c9c816ecc227ae13633400937b02faea2cc6ea998")
				.unchecked_into(),
		),
		(
			// Validator 03 (5Ck4sveGyXL8Y3Fd1cqcEnUmXwRxWRE64AyUVaKdVumnbbSm)
			hex!("1e055605aae178fb4db058e1ffa438812cecd16c45e194ed831c27508bea7519").into(),
			hex!("fe6af42ab4a7bb2e10433e80bd9fdc111d6e72ce8fe08669367814c42c760650").into(),
			hex!("4ef4feac5db9ed8568e5632c99d01770d0d5180ff9f54a68b9263607b4a84c50")
				.unchecked_into(),
			hex!("650a463dbfc49a59a711a5853cc3b76b994fb550521c9c1fce88bbfcb505a0a2")
				.unchecked_into(),
			hex!("fcba93f1ec0e4085191ffc85eb04aefd94c2f85636698544e4fb1d39318fc63f")
				.unchecked_into(),
			hex!("66dddbf4bfc95ba2c03a211ef0981be705ad4b9bc85e8801cb4626dce5d3b76c")
				.unchecked_into(),
			hex!("7aed99090267307ce52acb29409dd896a6007fccad645f097c1608630e2af43a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0323cab8369f319a036b007fc429a090c95137a28af6642225824d1039c6d925af")
				.unchecked_into(),
		),
		(
			// Validator 04 (5ECHUqJ8KczMu3ChhX2f9JD4w9Z9ZfgzKbu5KEJ7TobL1DBW)
			hex!("5e405f2306593d90387bf84c40132072b1ace7cd09b779ffc621b45f578acb0c").into(),
			hex!("7e1b682e851d28ca52a6bf7c4a2427bcc972ea3d2e050d9a76a6bbeb1962cc0e").into(),
			hex!("a614ca5bebc7f9ca588586bff699887f76c9c0f35dc5205997298591b6eb0468")
				.unchecked_into(),
			hex!("fbcb5eb389fa97171aeb6c3ebebf351f865c4955c107636ac217adc31497f5ea")
				.unchecked_into(),
			hex!("8885e1980a4c17d33fcfd43dbb428c996205a6c9ab803c426df174faab280a69")
				.unchecked_into(),
			hex!("ac0ae65120c74c443c3d9f5044e63f3d4355db81bb0564f54e12cbdf4a8a1c66")
				.unchecked_into(),
			hex!("4ad3522d3f7969612a3154bff140974e7d419eb78c9f913bf972b6dc98fbc12e")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03185959aa291c674525e11542586a0dcff13ed1f9707ed4680c1ea53cc1065758")
				.unchecked_into(),
		),
		(
			// Validator 05 (5DUwAkAaBme36xwFePyWyyVPAJs1fLDurtXuNopPPCQJnCnw)
			hex!("3eb6ef27bff8784a6354dd004b2258d95d2b2a655719abac1a297a7f014f324d").into(),
			hex!("cad4fdbd3b4a8cce42af32035b7aea18fcb5ae656d2a918c8df1cd1787dc987a").into(),
			hex!("da6dcc4d133ba503c1da253d653ddfcdb1b10c93e600b4f93234156723c8cf0c")
				.unchecked_into(),
			hex!("9aa9dcd771aa13430fefd48f898d808dd261aea15139415f6cf486d903963b5e")
				.unchecked_into(),
			hex!("b6100dc189c99966ed5dda6769eeb0f8e414e6a23778e5e19351699a6a0f961d")
				.unchecked_into(),
			hex!("1cf734f4cf2c5e1604d6e61e2db46d900b5d985dd444f7ee70be99f39354a803")
				.unchecked_into(),
			hex!("32ea373892b4366f4b04796370b93d755956ebcc7dce0148d3257804cf88d031")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0240b19d30d29f80109a6bc46cb5cbe23f47c92cc64f066fb72d656bb601fd2199")
				.unchecked_into(),
		),
		(
			// Validator 06 (5CoMjECS3gHczmRd24NZGyATuP4iCmU3S4p97Aehinudegcp)
			hex!("2087cb677d08ec47c6a3bc36b94e92226d47f32ebf18dbe39808014f3bb1f674").into(),
			hex!("76a9437cb077eabff5835feba2ef47f27c5168f317bd30a89c08be5ca8ab2872").into(),
			hex!("a28485b49599ecf33db835e344e05140f9e7a6cc780aadf8b5c23a3c68009150")
				.unchecked_into(),
			hex!("e2125290f01122073653d50a74a484d2e1cf88be0481a97aadb5d229f71be7dc")
				.unchecked_into(),
			hex!("42465319aa4054ba7f084d3213f9562324f277054fecd23d1165ac96f4f1b83b")
				.unchecked_into(),
			hex!("e0dc0dfa22f64dba6275425ed952af0995a6e28b3468b698d588c55781179f2f")
				.unchecked_into(),
			hex!("cedafc759a7142fe410c94e5aadca28691ac76281493a7563f747da8e9343547")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("035a83ad841ef9a5630dcc2958544fba00ffef3eb3daff3de4eeb23287c04228de")
				.unchecked_into(),
		),
		(
			// Validator 07 (5ENhEwumArbBpLNqc258MQe3pyZhgKjNMWTL8NaeCoc3Ayyf)
			hex!("6630cf9996f36ebd2586a0600324f0268f8bc54dc50e56d31ea6cfa7f8234219").into(),
			hex!("4c95aa79099097ad8d903873d47d1da2db6e7b44e0916eeef8d3817a5713ca4a").into(),
			hex!("7681025a71845c4594a9f2bcc63b16a95304f1a61c9dcb172620be903d165a09")
				.unchecked_into(),
			hex!("199c715e09b09e6df6b3aec0f73eb8e7d087cec5ac9895c1ec90e4d15d2a4801")
				.unchecked_into(),
			hex!("c6bb9cfafff598632cd0dc8113184bcce547f80bb632aa1700b3e31380c24642")
				.unchecked_into(),
			hex!("4418b055f753aa09fe8b911c8fd73a3c73b473a96c73d88ffafd6b558535ee3f")
				.unchecked_into(),
			hex!("f458e63119751ade86ec0d10d5eec14344677f80c856b50ece4d1c9891d6161e")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("031d4c68f53c3986d6f1a35fb478ae5b4eb9a2b4be80a56f4e64f9c6a5fc588387")
				.unchecked_into(),
		),
		(
			// Validator 08 (5Ey4cgchows4382WhyvMaoLeNENFoUpg3Zz5iHidZti7ypYy)
			hex!("80671bca44671b267fe03e28ab3b67fac9ff558ac46c82f3462fdb7bfb66f72c").into(),
			hex!("5af61022f0c41f9dcf5fc8345f376a45c538e357b1d03a6885235e9a4c0af72a").into(),
			hex!("dcca52fd09c6c0065717b2502f5e3e06cf6b881e2ce884673d5c1037c7df715e")
				.unchecked_into(),
			hex!("21a788e1c8ed708ee4f3b5bea9754c84cf9f4f4fb9b75bf77f59e631fff66141")
				.unchecked_into(),
			hex!("584ab0bddbf51cdfd06ab8bfa8f7a9657ac9eb66c941ce989272627b0a802d57")
				.unchecked_into(),
			hex!("2a985f1ee9033a3071cbeaf90cce6456b93412f6c97955209eb5f7d25f325c16")
				.unchecked_into(),
			hex!("9ca0ef0b6429e23f19084ca54b44408c46b4d0d860ffda08153a643939b0826f")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0298c25b07176904f580972faf2e671ad9ba13171412f1a336c30e52316836360b")
				.unchecked_into(),
		),
		(
			// Validator 09 (5H8YwbdehipCdPjLHkrCh5wHgaza5iVE1bhGiavGG7reWGLR)
			hex!("e01c2365959e8cda2e23d25bf1488cf9680091ebe4da865a873b9b2996a56a5e").into(),
			hex!("b01defd96ba59f5e344093cba6716ef4a6a3cf8c70668917b878e9cf1b21ff59").into(),
			hex!("7cef1de748d558c597dba4400aa1bd707b9591e217f189abde3bf4b54bcbc93d")
				.unchecked_into(),
			hex!("163482a228d4ceacaf1395b337a694a47591ee71b92c586b306829ca3c3c6695")
				.unchecked_into(),
			hex!("1a16c0b1a1c2ea50107967725739403439425d1bfdd3a8f7f5012da32353296b")
				.unchecked_into(),
			hex!("6802ad6ef283f8f567d04aed179725e03d2ff84d7b1701bfd57c6d6e230d7e2a")
				.unchecked_into(),
			hex!("62cde6881bf454c31e442d78acfa9705296bab02d129667489232f9617180f50")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0321d78fdf8abc1b68afb9a7c39a25eb0ce5cd1e9a81fab0ed031b933380e56ebf")
				.unchecked_into(),
		),
		(
			// Validator 10 (5EhMtwRJCJ8fQ8ZyNxVL5FGExjaobwgnGbrsq3dJoxJdoJEX)
			hex!("746d5a4641e7de16283ae25192d2b87bf397cedeeca132b70e244dc47638fb3f").into(),
			hex!("0e8477cfc2ff4e3fd51ed972f1e80d51ef30277bcfa193fa43c5547346201a68").into(),
			hex!("2ce69c24799a6c7e88c7ae491c38e7135e35370f81e0950fe3782df42ddc2e3f")
				.unchecked_into(),
			hex!("3581b75688b8966b9eeb034fb4197bc3dbd8030bae9ee840d964b454b7d836de")
				.unchecked_into(),
			hex!("34aa817d2d132d7cc110472767fabe21c74b65044202e5835861f6f03a06d73c")
				.unchecked_into(),
			hex!("d696d623a1c1a439917ef7e3cc4390b9fe6d6908bf3ffdecb8ee10bb5f028845")
				.unchecked_into(),
			hex!("1cccc5da2e3dca5b72ce6977f684d0015587c7830225babcf76d31b391e11e31")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("02b28cbfcf78666a932694b471cb54acc0f5e5837fd629ab9e6e813265ea2739b3")
				.unchecked_into(),
		),
		(
			// Validator 11 (5Fmm54cqKb7PywNjkZGro97eNHAYFrJt4vU2G3D4T8ubwvbW)
			hex!("a404948873be26098cc9d1acb1f08e34bc4f5e7a40ca0aec87d20b18a5d85158").into(),
			hex!("b2f80b184106f373adf5619fa1f41ea291a9ba44ce9bae26aab525cfd9f57354").into(),
			hex!("ee887402d3959cd138693f55219a9f0b3173d1dab86df73755ad7f3cc61a1737")
				.unchecked_into(),
			hex!("a0bda15674e2ef3edd6360f8b972d45fecd4e6b5663c45c23e042c3894d8a4b1")
				.unchecked_into(),
			hex!("88f48e07bc06bbfbaf975f9057fb9d6875726a03401273900eb6cb23e3ca4934")
				.unchecked_into(),
			hex!("ce6883eeb267e89f1a7dbf503a006652ee5afadea986b1a67aeb64b8facc194b")
				.unchecked_into(),
			hex!("fa6640956ae4ca70d69d1cedf4dd79036071f2ff7e313ab7fc9c5243f6d6cc64")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("031ed9964be2e8bd34807b8c2c0f654f469f57434110194c9a3eec40b775c1a62f")
				.unchecked_into(),
		),
		(
			// Validator 12 (5CiAWLRtzhHnv6Ph18fx2Y7zDVG1iBAqrqVF62mTsTs1mhiS)
			hex!("1c91ca39e8505616a9aeb5d0d28f1e6568be595aae651787e174f0649007d064").into(),
			hex!("fc98829dfb66c93859d9017efb5a4632238f83cb94222d368509cc07d8b82436").into(),
			hex!("f637726bc6258a5388dc1905314badb666a2ebb822f975b381df4eebc6534346")
				.unchecked_into(),
			hex!("866b5f767b59ab72c25a49b4b36183a181668b627ca040ef2e0ecfb177faa666")
				.unchecked_into(),
			hex!("18e734fe2e075e38988c7bb32df4eb19bde89759d3ebb4b4840bdda75898774a")
				.unchecked_into(),
			hex!("34c349a78ec01764b96c18e64bdcc93a5ce114ba6e61f21c67c708cd70c08806")
				.unchecked_into(),
			hex!("8604841c393b20a4ced0ec18dfbe234151837743485860e4d885145321f9977f")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("02a41a4c1939bc95a49883dadabbbf4ab956a072c4cf8614ce274c70d00e795551")
				.unchecked_into(),
		),
		(
			// Validator 13 (5C7tXxNJa9t1oKYVEJCPbpJRnsNjCsr3qUE5WSqRPSXNx1uY)
			hex!("026daded158eb1bae7c1221886940c95983408b14465a15aa4f33786fee43d35").into(),
			hex!("f4e5d2b699aaef7439a477ef3227c4f05a28ab3f39ddce28f98c55c6f5c5c66b").into(),
			hex!("88735555e72b1f1e5c6d8d7aef202370b7a20cbb61f20c89b845355c68a1263c")
				.unchecked_into(),
			hex!("17d2544b9222dd7a654f55d18d95b00a679a4ff2e16b7f491cf15170aed0bf32")
				.unchecked_into(),
			hex!("62df954e2cd72716f94d992da62160df950d32dce8dfdbfda57a79e4712e4d57")
				.unchecked_into(),
			hex!("644ead4f3bb28aaa313b27df57d04743fc2ed15041820b7516d2901701902152")
				.unchecked_into(),
			hex!("6e6fbcf7848ef7cb0fc86b415e2bf8da5d5f4e1a1d66f1d19aaefe52fe04655b")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("036c0a1d1325be895ca884229b078fb970ce9114f2fe02e0f91a2c8596910346bb")
				.unchecked_into(),
		),
		(
			// Validator 14 (5G9XFUFuZdUWuRmbfySNFYUi8tGWQCxm6J4q48TKy3bLJjbA)
			hex!("b49d7477dc29d3f424201014aa0b84df0f68602ea9a1347685f690208db9fc5f").into(),
			hex!("aa80f02af6101e1d3b50b8d93469044ef4da022edc7dd8947222a58ac0d4b06f").into(),
			hex!("78d806da22216618a494afd3b371e15b6b2b91792ad5016d42642b2f97a03613")
				.unchecked_into(),
			hex!("c91aaad20839c013114831c9cd1c1ac341e6e465c2f062613d752c799d19d91e")
				.unchecked_into(),
			hex!("0a42684a0a26458eef841d0e4d143a6a33157f14eddf7fe6a411cf8760268e6c")
				.unchecked_into(),
			hex!("3293b17ddf91bebf43dae3d4ce6341bc4166914130893056bc830f703239fa65")
				.unchecked_into(),
			hex!("e6d2a318b9e5edd0f313f9d73ab0680a45d6c3a6072685324c20de4bc47e860b")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("022636df8657b81717a2636f5f37ab8758dbcc4fe346091cdca9bfbe988b3682b9")
				.unchecked_into(),
		),
		(
			// Validator 15 (5G9kpHYkZqnpoeyp3SWKsjzGnbxg5oAGUJTLYzoXgVrPZJ2K)
			hex!("b4cb1f1720f48a3508bbab350215b78601fc03280ca6b1f11a3ad2f821a5b54f").into(),
			hex!("80004defd22a75d59c88847829c6a4b60dc1538046f8df8a4177976c6c019d24").into(),
			hex!("a0ac3ecfc106226513d6ddaa2ef04bf11ac63d669ddb5844593e8c6e68106019")
				.unchecked_into(),
			hex!("d39ccc4e718748cc413975c3bad847b5e9604134d6d1688378fa30eee6622580")
				.unchecked_into(),
			hex!("ea2e90306e2c6876f32ad0408908734a2bb5df7683fefc774fe43458d480cc5e")
				.unchecked_into(),
			hex!("94fe301fc9e8a2133bcd787a342c0d832dcfdf3c3c6671529b19304b1af6a02a")
				.unchecked_into(),
			hex!("4691dbf79210f579c04b61c5a45aa00d7cdda0e09d72ef7581011b149d6ffd50")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0332263995e9c4ad478575f1181ef86ac074c1af37a7e0a8e6a63dfbfdc0b4db25")
				.unchecked_into(),
		),
		(
			// Validator 16 (5GeP9GjAJwAArgzBnJxqAD2RfK33DnxHDsao7UKoBpsV7iRQ)
			hex!("caa04810a37d6b26ef7d6b9aef71ac0aaa81df9cf86653301f62ddf5bd04aa7c").into(),
			hex!("f219dd1fa4ee65d9c0050d7f0e3a93365d6fd6203cf8e41c6e4a4b28a7490d68").into(),
			hex!("62dff55a3d80ed29c357841d5852f86c51858785fbc198eee6e41d229e440206")
				.unchecked_into(),
			hex!("63154e37ac21f290412fbe166d566d2d47b5b5d6a44cb1cc1acc2ac57de7fe5e")
				.unchecked_into(),
			hex!("9c80c124af39f65be7399756ec9cc7cc31205edff421ef4b00901e9dbce29b79")
				.unchecked_into(),
			hex!("f47206bcd22f25807717e8312bcaaeb2562ebf827653af49924a1a6997d8e32b")
				.unchecked_into(),
			hex!("684497b357447d3262eb1fe024b8edb4546b34967dfa518190193b48650ee124")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0253ee761d00df9b21c0849dd3fcf16a074bf204d95f7a6c9b1deacbb34fd22629")
				.unchecked_into(),
		),
		(
			// Validator 17 (5G8zDxQRUwAgF16wRbJuZF9QuvYLbLE6hNHemt3FaLvU2J7o)
			hex!("b435030148bcc75a24fd8176fc42b4357037e3868f0d7a2b04658426e83d8016").into(),
			hex!("3687427d498660cbddba994f0e77175ff1a2e20c575b90f7cfa9bf8f05e8e256").into(),
			hex!("ba35ab722d2959885b6b2ba0d22aca54e689d05a60a36c63d55e73fc0e910d1d")
				.unchecked_into(),
			hex!("6c8dafc0397185cce02f5a27c9e58fe49e94fd5ad206f032e0182c6c06abaae5")
				.unchecked_into(),
			hex!("66856fc69cb308ef67b764a56b5c7cb13be28c0ad018e3151f610eb615e6b154")
				.unchecked_into(),
			hex!("564d7fba8397ee6728f8df9da726fe0fdded6378aa5713b5e1b56a7883595e57")
				.unchecked_into(),
			hex!("b8274d9bb0c9bfd755b54baa49f0322ee0dd9d4be3831edcbc0017f23e27406f")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0385157f530d9094e02ee54babe2c5c1e50291075a8ed5638f40ebe02cb6665eca")
				.unchecked_into(),
		),
		(
			// Validator 18 (5Gs1YvzVN2u95rjbFDxFdmF8BvpvWeWgEcGi4p46Mxca8G24)
			hex!("d441c94efc63d85b36e5e526cdf45658f9a1a49ef95d5fb11b3e3ac881c38e09").into(),
			hex!("9c6ff212c73649f545e9491e20ca490fa9224c97903d0c7cb33226c80577be65").into(),
			hex!("5e6498ebfabd7ef79b44e672376a6463777780514848af2d0202201e14bc1707")
				.unchecked_into(),
			hex!("99bb03fe796f8d9b2feb7ab931d29186212dc2ea8257f527e3a94fb89be38c83")
				.unchecked_into(),
			hex!("36528e8360c0de8d09ed0c75aec8af6dfa369eef78841d6829f61fc768032552")
				.unchecked_into(),
			hex!("34b9599cb65b284455b19c45a3d1acc299aa8f988fceca9d27ea3c58cd4bc548")
				.unchecked_into(),
			hex!("d0caefb6396f91bf6b876afa5187bae72d6a6ed8d28b5ddfcefc4d6f9514d448")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0214cf89a2e34598e1dbe114364d3767e0b512472793628c6418e2ccf1c6fa6012")
				.unchecked_into(),
		),
		(
			// Validator 19 (5Fjt81qbrvkskeFbCgUvQ3kyYA7BjKSsghXQS5VT98ctxsdN)
			hex!("a295d3287575815c83c1de28be46dd6b4c8ed528c84fd78fa2935da4d2dd6607").into(),
			hex!("bc8790fa81b4ff51333cd27fe65c05b11db0c3c07ba28ace6e3eef5ad80b361a").into(),
			hex!("76da67f41f7c5de3158883200211f7c7ce6dfb39ce4c3e93cfb482e5a1f59b18")
				.unchecked_into(),
			hex!("cbcada1bb20cc0f753c93cd047b7f6bc116a607770e5cdaf56af75ce7e669d43")
				.unchecked_into(),
			hex!("86438e7130540543b37766f2579f555dbd0fb3ace6af39198b63953d3ef52311")
				.unchecked_into(),
			hex!("fc2f281ffbf949eb1689384076a50456e69e9b784af7562505cfc3ba7ddf4e28")
				.unchecked_into(),
			hex!("16b9cf03d979a30315b746298a0922d02942519742ffac5785b0ab3319f82676")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03e537ddf29901fa3a3279aa235a3c0fc212deac32003eb846050b4d4a745761fd")
				.unchecked_into(),
		),
		(
			// Validator 20 (5D5YRetQTSwEZwvLP6fZSxKwYkMYWvjtPqmfgm48suSjHQZt)
			hex!("2cdfbaf5a4afd2b5c2bf2d7d21c6a00cc4e903b65c206f26a106b698f4c07a07").into(),
			hex!("d03ee9cb5da7ddd86d2321b6ed1dbac11e408ff17528d104a7db0d390461f401").into(),
			hex!("42141aa744a6aa48b779770379f4ef0531df4c46891f8fe4eaf48c237c5b544e")
				.unchecked_into(),
			hex!("1e2301bfd379f1e0385becc9a9fe516b4e82a62f36f751fac187d4ae231d40ce")
				.unchecked_into(),
			hex!("7e372de25e802a654d949856d5e6b83be6c153031302e27bfc76986717b9f805")
				.unchecked_into(),
			hex!("5cb510526d9b9dcdb3e699932c235d5ddf6b8b8737863ed0071cca3143ab620e")
				.unchecked_into(),
			hex!("e4bfdf1eb9226ef9bff278d3770f512db0b4ce695450399019221adc92697c30")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03a55fef63bcf95768efdcfdba7a459ff9ea19dd713d6475d395af8b3a8aebffe5")
				.unchecked_into(),
		),
		(
			// Validator 21 (5CcXoQJCfPZuTFq2iZG1RimSPs2xJHRMcjApTZHJphBMN5gU)
			hex!("18460017f648fc7548271f44bf430bcf34a13c5d11c2c7973e5872fc7f1bb679").into(),
			hex!("9a9b9e4b108f631322eb613e25e64409087a8682f336d050bc0a34652cc2ac18").into(),
			hex!("1c62e61e88cf3326e454607c3ae440580b6d93cd379ec65613841f56be55013b")
				.unchecked_into(),
			hex!("0152c766bc5fb84f44348aace3dc353ff747f2b49c2ce242acc630808e6929d0")
				.unchecked_into(),
			hex!("6acfe3455d2a0c1380d40e9d9428c850f363d588e460c7be19f2a138d524d139")
				.unchecked_into(),
			hex!("c4e09362cc4f22e1b90a523ca16f27c9e6938541926d3aa8d1b453b9596d6778")
				.unchecked_into(),
			hex!("3c5df4617b124baa961ce75266fcc4da642ea5d392631356198af631fcee1010")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03c53f46276c92ec235e483cc6df5e0a44cdafd40986c000bffaf79600caebd76b")
				.unchecked_into(),
		),
		(
			// Validator 22 (5GndGGg7A1G1tVLjbZcY9M5KCCLpp7UfW4JUNjFnE6Fh47dX)
			hex!("d0e9c77756f775aba3829920be3bc3ba9dcc28122d48a77830ffd36b73a86a2e").into(),
			hex!("24c6e8ea7f472b2ece933aefca0c21fb171cea641f94b18e504c1bd0a8242841").into(),
			hex!("501851e24834140a15e1e54fac0d5a791d7ce20211eb17b2b9715fbfbe2c733a")
				.unchecked_into(),
			hex!("7590c5e44e4caad2915349f9b262c086d5b98e58a4ff5ad848abccf3fba60ac7")
				.unchecked_into(),
			hex!("9440c09bc012cafe39bfdc13409444dcfcd58fb105d1a6cd9984561157b0e34d")
				.unchecked_into(),
			hex!("06ab371bd675b1692a6b69d7755384f17cf8594ff9bb62d9de8893b4ef377834")
				.unchecked_into(),
			hex!("a0b8f5b4531d7f70aaa98fa10844f418094a8bedfea09c1d0f1fd51238d3fb3a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03e86eceb5ac8fa30844c761b1ffa13f579e29a688d4fa2069c8b932d23403ea3f")
				.unchecked_into(),
		),
		(
			// Validator 23 (5EHCp5x4QPDUMgGp5aWZDJ4JvEKiwS4PJZJUa4Zodc7xHQBB)
			hex!("6200e272fd9ddab04fbd37565746e342aa4517d432bdb81b2f74d325b5745508").into(),
			hex!("d678190cc3b34e308747b089cdf0f7053cc0dfc99bac35aec816736f83fec95a").into(),
			hex!("544b3900238b5c51f35350c99c51119666514ffd394c7dffa41beab4da04b631")
				.unchecked_into(),
			hex!("57092f3776a76d1c65e06518dc69d91b6c431246696b8ce5324d982d57a61d02")
				.unchecked_into(),
			hex!("c60caeb65035e75b62fe2dc2cf1480acecfc1e23f4162cc28db80ea15d914f3d")
				.unchecked_into(),
			hex!("464fdd8c354c15106a11ef87b2bae0469aa0a6e852363dba1c0b4ae81e68b731")
				.unchecked_into(),
			hex!("2cbb09ace1cdec6a31e47a0ca13a337b386b0704c82172efc4329482215aab71")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("02830ccb14c81ca724551e84953bfd1bd2e9527bbc23a7cc161d4d0f663d37c38c")
				.unchecked_into(),
		),
		(
			// Validator 24 (5CnqLUZLZ4khcGx6FmWPuqMBRMC8GAkF6EWjNYT5MS2jFcTs)
			hex!("20217c2934ce0f4485d9b293a432a4be12deaeaf24f5899bea9738907c885a29").into(),
			hex!("deacd8b058301a08741a7d7e9b6a60c9804f85ceea8d9467915b9ebc9c00e61c").into(),
			hex!("b083ce6af9a920c0af1c221535348cdb43de02f72fac79e9b2fd8d00767bac7d")
				.unchecked_into(),
			hex!("2925028eae91d4566e51347e81404c1e117992cd0044f38eb8689de4dade620d")
				.unchecked_into(),
			hex!("ca457635ca90dd91504bd094733398fccf8ac8b2fe490add953ec25cf530f650")
				.unchecked_into(),
			hex!("40e9f9f972c2a8b8f8c9335fe14375579398396d429a21f582688e881e284279")
				.unchecked_into(),
			hex!("f87ef472806613e3180c485b82b45899ee8566eaaf51bfb61dbb141b9dfadb2a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0393766c3fa57ee7262f44332afd3e3e051373a87f1507ca039c22d5f44cf6b8f4")
				.unchecked_into(),
		),
		(
			// Validator 25 (5FnxvFT2huTXQRLEBY83ebSJc6Lf8CxQoUf1fDRpRNphEqhv)
			hex!("a4efb5ae7e9cefcd231426b093768464569455d37be76844410a410c349d6a1d").into(),
			hex!("66185d7af5c44b626148b0351779a628890458be2598d9610e52ac45a690d001").into(),
			hex!("7295a9877f6326de6c884878ab8ea062e49cfaca0f81c0ccbbf184bfc066b45a")
				.unchecked_into(),
			hex!("fbd33747e8080e4010822015d89c24ed3c9a99ec3f372cc456ef13ecaa6a2e02")
				.unchecked_into(),
			hex!("90d62096f4794c216883b402aa30df5d59fc4fce315585e4c663c8544166ee5f")
				.unchecked_into(),
			hex!("c82010ad6a8920202f6e398dfe5c7ca3e7068543cc9b4db3404c8b78edd1395b")
				.unchecked_into(),
			hex!("24339137d4eda2f1a3464900db0efccb2ada850ef59df2eb353978afa8fe3e03")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("028a46d348f39efb89388c73760cf9ba2da7a920d9d0c7dfb4fd5503f9fa1f6211")
				.unchecked_into(),
		),
		(
			// Validator 26 (5FWoqakZ1Bc4s1X5QhM72wWhx4tUiXWU6D1tkQDKAjb9sXNv)
			hex!("989d373435593082d15eb02a1bca8cb57a26bcad3559193f2198599125240e12").into(),
			hex!("7812912efb97dd1ea1cb90834a4af0e4483367aabc1b1c62891c56a5c790c37a").into(),
			hex!("6e717be6a8b874c53e7804d0bc9cc0a102cf6738b91c98adaa86135902f5cb4f")
				.unchecked_into(),
			hex!("139bad414d9dd2fe40faac4da0e763af04318b6b481a43a92555b3c42274416e")
				.unchecked_into(),
			hex!("d03f51c955ebb8bfe63bb6b472443176dcb2c53caae53ffe9be6db8df341bc7f")
				.unchecked_into(),
			hex!("aca64f0425038f8597deaebbeca2f8faa019fc7169ec392f1e28648fe3f70f67")
				.unchecked_into(),
			hex!("08a2fc717f65c6964556ac18dae1c135516960a63ebd74133f773221ba98dd18")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0215ab88e40a0edef0c1d61a358bd4aa7040c00c800fbd3dc654265d4bf1ddc564")
				.unchecked_into(),
		),
		(
			// Validator 27 (5GHk3DkZaK63UQ4UGGQDJHYDuXV6DdeKVRbQsMybaQrirabD)
			hex!("bae27816cf59abdb824df0992f28426bfcab1a51a7c4a1f2877d07b6377cc444").into(),
			hex!("4293ca7e260b3b80542a72da4f217eb0eb2d7d034c4adc959051e42cbab94b61").into(),
			hex!("901e143752afb367c5dee4196d725c75ed3521e2791754963700a38750413743")
				.unchecked_into(),
			hex!("b3644efaea004c393d8c30f74572efd7a2022a7c6473b43a867ceb71e98aa0ec")
				.unchecked_into(),
			hex!("06f580f3212475c043d2c6b61efff45aafb794e7a347c99e43179e41c3fb0302")
				.unchecked_into(),
			hex!("c88f1c2731d65bcbde6457e5c7b2b640cf1019595439c567360d9a32e77c9017")
				.unchecked_into(),
			hex!("bca548acf412c177f550079d1ccafa3bcb0957ea11865499b8074297af617053")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03fba0d558166aed91eb9522aac881dfc664fcd1f5aa69361ba12dcb7aec5100b3")
				.unchecked_into(),
		),
	]);

	// What the validators are funded with, taken out of the treasury's share rather than added
	// beside it.
	//
	// It used to be added: the four allocations summed to 200M and then every validator got
	// `STASH * 2` on top, so genesis actually minted 200,000,800 HEZ here and 200,004,200 on
	// Pezkuwichain. `hez_allocations_sum_to_200m` did not see it -- it adds four constants,
	// and the constants were right; what was wrong was the sentence beside them saying the
	// constants are the genesis supply. Two hundred million is a claim this project makes in
	// public, so the arithmetic is made to match the claim rather than the claim relaxed to
	// match the arithmetic.
	//
	// The treasury pays because bootstrapping the validators is what a state treasury is for,
	// and because it is the only allocation here big enough not to notice.
	// Divided out of the constant rather than multiplied up from a count, so seating another
	// validator moves nobody else's share and the Asset Hub's subtraction stays correct.
	let validator_funding: u128 = pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING;
	let validator_count = initial_authorities.len() as u128;
	let per_validator: u128 = validator_funding / validator_count;
	// Integer division leaves a remainder whenever the count does not divide the constant, and
	// dropping it would mint less than two hundred million. The first validator carries it.
	let first_validator_extra: u128 = validator_funding - per_validator * validator_count;

	// The XCM checking account's seed.
	//
	// `TeleportTracking` is `Some((CheckAccount::get(), MintLocation::Local))` here, so HEZ
	// that leaves for a teyrchain is held in this account rather than burned, and HEZ coming
	// back is released from it. An empty account therefore does not mean "nothing has moved";
	// it means nothing *can* move back, and the teleport is refused. That refusal is correct
	// and it is what two relay-side tests assert -- which is why this is a genesis matter and
	// not something to notice in production.
	//
	// The size is derived, not chosen. The rule: the seed must cover the most that could ever
	// come back, which is the HEZ in circulation on the other chains. At genesis that is the
	// airdrop pot and the presale pot, both minted on the Asset Hub -- everything the relay
	// does not mint itself. Writing a round number here instead would make the testnet
	// rehearse a flow the mainnet then fails, which is the whole reason a rehearsal exists.
	//
	// This is a mirror rather than new supply: the same HEZ is represented on the Asset Hub
	// and escrowed here, exactly as a teleport out would have left it. Governance is not
	// distorted by the size because `MaxTurnout` reads `VotableIssuance`, which is active
	// issuance minus this account.
	//
	// Two of the founding office's three budgets are held off this chain -- one on the Asset
	// Hub, one on People -- so the escrow covers them too. Without this the seed would be
	// short by exactly what those two chains mint, and the first teleport home of that size
	// would be refused with nothing on either chain saying why.
	let checking_account_seed: u128 = HEZ_AIRDROP_ALLOCATION + HEZ_PRESALE_ALLOCATION + HEZ_TREASURY_ALLOCATION
			- pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING
			+ 2 * pezkuwichain_runtime_constants::currency::HEZ_FOUNDING_OFFICE_FUNDING
			+ pezkuwichain_runtime_constants::currency::HEZ_ACCUMULATION_PEOPLE
			// The collators' pots on the Asset Hub and on People, minted there.
			+ pezkuwichain_runtime_constants::currency::HEZ_COLLATOR_POT_CARVE_OUT
			// The genesis validators' stakes are held on the Asset Hub too (spec K5).
			+ pezkuwichain_runtime_constants::currency::HEZ_GENESIS_STAKE_CARVE_OUT;
	let checking_account: AccountId = crate::XcmPallet::check_account();

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: vec![
				// HEZ Genesis Distribution (200M Total)
				// The airdrop's 40M is not here. It is minted straight into the Asset Hub's
				// airdrop pot, a keyless treasury instance -- so no key ever holds it and no
				// manual transfer has to be remembered after launch. See
				// `HEZ_AIRDROP_ALLOCATION`'s comment and the Asset Hub's `AirdropPot`.
				// 10% = 20M HEZ, less the fee budget carved out for root below.
				(
					founder_account.clone(),
					HEZ_FOUNDER_ALLOCATION
						- pezkuwichain_runtime_constants::currency::HEZ_SUDO_FUNDING
						- pezkuwichain_runtime_constants::currency::HEZ_FOUNDING_OFFICE_CARVE_OUT
						- pezkuwichain_runtime_constants::currency::HEZ_ACCUMULATION_CARVE_OUT
						- pezkuwichain_runtime_constants::currency::HEZ_COLLATOR_POT_CARVE_OUT
						// The genesis validators' stakes, minted and bonded on the Asset Hub.
						- pezkuwichain_runtime_constants::currency::HEZ_GENESIS_STAKE_CARVE_OUT,
				),
				// Root's fee budget, held by its three signers: a multisig account pays nothing
				// itself -- the signer who submits `as_multi` pays the fee and holds the deposit.
				// Carved out of the founder's share, not added to it, so the genesis total is
				// untouched -- the same shape as `HEZ_VALIDATOR_FUNDING` coming out of the
				// treasury's. Without it the first `sudo` call cannot pay and the chain launches
				// ungovernable.
				(
					sudo_signers[0].clone(),
					pezkuwichain_runtime_constants::currency::HEZ_SUDO_SIGNER_FUNDING,
				),
				(
					sudo_signers[1].clone(),
					pezkuwichain_runtime_constants::currency::HEZ_SUDO_SIGNER_FUNDING,
				),
				(
					sudo_signers[2].clone(),
					pezkuwichain_runtime_constants::currency::HEZ_SUDO_SIGNER_FUNDING,
				),
				// The founding office's fee budget on this chain. Two more like it are minted
				// on the Asset Hub and on People; all three come out of the founder's line
				// above through `HEZ_FOUNDING_OFFICE_CARVE_OUT`, and the two that are held
				// elsewhere are escrowed in `checking_account_seed` below. Nothing is added
				// to the supply and nothing is taken from anyone but the founder.
				(
					serok_account.clone(),
					pezkuwichain_runtime_constants::currency::HEZ_FOUNDING_OFFICE_FUNDING,
				),
				// The accumulation account, funded to its existential deposit so a fee smaller
				// than that deposit is kept rather than refused and burned. Out of the founder's
				// line through `HEZ_ACCUMULATION_CARVE_OUT`, like the office budgets; People's
				// twin is minted there and escrowed below.
				(
					pezpallet_accumulate_and_forward::Pezpallet::<crate::Runtime>::accumulation_account(),
					pezkuwichain_runtime_constants::currency::HEZ_ACCUMULATION_RELAY,
				),
				// The treasury's 40M is not here either. It is minted into the account the
				// Asset Hub's treasury pallet pays from -- see `HEZ_TREASURY_ALLOCATION`. What
				// stays on this side of it is the validator funding, below.
				// Escrow for what the Asset Hub holds -- see `checking_account_seed`.
				(checking_account, checking_account_seed),
			]
			.into_iter()
			// The validators' stashes, divided out of `HEZ_VALIDATOR_FUNDING` rather than
			// added on top of the allocations -- see the constant's own comment.
			.chain(initial_authorities.iter().enumerate().map(|(i, x)| {
				(
					x.0.clone(),
					if i == 0 { per_validator + first_validator_extra } else { per_validator },
				)
			}))
			.collect::<Vec<_>>(),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (
					x.0.clone(),
					x.0.clone(),
					pezkuwichain_session_keys(
						x.2.clone(),
						x.3.clone(),
						x.4.clone(),
						x.5.clone(),
						x.6.clone(),
						x.7.clone(),
					)
				))
				.collect::<Vec<_>>(),
		},
		babe: BabeConfig { epoch_config: BABE_GENESIS_EPOCH_CONFIG },
		sudo: SudoConfig { key: Some(sudo_account) },
		configuration: ConfigurationConfig { config: default_teyrchains_host_configuration() },
		registrar: RegistrarConfig { next_free_para_id: pezkuwi_primitives::LOWEST_PUBLIC_ID },
		staking_ah_client: StakingAhClientConfig {
			operating_mode: pezpallet_staking_async_ah_client::OperatingMode::Active,
			..Default::default()
		},
	})
}

// ============================================================================
// MAINNET SIMULATION PRESET - For local upgrade testing with real sudo key
// ============================================================================
//
// 2 validators: mainnet's Validator_01 and Validator_02, session keys included, each loaded
// into a local keystore from its own phrase (never the master's)
// Sudo = real founder account (requires SUDO_MNEMONIC at runtime)
// NO Alice/Bob — tests the exact upgrade path used on mainnet
//
fn pezkuwichain_mainnet_simulation_genesis() -> serde_json::Value {
	use hex_literal::hex;
	use pezsp_core::crypto::UncheckedInto;

	// Real founder account -- the new mainnet founder, generated 2026-09-15.
	// SS58: 5DPA5ctyUhFZcLoqNj11w1xEn3QqtDSmUjk4L6YxQNBWiDxS
	let founder_account: AccountId =
		hex!("3a4eed1ba224f6d76dec6f24da10b850248dc8db5e8de7effcaf25bea977fe7f").into();

	// 2 validators — real mainnet Validator_01 and Validator_02 keys, the same tuples as
	// `pezkuwichain_genesis_config`. Their session phrases are kept off-repository, one per key.
	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
		BeefyId,
	)> = Vec::from([
		(
			// Validator 01 (5G4e6KKUViiwUp6qPgvdqNxwtaGM7LzYB2KVr9bXMjJAfMDF)
			hex!("b0e442bf467ef368a9247474bf1a97a1c08ad9213673e3180fdab407a758ac3c").into(),
			hex!("10aec0daa42782e62b375845a5a496d6943a3b28c0d8803c12df0be24a641d22").into(),
			hex!("4e261f75a97625fe1d4258ae7671c1aeb77d29022282c235f8b25ff721f6de47")
				.unchecked_into(),
			hex!("12359b7c659f93473a8cdc9f236a4c9dab90dce478ef5a21d4a8be9587552565")
				.unchecked_into(),
			hex!("94f3bd94986d886db66743988c679be0f6d134bc10ec99809e577f788a490543")
				.unchecked_into(),
			hex!("3aa9fdea783b84df2f3a5b1024720de6b781c2df5e2bf65aa6794271c9a22874")
				.unchecked_into(),
			hex!("985b961a9fec7c232e90a609f908addb6129b3883da5b6b281b92bfe1cbbbf16")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("02507df6ce8f663ea68a3ea29da7ac1fda8dd467461f07d1afefa67b7969f1a59f")
				.unchecked_into(),
		),
		(
			// Validator 02 (5E5239nsL71Qb2MUmPwPtUM8pLb1Md4KkNBajmJ6wAHQwZP1)
			hex!("58b5ab34bfef86ccd2fda24b4b4034d8e77f03e24b19c810b1d3025456036806").into(),
			hex!("9cb78ac63399b9257d8ccb5a4fd0160e251f971b0ca5d4a8fb98d196e4b7721a").into(),
			hex!("8a1d0c705d1dd6e08bd3f75e4da1567a03b53baf26286b3db033f11cce4de327")
				.unchecked_into(),
			hex!("be0362dbef8ed5a21686a324ddf78575e09fe619944fa26fa79664888631034e")
				.unchecked_into(),
			hex!("84141c4692dda61075de310540ec4bde2d59508b8c58d6f1218442540ff88704")
				.unchecked_into(),
			hex!("e4226d2d23e1ace8ba9b4204a2fa3e9d416bd40349991f3c51aa92f3e54c9466")
				.unchecked_into(),
			hex!("8216ddb36f2229edc194f29fd87dee7777f24b09d749dce381f54fa9d2f2b30a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("026e1a3e151b57fe4cffb9967c9c816ecc227ae13633400937b02faea2cc6ea998")
				.unchecked_into(),
		),
	]);

	const STASH: u128 = 100 * HEZ;

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: vec![
				// Founder gets enough balance to pay for upgrades + testing
				(founder_account.clone(), 1_000_000 * HEZ),
			]
			.into_iter()
			.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH * 2)))
			.collect::<Vec<_>>(),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						pezkuwichain_session_keys(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
							x.6.clone(),
							x.7.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
		},
		babe: BabeConfig { epoch_config: BABE_GENESIS_EPOCH_CONFIG },
		// Mainnet's root is the signers' multisig now, not the founder's key and not one key.
		// A simulation that keeps an old shape rehearses a chain that no longer exists.
		sudo: SudoConfig {
			key: Some(pezkuwichain_runtime_constants::genesis::SUDO_MULTISIG.into()),
		},
		configuration: ConfigurationConfig { config: default_teyrchains_host_configuration() },
		registrar: RegistrarConfig { next_free_para_id: pezkuwi_primitives::LOWEST_PUBLIC_ID },
		staking_ah_client: StakingAhClientConfig {
			operating_mode: pezpallet_staking_async_ah_client::OperatingMode::Active,
			..Default::default()
		},
	})
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	use preset_names::*;
	let patch = match id.as_ref() {
		// ====================================================================
		// GENESIS PRESET - For mainnet with HEZ distribution
		// ====================================================================
		PRESET_GENESIS => pezkuwichain_genesis_config(),

		// ====================================================================
		// LOCAL TESTNET PRESET - For local multi-node testing
		// ====================================================================
		pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => pezkuwichain_local_testnet_genesis(),

		// ====================================================================
		// DEV PRESET - For single-node development
		// ====================================================================
		pezsp_genesis_builder::DEV_RUNTIME_PRESET => pezkuwichain_development_config_genesis(),

		// ====================================================================
		// STAGING TESTNET - For pre-production testing
		// ====================================================================
		"pezstaging_testnet" => pezkuwichain_staging_testnet_config_genesis(),

		// ====================================================================
		// VERSI LOCAL TESTNET - Extended local testing
		// ====================================================================
		"versi_local_testnet" => versi_local_testnet_genesis(),

		// ====================================================================
		// MAINNET SIMULATION - Local upgrade testing with real sudo key
		// ====================================================================
		"mainnet_simulation" => pezkuwichain_mainnet_simulation_genesis(),

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
		PresetId::from(pezsp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
		PresetId::from(pezsp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from("pezstaging_testnet"),
		PresetId::from("versi_local_testnet"),
		PresetId::from("mainnet_simulation"),
	]
}

/// A local or dev chain can reproduce a teleport, because its escrow exists.
///
/// `TeleportTracking` is `MintLocation::Local` here, so HEZ arriving from a teyrchain is paid
/// out of the checking account rather than minted. An unseeded account does not exist, every
/// inbound teleport fails with `NotWithdrawable`, and the sender's balance is gone on the far
/// side while its extrinsic reported success. That is the defect the Zagros launch shipped on
/// 2026-09-11 -- and the preset a developer reaches for first could not have shown it, because
/// the account was missing here too. A defect the testnet cannot reproduce is one the testnet
/// cannot catch, which is the whole argument for this test existing at all.
#[test]
fn the_local_preset_seeds_the_escrow() {
	let genesis = pezkuwichain_local_testnet_genesis();
	let amount = |e: &serde_json::Value| -> u128 {
		e[1].as_u64()
			.map(u128::from)
			.unwrap_or_else(|| e[1].to_string().parse().unwrap())
	};
	let rows = genesis["balances"]["balances"].as_array().expect("balances");

	use pezsp_core::crypto::Ss58Codec;
	let checking = crate::XcmPallet::check_account().to_ss58check();
	let seeded = rows
		.iter()
		.find(|e| e[0].as_str() == Some(checking.as_str()))
		.map(amount)
		.unwrap_or(0);
	assert!(seeded > 0, "the checking account must be seeded or no teleport can arrive");

	// And the rule is the production one: the two halves are the whole supply, counted once.
	let total: u128 = rows.iter().map(amount).sum();
	assert_eq!(
		total,
		200_000_000 * HEZ,
		"a testnet that mints a different supply than the mainnet is rehearsing a different \
		 chain -- the endowments and the escrow together are the two hundred million"
	);
}

/// The genesis seats the twenty-seven validators the fleet is built for.
///
/// The authority block is generated (`res/genesis/mainnet/emit_relay_authorities.py`) and the
/// generator takes a `--count`. Run it with the wrong one and the chain launches with a
/// smaller set than the machines standing behind it -- which is not a shortfall that shows up
/// as an error. The relay has no staking pallet; the genesis authority list *is* the set. Seat
/// four and start twenty-seven and the extra nodes sync without ever being asked to sign; seat
/// twenty-seven and start four and GRANDPA never finalises, because the threshold is two
/// thirds. Both look like a healthy chain from the logs of the node you happen to be reading.
/// The relay seats exactly the stashes the Asset Hub's genesis bonds (spec K5): one list in
/// `pezkuwichain_runtime_constants::genesis`, read by both. A seat whose stash is not bonded
/// there earns nothing; a stash bonded there that is not seated validates nothing.
#[test]
fn the_relay_seats_the_stashes_the_asset_hub_bonds() {
	let genesis = pezkuwichain_genesis_config();
	let seated: Vec<serde_json::Value> = genesis["session"]["keys"]
		.as_array()
		.expect("the session patch lists keys")
		.iter()
		.map(|k| k[0].clone())
		.collect();
	let bonded: Vec<serde_json::Value> = pezkuwichain_runtime_constants::genesis::VALIDATOR_STASHES
		.iter()
		.map(|s| serde_json::to_value(AccountId::from(*s)).expect("an account id serialises"))
		.collect();
	assert_eq!(seated, bonded);
}

#[test]
fn the_genesis_seats_twenty_seven_validators() {
	let genesis = pezkuwichain_genesis_config();
	let keys = genesis["session"]["keys"].as_array().expect("the session patch lists keys");
	assert_eq!(
		keys.len(),
		27,
		"mainnet launches with twenty-seven validators (Serok, 2026-09-12) -- regenerate the \
		 authority block with `emit_relay_authorities.py --count 27` if this moved"
	);

	// One stash, one seat. A duplicated tuple would pass the count and give one key two votes.
	let mut stashes: Vec<&str> = keys.iter().map(|k| k[0].as_str().expect("a stash")).collect();
	stashes.sort_unstable();
	let seated = stashes.len();
	stashes.dedup();
	assert_eq!(seated, stashes.len(), "a validator is seated twice -- one key, two votes");

	// Every seat is funded, or its first heartbeat cannot pay.
	let funded: Vec<&str> = genesis["balances"]["balances"]
		.as_array()
		.expect("balances")
		.iter()
		.map(|e| e[0].as_str().expect("an account"))
		.collect();
	for s in &stashes {
		assert!(funded.contains(s), "validator {s} is seated but holds nothing at genesis");
	}
}
