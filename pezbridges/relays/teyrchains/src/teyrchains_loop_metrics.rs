// Copyright 2019-2021 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

use pezbp_pezkuwi_core::teyrchains::ParaId;
use relay_utils::{
	metrics::{metric_name, register, Gauge, Metric, PrometheusError, Registry, U64},
	UniqueSaturatedInto,
};

/// Teyrchains sync metrics.
#[derive(Clone)]
pub struct TeyrchainsLoopMetrics {
	/// Best teyrchains header numbers at the source.
	best_source_block_numbers: Gauge<U64>,
	/// Best teyrchains header numbers at the target.
	best_target_block_numbers: Gauge<U64>,
}

impl TeyrchainsLoopMetrics {
	/// Create and register teyrchains loop metrics.
	pub fn new(prefix: Option<&str>) -> Result<Self, PrometheusError> {
		Ok(TeyrchainsLoopMetrics {
			best_source_block_numbers: Gauge::new(
				metric_name(prefix, "best_teyrchain_block_number_at_source"),
				"Best teyrchain block numbers at the source relay chain".to_string(),
			)?,
			best_target_block_numbers: Gauge::new(
				metric_name(prefix, "best_teyrchain_block_number_at_target"),
				"Best teyrchain block numbers at the target chain".to_string(),
			)?,
		})
	}

	/// Update best block number at source.
	pub fn update_best_teyrchain_block_at_source<Number: UniqueSaturatedInto<u64>>(
		&self,
		teyrchain: ParaId,
		block_number: Number,
	) {
		let block_number = block_number.unique_saturated_into();
		tracing::trace!(
			target: "bridge-metrics",
			?teyrchain,
			?block_number,
			"Updated value of metric 'best_teyrchain_block_number_at_source"
		);
		self.best_source_block_numbers.set(block_number);
	}

	/// Update best block number at target.
	pub fn update_best_teyrchain_block_at_target<Number: UniqueSaturatedInto<u64>>(
		&self,
		teyrchain: ParaId,
		block_number: Number,
	) {
		let block_number = block_number.unique_saturated_into();
		tracing::trace!(
			target: "bridge-metrics",
			?teyrchain,
			?block_number,
			"Updated value of metric 'best_teyrchain_block_number_at_target"
		);
		self.best_target_block_numbers.set(block_number);
	}
}

impl Metric for TeyrchainsLoopMetrics {
	fn register(&self, registry: &Registry) -> Result<(), PrometheusError> {
		register(self.best_source_block_numbers.clone(), registry)?;
		register(self.best_target_block_numbers.clone(), registry)?;
		Ok(())
	}
}
