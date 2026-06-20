//! Zagros: List validators and optionally deregister via ValidatorManager
//!
//! Step 1 (DRY RUN by default): List all validators, show which to keep/remove
//! Step 2 (with EXECUTE=1): Actually submit the deregister tx
//!
//! Run with:
//!   RPC_URL="ws://217.77.6.126:9948" \
//!   cargo run --release --example zagros_deregister -p pezkuwi-subxt
//!
//! To actually execute:
//!   SUDO_MNEMONIC="******" EXECUTE=1 KEEP=2 \
//!   RPC_URL="ws://217.77.6.126:9948" \
//!   cargo run --release --example zagros_deregister -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

/// Decode SCALE compact length prefix
fn decode_compact(data: &[u8]) -> (usize, usize) {
	let first = data[0];
	match first & 0x03 {
		0 => ((first >> 2) as usize, 1),
		1 => {
			let val = (((data[1] as u16) << 8 | first as u16) >> 2) as usize;
			(val, 2)
		},
		2 => {
			let val = (((data[3] as u32) << 24)
				| ((data[2] as u32) << 16)
				| ((data[1] as u32) << 8)
				| (first as u32))
				>> 2;
			(val as usize, 4)
		},
		_ => panic!("Big integer compact encoding not supported"),
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== ZAGROS: VALIDATOR DEREGISTRATION ===\n");

	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9948".to_string());
	let keep: usize = std::env::var("KEEP").unwrap_or_else(|_| "2".to_string()).parse()?;
	let execute = std::env::var("EXECUTE").unwrap_or_default() == "1";

	println!("RPC: {}", url);
	println!("Keep: {} validators", keep);
	println!("Mode: {}\n", if execute { "EXECUTE" } else { "DRY RUN" });

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected! specVersion: {}\n", api.runtime_version().spec_version);

	// Query QueuedKeys via raw storage — we know this key works and returns 21 entries
	// QueuedKeys = Vec<(ValidatorId, Keys)>
	// Storage key: twox128("Session") + twox128("QueuedKeys")
	let queued_keys_key =
		hex::decode("cec5070d609dd3497f72bde07fc96ba088dcde934c658227ee1dfafcd6e16903").unwrap();

	let raw_data = api.storage().at_latest().await?.fetch_raw(queued_keys_key).await?;

	if raw_data.is_empty() {
		println!("ERROR: QueuedKeys storage is empty!");
		return Ok(());
	}

	// Decode Vec length
	let (count, offset) = decode_compact(&raw_data);
	println!("QueuedKeys entries: {}", count);

	// Each entry: AccountId32 (32 bytes) + SessionKeys
	// SessionKeys for relay chain:
	//   grandpa: 32 bytes
	//   babe: 32 bytes
	//   im_online: 32 bytes (ImOnlineId)
	//   para_validator: 32 bytes
	//   para_assignment: 32 bytes
	//   authority_discovery: 32 bytes
	//   beefy: 33 bytes (ECDSA compressed)
	// Total SessionKeys = 32*6 + 33 = 225 bytes
	// Each entry = 32 (AccountId) + 225 (SessionKeys) = 257 bytes

	// But we need to verify this. Let's compute expected total size:
	let expected_entry_size = 32 + (32 * 6 + 33); // 257
	let expected_total = 1 + (count * expected_entry_size); // 1 byte compact + entries
	println!("Expected data size: {} bytes, actual: {} bytes", expected_total, raw_data.len());

	if raw_data.len() < offset + count * expected_entry_size {
		// Try without beefy (older runtime might not have it)
		let entry_no_beefy = 32 + (32 * 6); // 224
		let expected_no_beefy = 1 + (count * entry_no_beefy);
		println!("Without beefy: expected {} bytes", expected_no_beefy);

		if raw_data.len() >= offset + count * entry_no_beefy {
			println!("Using SessionKeys without Beefy (6 keys x 32 bytes)");
			extract_and_process(&raw_data, offset, count, entry_no_beefy, keep, execute, &api)
				.await?;
		} else {
			// Auto-detect entry size
			let remaining = raw_data.len() - offset;
			let entry_size = remaining / count;
			println!(
				"Auto-detected entry size: {} bytes (remaining={}, count={})",
				entry_size, remaining, count
			);
			extract_and_process(&raw_data, offset, count, entry_size, keep, execute, &api).await?;
		}
	} else {
		println!("Using SessionKeys with Beefy (6 keys x 32 + 33 beefy)");
		extract_and_process(&raw_data, offset, count, expected_entry_size, keep, execute, &api)
			.await?;
	}

	Ok(())
}

async fn extract_and_process(
	raw_data: &[u8],
	mut offset: usize,
	count: usize,
	entry_size: usize,
	keep: usize,
	execute: bool,
	api: &OnlineClient<PezkuwiConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
	let mut all_validators: Vec<Vec<u8>> = Vec::new();

	println!("\nValidators:\n");
	for i in 0..count {
		let account = raw_data[offset..offset + 32].to_vec();
		let label = if i < keep { "KEEP  " } else { "REMOVE" };
		println!("  [{:2}] [{}] 0x{}", i + 1, label, hex::encode(&account));
		all_validators.push(account);
		offset += entry_size;
	}

	if count <= keep {
		println!("\nAlready at {} validators, nothing to remove.", count);
		return Ok(());
	}

	let to_remove = &all_validators[keep..];
	println!("\n--- Summary ---");
	println!("Total: {}", count);
	println!("Keep: {}", keep);
	println!("Remove: {}", to_remove.len());

	if !execute {
		println!("\nDRY RUN complete. Set EXECUTE=1 and SUDO_MNEMONIC to submit.");
		return Ok(());
	}

	// Load sudo key
	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("\nSudo account: {}", sudo_keypair.public_key().to_account_id());

	// Build validators list for deregister call
	let validators_value: Vec<Value> = to_remove.iter().map(|v| Value::from_bytes(v)).collect();

	let deregister_call = pezkuwi_subxt::dynamic::tx(
		"ValidatorManager",
		"deregister_validators",
		vec![Value::unnamed_composite(validators_value)],
	);

	let sudo_call = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![deregister_call.into_value()]);

	println!("Submitting sudo(validatorManager.deregister_validators)...\n");

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
								println!("    >>> ValidatorsDeregistered event confirmed!");
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

	if success {
		println!("\nSUCCESS! {} validators queued for deregistration.", to_remove.len());
		println!("The change will take effect at current_session + 2.");
		println!("Monitor GRANDPA authorities to confirm.");
	} else {
		println!("\nFAILED!");
	}

	Ok(())
}
