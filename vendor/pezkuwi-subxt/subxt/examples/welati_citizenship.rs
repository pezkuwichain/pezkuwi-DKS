//! Welati Citizenship: Transfer to People Chain + Apply + Approve + Confirm
//!
//! Steps:
//! 1. Transfer 10 HEZ from founder to each wallet on People Chain
//! 2. Each wallet applies for citizenship (IdentityKyc.apply_for_citizenship)
//! 3. Founder approves each referral (IdentityKyc.approve_referral)
//! 4. Each wallet confirms citizenship (IdentityKyc.confirm_citizenship) → Welati NFT minted
//!
//! Environment variables:
//!   FOUNDER_MNEMONIC  - Founder wallet mnemonic (required)
//!   WALLETS_FILE      - JSON file with wallet list (required)
//!   PEOPLE_RPC        - People Chain RPC endpoint (default: ws://217.77.6.126:41944)
//!   SKIP              - Number of wallets to skip (default: 0)
//!
//! Wallets JSON format:
//!   [
//!     { "name": "Pool Name", "mnemonic": "word1 word2 ...", "ss58": "5..." },
//!     ...
//!   ]
//!
//! Run with:
//!   FOUNDER_MNEMONIC="..." WALLETS_FILE="wallets.json" \
//!     cargo run --release -p pezkuwi-subxt --example welati_citizenship
//!
//!   # Or run a specific phase:
//!   FOUNDER_MNEMONIC="..." WALLETS_FILE="wallets.json" \
//!     cargo run --release -p pezkuwi-subxt --example welati_citizenship -- transfer

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const PLANCKS_PER_HEZ: u128 = 1_000_000_000_000;
const DEFAULT_PEOPLE_RPC: &str = "ws://217.77.6.126:41944";

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

async fn submit_and_watch(
	api: &OnlineClient<PezkuwiConfig>,
	tx: pezkuwi_subxt::tx::DynamicPayload,
	signer: &Keypair,
	label: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
	use pezkuwi_subxt::tx::TxStatus;

	for attempt in 0..3 {
		if attempt > 0 {
			println!("  Retry {}...", attempt + 1);
			tokio::time::sleep(std::time::Duration::from_secs(18)).await;
		}

		let tx_progress = match api.tx().sign_and_submit_then_watch_default(&tx, signer).await {
			Ok(p) => p,
			Err(e) => {
				println!("  SUBMIT ERROR (attempt {}): {}", attempt + 1, e);
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
					}
				},
				Some(Ok(TxStatus::Error { message })) => {
					println!("  {} TX ERROR: {}", label, message);
					break;
				},
				Some(Ok(TxStatus::Invalid { message })) => {
					println!("  {} TX INVALID: {}", label, message);
					break;
				},
				Some(Ok(TxStatus::Dropped { message })) => {
					println!("  {} TX DROPPED: {}", label, message);
					break;
				},
				Some(Err(e)) => {
					println!("  {} STREAM ERROR: {}", label, e);
					return Err(e.into());
				},
				None => {
					println!("  {} STREAM ENDED", label);
					break;
				},
				_ => {},
			}
		}
	}
	Ok(false)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let ws = load_wallets();
	let args: Vec<String> = std::env::args().collect();
	let phase = args.get(1).map(|s| s.as_str()).unwrap_or("all");
	let skip: usize =
		std::env::var("SKIP").unwrap_or_else(|_| "0".to_string()).parse().unwrap_or(0);
	let rpc = std::env::var("PEOPLE_RPC").unwrap_or_else(|_| DEFAULT_PEOPLE_RPC.to_string());

	println!("=== WELATI CITIZENSHIP WORKFLOW ===");
	println!("People Chain RPC: {}", rpc);
	println!("Phase: {}", phase);
	println!("Wallets loaded: {}", ws.len());
	println!("Skip: {}\n", skip);

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&rpc).await?;
	println!("Connected to People Chain!\n");

	// ========== PHASE 1: TRANSFERS ==========
	if phase == "all" || phase == "transfer" {
		println!("========== PHASE 1: TRANSFER 10 HEZ TO PEOPLE CHAIN ==========\n");

		let mnemonic = Mnemonic::from_str(&founder_mnemonic())?;
		let founder_keypair = Keypair::from_phrase(&mnemonic, None)?;
		println!("Founder: {}\n", founder_keypair.public_key().to_account_id());

		for (i, w) in ws.iter().enumerate().skip(skip) {
			println!("--- [{}/{}] {} ({}) ---", i + 1, ws.len(), w.name, w.ss58);

			let dest: AccountId32 = w.ss58.parse()?;
			let amount = 10 * PLANCKS_PER_HEZ;

			let tx = pezkuwi_subxt::dynamic::tx(
				"Balances",
				"transfer_keep_alive",
				vec![
					Value::unnamed_variant("Id", vec![Value::from_bytes(&dest.0)]),
					Value::u128(amount),
				],
			);

			let ok = submit_and_watch(&api, tx, &founder_keypair, "TRANSFER").await?;
			if !ok {
				println!("  FAILED! Stopping.");
				return Ok(());
			}

			if i + 1 < ws.len() {
				tokio::time::sleep(std::time::Duration::from_secs(18)).await;
			}
		}
		println!("\n========== ALL TRANSFERS DONE ==========\n");

		if phase == "transfer" {
			return Ok(());
		}
		tokio::time::sleep(std::time::Duration::from_secs(18)).await;
	}

	// ========== PHASE 2: APPLY FOR CITIZENSHIP ==========
	if phase == "all" || phase == "apply" {
		println!("========== PHASE 2: APPLY FOR CITIZENSHIP ==========\n");

		for (i, w) in ws.iter().enumerate().skip(skip) {
			println!("--- [{}/{}] {} applying ---", i + 1, ws.len(), w.name);

			let mnemonic = Mnemonic::from_str(&w.mnemonic)?;
			let keypair = Keypair::from_phrase(&mnemonic, None)?;

			// Generate identity hash: H256(name)
			let identity_hash = pezsp_crypto_hashing::blake2_256(w.name.as_bytes());

			// IdentityKyc.apply_for_citizenship(identity_hash, referrer=None)
			// referrer=None will default to founder
			let tx = pezkuwi_subxt::dynamic::tx(
				"IdentityKyc",
				"apply_for_citizenship",
				vec![Value::from_bytes(&identity_hash), Value::unnamed_variant("None", vec![])],
			);

			let ok = submit_and_watch(&api, tx, &keypair, "APPLY").await?;
			if !ok {
				println!("  FAILED! Continuing...");
			}

			if i + 1 < ws.len() {
				tokio::time::sleep(std::time::Duration::from_secs(18)).await;
			}
		}
		println!("\n========== ALL APPLICATIONS SUBMITTED ==========\n");

		if phase == "apply" {
			return Ok(());
		}
		tokio::time::sleep(std::time::Duration::from_secs(18)).await;
	}

	// ========== PHASE 3: FOUNDER APPROVES REFERRALS ==========
	if phase == "all" || phase == "approve" {
		println!("========== PHASE 3: FOUNDER APPROVES REFERRALS ==========\n");

		let mnemonic = Mnemonic::from_str(&founder_mnemonic())?;
		let founder_keypair = Keypair::from_phrase(&mnemonic, None)?;
		println!("Founder: {}\n", founder_keypair.public_key().to_account_id());

		for (i, w) in ws.iter().enumerate().skip(skip) {
			println!("--- [{}/{}] Approving {} ---", i + 1, ws.len(), w.name);

			let applicant: AccountId32 = w.ss58.parse()?;

			// IdentityKyc.approve_referral(applicant)
			let tx = pezkuwi_subxt::dynamic::tx(
				"IdentityKyc",
				"approve_referral",
				vec![Value::from_bytes(&applicant.0)],
			);

			let ok = submit_and_watch(&api, tx, &founder_keypair, "APPROVE").await?;
			if !ok {
				println!("  FAILED! Continuing...");
			}

			if i + 1 < ws.len() {
				tokio::time::sleep(std::time::Duration::from_secs(18)).await;
			}
		}
		println!("\n========== ALL REFERRALS APPROVED ==========\n");

		if phase == "approve" {
			return Ok(());
		}
		tokio::time::sleep(std::time::Duration::from_secs(18)).await;
	}

	// ========== PHASE 4: CONFIRM CITIZENSHIP (MINT WELATI) ==========
	if phase == "all" || phase == "confirm" {
		println!("========== PHASE 4: CONFIRM CITIZENSHIP ==========\n");

		for (i, w) in ws.iter().enumerate().skip(skip) {
			println!("--- [{}/{}] {} confirming ---", i + 1, ws.len(), w.name);

			let mnemonic = Mnemonic::from_str(&w.mnemonic)?;
			let keypair = Keypair::from_phrase(&mnemonic, None)?;

			// IdentityKyc.confirm_citizenship()
			let tx = pezkuwi_subxt::dynamic::tx(
				"IdentityKyc",
				"confirm_citizenship",
				Vec::<Value>::new(),
			);

			let ok = submit_and_watch(&api, tx, &keypair, "CONFIRM").await?;
			if !ok {
				println!("  FAILED! Continuing...");
			}

			if i + 1 < ws.len() {
				tokio::time::sleep(std::time::Duration::from_secs(18)).await;
			}
		}
		println!("\n========== ALL CITIZENSHIPS CONFIRMED ==========\n");
	}

	println!("=== DONE ===");
	Ok(())
}
