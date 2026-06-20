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

// bizinikiwi-wasm-builder moved to integration tests to break circular dependency
// This build script creates a dummy wasm_binary.rs for std builds

fn main() {
	#[cfg(feature = "std")]
	{
		use std::io::Write;
		let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
		let wasm_binary_path = std::path::Path::new(&out_dir).join("wasm_binary.rs");

		let content = r#"
/// Wasm binary unwrap bloat.
#[cfg(all(feature = "std", not(feature = "runtime-benchmarks")))]
pub const WASM_BINARY: Option<&[u8]> = None;

/// Wasm binary unwrap bloat (for runtime-benchmarks feature).
#[cfg(all(feature = "std", feature = "runtime-benchmarks"))]
pub const WASM_BINARY: Option<&[u8]> = None;

/// Wasm binary unwrap bloat.
#[allow(dead_code)]
pub const WASM_BINARY_BLOATY: Option<&[u8]> = None;
"#;

		let mut file =
			std::fs::File::create(&wasm_binary_path).expect("Failed to create wasm_binary.rs");
		file.write_all(content.as_bytes()).expect("Failed to write wasm_binary.rs");
	}
}
