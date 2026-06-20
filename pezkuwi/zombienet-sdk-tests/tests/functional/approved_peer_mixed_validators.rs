// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Test what happens with a mixed fleet of validators (new ones which allow the ApprovedPeer
// UMPSignal and old ones which consider these invalid during backing). Collators that don't send
// these signals should not be affected, while collators sending the signal will see degraded
// backing throughput. No disputes should be raised and finality is not affected.

use anyhow::anyhow;
use tokio::time::Duration;

use pezcumulus_zombienet_sdk_helpers::{assert_finality_lag, assert_para_throughput};
use pezkuwi_primitives::Id as ParaId;
use pezkuwi_zombienet_orchestrator::network::node::LogLineCountOptions;
use pezkuwi_zombienet_sdk::{
	subxt::{OnlineClient, PezkuwiConfig},
	NetworkConfigBuilder,
};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn approved_peer_mixed_validators_test() -> Result<(), anyhow::Error> {
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
							"relay_vrf_modulo_samples": 2,
							"scheduler_params": {
								"group_rotation_frequency": 4,
								"max_validators_per_core": 5,
							}
						}
					}
				}))
				.with_validator(|node| node.with_name("validator-0"));

			let r = (1..7).fold(r, |acc, i| {
				acc.with_validator(|node| node.with_name(&format!("validator-{i}")))
			});

			(7..10).fold(r, |acc, i| {
				acc.with_validator(|node| {
					node.with_name(&format!("old-validator-{i}"))
						.with_image(
							std::env::var("OLD_PEZKUWI_IMAGE")
								.expect("OLD_PEZKUWI_IMAGE needs to be set")
								.as_str(),
						)
						.with_command(
							std::env::var("OLD_PEZKUWI_COMMAND")
								.unwrap_or(String::from("pezkuwi"))
								.as_str(),
						)
				})
			})
		})
		// Teyrchain 2000 is sending the ApprovedPeer ump signal.
		.with_teyrchain(|p| {
			p.with_id(2000)
				.with_default_command("undying-collator")
				.with_default_image(
					std::env::var("COL_IMAGE")
						.unwrap_or("docker.io/paritypr/colander:latest".to_string())
						.as_str(),
				)
				.pezcumulus_based(false)
				.with_default_args(vec![
					("-lteyrchain=debug").into(),
					("--experimental-send-approved-peer").into(),
				])
				.with_collator(|n| n.with_name("collator-2000"))
		})
		// Teyrchain 2001 is NOT sending the ApprovedPeer ump signal.
		.with_teyrchain(|p| {
			p.with_id(2001)
				.with_default_command("undying-collator")
				.with_default_image(
					std::env::var("COL_IMAGE")
						.unwrap_or("docker.io/paritypr/colander:latest".to_string())
						.as_str(),
				)
				.pezcumulus_based(false)
				.with_default_args(vec![("-lteyrchain=debug").into()])
				.with_collator(|n| n.with_name("collator-2001"))
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

	// The min throughput for para 2000 is going to be lower, but it depends on how the old
	// validators are distributed into backing groups.
	assert_para_throughput(
		&relay_client,
		15,
		[(ParaId::from(2000), 6..15), (ParaId::from(2001), 11..16)]
			.into_iter()
			.collect(),
	)
	.await?;

	assert_finality_lag(&relay_node.wait_client().await?, 5).await?;

	let old_relay_node = network.get_node("old-validator-9")?;
	let result = old_relay_node
		.wait_log_line_count_with_timeout(
			"Validation yielded an invalid candidate",
			false,
			LogLineCountOptions::new(|n| n == 1, Duration::from_secs(1), false),
		)
		.await?;
	assert!(result.success());

	// Check that no disputes are raised.
	assert!(relay_node
		.wait_metric_with_timeout("pezkuwi_teyrchain_candidate_disputes_total", |v| v > 0.0, 30u64)
		.await
		.is_err());

	relay_node
		.assert("pezkuwi_teyrchain_approval_checking_finality_lag", 0.0)
		.await?;

	relay_node.assert("pezkuwi_teyrchain_disputes_finality_lag", 0.0).await?;

	log::info!("Test finished successfully");

	Ok(())
}
