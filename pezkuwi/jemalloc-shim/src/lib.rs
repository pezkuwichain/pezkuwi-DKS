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

//! Shim crate to enable `jemalloc-allocator` feature for Pezkuwi crates.
//!
//! Because [there doesn't exist any easier way right now](https://github.com/rust-lang/cargo/issues/1197), we
//! need an entire crate to handle `jemalloc` enabling/disabling. This way we can enable it by
//! default on Linux, but have it optional on all other OSes.

/// Sets the global allocator to `jemalloc` when the feature is enabled.
#[cfg(feature = "jemalloc-allocator")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
