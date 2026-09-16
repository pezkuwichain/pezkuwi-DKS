//! Zagros: put citizens on the register the way citizens actually get there.
//!
//! The population gate is what opens the first of FAZ 3's four cross-chain paths: when the roll
//! reaches `min(WelatiPopulationThreshold, PopulationThresholdOverride)` the People chain tells
//! the Asset Hub, and the Asset Hub starts paying. On Zagros that figure is a hundred, scaled
//! from the mainnet's hundred thousand so the testnet can reach it.
//!
//! There is a shortcut and it is not taken here. `Tiki::grant_honorary_citizenship` accepts
//! Root, and the relay's sudo reaches this chain as Root through `ParentAsSuperuser`, so a
//! hundred citizens could be conjured in one batch. Three reasons not to: L-5 says sudo does
//! not do governance's work; the runtime's own comment says Root stands on that origin only
//! while sudo exists; and a register filled that way never exercises the path the forty million
//! will use. The real path costs three extrinsics per citizen and a returned deposit, which is
//! not a price worth avoiding.
//!
//! Each citizen therefore walks the whole route:
//!   1. `apply_for_citizenship(hash, Some(founder), None)` -- reserves the deposit
//!   2. founder `approve_referral(applicant)`               -- batched
//!   3. `confirm_citizenship()`                             -- returns the deposit
//!
//! Step 2 works in bulk only because the founder is `DefaultReferrer`, which exempts it from
//! the waiting period and the earned vouching capacity. That exemption was pointed at the
//! mainnet founder until 2026-09-16; on a chain where it still is, this script stalls at step 2
//! with `NotEligibleToVouch` after five approvals.
//!
//! Run:
//!   FOUNDER_MNEMONIC="..." FOUNDER_PATH="//zagros//allocation//founder" \
//!   COUNT=100 RPC_URL="wss://zagros-people-rpc.pezkuwichain.io" \
//!   cargo run --release --example zagros_register_citizens -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::sr25519::Keypair;
use pezkuwi_subxt_signer::SecretUri;
use std::str::FromStr;

const PLANCK: u128 = 1_000_000_000_000;

/// What each applicant needs before it can apply.
///
/// The deposit is reserved at apply and returned at confirm, so the cost of a citizen is the
/// fees alone -- but the deposit still has to be *there*, and an account that cannot reserve it
/// fails with an error about funds that reads like the wrong account was used.
const FUND_PER_CITIZEN: u128 = 2 * PLANCK;

async fn submit(
	api: &OnlineClient<PezkuwiConfig>,
	signer: &Keypair,
	tx: &pezkuwi_subxt_core::tx::payload::DynamicPayload,
	what: &str,
) -> Result<(), Box<dyn std::error::Error>> {
	use pezkuwi_subxt::tx::TxStatus;
	let mut progress = api.tx().sign_and_submit_then_watch_default(tx, signer).await?;
	loop {
		match progress.next().await {
			Some(Ok(TxStatus::InBestBlock(d))) => {
				d.wait_for_success().await.map_err(|e| format!("{what}: {e}"))?;
				return Ok(());
			},
			Some(Ok(_)) => continue,
			Some(Err(e)) => return Err(format!("{what}: {e}").into()),
			None => return Err(format!("{what}: subscription ended before a block").into()),
		}
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL").expect("RPC_URL required");
	let count: u32 = std::env::var("COUNT").unwrap_or_else(|_| "100".into()).parse()?;
	let phrase = std::env::var("FOUNDER_MNEMONIC").expect("FOUNDER_MNEMONIC required");
	let fpath =
		std::env::var("FOUNDER_PATH").unwrap_or_else(|_| "//zagros//allocation//founder".into());

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	let founder = Keypair::from_uri(&SecretUri::from_str(&format!("{phrase}{fpath}"))?)?;
	let founder_id: AccountId32 = founder.public_key().to_account_id();
	println!("founder : {founder_id}");
	println!("target  : {count} citizens\n");

	// Derived from the same phrase by path, so the whole cohort is reproducible from one secret
	// and a rerun addresses the same accounts rather than making new ones.
	let citizens: Vec<Keypair> = (1..=count)
		.map(|i| {
			Keypair::from_uri(
				&SecretUri::from_str(&format!("{phrase}//zagros//citizen//{i}")).unwrap(),
			)
			.unwrap()
		})
		.collect();

	// --- 1. fund, in batches: a hundred separate transfers is a hundred separate fees ---
	println!("== 1/4 funding");
	for (n, chunk) in citizens.chunks(25).enumerate() {
		let calls: Vec<Value> = chunk
			.iter()
			.map(|k| {
				pezkuwi_subxt::dynamic::tx(
					"Balances",
					"transfer_keep_alive",
					vec![
						Value::unnamed_variant(
							"Id",
							vec![Value::from_bytes(&k.public_key().to_account_id().0)],
						),
						Value::u128(FUND_PER_CITIZEN),
					],
				)
				.into_value()
			})
			.collect();
		let batch = pezkuwi_subxt::dynamic::tx(
			"Utility",
			"batch_all",
			vec![Value::unnamed_composite(calls)],
		);
		submit(&api, &founder, &batch, "funding batch").await?;
		println!("   batch {} ({} accounts)", n + 1, chunk.len());
	}

	// --- 2. apply: one signature each, so no batching is possible ---
	println!("== 2/4 applications");
	for (i, k) in citizens.iter().enumerate() {
		// The identity hash is opaque to the chain -- it stores it as proof of what was
		// submitted. Derived from the index so a rerun produces the same hash for the same
		// citizen rather than a second identity for one person.
		let mut hash = [0u8; 32];
		hash[..4].copy_from_slice(&((i as u32) + 1).to_le_bytes());
		hash[4..8].copy_from_slice(b"zgrs");
		let tx = pezkuwi_subxt::dynamic::tx(
			"IdentityKyc",
			"apply_for_citizenship",
			vec![
				Value::from_bytes(hash),
				Value::unnamed_variant(
					"Some",
					vec![Value::unnamed_variant("Id", vec![Value::from_bytes(&founder_id.0)])],
				),
				Value::unnamed_variant("None", vec![]),
			],
		);
		submit(&api, k, &tx, &format!("apply #{}", i + 1)).await?;
		if (i + 1) % 10 == 0 {
			println!("   {} / {count}", i + 1);
		}
	}

	// --- 3. approve: the founder is exempt from the vouching limits, so this can batch ---
	println!("== 3/4 approvals");
	for (n, chunk) in citizens.chunks(25).enumerate() {
		let calls: Vec<Value> = chunk
			.iter()
			.map(|k| {
				pezkuwi_subxt::dynamic::tx(
					"IdentityKyc",
					"approve_referral",
					vec![Value::unnamed_variant(
						"Id",
						vec![Value::from_bytes(&k.public_key().to_account_id().0)],
					)],
				)
				.into_value()
			})
			.collect();
		let batch = pezkuwi_subxt::dynamic::tx(
			"Utility",
			"batch_all",
			vec![Value::unnamed_composite(calls)],
		);
		submit(&api, &founder, &batch, "approval batch").await?;
		println!("   batch {} ({} approvals)", n + 1, chunk.len());
	}

	// --- 4. confirm: each citizen's own signature again, deposit comes back here ---
	println!("== 4/4 confirmations");
	for (i, k) in citizens.iter().enumerate() {
		let tx =
			pezkuwi_subxt::dynamic::tx("IdentityKyc", "confirm_citizenship", Vec::<Value>::new());
		submit(&api, k, &tx, &format!("confirm #{}", i + 1)).await?;
		if (i + 1) % 10 == 0 {
			println!("   {} / {count}", i + 1);
		}
	}

	// The chain's own count, not the loop's. A script that reports what it intended is not a
	// measurement: an application refused mid-run would still leave the loop finishing, and the
	// number that matters is the one the gate reads.
	let addr = pezkuwi_subxt::dynamic::storage::<Vec<Value>, Value>("IdentityKyc", "CitizenCount");
	let roll: u32 = api
		.storage()
		.at_latest()
		.await?
		.try_fetch(addr, Vec::new())
		.await?
		.ok_or("IdentityKyc::CitizenCount is not set")?
		.decode()?
		.as_u128()
		.ok_or("CitizenCount did not decode as a number")? as u32;

	println!("\nroll: {roll} citizens");
	if roll < count {
		return Err(
			format!("expected at least {count} on the roll, the chain reports {roll}").into()
		);
	}
	println!("GREEN: the register holds {roll}");
	Ok(())
}
