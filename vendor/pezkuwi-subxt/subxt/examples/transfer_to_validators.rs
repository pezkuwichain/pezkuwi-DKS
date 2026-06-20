//! Transfer HEZ from Founder (SQM) to all 21 validators
//!
//! Run with:
//!   SUDO_MNEMONIC="..." RPC_URL="ws://217.77.6.126:9944" \
//!   AMOUNT_HEZ=500000 \
//!   cargo run --release --example transfer_to_validators

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
	println!("=== TRANSFER HEZ TO VALIDATORS ===\n");

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

	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9944".to_string());
	let amount_hez: u128 =
		std::env::var("AMOUNT_HEZ").unwrap_or_else(|_| "500000".to_string()).parse()?;
	let amount_planck = amount_hez * PLANCKS_PER_HEZ;
	let skip: usize =
		std::env::var("SKIP").unwrap_or_else(|_| "0".to_string()).parse().unwrap_or(0);

	println!("RPC: {}", url);
	println!("Amount per validator: {} HEZ ({} TYR)", amount_hez, amount_planck);
	println!(
		"Total: {} HEZ to {} validators",
		amount_hez * (validators.len() - skip) as u128,
		validators.len() - skip
	);

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected!");

	let mnemonic_str =
		std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC environment variable required");
	let mnemonic = Mnemonic::from_str(&mnemonic_str)?;
	let keypair = Keypair::from_phrase(&mnemonic, None)?;
	println!("Sender: {}\n", keypair.public_key().to_account_id());

	let mut success_count = 0;
	let mut fail_count = 0;

	for (i, (name, ss58)) in validators.iter().enumerate().skip(skip) {
		println!("--- [{}/{}] {} ---", i + 1, validators.len(), name);

		let dest: AccountId32 = match ss58.parse() {
			Ok(a) => a,
			Err(e) => {
				println!("  ERROR: Invalid address: {}", e);
				fail_count += 1;
				continue;
			},
		};

		// Balances::transfer_keep_alive(dest, value)
		let transfer_tx = pezkuwi_subxt::dynamic::tx(
			"Balances",
			"transfer_keep_alive",
			vec![
				Value::unnamed_variant("Id", vec![Value::from_bytes(&dest.0)]),
				Value::u128(amount_planck),
			],
		);

		// Retry up to 3 times
		use pezkuwi_subxt::tx::TxStatus;
		let mut tx_ok = false;

		for attempt in 0..3 {
			let tx_progress =
				match api.tx().sign_and_submit_then_watch_default(&transfer_tx, &keypair).await {
					Ok(p) => p,
					Err(e) => {
						println!("  SUBMIT ERROR (attempt {}): {}", attempt + 1, e);
						tokio::time::sleep(std::time::Duration::from_secs(12)).await;
						continue;
					},
				};

			println!("  TX: 0x{}", hex::encode(tx_progress.extrinsic_hash().as_ref()));

			let mut progress = tx_progress;
			loop {
				let status = progress.next().await;
				match status {
					Some(Ok(TxStatus::InBestBlock(details))) => {
						match details.wait_for_success().await {
							Ok(events) => {
								for ev in events.iter().flatten() {
									if ev.pallet_name() == "Balances"
										&& ev.variant_name() == "Transfer"
									{
										println!("  SUCCESS: {} HEZ transferred", amount_hez);
										tx_ok = true;
									}
								}
								if !tx_ok {
									println!("  WARNING: No Transfer event found");
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
				break;
			}
			tokio::time::sleep(std::time::Duration::from_secs(12)).await;
		}

		if tx_ok {
			success_count += 1;
		} else {
			fail_count += 1;
		}

		// Wait for block inclusion before next TX
		if i + 1 < validators.len() {
			tokio::time::sleep(std::time::Duration::from_secs(12)).await;
		}
	}

	println!("\n=== RESULTS ===");
	println!("Success: {}/{}", success_count, validators.len() - skip);
	println!("Failed:  {}/{}", fail_count, validators.len() - skip);
	println!("Total transferred: {} HEZ", amount_hez * success_count as u128);

	Ok(())
}
