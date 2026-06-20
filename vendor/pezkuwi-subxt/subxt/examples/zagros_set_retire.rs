//! Zagros: Directly write ValidatorsToRetire via sudo(system.setStorage)
//!
//! This bypasses subxt's dynamic encoding by manually SCALE-encoding the data.
//!
//! Run with:
//!   SUDO_MNEMONIC="..." KEEP=4 RPC_URL="ws://217.77.6.126:9948" \
//!   cargo run --release --example zagros_set_retire -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

/// SCALE encode a compact unsigned integer
fn encode_compact(value: usize) -> Vec<u8> {
	if value < 64 {
		vec![(value as u8) << 2]
	} else if value < 16384 {
		let v = ((value as u16) << 2) | 0x01;
		v.to_le_bytes().to_vec()
	} else {
		panic!("Value too large for compact encoding: {}", value);
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== ZAGROS: SET ValidatorsToRetire via setStorage ===\n");

	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9948".to_string());
	let keep: usize = std::env::var("KEEP").unwrap_or_else(|_| "4".to_string()).parse()?;

	println!("RPC: {}", url);
	println!("Keep: {} validators\n", keep);

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected! specVersion: {}\n", api.runtime_version().spec_version);

	// Verify genesis hash (Zagros = 0xbb4a61ab...)
	let genesis = format!("0x{}", hex::encode(api.genesis_hash().as_ref()));
	println!("Genesis: {}", genesis);
	if !genesis.starts_with("0xbb4a61ab") {
		println!("ERROR: This is NOT Zagros! Aborting.");
		return Ok(());
	}
	println!("Confirmed: This is Zagros testnet.\n");

	// Read QueuedKeys to get all validator AccountIds
	let queued_keys_key =
		hex::decode("cec5070d609dd3497f72bde07fc96ba088dcde934c658227ee1dfafcd6e16903").unwrap();

	let raw_data = api.storage().at_latest().await?.fetch_raw(queued_keys_key).await?;

	let count = (raw_data[0] >> 2) as usize;
	let remaining = raw_data.len() - 1;
	let entry_size = remaining / count;
	println!("QueuedKeys: {} entries, {} bytes/entry", count, entry_size);

	if count <= keep {
		println!("Only {} validators, nothing to remove.", count);
		return Ok(());
	}

	// Extract all validator AccountIds
	let mut all_validators: Vec<Vec<u8>> = Vec::new();
	for i in 0..count {
		let offset = 1 + i * entry_size;
		let account = raw_data[offset..offset + 32].to_vec();
		all_validators.push(account);
	}

	let to_remove = &all_validators[keep..];
	println!("\nValidators to KEEP:");
	for (i, v) in all_validators[..keep].iter().enumerate() {
		println!("  [{:2}] KEEP   0x{}", i + 1, hex::encode(v));
	}
	println!("\nValidators to REMOVE:");
	for (i, v) in to_remove.iter().enumerate() {
		println!("  [{:2}] REMOVE 0x{}", keep + i + 1, hex::encode(v));
	}

	// SCALE-encode Vec<AccountId32> manually
	// Format: compact_length ++ (32 bytes × N)
	let mut encoded_retire = encode_compact(to_remove.len());
	for v in to_remove {
		encoded_retire.extend_from_slice(v);
	}
	println!("\nSCALE-encoded ValidatorsToRetire: {} bytes", encoded_retire.len());
	println!(
		"  compact_length: 0x{} (count={})",
		hex::encode(&encode_compact(to_remove.len())),
		to_remove.len()
	);

	// Storage key for ValidatorsToRetire
	let validators_to_retire_key =
		hex::decode("084e7f70a295a190e2e33fd3f8cdfcc2b664fa73499821e43a617aa0e82b17b1").unwrap();

	println!("\nStorage key: 0x{}", hex::encode(&validators_to_retire_key));
	println!(
		"Storage value: 0x{}...({} bytes)",
		hex::encode(&encoded_retire[..std::cmp::min(encoded_retire.len(), 40)]),
		encoded_retire.len()
	);

	// Load sudo key
	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("\nSudo account: {}", sudo_keypair.public_key().to_account_id());

	// Build sudo(system.setStorage(items))
	let set_storage_tx = pezkuwi_subxt::dynamic::tx(
		"System",
		"set_storage",
		vec![Value::unnamed_composite(vec![Value::unnamed_composite(vec![
			Value::from_bytes(&validators_to_retire_key),
			Value::from_bytes(&encoded_retire),
		])])],
	);

	let sudo_call = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![set_storage_tx.into_value()]);

	println!("\nSubmitting sudo(system.setStorage) to write ValidatorsToRetire...\n");

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
		println!("\nFAILED!");
		return Ok(());
	}

	// Verify by reading back the storage
	println!("\n=== VERIFICATION ===");
	tokio::time::sleep(std::time::Duration::from_secs(3)).await;

	let api2 = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	match api2
		.storage()
		.at_latest()
		.await?
		.fetch_raw(
			hex::decode("084e7f70a295a190e2e33fd3f8cdfcc2b664fa73499821e43a617aa0e82b17b1")
				.unwrap(),
		)
		.await
	{
		Ok(data) => {
			let stored_count = (data[0] >> 2) as usize;
			println!("ValidatorsToRetire: {} entries ({} bytes)", stored_count, data.len());
			if stored_count == to_remove.len() {
				println!("COUNT MATCHES! Storage write successful.");
			} else {
				println!("COUNT MISMATCH! Expected {}, got {}", to_remove.len(), stored_count);
			}
			// Show first few
			let mut off = 1;
			for i in 0..std::cmp::min(stored_count, 3) {
				if off + 32 <= data.len() {
					println!("  [{}] 0x{}", i + 1, hex::encode(&data[off..off + 32]));
					off += 32;
				}
			}
			if stored_count > 3 {
				println!("  ... ({} more)", stored_count - 3);
			}
		},
		Err(e) => {
			println!("ValidatorsToRetire: ERROR reading back: {}", e);
			println!("Storage might not have been written!");
		},
	}

	println!("\n=== DONE ===");
	println!("ValidatorsToRetire is now set with {} validators to remove.", to_remove.len());
	println!("At next session change, new_session() will take() these and remove them.");
	println!("Then at session+1 after that, GRANDPA authorities should change.");

	// Show timing info
	println!("\nSession = 600 slots × 6 sec = 60 min");
	println!("Expected GRANDPA change: ~60-120 minutes from now.");

	Ok(())
}
