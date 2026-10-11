// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Provides functionality to embed a [`RuntimeVersion`](crate::RuntimeVersion) as custom section
//! into a WASM file.

use alloc::borrow::Cow;
use codec::Encode;
use wasm_encoder::{CustomSection, RawSection};
use wasmparser::{Encoding, Parser, Payload};

#[derive(Clone, Copy, Eq, PartialEq, Debug, thiserror::Error)]
pub enum Error {
	#[error("Deserializing wasm failed")]
	Deserialize,
	#[error("Serializing wasm failed")]
	Serialize,
}

/// Embed the given `version` to the given `wasm` blob.
///
/// If there was already a runtime version embedded, this will be overwritten.
///
/// Returns the new WASM blob.
pub fn embed_runtime_version(
	wasm: &[u8],
	mut version: crate::RuntimeVersion,
) -> Result<Vec<u8>, Error> {
	let apis = version
		.apis
		.iter()
		.map(Encode::encode)
		.flat_map(|v| v.into_iter())
		.collect::<Vec<u8>>();

	version.apis.to_mut().clear();
	set_custom_sections(wasm, [("runtime_apis", apis), ("runtime_version", version.encode())])
}

/// Sets each `(name, contents)` custom section of `wasm`, in order: the first custom section
/// with that name gets the new contents where it is, and if there is none the section is
/// appended at the end of the module. Every other section is copied byte for byte.
///
/// The module is read section by section and its contents are not validated: whether it can
/// run is decided by the executor that runs it, not here.
fn set_custom_sections<const N: usize>(
	wasm: &[u8],
	sections: [(&'static str, Vec<u8>); N],
) -> Result<Vec<u8>, Error> {
	enum Section<'a> {
		Custom { name: &'a str, data: Cow<'a, [u8]> },
		Raw { id: u8, data: &'a [u8] },
	}

	let mut module = Vec::new();
	for payload in Parser::new(0).parse_all(wasm) {
		match payload.map_err(|_| Error::Deserialize)? {
			Payload::Version { num: 1, encoding: Encoding::Module, .. } => {},
			Payload::CustomSection(custom) => {
				module.push(Section::Custom { name: custom.name(), data: custom.data().into() })
			},
			Payload::End(_) => {},
			// Copied as part of the code section.
			Payload::CodeSectionEntry(_) => {},
			payload => match payload.as_section() {
				// The core module sections, type (1) to data count (12).
				Some((id @ 1..=12, range)) => module.push(Section::Raw { id, data: &wasm[range] }),
				// A component, a component or unknown section, or a tag section.
				_ => return Err(Error::Deserialize),
			},
		}
	}

	for (name, contents) in sections {
		let existing = module.iter_mut().find_map(|section| match section {
			Section::Custom { name: existing, data } if *existing == name => Some(data),
			_ => None,
		});
		match existing {
			Some(data) => *data = contents.into(),
			None => module.push(Section::Custom { name, data: contents.into() }),
		}
	}

	let mut encoder = wasm_encoder::Module::new();
	for section in &module {
		match section {
			Section::Custom { name, data } => {
				encoder.section(&CustomSection { name: (*name).into(), data: data.clone() })
			},
			Section::Raw { id, data } => encoder.section(&RawSection { id: *id, data }),
		};
	}
	Ok(encoder.finish())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	/// The payloads the recorded outputs were produced with.
	const APIS: &[u8] = &[0xdf, 0x6a, 0xcb, 0x68, 0x99, 0x07, 0x60, 0x9b, 2, 0, 0, 0];
	const VERSION: &[u8] = &[0x10, b't', b'e', b's', b't', 1, 0, 0, 0];

	fn embed(wasm: &[u8]) -> Result<Vec<u8>, Error> {
		set_custom_sections(
			wasm,
			[("runtime_apis", APIS.to_vec()), ("runtime_version", VERSION.to_vec())],
		)
	}

	/// `tests/embed-vectors.txt` holds what the previous, parity-wasm based implementation
	/// produced for the modules under the executor's `tests/vectors`: the output in hex, or
	/// `ERR`.
	#[test]
	fn matches_recorded_outputs() {
		let root = Path::new(env!("CARGO_MANIFEST_DIR"));
		let vectors = root.join("../../client/executor/common/tests/vectors");
		let manifest = std::fs::read_to_string(root.join("tests/embed-vectors.txt")).unwrap();
		let mut checked = 0;
		for line in manifest.lines() {
			let mut fields = line.split(' ');
			let (name, expected) = (fields.next().unwrap(), fields.next().unwrap());
			let input = std::fs::read(vectors.join(format!("{name}.wasm"))).unwrap();
			let actual = embed(&input).map(hex::encode).unwrap_or_else(|_| "ERR".into());
			// parity-wasm refused modules using post-MVP instructions or types. Reading section
			// by section does not look inside them; the executor still refuses to run them.
			if expected == "ERR" && name.starts_with("rejected-") {
				assert_ne!(actual, "ERR", "{name} is well-formed");
				continue;
			}
			assert_eq!(actual, expected, "{name}");
			checked += 1;
		}
		assert_eq!(checked, 8);
	}

	/// The same for the kitchensink runtime committed in the tree, by digest.
	#[test]
	fn kitchensink_matches_recorded_output() {
		use sha2::Digest;
		let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join(
			"../../pezframe/system/benchmarking/res/pez_kitchensink_runtime.compact.compressed.wasm",
		);
		let output = embed(&std::fs::read(runtime).unwrap()).unwrap();
		assert_eq!(
			format!("{}:{}", output.len(), hex::encode(sha2::Sha256::digest(&output))),
			"17072671:53e567a650bdc13fd1fc708c7e719ab07e921e8916c34133aadc0c215808f716",
		);
	}

	#[test]
	fn replaces_in_place_and_appends() {
		// `(module (memory 1))` with a `runtime_version` custom section before the memory.
		let mut module = b"\0asm\x01\0\0\0".to_vec();
		module.extend([0, 17, 15]);
		module.extend(b"runtime_version");
		module.push(0xaa);
		module.extend([5, 3, 1, 0, 1]);

		let embedded = embed(&module).unwrap();
		let mut expected = b"\0asm\x01\0\0\0".to_vec();
		expected.extend([0, 25, 15]);
		expected.extend(b"runtime_version");
		expected.extend(VERSION);
		expected.extend([5, 3, 1, 0, 1]);
		expected.extend([0, 25, 12]);
		expected.extend(b"runtime_apis");
		expected.extend(APIS);
		assert_eq!(embedded, expected);
		// Embedding again replaces both where they are.
		assert_eq!(embed(&embedded).unwrap(), embedded);
	}
}
