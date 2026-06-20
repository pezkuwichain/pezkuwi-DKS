// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::imports::*;
use pezframe_support::traits::OnInitialize;
use pezpallet_broker::{ConfigRecord, Configuration, CoreAssignment, CoreMask, ScheduleItem};
use pezsp_runtime::Perbill;
use zagros_runtime_constants::system_teyrchain::coretime::TIMESLICE_PERIOD;
use zagros_system_emulated_network::zagros_emulated_chain::zagros_runtime::Dmp;

#[test]
fn transact_hardcoded_weights_are_sane() {
	// There are three transacts with hardcoded weights sent from the Coretime Chain to the Relay
	// Chain across the CoretimeInterface which are triggered at various points in the sales cycle.
	// - Request core count - triggered directly by `start_sales` or `request_core_count`
	//   extrinsics.
	// - Request revenue info - triggered when each timeslice is committed.
	// - Assign core - triggered when an entry is encountered in the workplan for the next
	//   timeslice.

	// RuntimeEvent aliases to avoid warning from usage of qualified paths in assertions due to
	// <https://github.com/rust-lang/rust/issues/86935>
	type CoretimeEvent = <CoretimeZagros as Chain>::RuntimeEvent;
	type RelayEvent = <Zagros as Chain>::RuntimeEvent;

	Zagros::execute_with(|| {
		Dmp::make_teyrchain_reachable(CoretimeZagros::para_id());
	});

	// Reserve a workload, configure broker and start sales.
	CoretimeZagros::execute_with(|| {
		// Hooks don't run in emulated tests - workaround as we need `on_initialize` to tick things
		// along and have no concept of time passing otherwise.
		<CoretimeZagros as CoretimeZagrosPallet>::Broker::on_initialize(
			<CoretimeZagros as Chain>::System::block_number(),
		);

		let coretime_root_origin = <CoretimeZagros as Chain>::RuntimeOrigin::root();

		// Create and populate schedule with the worst case assignment on this core.
		let mut schedule = Vec::new();
		for i in 0..80 {
			schedule.push(ScheduleItem {
				mask: CoreMask::void().set(i),
				assignment: CoreAssignment::Task(2000 + i),
			})
		}

		assert_ok!(<CoretimeZagros as CoretimeZagrosPallet>::Broker::reserve(
			coretime_root_origin.clone(),
			schedule.try_into().expect("Vector is within bounds."),
		));

		// Configure broker and start sales.
		let config = ConfigRecord {
			advance_notice: 1,
			interlude_length: 1,
			leadin_length: 2,
			region_length: 1,
			ideal_bulk_proportion: Perbill::from_percent(40),
			limit_cores_offered: None,
			renewal_bump: Perbill::from_percent(2),
			contribution_timeout: 1,
		};
		assert_ok!(<CoretimeZagros as CoretimeZagrosPallet>::Broker::configure(
			coretime_root_origin.clone(),
			config
		));
		assert_ok!(<CoretimeZagros as CoretimeZagrosPallet>::Broker::start_sales(
			coretime_root_origin,
			100,
			0
		));
		assert_eq!(
			pezpallet_broker::Status::<<CoretimeZagros as Chain>::Runtime>::get()
				.unwrap()
				.core_count,
			1
		);

		assert_expected_events!(
			CoretimeZagros,
			vec![
				CoretimeEvent::Broker(
					pezpallet_broker::Event::ReservationMade { .. }
				) => {},
				CoretimeEvent::Broker(
					pezpallet_broker::Event::CoreCountRequested { core_count: 1 }
				) => {},
				CoretimeEvent::TeyrchainSystem(
					pezcumulus_pezpallet_teyrchain_system::Event::UpwardMessageSent { .. }
				) => {},
			]
		);
	});

	// Check that the request_core_count message was processed successfully. This will fail if the
	// weights are misconfigured.
	Zagros::execute_with(|| {
		Zagros::assert_ump_queue_processed(true, Some(CoretimeZagros::para_id()), None);

		assert_expected_events!(
			Zagros,
			vec![
				RelayEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	// Keep track of the relay chain block number so we can fast forward while still checking the
	// right block.
	let mut block_number_cursor = Zagros::ext_wrapper(<Zagros as Chain>::System::block_number);

	let config = CoretimeZagros::ext_wrapper(|| {
		Configuration::<<CoretimeZagros as Chain>::Runtime>::get()
			.expect("Pezpallet was configured earlier.")
	});

	// Now run up to the block before the sale is rotated.
	while block_number_cursor < TIMESLICE_PERIOD - config.advance_notice - 1 {
		CoretimeZagros::execute_with(|| {
			// Hooks don't run in emulated tests - workaround.
			<CoretimeZagros as CoretimeZagrosPallet>::Broker::on_initialize(
				<CoretimeZagros as Chain>::System::block_number(),
			);
		});

		Zagros::ext_wrapper(|| {
			block_number_cursor = <Zagros as Chain>::System::block_number();
		});
	}

	// In this block we trigger assign core.
	CoretimeZagros::execute_with(|| {
		// Hooks don't run in emulated tests - workaround.
		<CoretimeZagros as CoretimeZagrosPallet>::Broker::on_initialize(
			<CoretimeZagros as Chain>::System::block_number(),
		);

		assert_expected_events!(
			CoretimeZagros,
			vec![
				CoretimeEvent::Broker(
					pezpallet_broker::Event::SaleInitialized { .. }
				) => {},
				CoretimeEvent::Broker(
					pezpallet_broker::Event::CoreAssigned { .. }
				) => {},
				CoretimeEvent::TeyrchainSystem(
					pezcumulus_pezpallet_teyrchain_system::Event::UpwardMessageSent { .. }
				) => {},
			]
		);
	});

	// In this block we trigger request revenue.
	CoretimeZagros::execute_with(|| {
		// Hooks don't run in emulated tests - workaround.
		<CoretimeZagros as CoretimeZagrosPallet>::Broker::on_initialize(
			<CoretimeZagros as Chain>::System::block_number(),
		);

		assert_expected_events!(
			CoretimeZagros,
			vec![
				CoretimeEvent::TeyrchainSystem(
					pezcumulus_pezpallet_teyrchain_system::Event::UpwardMessageSent { .. }
				) => {},
			]
		);
	});

	// Check that the assign_core and request_revenue_info_at messages were processed successfully.
	// This will fail if the weights are misconfigured.
	Zagros::execute_with(|| {
		Zagros::assert_ump_queue_processed(true, Some(CoretimeZagros::para_id()), None);

		assert_expected_events!(
			Zagros,
			vec![
				RelayEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RelayEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				RelayEvent::Coretime(
					pezkuwi_runtime_teyrchains::coretime::Event::CoreAssigned { .. }
				) => {},
			]
		);
	});

	// Here we receive and process the notify_revenue XCM with zero revenue.
	CoretimeZagros::execute_with(|| {
		// Hooks don't run in emulated tests - workaround.
		<CoretimeZagros as CoretimeZagrosPallet>::Broker::on_initialize(
			<CoretimeZagros as Chain>::System::block_number(),
		);

		assert_expected_events!(
			CoretimeZagros,
			vec![
				CoretimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
				// Zero revenue in first timeslice so history is immediately dropped.
				CoretimeEvent::Broker(
					pezpallet_broker::Event::HistoryDropped { when: 0, revenue: 0 }
				) => {},
			]
		);
	});
}
