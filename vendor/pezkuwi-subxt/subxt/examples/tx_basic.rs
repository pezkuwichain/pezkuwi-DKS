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

	// Build a balance transfer extrinsic.
	let dest = pezkuwi::runtime_types::sp_runtime::multiaddress::MultiAddress::Id(
		pezkuwi::runtime_types::sp_core::crypto::AccountId32(dev::bob().public_key().0),
	);
	let balance_transfer_tx = pezkuwi::tx().balances().transfer_allow_death(dest, 10_000);

	// Submit the balance transfer extrinsic from Alice, and wait for it to be successful
	// and in a finalized block. We get back the extrinsic events if all is well.
	let from = dev::alice();
	let events = api
		.tx()
		.sign_and_submit_then_watch_default(&balance_transfer_tx, &from)
		.await?
		.wait_for_finalized_success()
		.await?;

	// Find a Transfer event and print it.
	let transfer_event = events.find_first::<pezkuwi::balances::events::Transfer>()?;
	if let Some(event) = transfer_event {
		println!("Balance transfer success: {event:?}");
	}

	Ok(())
}
