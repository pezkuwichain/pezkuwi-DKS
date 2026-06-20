#![allow(missing_docs)]
use pezkuwi_subxt::{
	ext::{
		codec::{Compact, Decode},
		frame_metadata::RuntimeMetadataPrefixed,
	},
	OnlineClient, PezkuwiConfig,
};

#[pezkuwi_subxt::subxt(runtime_metadata_path = "../artifacts/pezkuwi_metadata_small.scale")]
pub mod pezkuwi {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a client to use:
	let api = OnlineClient::<PezkuwiConfig>::new().await?;

	// Use runtime APIs at the latest block:
	let runtime_apis = api.runtime_api().at_latest().await?;

	// Ask for metadata and decode it:
	let result_bytes = runtime_apis.call_raw("Metadata_metadata", None).await?;
	let (_, meta): (Compact<u32>, RuntimeMetadataPrefixed) = Decode::decode(&mut &*result_bytes)?;

	println!("{meta:?}");
	Ok(())
}
