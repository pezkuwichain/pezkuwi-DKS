// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
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

use super::*;
use pezframe_benchmarking::v2::*;
use pezframe_system::RawOrigin;
use pezkuwi_primitives::ConsensusLog;
use pezsp_runtime::DigestItem;

// Random large number for the digest
const DIGEST_MAX_LEN: u32 = 65536;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn force_approve(d: Linear<0, DIGEST_MAX_LEN>) -> Result<(), BenchmarkError> {
		for _ in 0..d {
			pezframe_system::Pezpallet::<T>::deposit_log(ConsensusLog::ForceApprove(d).into());
		}

		#[extrinsic_call]
		_(RawOrigin::Root, d + 1);

		assert_eq!(
			pezframe_system::Pezpallet::<T>::digest().logs.last().unwrap(),
			&DigestItem::from(ConsensusLog::ForceApprove(d + 1)),
		);

		Ok(())
	}

	impl_benchmark_test_suite!(
		Pezpallet,
		crate::mock::new_test_ext(Default::default()),
		crate::mock::Test
	);
}
