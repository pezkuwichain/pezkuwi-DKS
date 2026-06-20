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

//! Tests for teyrchain-info pallet.

use crate::mock::*;
use crate::pezpallet::Pezpallet;
use pezcumulus_primitives_core::ParaId;
use pezframe_support::traits::Get;

#[test]
fn genesis_default_teyrchain_id_is_100() {
	new_test_ext().execute_with(|| {
		let id = TeyrchainInfo::teyrchain_id();
		assert_eq!(id, ParaId::from(100u32));
	});
}

#[test]
fn genesis_custom_teyrchain_id() {
	new_test_ext_with_id(2000).execute_with(|| {
		let id = TeyrchainInfo::teyrchain_id();
		assert_eq!(id, ParaId::from(2000u32));
	});
}

#[test]
fn teyrchain_id_getter_returns_stored_value() {
	new_test_ext_with_id(1234).execute_with(|| {
		assert_eq!(TeyrchainInfo::teyrchain_id(), ParaId::from(1234u32));
	});
}

#[test]
fn get_trait_returns_teyrchain_id() {
	new_test_ext_with_id(555).execute_with(|| {
		let id = <Pezpallet<Test> as Get<ParaId>>::get();
		assert_eq!(id, ParaId::from(555u32));
	});
}

#[test]
fn get_trait_matches_getter() {
	new_test_ext_with_id(999).execute_with(|| {
		let from_getter = TeyrchainInfo::teyrchain_id();
		let from_trait = <Pezpallet<Test> as Get<ParaId>>::get();
		assert_eq!(from_getter, from_trait);
	});
}
