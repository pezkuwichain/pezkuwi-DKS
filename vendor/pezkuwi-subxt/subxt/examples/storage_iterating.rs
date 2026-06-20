#![allow(missing_docs)]
use pezkuwi_subxt::{ext::futures::StreamExt, OnlineClient, PezkuwiConfig};

// Generate an interface that we can use from the node's metadata.
#[pezkuwi_subxt::subxt(runtime_metadata_path = "../artifacts/pezkuwi_metadata_small.scale")]
pub mod pezkuwi {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a new API client, configured to talk to Pezkuwi nodes.
	let api = OnlineClient::<PezkuwiConfig>::new().await?;

	// Build a storage query to access account information. Same as if we were
	// fetching a single value from this entry.
	let storage_query = pezkuwi::storage().system().account();

	// Use that query to access a storage entry, iterate over it and decode values.
	let client_at = api.storage().at_latest().await?;

	// We provide an empty tuple when iterating. If the storage entry had been an N map with
	// multiple keys, then we could provide any prefix of those keys to iterate over. This is
	// statically type checked, so only a valid number/type of keys in the tuple is accepted.
	let mut values = client_at.entry(storage_query)?.iter(()).await?;

	while let Some(kv) = values.next().await {
		let kv = kv?;

		// The key decodes into the type that the static address knows about, in this case a
		// tuple of one entry, because the only part of the key that we can decode is the
		// AccountId32 for each user.
		let (account_id32,) = kv.key()?.decode()?;

		// The value decodes into a statically generated type which holds account information.
		let value = kv.value().decode()?;

		let value_data = value.data;
		println!("{account_id32:?}:\n  {value_data:?}");
	}

	Ok(())
}
