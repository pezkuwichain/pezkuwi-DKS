//! Zagros Testnet: fund the new dedicated Sudo key with HEZ for tx fees.
//!
//! Run with:
//!   SENDER_MNEMONIC="<funded account phrase>" DEST_SS58="5..." AMOUNT_HEZ="1000" \
//!   RPC_URL="ws://217.77.6.126:9948" \
//!   cargo run --release --example zagros_fund_sudo -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const PLANCKS_PER_HEZ: u128 = 1_000_000_000_000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9948".to_string());
	let dest_ss58 = std::env::var("DEST_SS58").expect("DEST_SS58 required");
	let amount_hez: u128 = std::env::var("AMOUNT_HEZ").unwrap_or_else(|_| "1000".to_string()).parse()?;
	let amount_planck = amount_hez * PLANCKS_PER_HEZ;

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected: {}", url);

	let mnemonic_str = std::env::var("SENDER_MNEMONIC").expect("SENDER_MNEMONIC required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sender = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sender: {}", sender.public_key().to_account_id());

	let dest: AccountId32 = dest_ss58.parse()?;
	println!("Dest: {} ({} HEZ)", dest_ss58, amount_hez);

	let transfer_tx = pezkuwi_subxt::dynamic::tx(
		"Balances",
		"transfer_keep_alive",
		vec![Value::unnamed_variant("Id", vec![Value::from_bytes(&dest.0)]), Value::u128(amount_planck)],
	);

	use pezkuwi_subxt::tx::TxStatus;
	let tx_progress = api.tx().sign_and_submit_then_watch_default(&transfer_tx, &sender).await?;
	println!("TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

	let mut progress = tx_progress;
	loop {
		match progress.next().await {
			Some(Ok(TxStatus::InBestBlock(details))) => {
				match details.wait_for_success().await {
					Ok(events) => {
						for ev in events.iter().flatten() {
							println!("  {}::{}", ev.pallet_name(), ev.variant_name());
						}
					},
					Err(e) => println!("DISPATCH ERROR: {}", e),
				}
				break;
			},
			Some(Ok(TxStatus::Error { message })) => {
				println!("TX ERROR: {}", message);
				break;
			},
			Some(Ok(TxStatus::Invalid { message })) => {
				println!("TX INVALID: {}", message);
				break;
			},
			Some(Ok(TxStatus::Dropped { message })) => {
				println!("TX DROPPED: {}", message);
				break;
			},
			Some(Err(e)) => {
				println!("STREAM ERROR: {}", e);
				break;
			},
			None => {
				println!("STREAM ENDED");
				break;
			},
			_ => {},
		}
	}

	println!("Done.");
	Ok(())
}
