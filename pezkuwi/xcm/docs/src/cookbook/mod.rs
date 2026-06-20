// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
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

//! # XCM Cookbook
//!
//! A collection of XCM recipes.
//!
//! Each recipe is tested and explains all the code necessary to run it -- they're not just snippets
//! to copy and paste.

/// Configuring a teyrchain that only uses the Relay Chain native token.
/// In the case of Pezkuwi, this recipe will show you how to launch a teyrchain with no native
/// token -- dealing only on HEZ.
pub mod relay_token_transactor;
