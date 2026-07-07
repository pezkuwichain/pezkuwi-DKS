//! Zagros Testnet: rotate the Sudo key (one-off, dedicated Zagros sudo key
//! separate from the mainnet founder key it currently reuses).
//!
//! Run with:
//!   SUDO_MNEMONIC="<current sudo phrase>" NEW_PUBKEY="0x..." \
//!   cargo run --release --example zagros_setkey

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9948".to_string());
	println!("RPC: {}", url);

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected!");

	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let old_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Current sudo: {}", old_keypair.public_key().to_account_id());

	let new_pubkey_hex = std::env::var("NEW_PUBKEY").expect("NEW_PUBKEY environment variable required");
	let new_pubkey = hex::decode(new_pubkey_hex.trim_start_matches("0x"))?;
	println!("New sudo pubkey: 0x{}", hex::encode(&new_pubkey));

	// Sudo::set_key is dispatched directly by the current sudo account
	// (not wrapped in sudo(...)) — pallet_sudo checks the caller against
	// the stored Key itself.
	let address = Value::unnamed_variant("Id", vec![Value::from_bytes(&new_pubkey)]);
	let tx = pezkuwi_subxt::dynamic::tx("Sudo", "set_key", vec![address]);

	println!("\nSubmitting...");
	let tx_progress = api.tx().sign_and_submit_then_watch_default(&tx, &old_keypair).await?;
	println!("TX hash: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

	use pezkuwi_subxt::tx::TxStatus;
	let mut progress = tx_progress;
	loop {
		match progress.next().await {
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
