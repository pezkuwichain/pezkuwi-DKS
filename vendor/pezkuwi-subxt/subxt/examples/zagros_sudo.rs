//! Zagros Testnet: Generic sudo call sender
//!
//! Run with:
//!   SUDO_MNEMONIC="..." RPC_URL="ws://..." CALL=setValidatorCount|forceNewEra \
//!   cargo run --release --example zagros_sudo

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9948".to_string());
	let call_name = std::env::var("CALL").unwrap_or_else(|_| "setValidatorCount".to_string());

	println!("RPC: {}", url);
	println!("Call: {}", call_name);

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected!");

	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sudo: {}", sudo_keypair.public_key().to_account_id());

	let inner_call = match call_name.as_str() {
		"setValidatorCount" => {
			let count: u32 = std::env::var("COUNT").unwrap_or_else(|_| "4".to_string()).parse()?;
			println!("Setting validator count to {}", count);
			pezkuwi_subxt::dynamic::tx(
				"Staking",
				"set_validator_count",
				vec![Value::u128(count as u128)],
			)
		},
		"forceNewEra" => {
			println!("Forcing new era");
			pezkuwi_subxt::dynamic::tx("Staking", "force_new_era", Vec::<Value>::new())
		},
		"forceNewEraAlways" => {
			println!("Forcing new era always");
			pezkuwi_subxt::dynamic::tx("Staking", "force_new_era_always", Vec::<Value>::new())
		},
		"setStakingConfigs" => {
			// Deprecated: use setMinValidatorCount instead
			eprintln!("Use setMinValidatorCount instead");
			std::process::exit(1);
		},
		"setMinValidatorCount" => {
			let min_count: u32 =
				std::env::var("MIN_COUNT").unwrap_or_else(|_| "1".to_string()).parse().unwrap();
			println!("Setting minimum validator count to {}", min_count);
			let noop = Value::unnamed_variant("Noop", Vec::<Value>::new());
			let set_min = Value::unnamed_variant("Set", vec![Value::u128(min_count as u128)]);
			pezkuwi_subxt::dynamic::tx(
				"Staking",
				"set_staking_configs",
				vec![
					noop.clone(), // min_nominator_bond
					noop.clone(), // min_validator_bond
					noop.clone(), // max_nominator_count
					noop.clone(), // max_validator_count
					noop.clone(), // chill_threshold
					set_min,      // min_commission (used as min_validator_count proxy)
					noop.clone(), // max_staked_rewards
				],
			)
		},
		"setStorage" => {
			// Set arbitrary storage via sudo(system.setStorage)
			let key_hex = std::env::var("STORAGE_KEY").expect("STORAGE_KEY env var required");
			let value_hex = std::env::var("STORAGE_VALUE").expect("STORAGE_VALUE env var required");
			println!("Setting storage key={} value={}", key_hex, value_hex);

			let key_bytes = hex::decode(key_hex.trim_start_matches("0x")).unwrap();
			let value_bytes = hex::decode(value_hex.trim_start_matches("0x")).unwrap();

			// system.setStorage takes Vec<(Key, Value)>
			let item = Value::unnamed_composite([
				Value::from_bytes(&key_bytes),
				Value::from_bytes(&value_bytes),
			]);
			pezkuwi_subxt::dynamic::tx(
				"System",
				"set_storage",
				vec![Value::unnamed_composite([item])],
			)
		},
		_ => {
			eprintln!("Unknown call: {}", call_name);
			std::process::exit(1);
		},
	};

	let sudo_tx = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![inner_call.into_value()]);

	println!("\nSubmitting...");

	// Use sign_and_submit_then_watch to see TX lifecycle
	let tx_progress = api.tx().sign_and_submit_then_watch_default(&sudo_tx, &sudo_keypair).await?;

	println!("TX hash: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));
	println!("Watching TX status (Ctrl+C to abort)...");

	// Don't wait for finalization - just wait for in_block
	use pezkuwi_subxt::tx::TxStatus;

	let mut progress = tx_progress;
	loop {
		let status = progress.next().await;
		match status {
			Some(Ok(TxStatus::Validated)) => println!("  Status: Validated (in tx pool)"),
			Some(Ok(TxStatus::Broadcasted)) => println!("  Status: Broadcasted"),
			Some(Ok(TxStatus::InBestBlock(details))) => {
				println!("  Status: InBestBlock {:?}", details.block_hash());
				match details.wait_for_success().await {
					Ok(events) => {
						println!("  TX SUCCESS!");
						for ev in events.iter().flatten() {
							println!("    Event: {}::{}", ev.pallet_name(), ev.variant_name());
						}
					},
					Err(e) => println!("  TX dispatch error: {}", e),
				}
				break;
			},
			Some(Ok(TxStatus::InFinalizedBlock(details))) => {
				println!("  Status: Finalized {:?}", details.block_hash());
				break;
			},
			Some(Ok(TxStatus::Error { message })) => {
				println!("  Status: ERROR - {}", message);
				break;
			},
			Some(Ok(TxStatus::Invalid { message })) => {
				println!("  Status: INVALID - {}", message);
				break;
			},
			Some(Ok(TxStatus::Dropped { message })) => {
				println!("  Status: DROPPED - {}", message);
				break;
			},
			Some(Ok(TxStatus::NoLongerInBestBlock)) => {
				println!("  Status: No longer in best block");
			},
			Some(Err(e)) => {
				println!("  Stream error: {}", e);
				break;
			},
			None => {
				println!("  Stream ended");
				break;
			},
		}
	}

	println!("\nDone.");
	Ok(())
}
