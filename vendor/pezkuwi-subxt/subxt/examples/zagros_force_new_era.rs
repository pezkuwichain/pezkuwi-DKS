//! Zagros Testnet: Force new era via sudo
//!
//! Run with:
//!   SUDO_MNEMONIC="******" \
//!   RPC_URL="ws://217.77.6.126:9948" \
//!   cargo run --release --example zagros_force_new_era

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== ZAGROS FORCE NEW ERA ===\n");

	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9948".to_string());

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected to {}", url);

	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sudo account: {}\n", sudo_keypair.public_key().to_account_id());

	println!("Submitting sudo(staking.forceNewEra())...");

	let force_era_call =
		pezkuwi_subxt::dynamic::tx("Staking", "force_new_era", Vec::<Value>::new());

	let sudo_tx = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![force_era_call.into_value()]);

	let tx_hash = api.tx().sign_and_submit_default(&sudo_tx, &sudo_keypair).await?;
	println!("Submitted! TX hash: 0x{}", hex::encode(tx_hash.as_ref()));

	println!("\nDone. ForceNewEra triggered.");
	Ok(())
}
