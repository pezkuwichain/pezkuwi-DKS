#![allow(missing_docs)]
use pezkuwi_subxt::{
	dynamic::{At, Value},
	utils::AccountId32,
	OnlineClient, PezkuwiConfig,
};
use pezkuwi_subxt_signer::sr25519::dev;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a new API client, configured to talk to Pezkuwi nodes.
	let api = OnlineClient::<PezkuwiConfig>::new().await?;

	// Build a dynamic storage query to access account information.
	// here, we assume that there is one value to provide at this entry
	// to access a value; an AccountId32. In this example we don't know the
	// return type and so we set it to `Value`, which anything can decode into.
	let account = AccountId32(dev::alice().public_key().0);
	let storage_query =
		pezkuwi_subxt::dynamic::storage::<(AccountId32,), Value>("System", "Account");

	// Use that query to access a storage entry, fetch a result and decode the value.
	let client_at = api.storage().at_latest().await?;
	let account_info = client_at.entry(storage_query)?.fetch((account,)).await?.decode()?;

	// With out `Value` type we can dig in to find what we want using the `At`
	// trait and `.at()` method that this provides on the Value.
	println!("Alice has free balance: {}", account_info.at("data").at("free").unwrap());
	Ok(())
}
