// This file is part of Bizinikiwi.

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

use std::vec;

use codec::Encode;
use pezframe_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU32, ConstU64},
};
use pezsp_consensus_beefy::mmr::MmrLeafVersion;
use pezsp_io::TestExternalities;
use pezsp_runtime::{
	app_crypto::ecdsa::Public,
	impl_opaque_keys,
	traits::{ConvertInto, Keccak256, OpaqueKeys},
	BuildStorage,
};
use pezsp_state_machine::BasicExternalities;

use crate as pezpallet_beefy_mmr;

pub use pezsp_consensus_beefy::{
	ecdsa_crypto::AuthorityId as BeefyId, mmr::BeefyDataProvider, ConsensusLog, BEEFY_ENGINE_ID,
};
use pezsp_core::offchain::{testing::TestOffchainExt, OffchainDbExt, OffchainWorkerExt};

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: pezpallet_beefy::Pezpallet<Test>,
	}
}

type Block = pezframe_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		Session: pezpallet_session,
		Balances: pezpallet_balances,
		Mmr: pezpallet_mmr,
		Beefy: pezpallet_beefy,
		BeefyMmr: pezpallet_beefy_mmr,
	}
);
#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type AccountData = pezpallet_balances::AccountData<u64>;
	type Block = Block;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Test {
	type AccountStore = System;
}

impl pezpallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = u64;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = pezpallet_session::PeriodicSessions<ConstU64<1>, ConstU64<0>>;
	type NextSessionRotation = pezpallet_session::PeriodicSessions<ConstU64<1>, ConstU64<0>>;
	type SessionManager = MockSessionManager;
	type SessionHandler = <MockSessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = MockSessionKeys;
	type DisablingStrategy = ();
	type WeightInfo = ();
	type Currency = Balances;
	type KeyDeposit = ();
}

pub type MmrLeaf = pezsp_consensus_beefy::mmr::MmrLeaf<
	pezframe_system::pezpallet_prelude::BlockNumberFor<Test>,
	<Test as pezframe_system::Config>::Hash,
	crate::MerkleRootOf<Test>,
	Vec<u8>,
>;

impl pezpallet_mmr::Config for Test {
	const INDEXING_PREFIX: &'static [u8] = b"mmr";

	type Hashing = Keccak256;

	type LeafData = BeefyMmr;

	type OnNewRoot = pezpallet_beefy_mmr::DepositBeefyDigest<Test>;

	type BlockHashProvider = pezpallet_mmr::DefaultBlockHashProvider<Test>;

	type WeightInfo = ();

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pezpallet_beefy::Config for Test {
	type BeefyId = BeefyId;
	type MaxAuthorities = ConstU32<100>;
	type MaxNominators = ConstU32<1000>;
	type MaxSetIdSessionEntries = ConstU64<100>;
	type OnNewValidatorSet = BeefyMmr;
	type AncestryHelper = BeefyMmr;
	type WeightInfo = ();
	type KeyOwnerProof = pezsp_core::Void;
	type EquivocationReportSystem = ();
}

parameter_types! {
	pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(1, 5);
}

impl pezpallet_beefy_mmr::Config for Test {
	type LeafVersion = LeafVersion;

	type BeefyAuthorityToMerkleLeaf = pezpallet_beefy_mmr::BeefyEcdsaToEthereum;

	type LeafExtra = Vec<u8>;

	type BeefyDataProvider = DummyDataProvider;
	type WeightInfo = ();
}

pub struct DummyDataProvider;
impl BeefyDataProvider<Vec<u8>> for DummyDataProvider {
	fn extra_data() -> Vec<u8> {
		let mut col = vec![(15, vec![1, 2, 3]), (5, vec![4, 5, 6])];
		col.sort();
		pez_binary_merkle_tree::merkle_root::<<Test as pezpallet_mmr::Config>::Hashing, _>(
			col.into_iter().map(|pair| pair.encode()),
		)
		.as_ref()
		.to_vec()
	}
}

pub struct MockSessionManager;
impl pezpallet_session::SessionManager<u64> for MockSessionManager {
	fn end_session(_: pezsp_staking::SessionIndex) {}
	fn start_session(_: pezsp_staking::SessionIndex) {}
	fn new_session(idx: pezsp_staking::SessionIndex) -> Option<Vec<u64>> {
		if idx == 0 || idx == 1 {
			Some(vec![1, 2])
		} else if idx == 2 {
			Some(vec![3, 4])
		} else {
			None
		}
	}
}

// Note, that we can't use `UintAuthorityId` here. Reason is that the implementation
// of `to_public_key()` assumes, that a public key is 32 bytes long. This is true for
// ed25519 and sr25519 but *not* for ecdsa. A compressed ecdsa public key is 33 bytes,
// with the first one containing information to reconstruct the uncompressed key.
pub fn mock_beefy_id(id: u8) -> BeefyId {
	let mut buf: [u8; 33] = [id; 33];
	// Set to something valid.
	buf[0] = 0x02;
	let pk = Public::from_raw(buf);
	BeefyId::from(pk)
}

pub fn mock_authorities(vec: Vec<u8>) -> Vec<(u64, BeefyId)> {
	vec.into_iter().map(|id| ((id as u64), mock_beefy_id(id))).collect()
}

pub fn new_test_ext(ids: Vec<u8>) -> TestExternalities {
	new_test_ext_raw_authorities(mock_authorities(ids))
}

pub fn new_test_ext_raw_authorities(authorities: Vec<(u64, BeefyId)>) -> TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let session_keys: Vec<_> = authorities
		.iter()
		.enumerate()
		.map(|(_, id)| (id.0 as u64, id.0 as u64, MockSessionKeys { dummy: id.1.clone() }))
		.collect();

	BasicExternalities::execute_with_storage(&mut t, || {
		for (ref id, ..) in &session_keys {
			pezframe_system::Pezpallet::<Test>::inc_providers(id);
		}
	});

	pezpallet_session::GenesisConfig::<Test> { keys: session_keys, ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext: TestExternalities = t.into();
	let (offchain, _offchain_state) = TestOffchainExt::with_offchain_db(ext.offchain_db());
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));

	ext
}
