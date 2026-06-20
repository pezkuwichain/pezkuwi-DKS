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

#![allow(deprecated)]

use assert_cmd::cargo::cargo_bin;
use std::process::Command;
use tempfile::tempdir;

/// Tests that the `benchmark extrinsic` command works for
/// remark and transfer_keep_alive within the bizinikiwi dev runtime.
#[test]
fn benchmark_extrinsic_works() {
	benchmark_extrinsic("system", "remark");
	benchmark_extrinsic("balances", "transfer_keep_alive");
}

/// Checks that the `benchmark extrinsic` command works for the given pezpallet and extrinsic.
fn benchmark_extrinsic(pezpallet: &str, extrinsic: &str) {
	let base_dir = tempdir().expect("could not create a temp dir");

	let status = Command::new(cargo_bin("bizinikiwi-node"))
		.args(&["benchmark", "extrinsic", "--dev"])
		.arg("-d")
		.arg(base_dir.path())
		.args(&["--pezpallet", pezpallet, "--extrinsic", extrinsic])
		// Run with low repeats for faster execution.
		.args(["--warmup=10", "--repeat=10", "--max-ext-per-block=10"])
		.args(["--wasm-execution=compiled"])
		.status()
		.unwrap();

	assert!(status.success());
}
