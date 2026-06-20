#![allow(missing_docs)]
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};

#[pezkuwi_subxt::subxt(runtime_metadata_path = "../artifacts/pezkuwi_metadata_small.scale")]
pub mod pezkuwi {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Create a client to use:
	let api = OnlineClient::<PezkuwiConfig>::new().await?;

	// A query to obtain some constant:
	let constant_query = pezkuwi::constants().system().block_length();

	// Obtain the value:
	let value = api.constants().at(&constant_query)?;

	// Or obtain the bytes:
	let bytes = api.constants().bytes_at(&constant_query)?;

	println!("Encoded block length: {bytes:?}");
	println!("Block length: {value:?}");
	Ok(())
}
