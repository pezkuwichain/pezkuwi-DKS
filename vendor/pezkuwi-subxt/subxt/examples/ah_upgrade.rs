//! Asset Hub Runtime Upgrade (Mainnet)
//!
//! Two-step process:
//! 1. RC → XCM → AH: System.authorize_upgrade(blake2_256(wasm))
//! 2. AH direct: System.apply_authorized_upgrade(wasm)
//!
//! Run:
//!   RC_RPC="ws://217.77.6.126:9944" \
//!   AH_RPC="ws://217.77.6.126:40944" \
//!   WASM_FILE="target/release/wbuild/asset-hub-pezkuwichain-runtime/asset_hub_pezkuwichain_runtime.compact.compressed.wasm" \
//!   cargo run --release -p pezkuwi-subxt --example ah_upgrade

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const AH_PARA_ID: u128 = 1000;

fn load_sudo_keypair() -> Keypair {
	// 1. Try SUDO_MNEMONIC env var
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

	// 2. Fallback to seeds file
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

	panic!("SUDO_MNEMONIC required! Set env var or create /home/mamostehp/res/test_seeds.json");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("╔══════════════════════════════════════════╗");
	println!("║  ASSET HUB RUNTIME UPGRADE               ║");
	println!("╚══════════════════════════════════════════╝\n");

	let rc_url = std::env::var("RC_RPC").unwrap_or_else(|_| "ws://127.0.0.1:9944".to_string());
	let ah_url = std::env::var("AH_RPC").unwrap_or_else(|_| "ws://127.0.0.1:40944".to_string());
	let wasm_path = std::env::var("WASM_FILE").expect("WASM_FILE environment variable required");

	let sudo_keypair = load_sudo_keypair();
	println!("  Sudo: {}", sudo_keypair.public_key().to_account_id());

	// Load WASM
	let wasm_data = std::fs::read(&wasm_path)?;
	println!("  WASM: {} ({:.2} MB)", wasm_path, wasm_data.len() as f64 / 1_048_576.0);

	// Blake2-256 hash of WASM
	let code_hash = pezsp_crypto_hashing::blake2_256(&wasm_data);
	println!("  Code hash: 0x{}", hex::encode(code_hash));

	// Connect to RC
	let rc_api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&rc_url).await?;
	println!("  RC connected: {} (spec {})", rc_url, rc_api.runtime_version().spec_version);

	// Connect to AH
	let ah_api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&ah_url).await?;
	let old_spec = ah_api.runtime_version().spec_version;
	println!("  AH connected: {} (spec {})\n", ah_url, old_spec);

	// ═══════════════════════════════════════════
	// STEP 1: Authorize upgrade via XCM from RC
	// ═══════════════════════════════════════════
	println!("=== STEP 1: Authorize upgrade (RC → XCM → AH) ===");

	// Encode System::authorize_upgrade_without_checks(code_hash)
	// System pallet index = 0, call_index = 10 (verified from source)
	let mut encoded_call = Vec::with_capacity(34);
	encoded_call.push(0x00); // System pallet
	encoded_call.push(0x0a); // authorize_upgrade_without_checks = call_index(10)
	encoded_call.extend_from_slice(&code_hash);
	println!("  Encoded call: {} bytes", encoded_call.len());

	let dest = Value::unnamed_variant(
		"V3",
		vec![Value::named_composite([
			("parents", Value::u128(0)),
			(
				"interior",
				Value::unnamed_variant(
					"X1",
					vec![Value::unnamed_variant("Teyrchain", vec![Value::u128(AH_PARA_ID)])],
				),
			),
		])],
	);

	let message = Value::unnamed_variant(
		"V3",
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
					(
						"require_weight_at_most",
						Value::named_composite([
							("ref_time", Value::u128(5_000_000_000u128)),
							("proof_size", Value::u128(500_000u128)),
						]),
					),
					("call", Value::from_bytes(&encoded_call)),
				],
			),
		])],
	);

	let xcm_send = pezkuwi_subxt::dynamic::tx("XcmPallet", "send", vec![dest, message]);
	let sudo_tx = pezkuwi_subxt::dynamic::tx(
		"Sudo",
		"sudo_unchecked_weight",
		vec![
			xcm_send.into_value(),
			Value::named_composite([
				("ref_time", Value::u128(1u128)),
				("proof_size", Value::u128(1u128)),
			]),
		],
	);

	let progress = rc_api.tx().sign_and_submit_then_watch_default(&sudo_tx, &sudo_keypair).await?;
	let events = progress.wait_for_finalized_success().await?;

	let mut sent = false;
	for event in events.iter() {
		let event = event?;
		if event.pallet_name() == "XcmPallet" && event.variant_name() == "Sent" {
			sent = true;
		}
		if event.pallet_name() == "Sudo" || event.pallet_name() == "XcmPallet" {
			println!("  {}::{}", event.pallet_name(), event.variant_name());
		}
	}
	if !sent {
		println!("  ERROR: No XcmPallet::Sent event! Aborting.");
		return Ok(());
	}
	println!("  XCM authorize_upgrade sent!\n");

	// Wait for AH to process the XCM — poll AuthorizedUpgrade storage
	println!("Waiting for AH to process XCM authorize_upgrade...");
	let mut authorized = false;
	for attempt in 1..=30 {
		tokio::time::sleep(std::time::Duration::from_secs(6)).await;

		let ah_check = OnlineClient::<PezkuwiConfig>::from_insecure_url(&ah_url).await?;
		let block = ah_check.blocks().at_latest().await?;
		let block_num = block.number();

		let auth_key = pezsp_crypto_hashing::twox_128(b"System")
			.iter()
			.chain(pezsp_crypto_hashing::twox_128(b"AuthorizedUpgrade").iter())
			.copied()
			.collect::<Vec<u8>>();
		let result = ah_check.storage().at_latest().await?.fetch_raw(auth_key).await;
		match result {
			Ok(data) if !data.is_empty() => {
				println!(
					"  AuthorizedUpgrade found on AH at block {} (attempt {})!",
					block_num, attempt
				);
				authorized = true;
				break;
			},
			_ => {}, // NoValueFound or empty — not yet set
		}
		println!(
			"  Attempt {}/30: AH block {} — AuthorizedUpgrade not yet set...",
			attempt, block_num
		);
	}

	if !authorized {
		println!("  ERROR: AuthorizedUpgrade not set after 3 minutes. Aborting.");
		return Ok(());
	}

	// ═══════════════════════════════════════════
	// STEP 1.5: Fund sudo account on AH via XCM (if needed)
	// ═══════════════════════════════════════════
	println!("\n=== STEP 1.5: Fund sudo account on AH ===");
	let sudo_account_id = sudo_keypair.public_key().to_account_id();
	let account_bytes: [u8; 32] = *sudo_account_id.as_ref();

	// Check existing balance first
	let balance_key = {
		let mut key = Vec::new();
		key.extend_from_slice(&pezsp_crypto_hashing::twox_128(b"System"));
		key.extend_from_slice(&pezsp_crypto_hashing::twox_128(b"Account"));
		let hash = pezsp_crypto_hashing::blake2_128(&account_bytes);
		key.extend_from_slice(&hash);
		key.extend_from_slice(&account_bytes);
		key
	};
	let ah_storage = ah_api.storage().at_latest().await?;
	let has_balance = match ah_storage.fetch_raw(balance_key).await {
		Ok(data) => !data.is_empty(),
		Err(_) => false,
	};

	if has_balance {
		println!("  Sudo account already has funds on AH — skipping funding");
	} else {
		println!("  Sudo account has no funds — funding via XCM...");

		// Encode Balances::force_set_balance(who, new_free)
		// Balances pallet = 10, call_index = 8 (verified from source)
		let mut fund_call: Vec<u8> = Vec::new();
		fund_call.push(10u8); // Balances pallet
		fund_call.push(8u8); // force_set_balance
		fund_call.push(0u8); // MultiAddress::Id variant
		fund_call.extend_from_slice(&account_bytes);
		// 10,000 HEZ = 10_000 * 10^12 (12 decimals, NOT 18)
		let amount: u128 = 10_000_000_000_000_000u128; // 10,000 HEZ
		let amount_bytes = amount.to_le_bytes();
		let significant = amount_bytes.iter().rposition(|&b| b != 0).map(|i| i + 1).unwrap_or(1);
		let byte_len = significant.max(4);
		fund_call.push(((byte_len as u8 - 4) << 2) | 0b11);
		fund_call.extend_from_slice(&amount_bytes[..byte_len]);

		let fund_dest = Value::unnamed_variant(
			"V3",
			vec![Value::named_composite([
				("parents", Value::u128(0)),
				(
					"interior",
					Value::unnamed_variant(
						"X1",
						vec![Value::unnamed_variant("Teyrchain", vec![Value::u128(AH_PARA_ID)])],
					),
				),
			])],
		);

		let fund_msg = Value::unnamed_variant(
			"V3",
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
						(
							"require_weight_at_most",
							Value::named_composite([
								("ref_time", Value::u128(5_000_000_000u128)),
								("proof_size", Value::u128(500_000u128)),
							]),
						),
						("call", Value::from_bytes(&fund_call)),
					],
				),
			])],
		);

		let fund_xcm = pezkuwi_subxt::dynamic::tx("XcmPallet", "send", vec![fund_dest, fund_msg]);
		let fund_sudo = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![fund_xcm.into_value()]);

		let progress = rc_api
			.tx()
			.sign_and_submit_then_watch_default(&fund_sudo, &sudo_keypair)
			.await?;
		let events = progress.wait_for_finalized_success().await?;
		let fund_sent = events
			.iter()
			.flatten()
			.any(|e| e.pallet_name() == "XcmPallet" && e.variant_name() == "Sent");
		if fund_sent {
			println!("  [OK] Force set balance XCM sent");
		} else {
			println!("  [WARN] No XcmPallet::Sent event for funding");
		}

		println!("  Waiting 18s for DMP processing...");
		tokio::time::sleep(std::time::Duration::from_secs(18)).await;
	}

	// ═══════════════════════════════════════════
	// STEP 2: Enact upgrade on AH directly
	// ═══════════════════════════════════════════
	println!("\n=== STEP 2: Apply authorized upgrade on AH ===");
	println!("  Submitting {} bytes WASM...", wasm_data.len());

	let enact_call = pezkuwi_subxt::dynamic::tx(
		"System",
		"apply_authorized_upgrade",
		vec![Value::from_bytes(&wasm_data)],
	);

	let progress = ah_api
		.tx()
		.sign_and_submit_then_watch_default(&enact_call, &sudo_keypair)
		.await?;
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
		println!("\n  ValidationFunctionStored — waiting for relay chain enactment...");
	}

	// ═══════════════════════════════════════════
	// STEP 3: Verify (poll for spec change)
	// ═══════════════════════════════════════════
	println!("\nVerifying upgrade...");
	let mut verified = false;
	for attempt in 1..=10 {
		tokio::time::sleep(std::time::Duration::from_secs(12)).await;
		let ah_api2 = OnlineClient::<PezkuwiConfig>::from_insecure_url(&ah_url).await?;
		let new_spec = ah_api2.runtime_version().spec_version;
		if new_spec > old_spec {
			println!(
				"  AH spec_version: {} → {} — UPGRADE VERIFIED! (attempt {})",
				old_spec, new_spec, attempt
			);
			verified = true;
			break;
		}
		println!("  Attempt {}/10: spec still {} — waiting...", attempt, new_spec);
	}

	if !verified {
		println!("  WARNING: spec_version did not increase after 2 minutes!");
		println!("  Check AH logs and relay chain for enactment status.");
	}

	println!("\n╔══════════════════════════════════════════╗");
	println!("║  ASSET HUB UPGRADE COMPLETE              ║");
	println!("╚══════════════════════════════════════════╝");

	Ok(())
}
