// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
pub use crate::config::{
	SLOTS_PER_HISTORICAL_ROOT, SYNC_COMMITTEE_BITS_SIZE as SC_BITS_SIZE,
	SYNC_COMMITTEE_SIZE as SC_SIZE,
};
use pezframe_support::storage::types::OptionQuery;
use pezsnowbridge_core::RingBufferMapImpl;

// Specialize types based on configured sync committee size
pub type SyncCommittee = pezsnowbridge_beacon_primitives::SyncCommittee<SC_SIZE>;
pub type SyncCommitteePrepared = pezsnowbridge_beacon_primitives::SyncCommitteePrepared<SC_SIZE>;
pub type SyncAggregate = pezsnowbridge_beacon_primitives::SyncAggregate<SC_SIZE, SC_BITS_SIZE>;
pub type CheckpointUpdate = pezsnowbridge_beacon_primitives::CheckpointUpdate<SC_SIZE>;
pub type Update = pezsnowbridge_beacon_primitives::Update<SC_SIZE, SC_BITS_SIZE>;
pub type NextSyncCommitteeUpdate =
	pezsnowbridge_beacon_primitives::NextSyncCommitteeUpdate<SC_SIZE>;

pub use pezsnowbridge_beacon_primitives::{AncestryProof, ExecutionProof};

/// FinalizedState ring buffer implementation
pub type FinalizedBeaconStateBuffer<T> = RingBufferMapImpl<
	u32,
	crate::MaxFinalizedHeadersToKeep<T>,
	crate::FinalizedBeaconStateIndex<T>,
	crate::FinalizedBeaconStateMapping<T>,
	crate::FinalizedBeaconState<T>,
	OptionQuery,
>;
