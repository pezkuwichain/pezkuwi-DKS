//! kurd-mirror: keeps an IPFS node that serves `.kurd` sites in step with the register.
//!
//! `deny` refuses, through kubo's denylist, the content of every name the court or the
//! emergency council has suspended and of every name the court has revoked (D23). The wallet
//! already stops showing such a name -- it reads `Suspensions` beside `Names` -- so this is about
//! what our own servers hand out to anyone who asks by CID.
//!
//! One run is one pass; a systemd timer repeats it (`deploy/`).

mod chain;
mod denylist;
mod kubo;
mod register;

use std::{collections::BTreeSet, fs, path::PathBuf, process::Command};

use anyhow::{bail, Context, Result};
use cid::{multibase::Base, Cid};
use clap::{Parser, Subcommand};

use register::{Root, Seen};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
	#[command(subcommand)]
	command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
	/// Write the denylist from the register, once.
	Deny {
		/// HTTP JSON-RPC of a People node.
		#[arg(long)]
		rpc: String,
		/// kubo's RPC API.
		#[arg(long, default_value = "http://127.0.0.1:5001")]
		kubo: String,
		/// The denylist file this tool owns.
		#[arg(long, default_value = "/etc/ipfs/denylists/kurd-register.deny")]
		denylist: PathBuf,
		/// What the previous runs saw; needed to know what a revoked name served.
		#[arg(long, default_value = "/var/lib/kurd-mirror/seen.json")]
		state: PathBuf,
		/// Run when a rule has to be removed, which kubo picks up only on restart.
		#[arg(long, default_value = "systemctl restart ipfs")]
		restart: String,
	},
}

#[tokio::main]
async fn main() -> Result<()> {
	match Cli::parse().command {
		Cmd::Deny { rpc, kubo, denylist, state, restart } => {
			deny(&rpc, &kubo, &denylist, &state, &restart).await
		},
	}
}

async fn deny(
	rpc: &str,
	kubo: &str,
	denylist: &PathBuf,
	state: &PathBuf,
	restart: &str,
) -> Result<()> {
	let mut seen: Seen = match fs::read(state) {
		Ok(bytes) => {
			serde_json::from_slice(&bytes).with_context(|| format!("{}", state.display()))?
		},
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => Seen::default(),
		Err(e) => return Err(e).with_context(|| format!("{}", state.display())),
	};
	let (at, snap) = chain::Rpc::new(rpc).snapshot().await?;
	let refused = register::decide(&snap, &mut seen);
	// Saved before the denylist is touched: a revocation is known only from what was seen, and
	// must survive a run that fails further on.
	save(state, &seen)?;

	let rules = rules_for(&kubo::Kubo::new(kubo), &refused).await?;
	let restart_needed = denylist::apply(denylist, &rules)?;
	if restart_needed {
		let mut words = restart.split_whitespace();
		let Some(program) = words.next() else { bail!("--restart is empty") };
		let status = Command::new(program).args(words).status()?;
		if !status.success() {
			bail!("{restart}: {status}");
		}
	}
	println!(
		"block {at}: {} names, {} suspensions, {} refused roots, {} rules{}",
		snap.names.len(),
		snap.suspensions.len(),
		refused.len(),
		rules.len(),
		if restart_needed { ", daemon restarted" } else { "" },
	);
	Ok(())
}

/// Every block of refused content this node holds, less the blocks it keeps for anything else.
/// A rule stops only the CID it names -- a child block asked for by its own CID is still served
/// -- so the whole DAG is listed. A block shared with a site that still resolves, a common
/// script say, is left out, or refusing one site would break the other.
async fn rules_for(kubo: &kubo::Kubo, refused: &BTreeSet<Root>) -> Result<BTreeSet<String>> {
	let mut rules = BTreeSet::new();
	if refused.is_empty() {
		return Ok(rules);
	}
	let mut refused_roots = Vec::new();
	for root in refused {
		match root {
			Root::Ipfs(bytes) => refused_roots.push(Cid::try_from(bytes.as_slice())?),
			Root::Ipns(bytes) => {
				let name = Cid::try_from(bytes.as_slice())?.to_string_of_base(Base::Base36Lower)?;
				rules.insert(format!("/ipns/{name}"));
			},
		}
	}
	let refused_hashes: BTreeSet<_> = refused_roots.iter().map(|c| *c.hash()).collect();

	let mut kept = BTreeSet::new();
	for pin in kubo.pins().await? {
		if !refused_hashes.contains(pin.hash()) {
			kept.extend(kubo.blocks(&pin).await?.iter().map(|c| *c.hash()));
		}
	}
	for root in &refused_roots {
		for block in kubo.blocks(root).await? {
			if !kept.contains(block.hash()) {
				rules.insert(format!("/ipfs/{block}"));
			}
		}
	}
	Ok(rules)
}

fn save(path: &PathBuf, seen: &Seen) -> Result<()> {
	if let Some(dir) = path.parent() {
		fs::create_dir_all(dir)?;
	}
	let tmp = path.with_extension("json.tmp");
	fs::write(&tmp, serde_json::to_vec_pretty(seen)?)?;
	fs::rename(&tmp, path)?;
	Ok(())
}
