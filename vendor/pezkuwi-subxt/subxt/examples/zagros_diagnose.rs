//! Zagros: Diagnose ValidatorsToRetire storage
//!
//! This script:
//! 1. Reads QueuedKeys to get validator #5 (the first one to remove if keeping 4)
//! 2. Deregisters just that ONE validator via ValidatorManager
//! 3. Immediately reads ValidatorsToRetire to verify it was populated
//!
//! Run with:
//!   SUDO_MNEMONIC="..." RPC_URL="ws://217.77.6.126:9948" \
//!   cargo run --release --example zagros_diagnose -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== ZAGROS DEREGISTER DIAGNOSTIC ===\n");

	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9948".to_string());
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected! specVersion: {}\n", api.runtime_version().spec_version);

	// Storage keys
	let queued_keys_key =
		hex::decode("cec5070d609dd3497f72bde07fc96ba088dcde934c658227ee1dfafcd6e16903").unwrap();
	let validators_to_retire_key =
		hex::decode("084e7f70a295a190e2e33fd3f8cdfcc2b664fa73499821e43a617aa0e82b17b1").unwrap();

	// Step 1: Check ValidatorsToRetire BEFORE
	println!("=== STEP 1: Check ValidatorsToRetire BEFORE deregister ===");
	let retire_before = api
		.storage()
		.at_latest()
		.await?
		.fetch_raw(validators_to_retire_key.clone())
		.await?;
	if retire_before.is_empty() {
		println!("  ValidatorsToRetire: EMPTY (as expected)\n");
	} else {
		println!("  ValidatorsToRetire: {} bytes (already has data!)\n", retire_before.len());
	}

	// Step 2: Get validator #5 from QueuedKeys
	println!("=== STEP 2: Get test validator from QueuedKeys ===");
	let raw_data = api.storage().at_latest().await?.fetch_raw(queued_keys_key).await?;
	let count = (raw_data[0] >> 2) as usize;
	let remaining = raw_data.len() - 1;
	let entry_size = remaining / count;
	println!("  QueuedKeys: {} entries, {} bytes/entry", count, entry_size);

	if count <= 4 {
		println!("  Only {} validators, nothing to deregister", count);
		return Ok(());
	}

	// Get validator #5 (index 4, the first one to remove)
	let test_offset = 1 + 4 * entry_size;
	let test_validator = raw_data[test_offset..test_offset + 32].to_vec();
	println!("  Test validator (index 5): 0x{}\n", hex::encode(&test_validator));

	// Step 3: Load sudo key and submit deregister for ONE validator
	println!("=== STEP 3: Submit deregister for ONE validator ===");
	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("  Sudo account: {}", sudo_keypair.public_key().to_account_id());

	// Try TWO different encoding approaches

	// Approach A: Value::from_bytes (what we used before)
	println!("\n  --- Approach A: Value::from_bytes ---");
	let val_a = Value::from_bytes(&test_validator);
	println!("  Value type: {:?}", val_a);

	// Approach B: Value::unnamed_composite with raw bytes
	println!("\n  --- Approach B: Try AccountId32 from subxt ---");
	// In subxt, AccountId32 can be created from [u8; 32]
	let mut arr = [0u8; 32];
	arr.copy_from_slice(&test_validator);

	// Use approach A (same as before) to see if storage gets populated
	let validators_value = vec![Value::from_bytes(&test_validator)];
	let deregister_call = pezkuwi_subxt::dynamic::tx(
		"ValidatorManager",
		"deregister_validators",
		vec![Value::unnamed_composite(validators_value)],
	);

	// Print the encoded call data to debug
	println!("\n  Deregister call value: {:?}", deregister_call.call_data());

	let sudo_call = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![deregister_call.into_value()]);

	println!("\n  Submitting sudo(validatorManager.deregister_validators([1 validator]))...");

	use pezkuwi_subxt::tx::TxStatus;

	let tx_progress =
		api.tx().sign_and_submit_then_watch_default(&sudo_call, &sudo_keypair).await?;

	println!("  TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

	let mut progress = tx_progress;
	let mut success = false;
	loop {
		let status = progress.next().await;
		match status {
			Some(Ok(TxStatus::InBestBlock(details))) => {
				match details.wait_for_success().await {
					Ok(events) => {
						println!("  In best block! Events:");
						for ev in events.iter().flatten() {
							println!("    {}::{}", ev.pallet_name(), ev.variant_name());
							if ev.pallet_name() == "Sudo" && ev.variant_name() == "Sudid" {
								success = true;
							}
							if ev.pallet_name() == "ValidatorManager"
								&& ev.variant_name() == "ValidatorsDeregistered"
							{
								// Try to decode the event data
								println!("    >>> ValidatorsDeregistered event!");
								let bytes = ev.field_bytes();
								println!(
									"    >>> Event field bytes ({} bytes): 0x{}",
									bytes.len(),
									hex::encode(&bytes[..std::cmp::min(bytes.len(), 128)])
								);
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

	if !success {
		println!("\n  TX FAILED!");
		return Ok(());
	}

	// Step 4: IMMEDIATELY check ValidatorsToRetire AFTER
	println!("\n=== STEP 4: Check ValidatorsToRetire AFTER deregister ===");

	// Small delay to ensure state is updated
	tokio::time::sleep(std::time::Duration::from_secs(2)).await;

	let retire_after = api
		.storage()
		.at_latest()
		.await?
		.fetch_raw(validators_to_retire_key.clone())
		.await?;
	if retire_after.is_empty() {
		println!("  ValidatorsToRetire: EMPTY !!! (deregister didn't populate storage!)");
		println!("  THIS IS THE BUG!");
	} else {
		println!("  ValidatorsToRetire: {} bytes", retire_after.len());
		println!("  Raw hex: 0x{}", hex::encode(&retire_after));

		// Decode it
		let count = (retire_after[0] >> 2) as usize;
		println!("  Decoded count: {}", count);
		let mut offset = 1;
		for i in 0..count {
			if offset + 32 <= retire_after.len() {
				let account = &retire_after[offset..offset + 32];
				println!("    [{}] 0x{}", i + 1, hex::encode(account));
				offset += 32;
			}
		}

		// Check if the stored AccountId matches what we sent
		if count > 0 && retire_after.len() >= 33 {
			let stored = &retire_after[1..33];
			if stored == test_validator.as_slice() {
				println!("\n  MATCH! Stored AccountId matches sent AccountId.");
			} else {
				println!("\n  MISMATCH! Stored AccountId does NOT match!");
				println!("  Sent:   0x{}", hex::encode(&test_validator));
				println!("  Stored: 0x{}", hex::encode(stored));
			}
		}
	}

	// Step 5: Re-read raw storage one more time to triple-check
	println!("\n=== STEP 5: Final raw storage check ===");
	let retire_final = api
		.storage()
		.at_latest()
		.await?
		.fetch_raw(validators_to_retire_key.clone())
		.await?;
	println!("  ValidatorsToRetire final: {} bytes", retire_final.len());
	if !retire_final.is_empty() {
		println!("  Raw: 0x{}", hex::encode(&retire_final));
	}

	println!("\n=== DIAGNOSTIC COMPLETE ===");
	Ok(())
}
