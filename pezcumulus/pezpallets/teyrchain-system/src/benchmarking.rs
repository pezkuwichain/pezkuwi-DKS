// This file is part of Pezcumulus.

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

//! Benchmarking for the teyrchain-system pezpallet.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::teyrchain_inherent::InboundDownwardMessages;
use pezcumulus_primitives_core::{relay_chain::Hash as RelayHash, InboundDownwardMessage};
use pezframe_benchmarking::v2::*;
use pezsp_runtime::traits::BlakeTwo256;

#[benchmarks]
mod benchmarks {
	use super::*;

	/// Enqueue `n` messages via `enqueue_inbound_downward_messages`.
	///
	/// The limit is set to `1000` for benchmarking purposes as the actual limit is only known at
	/// runtime. However, the limit (and default) for Dotsama are magnitudes smaller.
	#[benchmark]
	fn enqueue_inbound_downward_messages(n: Linear<0, 1000>) {
		let msg = InboundDownwardMessage {
			sent_at: n, // The block number does not matter.
			msg: vec![0u8; MaxDmpMessageLenOf::<T>::get() as usize],
		};
		let msgs = vec![msg; n as usize];
		let head = mqp_head(&msgs);

		#[block]
		{
			Pezpallet::<T>::enqueue_inbound_downward_messages(
				head,
				InboundDownwardMessages::new(msgs).into_abridged(&mut usize::MAX.clone()),
			);
		}

		assert_eq!(ProcessedDownwardMessages::<T>::get(), n);
		assert_eq!(LastDmqMqcHead::<T>::get().head(), head);
	}

	/// Re-implements an easy version of the `MessageQueueChain` for testing purposes.
	fn mqp_head(msgs: &Vec<InboundDownwardMessage>) -> RelayHash {
		let mut head = Default::default();
		for msg in msgs.iter() {
			let msg_hash = BlakeTwo256::hash_of(&msg.msg);
			head = BlakeTwo256::hash_of(&(head, msg.sent_at, msg_hash));
		}
		head
	}

	impl_benchmark_test_suite! {
		Pezpallet,
		crate::mock::new_test_ext(),
		crate::mock::Test
	}
}
