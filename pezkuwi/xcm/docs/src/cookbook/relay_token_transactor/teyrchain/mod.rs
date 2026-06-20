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

//! # Runtime

use pezframe::{deps::pezframe_system, runtime::prelude::*, traits::IdentityLookup};
use xcm_executor::XcmExecutor;
use xcm_pez_simulator::mock_message_queue;

mod xcm_config;
use xcm_config::XcmConfig;

pub type Block = pezframe_system::mocking::MockBlock<Runtime>;
pub type AccountId = pezframe::deps::pezsp_runtime::AccountId32;
pub type Balance = u64;

construct_runtime! {
	pub struct Runtime {
		System: pezframe_system,
		MessageQueue: mock_message_queue,
		Balances: pezpallet_balances,
		XcmPallet: pezpallet_xcm,
	}
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type AccountData = pezpallet_balances::AccountData<Balance>;
}

impl mock_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Runtime {
	type AccountStore = System;
}
