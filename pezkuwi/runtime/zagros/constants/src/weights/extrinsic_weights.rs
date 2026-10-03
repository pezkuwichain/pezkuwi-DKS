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
//! DATE: 2026-09-27 (Y/M/D)
//! HOSTNAME: `vmi3220280`, CPU: `AMD EPYC Processor (with IBPB)`
//!
//! SHORT-NAME: `extrinsic`, LONG-NAME: `ExtrinsicBase`, RUNTIME: `zagros`
//! WARMUPS: `10`, REPEAT: `100`
//! WEIGHT-PATH: `./pezkuwi/runtime/zagros/constants/src/weights/`
//! WEIGHT-METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   /home/runner/.cargo/bin/pezframe-omni-bencher
//   v1
//   benchmark
//   overhead
//   --runtime
//   target/production/wbuild/zagros-runtime/zagros_runtime.wasm
//   --weight-path
//   ./pezkuwi/runtime/zagros/constants/src/weights/
//   --header
//   ./pezkuwi/file_header.txt
//   --warmup
//   10
//   --repeat
//   100

use pezsp_core::parameter_types;
use pezsp_weights::{constants::WEIGHT_REF_TIME_PER_NANOS, Weight};

parameter_types! {
	/// Weight of executing a NO-OP extrinsic, for example `System::remark`.
	/// Calculated by multiplying the *Average* with `1.0` and adding `0`.
	///
	/// Stats nanoseconds:
	///   Min, Max: 194_338, 306_820
	///   Average:  224_288
	///   Median:   219_478
	///   Std-Dev:  18787.28
	///
	/// Percentiles nanoseconds:
	///   99th: 281_131
	///   95th: 261_213
	///   75th: 230_764
	pub const ExtrinsicBaseWeight: Weight =
		Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(224_288), 0);
}

#[cfg(test)]
mod test_weights {
	use pezsp_weights::constants;

	/// Checks that the weight exists and is sane.
	// NOTE: If this test fails but you are sure that the generated values are fine,
	// you can delete it.
	#[test]
	fn sane() {
		let w = super::ExtrinsicBaseWeight::get();

		// At least 10 µs.
		assert!(
			w.ref_time() >= 10u64 * constants::WEIGHT_REF_TIME_PER_MICROS,
			"Weight should be at least 10 µs."
		);
		// At most 1 ms.
		assert!(
			w.ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
			"Weight should be at most 1 ms."
		);
	}
}
