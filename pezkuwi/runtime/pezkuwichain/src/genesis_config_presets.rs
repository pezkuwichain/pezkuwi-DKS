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
	let here =
		HEZ_FOUNDER_ALLOCATION + pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING;
	let on_asset_hub = HEZ_AIRDROP_ALLOCATION + HEZ_PRESALE_ALLOCATION + HEZ_TREASURY_ALLOCATION
		- pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING;
	assert_eq!(
		here,
		20_001_000 * HEZ,
		"the relay mints the founder's 20M and the validators' funding out of the treasury"
	);
	assert_eq!(here + on_asset_hub, 200_000_000 * HEZ, "HEZ total supply must equal 200M");
}

/// Root can pay for its own first call.
///
/// The two supply tests above both stay green whether root is funded or not, because the
/// funding is carved out of the founder's share -- the total never moves. That is the right
/// shape for the supply and the wrong shape for a guard, so the guard is here: it reads the
/// key out of `SudoConfig` and asks what the genesis actually mints to it.
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

	let to_root: u128 = balances.iter().filter(|e| e[0].as_str() == Some(root)).map(amount).sum();
	assert_eq!(
		to_root,
		pezkuwichain_runtime_constants::currency::HEZ_SUDO_FUNDING,
		"root must launch with its fee budget -- without it the first sudo call cannot pay"
	);

	// Carved, not added. If someone later funds root by minting extra, the supply tests catch
	// it; if they fund it by taking from somewhere that is not the founder, only this does.
	let founder_line: u128 = balances
		.iter()
		.map(amount)
		.find(|&a| {
			a == HEZ_FOUNDER_ALLOCATION - pezkuwichain_runtime_constants::currency::HEZ_SUDO_FUNDING
		})
		.unwrap_or(0);
	assert_eq!(
		founder_line,
		HEZ_FOUNDER_ALLOCATION - pezkuwichain_runtime_constants::currency::HEZ_SUDO_FUNDING,
		"root's budget comes out of the founder's allocation, not on top of it"
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
	let owned =
		HEZ_FOUNDER_ALLOCATION + pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING;
	// Escrow: the mirror of what the Asset Hub holds, so a teleport back has something to
	// release. Not new supply -- the same HEZ, represented there and held here.
	let escrow = HEZ_AIRDROP_ALLOCATION + HEZ_PRESALE_ALLOCATION + HEZ_TREASURY_ALLOCATION
		- pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING;

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

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts.iter().map(|k| (k.clone(), ENDOWMENT)).collect::<Vec<_>>(),
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

	// The chain's root key -- its own account, no longer the founder's.
	//
	// It used to be `founder_account`, which put root authority and personal property on one
	// key: losing or leaking it would not have cost the founder money, it would have handed
	// over the chain. Root is also the one authority meant to be retired, and retiring an
	// account is cleaner than retiring a person's wallet. Zagros has rehearsed the separate-key
	// shape since its own genesis; a testnet that rehearses a different topology than the
	// mainnet is not rehearsing the mainnet.
	// SS58: 5D4o1HMKEntLafi1f2tz4U9XVZpg1gz5YAR6mgcgoPT4jgzU
	let sudo_account: AccountId =
		hex!("2c4d909d9cba926dcf9cad71a154cc64adbbb0e8066b353a10e062707199c97c").into();

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
	// INITIAL VALIDATORS - 21 validators - Generated 2026-01-29
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
			hex!("5872d64bd3a67b8c8272c9b3eaaa4853333509076f09942d0560236756e8cb37")
				.unchecked_into(),
			hex!("40316a0bf1f562ad339556975360bcd97dc688b53b93c230f08f5ddab4dd79e2")
				.unchecked_into(),
			hex!("eaa8333a15b4d18ca40aeb5e53154f56acb8d5ce024129ee17904f5140d2464c")
				.unchecked_into(),
			hex!("7a30d53d07680b55bffc423d0dd75cf4c0d86e82d0c6ee3320c07e7c389c746b")
				.unchecked_into(),
			hex!("f8784e05c19f61e58eaeaa780c9315ec15e9e794110fb0f511e89baec1e38c7a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03559f13b26109776b0cd0074b6612cf1b9f0e2198af6f3d66c2a4ec6ddcf39407")
				.unchecked_into(),
		),
		(
			// Validator 02 (5E5239nsL71Qb2MUmPwPtUM8pLb1Md4KkNBajmJ6wAHQwZP1)
			hex!("58b5ab34bfef86ccd2fda24b4b4034d8e77f03e24b19c810b1d3025456036806").into(),
			hex!("9cb78ac63399b9257d8ccb5a4fd0160e251f971b0ca5d4a8fb98d196e4b7721a").into(),
			hex!("d874b6f6f9bf67d55ec69838129744fec8237fd8e77c2828ce9460a20916e42e")
				.unchecked_into(),
			hex!("e8f1ba8114c0eb72f540c7be75d675effbc87318154da4b53ca03f849960669a")
				.unchecked_into(),
			hex!("bce8e0618ad82b86e5fc775363fbbd5c230657fc431501e2c7229f289e477c62")
				.unchecked_into(),
			hex!("887b1a1d2d1155c16dcf64b070684a8a981b7ab99077ad66848739cc8f87c952")
				.unchecked_into(),
			hex!("764f5241852f479240812f969cd1393242cb318b8f2f9cee800dafef6803983a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("024d226f333f7d0a9f82bd3eba49e4585139f84934cd71dc052412768cd62a811f")
				.unchecked_into(),
		),
		(
			// Validator 03 (5Ck4sveGyXL8Y3Fd1cqcEnUmXwRxWRE64AyUVaKdVumnbbSm)
			hex!("1e055605aae178fb4db058e1ffa438812cecd16c45e194ed831c27508bea7519").into(),
			hex!("fe6af42ab4a7bb2e10433e80bd9fdc111d6e72ce8fe08669367814c42c760650").into(),
			hex!("cafe0fa9ada0cf4c855ee1b2fa4470afb08412f40c3b4fd55bfa916620beb251")
				.unchecked_into(),
			hex!("dfef0795a5a0d62ccef2ac266dbc00da8d7afd0edd73105f25cf965f5e6b4aa4")
				.unchecked_into(),
			hex!("4e91e79bca68a4eac754275547e50e1cf48fb94f22c4e96f9d22be5e21c6c608")
				.unchecked_into(),
			hex!("ac1686aeb648ad028b7e5529a68c38103ae23aa11eeea0d6b266e1c3a34bd400")
				.unchecked_into(),
			hex!("f2489b2ef55e46a12d30489587b14042a70f11d71e962ba44557f7e329a8104e")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("02a05b096d37ec53246e20be2576c8c90dc1cc3db91a8222fd3d8adcc39453bb38")
				.unchecked_into(),
		),
		(
			// Validator 04 (5ECHUqJ8KczMu3ChhX2f9JD4w9Z9ZfgzKbu5KEJ7TobL1DBW)
			hex!("5e405f2306593d90387bf84c40132072b1ace7cd09b779ffc621b45f578acb0c").into(),
			hex!("7e1b682e851d28ca52a6bf7c4a2427bcc972ea3d2e050d9a76a6bbeb1962cc0e").into(),
			hex!("485094935a78748d4dbee6a9482df8c57822da31363b185ede96f7086561ae7d")
				.unchecked_into(),
			hex!("8824112b2ae487c1a2803249c9a216cd7b98bec48f56419f230c1c73a8a73982")
				.unchecked_into(),
			hex!("7836fe64613c43d1d57d0f19913e175cbbc9d815d33441b5fc0b2323c27b406e")
				.unchecked_into(),
			hex!("8eb6cec50639d6ef23075e92267ff808807366995f57950321f963daca1ab50e")
				.unchecked_into(),
			hex!("568af92e0e13f50b7388290cad3c3a3756935f18415d335f4a145e7bae58a217")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("027c0b598c6dec74bd84f16b85efb0b0b845dba13466d962213d7711a0e71b9319")
				.unchecked_into(),
		),
		(
			// Validator 05 (5DUwAkAaBme36xwFePyWyyVPAJs1fLDurtXuNopPPCQJnCnw)
			hex!("3eb6ef27bff8784a6354dd004b2258d95d2b2a655719abac1a297a7f014f324d").into(),
			hex!("cad4fdbd3b4a8cce42af32035b7aea18fcb5ae656d2a918c8df1cd1787dc987a").into(),
			hex!("2af495161432d6c5494916f06b7e078149eec323c1368638cd400062a1b3e030")
				.unchecked_into(),
			hex!("8123e5f047c72426ee4e07aff03cc82b1d7fb7f68bbfa76d702efcaaacbdb1c6")
				.unchecked_into(),
			hex!("e2d08eea49545ba511789e509f8ce6fe05a2839493d5d0c91edd9e03bc6dde70")
				.unchecked_into(),
			hex!("b4d0b0f11462da6ca6f3976e7742438a06282cef82899e252cca3dbc22cd043a")
				.unchecked_into(),
			hex!("04831e0dc76942ce3bbd07366c67723366d078eac46aea322c3c07f5aaa98c63")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0311a5be1d05715f035493889cc230cd7dc635559f78a1ec38aa115ba6dd2d14de")
				.unchecked_into(),
		),
		(
			// Validator 06 (5CoMjECS3gHczmRd24NZGyATuP4iCmU3S4p97Aehinudegcp)
			hex!("2087cb677d08ec47c6a3bc36b94e92226d47f32ebf18dbe39808014f3bb1f674").into(),
			hex!("76a9437cb077eabff5835feba2ef47f27c5168f317bd30a89c08be5ca8ab2872").into(),
			hex!("50a2c2a5ba7fe265a3da7eac759c7602369783dcf3e7fb1a861cb11f88954d20")
				.unchecked_into(),
			hex!("df309fd938de2732c88ef7e49b5ad403ab4accc16839b03ac60322aea1463f9b")
				.unchecked_into(),
			hex!("86886b9b84c03e1c2b46db93c2f12b1fa4eea36621d8c034ca11f12665617305")
				.unchecked_into(),
			hex!("e0b993a7fd0d8df2f1d957be6d365bfde41dd0eb473cca11cbad9eb61a1af277")
				.unchecked_into(),
			hex!("d05493743c2bd106ac99aa33726964071ec9281ee17792179c6da8bcce4c5c3e")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0230d3fbb3dfd2249aff718bf23a1d840e8835f2e86449c6af94dea5886caeab9e")
				.unchecked_into(),
		),
		(
			// Validator 07 (5ENhEwumArbBpLNqc258MQe3pyZhgKjNMWTL8NaeCoc3Ayyf)
			hex!("6630cf9996f36ebd2586a0600324f0268f8bc54dc50e56d31ea6cfa7f8234219").into(),
			hex!("4c95aa79099097ad8d903873d47d1da2db6e7b44e0916eeef8d3817a5713ca4a").into(),
			hex!("e0fa4be2994a7c865e42a54ee05d308463e440c5cab6ccd009e902cfe3cc6570")
				.unchecked_into(),
			hex!("6c0ed28e099233629c22b523a3eb40d2fd75c1f82859afbc58cea9fa042f8c95")
				.unchecked_into(),
			hex!("e8ec9f17420d88f529b8fbaff2864b8c302983b8bb2bedb0a80da9de1c972e74")
				.unchecked_into(),
			hex!("3c8c7adb53af358f937f1e934899776519cafcb5b3ed2ec05e3ed15bc02ffa07")
				.unchecked_into(),
			hex!("40e187d7f4c8527fff54f0b3491e7c44a2028b80fe0cede4702dcebb99010b2a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0337f5e5e37d4de6fa294d92a163613f7eeabd08bfa1da5360caa6b21c14b2dc85")
				.unchecked_into(),
		),
		(
			// Validator 08 (5Ey4cgchows4382WhyvMaoLeNENFoUpg3Zz5iHidZti7ypYy)
			hex!("80671bca44671b267fe03e28ab3b67fac9ff558ac46c82f3462fdb7bfb66f72c").into(),
			hex!("5af61022f0c41f9dcf5fc8345f376a45c538e357b1d03a6885235e9a4c0af72a").into(),
			hex!("fa333b84e173b77a1252714e0ff239199d1be12128e88a4b2b7bcde48f769b2c")
				.unchecked_into(),
			hex!("db4c5435c3e8baa1480d2b7ed845db20890153b1f63a31ee92791808fe9f488a")
				.unchecked_into(),
			hex!("7a98231916dc169da0340f4e5d459c87048f0782711302a5ef5ea3ec44f5ec45")
				.unchecked_into(),
			hex!("3e85568e3c76db9b58e2ab5e5aa7b899238e09e98284a3b759612d5ab6951c52")
				.unchecked_into(),
			hex!("a0fd5e62c340ea350108ae4d07c16546f42f7e13e9b39427c867deca4701f741")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("030bff15f261f9fbf581600f6fc3f4ecf7bd7353e48f709e4dab444f910dc81fde")
				.unchecked_into(),
		),
		(
			// Validator 09 (5H8YwbdehipCdPjLHkrCh5wHgaza5iVE1bhGiavGG7reWGLR)
			hex!("e01c2365959e8cda2e23d25bf1488cf9680091ebe4da865a873b9b2996a56a5e").into(),
			hex!("b01defd96ba59f5e344093cba6716ef4a6a3cf8c70668917b878e9cf1b21ff59").into(),
			hex!("3aef62bf31af07b4f6b318a034c600e73ef1397ee83a6855d5ebb8eb85cffd01")
				.unchecked_into(),
			hex!("c789ada0efbf3051d49fd44e31d8ae5da246a4d774fb924ed39bd6c53b42f58a")
				.unchecked_into(),
			hex!("98ca3b9e67a6bbcbcc830cd8fae569abe52422bb7cde15b64acb9ca95ba28675")
				.unchecked_into(),
			hex!("066ade71a6a7c6a2c21a3235f29937edf2e29c8a1c8c1441b00d7ee2472a9616")
				.unchecked_into(),
			hex!("20aa365264b8308d75753a18d979060127949c577b350f7b50e7f0883934d514")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("02298f0898739a74d01cf1c86e5e9f93187dbc2d9f87755551dd3858419ecf7eac")
				.unchecked_into(),
		),
		(
			// Validator 10 (5EhMtwRJCJ8fQ8ZyNxVL5FGExjaobwgnGbrsq3dJoxJdoJEX)
			hex!("746d5a4641e7de16283ae25192d2b87bf397cedeeca132b70e244dc47638fb3f").into(),
			hex!("0e8477cfc2ff4e3fd51ed972f1e80d51ef30277bcfa193fa43c5547346201a68").into(),
			hex!("98ff4663426ff3e59feb5219de3d29e801cb279f2cc46972f4be415627c3c479")
				.unchecked_into(),
			hex!("6b1d5c8a6d54df9006bf81c30d165d1bb0005f0aa9953749b593c4e93a54851a")
				.unchecked_into(),
			hex!("eab594e66c1e4f2e284b19cf4a8b7f7e3ababe074a3f5e2f65cd9536f8b14968")
				.unchecked_into(),
			hex!("204f5feb211bcce5fe8fc942ccc5880546d0f131e4503954f18559b5422f934f")
				.unchecked_into(),
			hex!("08f3ef484bb1d38b33e43d5d81eb2148a4f70a99a23a2bacf8a1bbf6fb8eeb57")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0384ee0dcf5c7f74bc1e5e895b1fe4f0974f76f7c100667995e3c8e2efc1ebffd2")
				.unchecked_into(),
		),
		(
			// Validator 11 (5Fmm54cqKb7PywNjkZGro97eNHAYFrJt4vU2G3D4T8ubwvbW)
			hex!("a404948873be26098cc9d1acb1f08e34bc4f5e7a40ca0aec87d20b18a5d85158").into(),
			hex!("b2f80b184106f373adf5619fa1f41ea291a9ba44ce9bae26aab525cfd9f57354").into(),
			hex!("6abbf5ac58e22b052c25abb5d217cdb908234542b334de9ceb6834fb2b139919")
				.unchecked_into(),
			hex!("0838727b3e584104d155e2e15f6da7c2c2c0bf11a2179e40bb68110a061d957e")
				.unchecked_into(),
			hex!("aa6229996193eae31993b82ed4862545180b9c330b93666de3b686ba6ee60f10")
				.unchecked_into(),
			hex!("46627ddbfb5120eb61d0316fc9ec69098a7685aa77c12f5834e7c072551e9406")
				.unchecked_into(),
			hex!("784f3f378d81b01881dfc5fcf95f1d219dece9448d3d6e91a9bbfceb8b50b24f")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03829b07f7b46d8db9ba4918476509c30c7da80c9ba8c38869a95261d12afa456a")
				.unchecked_into(),
		),
		(
			// Validator 12 (5CiAWLRtzhHnv6Ph18fx2Y7zDVG1iBAqrqVF62mTsTs1mhiS)
			hex!("1c91ca39e8505616a9aeb5d0d28f1e6568be595aae651787e174f0649007d064").into(),
			hex!("fc98829dfb66c93859d9017efb5a4632238f83cb94222d368509cc07d8b82436").into(),
			hex!("b2d15bc1154e28ed9bbbb275f8cdeacb0f13e4e33f5bdcce5058b1775b360721")
				.unchecked_into(),
			hex!("f0098d083904c8c1efe4d7b199dc7110db12307f7e60abbb66de5bf0c84724ee")
				.unchecked_into(),
			hex!("58624c5660eaff47a16620351bd059d5483c3ace7e405dc35dff7a784bf6000f")
				.unchecked_into(),
			hex!("7e6ba3d0f16b17032cedd63713cf177fd09168167c918eeb74a7ff3d28bede0d")
				.unchecked_into(),
			hex!("fa46331351a5cd3dbe87373e4b2a5d41f46fdc53219a1154f0c67b74b6dd1335")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("029c1bfe2148d5f0ecf0e2dc149077662b78dc94e579faf3c2d9699e1180e06ccf")
				.unchecked_into(),
		),
		(
			// Validator 13 (5C7tXxNJa9t1oKYVEJCPbpJRnsNjCsr3qUE5WSqRPSXNx1uY)
			hex!("026daded158eb1bae7c1221886940c95983408b14465a15aa4f33786fee43d35").into(),
			hex!("f4e5d2b699aaef7439a477ef3227c4f05a28ab3f39ddce28f98c55c6f5c5c66b").into(),
			hex!("5a6d7be31174b89f260943b8ed45a928bcce62c9b6d5f355d273abdc8069ab4f")
				.unchecked_into(),
			hex!("c4f2c9649ce5a084b197e67193bc38b68fc28631627e760e1e5b805ec139e317")
				.unchecked_into(),
			hex!("40e2be4475983ebfbd1ed424f9002552ed6a6e0e19f5bde62712301189dc8e0c")
				.unchecked_into(),
			hex!("30292935be8a010b3dbff50200286215435769467379f33ac9e94cebbf8f0b54")
				.unchecked_into(),
			hex!("0e98c0cefc0a9f7be9fbc1823afd4f503f7ac908271e0a430255ee989755ac2a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("026ac6737c2428dda1fcd9cd9e541ed4bff4b2f013376534ceffe95d167fb75afa")
				.unchecked_into(),
		),
		(
			// Validator 14 (5G9XFUFuZdUWuRmbfySNFYUi8tGWQCxm6J4q48TKy3bLJjbA)
			hex!("b49d7477dc29d3f424201014aa0b84df0f68602ea9a1347685f690208db9fc5f").into(),
			hex!("aa80f02af6101e1d3b50b8d93469044ef4da022edc7dd8947222a58ac0d4b06f").into(),
			hex!("d0ec11e184135fd4528e76bc5b86d88bf9fae12d5125efa58413bfcb533e3d4a")
				.unchecked_into(),
			hex!("e34637a8433f96f89f4553d8e4f154b5443ff51f5137bca0b03eb8d918d92283")
				.unchecked_into(),
			hex!("98437732a420d928b8533679dd8596396390598cbd760149a276cab6e9422b62")
				.unchecked_into(),
			hex!("64fd7e933a3a0927fd01ecac119b33c1c5d6873c08dd3af08ecbb24d777fd817")
				.unchecked_into(),
			hex!("d89c9f2ea837e4a16e56057aae5f1ea035ef5d654fc8bc22d981f8a03ce7f90c")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("025d447e3d09965586fd567a4d30ad8b99021ae51a0a50fc087fbae1568ce4e8d1")
				.unchecked_into(),
		),
		(
			// Validator 15 (5G9kpHYkZqnpoeyp3SWKsjzGnbxg5oAGUJTLYzoXgVrPZJ2K)
			hex!("b4cb1f1720f48a3508bbab350215b78601fc03280ca6b1f11a3ad2f821a5b54f").into(),
			hex!("80004defd22a75d59c88847829c6a4b60dc1538046f8df8a4177976c6c019d24").into(),
			hex!("0ede1a6894a14163cb1e1e6dae4f1598231185f890639c5e1d5c0cd1c16bc634")
				.unchecked_into(),
			hex!("acff1bd7594d8c0fff2fa2de3ca1c188473483d17f4a3e981b35b87c2829884d")
				.unchecked_into(),
			hex!("aed934f40402a52f6f4ab71b303495d0ad892eee11e2af124810964880bc9904")
				.unchecked_into(),
			hex!("6642d0ad3d40a806023ebba589f4e4a7e6487e848cea683026205213e643ab63")
				.unchecked_into(),
			hex!("ecfeeb7b27eed82fbfab85ca2a3a731f4b426bca8c8380c5f7ef417eeaef1e4e")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0318ed954173ae27d06b0ce26389104504a0e7c64a9eaa7373076b1d9d073afff5")
				.unchecked_into(),
		),
		(
			// Validator 16 (5GeP9GjAJwAArgzBnJxqAD2RfK33DnxHDsao7UKoBpsV7iRQ)
			hex!("caa04810a37d6b26ef7d6b9aef71ac0aaa81df9cf86653301f62ddf5bd04aa7c").into(),
			hex!("f219dd1fa4ee65d9c0050d7f0e3a93365d6fd6203cf8e41c6e4a4b28a7490d68").into(),
			hex!("c62091196508282fe3a5b87340cde8fabef4ad7b83e785ca323265a631997371")
				.unchecked_into(),
			hex!("4fd56c1d4149e6402cc4c799688977a075b663290e6c31d489ac5c20f4a7fd1b")
				.unchecked_into(),
			hex!("bca4b848dde3fc586a89b596b2bbbfffd58615b1d6a12b8959d0cc62a130c46c")
				.unchecked_into(),
			hex!("9aec7f6560e05603091cb3ee0da85968513782e863d6ad4d73bd449f575bbf53")
				.unchecked_into(),
			hex!("1212845868593d5cbe0066c339e242b9fbb6c7f33906c3e10ac7a68efed10359")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("02a4f727ab1e03b213d548480b36744924f2f3a0db358b744653ce02f7fc17f4d7")
				.unchecked_into(),
		),
		(
			// Validator 17 (5G8zDxQRUwAgF16wRbJuZF9QuvYLbLE6hNHemt3FaLvU2J7o)
			hex!("b435030148bcc75a24fd8176fc42b4357037e3868f0d7a2b04658426e83d8016").into(),
			hex!("3687427d498660cbddba994f0e77175ff1a2e20c575b90f7cfa9bf8f05e8e256").into(),
			hex!("42cedf5f1660768dfb962f0e3c545492190bd3ee945627c44b479e55db8fa505")
				.unchecked_into(),
			hex!("760937fd0768822e87545e455f23db891def08c6717d4b0f0795d8f8ef9a193a")
				.unchecked_into(),
			hex!("7e2265c5b170dae5314b5ff90e0988cfbc18a2584b8584fd1f1b5e4995eb5b58")
				.unchecked_into(),
			hex!("20610eb04240a7aaf3d30ecca6b5a34c0831959c2e805fbcbfd563df51d1825f")
				.unchecked_into(),
			hex!("3004f02199844d25da412c9b9b974eed1e6a92a4743e7975b7bce1b1f0ea1075")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0317057cff06fbd61c335573affb5e69272e26cea2f759609ced126f509b7a90eb")
				.unchecked_into(),
		),
		(
			// Validator 18 (5Gs1YvzVN2u95rjbFDxFdmF8BvpvWeWgEcGi4p46Mxca8G24)
			hex!("d441c94efc63d85b36e5e526cdf45658f9a1a49ef95d5fb11b3e3ac881c38e09").into(),
			hex!("9c6ff212c73649f545e9491e20ca490fa9224c97903d0c7cb33226c80577be65").into(),
			hex!("10f4c4027f076f8bdf791c7dcda923b16f286330c383e5d9bf3ad7e27952c76c")
				.unchecked_into(),
			hex!("195ba54d9259f7545c5d45106ffe22812c9195c14979e56af8c5b78e3d312517")
				.unchecked_into(),
			hex!("5a090e287d07a4a86cda83d79bfb8f4985512c0fa7a5a986b8d58fc3f7f69c10")
				.unchecked_into(),
			hex!("bc7172ba8e52cf1ed920eaf36a2d2b01d39014875855888f06a6a52ecf84db59")
				.unchecked_into(),
			hex!("76cfe3120858373d37d9a72987ae2f55be807fdf6b22f9e21a8d459a755f775b")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("030e94b123d24a133c6cca2be2ad957daa6c44ce9f911f885c574935bb3c8a38bd")
				.unchecked_into(),
		),
		(
			// Validator 19 (5Fjt81qbrvkskeFbCgUvQ3kyYA7BjKSsghXQS5VT98ctxsdN)
			hex!("a295d3287575815c83c1de28be46dd6b4c8ed528c84fd78fa2935da4d2dd6607").into(),
			hex!("bc8790fa81b4ff51333cd27fe65c05b11db0c3c07ba28ace6e3eef5ad80b361a").into(),
			hex!("6449c21eda1292f50b914b1591259effdcda7c1f53c79bd730439615baddf417")
				.unchecked_into(),
			hex!("8945dd3db7f5f4a50060f36b19453696b32ddc14fd1fae51be406c82faf10daf")
				.unchecked_into(),
			hex!("ce3b7f04034a5baba807a05315b8c2e7d16c936e677167d8d841f522ac88c344")
				.unchecked_into(),
			hex!("947befc8c09e84c6a2c6ab4c682c6456b41baea3eaacc8f8e21695d0f007432a")
				.unchecked_into(),
			hex!("b4b3285d4864ec20107e837a8f3eb78ba41ac560a910ac739df6307ee0034838")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("038bbd111c68e08d627a72ada0510b96f77bea64251706042fe54fffd56d96ed7d")
				.unchecked_into(),
		),
		(
			// Validator 20 (5D5YRetQTSwEZwvLP6fZSxKwYkMYWvjtPqmfgm48suSjHQZt)
			hex!("2cdfbaf5a4afd2b5c2bf2d7d21c6a00cc4e903b65c206f26a106b698f4c07a07").into(),
			hex!("d03ee9cb5da7ddd86d2321b6ed1dbac11e408ff17528d104a7db0d390461f401").into(),
			hex!("68be26ce820ab7ee86fe0ba785b724fd5cb9a92e68d05835f258364e7055e755")
				.unchecked_into(),
			hex!("ff9da3dbf0f9439f5c81c7339d6b79bc13431905567e5028a59dd6368e0ef4f3")
				.unchecked_into(),
			hex!("e857835ddc43c689b845bd57d2a8619d157f62477acc7580273c00c8cc974b5d")
				.unchecked_into(),
			hex!("ba511d5ae31c60e4c608159bef8929706ccd48fc486188b64a1771076742de34")
				.unchecked_into(),
			hex!("2cd0e8e2164f1413a0eeca9a96c9813c38cbe6dfcd20988d1fa9b939251f0c3a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03739cb6d83ed03341d8338a2f0f8fce86586476e19b3d3d2caa338c80b1187c2c")
				.unchecked_into(),
		),
		(
			// Validator 21 (5CcXoQJCfPZuTFq2iZG1RimSPs2xJHRMcjApTZHJphBMN5gU)
			hex!("18460017f648fc7548271f44bf430bcf34a13c5d11c2c7973e5872fc7f1bb679").into(),
			hex!("9a9b9e4b108f631322eb613e25e64409087a8682f336d050bc0a34652cc2ac18").into(),
			hex!("400fcd6d5171de43ecbe0a0b4b530978ab7c283dbd38e3087297d8ff96034676")
				.unchecked_into(),
			hex!("4522f9fd030ecb6aab0acae832ccc018ced12f76cb2141d13756a76cda9e782b")
				.unchecked_into(),
			hex!("5a11608882d5abf4da33b95a3e83c221b18f6f4b3356953a35797df37a3a3135")
				.unchecked_into(),
			hex!("bad5227c4d731912be70f928c07be8ab93ea7b3782e9b2d85b524a309f914c3a")
				.unchecked_into(),
			hex!("28abcf9d40a897ae787092d25936f551ad7a99c755bd7711b2d7cb0a6fe26b1d")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("02701e840856e9a3a31f48fa914caf21b26296a57be0995613934cca69aecc2167")
				.unchecked_into(),
		),
		(
			// Validator 22 (5GndGGg7A1G1tVLjbZcY9M5KCCLpp7UfW4JUNjFnE6Fh47dX)
			hex!("d0e9c77756f775aba3829920be3bc3ba9dcc28122d48a77830ffd36b73a86a2e").into(),
			hex!("24c6e8ea7f472b2ece933aefca0c21fb171cea641f94b18e504c1bd0a8242841").into(),
			hex!("3a212dbe426c50c28563e89cefb327f2103839aa3c617cb9c1dafbb694434336")
				.unchecked_into(),
			hex!("f66e70d27c86cba76ff5d56eaaee348ae502ecf2b4d4e6ada6e00c396bea3d70")
				.unchecked_into(),
			hex!("e6c606c8d297d081d70a9493d4a3b194d3d6f65b79cee27eff8f65b0f189b42c")
				.unchecked_into(),
			hex!("1e5f769bf83f49db46c6fc1b8bd23e1645c8896823904d501095390c545ffd2e")
				.unchecked_into(),
			hex!("a4e90c7c008ca7562d2291c33b0de67fe52ebe19d8c8ac819f774b0ac9908c27")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("023aa5bce5d92aeddcb1eac9cd9f9bafd92ef414690885059918b6ca7ff6a7bd34")
				.unchecked_into(),
		),
		(
			// Validator 23 (5EHCp5x4QPDUMgGp5aWZDJ4JvEKiwS4PJZJUa4Zodc7xHQBB)
			hex!("6200e272fd9ddab04fbd37565746e342aa4517d432bdb81b2f74d325b5745508").into(),
			hex!("d678190cc3b34e308747b089cdf0f7053cc0dfc99bac35aec816736f83fec95a").into(),
			hex!("5a11261a222af6aec3f81a29ceef09da1b17de2b61eb2ae84389d305a8946105")
				.unchecked_into(),
			hex!("fa834deafeda0f9a003352ce5ec8f8a5ff32e40cbea61f07684db2b65c346972")
				.unchecked_into(),
			hex!("dae803b1f1a67479df29cb745e5bfe60d12fcf454f4f130879776482a7dd2e25")
				.unchecked_into(),
			hex!("84443379217d98e6b6bb03788f33c405d2b6eb9070ab4f25baadb5c9c6571359")
				.unchecked_into(),
			hex!("08590b42eb0eb037e72440ef201aa082efb5b109b6fde669b8313b00371b8b29")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0246abdc38b3f6d7a4aaf68b00e7d950ef97845b38c0728f93e487d03c62d8cfdb")
				.unchecked_into(),
		),
		(
			// Validator 24 (5CnqLUZLZ4khcGx6FmWPuqMBRMC8GAkF6EWjNYT5MS2jFcTs)
			hex!("20217c2934ce0f4485d9b293a432a4be12deaeaf24f5899bea9738907c885a29").into(),
			hex!("deacd8b058301a08741a7d7e9b6a60c9804f85ceea8d9467915b9ebc9c00e61c").into(),
			hex!("b8df2a35d30e51fdd42e7f061a7b4134646dac52cf43fb545736c608b3ce9107")
				.unchecked_into(),
			hex!("b3f1ed06023a52a4fc487fdd9a18c53e3825615abcca9e98d008041692aa5478")
				.unchecked_into(),
			hex!("f4162c7492492f916301b1bb6403d712255ce6a69e1e621d014e3a13cf599411")
				.unchecked_into(),
			hex!("1010fc5f5784079a6150f136fe1bc305c8c6069972ffcef3a7d27e95778a376a")
				.unchecked_into(),
			hex!("5cc2d80cf13fd5288863b4b804c7461baf33ce5afbc239b0efb59b60bfaeee22")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("030040859d93f8a4d68f24817b2818b10e75f10a0315d74558de9e1865181b2bab")
				.unchecked_into(),
		),
		(
			// Validator 25 (5FnxvFT2huTXQRLEBY83ebSJc6Lf8CxQoUf1fDRpRNphEqhv)
			hex!("a4efb5ae7e9cefcd231426b093768464569455d37be76844410a410c349d6a1d").into(),
			hex!("66185d7af5c44b626148b0351779a628890458be2598d9610e52ac45a690d001").into(),
			hex!("90be0c600d616b2ed9c1452c030cde1c1ea1718b838ede8d12c57f031a2f1b39")
				.unchecked_into(),
			hex!("831480deae22b5bb5e27b63e1b3a2e7173514eff45c0e15bd8ee89d2a8650c40")
				.unchecked_into(),
			hex!("2ab861fd8f367c31811e7704f2c0ca0196ff6acf9dd980cbaa4db65c26b93d2d")
				.unchecked_into(),
			hex!("f8d555b8389fc220bbbb32c205da81022dab26b49595c8f2bff49810db821a55")
				.unchecked_into(),
			hex!("3ca257743f819ac2a53b7558f1cea4e16289aa0337b41e9301283576c8c40529")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("029be165582ddde0035b0a8aa563b9b5623b0355911ea8955c4d3c193fa28d1469")
				.unchecked_into(),
		),
		(
			// Validator 26 (5FWoqakZ1Bc4s1X5QhM72wWhx4tUiXWU6D1tkQDKAjb9sXNv)
			hex!("989d373435593082d15eb02a1bca8cb57a26bcad3559193f2198599125240e12").into(),
			hex!("7812912efb97dd1ea1cb90834a4af0e4483367aabc1b1c62891c56a5c790c37a").into(),
			hex!("8656327e42063985a83aa49ec0efc452ee731d9deba73c8f30e952c0f582ce68")
				.unchecked_into(),
			hex!("b3f867c6683c4ac8b7b1371d4ca433811ed570f616ac5c4e23b566430d4354b6")
				.unchecked_into(),
			hex!("20f9153561e06ea1cf4ec3d218f3645b8614236908e957a8f4312c1904434c0c")
				.unchecked_into(),
			hex!("5c01a8b6df642859aea3c5e2d3637220fd7a260c628bc99d0b322f310a145807")
				.unchecked_into(),
			hex!("ba98d264b1343a5cff97874f2ffe5c646dfc3a21b6dc4400185f4a505326d063")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03342aecc1ad046337cb74068f3f2be4d206f9a86dcce81c9e2ff0c54af044eb97")
				.unchecked_into(),
		),
		(
			// Validator 27 (5GHk3DkZaK63UQ4UGGQDJHYDuXV6DdeKVRbQsMybaQrirabD)
			hex!("bae27816cf59abdb824df0992f28426bfcab1a51a7c4a1f2877d07b6377cc444").into(),
			hex!("4293ca7e260b3b80542a72da4f217eb0eb2d7d034c4adc959051e42cbab94b61").into(),
			hex!("08ba314aaceeff4cc0032059e5ea7fd27cff964187d28c542bfd77a5e082295e")
				.unchecked_into(),
			hex!("7dac098ecc0238fdc49b7e79dea31bb3fd463ab8f53e2b01ad380338cadfdae7")
				.unchecked_into(),
			hex!("cad85b99eb2edd0b3d6eb30726051546493959e9da895fb6cca5c9616ea76655")
				.unchecked_into(),
			hex!("1479df003057a04e99fec1419ffecf96e5fb0f14aaa5f78c44766fc750b7d124")
				.unchecked_into(),
			hex!("5818af2b0f166c3ca04554cb6894cc67535e0dbf41aa56b48bec8321062c671e")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0285ccd8c975fa89d5945af97b847308199929b4ef7e55f88911e2ff11ae8e6f40")
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
	let checking_account_seed: u128 =
		HEZ_AIRDROP_ALLOCATION + HEZ_PRESALE_ALLOCATION + HEZ_TREASURY_ALLOCATION
			- pezkuwichain_runtime_constants::currency::HEZ_VALIDATOR_FUNDING;
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
						- pezkuwichain_runtime_constants::currency::HEZ_SUDO_FUNDING,
				),
				// Root's fee budget. Carved out of the founder's share, not added to it, so the
				// genesis total is untouched -- the same shape as `HEZ_VALIDATOR_FUNDING` coming
				// out of the treasury's. Without it the first `sudo` call cannot pay its fee and
				// the chain launches ungovernable.
				(sudo_account.clone(), pezkuwichain_runtime_constants::currency::HEZ_SUDO_FUNDING,),
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
// 2 validators with derivable seeds (for local keystore insertion)
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

	// 2 validators — real mainnet Validator_01 and Validator_02 keys
	// Seed phrases stored offline in secure wallet storage
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
			hex!("5872d64bd3a67b8c8272c9b3eaaa4853333509076f09942d0560236756e8cb37")
				.unchecked_into(),
			hex!("40316a0bf1f562ad339556975360bcd97dc688b53b93c230f08f5ddab4dd79e2")
				.unchecked_into(),
			hex!("eaa8333a15b4d18ca40aeb5e53154f56acb8d5ce024129ee17904f5140d2464c")
				.unchecked_into(),
			hex!("7a30d53d07680b55bffc423d0dd75cf4c0d86e82d0c6ee3320c07e7c389c746b")
				.unchecked_into(),
			hex!("f8784e05c19f61e58eaeaa780c9315ec15e9e794110fb0f511e89baec1e38c7a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03559f13b26109776b0cd0074b6612cf1b9f0e2198af6f3d66c2a4ec6ddcf39407")
				.unchecked_into(),
		),
		(
			// Validator 02 (5E5239nsL71Qb2MUmPwPtUM8pLb1Md4KkNBajmJ6wAHQwZP1)
			hex!("58b5ab34bfef86ccd2fda24b4b4034d8e77f03e24b19c810b1d3025456036806").into(),
			hex!("9cb78ac63399b9257d8ccb5a4fd0160e251f971b0ca5d4a8fb98d196e4b7721a").into(),
			hex!("d874b6f6f9bf67d55ec69838129744fec8237fd8e77c2828ce9460a20916e42e")
				.unchecked_into(),
			hex!("e8f1ba8114c0eb72f540c7be75d675effbc87318154da4b53ca03f849960669a")
				.unchecked_into(),
			hex!("bce8e0618ad82b86e5fc775363fbbd5c230657fc431501e2c7229f289e477c62")
				.unchecked_into(),
			hex!("887b1a1d2d1155c16dcf64b070684a8a981b7ab99077ad66848739cc8f87c952")
				.unchecked_into(),
			hex!("764f5241852f479240812f969cd1393242cb318b8f2f9cee800dafef6803983a")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("024d226f333f7d0a9f82bd3eba49e4585139f84934cd71dc052412768cd62a811f")
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
		// Mainnet's root is its own key now, not the founder's. A simulation that keeps
		// the old shape rehearses a chain that no longer exists.
		// SS58: 5D4o1HMKEntLafi1f2tz4U9XVZpg1gz5YAR6mgcgoPT4jgzU
		sudo: SudoConfig {
			key: Some(
				hex!("2c4d909d9cba926dcf9cad71a154cc64adbbb0e8066b353a10e062707199c97c").into(),
			),
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

/// The genesis seats the twenty-seven validators the fleet is built for.
///
/// The authority block is generated (`res/genesis/mainnet/emit_relay_authorities.py`) and the
/// generator takes a `--count`. Run it with the wrong one and the chain launches with a
/// smaller set than the machines standing behind it -- which is not a shortfall that shows up
/// as an error. The relay has no staking pallet; the genesis authority list *is* the set. Seat
/// four and start twenty-seven and the extra nodes sync without ever being asked to sign; seat
/// twenty-seven and start four and GRANDPA never finalises, because the threshold is two
/// thirds. Both look like a healthy chain from the logs of the node you happen to be reading.
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
