//! One-off diagnostic: dump the RuntimeHoldReason enum's variant list, in
//! discriminant order, from live chain metadata.
//!
//! Run with:
//!   RPC_URL="ws://217.77.6.126:9944" cargo run --release -p pezkuwi-subxt --example holdreason_check

#![allow(missing_docs)]

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("RPC_URL").unwrap_or_else(|_| "ws://217.77.6.126:9944".to_string());
	let api = pezkuwi_subxt::OnlineClient::<pezkuwi_subxt::PezkuwiConfig>::from_insecure_url(&url)
		.await?;
	let metadata = api.metadata();
	let types = metadata.types();

	for ty in types.types.iter() {
		let path_str = ty.ty.path.segments.join("::");
		if path_str.contains("HoldReason") {
			println!("=== TYPE: {} ===", path_str);
			if let scale_info::TypeDef::Variant(v) = &ty.ty.type_def {
				for variant in &v.variants {
					println!("  index {}: {}", variant.index, variant.name);
				}
			} else {
				println!("  (not a variant/enum type)");
			}
		}
	}

	Ok(())
}
