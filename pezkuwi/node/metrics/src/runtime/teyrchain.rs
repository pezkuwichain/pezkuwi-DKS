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

//! Client side declaration and registration of the teyrchain Prometheus metrics.
//! All of the metrics have a correspondent runtime metric definition.

use crate::runtime::RuntimeMetricsProvider;
use pezkuwi_primitives::metric_definitions::{
	TEYRCHAIN_CREATE_INHERENT_BITFIELDS_SIGNATURE_CHECKS,
	TEYRCHAIN_INHERENT_DATA_BITFIELDS_PROCESSED, TEYRCHAIN_INHERENT_DATA_CANDIDATES_PROCESSED,
	TEYRCHAIN_INHERENT_DATA_DISPUTE_SETS_PROCESSED, TEYRCHAIN_INHERENT_DATA_WEIGHT,
	TEYRCHAIN_VERIFY_DISPUTE_SIGNATURE,
};

/// Register the teyrchain runtime metrics.
pub fn register_metrics(runtime_metrics_provider: &RuntimeMetricsProvider) {
	runtime_metrics_provider.register_counter(TEYRCHAIN_INHERENT_DATA_BITFIELDS_PROCESSED);

	runtime_metrics_provider.register_countervec(TEYRCHAIN_INHERENT_DATA_WEIGHT);
	runtime_metrics_provider.register_countervec(TEYRCHAIN_INHERENT_DATA_DISPUTE_SETS_PROCESSED);
	runtime_metrics_provider.register_countervec(TEYRCHAIN_INHERENT_DATA_CANDIDATES_PROCESSED);
	runtime_metrics_provider
		.register_countervec(TEYRCHAIN_CREATE_INHERENT_BITFIELDS_SIGNATURE_CHECKS);
	runtime_metrics_provider.register_histogram(TEYRCHAIN_VERIFY_DISPUTE_SIGNATURE);
}
