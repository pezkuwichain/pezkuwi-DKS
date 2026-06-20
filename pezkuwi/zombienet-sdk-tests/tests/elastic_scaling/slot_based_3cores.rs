// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Test that teyrchains that use a single slot-based collator with elastic scaling MVP and with
// elastic scaling with RFC103 can achieve full throughput of 3 candidates per block.

use anyhow::anyhow;

use pezcumulus_zombienet_sdk_helpers::{
	assert_finality_lag, assert_para_throughput, create_assign_core_call,
};
use pezkuwi_primitives::Id as ParaId;
use pezkuwi_zombienet_sdk::{
	subxt::{OnlineClient, PezkuwiConfig},
	subxt_signer::sr25519::dev,
	NetworkConfigBuilder,
};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn slot_based_3cores_test() -> Result<(), anyhow::Error> {
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
								// Num cores is 4, because 2 extra will be added automatically when registering the paras.
								"num_cores": 4,
								"max_validators_per_core": 2
							}
						}
					}
				}))
				.with_default_resources(|resources| {
					resources.with_request_cpu(4).with_request_memory("4G")
				})
				// Have to set a `with_node` outside of the loop below, so that `r` has the right
				// type.
				.with_validator(|node| node.with_name("validator-0"));

			(1..12).fold(r, |acc, i| {
				acc.with_validator(|node| node.with_name(&format!("validator-{i}")))
			})
		})
		.with_teyrchain(|p| {
			// Para 2100 uses the old elastic scaling mvp, which doesn't send the new UMP signal
			// commitment for selecting the core index.
			p.with_id(2100)
				.with_default_command("test-teyrchain")
				.with_default_image(images.pezcumulus())
				.with_chain("elastic-scaling-mvp")
				.with_default_args(vec![
					"--authoring=slot-based".into(),
					("-lteyrchain=debug,aura=debug").into(),
				])
				.with_collator(|n| n.with_name("collator-elastic-mvp"))
		})
		.with_teyrchain(|p| {
			// Para 2200 uses the new RFC103-enabled collator which sends the UMP signal commitment
			// for selecting the core index
			p.with_id(2200)
				.with_default_command("test-teyrchain")
				.with_default_image(images.pezcumulus())
				.with_chain("elastic-scaling")
				.with_default_args(vec![
					"--authoring=slot-based".into(),
					("-lteyrchain=debug,aura=debug").into(),
				])
				.with_collator(|n| n.with_name("collator-elastic"))
		})
		.build()
		.map_err(|e| {
			let errs = e.into_iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ");
			anyhow!("config errs: {errs}")
		})?;

	let spawn_fn = pezkuwi_zombienet_sdk::environment::get_spawn_fn();
	let network = spawn_fn(config).await?;

	let relay_node = network.get_node("validator-0")?;
	let para_node_elastic = network.get_node("collator-elastic")?;
	let para_node_elastic_mvp = network.get_node("collator-elastic-mvp")?;

	let relay_client: OnlineClient<PezkuwiConfig> = relay_node.wait_client().await?;
	let alice = dev::alice();

	let assign_cores_call = create_assign_core_call(&[(0, 2100), (1, 2100), (2, 2200), (3, 2200)]);
	// Assign two extra cores to each teyrchain.
	relay_client
		.tx()
		.sign_and_submit_then_watch_default(&assign_cores_call, &alice)
		.await?
		.wait_for_finalized_success()
		.await?;

	log::info!("2 more cores assigned to each teyrchain");

	// Expect a backed candidate count of at least 39 for each teyrchain in 15 relay chain blocks
	// (2.6 candidates per para per relay chain block).
	// Note that only blocks after the first session change and blocks that don't contain a session
	// change will be counted.
	// Since the calculated backed candidate count is theoretical and the CI tests are observed to
	// occasionally fail, let's apply 10% tolerance to the expected range: 39 - 10% = 35
	assert_para_throughput(
		&relay_client,
		15,
		[(ParaId::from(2100), 35..46), (ParaId::from(2200), 35..46)]
			.into_iter()
			.collect(),
	)
	.await?;

	// Assert the teyrchain finalized block height is also on par with the number of backed
	// candidates.
	assert_finality_lag(&para_node_elastic.wait_client().await?, 15).await?;
	assert_finality_lag(&para_node_elastic_mvp.wait_client().await?, 15).await?;

	log::info!("Test finished successfully");

	Ok(())
}
