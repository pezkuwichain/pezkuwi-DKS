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
use crate::{common::*, imports::*};
use asset_hub_zagros_runtime::governance::pezpallet_custom_origins::Origin;
use emulated_integration_tests_common::impls::Teyrchain;

#[test]
fn assethub_can_authorize_upgrade_for_itself() {
	let code_hash = [1u8; 32].into();
	type AssetHubRuntime = <AssetHubZagros as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubZagros as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pezpallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![AssetHubRuntimeCall::System(pezframe_system::Call::authorize_upgrade {
				code_hash,
			})],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	// store preimage
	let call_hash = call_hash_of::<AssetHubZagros>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubZagros>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(pezsp_runtime::ModuleError {
			index: 93,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(
		CollectivesZagros::sibling_location_of(<AssetHubZagros as Teyrchain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pezpallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubZagros>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	AssetHubZagros::execute_with(|| {
		assert!(<AssetHubZagros as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubZagros>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	AssetHubZagros::execute_with(|| {
		assert_eq!(
			<AssetHubZagros as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash
		)
	});
}

#[test]
fn assethub_can_authorize_upgrade_for_relay_chain() {
	let code_hash = [1u8; 32].into();
	type AssetHubRuntime = <AssetHubZagros as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubZagros as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pezpallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![build_xcm_send_authorize_upgrade_call::<AssetHubZagros, Zagros>(
				AssetHubZagros::parent_location(),
				&code_hash,
				None,
			)],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubZagros>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubZagros>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(pezsp_runtime::ModuleError {
			index: 93,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(
		CollectivesZagros::sibling_location_of(<AssetHubZagros as Teyrchain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pezpallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubZagros>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	Zagros::execute_with(|| assert!(<Zagros as Chain>::System::authorized_upgrade().is_none()));

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubZagros>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	Zagros::execute_with(|| {
		assert_eq!(<Zagros as Chain>::System::authorized_upgrade().unwrap().code_hash(), &code_hash)
	});
}

#[test]
fn assethub_can_authorize_upgrade_for_system_chains() {
	type AssetHubRuntime = <AssetHubZagros as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubZagros as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubZagros as Chain>::RuntimeOrigin;

	let code_hash_bridge_hub = [2u8; 32].into();
	let code_hash_collectives = [3u8; 32].into();
	let code_hash_coretime = [4u8; 32].into();
	let code_hash_people = [5u8; 32].into();

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pezpallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![
				build_xcm_send_authorize_upgrade_call::<AssetHubZagros, BridgeHubZagros>(
					AssetHubZagros::sibling_location_of(BridgeHubZagros::para_id()),
					&code_hash_bridge_hub,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubZagros, CollectivesZagros>(
					AssetHubZagros::sibling_location_of(CollectivesZagros::para_id()),
					&code_hash_collectives,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubZagros, CoretimeZagros>(
					AssetHubZagros::sibling_location_of(CoretimeZagros::para_id()),
					&code_hash_coretime,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubZagros, PeopleZagros>(
					AssetHubZagros::sibling_location_of(PeopleZagros::para_id()),
					&code_hash_people,
					None,
				),
			],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubZagros>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubZagros>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(pezsp_runtime::ModuleError {
			index: 93,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(
		CollectivesZagros::sibling_location_of(<AssetHubZagros as Teyrchain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pezpallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubZagros>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	BridgeHubZagros::execute_with(|| {
		assert!(<BridgeHubZagros as Chain>::System::authorized_upgrade().is_none())
	});
	CollectivesZagros::execute_with(|| {
		assert!(<CollectivesZagros as Chain>::System::authorized_upgrade().is_none())
	});
	CoretimeZagros::execute_with(|| {
		assert!(<CoretimeZagros as Chain>::System::authorized_upgrade().is_none())
	});
	PeopleZagros::execute_with(|| {
		assert!(<PeopleZagros as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubZagros>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	BridgeHubZagros::execute_with(|| {
		assert_eq!(
			<BridgeHubZagros as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_bridge_hub
		)
	});
	CollectivesZagros::execute_with(|| {
		assert_eq!(
			<CollectivesZagros as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_collectives
		)
	});
	CoretimeZagros::execute_with(|| {
		assert_eq!(
			<CoretimeZagros as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_coretime
		)
	});
	PeopleZagros::execute_with(|| {
		assert_eq!(
			<PeopleZagros as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_people
		)
	});
}
