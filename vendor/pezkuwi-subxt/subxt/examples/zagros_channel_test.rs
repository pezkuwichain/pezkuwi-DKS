//! Zagros: prove the AH -> People channel carries value, end to end.
//!
//! The runbook's channel test. Every HEZ on the Asset Hub sits in a keyless pot at genesis,
//! so there is nothing there to send: the test has to fund the sender first, and it does that
//! by teleporting from the relay. That makes it two tests, not one -- relay -> AH and then
//! AH -> People -- and both are paths the wallet will use.
//!
//! Balances are read before and after each hop and the run fails if one does not move. A
//! transaction that is merely `InBlock` proves nothing about an XCM message: the message is
//! executed on the *other* chain, later, and it can fail there while the submitting side
//! reports success.
//!
//! Run with:
//!   SENDER_MNEMONIC="<zagros master phrase>" AMOUNT_HEZ=10 \
//!   cargo run --release --example zagros_channel_test -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::{At, Value};
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::sr25519::Keypair;
use pezkuwi_subxt_signer::SecretUri;
use std::str::FromStr;
use std::time::Duration;

const HEZ: u128 = 1_000_000_000_000;
const ASSET_HUB: u32 = 1000;
const PEOPLE: u32 = 1004;

const RELAY_URL: &str = "wss://zagros-rpc.pezkuwichain.io";
const AH_URL: &str = "wss://zagros-asset-hub-rpc.pezkuwichain.io";
const PEOPLE_URL: &str = "wss://zagros-people-rpc.pezkuwichain.io";

// ---- XCM v5 values, built dynamically so no checked-in metadata can go stale on us ----

fn junctions_here() -> Value {
	Value::unnamed_variant("Here", vec![])
}

fn junctions_x1(j: Value) -> Value {
	Value::unnamed_variant("X1", vec![Value::unnamed_composite(vec![j])])
}

fn location(parents: u8, interior: Value) -> Value {
	Value::named_composite(vec![("parents", Value::u128(parents as u128)), ("interior", interior)])
}

fn versioned_location(l: Value) -> Value {
	Value::unnamed_variant("V5", vec![l])
}

/// `dest` for a chain seen from `parents` hops up: 0 from the relay, 1 from a sibling.
fn dest_teyrchain(parents: u8, para: u32) -> Value {
	versioned_location(location(
		parents,
		junctions_x1(Value::unnamed_variant("Teyrchain", vec![Value::u128(para as u128)])),
	))
}

fn beneficiary(id: [u8; 32]) -> Value {
	versioned_location(location(
		0,
		junctions_x1(Value::named_variant(
			"AccountId32",
			vec![
				("network", Value::unnamed_variant("None", vec![])),
				("id", Value::from_bytes(id)),
			],
		)),
	))
}

/// The native token, named from where the call is made: `Here` on the relay, one hop up
/// from a teyrchain.
fn native_assets(parents: u8, amount: u128) -> Value {
	let asset = Value::named_composite(vec![
		("id", Value::unnamed_composite(vec![location(parents, junctions_here())])),
		("fun", Value::unnamed_variant("Fungible", vec![Value::u128(amount)])),
	]);
	Value::unnamed_variant("V5", vec![Value::unnamed_composite(vec![asset])])
}

async fn free_balance(
	api: &OnlineClient<PezkuwiConfig>,
	who: &AccountId32,
) -> Result<u128, Box<dyn std::error::Error>> {
	let q = pezkuwi_subxt::dynamic::storage::<(AccountId32,), Value>("System", "Account");
	// An account that has never held anything has no entry at all; that is zero, not an
	// error, and the destination account is exactly that on the first run.
	let storage = api.storage().at_latest().await?;
	let fetched = storage.entry(q)?.fetch((who.clone(),)).await;
	Ok(match fetched {
		Ok(sv) => match sv.decode() {
			Ok(v) => v.at("data").at("free").and_then(|b| b.as_u128()).unwrap_or(0),
			Err(_) => 0,
		},
		Err(_) => 0,
	})
}

/// Poll until the balance moves, or give up. XCM lands a few blocks later than the
/// submitting chain's success, and how many depends on the queue.
async fn wait_for_gain(
	api: &OnlineClient<PezkuwiConfig>,
	who: &AccountId32,
	before: u128,
	label: &str,
) -> Result<u128, Box<dyn std::error::Error>> {
	for i in 1..=40 {
		tokio::time::sleep(Duration::from_secs(3)).await;
		let now = free_balance(api, who).await?;
		if now > before {
			println!(
				"  {label}: {:.6} -> {:.6} HEZ  ({}s)",
				before as f64 / HEZ as f64,
				now as f64 / HEZ as f64,
				i * 3
			);
			return Ok(now);
		}
		if i % 5 == 0 {
			println!("  {label}: hala {:.6} HEZ ({}s)", now as f64 / HEZ as f64, i * 3);
		}
	}
	Err(format!("{label}: balance never moved -- the XCM did not arrive").into())
}

async fn send(
	api: &OnlineClient<PezkuwiConfig>,
	signer: &Keypair,
	pallet: &str,
	dest: Value,
	who: [u8; 32],
	assets: Value,
) -> Result<(), Box<dyn std::error::Error>> {
	let tx = pezkuwi_subxt::dynamic::tx(
		pallet,
		"limited_teleport_assets",
		vec![
			dest,
			beneficiary(who),
			assets,
			Value::u128(0),                              // fee_asset_item
			Value::unnamed_variant("Unlimited", vec![]), // weight_limit
		],
	);
	let progress = api.tx().sign_and_submit_then_watch_default(&tx, signer).await?;
	println!("  tx 0x{}", hex::encode(progress.extrinsic_hash().as_ref()));
	let events = progress.wait_for_finalized_success().await?;
	for ev in events.iter().flatten() {
		let (p, v) = (ev.pallet_name(), ev.variant_name());
		if p != "System" && p != "TransactionPayment" && p != "Balances" {
			println!("    {p}::{v}");
		}
	}
	Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let amount: u128 =
		std::env::var("AMOUNT_HEZ").unwrap_or_else(|_| "10".into()).parse::<u128>()? * HEZ;
	let phrase = std::env::var("SENDER_MNEMONIC").expect("SENDER_MNEMONIC required");
	let path =
		std::env::var("SENDER_PATH").unwrap_or_else(|_| "//zagros//allocation//founder".into());
	let signer = Keypair::from_uri(&SecretUri::from_str(&format!("{phrase}{path}"))?)?;
	let who: [u8; 32] = signer.public_key().0;
	let acct = AccountId32(who);
	println!("sender {acct}  ({:.4} HEZ per hop)\n", amount as f64 / HEZ as f64);

	let relay = OnlineClient::<PezkuwiConfig>::from_url(RELAY_URL).await?;
	let ah = OnlineClient::<PezkuwiConfig>::from_url(AH_URL).await?;
	let people = OnlineClient::<PezkuwiConfig>::from_url(PEOPLE_URL).await?;

	let (r0, a0, p0) = (
		free_balance(&relay, &acct).await?,
		free_balance(&ah, &acct).await?,
		free_balance(&people, &acct).await?,
	);
	println!(
		"başlangıç: relay {:.4}  AH {:.4}  People {:.4} HEZ\n",
		r0 as f64 / HEZ as f64,
		a0 as f64 / HEZ as f64,
		p0 as f64 / HEZ as f64
	);
	if r0 < amount * 2 {
		return Err("sender does not hold enough on the relay for two hops".into());
	}

	println!("═══ 1/2  relay -> Asset Hub (teleport)");
	send(&relay, &signer, "XcmPallet", dest_teyrchain(0, ASSET_HUB), who, native_assets(0, amount))
		.await?;
	let a1 = wait_for_gain(&ah, &acct, a0, "AH").await?;

	// Send less than arrived: the first hop paid delivery and execution fees out of the
	// same asset, so `amount` is no longer there.
	let second = a1.saturating_sub(a0) / 2;
	println!("\n═══ 2/2  Asset Hub -> People (teleport, {:.6} HEZ)", second as f64 / HEZ as f64);
	send(&ah, &signer, "PezkuwiXcm", dest_teyrchain(1, PEOPLE), who, native_assets(1, second))
		.await?;
	wait_for_gain(&people, &acct, p0, "People").await?;

	println!("\nYEŞİL: iki yol da değer taşıdı -- relay->AH ve AH->People.");
	Ok(())
}
