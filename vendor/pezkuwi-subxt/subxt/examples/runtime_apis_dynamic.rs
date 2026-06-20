#![allow(missing_docs)]
use pezkuwi_subxt::{config::PezkuwiConfig, utils::AccountId32, OnlineClient};
use pezkuwi_subxt_signer::sr25519::dev;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a client to use:
	let api = OnlineClient::<PezkuwiConfig>::new().await?;

	// Create a "dynamic" runtime API payload that calls the
	// `AccountNonceApi_account_nonce` function. We could use the
	// `scale_value::Value` type as output, and a vec of those as inputs,
	// but since we know the input + return types we can pass them directly.
	// There is one input argument, so the inputs are a tuple of one element.
	let account = AccountId32(dev::alice().public_key().0);
	let runtime_api_call = pezkuwi_subxt::dynamic::runtime_api_call::<_, u64>(
		"AccountNonceApi",
		"account_nonce",
		(account,),
	);

	// Submit the call to get back a result.
	let nonce = api.runtime_api().at_latest().await?.call(runtime_api_call).await?;

	println!("Account nonce: {:#?}", nonce);
	Ok(())
}
