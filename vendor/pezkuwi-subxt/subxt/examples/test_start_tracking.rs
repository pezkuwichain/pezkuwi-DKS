//! Test start_score_tracking on People Chain
//!
//! 1. Transfer small amount from founder to validator on People Chain
//! 2. Call start_score_tracking() from validator account
//! 3. Verify StakingStartBlock was set
//!
//! FOUNDER_MNEMONIC="..." VALIDATOR_MNEMONIC="..." cargo run --release -p pezkuwi-subxt --example test_start_tracking

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const PEOPLE_RPC: &str = "ws://217.77.6.126:41944";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== TEST: start_score_tracking on People Chain ===\n");

	let founder_mnemonic_str =
		std::env::var("FOUNDER_MNEMONIC").expect("FOUNDER_MNEMONIC env var required");
	let validator_mnemonic_str =
		std::env::var("VALIDATOR_MNEMONIC").expect("VALIDATOR_MNEMONIC env var required");

	let founder_mnemonic = Mnemonic::from_str(&founder_mnemonic_str)?;
	let founder_keypair = Keypair::from_phrase(&founder_mnemonic, None)?;
	let founder_account = AccountId32(founder_keypair.public_key().0);

	let validator_mnemonic = Mnemonic::from_str(&validator_mnemonic_str)?;
	let validator_keypair = Keypair::from_phrase(&validator_mnemonic, None)?;
	let validator_account = AccountId32(validator_keypair.public_key().0);

	println!("Founder:   {}", founder_account);
	println!("Validator: {}", validator_account);

	println!("\nConnecting to People Chain: {}", PEOPLE_RPC);
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(PEOPLE_RPC).await?;
	println!("Connected (spec_version: {})\n", api.runtime_version().spec_version);

	// Step 1: Transfer 10 HEZ from founder to validator for tx fees
	let transfer_amount: u128 = 10_000_000_000_000; // 10 HEZ
	println!("--- Step 1: Transfer 10 HEZ to validator for fees ---");

	let transfer = pezkuwi_subxt::dynamic::tx(
		"Balances",
		"transfer_keep_alive",
		vec![
			Value::unnamed_variant("Id", [Value::from_bytes(&validator_account.0)]),
			Value::u128(transfer_amount),
		],
	);

	let progress = api.tx().sign_and_submit_then_watch_default(&transfer, &founder_keypair).await?;
	println!("TX: 0x{}", hex::encode(progress.extrinsic_hash().as_ref()));
	let events = progress.wait_for_finalized_success().await?;
	println!("Transfer finalized!");
	for ev in events.iter().flatten() {
		if ev.pallet_name() == "Balances" {
			println!("  {}::{}", ev.pallet_name(), ev.variant_name());
		}
	}

	// Step 2: Call start_score_tracking from validator
	println!("\n--- Step 2: start_score_tracking() ---");

	let start_tracking =
		pezkuwi_subxt::dynamic::tx("StakingScore", "start_score_tracking", Vec::<Value>::new());

	let progress = api
		.tx()
		.sign_and_submit_then_watch_default(&start_tracking, &validator_keypair)
		.await?;
	println!("TX: 0x{}", hex::encode(progress.extrinsic_hash().as_ref()));

	match progress.wait_for_finalized_success().await {
		Ok(events) => {
			println!("start_score_tracking SUCCESS!");
			for ev in events.iter().flatten() {
				let p = ev.pallet_name();
				if p == "StakingScore" || p == "Trust" || p == "System" {
					println!("  {}::{}", p, ev.variant_name());
				}
			}
		},
		Err(e) => {
			println!("start_score_tracking FAILED: {:?}", e);
		},
	}

	// Step 3: Verify StakingStartBlock was set
	println!("\n--- Step 3: Verify StakingStartBlock ---");

	let query = pezkuwi_subxt::dynamic::storage::<(AccountId32,), Value>(
		"StakingScore",
		"StakingStartBlock",
	);
	let storage = api.storage().at_latest().await?;
	match storage.entry(query)?.try_fetch((validator_account.clone(),)).await? {
		Some(val) => {
			let decoded = val.decode()?;
			println!("StakingStartBlock for validator: {:?}", decoded);
		},
		None => {
			println!("StakingStartBlock: NOT SET (ERROR!)");
		},
	}

	// Step 4: Check TrustScore
	println!("\n--- Step 4: Check TrustScore ---");
	let query = pezkuwi_subxt::dynamic::storage::<(AccountId32,), Value>("Trust", "TrustScores");
	match storage.entry(query)?.try_fetch((validator_account.clone(),)).await? {
		Some(val) => {
			let decoded = val.decode()?;
			println!("TrustScore for validator: {:?}", decoded);
		},
		None => {
			println!("TrustScore: NOT SET (may need on_initialize cycle)");
		},
	}

	println!("\n=== TEST COMPLETE ===");
	Ok(())
}
