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

//! Relay chain runtime mock.

use pezframe::{
	deps::{pezframe_support::weights::WeightMeter, pezsp_runtime::AccountId32},
	prelude::*,
	runtime::prelude::*,
	traits::{IdentityLookup, ProcessMessage, ProcessMessageError},
};
use pezkuwi_runtime_teyrchains::inclusion::{AggregateMessageOrigin, UmpQueueId};
use xcm::latest::prelude::*;

mod xcm_config;
pub use xcm_config::LocationToAccountId;
use xcm_config::XcmConfig;

pub type AccountId = AccountId32;
pub type Balance = u64;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pezpallet_balances::AccountData<Balance>;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Runtime {
	type AccountStore = System;
}

type Block = pezframe_system::mocking::MockBlock<Runtime>;

parameter_types! {
	/// Amount of weight that can be spent per block to service messages.
	pub MessageQueueServiceWeight: Weight = Weight::from_parts(1_000_000_000, 1_000_000);
	pub const MessageQueueHeapSize: u32 = 65_536;
	pub const MessageQueueMaxStale: u32 = 16;
}

/// Message processor to handle any messages that were enqueued into the `MessageQueue` pezpallet.
pub struct MessageProcessor;
impl ProcessMessage for MessageProcessor {
	type Origin = AggregateMessageOrigin;

	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		meter: &mut WeightMeter,
		id: &mut [u8; 32],
	) -> Result<bool, ProcessMessageError> {
		let para = match origin {
			AggregateMessageOrigin::Ump(UmpQueueId::Para(para)) => para,
		};
		xcm_builder::ProcessXcmMessage::<
			Junction,
			xcm_executor::XcmExecutor<XcmConfig>,
			RuntimeCall,
		>::process_message(message, Junction::Teyrchain(para.into()), meter, id)
	}
}

impl pezpallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Size = u32;
	type HeapSize = MessageQueueHeapSize;
	type MaxStale = MessageQueueMaxStale;
	type ServiceWeight = MessageQueueServiceWeight;
	type MessageProcessor = MessageProcessor;
	type QueueChangeHandler = ();
	type QueuePausedQuery = ();
	type WeightInfo = ();
	type IdleMaxServiceWeight = MessageQueueServiceWeight;
}

construct_runtime! {
	pub struct Runtime {
		System: pezframe_system,
		Balances: pezpallet_balances,
		MessageQueue: pezpallet_message_queue,
		XcmPallet: pezpallet_xcm,
	}
}
