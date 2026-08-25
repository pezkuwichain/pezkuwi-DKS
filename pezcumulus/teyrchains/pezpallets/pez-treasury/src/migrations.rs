// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Storage version for pezpallet-pez-treasury.
//!
//! There is no migration here. This file used to carry a `v1::MigrateToV1` that read four
//! storage items, logged how many it had found, wrote the version and returned -- it changed
//! nothing, and no runtime ever listed it in its `Migrations` tuple, so it never ran even to
//! do that. Both chains that use this pallet start from genesis at the current version.
//!
//! When a real migration is needed, add it here, raise `STORAGE_VERSION`, and put it in the
//! runtime's `Migrations` tuple -- a migration that is written but not wired is worse than no
//! migration, because it looks like the storage has been dealt with.

use pezframe_support::traits::StorageVersion;

/// Current storage version.
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
