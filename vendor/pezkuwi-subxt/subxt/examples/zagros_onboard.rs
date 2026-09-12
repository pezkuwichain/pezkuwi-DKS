//! Zagros: onboard a teyrchain with `paras_sudo_wrapper.sudo_schedule_para_initialize`.
//!
//! One call does two things, and the second is easy to miss: when `para_kind` is `Teyrchain`,
//! the pallet calls `AssignCoretime::assign_coretime`, which on this runtime adds a core and
//! gives the para a full share of it. So a separate `Coretime.assign_core` afterwards is not
//! "the step that makes it produce blocks" -- it is a second core nobody asked for.
//!
//! Run:
//!   SUDO_MNEMONIC="..." RPC_URL="ws://127.0.0.1:19944" PARA_ID=1000 \
//!   HEAD_FILE=/opt/zagros/onboard/asset-hub-zagros.head \
//!   WASM_FILE=/opt/zagros/onboard/asset-hub-zagros.wasm \
//!   cargo run --release --example zagros_onboard
#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::ext::scale_value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::sr25519::Keypair;
use pezkuwi_subxt_signer::SecretUri;
use std::str::FromStr;

fn read_hex(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
	let s = std::fs::read_to_string(path)?;
	let s = s.trim().trim_start_matches("0x");
	Ok((0..s.len())
		.step_by(2)
		.map(|i| u8::from_str_radix(&s[i..i + 2], 16))
		.collect::<Result<_, _>>()?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL")?;
	let para: u32 = std::env::var("PARA_ID")?.parse()?;
	let head = read_hex(&std::env::var("HEAD_FILE")?)?;
	let wasm = read_hex(&std::env::var("WASM_FILE")?)?;
	println!("para {para} · head {} bytes · wasm {} bytes", head.len(), wasm.len());

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	// The signer is the sudo account, and the sudo account is a *derivation*: every key in this
	// chain's wallet set hangs off one phrase by path. Signing with the bare phrase signs as an
	// account that holds nothing and is nobody -- the first attempt failed with "inability to
	// pay some fees" for exactly that reason, which reads like a funding problem and is not one.
	let uri = SecretUri::from_str(&format!(
		"{}{}",
		std::env::var("SUDO_MNEMONIC")?,
		std::env::var("SUDO_PATH").unwrap_or_else(|_| "//zagros//sudo".into())
	))?;
	let signer = Keypair::from_uri(&uri)?;
	println!("signer: {}", signer.public_key().to_account_id());

	let genesis = Value::named_composite(vec![
		("genesis_head", Value::from_bytes(head)),
		("validation_code", Value::from_bytes(wasm)),
		// `ParaKind` is a two-variant enum in Rust and a *bool* on the wire: it carries a hand
		// -written Encode/TypeInfo because the field used to be a bool and the encoding was
		// kept. `Teyrchain` is `true`. Passing the variant name gets rejected by the metadata.
		("para_kind", Value::bool(true)),
	]);
	let inner = pezkuwi_subxt::dynamic::tx(
		"ParasSudoWrapper",
		"sudo_schedule_para_initialize",
		vec![Value::u128(para as u128), genesis],
	);
	let call = pezkuwi_subxt::dynamic::tx("Sudo", "sudo", vec![inner.into_value()]);

	let events = api
		.tx()
		.sign_and_submit_then_watch_default(&call, &signer)
		.await?
		.wait_for_finalized_success()
		.await?;
	println!("finalized in block {:?}", events.extrinsic_hash());
	for e in events.iter() {
		let e = e?;
		println!("  {}::{}", e.pallet_name(), e.variant_name());
	}
	Ok(())
}
