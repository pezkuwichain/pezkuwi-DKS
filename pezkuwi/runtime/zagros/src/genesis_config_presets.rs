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
use pezsp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use pezsp_consensus_babe::AuthorityId as BabeId;
use pezsp_consensus_beefy::ecdsa_crypto::AuthorityId as BeefyId;
use pezsp_consensus_grandpa::AuthorityId as GrandpaId;
use pezsp_core::{crypto::get_public_from_string_or_panic, sr25519};
use pezsp_genesis_builder::PresetId;
use pezsp_keyring::Sr25519Keyring;
use zagros_runtime_constants::currency::UNITS as HEZ;

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

/// Kurdistan Treasury allocation: 20% = 40,000,000 HEZ
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
/// The two that stay are the two with an owner: the founder's is property, and the treasury's
/// is the relay's own. The two that moved are the two that answer to a body rather than to a
/// key, and the pots that hold them live on the Asset Hub.
#[test]
fn hez_allocations_sum_to_200m() {
	let here = HEZ_FOUNDER_ALLOCATION + HEZ_TREASURY_ALLOCATION;
	let on_asset_hub = HEZ_AIRDROP_ALLOCATION + HEZ_PRESALE_ALLOCATION;
	assert_eq!(here, 60_000_000 * HEZ, "the relay mints 60M: founder and treasury");
	assert_eq!(on_asset_hub, 140_000_000 * HEZ, "the Asset Hub mints the airdrop and presale pots");
	assert_eq!(here + on_asset_hub, 200_000_000 * HEZ, "HEZ total supply must equal 200M");
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

	// Owned balances: the founder's and the treasury's, with the validator stashes taken out
	// of the treasury's rather than added beside it.
	let owned = HEZ_FOUNDER_ALLOCATION + HEZ_TREASURY_ALLOCATION;
	// Escrow: the mirror of what the Asset Hub holds, so a teleport back has something to
	// release. Not new supply -- the same HEZ, represented there and held here.
	let escrow = HEZ_AIRDROP_ALLOCATION + HEZ_PRESALE_ALLOCATION;

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
	// ZAGROS ACCOUNTS - generated 2026-08-31, keys held with the Zagros wallet set
	//
	// Every address in this function used to be mainnet's: the same twenty-one validators,
	// the same founder, the same treasury, on both chains. A testnet exists to be broken
	// into, so a key that leaked here was a key to mainnet funds -- and a testnet whose
	// validators are mainnet's cannot be restarted from scratch without touching mainnet's
	// keystores, which is the one thing a testnet has to allow.
	// ==========================================================================

	// Founder account - receives 10% (20M HEZ)
	// SS58: 5Fhjq3KmYHgChQ7mfaRGz3hotzC1XTSsGXK8HChaid5sUrNS
	let founder_account: AccountId =
		hex!("a0f36b1ed6006a5ed8e492a1a5c5820cec6cb6feba17282f0bd41faacc1f8c12").into();

	// Kurdistan Treasury account - receives 20% (40M HEZ)
	// SS58: 5E5Go6imnF68WRN7pmHyKo3vVZmg71YCMaUbMkHeJdTYzWfY
	//
	// The presale's 100M and the airdrop's 40M have no account here at all: both are keyless
	// pots on the Asset Hub, one released by Parliament and one by the President and the
	// Prime Minister together.
	let treasury_account: AccountId =
		hex!("58e758ff62a1ca0ce950986596aa24607826d506b685351cc3c04abd7a07614a").into();

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
	// INITIAL VALIDATORS - four
	//
	// Four, not the twenty-one Pezkuwichain seats, and the number is decided here rather
	// than by starting fewer nodes. This relay has no staking pallet -- elections live on
	// the Asset Hub -- so the genesis authority set is exactly what `SessionConfig` lists.
	// Seat twenty-one and run four and GRANDPA never finalises: four is under a fifth of
	// the set and the threshold is two thirds.
	//
	// Four rather than two because two makes the threshold two: a single restart stops
	// finality, so there is no maintenance window. At four the threshold is three and one
	// node can go down.
	//
	// Generated by `emit_relay_authorities.py --count 4` from the Zagros wallet set, which
	// holds twenty-one; the remaining seventeen are spare rather than missing.
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
			// Validator 01 (5Gbb57WXHVrtVG6ozytVjAm9BCg1sNdBLLmtR1UmVcTm2UJB)
			hex!("c87eb313036e491d0b5c7b0777e89e84b1259191f434977acc47270c76754803").into(),
			hex!("52407c5c2f729c9637479999bacedf5ecde25e6f1f210c784964fdecc98b3850").into(),
			hex!("746f4eee0a4272db06bba47fdd9ed400bd9d33d2201fdc42b72ebab9a8ab0b2b")
				.unchecked_into(),
			hex!("8d999d5795b1b925ae8c8ea8e3c3f5d0b83cdcd48a828587b8a12db561f23841")
				.unchecked_into(),
			hex!("700fd76572b1fca1efba093b3b58e2d2feca4240ac38156b8273ad18f71d644a")
				.unchecked_into(),
			hex!("a04f2696209fc3d68541873eeb5ea5c9481966a8ae0917f770f2a81086fd0a2d")
				.unchecked_into(),
			hex!("e64c61be956d9535ec0db95a7cf9d7bdf90f5430fd9745a2271157a0b1368d7d")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("038dc15e2d91f12e2da4fcbc9f6f50df286443c3b134fa07154f24adc4a35add79")
				.unchecked_into(),
		),
		(
			// Validator 02 (5ED8FZU9u2xDYqdT2gX1p9QM96ivmCPTRMmNXT3Z9jSGkmQR)
			hex!("5ee48ccb625f4bbc567a535fc9afcda4609df861a3320d1910d99ec90e97eb65").into(),
			hex!("eaec3a7618960ab351c247c4d7a1b66f1e6481ddfe4c4a31e5fd71ef7e61917c").into(),
			hex!("58d1961601286709f5881ad74470979da6fb46bfced790f867699406a8fc6a6b")
				.unchecked_into(),
			hex!("bdc3feef9c3c9f6691581db9ba742c2ab1d32a7a7a02443d51bf1cf46e94fdd1")
				.unchecked_into(),
			hex!("c460392d5a5f645c0cfe7ccb18d92eb73cff220d6f4c1895493759afd4a30a6a")
				.unchecked_into(),
			hex!("966c1394a7ec07e7737b1645f29ec1228ac90ec7455e8ea936e014ee96dda834")
				.unchecked_into(),
			hex!("6a269400ea93792c187348da0aa39fd067f4114d221b31af8f6230be5bebc717")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03aba644665a0e35f7ec9fa6b710bacda721c818d2cefab7004b0e22137c9e747c")
				.unchecked_into(),
		),
		(
			// Validator 03 (5Dkd9PhMvuvYGSQmfBrqeH2F8FVnmaQ1MW8pM5Fe5MmhnjBU)
			hex!("4aae3051a6d781b8e25f9cace9488aacb31afc2a9e2a03120f52767bb0e8a908").into(),
			hex!("4afeeb5b2f7b7b5bf6963a97d451729327fcd02c816cb04939c36471758d9557").into(),
			hex!("6eef96d024a9c8a37ad485289e405882bbdfeb5c05c7be90aa8d3f7a67728b34")
				.unchecked_into(),
			hex!("97c9acdf6049bbbe3469cbef37ff8e299fe2e9d22eaeb2f09998f3f805540767")
				.unchecked_into(),
			hex!("002edd41eac61588ad360ba4fd6f7b4631f38e17a2acac0487c9ea689437456a")
				.unchecked_into(),
			hex!("62ab60c4087087a260ffd2cd0c74d4d2da33e9331e9357962298e7cdfaf4747a")
				.unchecked_into(),
			hex!("806bde0f35c591028a1e5ff2924915637460d02080cb2fdc19296a28fd2a0c2d")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("03b9664d708ddced935acf5b8b4d67a4dae68b372ada17589e464f5b8af5a3c156")
				.unchecked_into(),
		),
		(
			// Validator 04 (5HYNdbYjhvZaFUvtZK2q8MqQL1F46jccH1eRL5TXnemNaJZQ)
			hex!("f24751f6eeb86f2636cbf9679fcb8aacc6cf5e266c7f472505c9173fdc3b5458").into(),
			hex!("244b0b47c1b627b28a05531cb5d76539bb5a0bc6c9f0c51cd6a22403cdb38e6d").into(),
			hex!("d6e5a2fad0c8fe7bf656bafceee4e5019fda7d7bce94604a605cdf6a21634e51")
				.unchecked_into(),
			hex!("a47126f0b1da217ba52f9d7b039d489f55c16c7d465068eefa13881eb4b8dd78")
				.unchecked_into(),
			hex!("9492aec0d4e38e920ebed4d274df6fe026dceab04b6652b8536dc51db299d223")
				.unchecked_into(),
			hex!("6e218a3c2e5d2140174996a7046c1ca9dbb58829cd2c268e6a07108fcd0cb713")
				.unchecked_into(),
			hex!("c6c2ba4c04c5157169d3e6e5c4575222cbc71b3c8a3136c469c9faf72a76a139")
				.unchecked_into(),
			// BEEFY is ecdsa: 33 compressed bytes, not an AccountId
			hex!("0337f726c06697901ca27f0e6ba2b17ad3174dd19210589e7741937890329b31a5")
				.unchecked_into(),
		),
	]);

	// Validator stash amount
	const STASH: u128 = 100 * HEZ;

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
	let validator_funding: u128 = initial_authorities.len() as u128 * STASH * 2;

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
	let checking_account_seed: u128 = HEZ_AIRDROP_ALLOCATION + HEZ_PRESALE_ALLOCATION;
	let checking_account: AccountId = crate::XcmPallet::check_account();

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: vec![
				// HEZ Genesis Distribution (200M Total)
				// The airdrop's 40M is not here. It is minted straight into the Asset Hub's
				// airdrop pot, a keyless treasury instance -- so no key ever holds it and no
				// manual transfer has to be remembered after launch. See
				// `HEZ_AIRDROP_ALLOCATION`'s comment and the Asset Hub's `AirdropPot`.
				(founder_account.clone(), HEZ_FOUNDER_ALLOCATION), // 10% = 20M HEZ
				// 20% = 40M HEZ, less what the validator stashes take out of it.
				(treasury_account.clone(), HEZ_TREASURY_ALLOCATION - validator_funding),
				// Escrow for what the Asset Hub holds -- see `checking_account_seed`.
				(checking_account, checking_account_seed),
			]
			.into_iter()
			// Add validator stash balances (STASH * 2 to cover bond + existential deposit)
			.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH * 2)))
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
		// Zagros's own root key: 5HpEXEim6dogBSXRmWEzXXNjFeS97EYRVsfSpyfGBhGfpn4x
		//
		// This was Alice, and the reasoning was that every developer already holds her key and
		// a testnet has nothing to protect from them. The second half of that sentence is why
		// it was wrong: every developer holds her key, and so does everyone else. On a network
		// anyone can reach, Alice as root means anyone can upgrade the runtime, mint, or halt
		// the chain. Upstream draws the line in the same place -- its public testnet's sudo is
		// a real address, and Alice belongs to `dev` and `local`, which keep her below.
		//
		// The rehearsal argument is the stronger one. Mainnet's root is a key somebody guards,
		// and retiring sudo is a step this chain exists to practise; neither can be practised
		// with a key the world has. Held with the rest of the Zagros wallet set.
		sudo: SudoConfig {
			key: Some(
				hex!("fe5ff27956998b38004d1c49eb4ef1f1cd8d11bd4c89d3a8c12c00aa6fd5ee15").into()
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

	// Real founder account (sudo) — 5CyuFfbF95rzBxru7c9yEsX4XmQXUxpLUcbj9RLg9K1cGiiF
	let founder_account: AccountId =
		hex!("28925ed8b4c0c95402b31563251fd318414351114b1c7797ee788666d27d6305").into();

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
			// Validator 01 (5GipBJs2uNWTCazyZQ2vG3DEqLz4tXNmNZtBAT1Mtm1orZ5i)
			hex!("ce0189f16649560a8e250ee51233b97f20b528d9f534c54b40da5e1b785fb422").into(),
			hex!("781f2da4ec1f954ddbd96365b93d5b991427980475e10dd9f823979665399137").into(),
			hex!("e63ad8e22976bc2bdbc9776b3d104472ff70cfcd6a5247a2f62efdb09f66520f")
				.unchecked_into(),
			hex!("9497e1dabb5b7688da148813629076596c77eb47f0a18c971777c70bb38cd30d")
				.unchecked_into(),
			hex!("5e365f9c23e9fd65f28b63bd118f46faca2f82d286d00ac23ddb69fdd61b342f")
				.unchecked_into(),
			hex!("a854fce593b83d3a97ac4b0dc3ef220f69134753894cb16f28c67ae12db00419")
				.unchecked_into(),
			hex!("4859a231daa597501f616c189699afa576ec79b704f633267c5b940dc76a895d")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("02b97d26cb0553d662c52006fd6215736d0138d5dda92661422951a41dfa9d8f3a")
				.unchecked_into(),
		),
		(
			// Validator 02 (5HWFZbhkZuTUySXu6ZXYKrTHBnWXHvWRKLozE22zhnwXGGxk)
			hex!("f0a90883d86793bce27217a0070f61d66efe56033c876624ffa3468698175058").into(),
			hex!("86384da0a3d7dc41b1d2837c824f022dd34196d0e3ba40075934d4c216b5ea0f").into(),
			hex!("bc79edcffd121970d471b6811b167b21bb8aa158d5ce9143fd0d45f71aa4ba1a")
				.unchecked_into(),
			hex!("1b453491a1ad16feb2e4cc5b4bf85f21a54fbfaa9321e9dbd9b668b83355146c")
				.unchecked_into(),
			hex!("2ad0684fe19374a4c1ed49f92226cb1af5bb9977d6395de879c556ada080e759")
				.unchecked_into(),
			hex!("ee3de83cc3deaadb3e1159e1de5a677a47bd828d3899bf7579753293389d0655")
				.unchecked_into(),
			hex!("5eea9bf553a04467d3dafe9a5ed196410cffb96248519ab5a491c09fb5b68c2b")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("031a58225fbca7430f406dfa8917517f81284cc991f7b9e9f8f7d37f24a85869f7")
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
		sudo: SudoConfig { key: Some(founder_account) },
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
