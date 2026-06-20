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

//! Benchmarks for the BABE Pezpallet.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use pezframe_benchmarking::v2::*;

type Header = pezsp_runtime::generic::Header<u64, pezsp_runtime::traits::BlakeTwo256>;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn check_equivocation_proof(x: Linear<0, 1>) {
		// NOTE: generated with the test below `test_generate_equivocation_report_blob`.
		// the output is not deterministic since keys are generated randomly (and therefore
		// signature content changes). it should not affect the benchmark.
		// with the current benchmark setup it is not possible to generate this programmatically
		// from the benchmark setup.
		const EQUIVOCATION_PROOF_BLOB: [u8; 416] = [
			222, 241, 46, 66, 243, 228, 135, 233, 177, 64, 149, 170, 141, 92, 193, 106, 51, 73, 31,
			27, 80, 218, 220, 248, 129, 29, 20, 128, 243, 250, 134, 39, 11, 0, 0, 0, 0, 0, 0, 0,
			175, 157, 109, 148, 134, 193, 83, 104, 236, 16, 0, 42, 117, 51, 200, 37, 254, 101, 130,
			54, 255, 213, 59, 173, 46, 242, 63, 71, 182, 250, 103, 138, 40, 37, 179, 204, 113, 233,
			191, 158, 183, 171, 24, 55, 9, 252, 109, 95, 123, 149, 186, 103, 219, 10, 141, 69, 234,
			43, 225, 116, 73, 98, 9, 10, 54, 3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5,
			57, 29, 19, 154, 98, 177, 87, 231, 135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19,
			20, 8, 6, 66, 65, 66, 69, 52, 2, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 5, 66, 65, 66,
			69, 1, 1, 178, 85, 38, 96, 177, 93, 181, 237, 49, 49, 135, 252, 82, 178, 156, 243, 180,
			77, 215, 139, 219, 221, 41, 185, 129, 120, 82, 241, 62, 48, 193, 111, 116, 194, 166,
			215, 19, 49, 28, 171, 173, 185, 194, 65, 151, 52, 46, 120, 249, 100, 255, 182, 166, 76,
			174, 179, 160, 123, 160, 145, 58, 244, 247, 131, 175, 157, 109, 148, 134, 193, 83, 104,
			236, 16, 0, 42, 117, 51, 200, 37, 254, 101, 130, 54, 255, 213, 59, 173, 46, 242, 63,
			71, 182, 250, 103, 138, 40, 37, 179, 204, 113, 233, 191, 158, 183, 171, 24, 55, 9, 252,
			109, 95, 123, 149, 186, 103, 219, 10, 141, 69, 234, 43, 225, 116, 73, 98, 9, 10, 54, 3,
			23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231,
			135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20, 8, 6, 66, 65, 66, 69, 52, 2, 0,
			0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 5, 66, 65, 66, 69, 1, 1, 64, 26, 84, 182, 160, 222,
			34, 198, 165, 146, 79, 37, 85, 10, 215, 43, 129, 200, 156, 108, 87, 47, 47, 75, 74, 65,
			59, 187, 194, 29, 62, 86, 163, 207, 136, 60, 7, 233, 140, 149, 75, 55, 209, 127, 195,
			201, 55, 198, 181, 32, 185, 196, 112, 143, 147, 136, 206, 34, 74, 74, 182, 79, 234,
			143,
		];

		let equivocation_proof1: pezsp_consensus_babe::EquivocationProof<Header> =
			Decode::decode(&mut &EQUIVOCATION_PROOF_BLOB[..]).unwrap();

		let equivocation_proof2 = equivocation_proof1.clone();

		#[block]
		{
			pezsp_consensus_babe::check_equivocation_proof::<Header>(equivocation_proof1);
		}

		assert!(pezsp_consensus_babe::check_equivocation_proof::<Header>(equivocation_proof2));
	}

	impl_benchmark_test_suite!(Pezpallet, crate::mock::new_test_ext(3), crate::mock::Test,);
}
