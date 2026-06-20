#![allow(missing_docs)]
use pezkuwi_subxt::{
	backend::{legacy::LegacyRpcMethods, rpc::RpcClient},
	config::DefaultExtrinsicParamsBuilder as Params,
	OnlineClient, PezkuwiConfig,
};
use pezkuwi_subxt_signer::sr25519::dev;

#[pezkuwi_subxt::subxt(runtime_metadata_path = "../artifacts/pezkuwi_metadata_small.scale")]
pub mod pezkuwi {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// First, create a raw RPC client:
	let rpc_client = RpcClient::from_url("ws://127.0.0.1:9944").await?;

	// Use this to construct our RPC methods:
	let rpc = LegacyRpcMethods::<PezkuwiConfig>::new(rpc_client.clone());

	// We can use the same client to drive our full Subxt interface too:
	let api = OnlineClient::<PezkuwiConfig>::from_rpc_client(rpc_client.clone()).await?;

	// Now, we can make some RPC calls using some legacy RPC methods.
	println!(
		"📛 System Name: {:?}\n🩺 Health: {:?}\n🖫 Properties: {:?}\n🔗 Chain: {:?}\n",
		rpc.system_name().await?,
		rpc.system_health().await?,
		rpc.system_properties().await?,
		rpc.system_chain().await?
	);

	// We can also interleave RPC calls and using the full Subxt client, here to submit multiple
	// transactions using the legacy `system_account_next_index` RPC call, which returns a nonce
	// that is adjusted for any transactions already in the pool:

	let alice = dev::alice();
	let bob = dev::bob();

	loop {
		let alice_account_id = pezkuwi_subxt::config::bizinikiwi::AccountId32(alice.public_key().0);
		let current_nonce = rpc.system_account_next_index(&alice_account_id).await?;

		let ext_params = Params::new().mortal(8).nonce(current_nonce).build();

		let dest = pezkuwi::runtime_types::sp_runtime::multiaddress::MultiAddress::Id(
			pezkuwi::runtime_types::sp_core::crypto::AccountId32(bob.public_key().0),
		);
		let balance_transfer = pezkuwi::tx().balances().transfer_allow_death(dest, 1_000_000);

		let ext_hash = api
			.tx()
			.create_partial_offline(&balance_transfer, ext_params)?
			.sign(&alice)
			.submit()
			.await?;

		println!("Submitted ext {ext_hash} with nonce {current_nonce}");

		// Sleep less than block time, but long enough to ensure
		// not all transactions end up in the same block.
		tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
	}
}
