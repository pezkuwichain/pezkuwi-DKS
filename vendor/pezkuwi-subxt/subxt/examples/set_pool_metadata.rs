//! Set metadata (names) for existing nomination pools on Asset Hub
//!
//! Environment variables:
//!   WALLETS_FILE    - JSON file with wallet list (required)
//!   ASSET_HUB_RPC  - Asset Hub RPC endpoint (default: ws://217.77.6.126:40944)
//!   START_ID        - First pool ID to set metadata for (default: 1)
//!
//! Wallets JSON format:
//!   [
//!     { "name": "Pool Name", "mnemonic": "word1 word2 ...", "ss58": "5..." },
//!     ...
//!   ]
//!
//! Run with:
//!   WALLETS_FILE="wallets.json" \
//!     cargo run --release -p pezkuwi-subxt --example set_pool_metadata

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const DEFAULT_ASSET_HUB_RPC: &str = "ws://217.77.6.126:40944";

#[derive(serde::Deserialize)]
struct WalletInfo {
	name: String,
	mnemonic: String,
	#[allow(dead_code)]
	ss58: String,
}

fn load_wallets() -> Vec<WalletInfo> {
	let path = std::env::var("WALLETS_FILE").expect(
		"WALLETS_FILE environment variable required. \
		 Point it to a JSON file with wallet entries: \
		 [{\"name\": \"...\", \"mnemonic\": \"...\", \"ss58\": \"5...\"}]",
	);
	let data = std::fs::read_to_string(&path)
		.unwrap_or_else(|e| panic!("Failed to read wallets file '{}': {}", path, e));
	serde_json::from_str(&data)
		.unwrap_or_else(|e| panic!("Failed to parse wallets file '{}': {}", path, e))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== SET POOL METADATA ===\n");

	let start_id: u32 = std::env::var("START_ID").unwrap_or_else(|_| "1".to_string()).parse()?;

	let rpc = std::env::var("ASSET_HUB_RPC").unwrap_or_else(|_| DEFAULT_ASSET_HUB_RPC.to_string());

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&rpc).await?;
	println!("Connected to Asset Hub!\n");

	// First, query LastPoolId to confirm
	let last_pool_query =
		pezkuwi_subxt::dynamic::storage::<(), Value>("NominationPools", "LastPoolId");
	let storage = api.storage().at_latest().await?;
	let last_pool = storage.entry(last_pool_query)?.try_fetch(()).await?;
	if let Some(val) = last_pool {
		let decoded = val.decode()?;
		println!("LastPoolId raw value: {:?}", decoded);
		println!("LastPoolId as_u128: {:?}", decoded.as_u128());
	}

	let wallets = load_wallets();

	for (i, wallet) in wallets.iter().enumerate() {
		let pool_id = start_id + i as u32;
		println!("--- [{}/{}] Pool {} -> '{}' ---", i + 1, wallets.len(), pool_id, wallet.name);

		let mnemonic = Mnemonic::from_str(&wallet.mnemonic)?;
		let keypair = Keypair::from_phrase(&mnemonic, None)?;
		println!("  Signer: {}", keypair.public_key().to_account_id());

		let name_bytes = wallet.name.as_bytes().to_vec();
		let metadata_tx = pezkuwi_subxt::dynamic::tx(
			"NominationPools",
			"set_metadata",
			vec![Value::u128(pool_id as u128), Value::from_bytes(&name_bytes)],
		);

		let mut ok = false;
		for attempt in 0..3 {
			if attempt > 0 {
				println!("  Retry attempt {}...", attempt + 1);
				tokio::time::sleep(std::time::Duration::from_secs(18)).await;
			}

			let tx_progress =
				match api.tx().sign_and_submit_then_watch_default(&metadata_tx, &keypair).await {
					Ok(p) => p,
					Err(e) => {
						println!("  SUBMIT ERROR (attempt {}): {}", attempt + 1, e);
						continue;
					},
				};

			println!("  TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

			use pezkuwi_subxt::tx::TxStatus;
			let mut progress = tx_progress;
			loop {
				let status = progress.next().await;
				match status {
					Some(Ok(TxStatus::InBestBlock(details))) => {
						match details.wait_for_success().await {
							Ok(events) => {
								println!("  SUCCESS!");
								for ev in events.iter().flatten() {
									println!("    {}::{}", ev.pallet_name(), ev.variant_name());
								}
								ok = true;
							},
							Err(e) => {
								println!("  DISPATCH ERROR: {}", e);
							},
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

			if ok {
				break;
			}
		}

		if ok {
			println!("  Pool {} named '{}'\n", pool_id, wallet.name);
		} else {
			println!("  FAILED to name pool {}\n", pool_id);
		}

		// Wait between txs
		if i + 1 < wallets.len() {
			tokio::time::sleep(std::time::Duration::from_secs(18)).await;
		}
	}

	println!("\n=== DONE ===");
	Ok(())
}
