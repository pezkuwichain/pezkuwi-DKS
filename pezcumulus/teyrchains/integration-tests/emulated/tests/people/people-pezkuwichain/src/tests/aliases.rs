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

//! Tests related to XCM aliasing.
//!
//! The twin of the Zagros People suite, asking the same questions with the same expected
//! answers. Two differences, both from the network rather than the runtime:
//!
//! - The cross-chain lists carry no Bridge Hub, Coretime or Collectives rows. The mainnet runs
//!   the relay, the Asset Hub and People only; those chains exist on Zagros alone.
//! - The user teyrchain is Penpal A, the Penpal instance whose relay network is set to this one.
//!   Zagros uses Penpal B for the same reason.

use crate::imports::*;
use emulated_integration_tests_common::{macros::AccountId, test_cross_chain_alias};
use pezframe_support::traits::ContainsPair;
use xcm::latest::Junctions::*;

const ALLOWED: bool = true;
const DENIED: bool = false;

const TELEPORT_FEES: bool = true;
const RESERVE_TRANSFER_FEES: bool = false;

/// Fee budget for one cross-chain alias attempt. The macro splits it in half: one half pays for
/// local execution and delivery on the sending chain, the other is carried to the remote leg.
///
/// Sized for the most expensive sender in the lists below, Penpal, which weighs messages with
/// `FixedWeightBounds` instead of benchmarked weights and so bills a five-instruction message
/// for 320kb of proof size — nineteen times what a system teyrchain measures for the same
/// message. Under-fund it and the transfer fails with `NotHoldingFees` before the alias these
/// tests exist to check is ever attempted, so the tests go red for a reason that is not aliasing.
///
/// The floor was measured on the Zagros twin of this suite: it fails at `ED * 27` and passes at
/// `* 28`. It carries here because every input to it is the same on this network -- the People
/// runtime is the twin, Penpal is the same runtime, and both relays set the existential deposit
/// to one cent. It is set well above that floor because the floor moves with any weight change,
/// and a budget sitting one ED above it turns every such change into an unrelated red test.
///
/// The multiplier is large because `PEZKUWICHAIN_ED` is one cent and our cent is
/// `UNITS / 30_000`, three hundred times smaller than the scale upstream's `ED * N` fee sizings were tuned at.
/// The absolute budget here is ordinary; only the number of EDs it takes to say it is unusual.
const ALIAS_FEES: u128 = PEZKUWICHAIN_ED * 40;

#[test]
fn account_on_sibling_syschain_aliases_into_same_local_account() {
	// origin and target are the same account on different chains
	let origin: AccountId = [1; 32].into();
	let target = origin.clone();
	let fees = ALIAS_FEES;

	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		Location::parent(),
		origin.clone(),
		fees * 10,
	);

	// Aliasing same account on different chains
	test_cross_chain_alias!(
		vec![
			// between AH and People: allowed
			(AssetHubPezkuwichain, PeoplePezkuwichain, TELEPORT_FEES, ALLOWED),
			// between Penpal and People: denied
			(PenpalA, PeoplePezkuwichain, RESERVE_TRANSFER_FEES, DENIED)
		],
		origin,
		target,
		fees
	);
}

#[test]
fn account_on_sibling_syschain_cannot_alias_into_different_local_account() {
	// origin and target are different accounts on different chains
	let origin: AccountId = [1; 32].into();
	let target: AccountId = [2; 32].into();
	let fees = ALIAS_FEES;

	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		Location::parent(),
		origin.clone(),
		fees * 10,
	);

	// Aliasing different account on different chains
	test_cross_chain_alias!(
		vec![
			// between AH and People: denied
			(AssetHubPezkuwichain, PeoplePezkuwichain, TELEPORT_FEES, DENIED),
			// between Penpal and People: denied
			(PenpalA, PeoplePezkuwichain, RESERVE_TRANSFER_FEES, DENIED)
		],
		origin,
		target,
		fees
	);
}

#[test]
fn aliasing_child_locations() {
	use PeoplePezkuwichainXcmConfig as XcmConfig;
	PeoplePezkuwichain::execute_with(|| {
		// Allows aliasing descendant of origin.
		let origin = Location::new(1, X1([PalletInstance(8)].into()));
		let target = Location::new(1, X2([PalletInstance(8), GeneralIndex(9)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Teyrchain(8)].into()));
		let target = Location::new(
			1,
			X2([Teyrchain(8), AccountId32 { network: None, id: [1u8; 32] }].into()),
		);
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Teyrchain(8)].into()));
		let target =
			Location::new(1, X3([Teyrchain(8), PalletInstance(8), GeneralIndex(9)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		// Does not allow if not descendant.
		let origin = Location::new(1, X1([PalletInstance(8)].into()));
		let target = Location::new(0, X2([PalletInstance(8), GeneralIndex(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Teyrchain(8)].into()));
		let target = Location::new(
			0,
			X2([Teyrchain(8), AccountId32 { network: None, id: [1u8; 32] }].into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Teyrchain(8)].into()));
		let target = Location::new(0, X1([AccountId32 { network: None, id: [1u8; 32] }].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([AccountId32 { network: None, id: [1u8; 32] }].into()));
		let target = Location::new(0, X1([AccountId32 { network: None, id: [1u8; 32] }].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}

#[test]
fn asset_hub_root_aliases_anything() {
	use PeoplePezkuwichainXcmConfig as XcmConfig;
	PeoplePezkuwichain::execute_with(|| {
		// Allows AH root to alias anything.
		let origin = Location::new(1, X1([Teyrchain(1000)].into()));

		let target = Location::new(1, X1([Teyrchain(2000)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(1, X1([AccountId32 { network: None, id: [1u8; 32] }].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(
			1,
			X2([Teyrchain(8), AccountId32 { network: None, id: [1u8; 32] }].into()),
		);
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target =
			Location::new(1, X3([Teyrchain(42), PalletInstance(8), GeneralIndex(9)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 1 })].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(2, X2([GlobalConsensus(Pezkuwi), Teyrchain(1000)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(0, X2([PalletInstance(8), GeneralIndex(9)].into()));
		assert!(<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		// Other AH locations cannot alias anything.
		let origin = Location::new(1, X2([Teyrchain(1000), GeneralIndex(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X2([Teyrchain(1000), PalletInstance(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(
			1,
			X2([Teyrchain(1000), AccountId32 { network: None, id: [1u8; 32] }].into()),
		);
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		// Other root locations cannot alias anything.
		let origin = Location::new(1, Here);
		let target = Location::new(2, X1([GlobalConsensus(Ethereum { chain_id: 1 })].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(2, X2([GlobalConsensus(Pezkuwi), Teyrchain(1000)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let target = Location::new(0, X2([PalletInstance(8), GeneralIndex(9)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));

		let origin = Location::new(0, Here);
		let target = Location::new(1, X1([Teyrchain(2000)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Teyrchain(1001)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
		let origin = Location::new(1, X1([Teyrchain(1002)].into()));
		assert!(!<XcmConfig as xcm_executor::Config>::Aliasers::contains(&origin, &target));
	});
}

#[test]
fn authorized_cross_chain_aliases() {
	// origin and target are different accounts on different chains
	let origin: AccountId = [100; 32].into();
	let bad_origin: AccountId = [150; 32].into();
	let target: AccountId = [200; 32].into();
	let fees = ALIAS_FEES;

	let pal_admin = <PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get());
	PenpalA::mint_foreign_asset(pal_admin.clone(), Location::parent(), origin.clone(), fees * 10);
	PenpalA::mint_foreign_asset(pal_admin, Location::parent(), bad_origin.clone(), fees * 10);
	PeoplePezkuwichain::fund_accounts(vec![(target.clone(), fees * 10)]);

	// let's authorize `origin` on Penpal to alias `target` on People
	PeoplePezkuwichain::execute_with(|| {
		let penpal_origin = Location::new(
			1,
			X2([
				Teyrchain(PenpalA::para_id().into()),
				AccountId32 {
					network: Some(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)),
					id: origin.clone().into(),
				},
			]
			.into()),
		);
		// `target` adds `penpal_origin` as authorized alias
		assert_ok!(
			<PeoplePezkuwichain as PeoplePezkuwichainPallet>::PezkuwiXcm::add_authorized_alias(
				<PeoplePezkuwichain as Chain>::RuntimeOrigin::signed(target.clone()),
				Box::new(penpal_origin.into()),
				None
			)
		);
	});
	// Verify that unauthorized `bad_origin` cannot alias into `target`, from any chain.
	test_cross_chain_alias!(
		vec![
			// between AH and People: denied
			(AssetHubPezkuwichain, PeoplePezkuwichain, TELEPORT_FEES, DENIED),
			// between Penpal and People: denied
			(PenpalA, PeoplePezkuwichain, RESERVE_TRANSFER_FEES, DENIED)
		],
		bad_origin,
		target,
		fees
	);
	// Verify that only authorized `penpal::origin` can alias into `target`, while `origin` on other
	// chains cannot.
	test_cross_chain_alias!(
		vec![
			// between AH and People: denied
			(AssetHubPezkuwichain, PeoplePezkuwichain, TELEPORT_FEES, DENIED),
			// between Penpal and People: allowed
			(PenpalA, PeoplePezkuwichain, RESERVE_TRANSFER_FEES, ALLOWED)
		],
		origin,
		target,
		fees
	);
	// remove authorization for `origin` on Penpal to alias `target` on People
	PeoplePezkuwichain::execute_with(|| {
		// `target` removes all authorized aliases
		assert_ok!(
			<PeoplePezkuwichain as PeoplePezkuwichainPallet>::PezkuwiXcm::remove_all_authorized_aliases(
				<PeoplePezkuwichain as Chain>::RuntimeOrigin::signed(target.clone())
			)
		);
	});
	// Verify `penpal::origin` can no longer alias into `target` on People.
	test_cross_chain_alias!(
		vec![(PenpalA, PeoplePezkuwichain, RESERVE_TRANSFER_FEES, DENIED)],
		origin,
		target,
		fees
	);
}
