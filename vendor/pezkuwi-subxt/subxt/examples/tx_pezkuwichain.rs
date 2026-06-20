//! Pezkuwichain Token Transfer Test
//!
//! This example transfers HEZ from Alice to Bob on a local Pezkuwichain node.
//!
//! Run with: cargo run --example tx_pezkuwichain

#![allow(missing_docs)]
use pezkuwi_subxt::config::pezkuwi::{AccountId32, MultiAddress};
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::sr25519::dev;

// Generate interface from Pezkuwichain metadata
#[pezkuwi_subxt::subxt(runtime_metadata_path = "../artifacts/pezkuwichain_metadata.scale")]
pub mod pezkuwichain {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== PEZKUWICHAIN TOKEN TRANSFER TEST ===\n");

	// Connect to local node (default port 9944)
	let api = OnlineClient::<PezkuwiConfig>::from_url("ws://127.0.0.1:9944").await?;
	println!("✓ Connected to Pezkuwichain node");

	// Setup accounts
	let alice = dev::alice();
	let bob = dev::bob();

	println!("\n--- Accounts ---");
	println!("Alice: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY");
	println!("Bob:   5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty");

	// Transfer amount: 1 HEZ = 1_000_000_000_000 TYR (10^12)
	let transfer_amount: u128 = 1_000_000_000_000; // 1 HEZ

	println!("\n--- Executing Transfer ---");
	println!("From:   Alice");
	println!("To:     Bob");
	println!("Amount: {} TYR (1 HEZ)", transfer_amount);

	// Build transfer extrinsic using utility types
	let bob_account = AccountId32(bob.public_key().0);
	let dest = MultiAddress::Id(bob_account);

	let transfer_tx = pezkuwichain::tx().balances().transfer_allow_death(dest, transfer_amount);

	// Sign and submit
	println!("\nSubmitting transaction...");
	let events = api
		.tx()
		.sign_and_submit_then_watch_default(&transfer_tx, &alice)
		.await?
		.wait_for_finalized_success()
		.await?;

	println!("✓ Transaction finalized!");

	// Find Transfer event
	let transfer_event = events.find_first::<pezkuwichain::balances::events::Transfer>()?;
	if let Some(event) = transfer_event {
		println!("\n✓ Transfer event:");
		println!("  From:   {:?}", event.from);
		println!("  To:     {:?}", event.to);
		println!("  Amount: {} TYR", event.amount);
	}

	println!("\n=== TEST COMPLETED SUCCESSFULLY ===");
	Ok(())
}
