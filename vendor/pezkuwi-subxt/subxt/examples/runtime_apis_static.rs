#![allow(missing_docs)]
use pezkuwi_subxt::{config::PezkuwiConfig, OnlineClient};
use pezkuwi_subxt_signer::sr25519::dev;

#[pezkuwi_subxt::subxt(runtime_metadata_path = "../artifacts/pezkuwi_metadata_small.scale")]
pub mod pezkuwi {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a client to use:
	let api = OnlineClient::<PezkuwiConfig>::new().await?;

	// Create a runtime API payload that calls into
	// `AccountNonceApi_account_nonce` function.
	let account = pezkuwi::runtime_types::sp_core::crypto::AccountId32(dev::alice().public_key().0);
	let runtime_api_call = pezkuwi::apis().account_nonce_api().account_nonce(account);

	// Submit the call and get back a result.
	let nonce = api.runtime_api().at_latest().await?.call(runtime_api_call).await;

	println!("AccountNonceApi_account_nonce for Alice: {nonce:?}");
	Ok(())
}
