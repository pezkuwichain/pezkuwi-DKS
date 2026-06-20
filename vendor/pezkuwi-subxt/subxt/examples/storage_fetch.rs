#![allow(missing_docs)]
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};
use pezkuwi_subxt_signer::sr25519::dev;

// Generate an interface that we can use from the node's metadata.
#[pezkuwi_subxt::subxt(runtime_metadata_path = "../artifacts/pezkuwi_metadata_small.scale")]
pub mod pezkuwi {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a new API client, configured to talk to Pezkuwi nodes.
	let api = OnlineClient::<PezkuwiConfig>::new().await?;
	let account = pezkuwi::runtime_types::sp_core::crypto::AccountId32(dev::alice().public_key().0);

	// Build a storage query to access account information.
	let storage_query = pezkuwi::storage().system().account();

	// Use that query to access a storage entry, fetch a result and decode the value.
	// The static address knows that fetching requires a tuple of one value, an
	// AccountId32.
	let client_at = api.storage().at_latest().await?;
	let account_info = client_at.entry(storage_query)?.fetch((account,)).await?.decode()?;

	// The static address that we got from the subxt macro knows the expected input
	// and return types, so it is decoded into a static type for us.
	println!("Alice: {account_info:?}");
	Ok(())
}
