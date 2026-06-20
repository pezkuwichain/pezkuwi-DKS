//! Grant Noter tiki to all 21 validators via XCM Transact batch
//!
//! People Chain has no Sudo, so we send from relay chain:
//!   sudo(xcmPallet.send(Parachain(1004), Transact(
//!     utility.batch_all([tiki.grant_tiki(val, Noter) × 21])
//!   )))
//!
//! Run with:
//!   SUDO_MNEMONIC="..." cargo run --release -p pezkuwi-subxt --example grant_noter_tiki

#![allow(missing_docs, dead_code)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

const PEOPLE_CHAIN_PARA_ID: u32 = 1004;

// People Chain pallet indices
const UTILITY_PALLET: u8 = 40; // 0x28
const TIKI_PALLET: u8 = 61; // 0x3d

// Call indices
const GRANT_TIKI_CALL: u8 = 0;
const BATCH_ALL_CALL: u8 = 2;

// Tiki enum values
const TIKI_NOTER: u8 = 9; // 0x09

fn validators() -> Vec<(&'static str, &'static str)> {
	vec![
		("Çiyager (Cihat Türkan)", "5GipBJs2uNWTCazyZQ2vG3DEqLz4tXNmNZtBAT1Mtm1orZ5i"),
		("Mehmet Tunç", "5HWFZbhkZuTUySXu6ZXYKrTHBnWXHvWRKLozE22zhnwXGGxk"),
		("Nagihan Akarsel", "5CrB5BWJfLNWEZAsAXDKXdJUGzFMXKvYnwRX4DVMcgBwxSdx"),
		("Sait Çürükkaya", "5ELgySrX5ZyK7EWXjj6bAedyTCcTNWDANbiiipsT5gnpoCEp"),
		("Evdile Koçer", "5GCZQNjRdHofEHPvVq4ePrfDYcjRzQ1HQ2awHMX6AawpRYuM"),
		("Mam Zeki", "5H8jTzi4Gm4rbFtXw6h5enhLhgsuhNAqR5K2itmPiz83ymWy"),
		("Kakaî Falah", "5Fs3P5tHuL9cvwPQojsheViRRAjFkMMFa32jAkDSwW9mbTfU"),
		("Feryad Fazil Ömer", "5DXgq7uDXog6zcubT3wgtaYosoibjudz4w5ScPW2phLuAy3V"),
		("Mevlud Afand", "5FyFwbGLgPXun3azh6Gx83wCuUt5FTavb2WAVDYrjziVB9rN"),
		("Şêrko Fatih Şivandî", "5HEcuuypLDeJaSj6ZgH57aXhuviyeLNdw9QrCDJ8u6gsnjnL"),
		("Ramin Hüseyin Penahi", "5EpmpTXbMXpz6ixy3WhutdzcexzPbvybNKv4eiiN1kvTnQH5"),
		("Zanyar Moradi", "5DFsm3BBEgHmSEZkvwGKB7c7tiH2avhfuQE1SEjfMDGuczsW"),
		("Heidar Ghorbani", "5HePVUXjGSM2hVZ1YMz2V3KoX6EdQNEmmzUnUvpfGV95ofUR"),
		("Farhad Salimi", "5GP4nAcwtETTg1oAHQNvevmmhG8GEstGQeCirKEhaDTwpFgx"),
		("Vafa Azarbar", "5FYoCM3oeEGeoFY94EgXBhmABkRCabvPp72ur5bJNG3cK619"),
		("Dr. Aziz Mihemed", "5GspwkKF6aYzFkmAyBBQg7coSCSgDCore79fbW8uxJNAH347"),
		("Arîn Mîrkan", "5GmuX11pN2fC4Fyq1V7MuiYt3aevZcVQs3HZWKyzmap9bKfe"),
		("Ebu Leyla", "5FQptVCtM1qsxkLbQkATkw4Kio4M9LxWvM6TwgEo3QjmTXF3"),
		("Rêvan Kobanê", "5E7VD2qmso1yRfyq3t9u2qhauAgtmjZTybVsCARF5Zz9bXy6"),
		("Amanj Babani", "5Ccz5W7Q21g4UPCytzHxD3VSMLJ1BbbWSkJKFwsNtYRk3HkX"),
		("Xosrow Gulan", "5D7WPmK1SAJyYDdCtgqEzGJpWXQe3Lj9FqWL8z9waLTkUNv3"),
	]
}

/// SCALE compact encoding for small numbers (< 64)
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

/// Encode tiki.grant_tiki(dest: MultiAddress::Id(AccountId32), tiki: Tiki::Noter)
fn encode_grant_noter(account_id: &[u8; 32]) -> Vec<u8> {
	let mut encoded = Vec::with_capacity(36);
	encoded.push(TIKI_PALLET); // 0x3d
	encoded.push(GRANT_TIKI_CALL); // 0x00
	encoded.push(0x00); // MultiAddress::Id variant
	encoded.extend_from_slice(account_id);
	encoded.push(TIKI_NOTER); // 0x09
	encoded
}

/// Encode utility.batch_all(calls)
fn encode_batch_all(calls: Vec<Vec<u8>>) -> Vec<u8> {
	let mut encoded = vec![UTILITY_PALLET, BATCH_ALL_CALL]; // 0x28, 0x02
	encoded.extend(encode_compact(calls.len()));
	for call in calls {
		encoded.extend(call);
	}
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
							("ref_time", Value::u128(50_000_000_000u128)),
							("proof_size", Value::u128(5_000_000u128)),
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
	println!("=== GRANT NOTER TIKI TO 21 VALIDATORS ===\n");

	let relay_url =
		std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9944".to_string());

	let vals = validators();
	println!("Relay RPC: {}", relay_url);
	println!("People Chain Para ID: {}", PEOPLE_CHAIN_PARA_ID);
	println!("Validators: {}", vals.len());
	println!("Tiki: Noter (index {})\n", TIKI_NOTER);

	// Build individual grant_tiki calls
	let mut grant_calls: Vec<Vec<u8>> = Vec::new();
	for (name, ss58) in &vals {
		let account: AccountId32 = ss58.parse()?;
		let call = encode_grant_noter(&account.0);
		println!("  {} → {} bytes", name, call.len());
		grant_calls.push(call);
	}

	// Batch all 21 calls
	let batch_call = encode_batch_all(grant_calls);
	println!("\nBatch call: {} bytes (0x{}...)", batch_call.len(), hex::encode(&batch_call[..8]));

	// Connect to relay chain
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&relay_url).await?;
	println!("Connected! specVersion: {}\n", api.runtime_version().spec_version);

	// Load sudo keypair
	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sudo: {}\n", sudo_keypair.public_key().to_account_id());

	// Build XCM message
	let (dest, message) = build_xcm_values(&batch_call);

	// Wrap: xcmPallet.send → sudo.sudo_unchecked_weight
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

	println!("Submitting sudo(xcm.send(batch_all(grant_tiki × 21)))...");

	use pezkuwi_subxt::tx::TxStatus;
	let tx_progress =
		api.tx().sign_and_submit_then_watch_default(&sudo_call, &sudo_keypair).await?;

	println!("TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

	let mut progress = tx_progress;
	loop {
		let status = progress.next().await;
		match status {
			Some(Ok(TxStatus::InBestBlock(details))) => {
				match details.wait_for_success().await {
					Ok(events) => {
						let mut has_sudid = false;
						let mut has_sent = false;
						for ev in events.iter().flatten() {
							let pallet = ev.pallet_name();
							let variant = ev.variant_name();
							println!("  Event: {}::{}", pallet, variant);
							if pallet == "Sudo" && variant == "Sudid" {
								has_sudid = true;
							}
							if pallet == "XcmPallet" && variant == "Sent" {
								has_sent = true;
							}
						}
						if has_sudid && has_sent {
							println!("\nSUCCESS! XCM batch sent to People Chain.");
							println!("21 validators should now have Noter tiki.");
						} else {
							println!("\nWARNING: Expected Sudo::Sudid + XcmPallet::Sent");
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

	println!("\nVerify on People Chain (port 41944):");
	println!("  - UserTikis[validator] should include Noter");

	Ok(())
}
