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

//! Metrics for the trie cache.

#[cfg(feature = "std")]
pub use prometheus_impl::*;

#[cfg(feature = "std")]
mod prometheus_impl {
	use super::TrieHitStatsSnapshot;

	/// Metrics for the trie cache - stub implementation when prometheus is disabled.
	#[derive(Clone)]
	pub struct Metrics;

	/// Stub registry type when prometheus is disabled.
	pub struct Registry;

	/// Stub timer type.
	pub struct HistogramTimer;

	impl Drop for HistogramTimer {
		fn drop(&mut self) {}
	}

	impl Metrics {
		/// Create a new instance of the metrics.
		pub(crate) fn register(_registry: &Registry) -> Result<Self, ()> {
			Ok(Self)
		}

		/// Start a timer for the shared node cache update duration.
		pub(crate) fn start_shared_node_update_timer(&self) -> HistogramTimer {
			HistogramTimer
		}

		/// Start a timer for the shared value cache update duration.
		pub(crate) fn start_shared_value_update_timer(&self) -> HistogramTimer {
			HistogramTimer
		}

		/// Observe the shared node cache length.
		pub(crate) fn observe_local_node_cache_length(&self, _node_cache_len: usize) {}

		/// Observe the shared value cache length.
		pub(crate) fn observe_local_value_cache_length(&self, _value_cache_len: usize) {}

		/// Observe the shared node cache inline size.
		pub(crate) fn observe_node_cache_inline_size(&self, _cache_size: usize) {}

		/// Observe the shared value cache inline size.
		pub(crate) fn observe_value_cache_inline_size(&self, _cache_size: usize) {}

		/// Observe the shared node cache heap size.
		pub(crate) fn observe_node_cache_heap_size(&self, _cache_size: usize) {}

		/// Observe the shared value cache heap size.
		pub(crate) fn observe_value_cache_heap_size(&self, _cache_size: usize) {}

		/// Observe the hit stats from an instance of a local cache.
		pub(crate) fn observe_hits_stats(&self, _stats: &TrieHitStatsSnapshot) {}
	}
}

/// A snapshot of the hit/miss stats.
#[derive(Default, Copy, Clone, Debug)]
pub(crate) struct HitStatsSnapshot {
	pub(crate) shared_hits: u64,
	pub(crate) shared_fetch_attempts: u64,
	pub(crate) local_hits: u64,
	pub(crate) local_fetch_attempts: u64,
}

impl std::fmt::Display for HitStatsSnapshot {
	fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
		let shared_hits = self.shared_hits;
		let shared_fetch_attempts = self.shared_fetch_attempts;
		let local_hits = self.local_hits;
		let local_fetch_attempts = self.local_fetch_attempts;

		if shared_fetch_attempts == 0 && local_hits == 0 {
			write!(fmt, "empty")
		} else {
			let percent_local = (local_hits as f32 / local_fetch_attempts as f32) * 100.0;
			let percent_shared = (shared_hits as f32 / shared_fetch_attempts as f32) * 100.0;
			write!(
				fmt,
				"local hit rate = {}% [{}/{}], shared hit rate = {}% [{}/{}]",
				percent_local as u32,
				local_hits,
				local_fetch_attempts,
				percent_shared as u32,
				shared_hits,
				shared_fetch_attempts
			)
		}
	}
}

/// Snapshot of the hit/miss stats for the node cache and the value cache.
#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct TrieHitStatsSnapshot {
	pub(crate) node_cache: HitStatsSnapshot,
	pub(crate) value_cache: HitStatsSnapshot,
}
