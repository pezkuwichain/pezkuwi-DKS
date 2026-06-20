#![allow(missing_docs)]
use pezkuwi_subxt::{
	config::pezkuwi::PezkuwiExtrinsicParamsBuilder as Params, OnlineClient, PezkuwiConfig,
};
use pezkuwi_subxt_signer::sr25519::dev;

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
	let tx = pezkuwi::tx().balances().transfer_allow_death(dest, 10_000);

	// Configure the transaction parameters; we give a small tip and set the
	// transaction to live for 32 blocks from the `latest_block` above.
	let tx_params = Params::new().tip(1_000).mortal(32).build();

	// submit the transaction:
	let from = dev::alice();
	let hash = api.tx().sign_and_submit(&tx, &from, tx_params).await?;
	println!("Balance transfer extrinsic submitted with hash : {hash}");

	Ok(())
}
