// Copyright (C) Dijital Kurdistan Tech Institute
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

//! Who may start the PEZ release schedule.
//!
//! `pezpallet-pez-treasury` cannot mint and has no manual release. Its entire external surface
//! is one extrinsic, `activate_distribution`, and whoever can reach it decides when a state
//! that may not yet have enough citizens starts paying them -- an irreversible latch on a
//! fixed supply. The runtime binds it to `EnsureXcm<Equals<PeopleLocation>>`, because the
//! citizen register lives on the People chain and no other body is in a position to know.
//!
//! The pallet's own tests stand Root in for that origin, because a mock has no siblings. That
//! makes the binding itself -- the part that actually keeps other chains out -- untested
//! there. It is tested here, against the real runtime, by having each chain that might try it
//! actually try it.

use crate::imports::*;
use emulated_integration_tests_common::xcm_helpers::xcm_transact_paid_execution;
use teyrchains_common::AccountId;
use xcm::DoubleEncoded;

type AssetHubRuntime = <AssetHubPezkuwichain as Chain>::Runtime;
type AssetHubRuntimeCall = <AssetHubPezkuwichain as Chain>::RuntimeCall;

/// The call every test in this file tries to get executed on Asset Hub.
fn activation_call() -> DoubleEncoded<()> {
	AssetHubRuntimeCall::PezTreasury(
		pezpallet_pez_treasury::Call::<AssetHubRuntime>::activate_distribution {},
	)
	.encode()
	.into()
}

/// Whether Asset Hub has started the schedule.
fn schedule_started() -> bool {
	AssetHubPezkuwichain::execute_with(|| {
		pezpallet_pez_treasury::DistributionStarted::<AssetHubRuntime>::get()
	})
}

/// Give a sibling chain's sovereign account on Asset Hub enough to pay for execution.
///
/// Without this the message fails on fees rather than on origin, and a test that cannot tell
/// those apart proves nothing about who is allowed in.
fn fund_sovereign_account_of(para_id: u32) -> AccountId {
	let sovereign =
		AssetHubPezkuwichain::sovereign_account_id_of(Location::new(1, [Teyrchain(para_id)]));
	AssetHubPezkuwichain::fund_accounts(vec![(
		sovereign.clone(),
		ASSET_HUB_PEZKUWICHAIN_ED * 10_000_000_000,
	)]);
	sovereign
}

/// The XCM a sibling sends: pay for execution, then `Transact` the activation call.
fn activation_xcm(fee_payer: AccountId) -> VersionedXcm<()> {
	let fee_amount = ASSET_HUB_PEZKUWICHAIN_ED * 1_000_000;
	xcm_transact_paid_execution(
		activation_call(),
		OriginKind::Xcm,
		(Parent, fee_amount).into(),
		fee_payer,
	)
}

/// The People chain says the citizen register has passed the threshold, and it is believed.
#[test]
fn the_people_chain_can_start_the_schedule() {
	let fee_payer = fund_sovereign_account_of(PeoplePezkuwichain::para_id().into());
	let destination = PeoplePezkuwichain::sibling_location_of(AssetHubPezkuwichain::para_id());

	assert!(!schedule_started(), "the schedule was already running before the test began");

	PeoplePezkuwichain::execute_with(|| {
		assert_ok!(<PeoplePezkuwichain as PeoplePezkuwichainPallet>::PezkuwiXcm::send(
			<PeoplePezkuwichain as Chain>::RuntimeOrigin::root(),
			bx!(destination.into()),
			bx!(activation_xcm(fee_payer)),
		));
		PeoplePezkuwichain::assert_xcm_pezpallet_sent();
	});

	AssetHubPezkuwichain::execute_with(|| {
		type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
		AssetHubPezkuwichain::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubPezkuwichain,
			vec![
				RuntimeEvent::PezTreasury(
					pezpallet_pez_treasury::Event::TreasuryInitialized { .. }
				) => {},
			]
		);
	});

	assert!(schedule_started(), "the People chain was not able to start the schedule");
}

/// Every other chain is turned away.
///
/// These are the cases that matter. A bridge hub that can start the schedule means anything
/// that can reach the bridge can start it; an ordinary teyrchain that can means any teyrchain
/// can. Both must fail, and must fail without leaving the latch down.
#[test]
fn the_bridge_hub_cannot_start_the_schedule() {
	let para_id = BridgeHubPezkuwichain::para_id();
	let fee_payer = fund_sovereign_account_of(para_id.into());
	let destination = BridgeHubPezkuwichain::sibling_location_of(AssetHubPezkuwichain::para_id());

	assert!(!schedule_started());

	BridgeHubPezkuwichain::execute_with(|| {
		assert_ok!(<BridgeHubPezkuwichain as BridgeHubPezkuwichainPallet>::PezkuwiXcm::send(
			<BridgeHubPezkuwichain as Chain>::RuntimeOrigin::root(),
			bx!(destination.into()),
			bx!(activation_xcm(fee_payer)),
		));
	});

	AssetHubPezkuwichain::execute_with(|| {
		assert_no_activation();
	});
	assert!(!schedule_started(), "the bridge hub started the schedule");
}

#[test]
fn an_ordinary_teyrchain_cannot_start_the_schedule() {
	let para_id = PenpalA::para_id();
	let fee_payer = fund_sovereign_account_of(para_id.into());
	let destination = PenpalA::sibling_location_of(AssetHubPezkuwichain::para_id());

	assert!(!schedule_started());

	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as PenpalAPallet>::PezkuwiXcm::send(
			<PenpalA as Chain>::RuntimeOrigin::root(),
			bx!(destination.into()),
			bx!(activation_xcm(fee_payer)),
		));
	});

	AssetHubPezkuwichain::execute_with(|| {
		assert_no_activation();
	});
	assert!(!schedule_started(), "an ordinary teyrchain started the schedule");
}

/// Assert that Asset Hub processed a message and did not activate.
///
/// The message is allowed to arrive and be executed -- what must not happen is the call
/// succeeding. Checking for the absence of `TreasuryInitialized` is the direct statement of
/// that, and it holds whether the executor rejects the origin or the extrinsic does.
fn assert_no_activation() {
	type RuntimeEvent = <AssetHubPezkuwichain as Chain>::RuntimeEvent;
	assert!(
		!<AssetHubPezkuwichain as Chain>::events().iter().any(|e| matches!(
			e,
			RuntimeEvent::PezTreasury(pezpallet_pez_treasury::Event::TreasuryInitialized { .. })
		)),
		"the treasury was initialised by a chain that is not the People chain"
	);
}
