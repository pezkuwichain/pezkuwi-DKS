// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Zagros runs every system chain Pezkuwichain will take on.
//!
//! The live Zagros was launched with its relay, Asset Hub and People only. Its Bridge Hub,
//! Coretime and Collectives were kept as runtimes but never run, and a runtime nobody has run
//! next to the others is a claim, not a chain. Zagros is to run all six -- the three it lacks
//! are where they are tried before Pezkuwichain takes them on (Serok, 2026-10-03) -- so before
//! they are registered on the live relay, this raises the whole set together and asks one
//! question of it: does every one of the five teyrchains get its blocks included, at the
//! same time, on one relay.
//!
//! Para ids are the live ones: Asset Hub 1000, Collectives 1001, Bridge Hub 1002, People 1004,
//! Coretime 1005.

use anyhow::anyhow;
use serde_json::json;

use crate::utils::initialize_network;

use pezcumulus_zombienet_sdk_helpers::assert_para_throughput;
use pezkuwi_primitives::Id as ParaId;
use pezkuwi_zombienet_sdk::{
	subxt::{OnlineClient, PezkuwiConfig},
	NetworkConfig, NetworkConfigBuilder,
};

/// (para id, chain, collator name) for every teyrchain Zagros runs.
const TEYRCHAINS: [(u32, &str, &str); 5] = [
	(1000, "asset-hub-zagros-local", "asset-hub-collator"),
	(1001, "collectives-zagros-local", "collectives-collator"),
	(1002, "bridge-hub-zagros-local", "bridge-hub-collator"),
	(1004, "people-zagros-local", "people-collator"),
	(1005, "coretime-zagros-local", "coretime-collator"),
];

#[tokio::test(flavor = "multi_thread")]
async fn zagros_runs_every_system_chain() -> Result<(), anyhow::Error> {
	let _ = env_logger::try_init_from_env(
		env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
	);

	log::info!("Spawning the Zagros relay with all five teyrchains");
	let network = initialize_network(build_network_config().await?).await?;

	let relay = network.get_node("validator-0")?;
	let relay_client: OnlineClient<PezkuwiConfig> = relay.wait_client().await?;

	// Over thirty relay blocks, every teyrchain has to have candidates included. One core each,
	// so a chain that produces is included roughly every other relay block; the lower bound is
	// low enough to tolerate a slow start and still refuses a chain that never comes up.
	log::info!("Ensuring every teyrchain gets its blocks included");
	assert_para_throughput(
		&relay_client,
		30,
		TEYRCHAINS.map(|(id, _, _)| (ParaId::from(id), 5..31)),
		[],
	)
	.await?;

	log::info!("all five Zagros teyrchains are producing on one relay");
	Ok(())
}

async fn build_network_config() -> Result<NetworkConfig, anyhow::Error> {
	let images = pezkuwi_zombienet_sdk::environment::get_images_from_env();
	let mut builder = NetworkConfigBuilder::new().with_relaychain(|r| {
		let r = r
			.with_chain("zagros-local")
			// One core and one backing validator per teyrchain, so that no chain waits for a
			// core another one holds -- the question here is whether each chain runs, not how
			// they share.
			.with_genesis_overrides(json!({
				"configuration": {
					"config": {
						"scheduler_params": {
							"num_cores": TEYRCHAINS.len(),
							"max_validators_per_core": 1
						}
					}
				}
			}))
			.with_default_command("pezkuwi")
			.with_default_image(images.pezkuwi())
			.with_validator(|n| n.with_name("validator-0"));
		(1..TEYRCHAINS.len())
			.fold(r, |r, i| r.with_validator(|n| n.with_name(format!("validator-{i}"))))
	});

	for (id, chain, collator) in TEYRCHAINS {
		builder = builder.with_teyrchain(|p| {
			p.with_id(id)
				.with_chain(chain)
				.with_default_command("pezkuwi-teyrchain")
				.with_default_image(images.pezcumulus())
				.with_collator(|n| n.with_name(collator))
		});
	}

	builder
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
