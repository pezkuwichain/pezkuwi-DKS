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
use codec::Encode;
use pezframe_support::{
	assert_ok, pezsp_runtime::traits::Dispatchable, traits::schedule::DispatchTime,
};
use xcm_executor::traits::ConvertLocation;

#[test]
fn treasury_creates_asset_reward_pool() {
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;
		type Balances = <AssetHubZagros as AssetHubZagrosPallet>::Balances;

		let treasurer =
			Location::new(1, [Plurality { id: BodyId::Treasury, part: BodyPart::Voice }]);
		let treasurer_account =
			ahw_xcm_config::LocationToAccountId::convert_location(&treasurer).unwrap();

		assert_ok!(Balances::force_set_balance(
			<AssetHubZagros as Chain>::RuntimeOrigin::root(),
			treasurer_account.clone().into(),
			ASSET_HUB_ZAGROS_ED * 100_000,
		));

		let events = AssetHubZagros::events();
		match events.iter().last() {
			Some(RuntimeEvent::Balances(pezpallet_balances::Event::BalanceSet { who, .. })) => {
				assert_eq!(*who, treasurer_account)
			},
			_ => panic!("Expected Balances::BalanceSet event"),
		}
	});
	Zagros::execute_with(|| {
		type AssetHubZagrosRuntimeCall = <AssetHubZagros as Chain>::RuntimeCall;
		type AssetHubZagrosRuntime = <AssetHubZagros as Chain>::Runtime;
		type ZagrosRuntimeCall = <Zagros as Chain>::RuntimeCall;
		type ZagrosRuntime = <Zagros as Chain>::Runtime;
		type ZagrosRuntimeEvent = <Zagros as Chain>::RuntimeEvent;
		type ZagrosRuntimeOrigin = <Zagros as Chain>::RuntimeOrigin;

		Dmp::make_teyrchain_reachable(AssetHubZagros::para_id());

		let staked_asset_id = bx!(RelayLocation::get());
		let reward_asset_id = bx!(RelayLocation::get());

		let reward_rate_per_block = 1_000_000_000;
		let lifetime = 1_000_000_000;
		let admin = None;

		let create_pool_call =
			ZagrosRuntimeCall::XcmPallet(pezpallet_xcm::Call::<ZagrosRuntime>::send {
				dest: bx!(VersionedLocation::V4(
					xcm::v4::Junction::Teyrchain(AssetHubZagros::para_id().into()).into()
				)),
				message: bx!(VersionedXcm::V5(Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						fallback_max_weight: None,
						call: AssetHubZagrosRuntimeCall::AssetRewards(
							pezpallet_asset_rewards::Call::<AssetHubZagrosRuntime>::create_pool {
								staked_asset_id,
								reward_asset_id,
								reward_rate_per_block,
								expiry: DispatchTime::After(lifetime),
								admin
							}
						)
						.encode()
						.into(),
					}
				]))),
			});

		let treasury_origin: ZagrosRuntimeOrigin = Treasurer.into();
		assert_ok!(create_pool_call.dispatch(treasury_origin));

		assert_expected_events!(
			Zagros,
			vec![
				ZagrosRuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type Runtime = <AssetHubZagros as Chain>::Runtime;
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_eq!(1, pezpallet_asset_rewards::Pools::<Runtime>::iter().count());

		let events = AssetHubZagros::events();
		match events.iter().last() {
			Some(RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed {
				success: true,
				..
			})) => (),
			_ => panic!("Expected MessageQueue::Processed event"),
		}
	});
}
