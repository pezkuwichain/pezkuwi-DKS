//! Bond extra HEZ for all 21 validators
//!
//! Reads stash seed phrases from WALLETS_FILE, calls staking.bond_extra for each.
//!
//! Run with:
//!   WALLETS_FILE="/home/mamostehp/res/MAINNET_WALLETS_20260128_235407.json" \
//!   RPC_URL="ws://217.77.6.126:9944" \
//!   BOND_EXTRA_HEZ=499000 \
//!   cargo run --release --example bond_extra_validators -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const PLANCKS_PER_HEZ: u128 = 1_000_000_000_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== BOND EXTRA HEZ FOR VALIDATORS ===\n");

	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9944".to_string());
	let wallets_file = std::env::var("WALLETS_FILE")
		.expect("WALLETS_FILE env var required (e.g. export WALLETS_FILE=/path/to/wallets.json)");
	let bond_hez: u128 = std::env::var("BOND_EXTRA_HEZ")
		.unwrap_or_else(|_| "499000".to_string())
		.parse()?;
	let bond_planck = bond_hez * PLANCKS_PER_HEZ;
	let skip: usize =
		std::env::var("SKIP").unwrap_or_else(|_| "0".to_string()).parse().unwrap_or(0);

	println!("RPC: {}", url);
	println!("Wallets file: {}", wallets_file);
	println!("Bond extra per validator: {} HEZ", bond_hez);

	// Read wallet file
	let wallet_data: serde_json::Value =
		serde_json::from_str(&std::fs::read_to_string(&wallets_file)?)?;
	let wallets = wallet_data["wallets"].as_array().expect("wallets array not found");

	// Extract stash wallets (Validator_XX_Stash)
	let mut stash_wallets: Vec<(&str, &str)> = Vec::new();
	for w in wallets {
		let name = w["name"].as_str().unwrap_or("");
		if name.contains("Stash") && name.starts_with("Validator_") {
			let seed = w["seed_phrase"].as_str().expect("seed_phrase missing");
			stash_wallets.push((name, seed));
		}
	}
	stash_wallets.sort_by_key(|(name, _)| name.to_string());

	println!("Found {} stash wallets", stash_wallets.len());
	println!(
		"Total bond: {} HEZ to {} validators (skipping {})\n",
		bond_hez * (stash_wallets.len() - skip) as u128,
		stash_wallets.len() - skip,
		skip
	);

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected!\n");

	let mut success_count = 0;
	let mut fail_count = 0;

	for (i, (name, seed)) in stash_wallets.iter().enumerate().skip(skip) {
		println!("--- [{}/{}] {} ---", i + 1, stash_wallets.len(), name);

		let mnemonic = match Mnemonic::from_str(seed) {
			Ok(m) => m,
			Err(e) => {
				println!("  ERROR: Invalid mnemonic: {}", e);
				fail_count += 1;
				continue;
			},
		};
		let keypair = match Keypair::from_phrase(&mnemonic, None) {
			Ok(k) => k,
			Err(e) => {
				println!("  ERROR: Keypair error: {}", e);
				fail_count += 1;
				continue;
			},
		};
		let account = keypair.public_key().to_account_id();
		println!("  Account: {}", account);

		// staking.bond_extra(max_additional: Balance)
		let bond_extra_tx =
			pezkuwi_subxt::dynamic::tx("Staking", "bond_extra", vec![Value::u128(bond_planck)]);

		use pezkuwi_subxt::tx::TxStatus;
		let mut tx_ok = false;

		for attempt in 0..3 {
			let tx_progress =
				match api.tx().sign_and_submit_then_watch_default(&bond_extra_tx, &keypair).await {
					Ok(p) => p,
					Err(e) => {
						println!("  SUBMIT ERROR (attempt {}): {}", attempt + 1, e);
						tokio::time::sleep(std::time::Duration::from_secs(12)).await;
						continue;
					},
				};

			println!("  TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

			let mut progress = tx_progress;
			loop {
				let status = progress.next().await;
				match status {
					Some(Ok(TxStatus::InBestBlock(details))) => {
						match details.wait_for_success().await {
							Ok(events) => {
								for ev in events.iter().flatten() {
									if ev.pallet_name() == "Staking"
										&& ev.variant_name() == "Bonded"
									{
										println!("  SUCCESS: {} HEZ bonded", bond_hez);
										tx_ok = true;
									}
								}
								if !tx_ok {
									println!("  WARNING: No Staking::Bonded event");
									for ev in events.iter().flatten() {
										println!("    {}::{}", ev.pallet_name(), ev.variant_name());
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

			if tx_ok {
				break;
			}
			tokio::time::sleep(std::time::Duration::from_secs(12)).await;
		}

		if tx_ok {
			success_count += 1;
		} else {
			fail_count += 1;
		}

		// Wait between transactions (different signers so nonce isn't an issue,
		// but still good to not flood the mempool)
		if i + 1 < stash_wallets.len() {
			tokio::time::sleep(std::time::Duration::from_secs(6)).await;
		}
	}

	println!("\n=== RESULTS ===");
	println!("Success: {}/{}", success_count, stash_wallets.len() - skip);
	println!("Failed:  {}/{}", fail_count, stash_wallets.len() - skip);
	println!("Total bonded: {} HEZ", bond_hez * success_count as u128);

	Ok(())
}
