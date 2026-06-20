//! Zagros Testnet: Runtime upgrade + ValidatorCount fix
//!
//! Step 1: Deploy new WASM via sudo(sudoUncheckedWeight(system.setCodeWithoutChecks))
//! Step 2: Set ValidatorCount=2 and ForceEra=ForceNew via sudo(system.setStorage)
//!
//! Run with:
//!   SUDO_MNEMONIC="******" \
//!   WASM_FILE="/home/mamostehp/pezkuwi-sdk/target/release/wbuild/pezkuwichain-runtime/pezkuwichain_runtime.compact.compressed.wasm" \
//!   RPC_URL="ws://217.77.6.126:9948" \
//!   cargo run --release --example zagros_upgrade -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== ZAGROS RUNTIME UPGRADE + VALIDATOR FIX ===\n");

	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9948".to_string());
	let wasm_path = std::env::var("WASM_FILE").expect("WASM_FILE environment variable required");
	let new_validator_count: u32 =
		std::env::var("VALIDATOR_COUNT").unwrap_or_else(|_| "2".to_string()).parse()?;

	println!("RPC: {}", url);
	println!("WASM: {}", wasm_path);
	println!("Target validator count: {}", new_validator_count);

	// Load WASM
	let wasm_data = std::fs::read(&wasm_path)?;
	println!(
		"WASM size: {} bytes ({:.2} MB)",
		wasm_data.len(),
		wasm_data.len() as f64 / 1_048_576.0
	);

	// Connect
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	let rv = api.runtime_version();
	println!("Current on-chain specVersion: {}", rv.spec_version);

	// Load sudo key
	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sudo account: {}\n", sudo_keypair.public_key().to_account_id());

	// ==========================================
	// STEP 1: Runtime upgrade (deploy WASM)
	// ==========================================
	println!("=== STEP 1: RUNTIME UPGRADE ===");
	println!("Deploying WASM via sudo(sudoUncheckedWeight(system.setCodeWithoutChecks))...");

	let set_code = pezkuwi_subxt::dynamic::tx(
		"System",
		"set_code_without_checks",
		vec![Value::from_bytes(&wasm_data)],
	);

	let sudo_upgrade = pezkuwi_subxt::dynamic::tx(
		"Sudo",
		"sudo_unchecked_weight",
		vec![
			set_code.into_value(),
			Value::named_composite([
				("ref_time", Value::u128(1u128)),
				("proof_size", Value::u128(1u128)),
			]),
		],
	);

	use pezkuwi_subxt::tx::TxStatus;

	let tx_progress = api
		.tx()
		.sign_and_submit_then_watch_default(&sudo_upgrade, &sudo_keypair)
		.await?;

	println!("  TX submitted: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

	let mut progress = tx_progress;
	let mut upgrade_ok = false;
	loop {
		let status = progress.next().await;
		match status {
			Some(Ok(TxStatus::InBestBlock(details))) => {
				match details.wait_for_success().await {
					Ok(events) => {
						println!("  In best block! Events:");
						for ev in events.iter().flatten() {
							println!("    {}::{}", ev.pallet_name(), ev.variant_name());
							if ev.pallet_name() == "System" && ev.variant_name() == "CodeUpdated" {
								upgrade_ok = true;
							}
						}
					},
					Err(e) => println!("  DISPATCH ERROR: {}", e),
				}
				break;
			},
			Some(Ok(TxStatus::Error { message })) => {
				println!("  TX ERROR: {}", message);
				break;
			},
			Some(Ok(TxStatus::Invalid { message })) => {
				println!("  TX INVALID: {}", message);
				break;
			},
			Some(Ok(TxStatus::Dropped { message })) => {
				println!("  TX DROPPED: {}", message);
				break;
			},
			Some(Err(e)) => {
				println!("  STREAM ERROR: {}", e);
				break;
			},
			None => {
				println!("  STREAM ENDED");
				break;
			},
			_ => {},
		}
	}

	if !upgrade_ok {
		println!("\n  UPGRADE FAILED! Aborting.");
		return Ok(());
	}

	println!("  UPGRADE SUCCESS!\n");

	// Wait for next block to ensure new runtime is active
	println!("Waiting 12 seconds for new runtime to activate...");
	tokio::time::sleep(std::time::Duration::from_secs(12)).await;

	// Reconnect with new runtime
	let api2 = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	let rv2 = api2.runtime_version();
	println!("New on-chain specVersion: {}\n", rv2.spec_version);

	// ==========================================
	// STEP 2: Set ValidatorCount + ForceEra
	// ==========================================
	println!("=== STEP 2: SET VALIDATOR COUNT + FORCE ERA ===");

	// Storage keys (verified):
	// Staking::ValidatorCount: 0x5f3e4907f716ac89b6347d15ececedca138e71612491192d68deab7e6f563fe1
	// Staking::ForceEra:       0x5f3e4907f716ac89b6347d15ececedcaf7dad0317324aecae8744b87fc95f2f3

	let validator_count_key =
		hex::decode("5f3e4907f716ac89b6347d15ececedca138e71612491192d68deab7e6f563fe1").unwrap();
	let force_era_key =
		hex::decode("5f3e4907f716ac89b6347d15ececedcaf7dad0317324aecae8744b87fc95f2f3").unwrap();

	// ValidatorCount is u32 LE
	let validator_count_value = new_validator_count.to_le_bytes().to_vec();
	// ForceEra::ForceNew = 0x01
	let force_era_value = vec![0x01u8];

	println!("Setting ValidatorCount = {}", new_validator_count);
	println!("Setting ForceEra = ForceNew (0x01)");

	let set_storage_tx = pezkuwi_subxt::dynamic::tx(
		"System",
		"set_storage",
		vec![Value::unnamed_composite(vec![
			Value::unnamed_composite(vec![
				Value::from_bytes(&validator_count_key),
				Value::from_bytes(&validator_count_value),
			]),
			Value::unnamed_composite(vec![
				Value::from_bytes(&force_era_key),
				Value::from_bytes(&force_era_value),
			]),
		])],
	);

	let sudo_storage =
		pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![set_storage_tx.into_value()]);

	let tx_progress2 = api2
		.tx()
		.sign_and_submit_then_watch_default(&sudo_storage, &sudo_keypair)
		.await?;

	println!("  TX submitted: 0x{}", hex::encode(tx_progress2.extrinsic_hash().as_ref()));

	let mut progress2 = tx_progress2;
	let mut storage_ok = false;
	loop {
		let status = progress2.next().await;
		match status {
			Some(Ok(TxStatus::InBestBlock(details))) => {
				match details.wait_for_success().await {
					Ok(events) => {
						println!("  In best block! Events:");
						for ev in events.iter().flatten() {
							println!("    {}::{}", ev.pallet_name(), ev.variant_name());
							if ev.pallet_name() == "Sudo" && ev.variant_name() == "Sudid" {
								storage_ok = true;
							}
						}
					},
					Err(e) => println!("  DISPATCH ERROR: {}", e),
				}
				break;
			},
			Some(Ok(TxStatus::Error { message })) => {
				println!("  TX ERROR: {}", message);
				break;
			},
			Some(Ok(TxStatus::Invalid { message })) => {
				println!("  TX INVALID: {}", message);
				break;
			},
			Some(Ok(TxStatus::Dropped { message })) => {
				println!("  TX DROPPED: {}", message);
				break;
			},
			Some(Err(e)) => {
				println!("  STREAM ERROR: {}", e);
				break;
			},
			None => {
				println!("  STREAM ENDED");
				break;
			},
			_ => {},
		}
	}

	if !storage_ok {
		println!("\n  STORAGE FIX FAILED!");
	} else {
		println!("  STORAGE FIX SUCCESS!");
	}

	// ==========================================
	// STEP 3: Verify
	// ==========================================
	println!("\n=== VERIFICATION ===");
	tokio::time::sleep(std::time::Duration::from_secs(6)).await;

	let api3 = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	let rv3 = api3.runtime_version();
	println!("specVersion: {}", rv3.spec_version);

	// Read back storage to verify
	let vc_bytes = api3.storage().at_latest().await?.fetch_raw(validator_count_key).await?;
	if vc_bytes.len() >= 4 {
		let vc = u32::from_le_bytes([vc_bytes[0], vc_bytes[1], vc_bytes[2], vc_bytes[3]]);
		println!("ValidatorCount: {}", vc);
	}

	let fe_bytes = api3.storage().at_latest().await?.fetch_raw(force_era_key).await?;
	if !fe_bytes.is_empty() {
		let fe_name = match fe_bytes[0] {
			0x00 => "NotForcing",
			0x01 => "ForceNew",
			0x02 => "ForceNone",
			0x03 => "ForceAlways",
			_ => "Unknown",
		};
		println!("ForceEra: {} (0x{:02x})", fe_name, fe_bytes[0]);
	}

	println!("\n=== DONE ===");
	println!("Runtime upgraded and validator count set.");
	println!("Next era should elect {} validators.", new_validator_count);
	println!("Monitor: GRANDPA authorities should change within 1-2 sessions.");

	Ok(())
}
