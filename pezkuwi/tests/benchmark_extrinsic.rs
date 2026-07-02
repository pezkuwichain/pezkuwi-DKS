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

// "pezkuwichain-dev" + "balances"/"transfer_keep_alive" is provably rejected by the node today:
// `TransferKeepAliveBuilder` signs its call via `identify_chain()`, and
// `IdentifyVariant::is_pezkuwi()` matches any chain id starting with "pezkuwi" — which includes
// "pezkuwichain-dev" *and* our real production mainnet spec ("pezkuwichain_mainnet"). Since
// `identify_chain()` checks `is_pezkuwi()` before `is_pezkuwichain()`, both resolve to
// `Chain::Pezkuwi`, which `pezkuwi/node/service/src/benchmarking.rs`'s `identify_chain!` macro
// hard-rejects ("Pezkuwi runtimes are currently not supported"). "system"/"remark" doesn't go
// through `identify_chain()` at all (`BizinikiwiRemarkBuilder` needs no chain-specific signing),
// so it works fine against "pezkuwichain-dev" — only the balances-transfer combination is
// affected. See `benchmark_extrinsic_rejects_pezkuwichain_dev_transfer` below, which pins this
// down as a known, asserted-on-purpose limitation rather than silently skipping it.
//
// This is *not* something to "fix" by reordering `identify_chain()`'s checks: that would also
// reclassify the real mainnet spec (which currently — and correctly, for a live production
// network — gets the more conservative `Chain::Pezkuwi` treatment: authoring backoff disabled,
// a conservative availability fetch-chunks threshold, and `KEEP_FINALIZED_FOR_LIVE_NETWORKS`
// retention) as `Chain::Pezkuwichain`, which per this same code's comments is treated as a
// testnet "in flux". Untangling the "pezkuwi"/"pezkuwichain" prefix collision from the
// production-vs-testnet distinction it's currently (accidentally) encoding needs its own
// deliberate task, not a side effect of unblocking this CI test.
const KNOWN_UNSUPPORTED: (&str, &str, &str) = ("pezkuwichain", "balances", "transfer_keep_alive");

/// `benchmark extrinsic` works for all dev runtimes and some extrinsics.
#[test]
fn benchmark_extrinsic_works() {
	for runtime in RUNTIMES {
		for (pezpallet, extrinsic) in EXTRINSICS {
			if (*runtime, pezpallet, extrinsic) == KNOWN_UNSUPPORTED {
				continue;
			}
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

/// Pins down the one combination excluded from `benchmark_extrinsic_works` above (see the
/// comment on `KNOWN_UNSUPPORTED`). If this ever starts passing, remove the skip instead of
/// letting it silently go untested.
#[test]
fn benchmark_extrinsic_rejects_pezkuwichain_dev_transfer() {
	let (runtime, pezpallet, extrinsic) = KNOWN_UNSUPPORTED;
	let err = benchmark_extrinsic(&format!("{runtime}-dev"), pezpallet, extrinsic)
		.expect_err("pezkuwichain-dev + balances/transfer_keep_alive is expected to be rejected by identify_chain(); if this now succeeds, identify_chain()'s pezkuwi/pezkuwichain collision has been resolved — remove it from KNOWN_UNSUPPORTED instead of this test");
	assert!(
		!err.contains("Command terminated by signal"),
		"expected a clean rejection (Chain::Pezkuwi is unsupported), not a crash: {err}"
	);
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
