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
use emulated_integration_tests_common::accounts::ALICE;
use pezframe_support::traits::schedule::DispatchTime;

/// A reward pool is opened on this chain, by an account on this chain.
///
/// It used to be opened from the relay: the relay's `Treasurer` sent a `Transact` and the
/// treasury body's sovereign account here paid for it. Both halves of that are gone. The
/// treasury lives on this chain now, the relay has no treasurer, and the relay's
/// `SendXcmOrigin` no longer carries an origin that could send this message -- so the test
/// could not be repaired, only pointed at the arrangement we actually have.
///
/// What it still covers is the wiring the cross-chain framing was only a delivery mechanism
/// for: `CreatePoolOrigin` here is `EnsureSigned`, so opening a pool is a local act, and this
/// asserts the pallet is reachable and its freezer and consideration are configured.
#[test]
fn an_asset_reward_pool_is_opened_on_this_chain() {
	AssetHubPezkuwichain::execute_with(|| {
		type Runtime = <AssetHubPezkuwichain as Chain>::Runtime;
		type RuntimeOrigin = <AssetHubPezkuwichain as Chain>::RuntimeOrigin;
		type Balances = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Balances;

		let creator = AssetHubPezkuwichain::account_id_of(ALICE);

		assert_ok!(Balances::force_set_balance(
			RuntimeOrigin::root(),
			creator.clone().into(),
			ASSET_HUB_PEZKUWICHAIN_ED * 100_000,
		));

		assert_ok!(pezpallet_asset_rewards::Pezpallet::<Runtime>::create_pool(
			RuntimeOrigin::signed(creator),
			bx!(RelayLocation::get()),
			bx!(RelayLocation::get()),
			1_000_000_000,
			DispatchTime::After(1_000_000_000),
			None,
		));

		assert_eq!(1, pezpallet_asset_rewards::Pools::<Runtime>::iter().count());
	});
}
