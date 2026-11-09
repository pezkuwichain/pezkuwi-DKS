//! The denylist file kubo reads (IPIP-383, `/etc/ipfs/denylists/*.deny`).
//!
//! Measured on kubo 0.43.1: a rule appended to a file the daemon is already watching takes
//! effect at once; a new file, or a rule removed, needs the daemon restarted. So the file is
//! appended to whenever that is enough, and rewritten (and the daemon restarted) only when a
//! rule has to go -- a suspension lifted or run out.

use std::{collections::BTreeSet, fs, io::Write, path::Path};

use anyhow::{Context, Result};

pub const HEADER: &str = "version: 1\nname: kurd-register\ndescription: .kurd content the register has stopped; written by kurd-mirror, do not edit\n---\n";

#[derive(Debug, PartialEq, Eq)]
pub enum Change {
	None,
	/// Only new rules: append them, the daemon picks them up.
	Append(Vec<String>),
	/// A rule has to go, or the file is new: write it whole and restart the daemon.
	Rewrite,
}

/// The rules in a denylist file, header and comments left out.
pub fn rules_in(text: &str) -> BTreeSet<String> {
	let body = text.split_once("\n---\n").map_or(text, |(_, b)| b);
	body.lines()
		.map(str::trim)
		.filter(|l| !l.is_empty() && !l.starts_with('#'))
		.map(str::to_owned)
		.collect()
}

pub fn render(rules: &BTreeSet<String>) -> String {
	let mut out = HEADER.to_owned();
	for r in rules {
		out.push_str(r);
		out.push('\n');
	}
	out
}

pub fn change(current: Option<&str>, wanted: &BTreeSet<String>) -> Change {
	let Some(current) = current else { return Change::Rewrite };
	let have = rules_in(current);
	if !have.is_subset(wanted) || !current.starts_with(HEADER) {
		return Change::Rewrite;
	}
	let new: Vec<String> = wanted.difference(&have).cloned().collect();
	if new.is_empty() {
		Change::None
	} else {
		Change::Append(new)
	}
}

/// Makes `path` hold `wanted`. Returns whether the daemon has to be restarted.
pub fn apply(path: &Path, wanted: &BTreeSet<String>) -> Result<bool> {
	let current = match fs::read_to_string(path) {
		Ok(text) => Some(text),
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
		Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
	};
	match change(current.as_deref(), wanted) {
		Change::None => Ok(false),
		Change::Append(rules) => {
			// The daemon follows the file it opened; replacing the file would leave it reading
			// the old one, so this has to be a write to the same inode.
			let mut f = fs::OpenOptions::new().append(true).open(path)?;
			if !current.as_deref().unwrap_or_default().ends_with('\n') {
				f.write_all(b"\n")?;
			}
			for r in rules {
				writeln!(f, "{r}")?;
			}
			f.sync_all()?;
			Ok(false)
		},
		Change::Rewrite => {
			let tmp = path.with_extension("deny.tmp");
			fs::write(&tmp, render(wanted))?;
			fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
			Ok(true)
		},
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn set(rules: &[&str]) -> BTreeSet<String> {
		rules.iter().map(|r| r.to_string()).collect()
	}

	#[test]
	fn new_rules_are_appended_and_a_dropped_one_rewrites() {
		let file = render(&set(&["/ipfs/a"]));
		assert_eq!(change(Some(&file), &set(&["/ipfs/a"])), Change::None);
		assert_eq!(
			change(Some(&file), &set(&["/ipfs/a", "/ipfs/b"])),
			Change::Append(vec!["/ipfs/b".into()])
		);
		assert_eq!(change(Some(&file), &set(&["/ipfs/b"])), Change::Rewrite);
		assert_eq!(change(None, &set(&[])), Change::Rewrite);
		assert_eq!(change(Some("/ipfs/a\n"), &set(&["/ipfs/a"])), Change::Rewrite);
	}

	#[test]
	fn appending_keeps_the_inode_and_rewriting_asks_for_a_restart() {
		let dir = tempfile::tempdir().unwrap();
		let path = dir.path().join("kurd.deny");
		assert!(apply(&path, &set(&["/ipfs/a"])).unwrap());

		#[cfg(unix)]
		let inode = || std::os::unix::fs::MetadataExt::ino(&fs::metadata(&path).unwrap());
		#[cfg(unix)]
		let before = inode();
		assert!(!apply(&path, &set(&["/ipfs/a", "/ipfs/b"])).unwrap());
		#[cfg(unix)]
		assert_eq!(inode(), before);
		assert_eq!(rules_in(&fs::read_to_string(&path).unwrap()), set(&["/ipfs/a", "/ipfs/b"]));

		assert!(apply(&path, &set(&["/ipfs/b"])).unwrap());
		assert_eq!(fs::read_to_string(&path).unwrap(), render(&set(&["/ipfs/b"])));
	}
}
