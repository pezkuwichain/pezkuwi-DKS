//! Zagros relay: deliver a runtime upgrade, with the version check left on.
//!
//! There is already a `zagros_upgrade` example and it is the wrong tool for this. It calls
//! `set_code_without_checks`, which skips the guard that refuses a runtime whose
//! `spec_version` is not greater than the running one -- so a forgotten version bump installs
//! silently and the chain keeps answering with the old code while the deploy reports success.
//! It also writes `ValidatorCount` and `ForceEra` as raw storage keys, which belonged to a
//! different job. This one does the upgrade and nothing else.
//!
//! `set_code` is weighed at a whole block, so it cannot ride inside a normal `sudo`; the call
//! goes through `sudo_unchecked_weight`. That overrides the *fee* weight, not the version
//! check -- the two are often confused, and confusing them is how `without_checks` ends up
//! looking like the only option.
//!
//! The run is not finished when the extrinsic succeeds. `System::CodeUpdated` is the event
//! that says the runtime was replaced, and the chain's own `state_getRuntimeVersion` afterwards
//! is the only statement that the new code is what is running. Both are checked here, and the
//! process exits non-zero if either fails.
//!
//! Run:
//!   SUDO_MNEMONIC="..." SUDO_PATH="//zagros//sudo" \
//!   WASM_FILE=target/release/wbuild/zagros-runtime/zagros_runtime.compact.compressed.wasm \
//!   EXPECT_SPEC_VERSION=1020011 \
//!   RPC_URL="wss://zagros-rpc.pezkuwichain.io" \
//!   cargo run --release --example zagros_relay_upgrade -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::dynamic::Value;
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::sr25519::Keypair;
use pezkuwi_subxt_signer::SecretUri;
use std::str::FromStr;

/// Read `spec_version` through a freshly connected client.
///
/// Fresh on purpose: a client caches the runtime version it saw at connect time, so asking the
/// same handle after the upgrade returns the number from before it -- which would make this
/// check confirm itself. Reconnecting is the only way to get the chain's current answer.
async fn spec_version(url: &str) -> Result<u32, Box<dyn std::error::Error>> {
	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(url).await?;
	Ok(api.runtime_version().spec_version)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL").expect("RPC_URL required");
	let wasm_file = std::env::var("WASM_FILE").expect("WASM_FILE required");
	let expect: u32 = std::env::var("EXPECT_SPEC_VERSION")
		.expect("EXPECT_SPEC_VERSION required -- state what you believe you are installing")
		.parse()?;

	let code = std::fs::read(&wasm_file)?;
	println!("wasm        : {} ({} bytes)", wasm_file, code.len());

	let before = spec_version(&url).await?;
	println!("running now : spec_version {before}");
	println!("installing  : spec_version {expect}");
	// Caught here rather than by the chain, because the chain's refusal arrives as a failed
	// extrinsic in a block and reads like a permissions problem.
	if expect <= before {
		return Err(format!(
			"refusing: {expect} is not greater than the running {before}. `set_code` would be \
			 rejected -- bump `spec_version` in the runtime and rebuild."
		)
		.into());
	}

	let api = OnlineClient::<PezkuwiConfig>::from_insecure_url(&url).await?;
	let mnemonic = std::env::var("SUDO_MNEMONIC").expect("SUDO_MNEMONIC required");
	let path = std::env::var("SUDO_PATH").unwrap_or_default();
	let signer = Keypair::from_uri(&SecretUri::from_str(&format!("{mnemonic}{path}"))?)?;
	println!("sudo        : {}", signer.public_key().to_account_id());

	let set_code = pezkuwi_subxt::dynamic::tx("System", "set_code", vec![Value::from_bytes(&code)]);
	// The weight is nominal: `sudo_unchecked_weight` exists to stop `set_code`'s
	// whole-block weight from making the outer call unschedulable. It does not weaken the
	// version check, which lives inside `set_code` itself.
	let call = pezkuwi_subxt::dynamic::tx(
		"Sudo",
		"sudo_unchecked_weight",
		vec![
			set_code.into_value(),
			Value::named_composite(vec![
				("ref_time", Value::u128(1)),
				("proof_size", Value::u128(1)),
			]),
		],
	);

	use pezkuwi_subxt::tx::TxStatus;
	let mut progress = api.tx().sign_and_submit_then_watch_default(&call, &signer).await?;
	println!("tx          : 0x{}", hex::encode(progress.extrinsic_hash().as_ref()));

	let mut code_updated = false;
	loop {
		match progress.next().await {
			Some(Ok(TxStatus::InBestBlock(details))) => {
				let events = details.wait_for_success().await?;
				println!("in block    : {:?}", details.block_hash());
				for ev in events.iter().flatten() {
					println!("  {}::{}", ev.pallet_name(), ev.variant_name());
					if ev.pallet_name() == "System" && ev.variant_name() == "CodeUpdated" {
						code_updated = true;
					}
				}
				break;
			},
			Some(Ok(_)) => continue,
			Some(Err(e)) => return Err(e.into()),
			None => {
				return Err("the subscription ended before the transaction was in a block".into())
			},
		}
	}
	if !code_updated {
		return Err("the extrinsic succeeded but no `System::CodeUpdated` was emitted -- the \
		            runtime was not replaced"
			.into());
	}

	// The event says the code was replaced. Only the chain's own answer says what is running.
	tokio::time::sleep(std::time::Duration::from_secs(12)).await;
	let after = spec_version(&url).await?;
	println!("running now : spec_version {after}");
	if after != expect {
		return Err(
			format!("the chain reports {after}, not the {expect} that was installed").into()
		);
	}
	println!("\nGREEN: {before} -> {after}");
	Ok(())
}
