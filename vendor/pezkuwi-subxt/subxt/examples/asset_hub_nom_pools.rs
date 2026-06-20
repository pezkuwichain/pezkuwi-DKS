//! Asset Hub: Set NominationPools configs via XCM Transact from relay chain sudo
//!
//! Since Asset Hub has no sudo pallet, we send:
//!   relay: sudo(xcmPallet.send(Parachain(1000), Transact(NominationPools.set_configs(...))))
//!
//! Run with:
//!   SUDO_MNEMONIC="..." RPC_URL="ws://217.77.6.126:9944" \
//!   MIN_JOIN_BOND=10 MIN_CREATE_BOND=10000 \
//!   cargo run --release --example asset_hub_nom_pools

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

// 1 HEZ = 10^12 TYR (planck units)
const PLANCKS_PER_HEZ: u128 = 1_000_000_000_000;

// Asset Hub para ID
const ASSET_HUB_PARA_ID: u32 = 1000;

// NominationPools pallet index on Asset Hub
const NOM_POOLS_PALLET_INDEX: u8 = 81; // 0x51
									   // set_configs call index
const SET_CONFIGS_CALL_INDEX: u8 = 11; // 0x0b

/// SCALE encode ConfigOp::Noop
fn encode_noop() -> Vec<u8> {
	vec![0x00]
}

/// SCALE encode ConfigOp::Set(value) for u128 (Balance)
fn encode_set_u128(value: u128) -> Vec<u8> {
	let mut buf = vec![0x01]; // Set variant
	buf.extend_from_slice(&value.to_le_bytes()); // u128 LE = 16 bytes
	buf
}

/// SCALE encode the NominationPools::set_configs call
fn encode_set_configs_call(min_join_bond: u128, min_create_bond: u128) -> Vec<u8> {
	let mut encoded = Vec::new();

	// Pallet index + Call index
	encoded.push(NOM_POOLS_PALLET_INDEX);
	encoded.push(SET_CONFIGS_CALL_INDEX);

	// min_join_bond: ConfigOp<Balance> = Set(min_join_bond)
	encoded.extend_from_slice(&encode_set_u128(min_join_bond));

	// min_create_bond: ConfigOp<Balance> = Set(min_create_bond)
	encoded.extend_from_slice(&encode_set_u128(min_create_bond));

	// max_pools: ConfigOp<u32> = Noop
	encoded.extend_from_slice(&encode_noop());

	// max_members: ConfigOp<u32> = Noop
	encoded.extend_from_slice(&encode_noop());

	// max_members_per_pool: ConfigOp<u32> = Noop
	encoded.extend_from_slice(&encode_noop());

	// global_max_commission: ConfigOp<Perbill> = Noop
	encoded.extend_from_slice(&encode_noop());

	encoded
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== ASSET HUB: Set NominationPools Configs via XCM ===\n");

	let relay_url =
		std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9944".to_string());

	let min_join_hez: u128 =
		std::env::var("MIN_JOIN_BOND").unwrap_or_else(|_| "10".to_string()).parse()?;
	let min_create_hez: u128 = std::env::var("MIN_CREATE_BOND")
		.unwrap_or_else(|_| "10000".to_string())
		.parse()?;

	let min_join_bond = min_join_hez * PLANCKS_PER_HEZ;
	let min_create_bond = min_create_hez * PLANCKS_PER_HEZ;

	println!("Relay RPC: {}", relay_url);
	println!("Asset Hub Para ID: {}", ASSET_HUB_PARA_ID);
	println!("MinJoinBond: {} HEZ ({} TYR)", min_join_hez, min_join_bond);
	println!("MinCreateBond: {} HEZ ({} TYR)", min_create_hez, min_create_bond);

	// Connect to relay chain
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&relay_url).await?;
	println!("Connected to relay chain!");

	// Load sudo keypair
	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sudo: {}\n", sudo_keypair.public_key().to_account_id());

	// Encode the NominationPools::set_configs call for Asset Hub
	let encoded_call = encode_set_configs_call(min_join_bond, min_create_bond);
	println!("Encoded call: {} bytes (0x{})", encoded_call.len(), hex::encode(&encoded_call));

	// Build XCM destination: V3 MultiLocation { parents: 0, interior: X1(Teyrchain(1000)) }
	let dest = Value::unnamed_variant(
		"V3",
		vec![Value::named_composite([
			("parents", Value::u128(0)),
			(
				"interior",
				Value::unnamed_variant(
					"X1",
					vec![Value::unnamed_variant(
						"Teyrchain",
						vec![Value::u128(ASSET_HUB_PARA_ID as u128)],
					)],
				),
			),
		])],
	);

	// Build XCM V3 message: UnpaidExecution + Transact
	let message = Value::unnamed_variant(
		"V3",
		vec![Value::unnamed_composite(vec![
			Value::named_variant(
				"UnpaidExecution",
				[
					("weight_limit", Value::unnamed_variant("Unlimited", vec![])),
					("check_origin", Value::unnamed_variant("None", vec![])),
				],
			),
			Value::named_variant(
				"Transact",
				[
					("origin_kind", Value::unnamed_variant("Superuser", vec![])),
					(
						"require_weight_at_most",
						Value::named_composite([
							("ref_time", Value::u128(5_000_000_000u128)),
							("proof_size", Value::u128(500_000u128)),
						]),
					),
					("call", Value::from_bytes(&encoded_call)),
				],
			),
		])],
	);

	// Wrap in XcmPallet.send
	let xcm_send = pezkuwi_subxt::dynamic::tx("XcmPallet", "send", vec![dest, message]);

	// Wrap in sudo_unchecked_weight (no weight limit for sudo)
	let sudo_call = pezkuwi_subxt::dynamic::tx(
		"Sudo",
		"sudo_unchecked_weight",
		vec![
			xcm_send.into_value(),
			Value::named_composite([
				("ref_time", Value::u128(1u128)),
				("proof_size", Value::u128(1u128)),
			]),
		],
	);

	println!("Submitting: sudo(xcmPallet.send(Parachain(1000), Transact(NominationPools.set_configs)))...\n");

	// Submit and watch
	use pezkuwi_subxt::tx::TxStatus;

	let tx_progress =
		api.tx().sign_and_submit_then_watch_default(&sudo_call, &sudo_keypair).await?;

	println!("TX hash: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));
	println!("Watching TX status...");

	let mut progress = tx_progress;
	loop {
		let status = progress.next().await;
		match status {
			Some(Ok(TxStatus::Validated)) => println!("  Status: Validated"),
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
				break;
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

	println!("\nDone. XCM Transact sent to Asset Hub.");
	println!("Verify on Asset Hub (port 40944) that MinJoinBond and MinCreateBond are set.");

	Ok(())
}
