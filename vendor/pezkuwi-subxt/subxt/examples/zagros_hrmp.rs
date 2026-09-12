//! Zagros: open the HRMP channel between two system teyrchains.
//!
//! `establish_system_channel` is permissionless and takes `ensure_signed`: wrapping it in Sudo
//! makes it fail, because Root is not a signed origin. Any funded account can call it as long as
//! both paras are system teyrchains (id < 2000), and the channel opens with the configuration's
//! defaults rather than a negotiation.
//!
//! The extrinsic succeeding does not mean the channel is open. It takes effect at the next
//! session boundary, and `hrmp.hrmpChannels` stays empty until then -- which is exactly how this
//! step gets marked done while sibling transfers still fail.
//!
//! Run:
//!   SENDER_MNEMONIC="..." SENDER_PATH="//zagros//allocation//founder" \
//!   RPC_URL="ws://127.0.0.1:19944" SENDER_PARA=1000 RECIPIENT_PARA=1004 \
//!   cargo run --release -p pezkuwi-subxt --example zagros_hrmp
#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::sr25519::Keypair;
use pezkuwi_subxt_signer::SecretUri;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL")?;
	let a: u128 = std::env::var("SENDER_PARA")?.parse()?;
	let b: u128 = std::env::var("RECIPIENT_PARA")?.parse()?;

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	let uri = SecretUri::from_str(&format!(
		"{}{}",
		std::env::var("SENDER_MNEMONIC")?,
		std::env::var("SENDER_PATH").unwrap_or_default()
	))?;
	let signer = Keypair::from_uri(&uri)?;
	println!("signer: {} · channel {a} -> {b}", signer.public_key().to_account_id());

	let call = pezkuwi_subxt::dynamic::tx(
		"Hrmp",
		"establish_system_channel",
		vec![Value::u128(a), Value::u128(b)],
	);
	let events = api
		.tx()
		.sign_and_submit_then_watch_default(&call, &signer)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("finalized {:?}", events.extrinsic_hash());
	for e in events.iter() {
		let e = e?;
		println!("  {}::{}", e.pallet_name(), e.variant_name());
	}
	Ok(())
}
