//! Reads the register from a People node over JSON-RPC, every map at one finalized block.

use anyhow::{anyhow, bail, Context, Result};
use codec::{Decode, DecodeAll};
use pezpallet_navnisan::{Moment, NameHash, Suspension};
use pezsp_crypto_hashing::twox_128;
use serde_json::{json, Value};

use crate::register::{Record, Snapshot};

/// The pallet's name in both People runtimes' `construct_runtime!`.
const PALLET: &[u8] = b"Navnisan";
const PAGE: u32 = 1000;

pub fn storage_prefix(pallet: &[u8], item: &[u8]) -> Vec<u8> {
	[twox_128(pallet), twox_128(item)].concat()
}

/// `Names`, `Suspensions` and `Locked` are `Identity`-hashed by the node hash, so the key is the
/// prefix followed by the 32 bytes of the node.
fn node_of(key: &[u8], prefix_len: usize) -> Result<NameHash> {
	let raw: [u8; 32] = key
		.get(prefix_len..)
		.and_then(|k| k.try_into().ok())
		.ok_or_else(|| anyhow!("storage key of {} bytes is not prefix + node", key.len()))?;
	Ok(NameHash::from(raw))
}

pub struct Rpc {
	http: reqwest::Client,
	url: String,
}

impl Rpc {
	pub fn new(url: &str) -> Self {
		Self { http: reqwest::Client::new(), url: url.to_owned() }
	}

	async fn call(&self, method: &str, params: Value) -> Result<Value> {
		let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
		let reply: Value = self
			.http
			.post(&self.url)
			.json(&body)
			.send()
			.await
			.with_context(|| format!("{method}: cannot reach {}", self.url))?
			.error_for_status()?
			.json()
			.await?;
		if let Some(err) = reply.get("error") {
			bail!("{method}: {err}");
		}
		Ok(reply.get("result").cloned().unwrap_or(Value::Null))
	}

	async fn pairs(&self, prefix: &[u8], at: &str) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let prefix_hex = format!("0x{}", hex::encode(prefix));
		let mut out = Vec::new();
		let mut start: Option<String> = None;
		loop {
			let keys: Vec<String> = serde_json::from_value(
				self.call("state_getKeysPaged", json!([prefix_hex, PAGE, start, at])).await?,
			)?;
			if keys.is_empty() {
				break;
			}
			let changes = self.call("state_queryStorageAt", json!([keys, at])).await?;
			let changes = changes
				.get(0)
				.and_then(|c| c.get("changes"))
				.and_then(Value::as_array)
				.ok_or_else(|| anyhow!("state_queryStorageAt: no changes"))?;
			for change in changes {
				let (Some(k), Some(v)) = (change.get(0).and_then(Value::as_str), change.get(1))
				else {
					continue;
				};
				// A key listed a moment ago and gone by the time it is read: nothing to keep
				let Some(v) = v.as_str() else { continue };
				out.push((unhex(k)?, unhex(v)?));
			}
			if keys.len() < PAGE as usize {
				break;
			}
			start = keys.last().cloned();
		}
		Ok(out)
	}

	async fn map<V: Decode>(&self, item: &[u8], at: &str) -> Result<Vec<(NameHash, V)>> {
		let prefix = storage_prefix(PALLET, item);
		let item = String::from_utf8_lossy(item);
		self.pairs(&prefix, at)
			.await?
			.into_iter()
			.map(|(k, v)| {
				let node = node_of(&k, prefix.len())?;
				let value = V::decode_all(&mut &v[..])
					.map_err(|e| anyhow!("{item} at {node:?} does not decode: {e}"))?;
				Ok((node, value))
			})
			.collect()
	}

	/// The register at the newest finalized block, and that block's hash.
	pub async fn snapshot(&self) -> Result<(String, Snapshot)> {
		let at: String =
			serde_json::from_value(self.call("chain_getFinalizedHead", json!([])).await?)?;
		let now_key = format!("0x{}", hex::encode(storage_prefix(b"Timestamp", b"Now")));
		let now = match self.call("state_getStorage", json!([now_key, at])).await? {
			Value::String(v) => Moment::decode_all(&mut &unhex(&v)?[..])?,
			_ => bail!("Timestamp::Now is empty at {at}"),
		};
		let snap = Snapshot {
			names: self.map::<Record>(b"Names", &at).await?.into_iter().collect(),
			suspensions: self.map::<Suspension>(b"Suspensions", &at).await?.into_iter().collect(),
			locked: self.map::<u8>(b"Locked", &at).await?.into_iter().collect(),
			now,
		};
		Ok((at, snap))
	}
}

fn unhex(s: &str) -> Result<Vec<u8>> {
	Ok(hex::decode(s.trim_start_matches("0x"))?)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn timestamp_now_key_is_the_well_known_one() {
		assert_eq!(
			hex::encode(storage_prefix(b"Timestamp", b"Now")),
			"f0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb"
		);
	}

	#[test]
	fn node_is_the_key_after_the_prefix() {
		let prefix = storage_prefix(PALLET, b"Names");
		let node = NameHash::repeat_byte(7);
		let key = [prefix.clone(), node.as_bytes().to_vec()].concat();
		assert_eq!(node_of(&key, prefix.len()).unwrap(), node);
		assert!(node_of(&key[..key.len() - 1], prefix.len()).is_err());
	}
}
