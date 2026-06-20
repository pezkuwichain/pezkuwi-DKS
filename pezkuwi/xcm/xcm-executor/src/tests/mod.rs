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

//! Unit tests for the XCM executor.
//!
//! These exclude any cross-chain functionality. For those, look at the
//! `xcm-pez-emulator` based tests in the pezcumulus folder.
//! These tests deal with internal state changes of the XCVM.

mod initiate_transfer;
mod mock;
mod pay_fees;
mod set_asset_claimer;
