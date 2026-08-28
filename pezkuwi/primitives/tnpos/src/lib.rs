// This file is part of PezkuwiChain.

// Copyright (C) Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

//! Shared primitives for TNPoS: strata, committee arithmetic, score traits and the
//! security invariant. Deliberately runtime-free so the arithmetic can be tested and
//! audited on its own.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod committee;
pub mod invariant;
pub mod scores;
pub mod sortition;
pub mod stratum;

pub use stratum::{StratumConfig, StratumId};
