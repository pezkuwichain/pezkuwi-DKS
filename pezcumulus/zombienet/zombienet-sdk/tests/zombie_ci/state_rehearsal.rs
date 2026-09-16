// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The state coming into being, on a network raised for the purpose.
//!
//! FAZ 3 asks for four cross-chain paths carried live, every governance track exercised once, a
//! citizen initiative start to finish, and nothing done by sudo that governance is meant to do.
//! None of that can be rehearsed on the live Zagros: its slowest track runs two hundred and
//! seventeen days, every attempt is unrepeatable, and a mistake leaves a mark on a chain other
//! people are using.
//!
//! So it is rehearsed here, on a network that is raised and torn down inside a test run. What
//! makes it *our* rehearsal rather than a generic one is that it runs the Zagros runtimes with
//! Zagros's own preset -- the escrow seeded, the pots in place, the founding citizens written.
//! A `-local` chain without those could not even reproduce the teleport defect that cost the
//! first genesis, which is why the preset was fixed rather than worked around.
//!
//! This file covers the first and hardest step: filling the register until the population gate
//! opens, which is what sends the People chain's first message to the Asset Hub. The remaining
//! steps -- the election, the offices, the Dîwan, a budget and a spend -- build on the register
//! this leaves behind and are their own tests.
//!
//! Two things it deliberately does not do. It never uses sudo: `Tiki::grant_honorary_citizenship`
//! accepts Root and the relay reaches this chain as Root, so a hundred citizens could be
//! conjured in one batch -- and a register filled that way never exercises the route the forty
//! million will take. And it never asserts on its own loop: the number that matters is the one
//! the gate reads, so it is read off the chain at the end.

use anyhow::anyhow;
use pezkuwi_zombienet_sdk::{
	subxt::{dynamic::Value, OnlineClient, PezkuwiConfig},
	subxt_signer::{sr25519::dev, SecretUri},
	NetworkConfig, NetworkConfigBuilder,
};
use std::str::FromStr;

use crate::utils::initialize_network;

const ASSET_HUB_ID: u32 = 1000;
const PEOPLE_ID: u32 = 1004;

/// The roll the gate opens at, on a chain built for rehearsal.
///
/// A hundred here and a hundred thousand on the mainnet, and the two are the same decision: the
/// gate is set to the electorate floor, so the roll that can carry a question is the roll that
/// opens the treasury. Scaling one without the other is what broke the testnet before.
const POPULATION_GATE: u32 = 100;

/// What each applicant needs on hand.
///
/// The deposit is reserved at apply and returned at confirm, so a citizen costs only fees -- but
/// the deposit still has to *be there*, and an account that cannot reserve it fails with an
/// error about funds that reads like the wrong account was used.
const FUND_PER_CITIZEN: u128 = 2_000_000_000_000;

/// Derive the cohort from one phrase by path, so a rerun addresses the same accounts.
fn cohort(
	n: u32,
) -> Result<Vec<pezkuwi_zombienet_sdk::subxt_signer::sr25519::Keypair>, anyhow::Error> {
	// `//Alice` is the phrase every well-known key hangs off; deriving further from it keeps the
	// cohort reproducible without introducing a secret this repository would then be holding.
	(1..=n)
		.map(|i| {
			let uri = SecretUri::from_str(&format!("//Alice//citizen//{i}"))
				.map_err(|e| anyhow!("citizen {i}: {e}"))?;
			pezkuwi_zombienet_sdk::subxt_signer::sr25519::Keypair::from_uri(&uri)
				.map_err(|e| anyhow!("citizen {i}: {e}"))
		})
		.collect()
}

async fn citizen_count(api: &OnlineClient<PezkuwiConfig>) -> Result<u32, anyhow::Error> {
	let addr = pezkuwi_zombienet_sdk::subxt::dynamic::storage::<Vec<Value>, Value>(
		"IdentityKyc",
		"CitizenCount",
	);
	let v = api
		.storage()
		.at_latest()
		.await?
		.try_fetch(addr, Vec::new())
		.await?
		.ok_or_else(|| anyhow!("IdentityKyc::CitizenCount is not set"))?
		.decode()?;
	v.as_u128()
		.map(|n| n as u32)
		.ok_or_else(|| anyhow!("CitizenCount did not decode as a number"))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_register_fills_and_the_population_gate_opens() -> Result<(), anyhow::Error> {
	let _ = env_logger::try_init_from_env(
		env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
	);

	log::info!("Spawning relay + Asset Hub + People");
	let network = initialize_network(build_network_config().await?).await?;
	let people: OnlineClient<PezkuwiConfig> =
		network.get_node("people-collator-01")?.wait_client().await?;

	let start = citizen_count(&people).await?;
	log::info!("register starts at {start}");
	assert!(start > 0, "the genesis must seat founding citizens or nobody can vouch");

	// The founding generation is exempt from the waiting period -- `CitizenSince` is zero for
	// them -- but not from the vouching capacity, so the register grows in generations: each
	// citizen may vouch for a handful, waits, and may vouch again. That shape is the anti-sybil
	// design and walking it is the point; short-cutting it would rehearse nothing.
	let needed = POPULATION_GATE.saturating_sub(start);
	let cohort = cohort(needed)?;
	log::info!("{needed} applicants to bring the roll to {POPULATION_GATE}");

	// Fund them from Alice, whose endowment the preset provides.
	for chunk in cohort.chunks(25) {
		let calls: Vec<Value> = chunk
			.iter()
			.map(|k| {
				pezkuwi_zombienet_sdk::subxt::dynamic::tx(
					"Balances",
					"transfer_keep_alive",
					vec![
						Value::unnamed_variant(
							"Id",
							vec![Value::from_bytes(k.public_key().to_account_id().0)],
						),
						Value::u128(FUND_PER_CITIZEN),
					],
				)
				.into_value()
			})
			.collect();
		let batch = pezkuwi_zombienet_sdk::subxt::dynamic::tx(
			"Utility",
			"batch_all",
			vec![Value::unnamed_composite(calls)],
		);
		people
			.tx()
			.sign_and_submit_then_watch_default(&batch, &dev::alice())
			.await?
			.wait_for_finalized_success()
			.await?;
	}
	log::info!("cohort funded");

	// Vouchers available now: the founding citizens. As each generation confirms, it becomes the
	// next generation's vouchers -- which is how a register of two grows to a hundred without
	// anybody being exempt from anything.
	let mut vouchers: Vec<pezkuwi_zombienet_sdk::subxt_signer::sr25519::Keypair> =
		vec![dev::alice(), dev::bob()];
	let mut admitted = 0usize;

	while admitted < cohort.len() {
		let mut this_generation = Vec::new();
		for voucher in &vouchers {
			for _ in 0..5 {
				// `InitialVouchingCapacity`
				if admitted >= cohort.len() {
					break;
				}
				this_generation.push((cohort[admitted].clone(), voucher.clone()));
				admitted += 1;
			}
		}
		if this_generation.is_empty() {
			return Err(anyhow!(
				"no vouching capacity left with {admitted} of {} admitted",
				cohort.len()
			));
		}
		log::info!("generation of {} applicants", this_generation.len());

		for (i, (applicant, voucher)) in this_generation.iter().enumerate() {
			let mut hash = [0u8; 32];
			hash[..4]
				.copy_from_slice(&((admitted - this_generation.len() + i) as u32).to_le_bytes());
			hash[4..8].copy_from_slice(b"rhsl");
			let apply = pezkuwi_zombienet_sdk::subxt::dynamic::tx(
				"IdentityKyc",
				"apply_for_citizenship",
				vec![
					Value::from_bytes(hash),
					Value::unnamed_variant(
						"Some",
						vec![Value::unnamed_variant(
							"Id",
							vec![Value::from_bytes(voucher.public_key().to_account_id().0)],
						)],
					),
					Value::unnamed_variant("None", vec![]),
				],
			);
			people
				.tx()
				.sign_and_submit_then_watch_default(&apply, applicant)
				.await?
				.wait_for_finalized_success()
				.await?;

			let approve = pezkuwi_zombienet_sdk::subxt::dynamic::tx(
				"IdentityKyc",
				"approve_referral",
				vec![Value::unnamed_variant(
					"Id",
					vec![Value::from_bytes(applicant.public_key().to_account_id().0)],
				)],
			);
			// The one failure worth naming. A node built without `fast-runtime` runs this test
			// perfectly well and simply waits: the vouching period is a day, so the second
			// generation is refused with `VouchingTooSoon` and, if that were retried, the run
			// would sit there until something killed it. A timeout carries no information --
			// it looks the same whether the flag was missing, the network never came up, or
			// the register is genuinely stuck. Saying which turns three silences into one
			// sentence.
			if let Err(e) = people
				.tx()
				.sign_and_submit_then_watch_default(&approve, voucher)
				.await?
				.wait_for_finalized_success()
				.await
			{
				let msg = e.to_string();
				if msg.contains("VouchingTooSoon") {
					return Err(anyhow!(
						"vouching refused as too soon at generation boundary -- the nodes were \
						 built without `fast-runtime`, so the waiting period is a day rather \
						 than two blocks and this run cannot finish: {msg}"
					));
				}
				return Err(anyhow!("approve_referral failed: {msg}"));
			}

			let confirm = pezkuwi_zombienet_sdk::subxt::dynamic::tx(
				"IdentityKyc",
				"confirm_citizenship",
				Vec::<Value>::new(),
			);
			people
				.tx()
				.sign_and_submit_then_watch_default(&confirm, applicant)
				.await?
				.wait_for_finalized_success()
				.await?;
		}

		// The generation just admitted becomes the next one's vouchers, after the waiting
		// period -- two blocks in a rehearsal build, a day in production.
		vouchers = this_generation.into_iter().map(|(a, _)| a).collect();
		tokio::time::sleep(std::time::Duration::from_secs(30)).await;
	}

	let roll = citizen_count(&people).await?;
	log::info!("register holds {roll}");
	assert!(
		roll >= POPULATION_GATE,
		"the roll is {roll}, the gate opens at {POPULATION_GATE} -- the register did not fill"
	);

	Ok(())
}

async fn build_network_config() -> Result<NetworkConfig, anyhow::Error> {
	let images = pezkuwi_zombienet_sdk::environment::get_images_from_env();
	NetworkConfigBuilder::new()
		.with_relaychain(|r| {
			r.with_chain("zagros-local")
				.with_default_command("pezkuwi")
				.with_default_image(images.pezkuwi())
				.with_default_args(vec!["-lruntime=info".into()])
				.with_validator(|n| n.with_name("validator-01"))
				.with_validator(|n| n.with_name("validator-02"))
		})
		.with_teyrchain(|p| {
			p.with_id(ASSET_HUB_ID)
				.with_chain("asset-hub-zagros-local")
				.with_default_command("pezkuwi-teyrchain")
				.with_default_image(images.pezcumulus())
				.with_collator(|n| n.with_name("asset-hub-collator-01"))
		})
		.with_teyrchain(|p| {
			p.with_id(PEOPLE_ID)
				.with_chain("people-zagros-local")
				.with_default_command("pezkuwi-teyrchain")
				.with_default_image(images.pezcumulus())
				.with_collator(|n| n.with_name("people-collator-01"))
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
