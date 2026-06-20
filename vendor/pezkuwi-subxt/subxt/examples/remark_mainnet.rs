use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9944".to_string());
	let message = std::env::var("MESSAGE").expect("MESSAGE env var required");

	println!("RPC: {}", url);
	println!("Message: {}", message);
	println!("Message bytes: {}", message.len());

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected!");

	let mnemonic_str = std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Account: {}\n", keypair.public_key().to_account_id());

	let remark_tx = pezkuwi_subxt::dynamic::tx(
		"System",
		"remark_with_event",
		vec![Value::from_bytes(message.as_bytes())],
	);

	println!("Submitting remarkWithEvent...");

	use pezkuwi_subxt::tx::TxStatus;
	let tx_progress = api.tx().sign_and_submit_then_watch_default(&remark_tx, &keypair).await?;

	println!("TX hash: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

	let mut progress = tx_progress;
	loop {
		let status = progress.next().await;
		match status {
			Some(Ok(TxStatus::Validated)) => println!("  Validated"),
			Some(Ok(TxStatus::Broadcasted)) => println!("  Broadcasted"),
			Some(Ok(TxStatus::InBestBlock(details))) => {
				println!("  InBestBlock {:?}", details.block_hash());
				match details.wait_for_success().await {
					Ok(events) => {
						println!("  SUCCESS!");
						for ev in events.iter().flatten() {
							println!("    {}::{}", ev.pallet_name(), ev.variant_name());
						}
					},
					Err(e) => println!("  Error: {}", e),
				}
				break;
			},
			Some(Ok(TxStatus::Error { message })) => {
				println!("  ERROR: {}", message);
				break;
			},
			Some(Ok(TxStatus::Invalid { message })) => {
				println!("  INVALID: {}", message);
				break;
			},
			Some(Ok(TxStatus::Dropped { message })) => {
				println!("  DROPPED: {}", message);
				break;
			},
			Some(Err(e)) => {
				println!("  Error: {}", e);
				break;
			},
			None => {
				println!("  Stream ended");
				break;
			},
			_ => {},
		}
	}

	println!("\nDone.");
	Ok(())
}
