//! Mint Welati Tiki (citizenship NFT) for validators via XCM Transact
//!
//! People Chain has no sudo, so we send from relay chain:
//!   sudo(xcmPallet.send(Parachain(1004), Transact(Tiki.force_mint_citizen_nft(dest))))
//!
//! Run with:
//!   SUDO_MNEMONIC="..." RPC_URL="ws://217.77.6.126:9944" \
//!   cargo run --release --example mint_welati_tiki

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::bip39::Mnemonic;
use pezkuwi_subxt_signer::sr25519::Keypair;
use std::str::FromStr;

// People Chain para ID
const PEOPLE_CHAIN_PARA_ID: u32 = 1004;

// Tiki pallet index on People Chain
const TIKI_PALLET_INDEX: u8 = 61; // 0x3d
								  // force_mint_citizen_nft call index
const FORCE_MINT_CALL_INDEX: u8 = 2; // 0x02

/// Encode Tiki::force_mint_citizen_nft(dest) for People Chain
/// dest is MultiAddress::Id(AccountId32) = 0x00 + 32 bytes
fn encode_force_mint_call(account_id: &[u8; 32]) -> Vec<u8> {
	let mut encoded = Vec::with_capacity(35);
	encoded.push(TIKI_PALLET_INDEX); // 0x3d
	encoded.push(FORCE_MINT_CALL_INDEX); // 0x02
	encoded.push(0x00); // MultiAddress::Id variant
	encoded.extend_from_slice(account_id);
	encoded
}

/// Build XCM Transact message wrapped in sudo for relay chain
fn build_xcm_sudo_transact(encoded_call: &[u8]) -> (Value, Value) {
	// Destination: People Chain
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

	// XCM message: UnpaidExecution + Transact
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
					("call", Value::from_bytes(encoded_call)),
				],
			),
		])],
	);

	(dest, message)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("=== MINT WELATI TIKI FOR VALIDATORS ===\n");

	// Validator name → SS58 address mapping
	let validators: Vec<(&str, &str)> = vec![
		("Çiyager (Cihat Türkan)", "5GipBJs2uNWTCazyZQ2vG3DEqLz4tXNmNZtBAT1Mtm1orZ5i"),
		("Mehmet Tunç", "5HWFZbhkZuTUySXu6ZXYKrTHBnWXHvWRKLozE22zhnwXGGxk"),
		("Nagihan Akarsel", "5CrB5BWJfLNWEZAsAXDKXdJUGzFMXKvYnwRX4DVMcgBwxSdx"),
		("Sait Çürükkaya (Doktor Süleyman)", "5ELgySrX5ZyK7EWXjj6bAedyTCcTNWDANbiiipsT5gnpoCEp"),
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
	];

	let relay_url =
		std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9944".to_string());

	// Skip first N validators (already minted)
	let skip: usize =
		std::env::var("SKIP").unwrap_or_else(|_| "0".to_string()).parse().unwrap_or(0);

	println!("Relay RPC: {}", relay_url);
	println!("People Chain Para ID: {}", PEOPLE_CHAIN_PARA_ID);
	println!("Validators to mint: {} (skipping first {})\n", validators.len() - skip, skip);

	// Connect
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&relay_url).await?;
	println!("Connected to relay chain!");

	// Load sudo keypair
	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let sudo_keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sudo: {}\n", sudo_keypair.public_key().to_account_id());

	let mut success_count = 0;
	let mut fail_count = 0;

	for (i, (name, ss58)) in validators.iter().enumerate().skip(skip) {
		println!("--- [{}/{}] {} ---", i + 1, validators.len(), name);
		println!("  Address: {}", ss58);

		// Parse SS58 to AccountId32
		let account: AccountId32 = match ss58.parse() {
			Ok(a) => a,
			Err(e) => {
				println!("  ERROR: Invalid SS58 address: {}", e);
				fail_count += 1;
				continue;
			},
		};

		// Encode the call
		let encoded_call = encode_force_mint_call(&account.0);
		println!(
			"  Encoded call: 0x{}",
			hex::encode(&encoded_call[..4]) // just show prefix
		);

		// Build XCM message
		let (dest, message) = build_xcm_sudo_transact(&encoded_call);

		// Wrap in xcmPallet.send then sudo
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

		// Submit and watch
		use pezkuwi_subxt::tx::TxStatus;

		// Retry up to 3 times on submit error
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
								println!("  WARNING: Missing expected events");
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

		// Wait for block inclusion before sending next TX (block time = 6s)
		if i + 1 < validators.len() {
			tokio::time::sleep(std::time::Duration::from_secs(12)).await;
		}
	}

	println!("\n=== RESULTS ===");
	println!("Success: {}/{}", success_count, validators.len());
	println!("Failed:  {}/{}", fail_count, validators.len());

	if fail_count > 0 {
		println!("\nSome mints failed. Check People Chain events to verify.");
	} else {
		println!("\nAll Welati Tiki NFTs minted successfully!");
	}

	println!("Verify on People Chain (port 41944) that all validators have citizenship NFTs.");

	Ok(())
}
