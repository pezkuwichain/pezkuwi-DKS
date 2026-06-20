//! Check if validators have Noter tiki on People Chain
//!
//! Run with:
//!   cargo run --release -p pezkuwi-subxt --example check_noter_tiki

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::utils::AccountId32;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:41944".to_string());

	println!("=== CHECK NOTER TIKI ON PEOPLE CHAIN ===\n");
	println!("RPC: {}", url);

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	println!("Connected! spec_version: {}\n", api.runtime_version().spec_version);

	let storage = api.storage().at_latest().await?;

	let mut noter_count = 0;
	let mut no_tiki_count = 0;

	for (name, ss58) in validators() {
		let account: AccountId32 = ss58.parse()?;

		// Query UserTikis storage map: Tiki::UserTikis(AccountId32) -> Vec<Tiki>
		let query = pezkuwi_subxt::dynamic::storage::<Vec<Value>, Value>("Tiki", "UserTikis");
		match storage.try_fetch(query, vec![Value::from_bytes(&account.0)]).await? {
			Some(val) => {
				let decoded = val.decode()?;
				println!("{}: {:?}", name, decoded);
				noter_count += 1;
			},
			None => {
				println!("{}: NO TIKIS", name);
				no_tiki_count += 1;
			},
		}
	}

	println!("\n=== SUMMARY ===");
	println!("With tikis: {}", noter_count);
	println!("No tikis:   {}", no_tiki_count);

	Ok(())
}
