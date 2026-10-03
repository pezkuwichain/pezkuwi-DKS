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
//! SHORT-NAME: `block`, LONG-NAME: `BlockExecution`, RUNTIME: `asset-hub-pezkuwichain`
//! WARMUPS: `10`, REPEAT: `100`
//! WEIGHT-PATH: `./pezcumulus/teyrchains/runtimes/assets/asset-hub-pezkuwichain/src/weights/`
//! WEIGHT-METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   /home/runner/.cargo/bin/pezframe-omni-bencher
//   v1
//   benchmark
//   overhead
//   --runtime
//   target/production/wbuild/asset-hub-pezkuwichain-runtime/asset_hub_pezkuwichain_runtime.wasm
//   --weight-path
//   ./pezcumulus/teyrchains/runtimes/assets/asset-hub-pezkuwichain/src/weights/
//   --header
//   ./pezcumulus/file_header.txt
//   --warmup
//   10
//   --repeat
//   100
//   --para-id
//   1000

use pezsp_core::parameter_types;
use pezsp_weights::{constants::WEIGHT_REF_TIME_PER_NANOS, Weight};

parameter_types! {
	/// Weight of executing an empty block.
	/// Calculated by multiplying the *Average* with `1.0` and adding `0`.
	///
	/// Stats nanoseconds:
	///   Min, Max: 2_507_849, 11_632_375
	///   Average:  3_901_660
	///   Median:   3_837_016
	///   Std-Dev:  988341.61
	///
	/// Percentiles nanoseconds:
	///   99th: 5_447_019
	///   95th: 4_846_760
	///   75th: 4_221_895
	pub const BlockExecutionWeight: Weight =
		Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(3_901_660), 77_799);
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
