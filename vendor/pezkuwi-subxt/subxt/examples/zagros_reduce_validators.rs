//! Zagros Testnet: Reduce validator count from 21 to 4 via sudo
//!
//! Sends two sudo calls:
//! 1. sudo(staking.setValidatorCount(4))
//! 2. sudo(staking.forceNewEra())
//!
//! Run with:
//!   SUDO_MNEMONIC="******" \
//!   RPC_URL="ws://217.77.6.126:9948" \
//!   cargo run --release --example zagros_reduce_validators

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== ZAGROS VALIDATOR COUNT REDUCTION ===\n");

	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9948".to_string());
	let new_count: u32 =
		std::env::var("VALIDATOR_COUNT").unwrap_or_else(|_| "4".to_string()).parse()?;

	println!("RPC: {}", url);
	println!("Target validator count: {}", new_count);

	// Connect (insecure ws:// allowed for local/VPS connections)
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected!");

	// Load sudo key
	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	let sudo_address = sudo_keypair.public_key().to_account_id();
	println!("Sudo account: {}\n", sudo_address);

	// Step 1: sudo(staking.setValidatorCount(new_count))
	println!("[1/2] Setting validator count to {}...", new_count);

	let set_count_call = pezkuwi_subxt::dynamic::tx(
		"Staking",
		"set_validator_count",
		vec![Value::u128(new_count as u128)],
	);

	let sudo_tx_1 = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![set_count_call.into_value()]);

	// Use sign_and_submit_default (does NOT wait for finalization)
	let tx_hash_1 = api.tx().sign_and_submit_default(&sudo_tx_1, &sudo_keypair).await?;
	println!("  Submitted! TX hash: 0x{}", hex::encode(tx_hash_1.as_ref()));

	// Wait a bit for the tx to be included in a block
	println!("  Waiting 12 seconds for block inclusion...");
	tokio::time::sleep(std::time::Duration::from_secs(12)).await;

	// Step 2: sudo(staking.forceNewEra())
	println!("\n[2/2] Forcing new era...");

	let force_era_call =
		pezkuwi_subxt::dynamic::tx("Staking", "force_new_era", Vec::<Value>::new());

	let sudo_tx_2 = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![force_era_call.into_value()]);

	let tx_hash_2 = api.tx().sign_and_submit_default(&sudo_tx_2, &sudo_keypair).await?;
	println!("  Submitted! TX hash: 0x{}", hex::encode(tx_hash_2.as_ref()));

	println!("\n=== DONE ===");
	println!("Both sudo calls submitted successfully.");
	println!("Validator count: 21 -> {}", new_count);
	println!("ForceNewEra triggered.");
	println!();
	println!("Next steps:");
	println!("  - Wait for next era boundary (session change)");
	println!("  - GRANDPA should start finalizing with {} validators", new_count);
	println!("  - Monitor: curl -s -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chain_getFinalizedHead\",\"params\":[]}}' -H 'Content-Type: application/json' http://217.77.6.126:9948");

	Ok(())
}
