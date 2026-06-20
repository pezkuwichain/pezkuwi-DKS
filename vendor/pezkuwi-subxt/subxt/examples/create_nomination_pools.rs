//! Create Nomination Pools on Asset Hub
//!
//! Steps:
//! 1. Transfer HEZ from founder to each pool wallet (on Asset Hub)
//! 2. Each wallet creates a nomination pool with specified stake
//! 3. Set pool metadata (name)
//!
//! Environment variables:
//!   FOUNDER_MNEMONIC   - Founder wallet mnemonic (required)
//!   WALLETS_FILE       - JSON file with wallet list (required)
//!   ASSET_HUB_RPC      - Asset Hub RPC endpoint (default: ws://217.77.6.126:40944)
//!   SKIP               - Number of wallets to skip (default: 0)
//!   TRANSFER_HEZ       - HEZ to transfer to each wallet (default: 500000)
//!   BASE_STAKE_HEZ     - Starting stake for first pool (default: 490000, decreases by 10000 per pool)
//!
//! Wallets JSON format:
//!   [
//!     { "name": "Pool Name", "mnemonic": "word1 word2 ...", "ss58": "5..." },
//!     ...
//!   ]
//!
//! Run with:
//!   FOUNDER_MNEMONIC="..." WALLETS_FILE="wallets.json" \
//!     cargo run --release --example create_nomination_pools
//!
//!   # Or run a specific phase:
//!   FOUNDER_MNEMONIC="..." WALLETS_FILE="wallets.json" \
//!     cargo run --release --example create_nomination_pools -- transfer

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const PLANCKS_PER_HEZ: u128 = 1_000_000_000_000;
const DEFAULT_ASSET_HUB_RPC: &str = "ws://217.77.6.126:40944";

#[derive(serde::Deserialize)]
struct WalletInfo {
	name: String,
	mnemonic: String,
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

fn founder_mnemonic() -> String {
	std::env::var("FOUNDER_MNEMONIC").expect("FOUNDER_MNEMONIC environment variable required")
}

struct PoolConfig {
	name: String,
	mnemonic: String,
	ss58: String,
	transfer_hez: u128,
	stake_hez: u128,
}

fn build_pool_configs(wallets: Vec<WalletInfo>) -> Vec<PoolConfig> {
	let transfer_hez: u128 = std::env::var("TRANSFER_HEZ")
		.unwrap_or_else(|_| "500000".to_string())
		.parse()
		.expect("TRANSFER_HEZ must be a valid number");

	let base_stake: u128 = std::env::var("BASE_STAKE_HEZ")
		.unwrap_or_else(|_| "490000".to_string())
		.parse()
		.expect("BASE_STAKE_HEZ must be a valid number");

	wallets
		.into_iter()
		.enumerate()
		.map(|(i, w)| {
			let stake_hez = base_stake.saturating_sub(i as u128 * 10_000);
			PoolConfig { name: w.name, mnemonic: w.mnemonic, ss58: w.ss58, transfer_hez, stake_hez }
		})
		.collect()
}

async fn wait_for_success(
	mut progress: pezkuwi_subxt::tx::TxProgress<PezkuwiConfig, OnlineClient<PezkuwiConfig>>,
	label: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
	use pezkuwi_subxt::tx::TxStatus;
	loop {
		let status = progress.next().await;
		match status {
			Some(Ok(TxStatus::InBestBlock(details))) => match details.wait_for_success().await {
				Ok(events) => {
					println!("  {} SUCCESS!", label);
					for ev in events.iter().flatten() {
						println!("    {}::{}", ev.pallet_name(), ev.variant_name());
					}
					return Ok(true);
				},
				Err(e) => {
					println!("  {} DISPATCH ERROR: {}", label, e);
					return Ok(false);
				},
			},
			Some(Ok(TxStatus::Error { message })) => {
				println!("  {} TX ERROR: {}", label, message);
				return Ok(false);
			},
			Some(Ok(TxStatus::Invalid { message })) => {
				println!("  {} TX INVALID: {}", label, message);
				return Ok(false);
			},
			Some(Ok(TxStatus::Dropped { message })) => {
				println!("  {} TX DROPPED: {}", label, message);
				return Ok(false);
			},
			Some(Err(e)) => {
				println!("  {} STREAM ERROR: {}", label, e);
				return Err(e.into());
			},
			None => {
				println!("  {} STREAM ENDED", label);
				return Ok(false);
			},
			_ => {},
		}
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let wallets = load_wallets();
	let pool_configs = build_pool_configs(wallets);

	// Parse CLI args
	let args: Vec<String> = std::env::args().collect();
	let phase = args.get(1).map(|s| s.as_str()).unwrap_or("all");
	let skip: usize =
		std::env::var("SKIP").unwrap_or_else(|_| "0".to_string()).parse().unwrap_or(0);

	let rpc = std::env::var("ASSET_HUB_RPC").unwrap_or_else(|_| DEFAULT_ASSET_HUB_RPC.to_string());

	println!("=== NOMINATION POOL CREATOR ===");
	println!("Asset Hub RPC: {}", rpc);
	println!("Phase: {}", phase);
	println!("Skip: {}", skip);
	println!("Pools: {}\n", pool_configs.len());

	// Connect to Asset Hub
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&rpc).await?;
	println!("Connected to Asset Hub!\n");

	// ========== PHASE 1: TRANSFERS ==========
	if phase == "all" || phase == "transfer" {
		println!("========== PHASE 1: TRANSFERS ==========\n");

		let founder_mn = Mnemonic::from_str(&founder_mnemonic())?;
		let founder_keypair = Keypair::from_phrase(&founder_mn, None)?;
		println!("Founder: {}\n", founder_keypair.public_key().to_account_id());

		for (i, pool) in pool_configs.iter().enumerate().skip(skip) {
			println!(
				"--- [{}/{}] Transfer {} HEZ -> {} ({}) ---",
				i + 1,
				pool_configs.len(),
				pool.transfer_hez,
				pool.name,
				pool.ss58
			);

			let dest: AccountId32 = pool.ss58.parse()?;
			let amount_planck = pool.transfer_hez * PLANCKS_PER_HEZ;

			let mut tx_ok = false;
			for attempt in 0..3 {
				if attempt > 0 {
					println!("  Retry attempt {}...", attempt + 1);
					tokio::time::sleep(std::time::Duration::from_secs(18)).await;
				}

				let transfer_tx = pezkuwi_subxt::dynamic::tx(
					"Balances",
					"transfer_keep_alive",
					vec![
						Value::unnamed_variant("Id", vec![Value::from_bytes(&dest.0)]),
						Value::u128(amount_planck),
					],
				);

				let tx_progress = match api
					.tx()
					.sign_and_submit_then_watch_default(&transfer_tx, &founder_keypair)
					.await
				{
					Ok(p) => p,
					Err(e) => {
						println!("  SUBMIT ERROR (attempt {}): {}", attempt + 1, e);
						continue;
					},
				};

				println!("  TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

				tx_ok = wait_for_success(tx_progress, "TRANSFER").await?;
				if tx_ok {
					break;
				}
			}

			if !tx_ok {
				println!("  FAILED after 3 attempts! Stopping.");
				return Ok(());
			}

			// Wait between transactions for nonce to update
			if i + 1 < pool_configs.len() {
				println!("  Waiting 18s for next block...");
				tokio::time::sleep(std::time::Duration::from_secs(18)).await;
			}
		}

		println!("\n========== ALL TRANSFERS DONE ==========\n");

		if phase == "transfer" {
			return Ok(());
		}

		// Wait before pool creation
		println!("Waiting 24s before pool creation...\n");
		tokio::time::sleep(std::time::Duration::from_secs(24)).await;
	}

	// ========== PHASE 2: CREATE POOLS ==========
	if phase == "all" || phase == "pools" {
		println!("========== PHASE 2: CREATE POOLS ==========\n");

		for (i, pool) in pool_configs.iter().enumerate().skip(skip) {
			println!(
				"--- [{}/{}] Create pool '{}' with {} HEZ stake ---",
				i + 1,
				pool_configs.len(),
				pool.name,
				pool.stake_hez
			);

			// Load pool wallet keypair
			let pool_mnemonic = Mnemonic::from_str(&pool.mnemonic)?;
			let pool_keypair = Keypair::from_phrase(&pool_mnemonic, None)?;
			let pool_account = pool_keypair.public_key().to_account_id();
			println!("  Wallet: {}", pool_account);

			let stake_planck = pool.stake_hez * PLANCKS_PER_HEZ;

			// NominationPools::create(amount, root, nominator, bouncer)
			let mut create_ok = false;
			for attempt in 0..3 {
				if attempt > 0 {
					println!("  Create retry attempt {}...", attempt + 1);
					tokio::time::sleep(std::time::Duration::from_secs(18)).await;
				}

				let create_tx = pezkuwi_subxt::dynamic::tx(
					"NominationPools",
					"create",
					vec![
						Value::u128(stake_planck),
						Value::unnamed_variant("Id", vec![Value::from_bytes(&pool_account.0)]),
						Value::unnamed_variant("Id", vec![Value::from_bytes(&pool_account.0)]),
						Value::unnamed_variant("Id", vec![Value::from_bytes(&pool_account.0)]),
					],
				);

				let tx_progress = match api
					.tx()
					.sign_and_submit_then_watch_default(&create_tx, &pool_keypair)
					.await
				{
					Ok(p) => p,
					Err(e) => {
						println!("  SUBMIT ERROR (attempt {}): {}", attempt + 1, e);
						continue;
					},
				};

				println!("  TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

				create_ok = wait_for_success(tx_progress, "CREATE_POOL").await?;
				if create_ok {
					break;
				}
			}

			if !create_ok {
				println!("  FAILED after 3 attempts! Continuing to next pool...");
				tokio::time::sleep(std::time::Duration::from_secs(18)).await;
				continue;
			}

			// Wait for pool creation to settle
			tokio::time::sleep(std::time::Duration::from_secs(12)).await;

			// Query LastPoolId to get the pool_id
			let last_pool_query =
				pezkuwi_subxt::dynamic::storage::<(), Value>("NominationPools", "LastPoolId");
			let storage_client = api.storage().at_latest().await?;
			let last_pool = storage_client.entry(last_pool_query)?.try_fetch(()).await?;

			let pool_id = match last_pool {
				Some(val) => {
					let decoded = val.decode()?;
					decoded.as_u128().unwrap_or(0) as u32
				},
				None => {
					println!("  WARNING: Could not read LastPoolId");
					(i + 1) as u32 // fallback
				},
			};
			println!("  Pool ID: {}", pool_id);

			// NominationPools::set_metadata(pool_id, metadata)
			tokio::time::sleep(std::time::Duration::from_secs(6)).await;
			let name_bytes = pool.name.as_bytes().to_vec();
			for attempt in 0..3 {
				if attempt > 0 {
					println!("  Metadata retry attempt {}...", attempt + 1);
					tokio::time::sleep(std::time::Duration::from_secs(6)).await;
				}

				let metadata_tx = pezkuwi_subxt::dynamic::tx(
					"NominationPools",
					"set_metadata",
					vec![Value::u128(pool_id as u128), Value::from_bytes(&name_bytes)],
				);

				let tx_progress = match api
					.tx()
					.sign_and_submit_then_watch_default(&metadata_tx, &pool_keypair)
					.await
				{
					Ok(p) => p,
					Err(e) => {
						println!("  METADATA SUBMIT ERROR (attempt {}): {}", attempt + 1, e);
						continue;
					},
				};

				println!("  METADATA TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

				let ok = wait_for_success(tx_progress, "SET_METADATA").await?;
				if ok {
					break;
				}
				if attempt == 2 {
					println!("  WARNING: Metadata set failed for pool {}", pool_id);
				}
			}

			println!(
				"  Pool '{}' (ID: {}) created with {} HEZ\n",
				pool.name, pool_id, pool.stake_hez
			);

			// Wait between pools
			if i + 1 < pool_configs.len() {
				tokio::time::sleep(std::time::Duration::from_secs(12)).await;
			}
		}

		println!("\n========== ALL POOLS CREATED ==========");
	}

	// ========== SUMMARY ==========
	println!("\n=== SUMMARY ===");
	for (i, pool) in pool_configs.iter().enumerate() {
		println!(
			"  Pool {}: '{}' - {} HEZ staked by {}",
			i + 1,
			pool.name,
			pool.stake_hez,
			pool.ss58
		);
	}
	let total_transfer: u128 = pool_configs.iter().map(|p| p.transfer_hez).sum();
	let total_stake: u128 = pool_configs.iter().map(|p| p.stake_hez).sum();
	println!("\n  Total transferred: {} HEZ", total_transfer);
	println!("  Total staked: {} HEZ", total_stake);

	Ok(())
}
