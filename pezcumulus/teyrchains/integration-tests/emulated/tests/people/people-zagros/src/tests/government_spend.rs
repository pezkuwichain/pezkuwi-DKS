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

//! The register decides, the treasury pays: People sends the spend, the Asset Hub executes it.
//!
//! `welati::spend_budget` builds exactly this message and hands it to the router
//! (`welati.rs`, `send_government_spend`), and the Asset Hub gates the call behind
//! `GovernmentSpendOrigin = EnsureXcm<Equals<PeopleLocation>>`. Both halves were configured
//! and neither was ever run against the other: the pallet's own tests use a mock sender, and
//! every emulated People test drives the relay into People, never People out to a sibling.
//!
//! An origin check is not the only gate the message passes. The barrier runs first, and a
//! message that opens with `UnpaidExecution` needs the sender to be named in
//! `AllowExplicitUnpaidExecutionFrom`; People was not named there, so the message was refused
//! before the origin check was ever reached. Behind that sat a second fault: the hand-built
//! treasury call encoded the amount compactly, which is not how the call is declared.
//!
//! Neither could be seen from either side alone, and neither is loud: `spend_budget` docks the
//! approved budget and emits `BudgetSpent` as soon as the router accepts the message, so a
//! refusal on the Asset Hub leaves the register saying the money was spent.

use crate::imports::*;

use codec::Encode;
use pezframe_support::{pezsp_runtime::traits::Dispatchable, traits::fungibles::Mutate};
use pezpallet_assets::Instance1;
use teyrchains_common::{AccountId, Balance};

/// The Asset Hub, as People addresses it.
fn asset_hub() -> Location {
	Location::new(1, [Teyrchain(1000)])
}

#[test]
fn people_can_spend_from_the_government_pot_on_asset_hub() {
	let beneficiary: AccountId = [7u8; 32].into();
	let amount: Balance = 1_000_000_000_000;

	// Fund the government pot so a refusal here can only be about authority, never balance.
	//
	// In PEZ, not in HEZ. `spend_from_government_pot` pays in `T::PezAssetId`, an asset on this
	// chain, and the pot's native balance has no bearing on whether it can pay. Funding the
	// native side left the pot holding nothing it could spend, so the call failed on balance --
	// and `Transact` reports the enclosing message as processed either way, which is why the
	// only visible symptom was a missing event.
	AssetHubZagros::execute_with(|| {
		type Runtime = <AssetHubZagros as Chain>::Runtime;
		let pot = pezpallet_pez_treasury::Pezpallet::<Runtime>::government_pot_account_id();
		assert_ok!(<pezpallet_assets::Pezpallet<Runtime, Instance1> as Mutate<_>>::mint_into(
			<Runtime as pezpallet_pez_treasury::Config>::PezAssetId::get(),
			&pot,
			amount * 10,
		));
	});

	PeopleZagros::execute_with(|| {
		type Runtime = <PeopleZagros as Chain>::Runtime;
		type RuntimeCall = <PeopleZagros as Chain>::RuntimeCall;
		type RuntimeEvent = <PeopleZagros as Chain>::RuntimeEvent;
		type AssetHubCall = <AssetHubZagros as Chain>::RuntimeCall;
		type AssetHubRuntime = <AssetHubZagros as Chain>::Runtime;

		let spend = AssetHubCall::PezTreasury(
			pezpallet_pez_treasury::Call::<AssetHubRuntime>::spend_from_government_pot {
				beneficiary: beneficiary.clone(),
				amount,
			},
		);

		// The shape `send_government_spend` produces, instruction for instruction.
		let message = RuntimeCall::PezkuwiXcm(pezpallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(asset_hub())),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					call: spend.encode().into(),
					fallback_max_weight: None,
				},
			]))),
		});

		// Root is what a passed referendum dispatches as, and `EnsureXcmOrigin`'s root
		// fallback makes it speak as the chain itself -- the bare location the Asset Hub's
		// `Equals<PeopleLocation>` is written against.
		assert_ok!(message.dispatch(<PeopleZagros as Chain>::RuntimeOrigin::root()));

		assert_expected_events!(
			PeopleZagros,
			vec![
				RuntimeEvent::PezkuwiXcm(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::PezTreasury(
					pezpallet_pez_treasury::Event::GovernmentPotSpent { .. }
				) => {},
				RuntimeEvent::MessageQueue(
					pezpallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});
}
