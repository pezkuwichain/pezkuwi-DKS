// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! The state's own pallets that the founding sequence never touches.
//!
//! `state_rehearsal` fills the register and opens the gate; `state_rehearsal_offices` seats the
//! offices and carries the budget, the payroll and an initiative. Between them they exercise
//! `IdentityKyc`, `Tiki`, `Welati`, `PezRewards`, `PezTreasury` and a corner of `StakingScore`
//! and `Trust`. Four custom pallets were not reached at all -- `Perwerde`, `Referral` and
//! `Messaging` on People, `TokenWrapper` on the Asset Hub -- and neither were two paths of
//! `Welati` that landed after the rest: the silence rule for the two elected offices (WP-19)
//! and the appointed Rewsenbîr.
//!
//! An unexercised pallet in a runtime is not a neutral fact. Its calls are live on the day the
//! chain starts, its storage keys freeze at genesis, and the first person to find out whether
//! the origin it was wired to can actually sign is a citizen. That is the same reason the
//! tracks are each carried once: a path nobody has walked is indistinguishable from a broken
//! one until somebody walks it.
//!
//! **Every stage reads the effect back from storage.** An extrinsic that finalised has been
//! included, which is not the same as having done what it was sent to do -- the house carried a
//! motion whose call failed, once, and only an event nobody read said so. So each stage names
//! the record the call is supposed to change and compares it before and after.
//!
//! **Every stage also proves one refusal.** A gate that has only ever been seen to open has
//! not been seen to work: a check that never fires is indistinguishable from a check that was
//! never wired. Each negative here asserts the *named* error, because a refusal for the wrong
//! reason passes a test that only asks whether something was refused.
//!
//! The actors are the register's own cohort -- citizens admitted through the path the forty
//! million will take, not accounts conjured for the purpose -- picked by fixed index so a
//! failure names the same account on every run.

use anyhow::anyhow;
use pezkuwi_zombienet_sdk::{
	subxt::{
		dynamic::{self, At, Value},
		ext::scale_value,
		tx::DynamicPayload,
		OnlineClient, PezkuwiConfig,
	},
	subxt_signer::{
		sr25519::{dev, Keypair},
		SecretUri,
	},
};
use std::str::FromStr;

use super::state_rehearsal_offices::{
	fund_seats, has_tiki, office_call_on_people, raw_account, root_call_on_people, storage_value,
	tiki_holder,
};

// ---------------------------------------------------------------------------------------
// Who plays which part
// ---------------------------------------------------------------------------------------
//
// Indices into the register's cohort. Fixed rather than chosen at run time so that a failure
// names the same account every run, and spread out so that no account carries two parts whose
// effects could be mistaken for each other -- a teacher who is also on the board would be
// refused as `OwnerCannotRatify` and the test would be measuring its own casting.

/// Holds `WezireBelaw` for the education stages.
const EDUCATION_MINISTER: usize = 0;
/// The honorary board starts here and runs for `RatificationsRequired` places.
const BOARD_START: usize = 1;
/// The first index the board may not reach; `BOARD_START..TEACHER` is the most it can occupy.
const TEACHER: usize = 10;
/// Two who pass every course and one who fails the first, so the record shows both.
const STUDENTS: [usize; 3] = [11, 12, 13];
/// Nominated by the education minister and approved by the President.
const REWSENBIR_APPOINTEE: usize = 20;
/// Nominated by a President who also holds the portfolio -- and so never approved.
const REWSENBIR_SELF_APPROVED: usize = 21;
/// A citizen with no stake reported, and therefore no trust.
const UNSTAKED_CITIZEN: usize = 30;
/// A citizen who never registered a messaging key.
const KEYLESS_CITIZEN: usize = 31;
/// Claims to have brought the newcomer in, and is named by the newcomer.
const INVITER: usize = 40;
/// Also claims the newcomer, and is not named.
const STRANGER: usize = 41;
/// The smallest cohort the parts above fit in, with room left for a voucher who has vouched
/// for nobody yet.
const MIN_COHORT: usize = 60;

/// The most blocks the Perwerde stage will wait for `MinCourseDuration` to pass.
///
/// Two hundred People blocks is twenty minutes at six seconds. Past that the rehearsal is no
/// longer a rehearsal of the course, it is a rehearsal of waiting, and the honest report is
/// which constant made it wait.
const COURSE_WAIT_BUDGET_BLOCKS: u128 = 200;

/// A stake to report for the two messaging correspondents.
///
/// Trust is gated absolutely on a stake, and `Messaging` asks for `MinTrustScore` -- twenty on
/// a scale of a thousand. Any reported stake clears it: the smallest staking tier normalises to
/// two hundred and staking is weighted at a fifth, so a stake alone is worth forty. The amount
/// is the same order as the tracks file uses, for the same reason: a citizen who stakes a
/// little is the common case and the one worth rehearsing.
const MESSAGING_STAKE: u128 = 10_000_000_000_000;

/// HEZ wrapped in the TokenWrapper stage: ten, in the smallest unit.
const WRAP_AMOUNT: u128 = 10_000_000_000_000;

/// An upper bound on what one extrinsic may cost in fees, for reading a balance change.
///
/// Not a claim about the fee schedule. The wrap stage checks that the sender lost the wrapped
/// amount *plus something small*; without a ceiling, a wrap that also quietly took a second
/// amount would pass.
const FEE_CEILING: u128 = 1_000_000_000_000;

/// The asset the wrapper mints. `WrappedAssetId` on both hubs.
const WHEZ_ASSET_ID: u32 = 2;

/// How long to wait for a block number to be reached on People.
const BLOCK_POLL_SECS: u64 = 6;

// ---------------------------------------------------------------------------------------
// Small readers
// ---------------------------------------------------------------------------------------

/// Submit and wait for finality, returning the chain's refusal as text.
///
/// Text rather than the error type because every caller here does one of two things with a
/// refusal: reports it, or checks that it names the error it was supposed to. Both want the
/// rendered `Pallet::Variant`, and a submission that never reached the chain renders without
/// one -- which is exactly how `expect_refusal` tells the two apart.
async fn submit(
	api: &OnlineClient<PezkuwiConfig>,
	tx: &DynamicPayload,
	signer: &Keypair,
) -> Result<(), String> {
	let progress = match api.tx().sign_and_submit_then_watch_default(tx, signer).await {
		Ok(progress) => progress,
		// A pool refusal reads "Invalid Transaction (1010)" and nothing else in `{e}`; the
		// reason -- fees, a stale nonce, an extension -- is in the error's data, and the
		// signer's balance and nonce settle most of them. Both are written out, because the
		// alternative is a guess and a cycle costs an hour and a half.
		Err(e) => {
			let account = storage_value(api, "System", "Account", vec![account_key(signer)])
				.await
				.ok()
				.flatten()
				.map(|v| format!("{v}"))
				.unwrap_or_else(|| "no account".into());
			return Err(format!(
				"the submission itself failed: {e}; detail: {e:?}; the signer's account: {account}"
			));
		},
	};
	progress
		.wait_for_finalized_success()
		.await
		.map(|_| ())
		.map_err(|e| e.to_string())
}

/// Submit, and fail with `what` in the message if the chain refuses.
async fn must(
	api: &OnlineClient<PezkuwiConfig>,
	tx: &DynamicPayload,
	signer: &Keypair,
	what: &str,
) -> Result<(), anyhow::Error> {
	submit(api, tx, signer).await.map_err(|e| anyhow!("{what}: {e}"))
}

/// Submit something that must be refused, and refused for the named reason.
///
/// The reason is the point. A call refused for want of fees, for a malformed argument or for
/// a different rule than the one under test passes a check that only asks whether it failed,
/// and then the rule it was meant to prove has been proved by nothing.
async fn expect_refusal(
	api: &OnlineClient<PezkuwiConfig>,
	tx: &DynamicPayload,
	signer: &Keypair,
	error: &str,
	what: &str,
) -> Result<(), anyhow::Error> {
	match submit(api, tx, signer).await {
		Ok(()) => Err(anyhow!(
			"{what} was accepted; it should have been refused with `{error}`. The rule it is \
			 meant to prove is not in force on this runtime"
		)),
		Err(e) if e.contains(error) => {
			log::info!("{what}: refused as expected ({error})");
			Ok(())
		},
		Err(e) => Err(anyhow!(
			"{what} was refused, but not with `{error}`: {e}. A refusal for another reason \
			 proves nothing about the rule under test"
		)),
	}
}

/// Fold the primitives of a decoded value into bytes, or `None` if any of them is not a byte.
///
/// An `AccountId32` decodes as a newtype around `[u8; 32]`, a `BoundedVec<u8, _>` as a newtype
/// around a list of `u8`, and both render as tuples of numbers. Comparing renderings would tie
/// every check to `scale_value`'s display format; walking the tree and comparing bytes ties it
/// to nothing but the encoding.
fn bytes_of<T>(v: &scale_value::Value<T>) -> Option<Vec<u8>> {
	fn walk<T>(v: &scale_value::Value<T>, out: &mut Vec<u8>) -> bool {
		match &v.value {
			scale_value::ValueDef::Primitive(scale_value::Primitive::U128(n)) if *n < 256 => {
				out.push(*n as u8);
				true
			},
			scale_value::ValueDef::Composite(c) => c.values().all(|x| walk(x, out)),
			_ => false,
		}
	}
	let mut out = Vec::new();
	walk(v, &mut out).then_some(out)
}

/// Is this decoded account the given key?
fn is_account<T>(v: &scale_value::Value<T>, who: &Keypair) -> bool {
	bytes_of(v).as_deref() == Some(&who.public_key().to_account_id().0[..])
}

/// The items of a decoded list, seeing through one `BoundedVec` wrapper.
///
/// The same trap `bounded_len` in the offices file documents: a `BoundedVec` is described as a
/// one-field composite whose field is the vector, so the outer list has one element whatever
/// the inner holds. The discriminator is the child's *shape*: the wrapper's only child is an
/// unnamed list, while a one-message inbox's only child is a named struct.
fn list_items(v: Value) -> Vec<Value> {
	let scale_value::ValueDef::Composite(scale_value::Composite::Unnamed(items)) = v.value else {
		return Vec::new();
	};
	let wrapped = items.len() == 1
		&& matches!(
			&items[0].value,
			scale_value::ValueDef::Composite(scale_value::Composite::Unnamed(_))
		);
	if !wrapped {
		return items;
	}
	match items.into_iter().next().map(|inner| inner.value) {
		Some(scale_value::ValueDef::Composite(scale_value::Composite::Unnamed(inner))) => inner,
		_ => Vec::new(),
	}
}

/// A pallet constant as a number, read from the metadata of the chain that runs it.
///
/// Every bound a stage depends on is read this way rather than copied, so a `fast-runtime`
/// build and a production build are each measured against what they actually run -- and the
/// message when a bound makes a stage impossible quotes the number the chain gave.
fn constant_u128(
	api: &OnlineClient<PezkuwiConfig>,
	pallet: &str,
	name: &str,
) -> Result<u128, anyhow::Error> {
	let addr = dynamic::constant::<Value>(pallet, name);
	let v: Value = api
		.constants()
		.at(&addr)
		.map_err(|e| anyhow!("reading the constant {pallet}::{name}: {e}"))?;
	v.as_u128()
		.ok_or_else(|| anyhow!("{pallet}::{name} did not decode as a number: {v}"))
}

/// A storage value as a number, zero when the key is absent.
///
/// Only for `ValueQuery` items, where absence *is* zero. Using it on an `OptionQuery` would
/// read a missing record as a record of nothing.
async fn number_at(
	api: &OnlineClient<PezkuwiConfig>,
	pallet: &str,
	item: &str,
	keys: Vec<Value>,
) -> Result<u128, anyhow::Error> {
	Ok(storage_value(api, pallet, item, keys)
		.await?
		.and_then(|v| v.as_u128())
		.unwrap_or(0))
}

/// The chain's current block number.
async fn block_number(api: &OnlineClient<PezkuwiConfig>) -> Result<u128, anyhow::Error> {
	storage_value(api, "System", "Number", Vec::new())
		.await?
		.and_then(|v| v.as_u128())
		.ok_or_else(|| anyhow!("System::Number did not decode as a number"))
}

/// A cohort member addressed as a storage key or call argument.
fn account_key(k: &Keypair) -> Value {
	raw_account(k)
}

fn variant(name: &str) -> Value {
	Value::unnamed_variant(name, vec![])
}

/// Does the unique office `tiki` sit with `who`?
async fn holds_office(
	people: &OnlineClient<PezkuwiConfig>,
	tiki: &str,
	who: &Keypair,
) -> Result<bool, anyhow::Error> {
	Ok(tiki_holder(people, tiki).await?.map(|v| is_account(&v, who)).unwrap_or(false))
}

/// The cohort, checked to be large enough for every part this file hands out.
fn cast(cohort: &[Keypair]) -> Result<&[Keypair], anyhow::Error> {
	if cohort.len() < MIN_COHORT {
		return Err(anyhow!(
			"the register's cohort holds {} citizens and the pallet stages need {MIN_COHORT}: \
			 the genesis seated more founding citizens than this file assumed, so fewer were \
			 admitted to reach the gate. Lower the fixed indices or raise the gate",
			cohort.len()
		));
	}
	Ok(cohort)
}

/// Put the education portfolio in `who`'s hands, by the Prime Minister's appointment.
///
/// `appoint_minister` moves a unique portfolio in one step -- the old holder loses it as the
/// new one gains it -- so this is also how the portfolio is taken back from somebody.
async fn seat_education_minister(
	people: &OnlineClient<PezkuwiConfig>,
	pm: &Keypair,
	who: &Keypair,
) -> Result<(), anyhow::Error> {
	if !holds_office(people, "SerokWeziran", pm).await? {
		return Err(anyhow!(
			"nobody the rehearsal seated holds `SerokWeziran`, so no minister can be appointed. \
			 The offices stage confirms the Prime Minister -- look at its failure first"
		));
	}
	let target = who.clone();
	office_call_on_people(
		people,
		pm,
		"Welati",
		"appoint_minister",
		vec![raw_account(who), variant("WezireBelaw")],
		|| {
			let people = people;
			let target = target.clone();
			async move { holds_office(people, "WezireBelaw", &target).await }
		},
	)
	.await
	.map_err(|e| anyhow!("the Prime Minister could not hand over the education portfolio: {e}"))
}

// ---------------------------------------------------------------------------------------
// The appointed Rewsenbîr
// ---------------------------------------------------------------------------------------

/// A Rewsenbîr is proposed by the education minister and approved by the President, and the
/// two are never the same person.
///
/// The title is the education system's to confer, and appointing one is the bootstrap for the
/// years before it has graduated anybody. So the rule has two halves and both are walked here:
/// only `WezireBelaw` may propose a name -- not any minister, and not the President -- and the
/// President may not approve a name he proposed himself, which is the case where one person
/// holds both offices.
///
/// The cap, `MaxAppointedRewsenbir`, is read and the count is checked to move by exactly one.
/// Walking it to the cap would take a hundred appointments and prove only that a counter
/// counts; that `count_an_appointed_rewsenbir` is reached on the approval path is the part
/// worth seeing, and the count moving is what shows it.
pub(crate) async fn an_appointed_rewsenbir_takes_two_hands(
	people: &OnlineClient<PezkuwiConfig>,
	bench: &[Keypair],
	cohort: &[Keypair],
) -> Result<(), anyhow::Error> {
	let cohort = cast(cohort)?;
	let serok = &bench[0];
	let pm = &bench[4];
	let minister = &cohort[EDUCATION_MINISTER];
	let appointee = &cohort[REWSENBIR_APPOINTEE];
	let self_approved = &cohort[REWSENBIR_SELF_APPROVED];

	// The nominees need nothing -- the calls are the nominator's and the approver's -- but the
	// minister signs, and a portfolio does not come with a balance.
	fund_seats(people, &[minister.clone()]).await?;

	let cap = constant_u128(people, "Welati", "MaxAppointedRewsenbir")?;
	let before = number_at(people, "Welati", "AppointedRewsenbirCount", Vec::new()).await?;
	log::info!("appointed Rewsenbîr: {before} of a lifetime cap of {cap}");
	if before >= cap {
		return Err(anyhow!(
			"`AppointedRewsenbirCount` is already {before} against a cap of {cap} before this \
			 stage appointed anybody -- the genesis or an earlier stage spent the places"
		));
	}

	let nominate = |who: &Keypair, why: &[u8]| {
		dynamic::tx(
			"Welati",
			"nominate_official",
			vec![raw_account(who), variant("Rewsenbîr"), Value::from_bytes(why.to_vec())],
		)
	};

	// ---- the President, without the portfolio, cannot propose ----------------------------
	//
	// He passes the first check -- the President may nominate to most offices -- and must fall
	// at the second. That ordering is why this is worth asserting by name: a refusal for
	// `NotAuthorizedToNominate` would mean the President had lost the general power, not that
	// the Rewsenbîr rule held.
	if holds_office(people, "WezireBelaw", serok).await? {
		return Err(anyhow!(
			"the President already holds the education portfolio before this stage handed it \
			 to him; the stage cannot show the rule refusing a President without it"
		));
	}
	expect_refusal(
		people,
		&nominate(appointee, b"rehearsal: proposed by a President without the portfolio"),
		serok,
		"OnlyTheEducationMinisterNominatesRewsenbir",
		"a Rewsenbîr nomination by a President who is not the education minister",
	)
	.await?;

	// ---- the President with the portfolio cannot approve his own name ---------------------
	//
	// Nothing in the register stops one person holding both offices, so the rule has to stand
	// at the approval. Without it, a President who took the education portfolio would propose
	// and approve the same title alone -- the two-hands design reduced to one.
	log::info!("handing the education portfolio to the President, to test self-approval");
	seat_education_minister(people, pm, serok).await?;
	must(
		people,
		&nominate(self_approved, b"rehearsal: proposed and approved by one person"),
		serok,
		"the President, holding the education portfolio, could not nominate a Rewsenbîr",
	)
	.await?;
	let self_process = number_at(people, "Welati", "NextAppointmentId", Vec::new())
		.await?
		.checked_sub(1)
		.ok_or_else(|| anyhow!("a nomination was accepted and no appointment process exists"))?;
	let pending =
		storage_value(people, "Welati", "AppointmentProcesses", vec![Value::u128(self_process)])
			.await?
			.map(|v| format!("{v}"))
			.unwrap_or_default();
	if !pending.contains("WaitingPresidentialApproval") {
		return Err(anyhow!(
			"a Rewsenbîr nomination should wait on the President, and process {self_process} \
			 reads: {pending}. If it waits on Parliament, `ConfirmationRequired` or \
			 `requires_parliament_approval` has moved the title -- the self-approval rule then \
			 guards nothing"
		));
	}
	expect_refusal(
		people,
		&dynamic::tx("Welati", "approve_appointment", vec![Value::u128(self_process)]),
		serok,
		"CannotApproveOwnNomination",
		"the President approving a Rewsenbîr he nominated himself",
	)
	.await?;
	if has_tiki(people, self_approved, "Rewsenbîr").await? {
		return Err(anyhow!(
			"the self-approval was refused and the nominee holds the title anyway -- something \
			 other than the approval granted it"
		));
	}

	// ---- the two-handed path ---------------------------------------------------------------
	log::info!("handing the education portfolio to its own minister");
	seat_education_minister(people, pm, minister).await?;
	if holds_office(people, "WezireBelaw", serok).await? {
		return Err(anyhow!(
			"the portfolio was given to the minister and the President still holds it -- \
			 `seat_unique_tiki` did not move it"
		));
	}
	must(
		people,
		&nominate(appointee, b"rehearsal: proposed by the education minister"),
		minister,
		"the education minister could not nominate a Rewsenbîr",
	)
	.await?;
	let process = number_at(people, "Welati", "NextAppointmentId", Vec::new())
		.await?
		.checked_sub(1)
		.ok_or_else(|| anyhow!("a nomination was accepted and no appointment process exists"))?;
	must(
		people,
		&dynamic::tx("Welati", "approve_appointment", vec![Value::u128(process)]),
		serok,
		"the President could not approve the education minister's Rewsenbîr",
	)
	.await?;

	if !has_tiki(people, appointee, "Rewsenbîr").await? {
		return Err(anyhow!(
			"the appointment was approved and the appointee holds no `Rewsenbîr` tiki -- the \
			 approval wrote to this pallet's records and not to the register every other \
			 authority check reads"
		));
	}
	let after = number_at(people, "Welati", "AppointedRewsenbirCount", Vec::new()).await?;
	if after != before + 1 {
		return Err(anyhow!(
			"one Rewsenbîr was appointed and `AppointedRewsenbirCount` went from {before} to \
			 {after}. The cap is only a cap if every completed appointment spends exactly one \
			 place -- and the refused self-approval must have spent none"
		));
	}
	log::info!("Rewsenbîr appointed by two hands; {after} of {cap} places now spent");
	Ok(())
}

// ---------------------------------------------------------------------------------------
// Perwerde
// ---------------------------------------------------------------------------------------

/// What the opening half of the Perwerde stage leaves for the closing half.
///
/// Split in two because a course must run `MinCourseDuration` before its results can be
/// submitted, and that wait is the longest in this file. Opening the courses before the other
/// stages and closing them after lets the wait run underneath work that has to happen anyway.
pub(crate) struct OpenCourses {
	ids: Vec<u32>,
	teacher: Keypair,
	students: Vec<Keypair>,
	board: Vec<Keypair>,
	points: u128,
	/// The student who fails `ids[0]` and passes the rest.
	failing: usize,
	/// Each student's score and completed count when the courses opened.
	baseline: Vec<(u128, u128)>,
}

/// The minister seeds a board, a course is opened naming a teacher, students enrol, and the
/// teacher records results -- which count for nothing yet.
///
/// Enough courses are opened for the earned title to be reachable: `MinCoursesForRole`, or as
/// many as `RewsenbirThreshold` needs at `MaxPointsPerCourse` each, whichever is more. A single
/// course would prove the ratification and could never prove the title, because the pallet
/// deliberately refuses to let one large course carry anybody to one.
pub(crate) async fn the_ministry_seeds_a_board_and_opens_courses(
	people: &OnlineClient<PezkuwiConfig>,
	bench: &[Keypair],
	cohort: &[Keypair],
) -> Result<OpenCourses, anyhow::Error> {
	let cohort = cast(cohort)?;
	let serok = &bench[0];
	let pm = &bench[4];
	let minister = &cohort[EDUCATION_MINISTER];

	let required = constant_u128(people, "Perwerde", "RatificationsRequired")? as usize;
	let honorary_cap = constant_u128(people, "Perwerde", "MaxHonoraryMamoste")?;
	let points = constant_u128(people, "Perwerde", "MaxPointsPerCourse")?;
	let min_courses = constant_u128(people, "Perwerde", "MinCoursesForRole")?;
	let threshold = constant_u128(people, "Perwerde", "RewsenbirThreshold")?;
	if BOARD_START + required > TEACHER {
		return Err(anyhow!(
			"`RatificationsRequired` is {required}, and the board's places in the cohort run \
			 out at {} -- widen `BOARD_START..TEACHER`",
			TEACHER - BOARD_START
		));
	}
	if required as u128 > honorary_cap {
		return Err(anyhow!(
			"a course needs {required} ratifications and the minister may seed only \
			 {honorary_cap} teachers: on a new chain no course could ever be closed. That is a \
			 runtime defect, not a rehearsal one"
		));
	}
	if points == 0 {
		return Err(anyhow!("`MaxPointsPerCourse` is zero, so no course can be worth anything"));
	}
	let courses = min_courses.max(threshold.div_ceil(points)).max(1);
	if courses > 10 {
		return Err(anyhow!(
			"the earned title needs {courses} courses at {points} points each (threshold \
			 {threshold}, at least {min_courses} courses) -- more than a rehearsal should open"
		));
	}

	let board: Vec<Keypair> = cohort[BOARD_START..BOARD_START + required].to_vec();
	let teacher = cohort[TEACHER].clone();
	let students: Vec<Keypair> = STUDENTS.iter().map(|i| cohort[*i].clone()).collect();
	let mut actors = vec![minister.clone(), teacher.clone()];
	actors.extend(board.iter().cloned());
	actors.extend(students.iter().cloned());
	fund_seats(people, &actors).await?;

	seat_education_minister(people, pm, minister).await?;

	// ---- the board ----------------------------------------------------------------------
	//
	// Seeding teachers is the minister's and nobody else's: grading is where the power is, and
	// a bootstrap anybody could invoke would be a standing power to print standing. Refused
	// first for a citizen, so the seeding that follows is known to be the origin working and
	// not the origin being open.
	expect_refusal(
		people,
		&dynamic::tx("Perwerde", "appoint_honorary_mamoste", vec![account_key(&students[0])]),
		&students[0],
		// subxt renders `DispatchError::BadOrigin` as prose, not as the variant name.
		"Bad origin",
		"a citizen seeding a teacher",
	)
	.await?;

	let seeded_before = number_at(people, "Perwerde", "HonoraryMamosteCount", Vec::new()).await?;
	let mut to_seed = Vec::new();
	for member in &board {
		if !has_tiki(people, member, "Mamoste").await? {
			to_seed.push(
				dynamic::tx("Perwerde", "appoint_honorary_mamoste", vec![account_key(member)])
					.into_value(),
			);
		}
	}
	let seeding = to_seed.len() as u128;
	if !to_seed.is_empty() {
		// One batch, signed by the minister: `batch_all` dispatches each call with the signed
		// origin it was given, so `EnsureTiki<WezireBelaw>` sees the minister in every one.
		must(
			people,
			&dynamic::tx("Utility", "batch_all", vec![Value::unnamed_composite(to_seed)]),
			minister,
			"the education minister could not seed the board",
		)
		.await?;
	}
	for member in &board {
		if !has_tiki(people, member, "Mamoste").await? {
			return Err(anyhow!(
				"the minister's seeding was accepted and a board member holds no `Mamoste` \
				 tiki -- `EarnedRoles::grant_earned` refused, most likely because the member has \
				 no citizen NFT"
			));
		}
		if storage_value(people, "Perwerde", "HonoraryMamoste", vec![account_key(member)])
			.await?
			.is_none()
		{
			return Err(anyhow!(
				"a board member holds `Mamoste` and is not recorded as honorary -- the chain can \
				 no longer say how much of its examiner corps was seeded"
			));
		}
	}
	let seeded = number_at(people, "Perwerde", "HonoraryMamosteCount", Vec::new()).await?;
	if seeded != seeded_before + seeding {
		return Err(anyhow!(
			"{seeding} teachers were seeded and `HonoraryMamosteCount` went from {seeded_before} \
			 to {seeded}"
		));
	}
	log::info!("the board is seated: {required} honorary teachers, {seeded} seeded in all");

	// ---- the courses --------------------------------------------------------------------
	//
	// Opened by the President, who is one of the three hands `PerwerdeAdminOrigin` accepts and
	// the only one with a key: Root and the council resolve to a keyless account, which is why
	// the teacher is named in the call rather than taken from the origin.
	expect_refusal(
		people,
		&create_course(&teacher, b"Too valuable", points + 1),
		serok,
		"PointsExceedMax",
		"a course worth more than `MaxPointsPerCourse`",
	)
	.await?;

	let first = number_at(people, "Perwerde", "NextCourseId", Vec::new()).await? as u32;
	let opens: Vec<Value> = (0..courses)
		.map(|n| {
			create_course(&teacher, format!("Rehearsal course {n}").as_bytes(), points).into_value()
		})
		.collect();
	must(
		people,
		&dynamic::tx("Utility", "batch_all", vec![Value::unnamed_composite(opens)]),
		serok,
		"the President could not open the courses",
	)
	.await?;
	let next = number_at(people, "Perwerde", "NextCourseId", Vec::new()).await? as u32;
	let ids: Vec<u32> = (first..next).collect();
	if ids.len() as u128 != courses {
		return Err(anyhow!(
			"{courses} courses were opened and `NextCourseId` moved from {first} to {next}"
		));
	}
	for id in &ids {
		let course = storage_value(people, "Perwerde", "Courses", vec![Value::u128(*id as u128)])
			.await?
			.ok_or_else(|| anyhow!("course {id} was counted and not stored"))?;
		let owner_ok = course.at("owner").map(|o| is_account(o, &teacher)).unwrap_or(false);
		let status = course.at("status").map(|s| format!("{s}")).unwrap_or_default();
		if !owner_ok || !status.contains("Enrolling") {
			return Err(anyhow!(
				"course {id} should be open and taught by the named teacher, and reads: {course}"
			));
		}
	}
	log::info!("courses {ids:?} are open, each worth {points}");

	// ---- the class ----------------------------------------------------------------------
	//
	// Each student enrols in every course in one batch; the students run beside each other
	// because each has a nonce of their own.
	let enrolments = students.iter().map(|student| {
		let calls: Vec<Value> = ids
			.iter()
			.map(|id| {
				dynamic::tx("Perwerde", "enroll", vec![Value::u128(*id as u128)]).into_value()
			})
			.collect();
		let batch = dynamic::tx("Utility", "batch_all", vec![Value::unnamed_composite(calls)]);
		let student = student.clone();
		async move { must(people, &batch, &student, "a citizen could not enrol").await }
	});
	futures::future::try_join_all(enrolments).await?;
	for id in &ids {
		let enrolled =
			number_at(people, "Perwerde", "CourseEnrollmentCount", vec![Value::u128(*id as u128)])
				.await?;
		if enrolled != students.len() as u128 {
			return Err(anyhow!(
				"{} students enrolled in course {id} and the course counts {enrolled}",
				students.len()
			));
		}
	}

	// Teaching yourself is not a credential.
	expect_refusal(
		people,
		&dynamic::tx("Perwerde", "enroll", vec![Value::u128(ids[0] as u128)]),
		&teacher,
		"OwnerCannotEnrol",
		"a teacher enrolling in their own course",
	)
	.await?;

	// ---- the results, as drafts ---------------------------------------------------------
	let failing = students.len() - 1;
	let baseline = {
		let mut b = Vec::new();
		for s in &students {
			b.push((
				number_at(people, "Perwerde", "PerwerdeScores", vec![account_key(s)]).await?,
				number_at(people, "Perwerde", "CompletedCourses", vec![account_key(s)]).await?,
			));
		}
		b
	};
	let mut results = Vec::new();
	for (n, s) in students.iter().enumerate() {
		for (k, id) in ids.iter().enumerate() {
			let passed = !(n == failing && k == 0);
			results.push(
				dynamic::tx(
					"Perwerde",
					"record_result",
					vec![account_key(s), Value::u128(*id as u128), Value::bool(passed)],
				)
				.into_value(),
			);
		}
	}
	must(
		people,
		&dynamic::tx("Utility", "batch_all", vec![Value::unnamed_composite(results)]),
		&teacher,
		"the teacher could not record the results",
	)
	.await?;

	let draft = storage_value(
		people,
		"Perwerde",
		"Enrollments",
		vec![Value::u128(ids[0] as u128), account_key(&students[failing])],
	)
	.await?
	.ok_or_else(|| anyhow!("an enrolment the course counted is not stored"))?;
	let recorded = draft.at("passed").map(|p| format!("{p}")).unwrap_or_default();
	if !recorded.contains("false") {
		return Err(anyhow!(
			"the teacher recorded a fail and the enrolment reads `passed: {recorded}`"
		));
	}
	// A draft changes nothing. Before the board ratifies, a teacher's word is worth no points
	// at all -- that is the whole of the reform that took grading out of one person's hands.
	for (s, (score, done)) in students.iter().zip(&baseline) {
		let now = number_at(people, "Perwerde", "PerwerdeScores", vec![account_key(s)]).await?;
		let completed =
			number_at(people, "Perwerde", "CompletedCourses", vec![account_key(s)]).await?;
		if now != *score || completed != *done {
			return Err(anyhow!(
				"a recorded but unratified result moved a student's record: score {score} -> \
				 {now}, completed {done} -> {completed}. A draft must count for nothing"
			));
		}
	}
	expect_refusal(
		people,
		&dynamic::tx("Perwerde", "ratify_results", vec![Value::u128(ids[0] as u128)]),
		&board[0],
		"NotAwaitingRatification",
		"the board ratifying a course whose results were never submitted",
	)
	.await?;

	log::info!("results recorded as drafts on {} courses; the courses now have to run", ids.len());
	Ok(OpenCourses { ids, teacher, students, board, points, failing, baseline })
}

fn create_course(teacher: &Keypair, name: &[u8], points: u128) -> DynamicPayload {
	dynamic::tx(
		"Perwerde",
		"create_course",
		vec![
			raw_account(teacher),
			Value::from_bytes(name.to_vec()),
			Value::from_bytes(b"Opened by the state rehearsal".to_vec()),
			Value::from_bytes(b"ipfs://rehearsal".to_vec()),
			Value::u128(points),
		],
	)
}

/// The courses close: submitted after `MinCourseDuration`, ratified by the board, and the
/// passes become points -- and, for a student with enough of them, a title.
///
/// This is the half `MinCourseDuration` decides. If the build leaves it at its production
/// length the stage cannot finish inside a rehearsal, and it says so with the number: it shows
/// the gate refusing an early submission, which proves the rule is in force, and then fails
/// naming the constant rather than passing on a path it never walked.
pub(crate) async fn the_board_ratifies_and_the_record_counts(
	people: &OnlineClient<PezkuwiConfig>,
	open: &OpenCourses,
) -> Result<(), anyhow::Error> {
	let OpenCourses { ids, teacher, students, board, points, failing, baseline } = open;
	let min_duration = constant_u128(people, "Perwerde", "MinCourseDuration")?;
	let required = constant_u128(people, "Perwerde", "RatificationsRequired")?;
	let min_courses = constant_u128(people, "Perwerde", "MinCoursesForRole")?;
	let threshold = constant_u128(people, "Perwerde", "RewsenbirThreshold")?;

	let mut opened_at = 0u128;
	for id in ids {
		let course = storage_value(people, "Perwerde", "Courses", vec![Value::u128(*id as u128)])
			.await?
			.ok_or_else(|| anyhow!("course {id} has disappeared"))?;
		let at = course
			.at("created_at")
			.and_then(|v| v.as_u128())
			.ok_or_else(|| anyhow!("course {id} carries no `created_at`: {course}"))?;
		opened_at = opened_at.max(at);
	}
	let ready_at = opened_at + min_duration;
	let now = block_number(people).await?;

	if ready_at > now + COURSE_WAIT_BUDGET_BLOCKS {
		// The rule is live -- that much a rehearsal can show.
		expect_refusal(
			people,
			&dynamic::tx("Perwerde", "submit_results", vec![Value::u128(ids[0] as u128)]),
			teacher,
			"TooEarlyToSubmit",
			"submitting a course's results before it has run",
		)
		.await?;
		return Err(anyhow!(
			"`Perwerde::MinCourseDuration` is {min_duration} blocks on this build, so the \
			 courses opened at block {opened_at} cannot be submitted before block {ready_at}; \
			 it is block {now} and this stage waits at most {COURSE_WAIT_BUDGET_BLOCKS}. \
			 Ratification, `PerwerdeScores`, `CompletedCourses` and the earned Rewsenbîr title \
			 are therefore unexercised. The gate itself refused an early submission, so the \
			 rule is in force; what is missing is a `fast-runtime` value -- \
			 `rehearsal_period!(90 * DAYS, DAYS)` gives ninety blocks, and \
			 `MaxCourseDuration` wants the same treatment so it stays the longer of the two"
		));
	}

	while block_number(people).await? < ready_at {
		tokio::time::sleep(std::time::Duration::from_secs(BLOCK_POLL_SECS)).await;
	}
	log::info!("the courses have run their minimum; submitting");

	let submits: Vec<Value> = ids
		.iter()
		.map(|id| {
			dynamic::tx("Perwerde", "submit_results", vec![Value::u128(*id as u128)]).into_value()
		})
		.collect();
	must(
		people,
		&dynamic::tx("Utility", "batch_all", vec![Value::unnamed_composite(submits)]),
		teacher,
		"the teacher could not submit the results",
	)
	.await?;

	// The teacher is never one of the board. Without this a teacher who was also a Mamoste
	// would be one signature closer to closing their own course.
	expect_refusal(
		people,
		&dynamic::tx("Perwerde", "ratify_results", vec![Value::u128(ids[0] as u128)]),
		teacher,
		"OwnerCannotRatify",
		"a teacher ratifying their own course",
	)
	.await?;

	// Every member ratifies every course, members beside each other.
	let ratifications = board.iter().map(|member| {
		let calls: Vec<Value> = ids
			.iter()
			.map(|id| {
				dynamic::tx("Perwerde", "ratify_results", vec![Value::u128(*id as u128)])
					.into_value()
			})
			.collect();
		let batch = dynamic::tx("Utility", "batch_all", vec![Value::unnamed_composite(calls)]);
		let member = member.clone();
		async move { must(people, &batch, &member, "a board member could not ratify").await }
	});
	futures::future::try_join_all(ratifications).await?;

	for id in ids {
		let course = storage_value(people, "Perwerde", "Courses", vec![Value::u128(*id as u128)])
			.await?
			.ok_or_else(|| anyhow!("course {id} has disappeared"))?;
		let status = course.at("status").map(|s| format!("{s}")).unwrap_or_default();
		let count =
			number_at(people, "Perwerde", "RatificationCount", vec![Value::u128(*id as u128)])
				.await?;
		if !status.contains("Completed") || count < required {
			return Err(anyhow!(
				"course {id} was ratified by {} teachers against {required} required and reads \
				 status `{status}` with {count} ratifications",
				board.len()
			));
		}
	}

	// ---- the record ---------------------------------------------------------------------
	let n = ids.len() as u128;
	for (i, s) in students.iter().enumerate() {
		let (score0, done0) = baseline[i];
		let passed = if i == *failing { n - 1 } else { n };
		let score = number_at(people, "Perwerde", "PerwerdeScores", vec![account_key(s)]).await?;
		let done = number_at(people, "Perwerde", "CompletedCourses", vec![account_key(s)]).await?;
		if score != score0 + passed * points || done != done0 + passed {
			return Err(anyhow!(
				"student {i} passed {passed} of {n} courses worth {points} each and the record \
				 reads score {score0} -> {score}, completed {done0} -> {done}"
			));
		}
		let titled = has_tiki(people, s, "Rewsenbîr").await?;
		let earned = done >= min_courses && score >= threshold;
		if titled != earned {
			return Err(anyhow!(
				"student {i} has {done} courses and {score} points against {min_courses} and \
				 {threshold}, and {} the Rewsenbîr title. `award_earned_roles` runs on every \
				 course that closes; a mismatch here is the threshold check or the grant",
				if titled { "holds" } else { "does not hold" }
			));
		}
	}
	log::info!(
		"Perwerde carried: {n} courses ratified, the record counts, and the title follows the \
		 record and nothing else"
	);
	Ok(())
}

// ---------------------------------------------------------------------------------------
// Referral
// ---------------------------------------------------------------------------------------

/// The referral graph the register built is on chain, and an invitation settles only when both
/// sides name each other.
///
/// Two halves, and the first is the one the founding sequence should already have produced.
/// Every applicant named a voucher, and `confirm_citizenship` hands that name to `Referral` --
/// so each cohort member must have a `Referrals` record naming a citizen admitted *before*
/// them, and each voucher's `ReferralCount` must equal the number of records naming them.
/// That is the `try_state` invariant, read off a live chain rather than a test externality.
///
/// The second half is the user-facing call, `initiate_referral`: a claim that becomes an
/// invitation only if the newcomer names the same account. Two people claim one newcomer, the
/// newcomer names one, and only that one is credited.
pub(crate) async fn the_referral_graph_is_recorded_and_an_invitation_settles(
	people: &OnlineClient<PezkuwiConfig>,
	cohort: &[Keypair],
) -> Result<(), anyhow::Error> {
	let cohort = cast(cohort)?;

	// ---- the graph the register left ----------------------------------------------------
	let founders = [dev::alice(), dev::bob()];
	let mut tally: std::collections::BTreeMap<[u8; 32], u128> = Default::default();
	for (i, member) in cohort.iter().enumerate() {
		let record = storage_value(people, "Referral", "Referrals", vec![account_key(member)])
			.await?
			.ok_or_else(|| {
				anyhow!(
					"cohort member {i} is a citizen and has no `Referrals` record -- \
					 `OnKycApproved` did not reach the referral pallet, or reached it while the \
					 status was not yet `Approved`"
				)
			})?;
		let referrer = record.at("referrer").ok_or_else(|| {
			anyhow!("cohort member {i}'s referral record has no referrer: {record}")
		})?;
		// A tree rooted at the founders: everybody was vouched in by somebody already there.
		let earlier = founders.iter().chain(cohort[..i].iter()).find(|k| is_account(referrer, k));
		let Some(voucher) = earlier else {
			return Err(anyhow!(
				"cohort member {i} is recorded as referred by {referrer}, who is neither a \
				 founder nor a citizen admitted before them. The register vouches in \
				 generations, so the graph must be a tree rooted at genesis"
			));
		};
		*tally.entry(voucher.public_key().to_account_id().0).or_default() += 1;
	}
	for (voucher, counted) in &tally {
		let key = vec![Value::from_bytes(voucher)];
		let count = number_at(people, "Referral", "ReferralCount", key.clone()).await?;
		let stats = storage_value(people, "Referral", "ReferrerStatsStorage", key)
			.await?
			.and_then(|v| v.at("total_referrals").and_then(|t| t.as_u128()))
			.unwrap_or(0);
		let is_founder = founders.iter().any(|f| f.public_key().to_account_id().0 == *voucher);
		// A founder may carry referrals from the genesis as well; a cohort member has nothing
		// but what this run gave them, so for them the numbers must agree exactly.
		let agrees = if is_founder { count >= *counted } else { count == *counted };
		if !agrees || stats != count {
			return Err(anyhow!(
				"a voucher is named by {counted} cohort records, and `ReferralCount` says {count} \
				 while `ReferrerStatsStorage.total_referrals` says {stats}. Three records of the \
				 same referrals disagree, and the trust score reads one of them"
			));
		}
	}
	log::info!(
		"referral graph recorded: {} citizens, {} vouchers, every record names an earlier citizen",
		cohort.len(),
		tally.len()
	);

	// ---- an invitation settles ----------------------------------------------------------
	let inviter = &cohort[INVITER];
	let stranger = &cohort[STRANGER];
	// A voucher with capacity left: somebody from the last generation, who has vouched for
	// nobody. Found by reading rather than by index, because how the generations fall depends
	// on how many founders the genesis seated.
	let mut voucher = None;
	for member in cohort.iter().rev() {
		if number_at(people, "Referral", "ReferralCount", vec![account_key(member)]).await? == 0 {
			voucher = Some(member.clone());
			break;
		}
	}
	let voucher =
		voucher.ok_or_else(|| anyhow!("every cohort member has already vouched for somebody"))?;
	let newcomer = Keypair::from_uri(
		&SecretUri::from_str("//Alice//invited//1").map_err(|e| anyhow!("newcomer: {e}"))?,
	)
	.map_err(|e| anyhow!("newcomer: {e}"))?;
	fund_seats(people, &[inviter.clone(), stranger.clone(), voucher.clone(), newcomer.clone()])
		.await?;

	expect_refusal(
		people,
		&dynamic::tx("Referral", "initiate_referral", vec![account_key(inviter)]),
		inviter,
		"SelfReferral",
		"a citizen claiming to have invited themselves",
	)
	.await?;

	let claims = [inviter, stranger].map(|claimant| {
		let claim = dynamic::tx("Referral", "initiate_referral", vec![account_key(&newcomer)]);
		let claimant = claimant.clone();
		async move { must(people, &claim, &claimant, "a citizen could not record a claim").await }
	});
	futures::future::try_join_all(claims).await?;
	for claimant in [inviter, stranger] {
		if storage_value(
			people,
			"Referral",
			"Invitations",
			vec![account_key(&newcomer), account_key(claimant)],
		)
		.await?
		.is_none()
		{
			return Err(anyhow!("a claim was accepted and `Invitations` does not hold it"));
		}
	}
	let invited_before =
		number_at(people, "Referral", "InvitationCount", vec![account_key(inviter)]).await?;
	let stranger_before =
		number_at(people, "Referral", "InvitationCount", vec![account_key(stranger)]).await?;

	// The newcomer names the voucher who will stand for them and the inviter who brought them,
	// and the two are different people -- which is the case the separate record exists for.
	let mut hash = [0u8; 32];
	hash[..8].copy_from_slice(b"rhslrfrl");
	let apply = dynamic::tx(
		"IdentityKyc",
		"apply_for_citizenship",
		vec![
			Value::from_bytes(hash),
			Value::unnamed_variant("Some", vec![account_key(&voucher)]),
			Value::unnamed_variant("Some", vec![account_key(inviter)]),
		],
	);
	must(people, &apply, &newcomer, "the newcomer could not apply").await?;
	must(
		people,
		&dynamic::tx("IdentityKyc", "approve_referral", vec![account_key(&newcomer)]),
		&voucher,
		"the voucher could not approve the newcomer",
	)
	.await?;
	let vouched_before =
		number_at(people, "Referral", "ReferralCount", vec![account_key(&voucher)]).await?;
	must(
		people,
		&dynamic::tx("IdentityKyc", "confirm_citizenship", Vec::<Value>::new()),
		&newcomer,
		"the newcomer could not confirm citizenship",
	)
	.await?;

	let invited_by = storage_value(people, "Referral", "InvitedBy", vec![account_key(&newcomer)])
		.await?
		.ok_or_else(|| {
			anyhow!(
				"the newcomer named an inviter who had claimed them, and `InvitedBy` is empty -- \
				 the two statements did not settle"
			)
		})?;
	if !is_account(&invited_by, inviter) {
		return Err(anyhow!("`InvitedBy` names {invited_by}, not the inviter the newcomer named"));
	}
	let invited_after =
		number_at(people, "Referral", "InvitationCount", vec![account_key(inviter)]).await?;
	let stranger_after =
		number_at(people, "Referral", "InvitationCount", vec![account_key(stranger)]).await?;
	if invited_after != invited_before + 1 || stranger_after != stranger_before {
		return Err(anyhow!(
			"the inviter's count went {invited_before} -> {invited_after} and the stranger's \
			 {stranger_before} -> {stranger_after}; only the named claimant may be credited"
		));
	}
	if storage_value(
		people,
		"Referral",
		"Invitations",
		vec![account_key(&newcomer), account_key(stranger)],
	)
	.await?
	.is_some()
	{
		return Err(anyhow!(
			"the stranger's claim outlived the settlement -- a claim nobody confirmed is left \
			 standing against an account that is already a citizen"
		));
	}
	let record = storage_value(people, "Referral", "Referrals", vec![account_key(&newcomer)])
		.await?
		.ok_or_else(|| anyhow!("the newcomer is a citizen with no referral record"))?;
	if !record.at("referrer").map(|r| is_account(r, &voucher)).unwrap_or(false) {
		return Err(anyhow!(
			"the newcomer's referral names {record}, not the voucher who approved them -- the \
			 inviter and the voucher are different facts and were conflated"
		));
	}
	let vouched_after =
		number_at(people, "Referral", "ReferralCount", vec![account_key(&voucher)]).await?;
	if vouched_after != vouched_before + 1 {
		return Err(anyhow!(
			"the voucher's `ReferralCount` went {vouched_before} -> {vouched_after} for one \
			 citizen vouched in"
		));
	}

	// And once settled, nobody else can claim them.
	expect_refusal(
		people,
		&dynamic::tx("Referral", "initiate_referral", vec![account_key(&newcomer)]),
		stranger,
		"AlreadyReferred",
		"a claim on a citizen whose invitation has settled",
	)
	.await?;
	log::info!("referral carried: two claims, one named, one credited, the other cleared");
	Ok(())
}

// ---------------------------------------------------------------------------------------
// Messaging
// ---------------------------------------------------------------------------------------

/// A citizen with standing registers a key, another sends to it, and the inbox holds exactly
/// what was sent until it is acknowledged.
///
/// The gate is the trust score, and trust is gated on a stake -- so the correspondents are
/// given one the way the live chain gives it, a report from the relay, and a citizen without
/// one is shown being turned away. The chain never sees plaintext: the ciphertext here is
/// arbitrary bytes, and what is checked is that the chain stores them unaltered and addressed.
///
/// The era purge is not observed. `EraLength` is six hours of blocks and is not compressed in a
/// rehearsal build, so no rehearsal sees an era turn; the length is logged.
pub(crate) async fn a_citizen_writes_to_a_citizen(
	relay: &OnlineClient<PezkuwiConfig>,
	people: &OnlineClient<PezkuwiConfig>,
	cohort: &[Keypair],
) -> Result<(), anyhow::Error> {
	let cohort = cast(cohort)?;
	let sender = dev::alice();
	let recipient = dev::bob();
	let unstaked = &cohort[UNSTAKED_CITIZEN];
	let keyless = &cohort[KEYLESS_CITIZEN];
	let min_trust = constant_u128(people, "Messaging", "MinTrustScore")?;
	let era_length = constant_u128(people, "Messaging", "EraLength")?;
	log::info!("messaging asks for trust {min_trust}; an era is {era_length} blocks");

	for who in [&sender, &recipient] {
		give_standing(relay, people, who, min_trust).await.map_err(|e| {
			anyhow!("a correspondent could not be given the trust messaging asks for: {e}")
		})?;
	}
	if number_at(people, "Trust", "TrustScores", vec![account_key(unstaked)]).await? >= min_trust {
		return Err(anyhow!(
			"cohort member {UNSTAKED_CITIZEN} was meant to have no stake and has trust enough to \
			 message; something reported a stake for them"
		));
	}
	fund_seats(people, &[unstaked.clone(), keyless.clone()]).await?;

	// ---- the gate -----------------------------------------------------------------------
	expect_refusal(
		people,
		&dynamic::tx("Messaging", "register_encryption_key", vec![Value::from_bytes([0x5au8; 32])]),
		unstaked,
		"InsufficientTrustScore",
		"a citizen with no stake registering a messaging key",
	)
	.await?;

	// ---- a key, a message, an inbox ----------------------------------------------------
	let key = [0x42u8; 32];
	must(
		people,
		&dynamic::tx("Messaging", "register_encryption_key", vec![Value::from_bytes(key)]),
		&recipient,
		"a citizen with standing could not register a messaging key",
	)
	.await?;
	let stored =
		storage_value(people, "Messaging", "EncryptionKeys", vec![account_key(&recipient)])
			.await?
			.and_then(|v| bytes_of(&v));
	if stored.as_deref() != Some(&key[..]) {
		return Err(anyhow!("the key was registered and `EncryptionKeys` holds {stored:?}"));
	}

	let send = |to: &Keypair, ciphertext: &[u8]| {
		dynamic::tx(
			"Messaging",
			"send_message",
			vec![
				account_key(to),
				Value::from_bytes([0x11u8; 32]),
				Value::from_bytes([0x22u8; 24]),
				Value::from_bytes(ciphertext.to_vec()),
			],
		)
	};
	expect_refusal(
		people,
		&send(keyless, b"nobody can read this"),
		&sender,
		"RecipientNoEncryptionKey",
		"a message to a citizen who never published a key",
	)
	.await?;

	let ciphertext: Vec<u8> = (0u8..48).collect();
	// Start on a fresh era. `acknowledge_messages` clears the current era's inbox only -- an
	// earlier era's is left to the purge, by design -- so a send in one era and an
	// acknowledgement in the next leaves the message where this stage looks for it. The first
	// run with these stages did exactly that: an era is twenty blocks in a rehearsal build, and
	// a send, a read and an acknowledgement each wait for finality.
	let turned_from = number_at(people, "Messaging", "CurrentEra", Vec::new()).await?;
	let mut fresh = false;
	for _ in 0..(era_length as u64 * 2 * 6 / 2) {
		if number_at(people, "Messaging", "CurrentEra", Vec::new()).await? != turned_from {
			fresh = true;
			break;
		}
		tokio::time::sleep(std::time::Duration::from_secs(2)).await;
	}
	if !fresh {
		return Err(anyhow!(
			"the messaging era did not turn from {turned_from} in two era lengths ({era_length} \
			 blocks each) -- the era clock is not running"
		));
	}
	let era = number_at(people, "Messaging", "CurrentEra", Vec::new()).await?;
	let sent_before =
		number_at(people, "Messaging", "SendCount", vec![Value::u128(era), account_key(&sender)])
			.await?;
	must(people, &send(&recipient, &ciphertext), &sender, "a citizen could not send a message")
		.await?;
	let era_after = number_at(people, "Messaging", "CurrentEra", Vec::new()).await?;
	if era_after != era {
		return Err(anyhow!(
			"the era turned ({era} -> {era_after}) while the message was in flight, so the inbox \
			 cannot be read deterministically; rerun -- an era is {era_length} blocks"
		));
	}

	let inbox = storage_value(
		people,
		"Messaging",
		"Inbox",
		vec![Value::u128(era), account_key(&recipient)],
	)
	.await?
	.map(list_items)
	.unwrap_or_default();
	let delivered = inbox.iter().any(|m| {
		m.at("sender").map(|s| is_account(s, &sender)).unwrap_or(false)
			&& m.at("ciphertext").and_then(bytes_of).as_deref() == Some(&ciphertext[..])
	});
	if !delivered {
		return Err(anyhow!(
			"the message was accepted and the recipient's inbox for era {era} holds {} \
			 messages, none from the sender with the bytes sent",
			inbox.len()
		));
	}
	let sent_after =
		number_at(people, "Messaging", "SendCount", vec![Value::u128(era), account_key(&sender)])
			.await?;
	if sent_after != sent_before + 1 {
		return Err(anyhow!(
			"one message was sent and the sender's count for the era went {sent_before} -> \
			 {sent_after}; the rate limit counts on this number"
		));
	}

	must(
		people,
		&dynamic::tx("Messaging", "acknowledge_messages", Vec::<Value>::new()),
		&recipient,
		"the recipient could not acknowledge their inbox",
	)
	.await?;
	let left = storage_value(
		people,
		"Messaging",
		"Inbox",
		vec![Value::u128(era), account_key(&recipient)],
	)
	.await?
	.map(list_items)
	.unwrap_or_default();
	let era_at_ack = number_at(people, "Messaging", "CurrentEra", Vec::new()).await?;
	if era_at_ack != era {
		return Err(anyhow!(
			"the era turned ({era} -> {era_at_ack}) before the acknowledgement, which clears the \
			 current era only, so whether it cleared the message cannot be read; rerun -- an era \
			 is {era_length} blocks"
		));
	}
	if !left.is_empty() {
		return Err(anyhow!(
			"the recipient acknowledged and {} messages remain -- \"zero trace\" is the pallet's \
			 first promise",
			left.len()
		));
	}
	log::info!("messaging carried: key registered, message delivered intact, inbox cleared");
	Ok(())
}

/// Report a stake for `who` from the relay until their trust reaches `floor`.
///
/// More than once if it has to be, and the reason is a freeze rather than a flaky lane.
/// `PezRewards` holds every trust score still for the length of each claim window, so the
/// shares it pays out and the total they are divided by belong to one roll; a stake report
/// that lands inside a window is recorded by `StakingScore` and *not* turned into trust, and
/// nothing recomputes it when the window closes. Under `fast-runtime` a window is seven blocks
/// in every thirty, so roughly one report in four lands frozen. A real noter would simply
/// report again, and so does this -- after saying what it found.
async fn give_standing(
	relay: &OnlineClient<PezkuwiConfig>,
	people: &OnlineClient<PezkuwiConfig>,
	who: &Keypair,
	floor: u128,
) -> Result<(), anyhow::Error> {
	const ATTEMPTS: usize = 3;
	let mut last = None;
	for attempt in 1..=ATTEMPTS {
		let target = who.clone();
		match root_call_on_people(
			relay,
			people,
			"StakingScore",
			"receive_staking_details",
			vec![
				account_key(who),
				variant("RelayChain"),
				Value::u128(MESSAGING_STAKE),
				Value::u128(0),
				Value::u128(0),
			],
			|| {
				let people = people;
				let target = target.clone();
				async move {
					Ok(number_at(people, "Trust", "TrustScores", vec![account_key(&target)])
						.await? >= floor)
				}
			},
		)
		.await
		{
			Ok(()) => return Ok(()),
			Err(e) => {
				let frozen = storage_value(people, "Trust", "FrozenUntil", Vec::new())
					.await?
					.and_then(|v| v.as_u128());
				let now = block_number(people).await?;
				log::info!(
					"stake report {attempt} of {ATTEMPTS} did not reach trust {floor}: {e}; 					 trust frozen until {frozen:?}, now {now}"
				);
				last = Some(format!("{e} (trust frozen until {frozen:?} at block {now})"));
			},
		}
	}
	Err(anyhow!(
		"{ATTEMPTS} stake reports and trust is still below {floor}. Last: {}. If the roll was 		 not frozen, the stake landed and the staking tier and trust weights no longer add up to 		 the floor",
		last.unwrap_or_default()
	))
}

// ---------------------------------------------------------------------------------------
// TokenWrapper
// ---------------------------------------------------------------------------------------

/// The wrapper's own account: `PalletId(*b"pez/wrap")` turned into an account the way
/// `into_account_truncating` does it -- `"modl"`, the eight id bytes, zeros to thirty-two.
fn wrapper_account() -> [u8; 32] {
	let mut acc = [0u8; 32];
	acc[..4].copy_from_slice(b"modl");
	acc[4..12].copy_from_slice(b"pez/wrap");
	acc
}

/// What the wrapper's books say, read in one place so before and after compare like for like.
#[derive(Debug, Clone, Copy, PartialEq)]
struct WrapBooks {
	/// The holder's free HEZ.
	free: u128,
	/// The holder's wHEZ.
	wrapped: u128,
	/// wHEZ in existence.
	supply: u128,
	/// What the wrapper says it holds.
	locked: u128,
	/// What its account actually holds.
	backing: u128,
}

async fn wrap_books(
	hub: &OnlineClient<PezkuwiConfig>,
	who: &Keypair,
) -> Result<WrapBooks, anyhow::Error> {
	let free_of = |v: Option<Value>| {
		v.and_then(|v| v.at("data").and_then(|d| d.at("free")).and_then(|f| f.as_u128()))
			.unwrap_or(0)
	};
	let free = free_of(storage_value(hub, "System", "Account", vec![account_key(who)]).await?);
	let backing = free_of(
		storage_value(hub, "System", "Account", vec![Value::from_bytes(wrapper_account())]).await?,
	);
	let wrapped = storage_value(
		hub,
		"Assets",
		"Account",
		vec![Value::u128(WHEZ_ASSET_ID as u128), account_key(who)],
	)
	.await?
	.and_then(|v| v.at("balance").and_then(|b| b.as_u128()))
	.unwrap_or(0);
	let supply = storage_value(hub, "Assets", "Asset", vec![Value::u128(WHEZ_ASSET_ID as u128)])
		.await?
		.ok_or_else(|| {
			anyhow!(
				"asset {WHEZ_ASSET_ID} does not exist on the Asset Hub -- the preset is meant to \
				 create wHEZ empty, and without it every wrap fails at the mint"
			)
		})?
		.at("supply")
		.and_then(|s| s.as_u128())
		.ok_or_else(|| anyhow!("asset {WHEZ_ASSET_ID}'s details carry no `supply`"))?;
	let locked = number_at(hub, "TokenWrapper", "TotalLocked", Vec::new()).await?;
	Ok(WrapBooks { free, wrapped, supply, locked, backing })
}

/// HEZ wraps into wHEZ and back, one for one, and the books on both sides agree throughout.
///
/// Three invariants, checked before, between and after: every wHEZ in existence was minted by
/// a wrap (`supply == TotalLocked`); every wHEZ is backed by HEZ the wrapper actually holds
/// (`backing >= TotalLocked`); and a holder's movement on one side is exactly the movement on
/// the other. The second is the one that matters -- a wrapper whose recorded total ran ahead
/// of its account would be printing a claim on HEZ that is not there.
pub(crate) async fn hez_wraps_and_unwraps_one_for_one(
	hub: &OnlineClient<PezkuwiConfig>,
) -> Result<(), anyhow::Error> {
	let holder = dev::dave();
	let invariants = |b: &WrapBooks, when: &str| -> Result<(), anyhow::Error> {
		if b.supply != b.locked {
			return Err(anyhow!(
				"{when}: {} wHEZ exist and the wrapper records {} HEZ locked -- wHEZ was minted \
				 or burned by something other than the wrapper",
				b.supply,
				b.locked
			));
		}
		if b.backing < b.locked {
			return Err(anyhow!(
				"{when}: the wrapper records {} HEZ locked and its account holds {} -- the wHEZ \
				 in circulation is not fully backed",
				b.locked,
				b.backing
			));
		}
		Ok(())
	};

	let start = wrap_books(hub, &holder).await?;
	invariants(&start, "before wrapping")?;

	must(
		hub,
		&dynamic::tx("TokenWrapper", "wrap", vec![Value::u128(WRAP_AMOUNT)]),
		&holder,
		"an endowed account could not wrap HEZ",
	)
	.await?;
	let wrapped = wrap_books(hub, &holder).await?;
	invariants(&wrapped, "after wrapping")?;
	let spent = start.free.saturating_sub(wrapped.free);
	if wrapped.wrapped != start.wrapped + WRAP_AMOUNT
		|| wrapped.supply != start.supply + WRAP_AMOUNT
		|| wrapped.backing != start.backing + WRAP_AMOUNT
		|| spent < WRAP_AMOUNT
		|| spent > WRAP_AMOUNT + FEE_CEILING
	{
		return Err(anyhow!(
			"{WRAP_AMOUNT} was wrapped and the books moved from {start:?} to {wrapped:?}; the \
			 holder lost {spent}"
		));
	}

	// More than is held cannot be unwrapped, whatever the wrapper's account holds.
	expect_refusal(
		hub,
		&dynamic::tx("TokenWrapper", "unwrap", vec![Value::u128(wrapped.wrapped + 1)]),
		&holder,
		"InsufficientWrappedBalance",
		"unwrapping more wHEZ than the holder has",
	)
	.await?;

	let back = WRAP_AMOUNT / 2;
	must(
		hub,
		&dynamic::tx("TokenWrapper", "unwrap", vec![Value::u128(back)]),
		&holder,
		"the holder could not unwrap",
	)
	.await?;
	let unwrapped = wrap_books(hub, &holder).await?;
	invariants(&unwrapped, "after unwrapping")?;
	let gained = unwrapped.free as i128 - wrapped.free as i128;
	if unwrapped.wrapped != wrapped.wrapped - back
		|| unwrapped.supply != wrapped.supply - back
		|| unwrapped.backing != wrapped.backing - back
		|| gained > back as i128
		|| gained < back as i128 - FEE_CEILING as i128
	{
		return Err(anyhow!(
			"{back} was unwrapped and the books moved from {wrapped:?} to {unwrapped:?}; the \
			 holder gained {gained}"
		));
	}
	log::info!(
		"TokenWrapper carried: {WRAP_AMOUNT} wrapped, {back} unwrapped, supply {} backed by {}",
		unwrapped.supply,
		unwrapped.backing
	);
	Ok(())
}

// ---------------------------------------------------------------------------------------
// WP-19: the silence rule
// ---------------------------------------------------------------------------------------

/// The President proves his key still signs, and his office cannot be declared silent.
///
/// The vacancy itself is not reachable here, for two reasons that are both read off the chain
/// and logged. `vacate_silent_office` acts only on a mandate the calendar knows -- `TermEnds`
/// for the office -- and a founding President has none, by design: emptying an office with no
/// by-election scheduled would leave the state headless. And `OfficeInactivityPeriod` is
/// fourteen days of blocks on Zagros, uncompressed, deliberately longer than any test run so
/// that nothing trips it by accident. So the refusal asserted is whichever of the two the
/// chain is in, by name.
pub(crate) async fn the_president_checks_in_and_cannot_be_called_silent(
	people: &OnlineClient<PezkuwiConfig>,
	bench: &[Keypair],
) -> Result<(), anyhow::Error> {
	let serok = &bench[0];
	let other = &bench[1];
	let period = constant_u128(people, "Welati", "OfficeInactivityPeriod")?;
	log::info!("an office may be declared silent after {period} blocks");

	// ---- only the holder, and only for the two offices --------------------------------
	expect_refusal(
		people,
		&dynamic::tx("Welati", "office_check_in", vec![variant("Serok")]),
		other,
		"NotTheOfficeHolder",
		"a citizen checking in for an office they do not hold",
	)
	.await?;
	expect_refusal(
		people,
		&dynamic::tx("Welati", "office_check_in", vec![variant("WezireDarayiye")]),
		serok,
		"NotASilenceVacatableOffice",
		"checking in for a portfolio the silence rule does not cover",
	)
	.await?;

	// ---- the check-in -------------------------------------------------------------------
	let before = block_number(people).await?;
	must(
		people,
		&dynamic::tx("Welati", "office_check_in", vec![variant("Serok")]),
		serok,
		"the President could not check in",
	)
	.await?;
	let last = storage_value(people, "Welati", "OfficeLastActive", vec![variant("Serok")])
		.await?
		.ok_or_else(|| anyhow!("the President checked in and `OfficeLastActive` is empty"))?;
	let who_ok = last.at(0).map(|a| is_account(a, serok)).unwrap_or(false);
	let at = last.at(1).and_then(|b| b.as_u128()).unwrap_or(0);
	if !who_ok || at < before {
		return Err(anyhow!(
			"the President checked in after block {before} and `OfficeLastActive` reads {last}"
		));
	}

	// ---- and the office cannot be emptied -----------------------------------------------
	let has_term = storage_value(people, "Welati", "TermEnds", vec![variant("Presidential")])
		.await?
		.is_some();
	let expected = if has_term { "OfficeHolderIsStillReachable" } else { "OfficeHasNoElectedTerm" };
	expect_refusal(
		people,
		&dynamic::tx("Welati", "vacate_silent_office", vec![variant("Serok")]),
		other,
		expected,
		"declaring a President silent the block after he signed",
	)
	.await?;
	if !holds_office(people, "Serok", serok).await? {
		return Err(anyhow!("the vacancy was refused and the President no longer holds `Serok`"));
	}
	log::info!(
		"WP-19 carried as far as this network allows: check-in recorded at block {at}, vacancy \
		 refused ({expected}); the vacancy itself needs an elected term and {period} silent \
		 blocks"
	);
	Ok(())
}
