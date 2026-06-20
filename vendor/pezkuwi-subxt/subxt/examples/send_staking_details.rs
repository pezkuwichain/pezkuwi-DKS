//! Send receive_staking_details to People Chain for all 21 validators via XCM Transact
//!
//! People Chain StakingScore pallet (index 80) has:
//!   receive_staking_details(who, source, staked_amount, nominations_count, unlocking_chunks_count)
//!
//! source: StakingSource enum (0=RelayChain, 1=AssetHub)
//!
//! This populates CachedStakingDetails on People Chain so validators can
//! call start_score_tracking() and have their staking scores calculated.
//!
//! Run with:
//!   SUDO_MNEMONIC="..." cargo run --release -p pezkuwi-subxt --example send_staking_details

#![allow(missing_docs, dead_code)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const PEOPLE_CHAIN_PARA_ID: u32 = 1004;

// People Chain pallet indices
const STAKING_SCORE_PALLET: u8 = 80; // 0x50
const RECEIVE_STAKING_DETAILS_CALL: u8 = 1;

struct ValidatorInfo {
	name: &'static str,
	ss58: &'static str,
	staked_hez: u64, // in HEZ (will be multiplied by 10^12)
}

fn validators() -> Vec<ValidatorInfo> {
	vec![
		ValidatorInfo {
			name: "Çiyager (Cihat Türkan)",
			ss58: "5GipBJs2uNWTCazyZQ2vG3DEqLz4tXNmNZtBAT1Mtm1orZ5i",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Mehmet Tunç",
			ss58: "5HWFZbhkZuTUySXu6ZXYKrTHBnWXHvWRKLozE22zhnwXGGxk",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Nagihan Akarsel",
			ss58: "5CrB5BWJfLNWEZAsAXDKXdJUGzFMXKvYnwRX4DVMcgBwxSdx",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Sait Çürükkaya (Doktor Süleyman)",
			ss58: "5ELgySrX5ZyK7EWXjj6bAedyTCcTNWDANbiiipsT5gnpoCEp",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Evdile Koçer",
			ss58: "5GCZQNjRdHofEHPvVq4ePrfDYcjRzQ1HQ2awHMX6AawpRYuM",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Mam Zeki",
			ss58: "5H8jTzi4Gm4rbFtXw6h5enhLhgsuhNAqR5K2itmPiz83ymWy",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Kakaî Falah",
			ss58: "5Fs3P5tHuL9cvwPQojsheViRRAjFkMMFa32jAkDSwW9mbTfU",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Feryad Fazil Ömer",
			ss58: "5DXgq7uDXog6zcubT3wgtaYosoibjudz4w5ScPW2phLuAy3V",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Mevlud Afand",
			ss58: "5FyFwbGLgPXun3azh6Gx83wCuUt5FTavb2WAVDYrjziVB9rN",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Şêrko Fatih Şivandî",
			ss58: "5HEcuuypLDeJaSj6ZgH57aXhuviyeLNdw9QrCDJ8u6gsnjnL",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Ramin Hüseyin Penahi",
			ss58: "5EpmpTXbMXpz6ixy3WhutdzcexzPbvybNKv4eiiN1kvTnQH5",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Zanyar Moradi",
			ss58: "5DFsm3BBEgHmSEZkvwGKB7c7tiH2avhfuQE1SEjfMDGuczsW",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Heidar Ghorbani",
			ss58: "5HePVUXjGSM2hVZ1YMz2V3KoX6EdQNEmmzUnUvpfGV95ofUR",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Farhad Salimi",
			ss58: "5GP4nAcwtETTg1oAHQNvevmmhG8GEstGQeCirKEhaDTwpFgx",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Vafa Azarbar",
			ss58: "5FYoCM3oeEGeoFY94EgXBhmABkRCabvPp72ur5bJNG3cK619",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Dr. Aziz Mihemed",
			ss58: "5GspwkKF6aYzFkmAyBBQg7coSCSgDCore79fbW8uxJNAH347",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Arîn Mîrkan",
			ss58: "5GmuX11pN2fC4Fyq1V7MuiYt3aevZcVQs3HZWKyzmap9bKfe",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Ebu Leyla",
			ss58: "5FQptVCtM1qsxkLbQkATkw4Kio4M9LxWvM6TwgEo3QjmTXF3",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Rêvan Kobanê",
			ss58: "5E7VD2qmso1yRfyq3t9u2qhauAgtmjZTybVsCARF5Zz9bXy6",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Amanj Babani",
			ss58: "5Ccz5W7Q21g4UPCytzHxD3VSMLJ1BbbWSkJKFwsNtYRk3HkX",
			staked_hez: 499_100,
		},
		ValidatorInfo {
			name: "Xosrow Gulan",
			ss58: "5D7WPmK1SAJyYDdCtgqEzGJpWXQe3Lj9FqWL8z9waLTkUNv3",
			staked_hez: 499_100,
		},
	]
}

const PLANCK_PER_HEZ: u128 = 1_000_000_000_000;

/// StakingSource enum (SCALE-encoded as single byte)
const STAKING_SOURCE_RELAY_CHAIN: u8 = 0;
const STAKING_SOURCE_ASSET_HUB: u8 = 1;

/// Encode StakingScore.receive_staking_details(who, source, staked_amount, nominations_count, unlocking_chunks_count)
/// Pallet 80 (0x50), call_index 1
/// who: AccountId32 (32 bytes raw)
/// source: StakingSource (1 byte enum: 0=RelayChain, 1=AssetHub)
/// staked_amount: u128 LE (16 bytes)  - this is T::Balance which is u128
/// nominations_count: u32 LE (4 bytes)
/// unlocking_chunks_count: u32 LE (4 bytes)
fn encode_receive_staking_details(
	account_id: &[u8; 32],
	source: u8,
	staked_amount: u128,
	nominations_count: u32,
	unlocking_chunks_count: u32,
) -> Vec<u8> {
	let mut encoded = Vec::with_capacity(59);
	encoded.push(STAKING_SCORE_PALLET); // 0x50
	encoded.push(RECEIVE_STAKING_DETAILS_CALL); // 0x01
	encoded.extend_from_slice(account_id); // 32 bytes
	encoded.push(source); // 1 byte (StakingSource enum)
	encoded.extend_from_slice(&staked_amount.to_le_bytes()); // 16 bytes
	encoded.extend_from_slice(&nominations_count.to_le_bytes()); // 4 bytes
	encoded.extend_from_slice(&unlocking_chunks_count.to_le_bytes()); // 4 bytes
	encoded
}

/// Build XCM V3 message: UnpaidExecution + Transact
fn build_xcm_values(encoded_call: &[u8]) -> (Value, Value) {
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
						vec![Value::u128(PEOPLE_CHAIN_PARA_ID as u128)],
					)],
				),
			),
		])],
	);

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
							("ref_time", Value::u128(10_000_000_000u128)),
							("proof_size", Value::u128(1_000_000u128)),
						]),
					),
					("call", Value::from_bytes(encoded_call)),
				],
			),
		])],
	);

	(dest, message)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== SEND STAKING DETAILS TO PEOPLE CHAIN ===\n");

	let relay_url =
		std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9944".to_string());
	let skip: usize =
		std::env::var("SKIP").unwrap_or_else(|_| "0".to_string()).parse().unwrap_or(0);

	let vals = validators();
	println!("Relay RPC: {}", relay_url);
	println!("People Chain Para ID: {}", PEOPLE_CHAIN_PARA_ID);
	println!("Validators: {} (skip {})\n", vals.len(), skip);

	// Connect to relay chain
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&relay_url).await?;
	println!("Connected! specVersion: {}\n", api.runtime_version().spec_version);

	// Load sudo keypair
	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sudo: {}\n", sudo_keypair.public_key().to_account_id());

	let mut success_count = 0;
	let mut fail_count = 0;

	for (i, v) in vals.iter().enumerate().skip(skip) {
		let staked_planck = v.staked_hez as u128 * PLANCK_PER_HEZ;
		println!("--- [{}/{}] {} ---", i + 1, vals.len(), v.name);
		println!("  Address: {}", v.ss58);
		println!("  Staked: {} HEZ ({} planck)", v.staked_hez, staked_planck);

		let account: AccountId32 = match v.ss58.parse() {
			Ok(a) => a,
			Err(e) => {
				println!("  ERROR: Invalid SS58: {}", e);
				fail_count += 1;
				continue;
			},
		};

		// Encode receive_staking_details call
		// source = RelayChain (validators stake on Relay Chain)
		// nominations_count = 0 (validators don't nominate, they validate)
		// unlocking_chunks_count = 0 (no pending unstakes)
		let call = encode_receive_staking_details(
			&account.0,
			STAKING_SOURCE_RELAY_CHAIN,
			staked_planck,
			0,
			0,
		);
		println!("  Call: {} bytes (0x{}...)", call.len(), hex::encode(&call[..6]));

		// Build XCM message
		let (dest, message) = build_xcm_values(&call);

		// Wrap: xcmPallet.send(dest, message) → sudo.sudo_unchecked_weight(...)
		let xcm_send = pezkuwi_subxt::dynamic::tx("XcmPallet", "send", vec![dest, message]);

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

		// Submit with retries
		use pezkuwi_subxt::tx::TxStatus;
		let mut tx_progress_opt = None;
		for attempt in 0..3 {
			match api.tx().sign_and_submit_then_watch_default(&sudo_call, &sudo_keypair).await {
				Ok(p) => {
					tx_progress_opt = Some(p);
					break;
				},
				Err(e) => {
					println!("  SUBMIT ERROR (attempt {}): {}", attempt + 1, e);
					tokio::time::sleep(std::time::Duration::from_secs(12)).await;
				},
			}
		}

		let tx_progress = match tx_progress_opt {
			Some(p) => p,
			None => {
				println!("  FAILED after 3 attempts");
				fail_count += 1;
				continue;
			},
		};

		println!("  TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

		let mut progress = tx_progress;
		let mut tx_ok = false;
		loop {
			let status = progress.next().await;
			match status {
				Some(Ok(TxStatus::InBestBlock(details))) => {
					match details.wait_for_success().await {
						Ok(events) => {
							let mut has_sudid = false;
							let mut has_sent = false;
							for ev in events.iter().flatten() {
								if ev.pallet_name() == "Sudo" && ev.variant_name() == "Sudid" {
									has_sudid = true;
								}
								if ev.pallet_name() == "XcmPallet" && ev.variant_name() == "Sent" {
									has_sent = true;
								}
							}
							if has_sudid && has_sent {
								println!("  SUCCESS (Sudo::Sudid + XcmPallet::Sent)");
								tx_ok = true;
							} else {
								println!("  WARNING: Events:");
								for ev in events.iter().flatten() {
									println!("    {}::{}", ev.pallet_name(), ev.variant_name());
								}
							}
						},
						Err(e) => println!("  DISPATCH ERROR: {}", e),
					}
					break;
				},
				Some(Ok(TxStatus::Error { message })) => {
					println!("  TX ERROR: {}", message);
					break;
				},
				Some(Ok(TxStatus::Invalid { message })) => {
					println!("  TX INVALID: {}", message);
					break;
				},
				Some(Ok(TxStatus::Dropped { message })) => {
					println!("  TX DROPPED: {}", message);
					break;
				},
				Some(Err(e)) => {
					println!("  STREAM ERROR: {}", e);
					break;
				},
				None => {
					println!("  STREAM ENDED");
					break;
				},
				_ => {},
			}
		}

		if tx_ok {
			success_count += 1;
		} else {
			fail_count += 1;
		}

		// Wait between XCM sends
		if i + 1 < vals.len() {
			tokio::time::sleep(std::time::Duration::from_secs(12)).await;
		}
	}

	println!("\n=== RESULTS ===");
	println!("Success: {}/{}", success_count, vals.len());
	println!("Failed:  {}/{}", fail_count, vals.len());
	println!("\nVerify on People Chain (port 41944):");
	println!("  - CachedStakingDetails[validator] should have staked_amount set");
	println!("  - Validators can now call start_score_tracking()");

	Ok(())
}
