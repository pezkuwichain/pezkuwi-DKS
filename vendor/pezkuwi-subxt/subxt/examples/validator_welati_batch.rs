//! Make 21 validators Welati citizens via XCM Transact batch
//!
//! People Chain has no Sudo, so we send from relay chain:
//!   sudo(xcmPallet.send(Parachain(1004), Transact(
//!     utility.batch_all([
//!       system.set_storage([KycStatuses, CitizenReferrers, IdentityHashes]),
//!       tiki.force_mint_citizen_nft(validator)
//!     ])
//!   )))
//!
//! This sets all IdentityKyc storage AND mints Welati NFT in a single atomic batch.
//!
//! Run with:
//!   SUDO_MNEMONIC="..." cargo run --release -p pezkuwi-subxt --example validator_welati_batch
//!   SUDO_MNEMONIC="..." SKIP=5 cargo run --release -p pezkuwi-subxt --example validator_welati_batch

#![allow(missing_docs, dead_code)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const PEOPLE_CHAIN_PARA_ID: u32 = 1004;

// Founder account (referrer for all validators)
const FOUNDER_SS58: &str = "5CyuFfbF95rzBxru7c9yEsX4XmQXUxpLUcbj9RLg9K1cGiiF";

// People Chain pallet indices
const SYSTEM_PALLET: u8 = 0;
const UTILITY_PALLET: u8 = 40; // 0x28
const TIKI_PALLET: u8 = 61; // 0x3d

// Call indices
const SET_STORAGE_CALL: u8 = 4;
const BATCH_ALL_CALL: u8 = 2;
const FORCE_MINT_CALL: u8 = 2;

struct ValidatorInfo {
	name: &'static str,
	ss58: &'static str,
}

fn validators() -> Vec<ValidatorInfo> {
	vec![
		ValidatorInfo {
			name: "Çiyager (Cihat Türkan)",
			ss58: "5GipBJs2uNWTCazyZQ2vG3DEqLz4tXNmNZtBAT1Mtm1orZ5i",
		},
		ValidatorInfo {
			name: "Mehmet Tunç",
			ss58: "5HWFZbhkZuTUySXu6ZXYKrTHBnWXHvWRKLozE22zhnwXGGxk",
		},
		ValidatorInfo {
			name: "Nagihan Akarsel",
			ss58: "5CrB5BWJfLNWEZAsAXDKXdJUGzFMXKvYnwRX4DVMcgBwxSdx",
		},
		ValidatorInfo {
			name: "Sait Çürükkaya (Doktor Süleyman)",
			ss58: "5ELgySrX5ZyK7EWXjj6bAedyTCcTNWDANbiiipsT5gnpoCEp",
		},
		ValidatorInfo {
			name: "Evdile Koçer",
			ss58: "5GCZQNjRdHofEHPvVq4ePrfDYcjRzQ1HQ2awHMX6AawpRYuM",
		},
		ValidatorInfo {
			name: "Mam Zeki",
			ss58: "5H8jTzi4Gm4rbFtXw6h5enhLhgsuhNAqR5K2itmPiz83ymWy",
		},
		ValidatorInfo {
			name: "Kakaî Falah",
			ss58: "5Fs3P5tHuL9cvwPQojsheViRRAjFkMMFa32jAkDSwW9mbTfU",
		},
		ValidatorInfo {
			name: "Feryad Fazil Ömer",
			ss58: "5DXgq7uDXog6zcubT3wgtaYosoibjudz4w5ScPW2phLuAy3V",
		},
		ValidatorInfo {
			name: "Mevlud Afand",
			ss58: "5FyFwbGLgPXun3azh6Gx83wCuUt5FTavb2WAVDYrjziVB9rN",
		},
		ValidatorInfo {
			name: "Şêrko Fatih Şivandî",
			ss58: "5HEcuuypLDeJaSj6ZgH57aXhuviyeLNdw9QrCDJ8u6gsnjnL",
		},
		ValidatorInfo {
			name: "Ramin Hüseyin Penahi",
			ss58: "5EpmpTXbMXpz6ixy3WhutdzcexzPbvybNKv4eiiN1kvTnQH5",
		},
		ValidatorInfo {
			name: "Zanyar Moradi",
			ss58: "5DFsm3BBEgHmSEZkvwGKB7c7tiH2avhfuQE1SEjfMDGuczsW",
		},
		ValidatorInfo {
			name: "Heidar Ghorbani",
			ss58: "5HePVUXjGSM2hVZ1YMz2V3KoX6EdQNEmmzUnUvpfGV95ofUR",
		},
		ValidatorInfo {
			name: "Farhad Salimi",
			ss58: "5GP4nAcwtETTg1oAHQNvevmmhG8GEstGQeCirKEhaDTwpFgx",
		},
		ValidatorInfo {
			name: "Vafa Azarbar",
			ss58: "5FYoCM3oeEGeoFY94EgXBhmABkRCabvPp72ur5bJNG3cK619",
		},
		ValidatorInfo {
			name: "Dr. Aziz Mihemed",
			ss58: "5GspwkKF6aYzFkmAyBBQg7coSCSgDCore79fbW8uxJNAH347",
		},
		ValidatorInfo {
			name: "Arîn Mîrkan",
			ss58: "5GmuX11pN2fC4Fyq1V7MuiYt3aevZcVQs3HZWKyzmap9bKfe",
		},
		ValidatorInfo {
			name: "Ebu Leyla",
			ss58: "5FQptVCtM1qsxkLbQkATkw4Kio4M9LxWvM6TwgEo3QjmTXF3",
		},
		ValidatorInfo {
			name: "Rêvan Kobanê",
			ss58: "5E7VD2qmso1yRfyq3t9u2qhauAgtmjZTybVsCARF5Zz9bXy6",
		},
		ValidatorInfo {
			name: "Amanj Babani",
			ss58: "5Ccz5W7Q21g4UPCytzHxD3VSMLJ1BbbWSkJKFwsNtYRk3HkX",
		},
		ValidatorInfo {
			name: "Xosrow Gulan",
			ss58: "5D7WPmK1SAJyYDdCtgqEzGJpWXQe3Lj9FqWL8z9waLTkUNv3",
		},
	]
}

// ====== SCALE & Storage Key Helpers ======

/// Compute StorageMap key with Blake2_128Concat hasher
/// key = twox128(pallet) + twox128(storage) + blake2_128(map_key) + map_key
fn storage_map_key(pallet: &str, storage: &str, map_key: &[u8]) -> Vec<u8> {
	let mut key = Vec::with_capacity(16 + 16 + 16 + map_key.len());
	key.extend_from_slice(&pezsp_crypto_hashing::twox_128(pallet.as_bytes()));
	key.extend_from_slice(&pezsp_crypto_hashing::twox_128(storage.as_bytes()));
	key.extend_from_slice(&pezsp_crypto_hashing::blake2_128(map_key));
	key.extend_from_slice(map_key);
	key
}

/// SCALE compact encoding for small numbers (< 16384)
fn encode_compact(value: usize) -> Vec<u8> {
	if value < 64 {
		vec![(value as u8) << 2]
	} else if value < 16384 {
		let v = ((value as u16) << 2) | 0x01;
		v.to_le_bytes().to_vec()
	} else {
		panic!("Value too large for compact encoding: {}", value);
	}
}

/// Encode system.set_storage(items: Vec<(Vec<u8>, Vec<u8>)>)
fn encode_set_storage(items: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
	let mut encoded = vec![SYSTEM_PALLET, SET_STORAGE_CALL]; // 0x00, 0x04
	encoded.extend(encode_compact(items.len()));
	for (key, value) in items {
		encoded.extend(encode_compact(key.len()));
		encoded.extend(key);
		encoded.extend(encode_compact(value.len()));
		encoded.extend(value);
	}
	encoded
}

/// Encode tiki.force_mint_citizen_nft(dest: MultiAddress::Id(AccountId32))
fn encode_force_mint(account_id: &[u8; 32]) -> Vec<u8> {
	let mut encoded = Vec::with_capacity(35);
	encoded.push(TIKI_PALLET); // 0x3d
	encoded.push(FORCE_MINT_CALL); // 0x02
	encoded.push(0x00); // MultiAddress::Id variant
	encoded.extend_from_slice(account_id);
	encoded
}

/// Encode utility.batch_all(calls: Vec<RuntimeCall>)
fn encode_batch_all(calls: Vec<Vec<u8>>) -> Vec<u8> {
	let mut encoded = vec![UTILITY_PALLET, BATCH_ALL_CALL]; // 0x28, 0x02
	encoded.extend(encode_compact(calls.len()));
	for call in calls {
		encoded.extend(call);
	}
	encoded
}

const REFERRAL_PALLET: u8 = 52; // 0x34
const FORCE_CONFIRM_REFERRAL_CALL: u8 = 1;

/// Encode Referral.force_confirm_referral(referrer, referred)
/// call_index=1, both params are AccountId32 (raw 32 bytes, no MultiAddress)
fn encode_force_confirm_referral(referrer_id: &[u8; 32], referred_id: &[u8; 32]) -> Vec<u8> {
	let mut encoded = Vec::with_capacity(66);
	encoded.push(REFERRAL_PALLET); // 0x34
	encoded.push(FORCE_CONFIRM_REFERRAL_CALL); // 0x01
	encoded.extend_from_slice(referrer_id);
	encoded.extend_from_slice(referred_id);
	encoded
}

/// Build encoded call: Referral.force_confirm_referral(founder, validator)
fn build_validator_batch_call(
	validator_id: &[u8; 32],
	founder_id: &[u8; 32],
	_name: &str,
) -> Vec<u8> {
	// KycStatuses, CitizenReferrers, IdentityHashes already written via set_storage.
	// NFTs already minted via mint_welati_tiki.rs.
	// Now just confirm referral to update ReferralCount, Referrals, ReferrerStats.
	encode_force_confirm_referral(founder_id, validator_id)
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
	println!("=== VALIDATOR WELATI BATCH (XCM Transact) ===\n");

	let relay_url =
		std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9944".to_string());
	let skip: usize =
		std::env::var("SKIP").unwrap_or_else(|_| "0".to_string()).parse().unwrap_or(0);

	let vals = validators();
	println!("Relay RPC: {}", relay_url);
	println!("People Chain Para ID: {}", PEOPLE_CHAIN_PARA_ID);
	println!("Validators: {} (skip {})\n", vals.len(), skip);

	// Parse founder account
	let founder_account: AccountId32 = FOUNDER_SS58.parse()?;
	println!("Founder (referrer): {}\n", FOUNDER_SS58);

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
		println!("--- [{}/{}] {} ---", i + 1, vals.len(), v.name);
		println!("  Address: {}", v.ss58);

		let account: AccountId32 = match v.ss58.parse() {
			Ok(a) => a,
			Err(e) => {
				println!("  ERROR: Invalid SS58: {}", e);
				fail_count += 1;
				continue;
			},
		};

		// Build the batch call (setStorage + force_mint)
		let batch_call = build_validator_batch_call(&account.0, &founder_account.0, v.name);
		println!(
			"  Batch call: {} bytes (0x{}...)",
			batch_call.len(),
			hex::encode(&batch_call[..6])
		);

		// Build XCM message
		let (dest, message) = build_xcm_values(&batch_call);

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
	println!("  - KycStatuses[validator] = Approved");
	println!("  - CitizenReferrers[validator] = founder");
	println!("  - IdentityHashes[validator] = blake2_256(name)");
	println!("  - CitizenNft[validator] exists (Welati NFT minted)");

	Ok(())
}
