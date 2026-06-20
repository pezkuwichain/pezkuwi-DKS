//! Set StakingAhClient mode to Active on RC
//!
//! Run:
//!   SUDO_MNEMONIC="..." cargo run --release -p pezkuwi-subxt --example set_ah_client_active

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

fn load_sudo_keypair() -> Keypair {
	if let Ok(mnemonic_str) = std::env::var("SUDO_MNEMONIC") {
		if !mnemonic_str.is_empty() {
			if let Ok(mnemonic) = Mnemonic::from_str(&mnemonic_str) {
				if let Ok(kp) = Keypair::from_phrase(&mnemonic, None) {
					println!("  [sudo] Loaded from SUDO_MNEMONIC env var");
					return kp;
				}
			}
		}
	}
	let seeds_path = "/home/mamostehp/res/test_seeds.json";
	if let Ok(content) = std::fs::read_to_string(seeds_path) {
		if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
			if let Some(mnemonic_str) = json["sudo_mnemonic"].as_str() {
				if let Ok(mnemonic) = Mnemonic::from_str(mnemonic_str) {
					if let Ok(kp) = Keypair::from_phrase(&mnemonic, None) {
						println!("  [sudo] Loaded from {}", seeds_path);
						return kp;
					}
				}
			}
		}
	}
	panic!("SUDO_MNEMONIC required!");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== SET StakingAhClient MODE → Active ===\n");

	let rc_url = std::env::var("RC_RPC").unwrap_or_else(|_| "ws://127.0.0.1:9944".to_string());
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&rc_url).await?;
	println!("RC connected: spec {}", api.runtime_version().spec_version);

	let sudo_keypair = load_sudo_keypair();
	println!("Sudo: {}\n", sudo_keypair.public_key().to_account_id());

	// Check current mode first
	let mode_key = pezsp_crypto_hashing::twox_128(b"StakingAhClient")
		.iter()
		.chain(pezsp_crypto_hashing::twox_128(b"Mode").iter())
		.copied()
		.collect::<Vec<u8>>();
	let mode_val = match api.storage().at_latest().await?.fetch_raw(mode_key.clone()).await {
		Ok(data) => data,
		Err(_) => vec![],
	};
	let current_mode = if mode_val.is_empty() {
		"Passive (default/not set)"
	} else {
		match mode_val[0] {
			0 => "Passive",
			1 => "Buffered",
			2 => "Active",
			_ => "Unknown",
		}
	};
	println!("Current mode: {}", current_mode);

	// StakingAhClient.set_mode(Active)
	let set_mode = pezkuwi_subxt::dynamic::tx(
		"StakingAhClient",
		"set_mode",
		vec![Value::unnamed_variant("Active", vec![])],
	);
	let sudo_tx = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![set_mode.into_value()]);

	println!("Submitting sudo(StakingAhClient.set_mode(Active))...");
	let progress = api.tx().sign_and_submit_then_watch_default(&sudo_tx, &sudo_keypair).await?;
	let events = progress.wait_for_finalized_success().await?;

	for event in events.iter() {
		let event = event?;
		println!("  {}::{}", event.pallet_name(), event.variant_name());
	}

	// Verify new mode
	let mode_val = match api.storage().at_latest().await?.fetch_raw(mode_key.clone()).await {
		Ok(data) => data,
		Err(_) => vec![],
	};
	let new_mode = if mode_val.is_empty() {
		"(not found)"
	} else {
		match mode_val[0] {
			0 => "Passive",
			1 => "Buffered",
			2 => "Active",
			_ => "Unknown",
		}
	};
	println!("\nNew mode: {}", new_mode);

	if new_mode == "Active" {
		println!("\nStakingAhClient is now Active!");
		println!("RC will send SessionReports to AH via XCM at each session end.");
	}

	Ok(())
}
