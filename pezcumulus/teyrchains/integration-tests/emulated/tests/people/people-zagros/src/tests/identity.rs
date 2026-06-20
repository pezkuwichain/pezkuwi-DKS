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

//! Tests related to cross-chain identity operations.

use crate::imports::*;
use codec::Encode;
use emulated_integration_tests_common::accounts::ALICE;
use people_zagros_runtime::people::{IdentityField, IdentityInfo};
use pezframe_support::BoundedVec;
use pezpallet_identity::Data;
use xcm::latest::AssetTransferFilter;

#[test]
fn set_identity_cross_chain() {
	type Identity = <PeopleZagros as PeopleZagrosPallet>::Identity;

	let asset_hub_zagros_alice = AssetHubZagros::account_id_of(ALICE);
	let people_zagros_alice = PeopleZagros::account_id_of(ALICE);
	AssetHubZagros::fund_accounts(vec![(asset_hub_zagros_alice.clone(), ZAGROS_ED * 10000)]);
	PeopleZagros::fund_accounts(vec![(people_zagros_alice.clone(), ZAGROS_ED * 10000)]);

	PeopleZagros::execute_with(|| {
		// No identity for Alice
		assert!(!Identity::has_identity(&people_zagros_alice, IdentityField::Email as u64));
	});

	let destination = AssetHubZagros::sibling_location_of(PeopleZagros::para_id());
	let total_fees: Asset = (Location::parent(), ZAGROS_ED * 1000).into();
	let fees: Asset = (Location::parent(), ZAGROS_ED * 500).into();
	AssetHubZagros::execute_with(|| {
		type RuntimeEvent = <AssetHubZagros as Chain>::RuntimeEvent;

		let identity_info = IdentityInfo {
			email: Data::Raw(b"test@test.io".to_vec().try_into().unwrap()),
			..Default::default()
		};
		// Set Alice identity on People from Alice on AH
		let set_identity_call =
			<PeopleZagros as Chain>::RuntimeCall::Identity(pezpallet_identity::Call::<
				<PeopleZagros as Chain>::Runtime,
			>::set_identity {
				info: bx!(identity_info),
			});
		let xcm_message = Xcm::<()>(vec![
			WithdrawAsset(total_fees.into()),
			PayFees { asset: fees.clone() },
			InitiateTransfer {
				destination,
				remote_fees: Some(AssetTransferFilter::Teleport(fees.clone().into())),
				preserve_origin: true,
				assets: BoundedVec::new(),
				remote_xcm: Xcm(vec![
					// try to alias into `Alice` account local to People chain
					AliasOrigin(people_zagros_alice.clone().into()),
					// set identity for the local Alice account
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						call: set_identity_call.encode().into(),
						fallback_max_weight: None,
					},
					RefundSurplus,
					DepositAsset {
						assets: Wild(AllCounted(1)),
						beneficiary: people_zagros_alice.clone().into(),
					},
				]),
			},
			ExpectTransactStatus(MaybeErrorCode::Success),
			RefundSurplus,
			DepositAsset {
				assets: Wild(AllCounted(1)),
				beneficiary: asset_hub_zagros_alice.clone().into(),
			},
		]);

		let signed_origin =
			<AssetHubZagros as Chain>::RuntimeOrigin::signed(asset_hub_zagros_alice);
		assert_ok!(<AssetHubZagros as AssetHubZagrosPallet>::PezkuwiXcm::execute(
			signed_origin,
			bx!(xcm::VersionedXcm::from(xcm_message.into())),
			Weight::MAX
		));
		assert_expected_events!(
			AssetHubZagros,
			vec![
				RuntimeEvent::PezkuwiXcm(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeopleZagros::execute_with(|| {
		// Verify Alice on People now has identity
		assert!(Identity::has_identity(&people_zagros_alice, IdentityField::Email as u64));
	});
}
