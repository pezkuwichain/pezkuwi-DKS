//! The local kubo node, over its RPC API. Offline throughout: this tool only ever asks about
//! blocks the node already holds, and must not make it fetch the content it is about to refuse.

use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Context, Result};
use cid::Cid;
use serde_json::Value;

pub struct Kubo {
	http: reqwest::Client,
	api: String,
}

impl Kubo {
	pub fn new(api: &str) -> Self {
		Self { http: reqwest::Client::new(), api: api.trim_end_matches('/').to_owned() }
	}

	async fn post(&self, path: &str, query: &[(&str, &str)]) -> Result<String> {
		let reply = self
			.http
			.post(format!("{}/api/v0/{path}", self.api))
			.query(query)
			.send()
			.await
			.with_context(|| format!("kubo {path}: cannot reach {}", self.api))?;
		let status = reply.status();
		let body = reply.text().await?;
		if !status.is_success() {
			bail!("kubo {path}: {status} {body}");
		}
		Ok(body)
	}

	/// The roots this node keeps.
	pub async fn pins(&self) -> Result<BTreeSet<Cid>> {
		let body = self.post("pin/ls", &[("type", "recursive"), ("offline", "true")]).await?;
		let v: Value = serde_json::from_str(&body)?;
		let keys = v
			.get("Keys")
			.and_then(Value::as_object)
			.ok_or_else(|| anyhow!("pin/ls: no Keys"))?;
		keys.keys().map(|k| Ok(Cid::try_from(k.as_str())?)).collect()
	}

	/// Every block under `root` that this node holds, `root` included. A root the node does not
	/// hold gives just itself: a gateway that never fetches cannot serve the rest anyway.
	pub async fn blocks(&self, root: &Cid) -> Result<BTreeSet<Cid>> {
		let mut out = BTreeSet::from([*root]);
		let root_text = root.to_string();
		let query = [
			("arg", root_text.as_str()),
			("recursive", "true"),
			("unique", "true"),
			("offline", "true"),
		];
		let body = match self.post("refs", &query).await {
			Ok(body) => body,
			Err(_) => return Ok(out),
		};
		for line in body.lines().filter(|l| !l.trim().is_empty()) {
			let v: Value = serde_json::from_str(line)?;
			if let Some(r) = v.get("Ref").and_then(Value::as_str).filter(|r| !r.is_empty()) {
				out.insert(Cid::try_from(r)?);
			}
		}
		Ok(out)
	}
}
