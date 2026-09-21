// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Every one of the state's governance tracks, carried once.
//!
//! FAZ 3 asks for each track to be exercised, and the reason is narrower than "test the
//! governance pallet": a track is a set of curves, and a curve that never admits a decision is
//! indistinguishable from one that does until somebody tries. The approval and support curves
//! are written per track, by hand, against a roll size nobody had yet — so the first time any
//! of them is asked to pass a question is the first time anybody learns whether it can.
//!
//! **What has to be true before a citizen can vote at all.** `answer_referendum` refuses an
//! account with no trust, and trust is gated absolutely on a stake: a citizen who has staked
//! nothing has no standing here whatever else they have done. The stake itself lives on another
//! chain, so it reaches People as a report. That report is the missing step between a full
//! register and a state that can decide anything, and it is a step the live chain needs too —
//! seat the offices, appoint a noter, report the stakes, and only then can a question be put.
//!
//! The report arrives here as Root over XCM, which is not a shortcut: `receive_staking_details`
//! exempts chain-authenticated submissions from the dispute window precisely because they are
//! not a personal key's word, and a relay `Transact` is exactly that case. A noter with a bond
//! is the other accepted path and the one a person would use.
//!
//! The tracks are read from the chain rather than listed here. `Referenda::Tracks` is a runtime
//! constant, so a track added later is covered without this file being edited — and a list
//! written down here would be a second place for the set of tracks to live, which is how one of
//! them ends up never exercised.

use anyhow::anyhow;
use pezkuwi_zombienet_sdk::{
	subxt::{
		dynamic::{self, At, Value},
		ext::scale_value,
		OnlineClient, PezkuwiConfig,
	},
	subxt_signer::sr25519::{dev, Keypair},
	NetworkConfig, NetworkConfigBuilder,
};

use super::state_rehearsal_offices::assign_cores;
use crate::utils::initialize_network;

const ASSET_HUB_ID: u32 = 1000;
const PEOPLE_ID: u32 = 1004;

/// Enough to clear every track's decision deposit — the largest is a hundred UNITS — with room
/// for fees and the referendum's own submission deposit.
const VOTER_FUNDING: u128 = 500_000_000_000_000;

/// A stake to report for each voter.
///
/// The amount decides how much trust the stake is worth, and trust decides the weight of a
/// reward, but not the weight of a vote: the tally is one citizen, one voice. So this only has
/// to be enough to clear zero, and making it large would rehearse a different chain from the
/// one where most citizens stake a little.
const REPORTED_STAKE: u128 = 10_000_000_000_000;

/// How long to let a referendum walk from submission to a decision.
///
/// Every period in it — prepare, decision, confirm — is compressed in a rehearsal build, so
/// this bounds a *stuck* referendum rather than a slow one. A track whose curves never admit a
/// decision hangs here rather than failing, which is the outcome worth naming.
const TRACK_SETTLE_SECS: u64 = 420;

fn account(k: &Keypair) -> Value {
	Value::unnamed_variant("Id", vec![Value::from_bytes(k.public_key().to_account_id().0)])
}

/// The origin each track decides for, by track id.
///
/// Read off `TracksInfo::track_for`, which is the only mapping that matters: submitting with an
/// origin the track does not claim is refused, and submitting with the wrong one lands the
/// question in a different lane with different curves. Root is track 0; the state's four are
/// custom origins.
fn origin_for_track(id: u16) -> Option<Value> {
	let custom = |name: &str| {
		Some(Value::unnamed_variant("Origins", vec![Value::unnamed_variant(name, vec![])]))
	};
	match id {
		0 => Some(Value::unnamed_variant("system", vec![Value::unnamed_variant("Root", vec![])])),
		40 => custom("WelatiElection"),
		41 => custom("WelatiAdmin"),
		42 => custom("CitizenshipAdmin"),
		43 => custom("QeydRules"),
		_ => None,
	}
}

/// The tracks this chain declares, as (id, name).
///
/// From the runtime constant, so the set cannot drift from what the chain actually runs.
async fn tracks_on_chain(
	api: &OnlineClient<PezkuwiConfig>,
) -> Result<Vec<(u16, String)>, anyhow::Error> {
	let addr = dynamic::constant::<Value>("Referenda", "Tracks");
	let raw: Value = api.constants().at(&addr)?;
	let mut out = Vec::new();
	if let scale_value::ValueDef::Composite(list) = &raw.value {
		for entry in list.values() {
			// Each entry is `(id, info)`; the name is a byte string inside `info`.
			let scale_value::ValueDef::Composite(pair) = &entry.value else { continue };
			let mut it = pair.values();
			let (Some(id), Some(info)) = (it.next(), it.next()) else { continue };
			let Some(id) = id.as_u128() else { continue };
			let name = info
				.at("name")
				.map(|n| format!("{n}"))
				.unwrap_or_default()
				.chars()
				.filter(|c| c.is_ascii_alphanumeric() || *c == '_')
				.collect::<String>();
			out.push((id as u16, name));
		}
	}
	if out.is_empty() {
		return Err(anyhow!(
			"Referenda::Tracks decoded to nothing. The constant is what this test derives its \
			 work from, so an empty read means the whole run would pass by doing nothing"
		));
	}
	Ok(out)
}

async fn storage(
	api: &OnlineClient<PezkuwiConfig>,
	pallet: &str,
	item: &str,
	keys: Vec<Value>,
) -> Result<Option<Value>, anyhow::Error> {
	let addr = dynamic::storage::<Vec<Value>, Value>(pallet, item);
	let at = api.storage().at_latest().await?;
	Ok(match at.try_fetch(addr, keys).await? {
		Some(v) => Some(v.decode()?),
		None => None,
	})
}

/// One referendum's state, rendered.
///
/// `Referenda::ReferendumInfoFor` is a map and needs its index. Reading it without one returns
/// nothing no matter what the chain holds, which is how this file waited four hundred and twenty
/// seconds per track for a condition it could not observe: no run has ever reported a track as
/// carrying a question. Measured 2026-09-21.
async fn referendum_state(
	api: &OnlineClient<PezkuwiConfig>,
	index: u32,
) -> Result<String, anyhow::Error> {
	Ok(storage(api, "Referenda", "ReferendumInfoFor", vec![Value::u128(index as u128)])
		.await?
		.map(|v| format!("{v}"))
		.unwrap_or_else(|| "<no such referendum>".to_string()))
}

#[tokio::test(flavor = "multi_thread")]
async fn every_governance_track_carries_a_question() -> Result<(), anyhow::Error> {
	let _ = env_logger::try_init_from_env(
		env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
	);

	let network = initialize_network(build_network_config().await?).await?;
	let relay: OnlineClient<PezkuwiConfig> =
		network.get_node("validator-01")?.wait_client().await?;
	super::state_rehearsal_offices::open_system_channels(&relay).await?;
	assign_cores(&relay).await?;
	let people: OnlineClient<PezkuwiConfig> =
		network.get_node("people-collator-01")?.wait_client().await?;

	// The genesis seats its founding citizens; they are the electorate for this test. A bigger
	// roll is `state_rehearsal`'s business — what a track needs is voters with standing, not a
	// particular number of them, and the tally's floor is what decides whether a question can
	// carry at all.
	let voters = vec![dev::alice(), dev::bob(), dev::charlie()];

	// ---- give the electorate standing ----------------------------------------------------
	//
	// Without this every `answer_referendum` below is refused for no trust, and the whole file
	// would report a governance failure that is really an economy that never started.
	for voter in &voters {
		let who = voter.clone();
		super::state_rehearsal_offices::root_call_on_people(
			&relay,
			&people,
			"StakingScore",
			"receive_staking_details",
			vec![
				Value::from_bytes(who.public_key().to_account_id().0),
				Value::unnamed_variant("RelayChain", vec![]),
				Value::u128(REPORTED_STAKE),
				Value::u128(0),
				Value::u128(0),
			],
			|| {
				let people = &people;
				let who = who.clone();
				async move { Ok(trust_of(people, &who).await? > 0) }
			},
		)
		.await
		.map_err(|e| {
			anyhow!(
				"a stake could not be reported for a voter: {e}. Trust is gated absolutely on a \
				 stake, so no citizen can vote until this lands -- and a report from Root is \
				 exempt from the dispute window by design, so a timeout here is the message not \
				 arriving rather than it waiting"
			)
		})?;
	}
	log::info!("{} voters have standing", voters.len());

	// Fund them for deposits. The submission and decision deposits are reserved, not spent, but
	// they still have to be there.
	let calls: Vec<Value> = voters
		.iter()
		.map(|k| {
			dynamic::tx(
				"Balances",
				"transfer_keep_alive",
				vec![account(k), Value::u128(VOTER_FUNDING)],
			)
			.into_value()
		})
		.collect();
	let batch = dynamic::tx("Utility", "batch_all", vec![Value::unnamed_composite(calls)]);
	people
		.tx()
		.sign_and_submit_then_watch_default(&batch, &dev::alice())
		.await?
		.wait_for_finalized_success()
		.await?;

	// ---- one question per track ----------------------------------------------------------
	let tracks = tracks_on_chain(&people).await?;
	log::info!("the chain declares {} tracks", tracks.len());
	let mut carried = Vec::new();
	let mut unreached = Vec::new();

	for (id, name) in &tracks {
		let Some(proposal_origin) = origin_for_track(*id) else {
			// A track the chain has and this file does not know how to address. Louder than a
			// skip: an unexercised track is exactly what FAZ 3 is asking about, so it fails the
			// run rather than quietly shrinking the set.
			return Err(anyhow!(
				"track {id} (`{name}`) has no origin in `origin_for_track`. A track added to the \
				 runtime and not here is a track nothing exercises, which is the gap this test \
				 exists to close -- add its origin rather than letting the set shrink"
			));
		};

		// The question itself is deliberately inert: `System::remark`. What is being rehearsed
		// is the lane, not the payload, and a proposal that changes something would make a
		// failure ambiguous between "the track cannot decide" and "the change was rejected".
		let remark = dynamic::tx("System", "remark", vec![Value::from_bytes(name.as_bytes())]);
		let encoded = {
			use pezkuwi_zombienet_sdk::subxt::tx::Payload;
			remark.encode_call_data(&people.metadata())?
		};

		let submit = dynamic::tx(
			"Referenda",
			"submit",
			vec![
				proposal_origin,
				Value::unnamed_variant("Inline", vec![Value::from_bytes(encoded)]),
				Value::unnamed_variant("After", vec![Value::u128(1)]),
			],
		);
		people
			.tx()
			.sign_and_submit_then_watch_default(&submit, &voters[0])
			.await?
			.wait_for_finalized_success()
			.await
			.map_err(|e| anyhow!("track {id} (`{name}`) refused a submission: {e}"))?;

		let next = storage(&people, "Referenda", "ReferendumCount", Vec::new())
			.await?
			.and_then(|v| v.as_u128())
			.ok_or_else(|| anyhow!("Referenda::ReferendumCount is unset"))? as u32;
		let index = next.saturating_sub(1);

		let deposit =
			dynamic::tx("Referenda", "place_decision_deposit", vec![Value::u128(index as u128)]);
		people
			.tx()
			.sign_and_submit_then_watch_default(&deposit, &voters[0])
			.await?
			.wait_for_finalized_success()
			.await
			.map_err(|e| anyhow!("track {id} (`{name}`) refused a decision deposit: {e}"))?;

		// What the chain thinks of it, before anybody votes.
		//
		// Run 21 came back `Welati::ReferendumNotOngoing` on track 40 and the message could not
		// say what state the referendum *was* in -- so the next cycle would have been spent
		// finding that out. It is logged here and carried into the error below.
		let before = referendum_state(&people, index).await?;
		log::info!("referendum {index} on track {id} (`{name}`) before voting: {before}");

		for voter in &voters {
			let vote = dynamic::tx(
				"Welati",
				"answer_referendum",
				vec![Value::u128(index as u128), Value::bool(true)],
			);
			if let Err(e) = people
				.tx()
				.sign_and_submit_then_watch_default(&vote, voter)
				.await?
				.wait_for_finalized_success()
				.await
			{
				// A referendum that has already carried is the outcome this stage is looking
				// for, not a failure to reach it.
				//
				// The votes go in one at a time, and a lenient track does not need all three:
				// `welati_election` took the first one or two, confirmed, and was approved
				// before the last voter's extrinsic landed -- which left that voter answering
				// a poll that no longer existed. Run 22 read `Ongoing` with an empty tally
				// immediately before, then `Welati::ReferendumNotOngoing` from the vote, and
				// both were true. So: ask the chain what happened, and only call it an error
				// if the referendum is still open.
				let after = referendum_state(&people, index).await?;
				if after.contains("Ongoing") {
					return Err(anyhow!(
						"a citizen with standing could not answer referendum {index} on track \
						 {id} (`{name}`): {e}. It read as {before} before the vote and {after} \
						 after, so it was open at both ends and the refusal is about the voter"
					));
				}
				log::info!(
					"referendum {index} on track {id} (`{name}`) carried before every voter had \
					 answered; it now reads {after}"
				);
				break;
			}
		}

		// Deciding or confirming is the proof the lane works. Waiting for enactment as well
		// would measure the enactment period rather than the track, and the tracks differ in
		// that period by two orders of magnitude.
		let mut moved = false;
		for _ in 0..(TRACK_SETTLE_SECS / 2) {
			let s = referendum_state(&people, index).await?;
			if s.contains("Confirming") || s.contains("Approved") || s.contains("deciding") {
				moved = true;
				break;
			}
			tokio::time::sleep(std::time::Duration::from_secs(2)).await;
		}
		if moved {
			carried.push(format!("{id}:{name}"));
			log::info!("track {id} (`{name}`) carried a question");
		} else {
			unreached.push(format!("{id}:{name}"));
		}
	}

	assert!(
		unreached.is_empty(),
		"these tracks never reached a decision: {unreached:?}. The votes were accepted, so the \
		 curves are what did not admit one -- min_approval and min_support are written per track \
		 against a roll this network may be smaller than. Carried: {carried:?}"
	);
	log::info!("all {} tracks carried a question", tracks.len());
	Ok(())
}

async fn trust_of(
	people: &OnlineClient<PezkuwiConfig>,
	who: &Keypair,
) -> Result<u128, anyhow::Error> {
	Ok(storage(
		people,
		"Trust",
		"TrustScores",
		vec![Value::from_bytes(who.public_key().to_account_id().0)],
	)
	.await?
	.and_then(|v| v.as_u128())
	.unwrap_or(0))
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
