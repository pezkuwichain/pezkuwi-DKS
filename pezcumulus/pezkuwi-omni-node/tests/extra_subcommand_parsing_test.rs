// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

#![allow(deprecated)]

/// Integration tests that spawn the actual binary `pezkuwi-omni-node`
/// using `assert_cmd`. We verify that the help text
/// excludes the `export-chain-spec` sub‑command exactly as intended
use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;

#[test]
fn pezkuwi_omni_node_help_excludes_export_chain_spec() {
	// Run `pezkuwi-omni-node --help` and capture stdout.
	let output = Command::new(cargo_bin("pezkuwi-omni-node"))
		.arg("--help")
		.assert()
		.success()
		.get_output()
		.stdout
		.clone();

	let help_text = String::from_utf8_lossy(&output);
	assert!(
		!help_text.contains("export-chain-spec"),
		"`pezkuwi-omni-node --help` must NOT list the \"export-chain-spec\" subcommand"
	);
}
