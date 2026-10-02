// Copyright (C) Parity Technologies (UK) Ltd.
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

//! THIS FILE WAS AUTO-GENERATED USING THE BIZINIKIWI BENCHMARK CLI VERSION 32.0.1
//! DATE: 2026-09-27 (Y/M/D)
//! HOSTNAME: `vmi3220280`, CPU: `AMD EPYC Processor (with IBPB)`
//!
//! SHORT-NAME: `block`, LONG-NAME: `BlockExecution`, RUNTIME: `coretime-pezkuwichain`
//! WARMUPS: `10`, REPEAT: `100`
//! WEIGHT-PATH: `./pezcumulus/teyrchains/runtimes/coretime/coretime-pezkuwichain/src/weights/`
//! WEIGHT-METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   /home/runner/.cargo/bin/pezframe-omni-bencher
//   v1
//   benchmark
//   overhead
//   --runtime
//   target/production/wbuild/coretime-pezkuwichain-runtime/coretime_pezkuwichain_runtime.wasm
//   --weight-path
//   ./pezcumulus/teyrchains/runtimes/coretime/coretime-pezkuwichain/src/weights/
//   --header
//   ./pezcumulus/file_header.txt
//   --warmup
//   10
//   --repeat
//   100
//   --para-id
//   1005

use pezsp_core::parameter_types;
use pezsp_weights::{constants::WEIGHT_REF_TIME_PER_NANOS, Weight};

parameter_types! {
	/// Weight of executing an empty block.
	/// Calculated by multiplying the *Average* with `1.0` and adding `0`.
	///
	/// Stats nanoseconds:
	///   Min, Max: 662_004, 3_791_779
	///   Average:  1_014_976
	///   Median:   930_795
	///   Std-Dev:  380392.0
	///
	/// Percentiles nanoseconds:
	///   99th: 2_543_011
	///   95th: 1_338_044
	///   75th: 984_555
	pub const BlockExecutionWeight: Weight =
		Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(1_014_976), 2_397);
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
