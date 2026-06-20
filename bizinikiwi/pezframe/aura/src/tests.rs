// This file is part of Bizinikiwi.

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

//! Tests for the module.

#![cfg(test)]

use super::pezpallet;
use crate::mock::{build_ext_and_execute_test, Aura, MockDisabledValidators, System, Test};
use codec::Encode;
use pezframe_support::traits::OnInitialize;
use pezsp_consensus_aura::{Slot, AURA_ENGINE_ID};
use pezsp_runtime::{Digest, DigestItem};

#[test]
fn initial_values() {
	build_ext_and_execute_test(vec![0, 1, 2, 3], || {
		assert_eq!(pezpallet::CurrentSlot::<Test>::get(), 0u64);
		assert_eq!(pezpallet::Authorities::<Test>::get().len(), Aura::authorities_len());
		assert_eq!(Aura::authorities_len(), 4);
	});
}

#[test]
#[should_panic(
	expected = "Validator with index 1 is disabled and should not be attempting to author blocks."
)]
fn disabled_validators_cannot_author_blocks() {
	build_ext_and_execute_test(vec![0, 1, 2, 3], || {
		// slot 1 should be authored by validator at index 1
		let slot = Slot::from(1);
		let pre_digest =
			Digest { logs: vec![DigestItem::PreRuntime(AURA_ENGINE_ID, slot.encode())] };

		System::reset_events();
		System::initialize(&1, &System::parent_hash(), &pre_digest);

		// let's disable the validator
		MockDisabledValidators::disable_validator(1);

		// and we should not be able to initialize the block
		Aura::on_initialize(1);
	});
}

#[test]
#[should_panic(expected = "Slot must increase")]
fn pezpallet_requires_slot_to_increase_unless_allowed() {
	build_ext_and_execute_test(vec![0, 1, 2, 3], || {
		crate::mock::AllowMultipleBlocksPerSlot::set(false);

		let slot = Slot::from(1);
		let pre_digest =
			Digest { logs: vec![DigestItem::PreRuntime(AURA_ENGINE_ID, slot.encode())] };

		System::reset_events();
		System::initialize(&1, &System::parent_hash(), &pre_digest);

		// and we should not be able to initialize the block with the same slot a second time.
		Aura::on_initialize(1);
		Aura::on_initialize(1);
	});
}

#[test]
fn pezpallet_can_allow_unchanged_slot() {
	build_ext_and_execute_test(vec![0, 1, 2, 3], || {
		let slot = Slot::from(1);
		let pre_digest =
			Digest { logs: vec![DigestItem::PreRuntime(AURA_ENGINE_ID, slot.encode())] };

		System::reset_events();
		System::initialize(&1, &System::parent_hash(), &pre_digest);

		crate::mock::AllowMultipleBlocksPerSlot::set(true);

		// and we should be able to initialize the block with the same slot a second time.
		Aura::on_initialize(1);
		Aura::on_initialize(1);
	});
}

#[test]
#[should_panic(expected = "Slot must not decrease")]
fn pezpallet_always_rejects_decreasing_slot() {
	build_ext_and_execute_test(vec![0, 1, 2, 3], || {
		let slot = Slot::from(2);
		let pre_digest =
			Digest { logs: vec![DigestItem::PreRuntime(AURA_ENGINE_ID, slot.encode())] };

		System::reset_events();
		System::initialize(&1, &System::parent_hash(), &pre_digest);

		crate::mock::AllowMultipleBlocksPerSlot::set(true);

		Aura::on_initialize(1);
		System::finalize();

		let earlier_slot = Slot::from(1);
		let pre_digest =
			Digest { logs: vec![DigestItem::PreRuntime(AURA_ENGINE_ID, earlier_slot.encode())] };
		System::initialize(&2, &System::parent_hash(), &pre_digest);
		Aura::on_initialize(2);
	});
}
