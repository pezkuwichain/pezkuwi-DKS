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
//! This file covers the first and hardest step, and the first of the four paths with it:
//! filling the register until the population gate opens, and then following that gate across
//! to the Asset Hub. Nothing is submitted for the crossing -- `check_population_gate` runs on
//! People's own `on_initialize` and sends `pez-treasury::activate_distribution` when the roll
//! is large enough, and People's `SendXcmOrigin` converts only the three governance origins,
//! so not even the relay's sudo can forge that message. The mechanism is the only way through,
//! which is precisely what makes it worth rehearsing rather than asserting.
//!
//! The offices, the budget and the citizen's initiative are in `state_rehearsal_offices`, and
//! build on the register this leaves behind.
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

/// How long to let the gate report and the Asset Hub answer.
///
/// Not a guess about XCM's speed: both uses poll until the effect appears and only fail when
/// this runs out. What it bounds is how long a *broken* path takes to say so.
const GATE_SETTLE_SECS: u64 = 240;

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
	// Before anything else: a registered teyrchain with no core collates into the void.
	let relay: OnlineClient<PezkuwiConfig> =
		network.get_node("validator-01")?.wait_client().await?;
	super::state_rehearsal_offices::assign_cores(&relay).await?;
	super::state_rehearsal_offices::open_system_channels(&relay).await?;
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

		// Applicants apply in parallel, vouchers approve in sequence *per voucher*.
		//
		// The first version waited for finalisation on every one of three extrinsics per
		// applicant, one applicant at a time: ninety-five of them is two hundred and eighty-five
		// round trips of about twelve seconds, which is the best part of an hour and is why this
		// stage was the one that ran out of its twenty-minute ceiling on 2026-09-19 while the
		// other five finished.
		//
		// Applicants are independent signers, so their applications can all be in flight at
		// once. Vouchers are not: one voucher approves five applicants and those five share a
		// nonce, so they are grouped and each group runs in order while the groups run beside
		// each other. Submitting a voucher's five at once would have them race for one nonce and
		// four would be dropped as duplicates -- a failure that reads like the chain refusing
		// the approval.
		let base = admitted - this_generation.len();
		let applies = this_generation.iter().enumerate().map(|(i, (applicant, voucher))| {
			let mut hash = [0u8; 32];
			hash[..4].copy_from_slice(&((base + i) as u32).to_le_bytes());
			hash[4..8].copy_from_slice(b"rhsl");
			let apply = pezkuwi_zombienet_sdk::subxt::dynamic::tx(
				"IdentityKyc",
				"apply_for_citizenship",
				vec![
					Value::from_bytes(hash),
					// `Option<AccountId>`, not `Option<MultiAddress>`: thirty-two bytes with no
					// `Id` wrapper, because this pallet takes the account directly.
					Value::unnamed_variant(
						"Some",
						vec![Value::from_bytes(voucher.public_key().to_account_id().0)],
					),
					Value::unnamed_variant("None", vec![]),
				],
			);
			let people = &people;
			let applicant = applicant.clone();
			async move {
				people
					.tx()
					.sign_and_submit_then_watch_default(&apply, &applicant)
					.await?
					.wait_for_finalized_success()
					.await
					.map_err(|e| anyhow!("apply_for_citizenship: {e}"))?;
				Ok::<_, anyhow::Error>(())
			}
		});
		futures::future::try_join_all(applies).await?;

		// Group by voucher so each voucher's approvals keep their order.
		let mut by_voucher: std::collections::BTreeMap<[u8; 32], Vec<_>> = Default::default();
		for (applicant, voucher) in &this_generation {
			by_voucher
				.entry(voucher.public_key().to_account_id().0)
				.or_default()
				.push((applicant.clone(), voucher.clone()));
		}
		let approvals = by_voucher.into_values().map(|group| {
			let people = &people;
			async move {
				for (applicant, voucher) in group {
					let approve = pezkuwi_zombienet_sdk::subxt::dynamic::tx(
						"IdentityKyc",
						"approve_referral",
						vec![Value::from_bytes(applicant.public_key().to_account_id().0)],
					);
					// The one failure worth naming. A node built without `fast-runtime` runs
					// this perfectly well and simply waits: the vouching period is a day, so the
					// second generation is refused with `VouchingTooSoon` and the run would sit
					// there until something killed it. A timeout carries no information -- it
					// looks the same whether the flag was missing, the network never came up, or
					// the register is genuinely stuck.
					if let Err(e) = people
						.tx()
						.sign_and_submit_then_watch_default(&approve, &voucher)
						.await?
						.wait_for_finalized_success()
						.await
					{
						let msg = e.to_string();
						if msg.contains("VouchingTooSoon") {
							return Err(anyhow!(
								"vouching refused as too soon at a generation boundary -- the \
								 nodes were built without `fast-runtime`, so the waiting period \
								 is a day rather than two blocks and this run cannot finish: \
								 {msg}"
							));
						}
						return Err(anyhow!("approve_referral failed: {msg}"));
					}
				}
				Ok::<_, anyhow::Error>(())
			}
		});
		futures::future::try_join_all(approvals).await?;

		// Confirmations are the applicant's own again, so they go back in parallel.
		let confirms = this_generation.iter().map(|(applicant, _)| {
			let confirm = pezkuwi_zombienet_sdk::subxt::dynamic::tx(
				"IdentityKyc",
				"confirm_citizenship",
				Vec::<Value>::new(),
			);
			let people = &people;
			let applicant = applicant.clone();
			async move {
				people
					.tx()
					.sign_and_submit_then_watch_default(&confirm, &applicant)
					.await?
					.wait_for_finalized_success()
					.await
					.map_err(|e| anyhow!("confirm_citizenship: {e}"))?;
				Ok::<_, anyhow::Error>(())
			}
		});
		futures::future::try_join_all(confirms).await?;

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

	// ---- and the gate opens ------------------------------------------------------------
	//
	// Filling the register was never the point on its own; it is the precondition for the
	// first of the four cross-chain paths, and until this file asserted it the test's own name
	// promised more than it delivered.
	//
	// Nothing is submitted here. `check_population_gate` runs on the People chain's own
	// `on_initialize`, compares the roll against `min(PopulationThreshold, override)`, and if
	// it has been reached sends the Asset Hub a `Transact` carrying
	// `pez-treasury::activate_distribution`. So the whole path is the chain acting on its own
	// -- which is exactly why it is worth rehearsing: nobody can make it happen by hand.
	// People's `SendXcmOrigin` converts only the three governance origins, and not Root, so
	// even the relay's sudo cannot forge this message. The mechanism is the only way through.
	log::info!("waiting for the population gate to report");
	let reported = wait_for(&people, "Welati", "PopulationGateReported", GATE_SETTLE_SECS, |v| {
		format!("{v}").contains("true")
	})
	.await;
	if !reported {
		return Err(anyhow!(
			"the roll reached {roll} but `PopulationGateReported` never turned true within \
			 {GATE_SETTLE_SECS}s. The check runs every `PopulationCheckPeriod` -- a day of \
			 blocks unless the node was built with `fast-runtime`, which is the usual cause. \
			 If the flag is set and this still fails, look for `PopulationReportFailed`: the \
			 send is deliberately not latched, so a closed channel retries rather than sticking"
		));
	}

	// The proof is on the other chain. A report that never arrived leaves People looking
	// perfectly healthy, so asking People whether it sent something is not evidence that
	// anything was received.
	let asset_hub: OnlineClient<PezkuwiConfig> =
		network.get_node("asset-hub-collator-01")?.wait_client().await?;
	log::info!("waiting for the Asset Hub to activate distribution");
	let started =
		wait_for(&asset_hub, "PezTreasury", "DistributionStarted", GATE_SETTLE_SECS, |v| {
			format!("{v}").contains("true")
		})
		.await;
	assert!(
		started,
		"People reported the threshold but the Asset Hub never set `DistributionStarted`. The \
		 message was accepted for delivery, so this is execution on the far side: check the \
		 HRMP channel, and check that `TreasuryPalletIndex`/`ACTIVATE_DISTRIBUTION_CALL_INDEX` \
		 still address `pez-treasury::activate_distribution` (`check-cross-chain-call-addresses.py`)"
	);

	log::info!("path 1 carried: register {roll} -> gate reported -> distribution active");

	// ---- and now the three paths that needed this one to have happened --------------------
	//
	// Paths 3 and 4 used to raise their own network and then stand on a precondition they had
	// no way to arrange: only the register reaching the gate starts distribution, and a genesis
	// roll of five never gets there. They belong here, on the one network where the gate has
	// actually opened.
	//
	// The bench is seated first because the payroll pays *seated members*; the founding hand
	// that seats it is named in this chain's genesis, not granted by anything running.
	//
	// The whole house, not a token five, and the reason is arithmetic rather than thoroughness.
	// `get_voting_threshold` counts against `ParliamentSize` -- the constant, two hundred and
	// one -- never against the number of people sitting, so a simple majority is a hundred and
	// one ayes whatever the bench holds. A founding house of five can be seated, can open a
	// proposal, can vote on it, and can never carry one. Measured 2026-09-19: the budget stage
	// did exactly that, five ayes, and `finalize_proposal` refused. The pallet says so in its
	// own words -- "a founding house of twenty cannot pass anything a house of two hundred and
	// one could not" -- so the rehearsal seats the house the state actually starts with, which
	// is also what the mainnet founding sequence does.
	//
	// Seating asks for no citizenship and voting asks only for a seat, so the rest are plain
	// derived keys: one call to seat them, and a majority of them to carry a question.
	let house = super::state_rehearsal_offices::founding_house()?;
	let bench = super::state_rehearsal_offices::founding_bench();
	let serok = bench[0].clone();
	// Read the count out before the closure: `settled` is an `Fn`, so anything it touches has to
	// outlive every call, and `house` is voted with afterwards.
	let want = house.len();
	let members: Vec<Value> = house
		.iter()
		.map(|k| Value::from_bytes(k.public_key().to_account_id().0))
		.collect();
	super::state_rehearsal_offices::office_call_on_people(
		&people,
		&serok,
		"Welati",
		"seat_founding_parliament",
		vec![Value::unnamed_composite(members)],
		|| {
			let people = &people;
			async move {
				Ok(super::state_rehearsal_offices::bench_size(people, "ParliamentMembers").await?
					>= want)
			}
		},
	)
	.await?;

	// Path 2 before paths 3 and 4: a budget is the first thing a seated house does, and the
	// government pot it draws on is filled by the same release the payroll reports.
	super::state_rehearsal_offices::a_budget_is_voted_and_the_treasurer_spends_it(
		&people, &asset_hub, &house, &bench,
	)
	.await?;

	super::state_rehearsal_offices::the_treasury_funds_the_payroll_and_the_payroll_pays_across(
		&people, &asset_hub, &bench,
	)
	.await?;

	Ok(())
}

/// Poll one storage item until it satisfies `done`, or give up.
///
/// Every cross-chain assertion here needs the same shape, and the shape matters: the effect
/// lands in a later block on another chain, so a single read after a sleep is a guess about
/// how long that takes. Returning a bool rather than asserting lets each caller say what its
/// own failure means -- a timeout on the gate and a timeout on the Asset Hub point at
/// completely different things.
pub(crate) async fn wait_for<F>(
	api: &OnlineClient<PezkuwiConfig>,
	pallet: &str,
	item: &str,
	secs: u64,
	done: F,
) -> bool
where
	F: Fn(&Value) -> bool,
{
	let addr = pezkuwi_zombienet_sdk::subxt::dynamic::storage::<Vec<Value>, Value>(pallet, item);
	for _ in 0..(secs / 6) {
		if let Ok(at) = api.storage().at_latest().await {
			if let Ok(Some(raw)) = at.try_fetch(addr.clone(), Vec::new()).await {
				if let Ok(v) = raw.decode() {
					if done(&v) {
						return true;
					}
				}
			}
		}
		tokio::time::sleep(std::time::Duration::from_secs(6)).await;
	}
	false
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
