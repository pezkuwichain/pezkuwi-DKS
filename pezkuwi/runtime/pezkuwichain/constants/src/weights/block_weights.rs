// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Pezkuwi.

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

//! THIS FILE WAS AUTO-GENERATED USING THE BIZINIKIWI BENCHMARK CLI VERSION 32.0.1
//! DATE: 2026-08-28 (Y/M/D)
//! HOSTNAME: `vmi3519825`, CPU: `AMD EPYC Processor (with IBPB)`
//!
//! SHORT-NAME: `block`, LONG-NAME: `BlockExecution`, RUNTIME: `pezkuwichain`
//! WARMUPS: `10`, REPEAT: `100`
//! WEIGHT-PATH: `./pezkuwi/runtime/pezkuwichain/constants/src/weights/`
//! WEIGHT-METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   ./target/production/pezframe-omni-bencher
//   v1
//   benchmark
//   overhead
//   --runtime
//   target/production/wbuild/pezkuwichain-runtime/pezkuwichain_runtime.wasm
//   --genesis-builder
//   runtime
//   --genesis-builder-preset
//   development
//   --wasm-execution
//   compiled
//   --weight-path
//   ./pezkuwi/runtime/pezkuwichain/constants/src/weights/
//   --header
//   ./pezkuwi/file_header.txt
//   --warmup
//   10
//   --repeat
//   100

use pezsp_core::parameter_types;
use pezsp_weights::{constants::WEIGHT_REF_TIME_PER_NANOS, Weight};

parameter_types! {
	/// Weight of executing an empty block.
	/// Calculated by multiplying the *Average* with `1.0` and adding `0`.
	///
	/// Stats nanoseconds:
	///   Min, Max: 1_125_887, 7_782_229
	///   Average:  1_895_036
	///   Median:   1_483_444
	///   Std-Dev:  1159043.44
	///
	/// Percentiles nanoseconds:
	///   99th: 6_892_351
	///   95th: 4_113_072
	///   75th: 1_936_699
	pub const BlockExecutionWeight: Weight =
		Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(1_895_036), 0);
}

#[cfg(test)]
mod test_weights {
	use pezsp_weights::constants;

	/// Checks that the weight exists and is sane.
	// NOTE: If this test fails but you are sure that the generated values are fine,
	// you can delete it.
	#[test]
	fn sane() {
		let w = super::BlockExecutionWeight::get();

		// At least 100 µs.
		assert!(
			w.ref_time() >= 100u64 * constants::WEIGHT_REF_TIME_PER_MICROS,
			"Weight should be at least 100 µs."
		);
		// At most 50 ms.
		assert!(
			w.ref_time() <= 50u64 * constants::WEIGHT_REF_TIME_PER_MILLIS,
			"Weight should be at most 50 ms."
		);
	}
}
