// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Test that a teyrchain that uses a basic collator (like adder-collator) with elastic scaling
// can achieve full throughput of 3 candidates per block.

use anyhow::anyhow;
use pezcumulus_zombienet_sdk_helpers::{assert_para_throughput, create_assign_core_call};
use pezkuwi_primitives::Id as ParaId;
use pezkuwi_zombienet_sdk::{
	subxt::{OnlineClient, PezkuwiConfig},
	subxt_signer::sr25519::dev,
	NetworkConfigBuilder,
};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn basic_3cores_test() -> Result<(), anyhow::Error> {
	let _ = env_logger::try_init_from_env(
		env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
	);

	let images = pezkuwi_zombienet_sdk::environment::get_images_from_env();

	let config = NetworkConfigBuilder::new()
		.with_relaychain(|r| {
			let r = r
				.with_chain("pezkuwichain-local")
				.with_default_command("pezkuwi")
				.with_default_image(images.pezkuwi())
				.with_default_args(vec![("-lteyrchain=debug").into()])
				.with_genesis_overrides(json!({
					"configuration": {
						"config": {
							"scheduler_params": {
								"num_cores": 2,
								"max_validators_per_core": 1
							},
						}
					}
				}))
				// Have to set a `with_node` outside of the loop below, so that `r` has the right
				// type.
				.with_validator(|node| node.with_name("validator-0"));

			(1..4).fold(r, |acc, i| {
				acc.with_validator(|node| node.with_name(&format!("validator-{i}")))
			})
		})
		.with_teyrchain(|p| {
			p.with_id(2000)
				.with_default_command("adder-collator")
				.pezcumulus_based(false)
				.with_default_image(images.pezcumulus())
				.with_default_args(vec![("-lteyrchain=debug").into()])
				.with_collator(|n| n.with_name("adder-2000"))
		})
		.with_teyrchain(|p| {
			p.with_id(2001)
				.with_default_command("adder-collator")
				.pezcumulus_based(false)
				.with_default_image(images.pezcumulus())
				.with_default_args(vec![("-lteyrchain=debug").into()])
				.with_collator(|n| n.with_name("adder-2001"))
		})
		.build()
		.map_err(|e| {
			let errs = e.into_iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ");
			anyhow!("config errs: {errs}")
		})?;

	let spawn_fn = pezkuwi_zombienet_sdk::environment::get_spawn_fn();
	let network = spawn_fn(config).await?;

	let relay_node = network.get_node("validator-0")?;

	let relay_client: OnlineClient<PezkuwiConfig> = relay_node.wait_client().await?;
	let alice = dev::alice();

	// Assign two extra cores to adder-2000.
	relay_client
		.tx()
		.sign_and_submit_then_watch_default(
			&create_assign_core_call(&[(0, 2000), (1, 2000)]),
			&alice,
		)
		.await?
		.wait_for_finalized_success()
		.await?;

	log::info!("2 more cores assigned to adder-2000");

	assert_para_throughput(
		&relay_client,
		15,
		[(ParaId::from(2000), 38..46), (ParaId::from(2001), 12..16)]
			.into_iter()
			.collect(),
	)
	.await?;

	log::info!("Test finished successfully");

	Ok(())
}
