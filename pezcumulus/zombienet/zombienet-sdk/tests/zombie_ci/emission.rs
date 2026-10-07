// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The Asset Hub's era clock turns, every era mints HEZ, and the committee is paid for its work.
//!
//! Measured on the live Zagros on 2026-10-04: the Asset Hub's `ActiveEra` had stayed at 0 since
//! genesis, because the genesis preset set `ForceNone` and nothing ever started an era -- no
//! inflation, no treasury share, no staking reward, for weeks, and nothing in CI asked. This
//! asks, on a fresh local network: does the era advance and issuance grow, and does an era's
//! pay reach the validators who did its work, split half to each validator and half to its
//! nominators, with the half no one is owed going to the treasury.
//!
//! The relay validators' stashes are made Asset Hub validators here -- funded, bonded and
//! validating -- because only an account with an exposure can be paid: the relay reports work
//! by stash, and without a stake on this chain that work is paid to the treasury. One is
//! nominated and one is not, so both halves of the split are measured. There is no People
//! chain in this network, so no committee snapshot: every validator is a target and every
//! point weighs the full thousand, and what is measured is the split, not the trust.

use anyhow::anyhow;
use pezkuwi_zombienet_sdk::{
	subxt::{
		dynamic::{self, At, Value},
		tx::DynamicPayload,
		OnlineClient, PezkuwiConfig,
	},
	subxt_signer::{
		sr25519::{dev, Keypair},
		SecretUri,
	},
	NetworkConfig, NetworkConfigBuilder,
};
use serde_json::json;
use std::str::FromStr;

use super::state_rehearsal_offices::{raw_account, storage_value};
use crate::utils::initialize_network;

const ASSET_HUB_ID: u32 = 1000;
/// Fast runtime: a session is 20 blocks and, once planned, an era lasts one more session.
/// Thirty minutes is several eras of headroom; a clock that is stopped never gets there.
const ERA_WAIT_SECS: u64 = 1800;
const HEZ: u128 = 1_000_000_000_000;
/// Above the 10,000 HEZ validator bond.
const BOND: u128 = 20_000 * HEZ;

fn era_index(v: &Value) -> Option<u128> {
	v.at("index").and_then(|i| i.as_u128())
}

/// The stash zombienet derives for a relay validator node: `//<Name>//stash`, the account the
/// relay reports its work under.
fn relay_stash(node: &str) -> Result<Keypair, anyhow::Error> {
	let mut name = node.to_string();
	let first = name.remove(0).to_uppercase();
	let uri = SecretUri::from_str(&format!("//{first}{name}//stash"))
		.map_err(|e| anyhow!("{node}: {e}"))?;
	Keypair::from_uri(&uri).map_err(|e| anyhow!("{node}: {e}"))
}

/// The Asset Hub treasury: `PalletId(*b"py/trsry")` as an account.
fn treasury() -> Value {
	let mut id = [0u8; 32];
	id[..4].copy_from_slice(b"modl");
	id[4..12].copy_from_slice(b"py/trsry");
	Value::from_bytes(id)
}

fn id(k: &Keypair) -> Value {
	Value::unnamed_variant("Id", vec![raw_account(k)])
}

async fn must(
	api: &OnlineClient<PezkuwiConfig>,
	tx: &DynamicPayload,
	signer: &Keypair,
	what: &str,
) -> Result<(), anyhow::Error> {
	api.tx()
		.sign_and_submit_then_watch_default(tx, signer)
		.await
		.map_err(|e| anyhow!("{what}: submission failed: {e}"))?
		.wait_for_finalized_success()
		.await
		.map(|_| ())
		.map_err(|e| anyhow!("{what}: {e}"))
}

async fn u128_at(
	api: &OnlineClient<PezkuwiConfig>,
	pallet: &str,
	item: &str,
	keys: Vec<Value>,
) -> Result<Option<u128>, anyhow::Error> {
	Ok(storage_value(api, pallet, item, keys).await?.and_then(|v| v.as_u128()))
}

async fn free(api: &OnlineClient<PezkuwiConfig>, who: Value) -> Result<u128, anyhow::Error> {
	Ok(storage_value(api, "System", "Account", vec![who])
		.await?
		.and_then(|v| v.at("data").and_then(|d| d.at("free")).and_then(|f| f.as_u128()))
		.unwrap_or(0))
}

async fn active_era(api: &OnlineClient<PezkuwiConfig>) -> Result<u128, anyhow::Error> {
	Ok(storage_value(api, "Staking", "ActiveEra", vec![])
		.await?
		.as_ref()
		.and_then(era_index)
		.unwrap_or(0))
}

#[tokio::test(flavor = "multi_thread")]
async fn ah_era_advances_and_issuance_grows() -> Result<(), anyhow::Error> {
	let _ = env_logger::try_init_from_env(
		env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
	);
	let network = initialize_network(build_network_config().await?).await?;
	let ah: OnlineClient<PezkuwiConfig> =
		network.get_node("asset-hub-collator")?.wait_client().await?;

	let issuance_at_start = u128_at(&ah, "Balances", "TotalIssuance", vec![])
		.await?
		.ok_or_else(|| anyhow!("no TotalIssuance on the Asset Hub"))?;
	log::info!("Asset Hub issuance at start: {issuance_at_start}");

	// The relay validators' stashes validate here; validator-0 is nominated, validator-1 not.
	let (alice, bob) = (dev::alice(), dev::bob());
	let (v0, v1) = (relay_stash("validator-0")?, relay_stash("validator-1")?);
	for (v, name) in [(&v0, "validator-0"), (&v1, "validator-1")] {
		let fund = dynamic::tx(
			"Balances",
			"transfer_keep_alive",
			vec![id(v), Value::u128(BOND + 1_000 * HEZ)],
		);
		must(&ah, &fund, &alice, &format!("fund {name}'s stash")).await?;
		let bond = dynamic::tx(
			"Staking",
			"bond",
			vec![Value::u128(BOND), Value::unnamed_variant("Stash", vec![])],
		);
		must(&ah, &bond, v, &format!("{name} bonds")).await?;
		let prefs = Value::named_composite([
			("commission", Value::u128(0)),
			("blocked", Value::bool(false)),
		]);
		must(
			&ah,
			&dynamic::tx("Staking", "validate", vec![prefs]),
			v,
			&format!("{name} validates"),
		)
		.await?;
	}
	let bond = dynamic::tx(
		"Staking",
		"bond",
		vec![Value::u128(BOND), Value::unnamed_variant("Stash", vec![])],
	);
	must(&ah, &bond, &bob, "bob bonds").await?;
	let nominate = dynamic::tx("Staking", "nominate", vec![Value::unnamed_composite([id(&v0)])]);
	must(&ah, &nominate, &bob, "bob nominates validator-0").await?;
	let bonded_in = active_era(&ah).await?;
	log::info!("stakers in place during era {bonded_in}");

	// The first era elected with them in it, and its end.
	let mut paid_era = None;
	for _ in 0..(ERA_WAIT_SECS / 6) {
		let now = active_era(&ah).await?;
		for e in (bonded_in + 1)..now {
			let exposed = storage_value(
				&ah,
				"Staking",
				"ErasStakersOverview",
				vec![Value::u128(e), raw_account(&v0)],
			)
			.await?
			.is_some();
			if exposed {
				paid_era = Some(e);
				break;
			}
		}
		if paid_era.is_some() {
			break;
		}
		tokio::time::sleep(std::time::Duration::from_secs(6)).await;
	}
	let era = paid_era.ok_or_else(|| {
		anyhow!(
			"no era both exposed validator-0 and ended in {ERA_WAIT_SECS}s (active era {}) -- the \
			 clock stopped or the election never took the new validators",
			bonded_in
		)
	})?;

	let issuance_now = u128_at(&ah, "Balances", "TotalIssuance", vec![]).await?.unwrap_or(0);
	if issuance_now <= issuance_at_start {
		return Err(anyhow!(
			"era {era} ended but TotalIssuance did not grow ({issuance_at_start} -> \
			 {issuance_now}): the treasury's share was never minted"
		));
	}
	let points = storage_value(&ah, "Staking", "ErasRewardPoints", vec![Value::u128(era)]).await?;
	log::info!("era {era}: issuance {issuance_at_start} -> {issuance_now}; points {points:?}");

	// validator-0 and its nominator: half each.
	let (v0_before, bob_before) =
		(free(&ah, raw_account(&v0)).await?, free(&ah, raw_account(&bob)).await?);
	let payout = dynamic::tx("Staking", "payout_stakers", vec![raw_account(&v0), Value::u128(era)]);
	must(&ah, &payout, &alice, "pay validator-0's era").await?;
	let v0_got = free(&ah, raw_account(&v0)).await? - v0_before;
	let bob_got = free(&ah, raw_account(&bob)).await? - bob_before;
	if v0_got == 0 {
		return Err(anyhow!(
			"validator-0 was paid nothing for era {era}: its work did not reach its exposure \
			 (points {points:?})"
		));
	}
	if v0_got.abs_diff(bob_got) > 2 {
		return Err(anyhow!(
			"validator-0 got {v0_got} and its only nominator {bob_got}: not half and half"
		));
	}

	// validator-1 has no nominator: its half goes to the treasury.
	let (v1_before, treasury_before) =
		(free(&ah, raw_account(&v1)).await?, free(&ah, treasury()).await?);
	let payout = dynamic::tx("Staking", "payout_stakers", vec![raw_account(&v1), Value::u128(era)]);
	must(&ah, &payout, &alice, "pay validator-1's era").await?;
	let v1_got = free(&ah, raw_account(&v1)).await? - v1_before;
	let treasury_got = free(&ah, treasury()).await?.saturating_sub(treasury_before);
	if v1_got == 0 || treasury_got < v1_got.saturating_sub(2) {
		return Err(anyhow!(
			"validator-1 got {v1_got} and the treasury {treasury_got}: the nominators' half of a \
			 validator with none did not reach the treasury"
		));
	}
	log::info!(
		"era {era}: validator-0 {v0_got}, its nominator {bob_got}; validator-1 {v1_got}, \
		 treasury {treasury_got}"
	);
	Ok(())
}

async fn build_network_config() -> Result<NetworkConfig, anyhow::Error> {
	let images = pezkuwi_zombienet_sdk::environment::get_images_from_env();
	NetworkConfigBuilder::new()
		.with_relaychain(|r| {
			r.with_chain("zagros-local")
				// The preset's two configured cores come first and the Asset Hub, registered at
				// genesis, opens its core after them -- behind a validator group two validators
				// never fill, so it stopped at block 3 and no session ever ended (measured on
				// the 2026-10-07 run: Asset Hub best #3, relay #306). With none configured its
				// registration makes core 0, and one validator per group backs it.
				.with_genesis_overrides(json!({
					"configuration": {
						"config": {
							"scheduler_params": { "num_cores": 0, "max_validators_per_core": 1 }
						}
					}
				}))
				.with_default_command("pezkuwi")
				.with_default_image(images.pezkuwi())
				.with_validator(|n| n.with_name("validator-0"))
				.with_validator(|n| n.with_name("validator-1"))
		})
		.with_teyrchain(|p| {
			p.with_id(ASSET_HUB_ID)
				.with_chain("asset-hub-zagros-local")
				.with_default_command("pezkuwi-teyrchain")
				.with_default_image(images.pezcumulus())
				.with_collator(|n| n.with_name("asset-hub-collator"))
		})
		.with_global_settings(|g| match std::env::var("ZOMBIENET_SDK_BASE_DIR") {
			Ok(val) => g.with_base_dir(val),
			_ => g,
		})
		.build()
		.map_err(|e| {
			let errs = e.into_iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ");
			anyhow!("config errs: {errs}")
		})
}
