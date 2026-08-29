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

/// The two franchises must not name the same subject, and neither may name the register's.
///
/// One ballot counts citizens, one each; the other counts holdings. A subject appearing on both
/// is a subject a holding can reach, and the register would be for sale.
///
/// This reads the three real track lists rather than a remembered copy of them. The version that
/// stood in the Asset Hub's own tests compared against a hardcoded array of the register's three
/// names and never looked at the relay -- a sentinel that keeps its own copy of the thing it
/// guards stops guarding it the first time the original is renamed. Only this crate can see all
/// three runtimes at once, so this is where the comparison belongs.
#[test]
fn no_subject_is_decided_by_two_different_electorates() {
	use pezkuwichain_system_emulated_network::{
		people_pezkuwichain_emulated_chain::people_pezkuwichain_runtime,
		pezkuwichain_emulated_chain::pezkuwichain_runtime,
	};
	use pezpallet_referenda::TracksInfo as _;

	fn names<T: pezpallet_referenda::TracksInfo<Balance, teyrchains_common::BlockNumber>>(
	) -> Vec<String> {
		T::tracks()
			.map(|t| String::from_utf8_lossy(&t.info.name).trim_end_matches('\0').to_string())
			.collect()
	}

	let economic = names::<asset_hub_pezkuwichain_runtime::governance::TracksInfo>();
	let state = names::<people_pezkuwichain_runtime::governance::TracksInfo>();
	let relay = names::<pezkuwichain_runtime::governance::TracksInfo>();

	// Nothing is empty, or every assertion below passes by saying nothing.
	assert!(!economic.is_empty() && !state.is_empty() && !relay.is_empty());

	for s in &state {
		assert!(!economic.contains(s), "`{s}` is decided by both citizens and holdings");
		assert!(!relay.contains(s), "`{s}` is a register subject and the relay weighs tokens");
	}

	// Neither token-weighted chain keeps a `root` track. Root reaches every chain in the
	// network, and on both of these the electorate is holdings -- so Root arrives from the
	// register's referendum instead, and a track named `root` here would be a way around it.
	for (chain, list) in [("asset hub", &economic), ("relay", &relay)] {
		assert!(
			!list.iter().any(|n| n == "root"),
			"the {chain} has a root track again, and holdings would decide the code"
		);
	}
}
/// The People chain's mirror of this runtime's `SessionKeys` decodes what this one produces.
///
/// People checks a registration before forwarding it, and it can only do that against a
/// structurally identical type it declares itself. If the two drift, People accepts a payload
/// this chain then refuses -- and People is left holding a record of a key that was never
/// registered here, which is the one disagreement the design is built to prevent.
#[test]
fn the_relay_key_mirror_matches() {
	use codec::{Decode, Encode};
	use pezkuwichain_system_emulated_network::{
		people_pezkuwichain_emulated_chain::people_pezkuwichain_runtime,
		pezkuwichain_emulated_chain::pezkuwichain_runtime,
	};

	let ours = pezkuwichain_runtime::SessionKeys {
		grandpa: pezsp_core::ed25519::Public::from_raw([1u8; 32]).into(),
		babe: pezsp_core::sr25519::Public::from_raw([2u8; 32]).into(),
		para_validator: pezsp_core::sr25519::Public::from_raw([3u8; 32]).into(),
		para_assignment: pezsp_core::sr25519::Public::from_raw([4u8; 32]).into(),
		authority_discovery: pezsp_core::sr25519::Public::from_raw([5u8; 32]).into(),
		beefy: pezsp_core::ecdsa::Public::from_raw([6u8; 33]).into(),
	}
	.encode();

	let mirrored = people_pezkuwichain_runtime::people::RelaySessionKeys::decode(&mut &ours[..]);
	assert!(mirrored.is_ok(), "People's mirror cannot decode this chain's session keys");
	assert_eq!(
		mirrored.expect("decoded").encode(),
		ours,
		"the mirror decodes but re-encodes differently: a field has changed size"
	);

	// Round-tripping the bytes is not enough on its own. `grandpa` and `babe` are both 32
	// bytes, so swapping them decodes cleanly and re-encodes identically -- and every key
	// would then be registered against the wrong role. The key-type ids are what carry the
	// roles, so they are what has to match, in order.
	use pezsp_runtime::traits::OpaqueKeys;
	assert_eq!(
		people_pezkuwichain_runtime::people::RelaySessionKeys::key_ids(),
		pezkuwichain_runtime::SessionKeys::key_ids(),
		"the mirror names the same keys in a different order, or names different keys"
	);
}
