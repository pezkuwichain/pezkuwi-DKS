#![allow(missing_docs)]
use pezkuwi_subxt::config::{
	BizinikiwConfig, Config, DefaultExtrinsicParams, DefaultExtrinsicParamsBuilder, PezkuwiConfig,
};
use pezkuwi_subxt_signer::sr25519::dev;

#[pezkuwi_subxt::subxt(
	runtime_metadata_path = "../artifacts/pezkuwi_metadata_full.scale",
	derive_for_type(
		path = "staging_xcm::v3::multilocation::MultiLocation",
		derive = "Clone, codec::Encode",
		recursive
	)
)]
pub mod runtime {}
use runtime::runtime_types::{
	staging_xcm::v3::multilocation::MultiLocation, xcm::v3::junctions::Junctions,
};

// We don't need to construct this at runtime, so an empty enum is appropriate.
pub enum AssetHubConfig {}

impl Config for AssetHubConfig {
	type AccountId = <BizinikiwConfig as Config>::AccountId;
	type Address = <PezkuwiConfig as Config>::Address;
	type Signature = <BizinikiwConfig as Config>::Signature;
	type Hasher = <BizinikiwConfig as Config>::Hasher;
	type Header = <BizinikiwConfig as Config>::Header;
	type ExtrinsicParams = DefaultExtrinsicParams<AssetHubConfig>;
	// Here we use the MultiLocation from the metadata as a part of the config:
	// The `ChargeAssetTxPayment` signed extension that is part of the ExtrinsicParams above, now
	// uses the type:
	type AssetId = MultiLocation;
}

#[tokio::main]
async fn main() {
	// With the config defined, we can create an extrinsic with subxt:
	let client = pezkuwi_subxt::OnlineClient::<AssetHubConfig>::new().await.unwrap();
	let tx_payload = runtime::tx().system().remark(b"Hello".to_vec());

	// Build extrinsic params using an asset at this location as a tip:
	let location: MultiLocation = MultiLocation { parents: 3, interior: Junctions::Here };
	let tx_config = DefaultExtrinsicParamsBuilder::<AssetHubConfig>::new()
		.tip_of(1234, location)
		.build();

	// And provide the extrinsic params including the tip when submitting a transaction:
	let _ = client
		.tx()
		.sign_and_submit_then_watch(&tx_payload, &dev::alice(), tx_config)
		.await;
}
