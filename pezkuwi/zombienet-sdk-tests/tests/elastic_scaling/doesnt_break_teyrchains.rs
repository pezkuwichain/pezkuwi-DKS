// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Test that a paraid that doesn't use elastic scaling which acquired multiple cores does not brick
// itself if ElasticScalingMVP feature is enabled in genesis.

use anyhow::anyhow;
use codec::Decode;
use pezcumulus_zombienet_sdk_helpers::{
	assert_finality_lag, assert_para_throughput, assign_cores, wait_for_pvf_prepare,
};
use pezkuwi_primitives::{CoreIndex, Id as ParaId};
use pezkuwi_zombienet_sdk::{
	subxt::{OnlineClient, PezkuwiConfig},
	NetworkConfigBuilder,
};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};

#[tokio::test(flavor = "multi_thread")]
async fn doesnt_break_teyrchains_test() -> Result<(), anyhow::Error> {
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
								"num_cores": 1,
								"max_validators_per_core": 2,
							}
						}
					}
				}))
				// Have to set a `with_validator` outside of the loop below, so that `r` has the
				// right type.
				.with_validator(|node| node.with_name("validator-0"));

			(1..4).fold(r, |acc, i| {
				acc.with_validator(|node| node.with_name(&format!("validator-{i}")))
			})
		})
		.with_teyrchain(|p| {
			// Use default, which has 6 second slot time. Also, don't use slot-based collator.
			p.with_id(2000)
				.with_default_command("pezkuwi-teyrchain")
				.with_default_image(images.pezcumulus())
				.with_default_args(vec![("-lteyrchain=debug,aura=debug").into()])
				.with_collator(|n| n.with_name("collator-2000"))
		})
		.build()
		.map_err(|e| {
			let errs = e.into_iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" ");
			anyhow!("config errs: {errs}")
		})?;

	let spawn_fn = pezkuwi_zombienet_sdk::environment::get_spawn_fn();
	let network = spawn_fn(config).await?;

	let relay_node = network.get_node("validator-0")?;
	let para_node = network.get_node("collator-2000")?;

	let relay_client: OnlineClient<PezkuwiConfig> = relay_node.wait_client().await?;

	assign_cores(&relay_client, 2000, vec![0]).await?;

	let para_id = ParaId::from(2000);
	// Wait for PVF preparation to complete.
	wait_for_pvf_prepare(&network, 1).await?;
	// Expect the teyrchain to be making normal progress, 1 candidate backed per relay chain block.
	// Lowering to 12 to make sure CI passes.
	assert_para_throughput(&relay_client, 15, [(para_id, 12..16)], []).await?;

	let para_client = para_node.wait_client().await?;
	// Assert the teyrchain finalized block height is also on par with the number of backed
	// candidates.
	// Increasing to 6 to make sure CI passes.
	assert_finality_lag(&para_client, 6).await?;

	// Sanity check that indeed the teyrchain has two assigned cores.
	let cq = BTreeMap::<CoreIndex, VecDeque<ParaId>>::decode(
		&mut &relay_client
			.runtime_api()
			.at_latest()
			.await?
			.call_raw("TeyrchainHost_claim_queue", None)
			.await?[..],
	)?;

	// Get looakahead config
	let lookahead = u32::decode(
		&mut &relay_client
			.runtime_api()
			.at_latest()
			.await?
			.call_raw("TeyrchainHost_scheduling_lookahead", None)
			.await?[..],
	)?;

	assert_eq!(
		cq,
		[
			(CoreIndex(0), std::iter::repeat_n(para_id, lookahead as usize).collect()),
			(CoreIndex(1), std::iter::repeat_n(para_id, lookahead as usize).collect()),
		]
		.into_iter()
		.collect()
	);

	log::info!("Test finished successfully");

	Ok(())
}
