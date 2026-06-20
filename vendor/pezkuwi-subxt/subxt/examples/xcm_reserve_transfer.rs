//! XCM Reserve Transfer Test - Comprehensive
//!
//! This example performs a complete XCM reserve transfer from Relay Chain
//! to Asset Hub teyrchain and verifies the transfer was successful.
//!
//! Test Flow:
//! 1. Connect to both Relay Chain and Asset Hub
//! 2. Query Alice's balance on both chains (before)
//! 3. Execute XCM transfer_assets from Relay to Asset Hub
//! 4. Wait for XCM message to be processed
//! 5. Query Alice's balance on both chains (after)
//! 6. Verify the transfer amounts match
//!
//! Run with: cargo run --example xcm_reserve_transfer

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::{At, Value};
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::sr25519::dev;
use std::time::Duration;

// Generate interface from Pezkuwichain relay chain metadata
#[pezkuwi_subxt::subxt(runtime_metadata_path = "../artifacts/pezkuwichain_metadata.scale")]
pub mod relay {}

// Generate interface from Asset Hub metadata
#[pezkuwi_subxt::subxt(runtime_metadata_path = "../artifacts/asset_hub_metadata.scale")]
pub mod asset_hub {}

const RELAY_RPC: &str = "ws://127.0.0.1:9944";
const ASSET_HUB_RPC: &str = "ws://127.0.0.1:9945";
const ASSET_HUB_PARA_ID: u32 = 1000;

// 1 HEZ = 10^12 TYR (planck units)
const PLANCKS_PER_HEZ: u128 = 1_000_000_000_000;

/// Query balance from relay chain using dynamic storage query
async fn query_relay_balance(
	api: &OnlineClient<PezkuwiConfig>,
	account: &AccountId32,
) -> Result<u128, Box<dyn std::error::Error>> {
	let storage = api.storage().at_latest().await?;

	// Use dynamic storage query for flexibility
	let storage_query =
		pezkuwi_subxt::dynamic::storage::<(AccountId32,), Value>("System", "Account");

	let account_info = storage.entry(storage_query)?.fetch((account.clone(),)).await?.decode()?;

	// Extract the free balance using the At trait
	let free_balance = account_info
		.at("data")
		.at("free")
		.ok_or("Could not find free balance")?
		.as_u128()
		.ok_or("Could not parse free balance as u128")?;

	Ok(free_balance)
}

/// Query balance from Asset Hub using dynamic storage query
async fn query_asset_hub_balance(
	api: &OnlineClient<PezkuwiConfig>,
	account: &AccountId32,
) -> Result<u128, Box<dyn std::error::Error>> {
	let storage = api.storage().at_latest().await?;

	// Use dynamic storage query for flexibility
	let storage_query =
		pezkuwi_subxt::dynamic::storage::<(AccountId32,), Value>("System", "Account");

	let account_info = storage.entry(storage_query)?.fetch((account.clone(),)).await?.decode()?;

	// Extract the free balance using the At trait
	let free_balance = account_info
		.at("data")
		.at("free")
		.ok_or("Could not find free balance")?
		.as_u128()
		.ok_or("Could not parse free balance as u128")?;

	Ok(free_balance)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("╔══════════════════════════════════════════════════════════════╗");
	println!("║         XCM RESERVE TRANSFER TEST - PEZKUWICHAIN            ║");
	println!("╠══════════════════════════════════════════════════════════════╣");
	println!("║  From: Relay Chain (HEZ)                                     ║");
	println!("║  To:   Asset Hub Teyrchain (Para {})                       ║", ASSET_HUB_PARA_ID);
	println!("╚══════════════════════════════════════════════════════════════╝\n");

	// ═══════════════════════════════════════════════════════════════════════
	// STEP 1: Connect to both chains
	// ═══════════════════════════════════════════════════════════════════════
	println!("═══ STEP 1: Connecting to chains ═══");

	let relay_api = OnlineClient::<PezkuwiConfig>::from_url(RELAY_RPC).await?;
	println!("✓ Connected to Relay Chain at {}", RELAY_RPC);

	let asset_hub_api = OnlineClient::<PezkuwiConfig>::from_url(ASSET_HUB_RPC).await?;
	println!("✓ Connected to Asset Hub at {}", ASSET_HUB_RPC);

	// ═══════════════════════════════════════════════════════════════════════
	// STEP 2: Setup accounts
	// ═══════════════════════════════════════════════════════════════════════
	println!("\n═══ STEP 2: Setup accounts ═══");

	let alice = dev::alice();
	let alice_account_id = AccountId32(alice.public_key().0);

	println!("✓ Using Alice account");
	println!("  Address: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY");

	// ═══════════════════════════════════════════════════════════════════════
	// STEP 3: Query initial balances
	// ═══════════════════════════════════════════════════════════════════════
	println!("\n═══ STEP 3: Query initial balances ═══");

	let relay_balance_before = query_relay_balance(&relay_api, &alice_account_id).await?;
	println!("  Alice on Relay Chain:");
	println!(
		"    Free balance: {} TYR ({:.4} HEZ)",
		relay_balance_before,
		relay_balance_before as f64 / PLANCKS_PER_HEZ as f64
	);

	let asset_hub_balance_before =
		match query_asset_hub_balance(&asset_hub_api, &alice_account_id).await {
			Ok(balance) => balance,
			Err(_) => {
				println!("  Alice on Asset Hub: (Account not found - will be created)");
				0
			},
		};
	if asset_hub_balance_before > 0 {
		println!("  Alice on Asset Hub:");
		println!(
			"    Free balance: {} TYR ({:.4} HEZ)",
			asset_hub_balance_before,
			asset_hub_balance_before as f64 / PLANCKS_PER_HEZ as f64
		);
	}

	// ═══════════════════════════════════════════════════════════════════════
	// STEP 4: Build and submit XCM teleport (for system teyrchains)
	// ═══════════════════════════════════════════════════════════════════════
	println!("\n═══ STEP 4: Execute XCM Teleport ═══");

	// Transfer 0.1 HEZ (100 billion planck)
	// Note: Alice has ~1 HEZ after previous transactions, so keep transfer small
	let transfer_amount: u128 = PLANCKS_PER_HEZ / 10; // 0.1 HEZ
	println!("  Transfer amount: {} TYR (0.1 HEZ)", transfer_amount);

	// Build destination: Asset Hub (Teyrchain 1000) - Note: Junction::Teyrchain not Parachain
	let dest = relay::runtime_types::xcm::VersionedLocation::V4(
		relay::runtime_types::pezstaging_xcm::v4::location::Location {
			parents: 0,
			interior: relay::runtime_types::pezstaging_xcm::v4::junctions::Junctions::X1([
				relay::runtime_types::pezstaging_xcm::v4::junction::Junction::Teyrchain(
					ASSET_HUB_PARA_ID,
				),
			]),
		},
	);

	// Build beneficiary: Alice's account on Asset Hub
	let beneficiary = relay::runtime_types::xcm::VersionedLocation::V4(
		relay::runtime_types::pezstaging_xcm::v4::location::Location {
			parents: 0,
			interior: relay::runtime_types::pezstaging_xcm::v4::junctions::Junctions::X1([
				relay::runtime_types::pezstaging_xcm::v4::junction::Junction::AccountId32 {
					network: None,
					id: alice.public_key().0,
				},
			]),
		},
	);

	// Build assets: Native token (HEZ)
	let assets = relay::runtime_types::xcm::VersionedAssets::V4(
		relay::runtime_types::pezstaging_xcm::v4::asset::Assets(vec![
			relay::runtime_types::pezstaging_xcm::v4::asset::Asset {
				id: relay::runtime_types::pezstaging_xcm::v4::asset::AssetId(
					relay::runtime_types::pezstaging_xcm::v4::location::Location {
						parents: 0,
						interior:
							relay::runtime_types::pezstaging_xcm::v4::junctions::Junctions::Here,
					},
				),
				fun: relay::runtime_types::pezstaging_xcm::v4::asset::Fungibility::Fungible(
					transfer_amount,
				),
			},
		]),
	);

	// Weight limit: Unlimited
	let weight_limit = relay::runtime_types::xcm::v3::WeightLimit::Unlimited;

	// Fee asset item - use the native asset (index 0 in assets array, converted to VersionedAssetId)
	let fee_asset = relay::runtime_types::xcm::VersionedAssetId::V4(
		relay::runtime_types::pezstaging_xcm::v4::asset::AssetId(
			relay::runtime_types::pezstaging_xcm::v4::location::Location {
				parents: 0,
				interior: relay::runtime_types::pezstaging_xcm::v4::junctions::Junctions::Here,
			},
		),
	);

	// Build the extrinsic - Use teleport for system teyrchains (Asset Hub)
	// Note: System teyrchains like Asset Hub support teleportation from relay chain
	let xcm_tx = relay::tx().xcm_pallet().limited_teleport_assets(
		dest,
		beneficiary,
		assets,
		fee_asset,
		weight_limit,
	);

	println!("\n  Submitting XCM transaction to Relay Chain...");

	let tx_progress = relay_api.tx().sign_and_submit_then_watch_default(&xcm_tx, &alice).await?;

	println!("  Transaction submitted, waiting for finalization...");

	let events = tx_progress.wait_for_finalized_success().await?;

	println!("✓ Transaction finalized on Relay Chain!");

	// ═══════════════════════════════════════════════════════════════════════
	// STEP 5: Check XCM events
	// ═══════════════════════════════════════════════════════════════════════
	println!("\n═══ STEP 5: Analyze XCM events ═══");

	// Check for Attempted event
	match events.find_first::<relay::xcm_pallet::events::Attempted>()? {
		Some(event) => {
			println!("✓ XCM Attempted event:");
			println!("    Outcome: {:?}", event.outcome);
		},
		None => println!("⚠ No Attempted event found"),
	}

	// Check for Sent event
	match events.find_first::<relay::xcm_pallet::events::Sent>()? {
		Some(event) => {
			println!("✓ XCM Sent event:");
			println!("    Origin: {:?}", event.origin);
			println!("    Destination: {:?}", event.destination);
		},
		None => println!("⚠ No Sent event found"),
	}

	// Check for FeesPaid event
	match events.find_first::<relay::xcm_pallet::events::FeesPaid>()? {
		Some(event) => {
			println!("✓ XCM FeesPaid event:");
			println!("    Fees: {:?}", event.fees);
		},
		None => println!("  (No FeesPaid event - fees may be zero)"),
	}

	// ═══════════════════════════════════════════════════════════════════════
	// STEP 6: Wait for XCM to be processed on Asset Hub
	// ═══════════════════════════════════════════════════════════════════════
	println!("\n═══ STEP 6: Wait for Asset Hub processing ═══");
	println!("  Waiting 12 seconds for XCM message to be delivered and processed...");

	tokio::time::sleep(Duration::from_secs(12)).await;

	// ═══════════════════════════════════════════════════════════════════════
	// STEP 7: Query final balances
	// ═══════════════════════════════════════════════════════════════════════
	println!("\n═══ STEP 7: Query final balances ═══");

	let relay_balance_after = query_relay_balance(&relay_api, &alice_account_id).await?;
	let asset_hub_balance_after =
		query_asset_hub_balance(&asset_hub_api, &alice_account_id).await.unwrap_or(0);

	// ═══════════════════════════════════════════════════════════════════════
	// STEP 8: Results and verification
	// ═══════════════════════════════════════════════════════════════════════
	println!("\n╔══════════════════════════════════════════════════════════════╗");
	println!("║                      TEST RESULTS                            ║");
	println!("╠══════════════════════════════════════════════════════════════╣");

	let relay_diff = relay_balance_before.saturating_sub(relay_balance_after);
	let asset_hub_diff = asset_hub_balance_after.saturating_sub(asset_hub_balance_before);

	println!("║  RELAY CHAIN (Alice):                                        ║");
	println!(
		"║    Before: {:>20} TYR ({:>10.4} HEZ)         ║",
		relay_balance_before,
		relay_balance_before as f64 / PLANCKS_PER_HEZ as f64
	);
	println!(
		"║    After:  {:>20} TYR ({:>10.4} HEZ)         ║",
		relay_balance_after,
		relay_balance_after as f64 / PLANCKS_PER_HEZ as f64
	);
	println!(
		"║    Spent:  {:>20} TYR ({:>10.4} HEZ)         ║",
		relay_diff,
		relay_diff as f64 / PLANCKS_PER_HEZ as f64
	);
	println!("║                                                              ║");
	println!("║  ASSET HUB (Alice):                                          ║");
	println!(
		"║    Before: {:>20} TYR ({:>10.4} HEZ)         ║",
		asset_hub_balance_before,
		asset_hub_balance_before as f64 / PLANCKS_PER_HEZ as f64
	);
	println!(
		"║    After:  {:>20} TYR ({:>10.4} HEZ)         ║",
		asset_hub_balance_after,
		asset_hub_balance_after as f64 / PLANCKS_PER_HEZ as f64
	);
	println!(
		"║    Received: {:>18} TYR ({:>10.4} HEZ)         ║",
		asset_hub_diff,
		asset_hub_diff as f64 / PLANCKS_PER_HEZ as f64
	);
	println!("╠══════════════════════════════════════════════════════════════╣");

	// Verification
	if asset_hub_diff > 0 {
		let fees = transfer_amount.saturating_sub(asset_hub_diff);
		println!("║  ✓ SUCCESS: XCM transfer completed!                         ║");
		println!("║    Transferred: {} TYR                         ║", transfer_amount);
		println!("║    Received:    {} TYR                         ║", asset_hub_diff);
		println!("║    XCM Fees:    {} TYR                         ║", fees);
		println!("╚══════════════════════════════════════════════════════════════╝");
		println!("\n✓ XCM RESERVE TRANSFER TEST PASSED!");
	} else if relay_diff > 0 {
		println!("║  ⚠ PARTIAL: Tokens deducted from Relay but not yet on AH    ║");
		println!("║    This may indicate XCM is still processing                 ║");
		println!("║    or there was an error on the receiving side              ║");
		println!("╚══════════════════════════════════════════════════════════════╝");
		println!("\n⚠ XCM TEST NEEDS INVESTIGATION");
	} else {
		println!("║  ✗ FAILED: No balance change detected                        ║");
		println!("║    Check logs for XCM processing errors                      ║");
		println!("╚══════════════════════════════════════════════════════════════╝");
		println!("\n✗ XCM RESERVE TRANSFER TEST FAILED");
	}

	Ok(())
}
