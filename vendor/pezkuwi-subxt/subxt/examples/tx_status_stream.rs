#![allow(missing_docs)]
use pezkuwi_subxt::{tx::TxStatus, OnlineClient, PezkuwiConfig};
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

	// Submit the balance transfer extrinsic from Alice, and then monitor the
	// progress of it.
	let from = dev::alice();
	let mut balance_transfer_progress =
		api.tx().sign_and_submit_then_watch_default(&balance_transfer_tx, &from).await?;

	while let Some(status) = balance_transfer_progress.next().await {
		match status? {
			// It's finalized in a block!
			TxStatus::InFinalizedBlock(in_block) => {
				println!(
					"Transaction {:?} is finalized in block {:?}",
					in_block.extrinsic_hash(),
					in_block.block_hash()
				);

				// grab the events and fail if no ExtrinsicSuccess event seen:
				let events = in_block.wait_for_success().await?;
				// We can look for events (this uses the static interface; we can also iterate
				// over them and dynamically decode them):
				let transfer_event = events.find_first::<pezkuwi::balances::events::Transfer>()?;

				if let Some(event) = transfer_event {
					println!("Balance transfer success: {event:?}");
				} else {
					println!("Failed to find Balances::Transfer Event");
				}
			},
			// Just log any other status we encounter:
			other => {
				println!("Status: {other:?}");
			},
		}
	}
	Ok(())
}
