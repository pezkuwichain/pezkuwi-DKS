// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The offices being filled, on a network raised for the purpose.
//!
//! `state_rehearsal` fills the register until the population gate opens. This file carries what
//! the register makes possible: a Parliament seated, a court on its bench, a Prime Minister
//! named and confirmed, a finance portfolio handed out, a budget voted and spent, and a
//! citizen's initiative carried from one signature to a live referendum.
//!
//! **Why the first offices are not won.** Standing for office needs a trust score -- a hundred
//! out of a thousand for a seat, two hundred and fifty for the presidency -- and trust is gated
//! absolutely on having something staked, then weighted across education, referrals and offices
//! held. On the day a chain starts nobody has any of that, so the first Parliament cannot be
//! elected: it is seated, and the pallet makes its mandate temporary by construction.
//!
//! **The founding hand lives on this chain.** The presidency is not granted here: genesis seats
//! `Tiki::Serok` through `TikiConfig::founding_government`, because nothing running can grant it
//! afterwards. From there the President seats the founding Parliament, qualifies and appoints the
//! court, and nominates a Prime Minister whom Parliament -- and deliberately not he -- confirms.
//!
//! An earlier version of this file drove all of that from the relay's sudo, through
//! `XcmPallet::send` and `Transact{Superuser}`. Every call of it was dropped on arrival and the
//! runs died waiting: `TheRegisterIsNotWritableFromAbroad` refuses `Welati`, `Tiki`, `Diwan`,
//! `Parliament`, `Trust` and `IdentityKyc` before the origin is even resolved, so the relay's
//! sudo is exactly the hand it takes away. Worse, it refuses them *quietly* -- a filtered
//! `Transact` does not fail the relay extrinsic that carried it, so the send reported success
//! and the effect never came. Three comments in the tree said the relay's sudo was the founding
//! hand; the filter had said otherwise since the day it was written.
//!
//! So this rehearses what the live chain can actually do, which is the only thing worth
//! rehearsing: an office named at genesis, signing on the chain whose register it writes.
//!
//! Nothing here asserts on its own bookkeeping. Every stage reads back the storage the *chain*
//! keeps and compares against that, because a test that counts its own loop only proves the
//! loop ran.

use anyhow::anyhow;
use pezkuwi_zombienet_sdk::{
	subxt::{
		// `Hasher` is the trait behind `client.hasher()`; `parliament_decides` needs it in scope
		// to hash the encoded call into the proposal hash the collective stores.
		config::Hasher,
		dynamic::{self, At, Value},
		ext::scale_value,
		tx::{DynamicPayload, Payload},
		OnlineClient,
		PezkuwiConfig,
	},
	subxt_signer::sr25519::{dev, Keypair},
	NetworkConfig, NetworkConfigBuilder,
};

use super::state_rehearsal::wait_for;
use crate::utils::initialize_network;

const ASSET_HUB_ID: u32 = 1000;
const PEOPLE_ID: u32 = 1004;

/// The founding bench. Small on purpose: `ParliamentSize` is a ceiling, not a quorum, and
/// `seat_founding_parliament` takes whoever it is given. Five is enough for a majority to be a
/// real count rather than a single vote deciding everything.
const FOUNDING_MEMBERS: usize = 5;

/// `WelatiParliamentSize` on both twins. Written here rather than read from the chain because a
/// rehearsal that adapts to whatever the chain says cannot notice the chain saying the wrong
/// thing: if this ever disagrees with the runtime, the seating call fails and that is the report
/// worth having.
const PARLIAMENT_SIZE: u32 = 201;

/// Ayes a simple majority needs: `ParliamentSize / 2 + 1`, counted against the constant and not
/// against the number of people sitting.
const SIMPLE_MAJORITY: usize = (PARLIAMENT_SIZE as usize) / 2 + 1;

/// How many extrinsics this rehearsal sends at one node at once.
///
/// Not a throughput knob: every in-flight submission holds an RPC subscription open until its
/// extrinsic finalises, and past that point the node answers `Request timeout` rather than
/// anything about the chain. Twenty-five was already the figure `fund_seats` used; run 20 showed
/// the votes needed it too.
const VOTE_CHUNK: usize = 25;

/// Ten HEZ a seat, out of the founder's account, decided by Serok on 2026-09-19.
///
/// A fee budget rather than an endowment: a vote, a proposal and the existential deposit, with
/// room to spare and nothing that could be mistaken for a stake. It comes from the founder
/// because on this chain the founder is the only account that holds anything at genesis -- the
/// same place the register's cohort is funded from, and the same place mainnet's launch capital
/// sits.
const FUND_PER_SEAT: u128 = 10_000_000_000_000;

/// `SIMPLE_MAJORITY` for callers outside this module.
pub(crate) fn simple_majority() -> usize {
	SIMPLE_MAJORITY
}

/// How long to wait for a message the relay sends to arrive and execute on People.
///
/// Not a guess about how fast XCM is: every use of it polls storage until the effect appears
/// and only fails when this runs out, so a slow lane costs seconds and a broken one is still
/// reported as a broken lane rather than as a timeout.
const XCM_SETTLE_SECS: u64 = 180;

/// How long to wait for a call signed *on People* to show its effect in People's storage.
///
/// Shorter than the XCM figure and for a different reason: there is no lane to cross, so what
/// is being waited on is block production and nothing else. Kept as its own constant rather
/// than shared, because a timeout here and a timeout on `XCM_SETTLE_SECS` point at completely
/// different faults and a single number would hide which one happened.
const SETTLE_SECS: u64 = 90;

/// PEZ on the Asset Hub. The governance token is an asset there, not a native balance, so a
/// spend is read out of `Assets::Account` rather than `System::Account`.
const PEZ_ASSET_ID: u32 = 1;

/// Longer than the XCM budget, because this one waits for a period rather than a message: an
/// epoch closes on People's own clock, compressed in a rehearsal build but still several
/// blocks away.
const EPOCH_SETTLE_SECS: u64 = 300;

// ---------------------------------------------------------------------------------------
// Sending Root into People
// ---------------------------------------------------------------------------------------

/// SCALE-encode a People call against *People's own* metadata.
///
/// The pallet and call indices are deliberately not written down here. They are built by hand
/// elsewhere in this repository and that is a standing hazard -- a renumbered pallet changes
/// the meaning of a constant nobody edited, and nothing fails until the wrong extrinsic runs.
/// The client is already connected to the chain that will execute this, so its metadata is the
/// authority, and a renamed call fails here with the name in the message.
fn encode_people_call(
	people: &OnlineClient<PezkuwiConfig>,
	pallet: &str,
	call: &str,
	fields: Vec<Value>,
) -> Result<Vec<u8>, anyhow::Error> {
	let payload = dynamic::tx(pallet, call, fields);
	payload
		.encode_call_data(&people.metadata())
		.map_err(|e| anyhow!("encoding {pallet}::{call} against People's metadata: {e}"))
}

/// Wrap an encoded People call in `sudo(xcmPallet.send(People, Transact(Superuser)))`.
///
/// `UnpaidExecution` first: the relay is a location People waives fees for, and without it the
/// message is dropped for want of an asset to buy execution with -- silently, because a dropped
/// message is not a failed extrinsic. The relay's `send` succeeds either way, so the proof that
/// this worked is always the storage read at the other end.
fn relay_root_into_people(encoded_call: Vec<u8>) -> DynamicPayload {
	let dest = Value::unnamed_variant(
		"V5",
		vec![Value::named_composite([
			("parents", Value::u128(0)),
			(
				"interior",
				Value::unnamed_variant(
					"X1",
					vec![Value::unnamed_composite(vec![Value::unnamed_variant(
						"Teyrchain",
						vec![Value::u128(PEOPLE_ID as u128)],
					)])],
				),
			),
		])],
	);

	let message = Value::unnamed_variant(
		"V5",
		vec![Value::unnamed_composite(vec![
			Value::named_variant(
				"UnpaidExecution",
				[
					("weight_limit", Value::unnamed_variant("Unlimited", vec![])),
					("check_origin", Value::unnamed_variant("None", vec![])),
				],
			),
			Value::named_variant(
				"Transact",
				[
					("origin_kind", Value::unnamed_variant("Superuser", vec![])),
					// v5 carries a *fallback* weight, used only by a peer too old to weigh the
					// call itself. Generous on purpose: too small silently under-weighs the
					// call on exactly the path this is meant to prove.
					(
						"fallback_max_weight",
						Value::unnamed_variant(
							"Some",
							vec![Value::named_composite([
								("ref_time", Value::u128(10_000_000_000u128)),
								("proof_size", Value::u128(1_000_000u128)),
							])],
						),
					),
					("call", Value::from_bytes(encoded_call)),
				],
			),
		])],
	);

	let send = dynamic::tx("XcmPallet", "send", vec![dest, message]);
	dynamic::tx(
		"Sudo",
		"sudo_unchecked_weight",
		vec![
			send.into_value(),
			Value::named_composite([
				("ref_time", Value::u128(1u128)),
				("proof_size", Value::u128(1u128)),
			]),
		],
	)
}

/// Give each teyrchain a core, without which it is registered and never scheduled.
///
/// The relay runs agile coretime: `Coretime` and `OnDemandAssignmentProvider`, not the legacy
/// fixed mapping. Registration puts a para in `ParaLifecycles` as `Parachain` and gives it a
/// code hash, and that is where the genesis stops -- a core is assigned afterwards, normally by
/// the coretime chain over XCM. A rehearsal network has no coretime chain, so nothing ever
/// assigns one and the para sits there collating into the void.
///
/// Measured 2026-09-19, and none of it was visible from inside a test: the relay was healthy at
/// 159 blocks and fifteen epochs, both paras were `Parachain` with their code present, the
/// collator said "Is collating: yes" and proposed blocks 1, 2, 3 -- over and over from the same
/// parent, because none of them was ever backed. The only line that named anything was the
/// collator's database finally refusing the pile: "Too many sibling blocks at #1 inserted".
///
/// Open the HRMP channels the two system teyrchains talk over.
///
/// Without these the register can fill, the gate can fire, the pallet can build its message --
/// and the send returns `Transport("NoChannel")` every time. Measured 2026-09-19: the roll
/// reached a hundred, the gate fired on schedule, and thirty-nine consecutive reports were
/// dropped at the transport for want of a lane. Nothing in the runtimes was wrong; there was
/// simply no road. The report is deliberately not latched, which is why it kept trying rather
/// than sticking -- and why the only symptom was a flag that never turned true.
///
/// `establish_system_channel` rather than `force_open_hrmp_channel`: it takes any signed origin
/// provided both ends are system chains, which 1000 and 1004 are, so this rehearses the call a
/// launch would actually make instead of reaching for sudo.
///
/// Both directions, because a channel is one-way. People sends the population report to the hub;
/// the hub sends the release report back.
pub(crate) async fn open_system_channels(
	relay: &OnlineClient<PezkuwiConfig>,
) -> Result<(), anyhow::Error> {
	for (from, to) in [(PEOPLE_ID, ASSET_HUB_ID), (ASSET_HUB_ID, PEOPLE_ID)] {
		let tx = dynamic::tx(
			"Hrmp",
			"establish_system_channel",
			vec![Value::u128(from as u128), Value::u128(to as u128)],
		);
		match relay
			.tx()
			.sign_and_submit_then_watch_default(&tx, &dev::alice())
			.await?
			.wait_for_finalized_success()
			.await
		{
			Ok(_) => log::info!("hrmp channel {from} -> {to} open"),
			// An already-open channel is the state this wants, not a failure. Anything else is.
			Err(e) if format!("{e}").contains("ChannelAlreadyExists") => {
				log::info!("hrmp channel {from} -> {to} was already open")
			},
			Err(e) => return Err(anyhow!("could not open the hrmp channel {from} -> {to}: {e}")),
		}
	}
	Ok(())
}

/// `assign_core` takes Root or the broker para, so the relay's sudo can do it directly. 57600
/// is the whole of a core; the assignment runs from block zero with no end.
pub(crate) async fn assign_cores(relay: &OnlineClient<PezkuwiConfig>) -> Result<(), anyhow::Error> {
	for (core, para) in [(0u32, ASSET_HUB_ID), (1u32, PEOPLE_ID)] {
		let assign = dynamic::tx(
			"Coretime",
			"assign_core",
			vec![
				Value::u128(core as u128),
				Value::u128(0),
				Value::unnamed_composite(vec![Value::unnamed_composite(vec![
					Value::unnamed_variant("Task", vec![Value::u128(para as u128)]),
					Value::u128(57_600),
				])]),
				Value::unnamed_variant("None", vec![]),
			],
		);
		let sudo = dynamic::tx("Sudo", "sudo", vec![assign.into_value()]);
		relay
			.tx()
			.sign_and_submit_then_watch_default(&sudo, &dev::alice())
			.await?
			.wait_for_finalized_success()
			.await
			.map_err(|e| anyhow!("assigning core {core} to para {para}: {e}"))?;
		log::info!("core {core} assigned to teyrchain {para}");
	}
	Ok(())
}

/// Send one call to People signed by an office that lives there, and wait for the effect.
///
/// `settled` is what makes this honest, and it is the half that mattered when this helper sent
/// its calls from the relay instead: an extrinsic that finalises has been *included*, which is
/// not the same as having done what it was sent to do. Every caller supplies the question whose
/// answer changes, and this returns only when the chain itself says so.
pub(crate) async fn office_call_on_people<F, Fut>(
	people: &OnlineClient<PezkuwiConfig>,
	signer: &Keypair,
	pallet: &str,
	call: &str,
	fields: Vec<Value>,
	settled: F,
) -> Result<(), anyhow::Error>
where
	F: Fn() -> Fut,
	Fut: std::future::Future<Output = Result<bool, anyhow::Error>>,
{
	if settled().await? {
		return Ok(());
	}
	let tx = dynamic::tx(pallet, call, fields);
	people
		.tx()
		.sign_and_submit_then_watch_default(&tx, signer)
		.await?
		.wait_for_finalized_success()
		.await
		.map_err(|e| {
			anyhow!(
				"{pallet}::{call} was refused on People: {e}. The signer is the founding office \
				 seated by genesis, so a `BadOrigin` here means `TikiConfig::founding_government` \
				 did not name this key -- check the preset before the call"
			)
		})?;

	for _ in 0..(SETTLE_SECS / 6) {
		tokio::time::sleep(std::time::Duration::from_secs(6)).await;
		if settled().await? {
			return Ok(());
		}
	}
	Err(anyhow!(
		"{pallet}::{call} was accepted on People but the effect never showed within \
		 {SETTLE_SECS}s -- the extrinsic succeeded, so look at what the call did rather than at \
		 whether it arrived"
	))
}

/// Carry one call through the Parliament collective, because Root is not available to do it.
///
/// `confirm_prime_minister` takes `RootOrParliament`, and on this chain neither half is a
/// shortcut: Root arrives only through a twenty-eight-day referendum on a roll that does not
/// exist yet, and the relay's Root is refused by the register filter. That leaves the house
/// itself, which is the right answer anyway -- the President nominates and Parliament confirms
/// precisely so that one person is not both parties to the appointment.
///
/// The proposal hash is computed rather than read back from the `Proposed` event: the collective
/// hashes the encoded `RuntimeCall`, which is exactly what `encode_call_data` produces, and a
/// hash derived from the same bytes the chain saw cannot disagree with it.
async fn parliament_decides<F, Fut>(
	people: &OnlineClient<PezkuwiConfig>,
	voters: &[Keypair],
	pallet: &str,
	call: &str,
	settled: F,
) -> Result<(), anyhow::Error>
where
	F: Fn() -> Fut,
	Fut: std::future::Future<Output = Result<bool, anyhow::Error>>,
{
	if settled().await? {
		return Ok(());
	}
	let encoded = encode_people_call(people, pallet, call, Vec::new())?;
	let hash = people.hasher().hash(&encoded);
	let length_bound = encoded.len() as u64 + 8;

	// The index the collective will give this motion. Read rather than assumed: a rehearsal
	// that has already proposed something would otherwise vote on the wrong number and time
	// out with nothing to show for it.
	let index: u64 = storage_value(people, "Parliament", "ProposalCount", Vec::new())
		.await?
		.map(|v| format!("{v}").trim().parse().unwrap_or(0))
		.unwrap_or(0);

	// More than half of the *seated collective*, which is what `RootOrParliament` asks for.
	//
	// Written from the founding bench's length until run 18, and that is the run it cost.
	// `close` hands the inner call `RawOrigin::Members(ayes, seats)`, where `seats` is the
	// collective's own roster -- two hundred and one here -- and
	// `EnsureProportionMoreThan<1, 2>` then asks `ayes * 2 > seats`. A threshold of three
	// passed the collective's own tally and failed that arithmetic, so the motion carried and
	// `confirm_prime_minister` came back `BadOrigin`: a green close over a refused dispatch,
	// which is exactly the shape the `Executed` reading below was added to catch.
	let seated = collective_size(people).await?;
	let threshold = (seated / 2 + 1) as u64;
	if voters.len() < threshold as usize {
		return Err(anyhow!(
			"{pallet}::{call} needs {threshold} ayes of the {seated} seats the collective holds, \
			 and only {} signers were handed to this call. Seat and fund the majority before \
			 asking the house a question",
			voters.len()
		));
	}
	let proposal = Value::unnamed_variant(pallet, vec![Value::unnamed_variant(call, vec![])]);

	let propose = dynamic::tx(
		"Parliament",
		"propose",
		vec![Value::u128(threshold as u128), proposal, Value::u128(length_bound as u128)],
	);
	people
		.tx()
		.sign_and_submit_then_watch_default(&propose, &voters[0])
		.await?
		.wait_for_finalized_success()
		.await
		.map_err(|e| anyhow!("the house would not receive a motion on {pallet}::{call}: {e}"))?;

	// Every one of the threshold votes explicitly, the proposer included.
	//
	// Measured against `do_propose_proposed` rather than assumed from upstream: this collective
	// opens the motion with `ayes: vec![]`, so proposing is not voting here. Skipping the
	// proposer would leave the tally one short and `close` would fail with `TooEarly` -- forty
	// minutes after the mistake, which is the cost of reading a pallet by memory.
	//
	// Concurrently, but in chunks, and both halves of that are measured.
	//
	// One after another is a hundred and one block times and overruns this test's budget. All
	// hundred and one at once is worse in a way that reads as a chain fault: each submission
	// holds an RPC subscription open until its extrinsic finalises, and run 20 came back
	// *"Could not fetch event bytes ... Request timeout"* -- the node's RPC saturated, the votes
	// themselves were fine. Twenty-five is the size `fund_seats` already uses on this node for
	// the same reason.
	for chunk in voters.iter().take(threshold as usize).collect::<Vec<_>>().chunks(VOTE_CHUNK) {
		let votes = chunk.iter().map(|member| {
			let people = &people;
			async move {
				let vote = dynamic::tx(
					"Parliament",
					"vote",
					vec![
						Value::from_bytes(hash.as_ref()),
						Value::u128(index as u128),
						Value::bool(true),
					],
				);
				people
					.tx()
					.sign_and_submit_then_watch_default(&vote, *member)
					.await?
					.wait_for_finalized_success()
					.await
					.map_err(|e| anyhow!("a member could not vote on {pallet}::{call}: {e}"))?;
				Ok::<(), anyhow::Error>(())
			}
		});
		futures::future::try_join_all(votes).await?;
	}

	let close = dynamic::tx(
		"Parliament",
		"close",
		vec![
			Value::from_bytes(hash.as_ref()),
			Value::u128(index as u128),
			Value::named_composite([
				("ref_time", Value::u128(10_000_000_000)),
				("proof_size", Value::u128(1_000_000)),
			]),
			Value::u128(length_bound as u128),
		],
	);
	let closed = people
		.tx()
		.sign_and_submit_then_watch_default(&close, &voters[0])
		.await?
		.wait_for_finalized_success()
		.await
		.map_err(|e| {
			anyhow!(
				"the motion on {pallet}::{call} could not be closed: {e}. `TooEarly` means the \
				 threshold was not reached; `WrongProposalWeight` means the bound above is under \
				 what the call actually costs"
			)
		})?;

	// A closed motion says nothing about the call it carried.
	//
	// `close` succeeds when the tally is reached; the inner dispatch runs afterwards and its
	// result is reported in `Executed`, not in the extrinsic. So a motion can carry, the call can
	// fail, and both the close and the tally look perfect. Measured 2026-09-20: the house carried
	// `confirm_prime_minister` and the tiki never moved, and the only thing the rehearsal could
	// say was that an effect had not appeared within ninety seconds -- which points at the chain
	// when the answer was in an event nobody read.
	for ev in closed.iter().filter_map(Result::ok) {
		if ev.pallet_name() == "Parliament" && ev.variant_name() == "Executed" {
			// `decode_as_fields` into a `scale_value` composite: the event's shape comes from
			// the metadata, so this does not need a static type for `Executed` and does not go
			// stale when the variant gains a field.
			let rendered = ev
				.decode_as_fields::<scale_value::Composite<()>>()
				.map(|f| format!("{f}"))
				.unwrap_or_else(|e| format!("<undecodable: {e}>"));
			if !rendered.contains("Ok") {
				return Err(anyhow!(
					"the house carried {pallet}::{call} and the call itself failed: {rendered}. \
					 The tally and the close are both fine; what failed is the dispatch the \
					 motion was carrying"
				));
			}
			log::info!("{pallet}::{call} executed: {rendered}");
		}
	}

	for _ in 0..(SETTLE_SECS / 6) {
		tokio::time::sleep(std::time::Duration::from_secs(6)).await;
		if settled().await? {
			return Ok(());
		}
	}
	Err(anyhow!(
		"the house carried {pallet}::{call} but the effect never showed within {SETTLE_SECS}s -- \
		 a closed motion whose inner call failed leaves `Executed` with an error inside it, so \
		 read that event rather than the close"
	))
}

/// Send one Root call into People from the relay, for the calls the register filter lets through.
///
/// This is not the founding path and it must not be used as one. `TheRegisterIsNotWritableFromAbroad`
/// refuses `Welati`, `Tiki`, `Diwan`, `Parliament`, `Trust` and `IdentityKyc` whatever origin
/// carries them, and refuses them without failing the relay extrinsic -- so a register call sent
/// this way reports a clean send and then never happens. Use `office_call_on_people` for those.
///
/// What is left is real and is why this stayed: `StakingScore::receive_staking_details` is a fact
/// the relay owns and People only records, so the relay reporting it over `Transact` is the
/// production path rather than a shortcut around one.
///
/// `settled` is what makes this honest. The relay extrinsic finalising means the *message* was
/// sent, nothing more: execution on the other side is a separate block on a separate chain and
/// can fail there with the relay none the wiser. So every caller supplies the question whose
/// answer changes, and this returns only when the chain itself says so.
pub(crate) async fn root_call_on_people<F, Fut>(
	relay: &OnlineClient<PezkuwiConfig>,
	people: &OnlineClient<PezkuwiConfig>,
	pallet: &str,
	call: &str,
	fields: Vec<Value>,
	settled: F,
) -> Result<(), anyhow::Error>
where
	F: Fn() -> Fut,
	Fut: std::future::Future<Output = Result<bool, anyhow::Error>>,
{
	if settled().await? {
		return Ok(());
	}
	let encoded = encode_people_call(people, pallet, call, fields)?;
	let tx = relay_root_into_people(encoded);
	relay
		.tx()
		.sign_and_submit_then_watch_default(&tx, &dev::alice())
		.await?
		.wait_for_finalized_success()
		.await
		.map_err(|e| anyhow!("relay refused to send {pallet}::{call}: {e}"))?;

	for _ in 0..(XCM_SETTLE_SECS / 6) {
		tokio::time::sleep(std::time::Duration::from_secs(6)).await;
		if settled().await? {
			return Ok(());
		}
	}
	Err(anyhow!(
		"{pallet}::{call} was sent from the relay but People never showed the effect within \
		 {XCM_SETTLE_SECS}s -- the message was accepted for delivery, so look at execution on \
		 People rather than at the relay: a dropped or trapped XCM leaves no failed extrinsic"
	))
}

// ---------------------------------------------------------------------------------------
// Reading People
// ---------------------------------------------------------------------------------------

async fn storage_value(
	api: &OnlineClient<PezkuwiConfig>,
	pallet: &str,
	item: &str,
	keys: Vec<Value>,
) -> Result<Option<Value>, anyhow::Error> {
	let addr = dynamic::storage::<Vec<Value>, Value>(pallet, item);
	// Bound the storage client before using it: `at_latest()` returns a value the fetch
	// borrows from, so chaining the two drops it while the borrow is still live.
	let at = api.storage().at_latest().await?;
	let fetched = at.try_fetch(addr, keys).await?;
	match fetched {
		Some(v) => Ok(Some(v.decode()?)),
		None => Ok(None),
	}
}

/// How many names sit on a bench.
///
/// Reads the roster the pallet keeps rather than counting what this file asked for: the two
/// disagree exactly when something went wrong, which is the only time the number matters.
pub(crate) async fn bench_size(
	people: &OnlineClient<PezkuwiConfig>,
	item: &str,
) -> Result<usize, anyhow::Error> {
	match storage_value(people, "Welati", item, Vec::new()).await? {
		None => Ok(0),
		Some(v) => match v.value {
			scale_value::ValueDef::Composite(c) => Ok(bounded_len(c)),
			other => Err(anyhow!("Welati::{item} decoded as {other:?}, not a list")),
		},
	}
}

/// How many seats the Parliament *collective* holds.
///
/// Not the same number as `Welati::ParliamentMembers`, and the difference is the whole point:
/// `EnsureProportionMoreThan` divides by the collective's roster, so that is the roster a
/// threshold has to be written against. Reading it here rather than deriving it from
/// `PARLIAMENT_SIZE` also means a partly seated house produces a threshold that still passes
/// instead of one that is silently too small.
pub(crate) async fn collective_size(
	people: &OnlineClient<PezkuwiConfig>,
) -> Result<usize, anyhow::Error> {
	match storage_value(people, "Parliament", "Members", Vec::new()).await? {
		None => Ok(0),
		Some(v) => match v.value {
			scale_value::ValueDef::Composite(c) => Ok(bounded_len(c)),
			other => Err(anyhow!("Parliament::Members decoded as {other:?}, not a list")),
		},
	}
}

/// How many items a decoded list holds, seeing through the `BoundedVec` wrapper.
///
/// `BoundedVec<T, S>` encodes exactly like `Vec<T>`, but it is *described* in the metadata as a
/// composite with one unnamed field that is the vector -- so decoding by type gives a list of
/// length one whose only element is the real list. Counting the outer one returns 1 no matter
/// how many members are seated.
///
/// That cost a rehearsal run on 2026-09-19. `seat_founding_parliament` was accepted, the storage
/// was written, and the test waited ninety seconds for `1 >= 5` to become true. The failure read
/// as "the call did nothing", which is the most expensive kind of wrong: it points at the chain
/// when the fault is in the reader.
///
/// The discriminator is the child's *naming*, not the outer length, and the difference matters:
/// a list that genuinely holds one member is also a composite of length one. `ParliamentMember`
/// is a named struct, so a one-member bench decodes as `[Named{..}]` and is left alone; the
/// `BoundedVec` wrapper's only child is the unnamed vector, so that one is descended into. A
/// length-only test would have counted a lone member's fields and returned a plausible number.
///
/// One level either way. Deeper would start unwrapping the members themselves.
fn bounded_len<T>(c: scale_value::Composite<T>) -> usize {
	if let scale_value::Composite::Unnamed(items) = &c {
		if items.len() == 1 {
			if let scale_value::ValueDef::Composite(scale_value::Composite::Unnamed(inner)) =
				&items[0].value
			{
				return inner.len();
			}
		}
	}
	c.len()
}

/// Who holds an office, if anybody does.
async fn tiki_holder(
	people: &OnlineClient<PezkuwiConfig>,
	tiki_variant: &str,
) -> Result<Option<Value>, anyhow::Error> {
	storage_value(people, "Tiki", "TikiHolder", vec![Value::unnamed_variant(tiki_variant, vec![])])
		.await
}

/// Does this account hold a given tiki?
///
/// Reads `Tiki::UserTikis`, which is the list the court's qualification check reads, and looks
/// for the variant by name. Comparing rendered names rather than decoding the enum keeps this
/// from carrying a copy of an index that the runtime is free to renumber.
async fn has_tiki(
	people: &OnlineClient<PezkuwiConfig>,
	who: &Keypair,
	variant: &str,
) -> Result<bool, anyhow::Error> {
	let held = storage_value(
		people,
		"Tiki",
		"UserTikis",
		vec![Value::from_bytes(who.public_key().to_account_id().0)],
	)
	.await?;
	Ok(match held {
		None => false,
		Some(v) => format!("{v}").contains(variant),
	})
}

/// A `MultiAddress`, for the calls that take one.
///
/// `Lookup::Source` is an enum and wants the `Id` variant around the bytes. Everything in this
/// tree that takes a *plain* `AccountId` wants `raw_account` instead, and the two are not
/// interchangeable: passing a `MultiAddress` where an `AccountId` belongs fails at encoding
/// with "Cannot encode Str into type with ID 2", which reads like a string problem and is
/// really a variant that the target type has no place for. Measured 2026-09-18, on four calls.
fn multi_address(k: &Keypair) -> Value {
	Value::unnamed_variant("Id", vec![Value::from_bytes(k.public_key().to_account_id().0)])
}

/// A bare `AccountId`: thirty-two bytes and no wrapper.
fn raw_account(k: &Keypair) -> Value {
	Value::from_bytes(k.public_key().to_account_id().0)
}

/// The bench this rehearsal seats: the well-known keys, which the local preset endows and
/// makes founding citizens. Deriving strangers would only add a funding round to reach the
/// same place, and the offices here are seated by Root rather than won, so standing does not
/// enter into it.
/// Put a fee budget on every seat that is going to vote.
///
/// Seating a house and voting in one are different asks: `seat_founding_parliament` takes a list
/// of accounts and never touches their balances, but a vote is an extrinsic and an extrinsic is
/// paid for. Measured 2026-09-19: the house was seated, the proposal opened, and the first vote
/// came back `Invalid Transaction (1010)` -- the signer could not pay. Only the hundred accounts
/// the register funded had anything, and a simple majority of two hundred and one is a hundred
/// and one, so the rehearsal was exactly one funded account short of being able to carry a
/// question.
///
/// Funded in chunks, like the register's cohort, because a batch of a hundred and one transfers
/// is a larger block than the runtime wants to build in one go.
pub(crate) async fn fund_seats(
	people: &OnlineClient<PezkuwiConfig>,
	seats: &[Keypair],
) -> Result<(), anyhow::Error> {
	for chunk in seats.chunks(25) {
		let calls: Vec<Value> = chunk
			.iter()
			.map(|k| {
				dynamic::tx(
					"Balances",
					"transfer_keep_alive",
					vec![
						Value::unnamed_variant(
							"Id",
							vec![Value::from_bytes(k.public_key().to_account_id().0)],
						),
						Value::u128(FUND_PER_SEAT),
					],
				)
				.into_value()
			})
			.collect();
		let batch = dynamic::tx("Utility", "batch_all", vec![Value::unnamed_composite(calls)]);
		// The founder pays. `dev::alice()` is the founding citizen in the local preset -- the
		// account that holds NFT #0 and the endowment -- so this is the founder's account and
		// not a convenient key that happens to have money.
		people
			.tx()
			.sign_and_submit_then_watch_default(&batch, &dev::alice())
			.await?
			.wait_for_finalized_success()
			.await
			.map_err(|e| anyhow!("could not fund a chunk of seats: {e}"))?;
	}
	log::info!("{} seats funded", seats.len());
	Ok(())
}

/// The full house the state starts with: `ParliamentSize` seats.
///
/// Separate from `founding_bench`, which is the handful that holds *offices* -- the President,
/// the court, the Prime Minister. A seat and an office are different things: the office-holders
/// are named well-known keys so a failure names a person, while the house only has to be large
/// enough that a majority of `ParliamentSize` can exist at all.
///
/// The first five are the bench, so the President and the ministers sit in the house they act
/// in rather than beside it, which is what the constitution describes.
pub(crate) fn founding_house() -> Result<Vec<Keypair>, anyhow::Error> {
	use pezkuwi_zombienet_sdk::subxt_signer::SecretUri;
	use std::str::FromStr;
	let mut house = founding_bench();
	for i in (house.len() as u32 + 1)..=PARLIAMENT_SIZE {
		let uri = SecretUri::from_str(&format!("//Alice//seat//{i}"))
			.map_err(|e| anyhow!("seat {i}: {e}"))?;
		house.push(Keypair::from_uri(&uri).map_err(|e| anyhow!("seat {i}: {e}"))?);
	}
	Ok(house)
}

pub(crate) fn founding_bench() -> Vec<Keypair> {
	vec![dev::alice(), dev::bob(), dev::charlie(), dev::dave(), dev::eve()]
		.into_iter()
		.take(FOUNDING_MEMBERS)
		.collect()
}

// ---------------------------------------------------------------------------------------
// The stages
// ---------------------------------------------------------------------------------------
/// The offices, on a network whose register has filled and whose house is already seated.
///
/// Not a test of its own: the budget stage needs a finance minister to hold the purse, and the
/// minister is appointed at the end of this sequence. Run on separate networks, the budget was
/// approved and then had nobody to spend it -- "the budget was approved but no finance minister
/// holds the purse", measured 2026-09-20 on run 12, one layer after the votes started carrying.
///
/// The seating of the house is *not* here. The merged flow seats two hundred and one before this
/// runs, because the threshold is counted against `ParliamentSize` and a smaller bench can carry
/// nothing -- see `founding_house`.
pub(crate) async fn the_founding_offices_are_filled(
	people: &OnlineClient<PezkuwiConfig>,
	house: &[Keypair],
) -> Result<(), anyhow::Error> {
	// The whole seated house, not the founding five: the one motion this stage carries is
	// weighed against the collective's roster, so a majority of that roster has to sign it.
	// `founding_house()` opens with `founding_bench()`, so `house[0]` is still the Serok and
	// the offices below are still filled from the same five accounts.
	// ---- The President, already in office -----------------------------------------------
	//
	// Not granted here, and there is no way to grant it: `Tiki::Serok` is an *Elected* role
	// whose dispatchable origin is Root alone, and Root is not reachable on this chain on its
	// first day -- the relay's is refused by the register filter and this chain's own Root
	// track wants a referendum on a roll that does not exist. So genesis seats it, through
	// `TikiConfig::founding_government`, and the rehearsal's job here is to *check* that
	// rather than to perform it.
	//
	// It is read before anything else because everything below depends on it: the seating of
	// the house, the court's qualifications and the nomination are all this account's to sign.
	let serok = house[0].clone();
	assert!(
		has_tiki(people, &serok, "Serok").await?,
		"genesis did not seat this key as Serok, so nothing below can be signed -- the local \
		 preset's `founding_government` is where that is decided"
	);

	// ---- The Dîwan, by the ordinary procedure -------------------------------------------
	//
	// Five seats, not eleven, and that is the design rather than a shortfall: `DiwanSize` is
	// eleven and `DiwanElectedSeats` six, so `appoint_diwan_member` refuses a sixth
	// appointment. The other six are Parliament's to elect once citizens have standing, and
	// `RootOrDiwan` is a *proportion* of whatever bench exists -- two thirds of five is four
	// -- so a court of five is a working court, not a broken one.
	//
	// A qualifying professional tiki first. The court check asks for one of fourteen, and on
	// the day a chain starts nobody holds any, so the grant is part of the founding act. It
	// is Root's here only because the President cannot yet be relied on to have granted
	// himself nothing: `grant_tiki` takes `RootOrSerokOrCouncil`, and using Root keeps the
	// qualification and the appointment in different hands.
	//
	// Then the appointment itself is *signed by the President*. That is the whole reason the
	// presidency is granted above: Root could do this, and doing it that way would rehearse a
	// path the state does not use once it has a head.
	let court: Vec<&Keypair> = house.iter().skip(1).take(3).collect();
	for (i, member) in court.iter().enumerate() {
		let want = i + 1;
		log::info!("qualifying and appointing court member {want}");
		office_call_on_people(
			people,
			&serok,
			"Tiki",
			"grant_tiki",
			vec![multi_address(member), Value::unnamed_variant("Hiquqnas", vec![])],
			|| {
				let people = people;
				let who = (*member).clone();
				async move { has_tiki(people, &who, "Hiquqnas").await }
			},
		)
		.await?;

		let appoint = dynamic::tx(
			"Welati",
			"appoint_diwan_member",
			vec![Value::from_bytes(member.public_key().to_account_id().0)],
		);
		people
			.tx()
			.sign_and_submit_then_watch_default(&appoint, &serok)
			.await?
			.wait_for_finalized_success()
			.await
			.map_err(|e| {
				anyhow!(
					"the President could not seat court member {want}: {e}. \
					 `NotQualifiedForTheCourt` means the professional tiki did not land; \
					 `AppointedCourtSeatsAreFull` means more than \
					 `DiwanSize - DiwanElectedSeats` were attempted, which is a fault in this \
					 test rather than in the chain"
				)
			})?;
	}
	let court_size = bench_size(people, "DiwanMembers").await?;
	assert_eq!(
		court_size,
		court.len(),
		"the President seated {court_size} of {} court members",
		court.len()
	);

	// ---- The executive ----------------------------------------------------------------
	//
	// The President names, Parliament confirms, and the two are deliberately different
	// bodies: `ConfirmationOrigin` is Root or Parliament and leaves the President off it,
	// because one person cannot be both parties to an appointment. So the nomination is
	// signed by the President -- on his own authority, not Root's -- and the confirmation
	// comes from the other side. The house had to be seated first for that to be possible.
	let pm = house[4].clone();
	log::info!("the President nominating a Prime Minister");
	let nominate = dynamic::tx(
		"Welati",
		"appoint_prime_minister",
		vec![Value::from_bytes(pm.public_key().to_account_id().0)],
	);
	people
		.tx()
		.sign_and_submit_then_watch_default(&nominate, &serok)
		.await?
		.wait_for_finalized_success()
		.await
		.map_err(|e| {
			anyhow!(
				"the President could not nominate a Prime Minister: {e}. This call takes Root \
				 or the Serok, so a refusal here means the presidency never landed"
			)
		})?;
	assert!(
		storage_value(people, "Welati", "PendingPrimeMinister", Vec::new())
			.await?
			.is_some(),
		"the nomination was accepted but no pending Prime Minister is recorded"
	);

	log::info!("Parliament confirming");
	parliament_decides(people, house, "Welati", "confirm_prime_minister", || {
		let people = people;
		async move { Ok(tiki_holder(people, "SerokWeziran").await?.is_some()) }
	})
	.await
	.map_err(|e| {
		anyhow!(
			"the nomination stood but was never confirmed into office: {e}. The confirming \
			 origin is Root or Parliament -- if the house is empty the call is refused, so \
			 check the seating stage before the message"
		)
	})?;

	// ---- A portfolio ------------------------------------------------------------------
	//
	// The finance portfolio specifically, because `spend_budget` asks for that holder by
	// name and the budget stage below depends on it. Appointed by the Prime Minister, not by
	// Root: `appoint_minister` checks the caller *is* the Prime Minister, so this only works
	// if the confirmation above genuinely put them in office. It is the one assertion here
	// that the previous stage cannot fake.
	let treasurer = house[2].clone();
	log::info!("the Prime Minister appointing a finance minister");
	let appoint = dynamic::tx(
		"Welati",
		"appoint_minister",
		vec![raw_account(&treasurer), Value::unnamed_variant("WezireDarayiye", vec![])],
	);
	people
		.tx()
		.sign_and_submit_then_watch_default(&appoint, &pm)
		.await?
		.wait_for_finalized_success()
		.await
		.map_err(|e| {
			anyhow!(
				"the Prime Minister could not appoint a minister: {e}. This call checks the \
				 caller holds the office, so a failure here means the confirmation did not \
				 take effect even though the storage said it had"
			)
		})?;

	let holder = tiki_holder(people, "WezireDarayiye").await?;
	assert!(
		holder.is_some(),
		"the finance portfolio is still vacant after a successful appointment"
	);

	log::info!(
		"offices filled: {} in the house, {court_size} on the bench, executive seated",
		bench_size(people, "ParliamentMembers").await?
	);
	Ok(())
}

/// Path 2, on a network whose register has opened the gate and whose house is fully seated.
///
/// Two preconditions it could not meet alone, and it used to raise its own network and stand on
/// both. The government pot is filled by the monthly release, which only `activate_distribution`
/// starts, and only the register reaching the population gate does that. And a question needs a
/// hundred and one ayes -- `ParliamentSize / 2 + 1`, counted against the constant -- so the five
/// office-holders could open a proposal and never carry it.
pub(crate) async fn a_budget_is_voted_and_the_treasurer_spends_it(
	people: &OnlineClient<PezkuwiConfig>,
	asset_hub: &OnlineClient<PezkuwiConfig>,
	house: &[Keypair],
	bench: &[Keypair],
) -> Result<(), anyhow::Error> {
	// The founding hand, seated by genesis rather than granted here -- see the note in
	// `the_founding_offices_are_filled_and_the_executive_is_confirmed`. `seat_founding_parliament`
	// takes `ensure_root_or_serok`, and on this chain only the second half exists.
	// Only a sitting member or the President may propose, which is why the house comes first.
	let proposer = bench[0].clone();
	let amount: u128 = 1_000_000_000_000; // one HEZ, in the smallest unit
	let submit = dynamic::tx(
		"Welati",
		"submit_proposal",
		vec![
			Value::from_bytes(b"Rehearsal budget".to_vec()),
			Value::from_bytes(b"A first line of spending, to prove the path carries".to_vec()),
			Value::unnamed_variant("ParliamentSimpleMajority", vec![]),
			Value::unnamed_variant("Normal", vec![]),
			Value::unnamed_variant("Some", vec![Value::u128(amount)]),
		],
	);
	people
		.tx()
		.sign_and_submit_then_watch_default(&submit, &proposer)
		.await?
		.wait_for_finalized_success()
		.await?;

	// The proposal's id is whatever the chain gave it. Reading `NextProposalId` and stepping
	// back one is the only way to learn it without trusting this file's own count.
	let next = storage_value(people, "Welati", "NextProposalId", Vec::new())
		.await?
		.ok_or_else(|| anyhow!("Welati::NextProposalId is unset after a successful proposal"))?;
	let next = next
		.as_u128()
		.ok_or_else(|| anyhow!("NextProposalId did not decode as a number"))? as u32;
	let proposal_id = next.checked_sub(1).ok_or_else(|| anyhow!("no proposal was recorded"))?;
	log::info!("proposal {proposal_id} is open");

	// Every member votes aye. A simple majority would do; all of them makes the tally
	// unambiguous if `finalize_proposal` later says it did not pass.
	// All at once, not one after another.
	//
	// A hundred and one votes submitted in sequence, each awaited to finalisation, is a hundred
	// and one block times: measured 2026-09-20, it ate the rest of a sixty-minute budget after
	// the register had already spent thirty-five minutes filling. Every voter is a different
	// signer with its own nonce, so nothing forces them to queue -- this is the same shape the
	// register uses for its applications.
	// In chunks, for the same reason the parliamentary votes are: a hundred and one open RPC
	// subscriptions saturate this node and it answers `Request timeout` instead of anything
	// about the chain. Measured on run 20.
	for chunk in house.iter().take(SIMPLE_MAJORITY).collect::<Vec<_>>().chunks(VOTE_CHUNK) {
		let votes = chunk.iter().map(|member| {
			let people = &people;
			async move {
				let vote = dynamic::tx(
					"Welati",
					"vote_on_proposal",
					vec![
						Value::u128(proposal_id as u128),
						Value::unnamed_variant("Aye", vec![]),
						Value::unnamed_variant("None", vec![]),
					],
				);
				people
					.tx()
					.sign_and_submit_then_watch_default(&vote, *member)
					.await?
					.wait_for_finalized_success()
					.await
					.map_err(|e| {
						anyhow!(
							"a seated member could not vote on proposal {proposal_id}: {e}. Voting \
							 opens after `ProposalVotingDelay`; on a node built without \
							 `fast-runtime` that is a full day of blocks and this run cannot reach it"
						)
					})?;
				Ok::<(), anyhow::Error>(())
			}
		});
		futures::future::try_join_all(votes).await?;
	}

	// `finalize_proposal` passes a proposal as soon as the ayes reach the threshold -- it
	// does not wait for the window to close. Anybody may call it.
	let finalize =
		dynamic::tx("Welati", "finalize_proposal", vec![Value::u128(proposal_id as u128)]);
	people
		.tx()
		.sign_and_submit_then_watch_default(&finalize, &proposer)
		.await?
		.wait_for_finalized_success()
		.await?;

	let budget = storage_value(people, "Welati", "ApprovedBudget", Vec::new())
		.await?
		.and_then(|v| v.as_u128())
		.unwrap_or(0);
	assert!(
		budget >= amount,
		"the house voted {amount} through but the approved budget stands at {budget} -- the \
		 proposal was finalised without its money being credited"
	);
	log::info!("approved budget is {budget}");

	// And the spend, which only the finance minister may make. This stage does not seat one
	// -- that is the previous test's business -- so if the portfolio is vacant here, say so
	// plainly rather than reporting the refusal as a fault in the spending path.
	let treasurer = bench[2].clone();
	if tiki_holder(people, "WezireDarayiye").await?.is_none() {
		return Err(anyhow!(
			"the budget was approved but no finance minister holds the purse, so the spend \
			 cannot be rehearsed here -- run \
			 `the_founding_offices_are_filled_and_the_executive_is_confirmed` first"
		));
	}
	let spend = dynamic::tx(
		"Welati",
		"spend_budget",
		vec![Value::from_bytes(dev::ferdie().public_key().to_account_id().0), Value::u128(amount)],
	);
	people
		.tx()
		.sign_and_submit_then_watch_default(&spend, &treasurer)
		.await?
		.wait_for_finalized_success()
		.await?;

	let left = storage_value(people, "Welati", "ApprovedBudget", Vec::new())
		.await?
		.and_then(|v| v.as_u128())
		.unwrap_or(0);
	assert!(
		left < budget,
		"the minister spent {amount} but the approved budget is still {left} -- the allowance \
		 was not drawn down, which would let the same money be spent again"
	);
	log::info!("budget drawn down from {budget} to {left}");

	// ---- and the money arrives, which is the half People cannot show -------------------
	//
	// Drawing the allowance down happens on People whether or not anything reaches the Asset
	// Hub. The budget is an authorisation; the PEZ is on the other chain, and the only proof
	// that the authorisation carried is the beneficiary's balance there. Reading People twice
	// would have shown a spend that went nowhere as a complete success.
	//
	// This is path 2 of the four: People's finance minister spending, `GovernmentSpendOrigin`
	// on the far side accepting it because it arrived from People and from nowhere else.
	// The hub client is the caller's: this stage runs on the network the register filled, so
	// raising a second connection here would be raising it to the same node twice.
	let paid = wait_for_pez(asset_hub, &dev::ferdie(), amount, XCM_SETTLE_SECS).await?;
	assert!(
		paid,
		"the minister's spend was accepted on People and the allowance fell, but the Asset Hub \
		 never credited {amount} PEZ to the beneficiary. The message was sent, so look at \
		 execution there: `InsufficientGovernmentPotBalance` means the pot was never funded \
		 (distribution has to be active first), while nothing at all in the queue points at \
		 `SPEND_FROM_GOVERNMENT_POT_CALL_INDEX` addressing the wrong call"
	);
	log::info!("path 2 carried: {amount} PEZ left the government pot on the Asset Hub");
	Ok(())
}

/// Wait until an account holds at least `want` PEZ on the Asset Hub.
///
/// PEZ is asset 1 there, so this is `Assets::Account(1, who)` — a map with two keys, which is
/// the reason it does not share the plain `storage_value` helper above.
async fn wait_for_pez(
	asset_hub: &OnlineClient<PezkuwiConfig>,
	who: &Keypair,
	want: u128,
	secs: u64,
) -> Result<bool, anyhow::Error> {
	let addr = dynamic::storage::<Vec<Value>, Value>("Assets", "Account");
	let keys = vec![
		Value::u128(PEZ_ASSET_ID as u128),
		Value::from_bytes(who.public_key().to_account_id().0),
	];
	for _ in 0..(secs / 6) {
		let at = asset_hub.storage().at_latest().await?;
		if let Some(raw) = at.try_fetch(addr.clone(), keys.clone()).await? {
			let v = raw.decode()?;
			// The account record carries more than the balance, so pull the named field out
			// rather than trusting a position that a later field could shift.
			if let Some(bal) = v.at("balance").and_then(|b| b.as_u128()) {
				if bal >= want {
					return Ok(true);
				}
			}
		}
		tokio::time::sleep(std::time::Duration::from_secs(6)).await;
	}
	Ok(false)
}

/// One citizen opens a question, the roll backs it, and it becomes a referendum.
///
/// The initiative is the path that does not go through Parliament at all, which is why it is
/// worth its own rehearsal: everything else here is the state acting on itself.
#[tokio::test(flavor = "multi_thread")]
async fn a_citizen_initiative_reaches_a_referendum() -> Result<(), anyhow::Error> {
	let _ = env_logger::try_init_from_env(
		env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
	);

	let network = initialize_network(build_network_config().await?).await?;
	let people: OnlineClient<PezkuwiConfig> =
		network.get_node("people-collator-01")?.wait_client().await?;

	// Any preimage hash will do: `launch_initiative` hands the proposal to the referenda
	// pallet, which accepts a hash it has not seen -- the preimage only has to exist by the
	// time the referendum would enact. What is being rehearsed is the route to the ballot,
	// not what is on it.
	let proposal_hash = [7u8; 32];
	let track: u16 = 0; // `root`, the first track People declares

	let proposer = dev::alice();
	let open = dynamic::tx(
		"Welati",
		"open_initiative",
		vec![Value::u128(track as u128), Value::from_bytes(proposal_hash), Value::u128(32)],
	);
	// Logged step by step from here on, and the reason is a nineteen-minute silence.
	//
	// Run 18 spawned this stage's network, assigned its cores, and then printed nothing at all
	// until the twenty-minute cap killed it. Every line between here and the end was a submit
	// awaiting finalisation, so the log could not say which one never came back -- a stall and
	// a slow chain look identical when neither is narrated. Measured 2026-09-20.
	log::info!("opening an initiative on track {track}");
	people
		.tx()
		.sign_and_submit_then_watch_default(&open, &proposer)
		.await?
		.wait_for_finalized_success()
		.await
		.map_err(|e| anyhow!("the initiative could not be opened on track {track}: {e}"))?;

	let next = storage_value(&people, "Welati", "NextInitiativeId", Vec::new())
		.await?
		.and_then(|v| v.as_u128())
		.ok_or_else(|| anyhow!("Welati::NextInitiativeId is unset after opening one"))?
		as u32;
	let id = next.checked_sub(1).ok_or_else(|| anyhow!("no initiative was recorded"))?;

	// Backing is a share of the roll, so how many signatures are needed depends on how many
	// citizens there are. Everybody available backs it; the launch below is what decides
	// whether that was enough, and its error names the shortfall.
	log::info!("initiative {id} is open; collecting backing");
	for (n, backer) in [dev::bob(), dev::charlie(), dev::dave(), dev::eve()].into_iter().enumerate()
	{
		log::info!("backer {} of 4 signing initiative {id}", n + 1);
		let back = dynamic::tx("Welati", "back_initiative", vec![Value::u128(id as u128)]);
		// A citizen who cannot back it is not a failure of this path -- an account that never
		// completed registration is simply not on the roll. Only the launch is decisive.
		if let Err(e) = people
			.tx()
			.sign_and_submit_then_watch_default(&back, &backer)
			.await?
			.wait_for_finalized_success()
			.await
		{
			log::info!("a backer was refused, continuing: {e}");
		}
	}

	log::info!("launching initiative {id}");
	let launch = dynamic::tx("Welati", "launch_initiative", vec![Value::u128(id as u128)]);
	people
		.tx()
		.sign_and_submit_then_watch_default(&launch, &proposer)
		.await?
		.wait_for_finalized_success()
		.await
		.map_err(|e| {
			anyhow!(
				"initiative {id} could not be launched: {e}. `NotEnoughBacking` here means the \
				 threshold is a share of a roll larger than the number of citizens this \
				 network seated -- fill the register first with `state_rehearsal`"
			)
		})?;

	// The proof is a referendum existing, on the referenda pallet, not a flag on the
	// initiative. `ReferendumCount` is the referenda pallet's own counter.
	let count = storage_value(&people, "Referenda", "ReferendumCount", Vec::new())
		.await?
		.and_then(|v| v.as_u128())
		.unwrap_or(0);
	assert!(
		count > 0,
		"the initiative launched but no referendum exists -- the hand-off to the referenda \
		 pallet is where this path breaks, and it breaks silently"
	);
	log::info!("initiative {id} carried; the chain holds {count} referenda");
	Ok(())
}
/// Paths 3 and 4, on a network whose register has already opened the gate.
///
/// Not a test of its own any more, and the reason is a precondition it could never meet. These
/// stages need `DistributionStarted` on the Asset Hub, and the only thing that sets it is the
/// People chain's register reaching the population gate -- `activate_distribution` takes
/// `EnsureXcm<Equals<PeopleLocation>>` with root explicitly refused, so there is no shortcut and
/// there was never going to be one. On its own network, with a genesis roll of five, the stage
/// stood on a precondition it could not arrange; the earlier version sent a call that does not
/// exist and swallowed the error, which is how it looked like it was running for so long.
///
/// So it runs where the precondition is true: `state_rehearsal` fills the register, proves the
/// gate, and then calls this. One network instead of two, and the ten minutes a second spawn
/// costs are returned as well.
pub(crate) async fn the_treasury_funds_the_payroll_and_the_payroll_pays_across(
	people: &OnlineClient<PezkuwiConfig>,
	asset_hub: &OnlineClient<PezkuwiConfig>,
	bench: &[Keypair],
) -> Result<(), anyhow::Error> {
	// ---- path 4: the hub reports what it released ---------------------------------------
	//
	// Release zero is due the moment distribution starts -- the schedule is derived from the
	// release index, and index zero is due after no blocks at all, so the state begins paying
	// in the era it has enough citizens rather than a month later. The report crosses to
	// People and lands as a running total, which is what the payroll spends against.
	log::info!("waiting for the treasury's funding report to reach People");
	let funded = wait_for(people, "PezRewards", "ReportedIncentiveTotal", XCM_SETTLE_SECS, |v| {
		v.as_u128().map(|n| n > 0).unwrap_or(false)
	})
	.await;
	assert!(
		funded,
		"distribution is active on the Asset Hub but People's `ReportedIncentiveTotal` is still 		 zero. The report is sent from the hub's `on_initialize`, so this is either the release 		 not happening -- look for `MonthlyReleaseFailed` there -- or the message not arriving: 		 `NOTE_INCENTIVE_FUNDING_CALL_INDEX` and `PezRewardsPalletIndex` are what address it"
	);

	// ---- path 3: a member claims, and the hub pays ---------------------------------------
	//
	// An epoch has to close first. It closes on People's own `on_initialize` after
	// `EpochLength`, which is compressed in a rehearsal build and thirty days otherwise --
	// so a failure here is very often a node built without `fast-runtime`.
	log::info!("waiting for the first epoch to finalise");
	let closed = wait_for(people, "PezRewards", "EpochInfo", EPOCH_SETTLE_SECS, |v| {
		v.at("total_epochs_completed")
			.and_then(|n| n.as_u128())
			.map(|n| n > 0)
			.unwrap_or(false)
	})
	.await;
	assert!(
		closed,
		"no epoch finalised within {EPOCH_SETTLE_SECS}s. `EpochLength` is thirty days unless 		 the runtime was built with `fast-runtime`, and nothing downstream of a closed epoch 		 can be reached until one is"
	);

	// Which epoch is claimable is the chain's to say, not this file's.
	//
	// It used to claim epoch zero. Under `fast-runtime` an epoch lasts minutes, so by the time
	// the stages above have run, epoch zero has been finalised *and* closed by the next one, and
	// the claim comes back `NotInClaimPeriod` -- a correct refusal against a wrong assumption.
	// `EpochInClaim` is the pallet's own answer and moves with it. Measured on run 20.
	let epoch = storage_value(people, "PezRewards", "EpochInClaim", Vec::new())
		.await?
		.and_then(|v| v.as_u128())
		.ok_or_else(|| {
			anyhow!(
				"an epoch finalised but `PezRewards::EpochInClaim` is unset, so nothing is \
				 claimable. The window is `ClaimPeriod` blocks wide and closes on its own"
			)
		})? as u32;
	log::info!("epoch {epoch} is in its claim window");

	let claimant = bench[0].clone();
	let claim = dynamic::tx("PezRewards", "claim_reward", vec![Value::u128(epoch as u128)]);
	people
		.tx()
		.sign_and_submit_then_watch_default(&claim, &claimant)
		.await?
		.wait_for_finalized_success()
		.await
		.map_err(|e| {
			anyhow!(
				"a seated member could not claim epoch {epoch}: {e}. `NothingToClaim` means the seat 				 share was zero, which happens when the pot was never funded -- check path 4 				 above before looking at the claim"
			)
		})?;

	// And the proof is on the hub again. People records the claim either way; whether the PEZ
	// moved is a fact about the other chain.
	let paid = wait_for_pez(asset_hub, &claimant, 1, XCM_SETTLE_SECS).await?;
	assert!(
		paid,
		"the claim was accepted on People but no PEZ reached the claimant on the Asset Hub. 		 The incentive pot is the one that pays this, and it is a different pot from the 		 government one -- an empty incentive pot means the release never credited it"
	);
	log::info!("paths 3 and 4 carried: funding reported, claim paid across");
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
