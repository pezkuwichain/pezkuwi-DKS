// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

//! Tests for the ping pezpallet.

use super::{mock::*, *};
use alloc::vec;
use pezcumulus_pezpallet_xcm::Origin as CumulusOrigin;
use pezcumulus_primitives_core::ParaId;
use pezframe_support::{assert_noop, assert_ok, traits::Hooks};
use pezsp_runtime::traits::BadOrigin;

// ============================================================================
// start() tests
// ============================================================================

#[test]
fn start_works_with_root() {
	new_test_ext().execute_with(|| {
		let para = ParaId::from(2000u32);
		let payload = vec![1u8; 32];

		assert_ok!(Ping::start(RuntimeOrigin::root(), para, payload.clone()));

		let targets = Targets::<Test>::get();
		assert_eq!(targets.len(), 1);
		assert_eq!(targets[0].0, para);
		assert_eq!(targets[0].1.to_vec(), payload);
	});
}

#[test]
fn start_rejects_non_root() {
	new_test_ext().execute_with(|| {
		let para = ParaId::from(2000u32);
		let payload = vec![1u8; 32];

		assert_noop!(Ping::start(RuntimeOrigin::signed(1), para, payload), BadOrigin);
	});
}

#[test]
fn start_rejects_payload_too_large() {
	new_test_ext().execute_with(|| {
		let para = ParaId::from(2000u32);
		// MaxPayloadSize is 1024, so 1025 should fail.
		let payload = vec![0u8; 1025];

		assert_noop!(
			Ping::start(RuntimeOrigin::root(), para, payload),
			Error::<Test>::PayloadTooLarge
		);
	});
}

#[test]
fn start_respects_max_payload_boundary() {
	new_test_ext().execute_with(|| {
		let para = ParaId::from(2000u32);
		// Exactly 1024 should succeed.
		let payload = vec![0u8; 1024];

		assert_ok!(Ping::start(RuntimeOrigin::root(), para, payload));
		assert_eq!(Targets::<Test>::get().len(), 1);
	});
}

#[test]
fn start_rejects_too_many_targets() {
	new_test_ext().execute_with(|| {
		// MaxTeyrchains is 100.
		for i in 0..100u32 {
			assert_ok!(Ping::start(RuntimeOrigin::root(), ParaId::from(2000 + i), vec![0u8; 4],));
		}

		// 101st should fail.
		assert_noop!(
			Ping::start(RuntimeOrigin::root(), ParaId::from(3000u32), vec![0u8; 4]),
			Error::<Test>::TooManyTargets
		);
	});
}

// ============================================================================
// start_many() tests
// ============================================================================

#[test]
fn start_many_adds_multiple_entries() {
	new_test_ext().execute_with(|| {
		let para = ParaId::from(2000u32);
		let payload = vec![1u8; 16];

		assert_ok!(Ping::start_many(RuntimeOrigin::root(), para, 5, payload.clone()));

		let targets = Targets::<Test>::get();
		assert_eq!(targets.len(), 5);
		for (p, pl) in targets.iter() {
			assert_eq!(*p, para);
			assert_eq!(pl.to_vec(), payload);
		}
	});
}

#[test]
fn start_many_rejects_non_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Ping::start_many(RuntimeOrigin::signed(1), ParaId::from(2000u32), 1, vec![0u8]),
			BadOrigin
		);
	});
}

#[test]
fn start_many_respects_max_targets() {
	new_test_ext().execute_with(|| {
		// Try to add 101 targets at once — should fail after reaching 100.
		assert_noop!(
			Ping::start_many(RuntimeOrigin::root(), ParaId::from(2000u32), 101, vec![0u8; 4]),
			Error::<Test>::TooManyTargets
		);
	});
}

#[test]
fn start_many_rejects_payload_too_large() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Ping::start_many(RuntimeOrigin::root(), ParaId::from(2000u32), 1, vec![0u8; 1025],),
			Error::<Test>::PayloadTooLarge
		);
	});
}

// ============================================================================
// stop() tests
// ============================================================================

#[test]
fn stop_removes_first_matching_target() {
	new_test_ext().execute_with(|| {
		let para_a = ParaId::from(2000u32);
		let para_b = ParaId::from(2001u32);

		assert_ok!(Ping::start(RuntimeOrigin::root(), para_a, vec![1u8; 4]));
		assert_ok!(Ping::start(RuntimeOrigin::root(), para_b, vec![2u8; 4]));
		assert_ok!(Ping::start(RuntimeOrigin::root(), para_a, vec![3u8; 4]));
		assert_eq!(Targets::<Test>::get().len(), 3);

		// stop removes only the first match for para_a.
		assert_ok!(Ping::stop(RuntimeOrigin::root(), para_a));

		let targets = Targets::<Test>::get();
		assert_eq!(targets.len(), 2);
		// swap_remove replaces position 0 with the last element.
		// After removing index 0 (para_a, [1;4]), the vec becomes [(para_a, [3;4]), (para_b, [2;4])].
		let paras: Vec<ParaId> = targets.iter().map(|(p, _)| *p).collect();
		assert!(paras.contains(&para_b));
	});
}

#[test]
fn stop_rejects_non_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(Ping::stop(RuntimeOrigin::signed(1), ParaId::from(2000u32)), BadOrigin);
	});
}

#[test]
fn stop_noop_if_no_match() {
	new_test_ext().execute_with(|| {
		assert_ok!(Ping::start(RuntimeOrigin::root(), ParaId::from(2000u32), vec![0u8; 4]));
		// Stopping a different para is a no-op, not an error.
		assert_ok!(Ping::stop(RuntimeOrigin::root(), ParaId::from(9999u32)));
		assert_eq!(Targets::<Test>::get().len(), 1);
	});
}

// ============================================================================
// stop_all() tests
// ============================================================================

#[test]
fn stop_all_clears_everything() {
	new_test_ext().execute_with(|| {
		assert_ok!(Ping::start(RuntimeOrigin::root(), ParaId::from(2000u32), vec![1u8; 4]));
		assert_ok!(Ping::start(RuntimeOrigin::root(), ParaId::from(2001u32), vec![2u8; 4]));
		assert_eq!(Targets::<Test>::get().len(), 2);

		// Pass None to clear all.
		assert_ok!(Ping::stop_all(RuntimeOrigin::root(), None));
		assert_eq!(Targets::<Test>::get().len(), 0);
	});
}

#[test]
fn stop_all_with_para_filter() {
	new_test_ext().execute_with(|| {
		let para_a = ParaId::from(2000u32);
		let para_b = ParaId::from(2001u32);

		assert_ok!(Ping::start(RuntimeOrigin::root(), para_a, vec![1u8; 4]));
		assert_ok!(Ping::start(RuntimeOrigin::root(), para_b, vec![2u8; 4]));
		assert_ok!(Ping::start(RuntimeOrigin::root(), para_a, vec![3u8; 4]));
		assert_eq!(Targets::<Test>::get().len(), 3);

		// Remove only entries for para_a.
		assert_ok!(Ping::stop_all(RuntimeOrigin::root(), Some(para_a)));

		let targets = Targets::<Test>::get();
		assert_eq!(targets.len(), 1);
		assert_eq!(targets[0].0, para_b);
	});
}

#[test]
fn stop_all_rejects_non_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(Ping::stop_all(RuntimeOrigin::signed(1), None), BadOrigin);
	});
}

// ============================================================================
// ping() tests
// ============================================================================

#[test]
fn ping_from_sibling_para_works() {
	new_test_ext().execute_with(|| {
		let para_id = ParaId::from(2000u32);
		let origin = RuntimeOrigin::from(CumulusOrigin::SiblingTeyrchain(para_id));

		assert_ok!(Ping::ping(origin, 1, vec![42u8; 8]));

		// Should have deposited a Pinged event.
		System::assert_has_event(Event::<Test>::Pinged(para_id, 1, vec![42u8; 8]).into());
	});
}

#[test]
fn ping_rejects_signed_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(Ping::ping(RuntimeOrigin::signed(1), 1, vec![0u8; 4]), BadOrigin);
	});
}

#[test]
fn ping_rejects_root_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(Ping::ping(RuntimeOrigin::root(), 1, vec![0u8; 4]), BadOrigin);
	});
}

// ============================================================================
// pong() tests
// ============================================================================

#[test]
fn pong_processes_known_ping() {
	new_test_ext().execute_with(|| {
		let para_id = ParaId::from(2000u32);

		// Insert a ping record at block 1 (current block).
		Pings::<Test>::insert(42u32, 1u64);

		let origin = RuntimeOrigin::from(CumulusOrigin::SiblingTeyrchain(para_id));
		assert_ok!(Ping::pong(origin, 42, vec![0u8; 4]));

		// The ping entry should be removed.
		assert!(Pings::<Test>::get(42u32).is_none());

		// Should emit Ponged event with the round-trip time.
		System::assert_has_event(Event::<Test>::Ponged(para_id, 42, vec![0u8; 4], 0u64).into());
	});
}

#[test]
fn pong_handles_unknown_pong() {
	new_test_ext().execute_with(|| {
		let para_id = ParaId::from(2000u32);
		let origin = RuntimeOrigin::from(CumulusOrigin::SiblingTeyrchain(para_id));

		// No ping entry for seq 99.
		assert_ok!(Ping::pong(origin, 99, vec![0u8; 4]));

		System::assert_has_event(Event::<Test>::UnknownPong(para_id, 99, vec![0u8; 4]).into());
	});
}

#[test]
fn pong_rejects_signed_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(Ping::pong(RuntimeOrigin::signed(1), 1, vec![0u8; 4]), BadOrigin);
	});
}

// ============================================================================
// on_finalize() hook tests
// ============================================================================

#[test]
fn on_finalize_sends_pings_to_targets() {
	new_test_ext().execute_with(|| {
		let para = ParaId::from(2000u32);
		assert_ok!(Ping::start(RuntimeOrigin::root(), para, vec![7u8; 4]));

		// PingCount starts at 0.
		assert_eq!(PingCount::<Test>::get(), 0);

		// Trigger on_finalize.
		Ping::on_finalize(1u64);

		// PingCount should have incremented.
		assert_eq!(PingCount::<Test>::get(), 1);

		// Should have recorded the ping in Pings storage.
		assert_eq!(Pings::<Test>::get(1u32), Some(1u64));

		// Should have emitted PingSent event (MockXcmSender always succeeds).
		System::assert_has_event(
			Event::<Test>::PingSent(para, 1, vec![7u8; 4], [0u8; 32], Assets::new()).into(),
		);
	});
}

#[test]
fn on_finalize_increments_seq_per_target() {
	new_test_ext().execute_with(|| {
		let para_a = ParaId::from(2000u32);
		let para_b = ParaId::from(2001u32);
		assert_ok!(Ping::start(RuntimeOrigin::root(), para_a, vec![1u8; 4]));
		assert_ok!(Ping::start(RuntimeOrigin::root(), para_b, vec![2u8; 4]));

		Ping::on_finalize(1u64);

		// Two targets = two pings, seq 1 and 2.
		assert_eq!(PingCount::<Test>::get(), 2);
		assert!(Pings::<Test>::get(1u32).is_some());
		assert!(Pings::<Test>::get(2u32).is_some());
	});
}

#[test]
fn on_finalize_noop_with_no_targets() {
	new_test_ext().execute_with(|| {
		Ping::on_finalize(1u64);
		assert_eq!(PingCount::<Test>::get(), 0);
	});
}
