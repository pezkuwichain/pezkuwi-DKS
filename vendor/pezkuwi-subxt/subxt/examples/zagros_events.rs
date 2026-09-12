//! Print the events of the last N blocks of a chain, so an XCM failure can be read.
//!
//! An XCM that fails does so on the receiving chain, in a block nobody watched, and the
//! sending chain reported success. The only way to see it is to go and look.
//!
//! Run with:
//!   URL=wss://zagros-asset-hub-rpc.pezkuwichain.io BLOCKS=40 \
//!   cargo run --release --example zagros_events -p pezkuwi-subxt

#![allow(missing_docs)]
use pezkuwi_subxt::{OnlineClient, PezkuwiConfig};

/// Events every block carries. Printing them buries the one that matters.
const NOISE: &[(&str, &str)] = &[
	("System", "ExtrinsicSuccess"),
	("TransactionPayment", "TransactionFeePaid"),
	("Balances", "Deposit"),
	("Balances", "Withdraw"),
	("Treasury", "UpdatedInactive"),
	("ParaInclusion", "CandidateBacked"),
	("ParaInclusion", "CandidateIncluded"),
	("ParaInclusion", "CandidateTimedOut"),
	("TeyrchainSystem", "ValidationFunctionApplied"),
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = std::env::var("URL").expect("URL required");
	let n: u32 = std::env::var("BLOCKS").unwrap_or_else(|_| "40".into()).parse()?;
	let api = OnlineClient::<PezkuwiConfig>::from_url(&url).await?;

	let head = api.blocks().at_latest().await?;
	let top = head.number();
	println!("{url}\nen üst blok {top}, son {n} blok taranıyor\n");

	// Walk back by parent hash rather than asking for a hash by number: the
	// height-to-hash RPC is not on the typed backend, and the parent link is
	// what a chain guarantees anyway.
	let mut hash = head.hash();
	for _ in 0..=n {
		let block = api.blocks().at(hash).await?;
		let h = block.number();
		hash = block.header().parent_hash;
		let mut lines = Vec::new();
		for ev in block.events().await?.iter() {
			let ev = ev?;
			let (p, v) = (ev.pallet_name().to_string(), ev.variant_name().to_string());
			if NOISE.iter().any(|(a, b)| *a == p && *b == v) {
				continue;
			}
			let field_str =
				match ev.decode_as_fields::<pezkuwi_subxt::ext::scale_value::Composite<()>>() {
					Ok(f) => {
						let s = format!("{f}");
						if s.len() > 300 {
							format!("{}…", &s[..300])
						} else {
							s
						}
					},
					Err(e) => format!("<decode failed: {e}>"),
				};
			lines.push(format!("    {p}::{v}  {field_str}"));
		}
		if !lines.is_empty() {
			println!("  #{h}");
			for l in lines {
				println!("{l}");
			}
		}
	}
	Ok(())
}
