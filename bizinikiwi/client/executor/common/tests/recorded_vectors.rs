// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Holds `RuntimeBlob` to the outputs of the `parity-wasm` / `wasm-instrument` implementation
//! it replaced.
//!
//! `tests/vectors/expected.txt` and `tests/vectors/kitchensink.txt` were written by running
//! the previous implementation on the modules in `tests/vectors` (compiled from the `.wat`
//! files next to them) and on the kitchensink runtime committed under
//! `pezframe/system/benchmarking/res`. Each line is `<input> <operation> <result>`, where the
//! result is the output module in hex, its SHA-256 (`sha256:<len>:<digest>`), `ERR` when the
//! old code returned an error, or the custom section contents / `NONE`.
//!
//! The `pvf` operation is what a teyrchain validation function goes through before it is
//! compiled: stack-height metering, then the memory import turned into an export and its
//! maximum cleared. Its output decides when a candidate traps, so equality here is byte
//! equality, not just equivalent behaviour.

#![allow(deprecated)]

use pezsc_executor_common::{runtime_blob::RuntimeBlob, wasm_runtime::HeapAllocStrategy};
use sha2::Digest;
use std::path::{Path, PathBuf};

fn vectors_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors")
}

/// Runs `operation` on `input` the way the recorder ran it on the old implementation.
fn run(input: &[u8], operation: &str) -> Option<Result<Vec<u8>, ()>> {
	let blob = || RuntimeBlob::new(input).map_err(drop);
	let finish = |mut blob: RuntimeBlob| -> Result<RuntimeBlob, ()> {
		blob.convert_memory_import_into_export().map_err(drop)?;
		blob.clear_memory_max_limit().map_err(drop)?;
		Ok(blob)
	};
	let heap = |strategy| -> Result<Vec<u8>, ()> {
		let mut blob = blob()?;
		blob.convert_memory_import_into_export().map_err(drop)?;
		blob.setup_memory_according_to_heap_alloc_strategy(strategy).map_err(drop)?;
		Ok(blob.serialize())
	};
	Some(match operation {
		"roundtrip" => blob().map(RuntimeBlob::serialize),
		"plain" => blob().and_then(finish).map(RuntimeBlob::serialize),
		"pvf" => blob()
			.and_then(|b| b.inject_stack_depth_metering(65536).map_err(drop))
			.and_then(finish)
			.map(RuntimeBlob::serialize),
		"stack1024" => blob()
			.and_then(|b| b.inject_stack_depth_metering(1024).map_err(drop))
			.map(RuntimeBlob::serialize),
		"heap_dyn_some" => heap(HeapAllocStrategy::Dynamic { maximum_pages: Some(1) }),
		"heap_dyn_none" => heap(HeapAllocStrategy::Dynamic { maximum_pages: None }),
		"heap_static" => heap(HeapAllocStrategy::Static { extra_pages: 5 }),
		_ => return None,
	})
}

fn render(output: Result<Vec<u8>, ()>, digest: bool) -> String {
	match output {
		Err(()) => "ERR".into(),
		Ok(bytes) if digest => {
			format!("sha256:{}:{}", bytes.len(), hex::encode(sha2::Sha256::digest(&bytes)))
		},
		Ok(bytes) => hex::encode(bytes),
	}
}

fn custom_section(input: &[u8]) -> String {
	match RuntimeBlob::new(input) {
		Err(_) => "ERR".into(),
		Ok(blob) => blob
			.custom_section_contents("runtime_version")
			.map_or("NONE".into(), hex::encode),
	}
}

/// Checks every line of `manifest` against the current implementation; returns how many were
/// checked.
fn check(manifest: &str, load: impl Fn(&str) -> Vec<u8>, digest: bool) -> usize {
	let mut checked = 0;
	let mut mismatches = Vec::new();
	let mut inputs = std::collections::HashMap::new();
	for line in manifest.lines().filter(|l| !l.is_empty()) {
		let mut fields = line.splitn(3, ' ');
		let (name, operation, expected) =
			(fields.next().unwrap(), fields.next().unwrap(), fields.next().unwrap());
		let input = inputs.entry(name.to_owned()).or_insert_with(|| load(name));
		let actual = match operation {
			"custom_rv" => custom_section(input),
			_ => match run(input, operation) {
				Some(output) => render(output, digest),
				// Operations of other crates, recorded in the same run.
				None => continue,
			},
		};
		if actual != expected {
			mismatches
				.push(format!("{name} {operation}:\n  expected {expected}\n  actual   {actual}"));
		}
		checked += 1;
	}
	assert!(mismatches.is_empty(), "differs from the recorded output:\n{}", mismatches.join("\n"));
	checked
}

#[test]
fn small_modules_match_recorded_outputs() {
	let manifest = std::fs::read_to_string(vectors_dir().join("expected.txt")).unwrap();
	let checked = check(
		&manifest,
		|name| std::fs::read(vectors_dir().join(format!("{name}.wasm"))).unwrap(),
		false,
	);
	assert!(checked > 100, "only {checked} vectors checked");
}

#[test]
fn kitchensink_runtime_matches_recorded_outputs() {
	let manifest = std::fs::read_to_string(vectors_dir().join("kitchensink.txt")).unwrap();
	let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join(
		"../../../pezframe/system/benchmarking/res/pez_kitchensink_runtime.compact.compressed.wasm",
	);
	let checked = check(
		&manifest,
		|_| {
			let blob = std::fs::read(&runtime).unwrap();
			pezsp_maybe_compressed_blob::decompress(&blob, usize::MAX).unwrap().into_owned()
		},
		true,
	);
	assert_eq!(checked, 8);
}
