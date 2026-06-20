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
	SessionConfig, SessionKeys, SudoConfig, BABE_GENESIS_EPOCH_CONFIG,
};
#[cfg(not(feature = "std"))]
use alloc::format;
use alloc::{vec, vec::Vec};
use pezframe_support::build_struct_json_patch;
use pezkuwi_primitives::{AccountId, AssignmentId, SchedulerParams, ValidatorId};
use pezkuwichain_runtime_constants::currency::UNITS as TYR;
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
pub const HEZ_FOUNDER_ALLOCATION: u128 = 20_000_000 * TYR;

/// Presale allocation: 50% = 100,000,000 HEZ
pub const HEZ_PRESALE_ALLOCATION: u128 = 100_000_000 * TYR;

/// Kurdistan Treasury allocation: 20% = 40,000,000 HEZ
pub const HEZ_TREASURY_ALLOCATION: u128 = 40_000_000 * TYR;

/// Airdrop allocation: 20% = 40,000,000 HEZ
pub const HEZ_AIRDROP_ALLOCATION: u128 = 40_000_000 * TYR;

// ===========================================================================
// COMPILE-TIME VALIDATION: Ensure allocations sum to 200M genesis supply
// ===========================================================================
const _: () = assert!(
	HEZ_FOUNDER_ALLOCATION
		+ HEZ_PRESALE_ALLOCATION
		+ HEZ_TREASURY_ALLOCATION
		+ HEZ_AIRDROP_ALLOCATION
		== 200_000_000 * TYR,
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
			// System teyrchains için 2 core gerekli (Asset Hub + People Chain)
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

#[test]
fn hez_allocations_sum_to_200m() {
	// Runtime validation that allocations sum to 200M
	let total = HEZ_FOUNDER_ALLOCATION
		+ HEZ_PRESALE_ALLOCATION
		+ HEZ_TREASURY_ALLOCATION
		+ HEZ_AIRDROP_ALLOCATION;
	assert_eq!(total, 200_000_000 * TYR, "HEZ total supply must equal 200M");
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

	const ENDOWMENT: u128 = 1_000_000 * TYR;

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

	const ENDOWMENT: u128 = 1_000_000 * TYR;
	const STASH: u128 = 100 * TYR;

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
	// MAINNET ACCOUNTS - Generated 2026-01-29 (NEW SECURE WALLETS)
	// ==========================================================================

	// Founder account - receives 10% (20M HEZ)
	// Founder_Satoshi_Qazi_Muhammed
	// SS58: 5CyuFfbF95rzBxru7c9yEsX4XmQXUxpLUcbj9RLg9K1cGiiF
	let founder_account: AccountId =
		hex!("28925ed8b4c0c95402b31563251fd318414351114b1c7797ee788666d27d6305").into();

	// Presale account - receives 50% (100M HEZ)
	// Presale_1
	// SS58: 5Fs1VXbPVvmHAaQ8a7bKcdJ8h8c1mgJKLJ6Pwce69fSqhLJ5
	let presale_account: AccountId =
		hex!("a8055af9df1db60bea4277f7e91157246a6245123564bff10435f461f284bf55").into();

	// Kurdistan Treasury account - receives 20% (40M HEZ)
	// Treasury_1
	// SS58: 5EhCpn82QtdU53MF6PoNFrKHgSrsfcAxFTMwrn3JYf9dioQw
	let treasury_account: AccountId =
		hex!("744ed0812d6096827376b4625fe4f840d4950d5aef0ab12902e64c444c8e9d29").into();

	// Airdrop/Staking Rewards account - receives 20% (40M HEZ)
	// Staking_Rewards_Pool
	// SS58: 5EhNrCuXujRKefXrY76wkFMUHVNNpDZpGXvf2L1zTuEK1KsZ
	let airdrop_account: AccountId =
		hex!("74708f70ad76a617a8c90032bf69869b983660b37f917268c0934fd782b1e350").into();

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
		(
			// Validator 03 (5CrB5BWJfLNWEZAsAXDKXdJUGzFMXKvYnwRX4DVMcgBwxSdx)
			hex!("22ada8d8e51affa8aa9169628a87f251ad6b99d191a7cff0d6a2bc6355f9827d").into(),
			hex!("36f6b0c0e42774526e76c1b7f6f1f35e1cad7594dd7928ef125b85dbbdd2b420").into(),
			hex!("a233520afef4b7d268b69a2c67074938eb536cb3f77b825f78d61f2d16353571")
				.unchecked_into(),
			hex!("c22f1b4411879654538c11f088622d0b1ba695f9809e4ca5c79435e6f91f9fa3")
				.unchecked_into(),
			hex!("68b6a3c0aaa9264d1ac04cf8a698c0a82db09d1edab3c37b34e52b543b85393b")
				.unchecked_into(),
			hex!("ec31a2d91f9e55ede3239ee11eea5fe3bcf8ada2e0a5502b9fb84ef09466e47c")
				.unchecked_into(),
			hex!("3c6035a77aa676b10959ba408bf4ad6cd37966fca825a8c5c444627f40b61b10")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("03d5503ae0eb6fc5c7f368e2e43a1189ff0129de6c1cdd5d5090c8283df0dd43e6")
				.unchecked_into(),
		),
		(
			// Validator 04 (5ELgySrX5ZyK7EWXjj6bAedyTCcTNWDANbiiipsT5gnpoCEp)
			hex!("64a96ba3228df496787dcf68c7456ab1a8ad8381baa64e3d917fedce90debc13").into(),
			hex!("b2a232c9e62f3a143b053129714fdc96aafea4aa6e2323cbe07a88648043db19").into(),
			hex!("806af944dee83e7d61ee56d3d58703613e283f0946f4eb27ae527e5548dde80f")
				.unchecked_into(),
			hex!("f51ef76f99bdee1137836fe9554d24302c060200b601a6ef778fefef9d0c7793")
				.unchecked_into(),
			hex!("c6563f0e1f657d47cd84aa877b5b9ad507eca878f0d104767add32dc0b709f7a")
				.unchecked_into(),
			hex!("a61fed1300423685e286cbcb5ff993a8300614051117f16340f4b3ceceffe570")
				.unchecked_into(),
			hex!("f2bae1ba3625520d9eee4c3c3e35415dae8bf818c1cd218cde37d24477152748")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("031350bb37bb741b06b0b966168a0b8c75e1787886bcae1abb0d22ed947b7937c6")
				.unchecked_into(),
		),
		(
			// Validator 05 (5GCZQNjRdHofEHPvVq4ePrfDYcjRzQ1HQ2awHMX6AawpRYuM)
			hex!("b6ee70cad1183361924ac3fbd237d9e398a8dcbee56340a3d0e431ed16af4c3c").into(),
			hex!("fceb174bd5553d61f5059b3017622f4b8f6600ae9b95df524704cf11ea2c5b2f").into(),
			hex!("705b39924a158ea2257f0e58b04ead3886e689690cd7965bf5e392b3e06d835b")
				.unchecked_into(),
			hex!("7f228a7bb20f0a2e0843e92713ea02bf253eb235fc24d24a2473ebaf461a014d")
				.unchecked_into(),
			hex!("7c0bbc0078052b82102dc15c1c031f4af70dc771274203acac13ad07c8022309")
				.unchecked_into(),
			hex!("c8f2141d2bb0d307ad7871440699c31efa4d560aa0c28515979f4377b4e57c40")
				.unchecked_into(),
			hex!("eaf152cfc6d9c2ade7951c8d009f70942c85c1df734cc6769b162e5c0ff6bc5f")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("02a306f71752509dce6386f04b914cc957949f627a90c4ef4e1773febef96c2f0c")
				.unchecked_into(),
		),
		(
			// Validator 06 (5H8jTzi4Gm4rbFtXw6h5enhLhgsuhNAqR5K2itmPiz83ymWy)
			hex!("e03f90c7f34fc73016c71523c698d8bd869c960f4e401575fe93ba90f51b1d7f").into(),
			hex!("c409a6d1e6d034a05ded26b3fa8f0d7ad54648926c438f0f0927238edd7f8758").into(),
			hex!("c416f06351d396128eb3f5b3608007e15d72c3c9bb670434522f4591af928276")
				.unchecked_into(),
			hex!("1667de8cd9a2da709f9efd662dc8e983d936b85eb6cd89c49795f9a0c4560128")
				.unchecked_into(),
			hex!("cc04b11029a9fe6164735018b74d74e8d1ef6b8e7d4a9d9e73cee0abe545ed4f")
				.unchecked_into(),
			hex!("46b29a24719e7f39698dab9f38f382f2cf33d880bb31ab84442b26ff2f913034")
				.unchecked_into(),
			hex!("566dc53c011abee57f7085c4938f22a5a488e8f633391f353669390f9b659137")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("03b08173722e8bd87ee7a5d362ff4bc5d98d17fe1acfb4181e224085e71b3d89c1")
				.unchecked_into(),
		),
		(
			// Validator 07 (5Fs3P5tHuL9cvwPQojsheViRRAjFkMMFa32jAkDSwW9mbTfU)
			hex!("a80bb6c971fd1746c40e2d7602c0b5c0c0caa4a3b65091da35d35815cd63a453").into(),
			hex!("c636188c6d24698cace1b83add20206842843a124279f99863ef2c68c0f08236").into(),
			hex!("549a6a2050c088c9425656f58c744fa0b43ee665c2880cea6c35d0ce49288515")
				.unchecked_into(),
			hex!("835670e0c86151c2cb4749cff8fbd71f122322d545091c361da7855987119bbf")
				.unchecked_into(),
			hex!("f8d63771080fc2c251048ba73ccce131cdc5ff83daaa1578e947c3e7a8116340")
				.unchecked_into(),
			hex!("3cb203eb8bea5a42cfd1f0154ae503add616314e6964ce27582ee01d4c97fa2d")
				.unchecked_into(),
			hex!("2ef9481ba1e76727b2a479860ef1fdc6ab3ab701712b6c258e73dc21f6b50367")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("028d8719a1a5147db239bacfe9cf0fabf239dc5639da038fe63d466c46f2aab5fe")
				.unchecked_into(),
		),
		(
			// Validator 08 (5DXgq7uDXog6zcubT3wgtaYosoibjudz4w5ScPW2phLuAy3V)
			hex!("40d067361a3d1f954d4258509d4f27b9bf3d6a3d0a3e2a5ca079ee896cab375f").into(),
			hex!("622fa71e37f6854293e1f766b580001f9a9793330064305b76f1fc232f93ca35").into(),
			hex!("fa4df66ef8c5dab3a135e680d1fad0dd40454c147da6b2ebb33e0abf199c9736")
				.unchecked_into(),
			hex!("d45cef628619987beda6cb2537930441c2cfa8a58cb523271912d98b02b06c24")
				.unchecked_into(),
			hex!("2e9845a40efff2c8f68492e90aa341b9e56fc2a17d0c3a108a73c6e22d283a31")
				.unchecked_into(),
			hex!("a6f75e66cdb1dbb9df9b28dd648b165a7486fb24497d646b8a4592d12cecf029")
				.unchecked_into(),
			hex!("6e7fdf190030d2f5a1a7af0e03f774c2f038e89a548d5ff44590d76d2b7a8402")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("02c0c80f02c56a9a67c5dbf443b7c53136d30f477356f99ef64e026997286bd4e5")
				.unchecked_into(),
		),
		(
			// Validator 09 (5FyFwbGLgPXun3azh6Gx83wCuUt5FTavb2WAVDYrjziVB9rN)
			hex!("acc977fd38e3d1347ab8973db0afb2b5a06b7e2b91ac94ac285b2b9513ccea2f").into(),
			hex!("be08aa2e9926a6affd87d8a2eef8a67826c6a132308231aaeeaa51cff39af33d").into(),
			hex!("4ee0b888b296eb13c5283f9ef739d154758dd2efa8c71cfed46b638cb8be9857")
				.unchecked_into(),
			hex!("2b6fe25b1b8b9111c8715339f3faa5509b3f222ec735f4fb3a4330bf4177001f")
				.unchecked_into(),
			hex!("504578b39414c45d565bbc19987337a8e180397af472308af23a97bae3e2bf38")
				.unchecked_into(),
			hex!("c0995d8c57bad9700fe8ef1e43451dfe47b2ae1a8db610da32273d1679a1ba1d")
				.unchecked_into(),
			hex!("bc5acf1e3df528b64d680227f560e104301727083b4586d7706cd52a77c69317")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("02cb75ef6f6d501de0bd3222a6c619fbf6ca14733839f0470dbc2a66823f75a729")
				.unchecked_into(),
		),
		(
			// Validator 10 (5HEcuuypLDeJaSj6ZgH57aXhuviyeLNdw9QrCDJ8u6gsnjnL)
			hex!("e4bcfa69c15f955817e15325269e023d2ee7dd3db362794cc7a03e25d6dc5b47").into(),
			hex!("38c752203223f6e79585f47ae644bfbe8f9152c950eb79b2db9115eb7b05c118").into(),
			hex!("c2570f50468e73614cac294dcee7b228b969df53160c2a491dab9837aca3fd46")
				.unchecked_into(),
			hex!("add76170ef4ac4120824f85e0be728b5984b771512a91bc30c70a0194e973313")
				.unchecked_into(),
			hex!("dc2e8f2349da00d88fcccecd7b905179878223f1f4ca8e68f7d09eb2d8bfe92c")
				.unchecked_into(),
			hex!("a44329940eaad42d23d456c995f09c90c4f3eb91774c4037ccd3d9e4ddabd523")
				.unchecked_into(),
			hex!("e0e09485ba5823ac1752952f6f6506f409c0d0d2facdfc064a1956e13d5e0778")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("03d1256d431fdb43b5f10a3367334241d964fa24f75cbf617c4daca092d2807268")
				.unchecked_into(),
		),
		(
			// Validator 11 (5EpmpTXbMXpz6ixy3WhutdzcexzPbvybNKv4eiiN1kvTnQH5)
			hex!("7a149a024f1e0b8b6935829a8b966ad369fe9484df3a82104d9d1d1ee01ed572").into(),
			hex!("6cb70f3b6abf38b6624c9d66931237ce0e1402bbc2ed5d2697485d1f509cde3d").into(),
			hex!("727272a3bbf7a86a1cf83ae396f21497e768a0dab34774f7be95b86d2dfa4f41")
				.unchecked_into(),
			hex!("46b44ef1edfd35e48996a00dd1621439342370e551467e2e83d92b4b68cbae4b")
				.unchecked_into(),
			hex!("400de6b7b9cab9b0e99616ec1847251776fad2e3ebe5befad65037501711f022")
				.unchecked_into(),
			hex!("be473d7b91a725007cc4e415ef501435e84b4d75f7d02666399b9a38db3f5937")
				.unchecked_into(),
			hex!("ba2e8b94884e5537c30a4e20e98b5b7fe12fee836859006d96d37aff76484a69")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("02b1e027acb11655b805365858ee9ca98fadc29622a44e0682ea681731e6069db6")
				.unchecked_into(),
		),
		(
			// Validator 12 (5DFsm3BBEgHmSEZkvwGKB7c7tiH2avhfuQE1SEjfMDGuczsW)
			hex!("34c144e4a1dcf884c75ef9b9f08fca1d5f77b8220f4df61514322f0f591a0938").into(),
			hex!("661f3c412ba5a9d7800484ec5758630aea077fdae81705548a09160a54135334").into(),
			hex!("ea38b9f19a7ec2d0fd5561a6e8fa6bb5719c422a1e69905b460a3435da32df3f")
				.unchecked_into(),
			hex!("30b77c85132f3f03b65751c8931e47d32973bfca1a638cf50a060bc4313d8c95")
				.unchecked_into(),
			hex!("641c34d47e356e3819af577e0263351ef7238d91726be728402f61d9ca374840")
				.unchecked_into(),
			hex!("66e28f11b84ea0f3de439280e098a080b980fc81d0f25bdc51ca5e85c5085122")
				.unchecked_into(),
			hex!("da614c956a74ff0e791a5ce6e5b2acfcf6de251279034d58e40d47817e3eeb1c")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("0268ec971d851bc687b7792d8818e833b4f86296117de1e2bae220d8a54cb98668")
				.unchecked_into(),
		),
		(
			// Validator 13 (5HePVUXjGSM2hVZ1YMz2V3KoX6EdQNEmmzUnUvpfGV95ofUR)
			hex!("f6ddb003fed15c7ca1b266bd244e65a639a444b9e2bc715c0dd5434565c7eb07").into(),
			hex!("8a86427d6fa47d9a8444ac83373d23b475da9b5495ab1347bb0aec96976c8f03").into(),
			hex!("c8fb1829d4b387ce30ac97733d7e0f1b680434d40eb6ff5853cede0bc8fcf626")
				.unchecked_into(),
			hex!("628e5573c7741078d2ad579a4f029b42b196c7f1ef9acc52326fbca0e5c915ed")
				.unchecked_into(),
			hex!("069a61fd8aaff5a526632ef31c0aabdf9efdd3b8414cd637b4e51e568428dc52")
				.unchecked_into(),
			hex!("8053465ddf970e400e0ec2a190781bb9c5619c986512580dc84f4b61e8af2f3f")
				.unchecked_into(),
			hex!("de45dfac7a51d17898bb238dca449bf1c8f6fb95de547b2c788392e51e971f04")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("03a03589b0023447c56d789e6074430bb08498ee132dae418be9ec0f753c196acb")
				.unchecked_into(),
		),
		(
			// Validator 14 (5GP4nAcwtETTg1oAHQNvevmmhG8GEstGQeCirKEhaDTwpFgx)
			hex!("bef1c8373f3bf894c4de8f645e2d319302502bf3f53be8639e9f49b6e2994315").into(),
			hex!("369bb1432aabcdb9c867007ced947675221299435cc817e4f3716ff0b926d56e").into(),
			hex!("68d8f4c3d74ba60bad6dfcf8241556a940606d0bc5874611a3f2be0fe1b57d76")
				.unchecked_into(),
			hex!("3fbaa2101aaeaaa3f3fc150c79c5b98f8329a2846dea20e8a02e1bb5c13818b8")
				.unchecked_into(),
			hex!("9099637497a754e54e252eb689f77cefb2b921d70722f8aed7380f27f371eb27")
				.unchecked_into(),
			hex!("88b009545c03cb600264489ba8b8e61f62eedc99064a51d3405fb517542da04a")
				.unchecked_into(),
			hex!("d06475ed615fec5b4b3c900f7779903d6e95173ab944840ce4acf8c30b5f3b32")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("035f1efad44a7de67474b9425bc3000e4cab8363e498e8f9804b9e3d8401a9b144")
				.unchecked_into(),
		),
		(
			// Validator 15 (5FYoCM3oeEGeoFY94EgXBhmABkRCabvPp72ur5bJNG3cK619)
			hex!("9a218b96cf0267b1a7f8d9d1932ed60ecb0a7a681fb882479a93c2ce4b869610").into(),
			hex!("9a3cf4772d884077680b268a76ee703d1aefb9a4c48f1576e9874a61db3a0406").into(),
			hex!("241fa3be1b57d3be4ff6fc7808fab44d8df93856699bcfe9d5768c219003c70a")
				.unchecked_into(),
			hex!("3d07a507b495b2eb295b1045e550ad657fd55ac7dea773bdfd5f80403b3d8f4f")
				.unchecked_into(),
			hex!("ec9a49b0a41d20eb40356c3bd1186fc8281a300c92cfac167c3d27bdfbb3bf28")
				.unchecked_into(),
			hex!("aa1af739a5525dd6a186ec367b227b2c76859230d458329713edaebc48f7b74d")
				.unchecked_into(),
			hex!("509fc3e6d9edc45539fae863e9a81be33e5f0ee370c78533a0eb0a975786af39")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("033ac8624837a0ed5fb7f1ab9c08d58469c42e2598c49e12947b97f5cceabab930")
				.unchecked_into(),
		),
		(
			// Validator 16 (5GspwkKF6aYzFkmAyBBQg7coSCSgDCore79fbW8uxJNAH347)
			hex!("d4e153a229ac679831421ab2604b6fc347c8a1180f6c977d60a32cb7cac9eb03").into(),
			hex!("3e5672b2a4a19ed8fe9c5483940753d7e3edf062ea64af35db529de084890c3b").into(),
			hex!("ae0eb849027893c9f5fbe2512e0c0e1bd330e51d7afbe5ee9345ca997691c004")
				.unchecked_into(),
			hex!("7ff1b4e854560d0d7739957dffaf182ab36ba67c2290016274376638493d95eb")
				.unchecked_into(),
			hex!("e022e8ba625a94452df5058127c4471289e08bc026561a428962b43032ea8d4e")
				.unchecked_into(),
			hex!("7e83a62c3c60106c7b448453827e300ac75c269bed996b69e5bb5a7b99aeee7e")
				.unchecked_into(),
			hex!("7875d43192c57dc8028a9468d7b18e66ae46b065731e28b6691b5614fa505740")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("0392e5b4d2653b0409fcc25649536516d16c89587e44ddccfd4edda779358418ad")
				.unchecked_into(),
		),
		(
			// Validator 17 (5GmuX11pN2fC4Fyq1V7MuiYt3aevZcVQs3HZWKyzmap9bKfe)
			hex!("d05d3fde48e6a9b739f2ebac00881248dcb04000c1953dc17a325318e7678b74").into(),
			hex!("7ce0889313160507a7cefaae2bd9cdea763e5ad57a1a294739bca41bf4fc336d").into(),
			hex!("e495464eaac8f8a56bbf0171e78d43b5da1d4f5bca5fa22afe2bdbec3588df62")
				.unchecked_into(),
			hex!("1229786eeff8c0bf22358d63de43812a9c3315fc3f77d871e41828f42d39d4c3")
				.unchecked_into(),
			hex!("ec7e5b3ada32f54173bb0e4cbf5b22eef6dba0f6e3e380353fddce1dea0e5a4f")
				.unchecked_into(),
			hex!("68a2ff42beef103bd09ce879d2d64ca2aa6dcb91211653c9a4a66802cb19db16")
				.unchecked_into(),
			hex!("5e28fa8dea95c520ed42509a7c5050617688148805c211ff634553b8b734db38")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("03d1d118b8a51a1cb72f3f0ebcf11cae21963c6dd1418e010d17e5c52996c302cb")
				.unchecked_into(),
		),
		(
			// Validator 18 (5FQptVCtM1qsxkLbQkATkw4Kio4M9LxWvM6TwgEo3QjmTXF3)
			hex!("940d47307adb77756d7d99677120ec176bdee0e88df1b877f3a3b36a6168a13e").into(),
			hex!("d26c61d2a79befe1ed1522dc79dc487a1b01b7b0424bbee233c3b6d6133b0b36").into(),
			hex!("de9e6dc042e3192934b495c01b665ce5024e77bb3516397b0d8b8d9a3f22e346")
				.unchecked_into(),
			hex!("bbeccb7ece29ad0831840ff0a620e039c464323a7b5539e7df48c60084099198")
				.unchecked_into(),
			hex!("5c9292dd791104d019291f495caea1662bef2dfa4c6b7e39fe26a03ff64e1d2f")
				.unchecked_into(),
			hex!("40fba87e041a3fc317f15e72f98c2b45a411e262122bfedcdacbfc88c7677e4e")
				.unchecked_into(),
			hex!("0ecdca8ea4479ea078041778a72ba70a591c19b380620ef8ec22eb3e883be36e")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("03cab73da1d53c5fa60ea2a52ca5f9c11998c07fbd9cec89707ee1db87051e3e65")
				.unchecked_into(),
		),
		(
			// Validator 19 (5E7VD2qmso1yRfyq3t9u2qhauAgtmjZTybVsCARF5Zz9bXy6)
			hex!("5a979f8534d09e40776fc69aa27659fec47023281e1192bf2eda74adc8779f1b").into(),
			hex!("4268358bfd339d689b2a2330f14671c474ae154bf5659d65f8b621f0643db515").into(),
			hex!("0406457ca0553ea12faaa39ec9e486c33852aa2733299b413451e156dbab5820")
				.unchecked_into(),
			hex!("29eb2da3b8063c893792729c504ea67a1eb7179bfeb2d46770fb1b2e751fd9ea")
				.unchecked_into(),
			hex!("a0a72ce07da14e83c75abaf1c6e498193f15a0a6f696c24107f0d5e267240e2a")
				.unchecked_into(),
			hex!("e81cc87d913aa31e46df7bedcbe57b1284111fe7abaedb3fc18377d9364b845a")
				.unchecked_into(),
			hex!("0a823f91ad2e5cf332a41777ee62c8f29c727ef4b24bb3c739dcc17ea5a32725")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("03821bf29d19ce075bc045f245c4e10e06bce58695ba0f1e6a31e94e4a435cfeb2")
				.unchecked_into(),
		),
		(
			// Validator 20 (5Ccz5W7Q21g4UPCytzHxD3VSMLJ1BbbWSkJKFwsNtYRk3HkX)
			hex!("189e75681b36a25fce4709241ce16863c6b4bddef2a7c55f5249d2c7fe77ba50").into(),
			hex!("8878312c9ddd3c5071c21a96baa751e6668a6d0428ba9897d714340084168e57").into(),
			hex!("e68bd3aefd3003c06404eba43dacd5aa3888217a07f9df7cdaa17918e63bb92f")
				.unchecked_into(),
			hex!("046ddb7c2a1466ab4ed203e3a64d7b0552917cd75bb70c52db4c1b2e8470de27")
				.unchecked_into(),
			hex!("d4fb75a88fb9e4a3b954643852d6a80e75f41d8dfe599e905639fcaa09b8c43a")
				.unchecked_into(),
			hex!("3e1438dacaf7ed4355e5207136a8eaa505ac042c385cfee1110ee647b05e2541")
				.unchecked_into(),
			hex!("c2821fde10d149743cf7f6cf3249c8e2e5bc17d9364b8a6af0119fbc8cbf8469")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("020ba75ce3c26fab250a4abe8fdd5b45c2b9feec308ff6b8bab98e8f25b5e8f686")
				.unchecked_into(),
		),
		(
			// Validator 21 (5D7WPmK1SAJyYDdCtgqEzGJpWXQe3Lj9FqWL8z9waLTkUNv3)
			hex!("2e5f60fd87b662097c182f409f49d23da112b132c3057bfe0956094436663854").into(),
			hex!("ee53261dfdee7320d2738d7eb249510244ba70d70acd668b03c219f79ab3400c").into(),
			hex!("9a00b890d45479714957e233fb4f5e197f912d5b7ab31c976af3926b9b279d7d")
				.unchecked_into(),
			hex!("81ff23b7233abbc8411099391d860582b870f4003184e5ea18c02d8554e6f9eb")
				.unchecked_into(),
			hex!("5a7f4f5995829f9f7626b45cbbffd5cbaea392eda7299d297cd58dadabdb1531")
				.unchecked_into(),
			hex!("04f83a2cc77593e656073e9b189de1d960a98222f269a66aec483fa299a9816f")
				.unchecked_into(),
			hex!("a6f0d33a810d2023f0b05bb1aeda000307c66e608a2ec096df2075a2d8d3204e")
				.unchecked_into(),
			// BEEFY: from mainnet keystore (substrate ECDSA derivation)
			hex!("027820252d089e6e9e95e8a328ffef1f1e03173d04be4f2957aeb54fa36a94303b")
				.unchecked_into(),
		),
	]);

	// Validator stash amount
	const STASH: u128 = 100 * TYR;

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: vec![
				// HEZ Genesis Distribution (200M Total)
				(founder_account.clone(), HEZ_FOUNDER_ALLOCATION), // 10% = 20M HEZ
				(presale_account.clone(), HEZ_PRESALE_ALLOCATION), // 50% = 100M HEZ
				(treasury_account.clone(), HEZ_TREASURY_ALLOCATION), // 20% = 40M HEZ
				(airdrop_account.clone(), HEZ_AIRDROP_ALLOCATION), // 20% = 40M HEZ
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
		sudo: SudoConfig { key: Some(founder_account) },
		configuration: ConfigurationConfig { config: default_teyrchains_host_configuration() },
		registrar: RegistrarConfig { next_free_para_id: pezkuwi_primitives::LOWEST_PUBLIC_ID },
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

	const STASH: u128 = 100 * TYR;

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: vec![
				// Founder gets enough balance to pay for upgrades + testing
				(founder_account.clone(), 1_000_000 * TYR),
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
