// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The Asset Hub's era clock turns and every era mints HEZ.
//!
//! Measured on the live Zagros on 2026-10-04: the Asset Hub's `ActiveEra` had stayed at 0 since
//! genesis, because the genesis preset set `ForceNone` and nothing ever started an era -- no
//! inflation, no treasury share, no staking reward, for weeks, and nothing in CI asked. This
//! asks: on a fresh local network, does the era advance and does `TotalIssuance` grow.

use anyhow::anyhow;
use pezkuwi_zombienet_sdk::{
	subxt::{
		dynamic::{At, Value},
		OnlineClient, PezkuwiConfig,
	},
	NetworkConfig, NetworkConfigBuilder,
};

use super::state_rehearsal_offices::storage_value;
use crate::utils::initialize_network;

const ASSET_HUB_ID: u32 = 1000;
/// Fast runtime: a session is 20 blocks and, once planned, an era lasts one more session.
/// Thirty minutes is several eras of headroom; a clock that is stopped never gets there.
const ERA_WAIT_SECS: u64 = 1800;

fn era_index(v: &Value) -> Option<u128> {
	v.at("index").and_then(|i| i.as_u128())
}

#[tokio::test(flavor = "multi_thread")]
async fn ah_era_advances_and_issuance_grows() -> Result<(), anyhow::Error> {
	let _ = env_logger::try_init_from_env(
		env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
	);
	let network = initialize_network(build_network_config().await?).await?;
	let ah: OnlineClient<PezkuwiConfig> =
		network.get_node("asset-hub-collator")?.wait_client().await?;

	let issuance_at_start = storage_value(&ah, "Balances", "TotalIssuance", vec![])
		.await?
		.and_then(|v| v.as_u128())
		.ok_or_else(|| anyhow!("no TotalIssuance on the Asset Hub"))?;
	log::info!("Asset Hub issuance at start: {issuance_at_start}");

	let mut reached = None;
	for _ in 0..(ERA_WAIT_SECS / 6) {
		if let Some(era) = storage_value(&ah, "Staking", "ActiveEra", vec![])
			.await?
			.as_ref()
			.and_then(era_index)
		{
			if era >= 1 {
				reached = Some(era);
				break;
			}
		}
		tokio::time::sleep(std::time::Duration::from_secs(6)).await;
	}
	let era = reached.ok_or_else(|| {
		anyhow!("the Asset Hub's active era did not leave 0 in {ERA_WAIT_SECS}s -- no era ends, so EraPayout never mints")
	})?;

	let issuance_now = storage_value(&ah, "Balances", "TotalIssuance", vec![])
		.await?
		.and_then(|v| v.as_u128())
		.ok_or_else(|| anyhow!("no TotalIssuance on the Asset Hub"))?;
	if issuance_now <= issuance_at_start {
		return Err(anyhow!(
			"the active era reached {era} but TotalIssuance did not grow ({issuance_at_start} -> {issuance_now}): an era ended without minting"
		));
	}
	log::info!("era {era}, issuance {issuance_at_start} -> {issuance_now}");
	Ok(())
}

async fn build_network_config() -> Result<NetworkConfig, anyhow::Error> {
	let images = pezkuwi_zombienet_sdk::environment::get_images_from_env();
	NetworkConfigBuilder::new()
		.with_relaychain(|r| {
			r.with_chain("zagros-local")
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
