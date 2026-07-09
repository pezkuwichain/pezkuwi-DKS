//! Relay Chain Runtime Upgrade (Mainnet)
//!
//! Two-step process (same pattern as ah_upgrade.rs / people_upgrade.rs, minus
//! the XCM hop — Sudo is local on the relay chain):
//! 1. RC direct: System.authorize_upgrade_without_checks(blake2_256(wasm))
//! 2. RC direct: System.apply_authorized_upgrade(wasm)
//!
//! Run:
//!   SUDO_MNEMONIC="..." \
//!   RC_RPC="ws://217.77.6.126:9944" \
//!   WASM_FILE="target/release/wbuild/pezkuwichain-runtime/pezkuwichain_runtime.compact.compressed.wasm" \
//!   cargo run --release -p pezkuwi-subxt --example rc_upgrade

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
	panic!("SUDO_MNEMONIC environment variable required");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("╔══════════════════════════════════════════╗");
	println!("║  RELAY CHAIN RUNTIME UPGRADE             ║");
	println!("╚══════════════════════════════════════════╝\n");

	let rc_url = std::env::var("RC_RPC").unwrap_or_else(|_| "ws://127.0.0.1:9944".to_string());
	let wasm_path = std::env::var("WASM_FILE").expect("WASM_FILE environment variable required");

	let sudo_keypair = load_sudo_keypair();
	println!("  Sudo: {}", sudo_keypair.public_key().to_account_id());

	// Load WASM
	let wasm_data = std::fs::read(&wasm_path)?;
	println!("  WASM: {} ({:.2} MB)", wasm_path, wasm_data.len() as f64 / 1_048_576.0);
	let code_hash = pezsp_crypto_hashing::blake2_256(&wasm_data);
	println!("  Code hash: 0x{}", hex::encode(code_hash));

	// Connect
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&rc_url).await?;
	let old_spec = api.runtime_version().spec_version;
	println!("  RC connected: {} (spec {})\n", rc_url, old_spec);

	// ═══════════════════════════════════════════
	// STEP 1: Authorize upgrade (local Sudo, no XCM needed)
	// ═══════════════════════════════════════════
	println!("=== STEP 1: Authorize upgrade ===");

	let authorize_call = pezkuwi_subxt::dynamic::tx(
		"System",
		"authorize_upgrade_without_checks",
		vec![Value::from_bytes(&code_hash)],
	);
	let sudo_tx = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![authorize_call.into_value()]);

	let progress = api.tx().sign_and_submit_then_watch_default(&sudo_tx, &sudo_keypair).await?;
	let events = progress.wait_for_finalized_success().await?;

	let mut sudo_ok = false;
	for event in events.iter() {
		let event = event?;
		println!("  {}::{}", event.pallet_name(), event.variant_name());
		if event.pallet_name() == "Sudo" && event.variant_name() == "Sudid" {
			sudo_ok = true;
		}
	}
	if !sudo_ok {
		println!("  ERROR: No Sudo::Sudid event! Aborting.");
		return Ok(());
	}

	// Confirm AuthorizedUpgrade storage is set
	let auth_key = pezsp_crypto_hashing::twox_128(b"System")
		.iter()
		.chain(pezsp_crypto_hashing::twox_128(b"AuthorizedUpgrade").iter())
		.copied()
		.collect::<Vec<u8>>();
	let result = api.storage().at_latest().await?.fetch_raw(auth_key).await;
	match result {
		Ok(data) if !data.is_empty() => println!("  AuthorizedUpgrade confirmed on-chain.\n"),
		_ => {
			println!("  ERROR: AuthorizedUpgrade not set after sudo call. Aborting.");
			return Ok(());
		},
	}

	// ═══════════════════════════════════════════
	// STEP 2: Apply authorized upgrade
	// ═══════════════════════════════════════════
	println!("=== STEP 2: Apply authorized upgrade ===");
	println!("  Submitting {} bytes WASM...", wasm_data.len());

	let enact_call = pezkuwi_subxt::dynamic::tx(
		"System",
		"apply_authorized_upgrade",
		vec![Value::from_bytes(&wasm_data)],
	);

	let progress =
		api.tx().sign_and_submit_then_watch_default(&enact_call, &sudo_keypair).await?;
	let events = progress.wait_for_finalized_success().await?;

	let mut code_updated = false;
	for event in events.iter() {
		let event = event?;
		println!("  {}::{}", event.pallet_name(), event.variant_name());
		if event.pallet_name() == "System" && event.variant_name() == "CodeUpdated" {
			code_updated = true;
		}
	}

	if code_updated {
		println!("\n  UPGRADE SUCCESS!");
	} else {
		println!("\n  WARNING: No CodeUpdated event!");
	}

	// ═══════════════════════════════════════════
	// STEP 3: Verify
	// ═══════════════════════════════════════════
	println!("\nVerifying upgrade...");
	let mut verified = false;
	for attempt in 1..=5 {
		tokio::time::sleep(std::time::Duration::from_secs(12)).await;
		let api2 = OnlineClient::<PezkuwiConfig>::from_insecure_url(&rc_url).await?;
		let new_spec = api2.runtime_version().spec_version;
		if new_spec > old_spec {
			println!(
				"  spec_version: {} → {} — UPGRADE VERIFIED! (attempt {})",
				old_spec, new_spec, attempt
			);
			verified = true;
			break;
		}
		println!("  Attempt {}/5: spec still {} — waiting...", attempt, new_spec);
	}

	if !verified {
		println!("  WARNING: spec_version did not increase after 1 minute!");
	}

	println!("\n╔══════════════════════════════════════════╗");
	println!("║  RELAY CHAIN UPGRADE COMPLETE            ║");
	println!("╚══════════════════════════════════════════╝");

	Ok(())
}
