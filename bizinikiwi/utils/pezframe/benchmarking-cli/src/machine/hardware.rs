// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Contains types to define hardware requirements.

use pezsc_sysinfo::Requirements;
use std::sync::LazyLock;

/// The hardware requirements as measured on reference hardware.
///
/// These are ours, measured on the machine the weights are generated on: four cores, 8 GB, a
/// Contabo SSD -- the weakest class in the validator fleet. That choice is deliberate. Block
/// production waits on the slowest validator, not the fastest, so a threshold taken from a
/// stronger machine would pass every node while still describing work they cannot finish in
/// time.
///
/// Each figure is the measured score less ten percent, so an ordinary machine of that class
/// passes and a genuinely degraded one does not. Re-take them with `benchmark machine` on the
/// reference host if the fleet moves to different hardware, and re-generate the weights in the
/// same move -- the two belong together.
///
/// The numbers upstream ships are for its own reference hardware, which is roughly twice this
/// on memory and disk. Keeping those here would have meant every node in this fleet reporting
/// a hardware failure on every check, which is noise that hides a real one.
pub static BIZINIKIWI_REFERENCE_HARDWARE: LazyLock<Requirements> = LazyLock::new(|| {
	let raw = include_bytes!("reference_hardware.json").as_slice();
	serde_json::from_slice(raw).expect("Hardcoded data is known good; qed")
});

#[cfg(test)]
mod tests {
	use super::*;
	use pezsc_sysinfo::{Metric, Requirement, Requirements, Throughput};

	/// `BIZINIKIWI_REFERENCE_HARDWARE` can be decoded.
	#[test]
	fn json_static_data() {
		let raw = serde_json::to_string(&*BIZINIKIWI_REFERENCE_HARDWARE).unwrap();
		let decoded: Requirements = serde_json::from_str(&raw).unwrap();

		assert_eq!(decoded, BIZINIKIWI_REFERENCE_HARDWARE.clone());
	}

	/// The hard-coded values are correct.
	#[test]
	fn json_static_data_is_correct() {
		assert_eq!(
			*BIZINIKIWI_REFERENCE_HARDWARE,
			Requirements(vec![
				Requirement {
					metric: Metric::Blake2256,
					minimum: Throughput::from_mibs(1000.00),
					validator_only: false
				},
				Requirement {
					metric: Metric::Blake2256Parallel { num_cores: 8 },
					minimum: Throughput::from_mibs(1000.00),
					validator_only: true,
				},
				Requirement {
					metric: Metric::Sr25519Verify,
					minimum: Throughput::from_kibs(637.619999744),
					validator_only: false
				},
				Requirement {
					metric: Metric::MemCopy,
					minimum: Throughput::from_gibs(11.4925205078125003),
					validator_only: false,
				},
				Requirement {
					metric: Metric::DiskSeqWrite,
					minimum: Throughput::from_mibs(950.0),
					validator_only: false,
				},
				Requirement {
					metric: Metric::DiskRndWrite,
					minimum: Throughput::from_mibs(420.0),
					validator_only: false
				},
			])
		);
	}
}
