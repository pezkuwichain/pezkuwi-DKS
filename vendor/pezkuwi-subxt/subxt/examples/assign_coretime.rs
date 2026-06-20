//! Assign coretime cores to parachains on local simulation
//!
//! Steps:
//! 1. Set core count to 2
//! 2. Assign core 0 to AH (para 1000) - full core
//! 3. Assign core 1 to People (para 1004) - full core
//!
//! Run:
//!   SUDO_MNEMONIC="..." cargo run --release -p pezkuwi-subxt --example assign_coretime

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== ASSIGN CORETIME ===\n");

	let rc_url = std::env::var("RC_RPC").unwrap_or_else(|_| "ws://127.0.0.1:9944".to_string());
	let api = OnlineClient::<PezkuwiConfig>::from_url(&rc_url).await?;
	println!("Connected to RC: {}", rc_url);

	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sudo: {}", sudo_keypair.public_key().to_account_id());

	// Get current block number via RPC
	let block = api.blocks().at_latest().await?;
	let current_block = block.number();
	println!("Current block: {}\n", current_block);

	// Step 1: Set core count to 2
	// Coretime.request_core_count(count: u16) - call_index 1, pallet 74
	println!("Step 1: Setting core count to 2...");
	let set_cores =
		pezkuwi_subxt::dynamic::tx("Coretime", "request_core_count", vec![Value::u128(2)]);
	let sudo_tx = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![set_cores.into_value()]);

	let progress = api.tx().sign_and_submit_then_watch_default(&sudo_tx, &sudo_keypair).await?;
	let events = progress.wait_for_finalized_success().await?;
	for event in events.iter() {
		let event = event?;
		if event.pallet_name() == "Sudo" {
			println!("  {}::{}", event.pallet_name(), event.variant_name());
		}
	}

	// Step 2: Assign core 0 to AH (para 1000)
	// Coretime.assign_core(core: u16, begin: BlockNumber, assignment: Vec<(CoreAssignment, u16)>,
	// end_hint: Option<BlockNumber>) CoreAssignment: Idle=0, Pool=1, Task(ParaId)=2
	// PartsOf57600: 57600 = full core
	println!("\nStep 2: Assigning core 0 to AH (para 1000)...");
	let begin = Value::u128(current_block as u128 + 1);

	let assignment_ah = Value::unnamed_composite(vec![Value::unnamed_composite(vec![
		// CoreAssignment::Task(1000)
		Value::unnamed_variant("Task", vec![Value::u128(1000)]),
		// PartsOf57600 = 57600 (full core)
		Value::u128(57600),
	])]);

	let assign_ah = pezkuwi_subxt::dynamic::tx(
		"Coretime",
		"assign_core",
		vec![
			Value::u128(0),                         // core index
			begin.clone(),                          // begin
			assignment_ah,                          // assignment
			Value::unnamed_variant("None", vec![]), // end_hint
		],
	);
	let sudo_ah = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![assign_ah.into_value()]);

	let progress = api.tx().sign_and_submit_then_watch_default(&sudo_ah, &sudo_keypair).await?;
	let events = progress.wait_for_finalized_success().await?;
	for event in events.iter() {
		let event = event?;
		if event.pallet_name() == "Sudo" || event.pallet_name() == "Coretime" {
			println!("  {}::{}", event.pallet_name(), event.variant_name());
		}
	}

	// Step 3: Assign core 1 to People (para 1004)
	println!("\nStep 3: Assigning core 1 to People (para 1004)...");
	let assignment_people = Value::unnamed_composite(vec![Value::unnamed_composite(vec![
		Value::unnamed_variant("Task", vec![Value::u128(1004)]),
		Value::u128(57600),
	])]);

	let assign_people = pezkuwi_subxt::dynamic::tx(
		"Coretime",
		"assign_core",
		vec![
			Value::u128(1),                         // core index
			begin,                                  // begin
			assignment_people,                      // assignment
			Value::unnamed_variant("None", vec![]), // end_hint
		],
	);
	let sudo_people = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![assign_people.into_value()]);

	let progress = api.tx().sign_and_submit_then_watch_default(&sudo_people, &sudo_keypair).await?;
	let events = progress.wait_for_finalized_success().await?;
	for event in events.iter() {
		let event = event?;
		if event.pallet_name() == "Sudo" || event.pallet_name() == "Coretime" {
			println!("  {}::{}", event.pallet_name(), event.variant_name());
		}
	}

	println!("\nDone! Core count set to 2, cores assigned.");
	println!("Wait 2 sessions (~20 blocks) for core_count change to take effect.");
	println!("Parachains should start producing backed blocks after that.");

	Ok(())
}
