// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

#![allow(deprecated)]
// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

#[allow(deprecated)]
use assert_cmd::cargo::cargo_bin;
use std::{process::Command, result::Result};

static RUNTIMES: &[&str] = &["zagros", "pezkuwichain"];

static EXTRINSICS: [(&str, &str); 2] = [("system", "remark"), ("balances", "transfer_keep_alive")];

/// `benchmark extrinsic` works for all dev runtimes and some extrinsics.
#[test]
fn benchmark_extrinsic_works() {
	for runtime in RUNTIMES {
		for (pezpallet, extrinsic) in EXTRINSICS {
			let runtime = format!("{}-dev", runtime);
			// `assert!(x.is_ok())` alone discards the `Err` payload — surface it explicitly
			// so the signal-vs-exit-code diagnosis in `benchmark_extrinsic` is actually visible
			// in the test failure output instead of a bare "assertion failed: ...is_ok()".
			if let Err(e) = benchmark_extrinsic(&runtime, pezpallet, extrinsic) {
				panic!("benchmark_extrinsic({runtime}, {pezpallet}, {extrinsic}) failed: {e}");
			}
		}
	}
}

/// `benchmark extrinsic` rejects all non-dev runtimes.
#[test]
fn benchmark_extrinsic_rejects_non_dev_runtimes() {
	for runtime in RUNTIMES {
		assert!(benchmark_extrinsic(runtime, "system", "remark").is_err());
	}
}

#[allow(deprecated)]
fn benchmark_extrinsic(runtime: &str, pezpallet: &str, extrinsic: &str) -> Result<(), String> {
	let status = Command::new(cargo_bin("pezkuwi"))
		.args(["benchmark", "extrinsic", "--chain", runtime])
		.args(["--pezpallet", pezpallet, "--extrinsic", extrinsic])
		// Run with low repeats for faster execution.
		.args(["--repeat=1", "--warmup=1", "--max-ext-per-block=1"])
		.status()
		.map_err(|e| format!("command failed: {:?}", e))?;

	if !status.success() {
		// `status.success() == false` covers both a clean non-zero exit (real Err/panic
		// bubbling up through `main()`) and process termination by signal (e.g. SIGABRT
		// from an aborting allocator/native library during teardown) — those need very
		// different fixes, so surface which one this is instead of the previous opaque
		// "Command failed".
		#[cfg(unix)]
		{
			use std::os::unix::process::ExitStatusExt;
			if let Some(signal) = status.signal() {
				return Err(format!(
					"Command terminated by signal {signal} (code={:?})",
					status.code()
				));
			}
		}
		return Err(format!("Command failed with exit status {:?}", status.code()));
	}

	Ok(())
}
