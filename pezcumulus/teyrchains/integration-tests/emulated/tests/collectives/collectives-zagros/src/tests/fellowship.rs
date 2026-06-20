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
use collectives_fellowship::pezpallet_fellowship_origins::Origin::Fellows as FellowsOrigin;
use pezframe_support::{assert_ok, pezsp_runtime::traits::Dispatchable};

#[test]
fn fellows_whitelist_call() {
	CollectivesZagros::execute_with(|| {
		type RuntimeEvent = <CollectivesZagros as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesZagros as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesZagros as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesZagros as Chain>::Runtime;
		type ZagrosCall = <Zagros as Chain>::RuntimeCall;
		type ZagrosRuntime = <Zagros as Chain>::Runtime;

		let call_hash = [1u8; 32].into();

		let whitelist_call = RuntimeCall::PezkuwiXcm(pezpallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::parent())),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					call: ZagrosCall::Whitelist(
						pezpallet_whitelist::Call::<ZagrosRuntime>::whitelist_call { call_hash }
					)
					.encode()
					.into(),
					fallback_max_weight: None
				}
			]))),
		});

		let fellows_origin: RuntimeOrigin = FellowsOrigin.into();

		assert_ok!(whitelist_call.dispatch(fellows_origin));

		assert_expected_events!(
			CollectivesZagros,
			vec![
				RuntimeEvent::PezkuwiXcm(pezpallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	Zagros::execute_with(|| {
		type RuntimeEvent = <Zagros as Chain>::RuntimeEvent;

		assert_expected_events!(
			Zagros,
			vec![
				RuntimeEvent::Whitelist(pezpallet_whitelist::Event::CallWhitelisted { .. }) => {},
				RuntimeEvent::MessageQueue(pezpallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}
