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
use pezframe_support::{pezsp_runtime::traits::Dispatchable, traits::schedule::DispatchTime};
use xcm_executor::traits::ConvertLocation;

#[test]
fn treasury_creates_asset_reward_pool() {
	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		type Balances = <AssetHubPezkuwichain as AssetHubPezkuwichainPallet>::Balances;

		let treasurer =
			Location::new(1, [Plurality { id: BodyId::Treasury, part: BodyPart::Voice }]);
		let treasurer_account =
			ahr_xcm_config::LocationToAccountId::convert_location(&treasurer).unwrap();

		assert_ok!(Balances::force_set_balance(
			<AssetHubPezkuwichain as Chain>::RuntimeOrigin::root(),
			treasurer_account.clone().into(),
			ASSET_HUB_PEZKUWICHAIN_ED * 100_000,
		));

		let events = AssetHubPezkuwichain::events();
		match events.iter().last() {
			Some(RuntimeEvent::Balances(pezpallet_balances::Event::BalanceSet { who, .. })) => {
				assert_eq!(*who, treasurer_account)
			},
			_ => panic!("Expected Balances::BalanceSet event"),
		}
	});

	Pezkuwichain::execute_with(|| {
		type AssetHubPezkuwichainRuntimeCall = <AssetHubPezkuwichain as Chain>::RuntimeCall;
		type AssetHubPezkuwichainRuntime = <AssetHubPezkuwichain as Chain>::Runtime;
		type PezkuwichainRuntimeCall = <Pezkuwichain as Chain>::RuntimeCall;
		type PezkuwichainRuntime = <Pezkuwichain as Chain>::Runtime;
		type PezkuwichainRuntimeEvent = <Pezkuwichain as Chain>::RuntimeEvent;
		type PezkuwichainRuntimeOrigin = <Pezkuwichain as Chain>::RuntimeOrigin;

		Dmp::make_teyrchain_reachable(AssetHubPezkuwichain::para_id());

		let staked_asset_id = bx!(RelayLocation::get());
		let reward_asset_id = bx!(RelayLocation::get());

		let reward_rate_per_block = 1_000_000_000;
		let lifetime = 1_000_000_000;
		let admin = None;

		let create_pool_call =
			PezkuwichainRuntimeCall::XcmPallet(pezpallet_xcm::Call::<PezkuwichainRuntime>::send {
				dest: bx!(VersionedLocation::V4(
					xcm::v4::Junction::Teyrchain(AssetHubPezkuwichain::para_id().into()).into()
				)),
				message: bx!(VersionedXcm::V5(Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						fallback_max_weight: None,
						call: AssetHubPezkuwichainRuntimeCall::AssetRewards(
							pezpallet_asset_rewards::Call::<AssetHubPezkuwichainRuntime>::create_pool {
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

		let treasury_origin: PezkuwichainRuntimeOrigin = Treasurer.into();
		assert_ok!(create_pool_call.dispatch(treasury_origin));

		assert_expected_events!(
			Pezkuwichain,
			vec![
				PezkuwichainRuntimeEvent::XcmPallet(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	AssetHubPezkuwichain::execute_with(|| {
		type Runtime = <AssetHubPezkuwichain as Chain>::Runtime;
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;

		assert_eq!(1, pezpallet_asset_rewards::Pools::<Runtime>::iter().count());

		let events = AssetHubPezkuwichain::events();
		match events.iter().last() {
			Some(RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed {
				success: true,
				..
			})) => (),
			_ => panic!("Expected MessageQueue::Processed event"),
		}
	});
}
