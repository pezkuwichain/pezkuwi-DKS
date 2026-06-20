// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

use pezbp_header_pez_chain::ChainWithGrandpa;
use pezbp_pezkuwi_core::teyrchains::ParaId;
use pezbp_runtime::{Chain, ChainId, Teyrchain};
use pezframe_support::{
	construct_runtime, derive_impl, parameter_types, traits::ConstU32, weights::Weight,
};
use pezsp_runtime::{
	testing::H256,
	traits::{BlakeTwo256, Header as HeaderT},
	MultiSignature, StateVersion,
};

use crate as pezpallet_bridge_teyrchains;

pub type AccountId = u64;

pub type RelayBlockHeader =
	pezsp_runtime::generic::Header<crate::RelayBlockNumber, crate::RelayBlockHasher>;

type Block = pezframe_system::mocking::MockBlock<TestRuntime>;

pub const PARAS_PALLET_NAME: &str = "Paras";
pub const UNTRACKED_TEYRCHAIN_ID: u32 = 10;
// use exact expected encoded size: `vec_len_size + header_number_size + state_root_hash_size`
pub const MAXIMAL_TEYRCHAIN_HEAD_DATA_SIZE: u32 = 1 + 8 + 32;
// total teyrchains that we use in tests
pub const TOTAL_TEYRCHAINS: u32 = 4;

pub type RegularTeyrchainHeader = pezsp_runtime::testing::Header;
pub type RegularTeyrchainHasher = BlakeTwo256;
pub type BigTeyrchainHeader = pezsp_runtime::generic::Header<u128, BlakeTwo256>;

pub struct Teyrchain1;

impl Chain for Teyrchain1 {
	const ID: ChainId = *b"pch1";

	type BlockNumber = u64;
	type Hash = H256;
	type Hasher = RegularTeyrchainHasher;
	type Header = RegularTeyrchainHeader;
	type AccountId = u64;
	type Balance = u64;
	type Nonce = u64;
	type Signature = MultiSignature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		0
	}
	fn max_extrinsic_weight() -> Weight {
		Weight::zero()
	}
}

impl Teyrchain for Teyrchain1 {
	const TEYRCHAIN_ID: u32 = 1;
	const MAX_HEADER_SIZE: u32 = 1_024;
}

pub struct Teyrchain2;

impl Chain for Teyrchain2 {
	const ID: ChainId = *b"pch2";

	type BlockNumber = u64;
	type Hash = H256;
	type Hasher = RegularTeyrchainHasher;
	type Header = RegularTeyrchainHeader;
	type AccountId = u64;
	type Balance = u64;
	type Nonce = u64;
	type Signature = MultiSignature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		0
	}
	fn max_extrinsic_weight() -> Weight {
		Weight::zero()
	}
}

impl Teyrchain for Teyrchain2 {
	const TEYRCHAIN_ID: u32 = 2;
	const MAX_HEADER_SIZE: u32 = 1_024;
}

pub struct Teyrchain3;

impl Chain for Teyrchain3 {
	const ID: ChainId = *b"pch3";

	type BlockNumber = u64;
	type Hash = H256;
	type Hasher = RegularTeyrchainHasher;
	type Header = RegularTeyrchainHeader;
	type AccountId = u64;
	type Balance = u64;
	type Nonce = u64;
	type Signature = MultiSignature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		0
	}
	fn max_extrinsic_weight() -> Weight {
		Weight::zero()
	}
}

impl Teyrchain for Teyrchain3 {
	const TEYRCHAIN_ID: u32 = 3;
	const MAX_HEADER_SIZE: u32 = 1_024;
}

// this teyrchain is using u128 as block number and stored head data size exceeds limit
pub struct BigTeyrchain;

impl Chain for BigTeyrchain {
	const ID: ChainId = *b"bpch";

	type BlockNumber = u128;
	type Hash = H256;
	type Hasher = RegularTeyrchainHasher;
	type Header = BigTeyrchainHeader;
	type AccountId = u64;
	type Balance = u64;
	type Nonce = u64;
	type Signature = MultiSignature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		0
	}
	fn max_extrinsic_weight() -> Weight {
		Weight::zero()
	}
}

impl Teyrchain for BigTeyrchain {
	const TEYRCHAIN_ID: u32 = 4;
	const MAX_HEADER_SIZE: u32 = 2_048;
}

construct_runtime! {
	pub enum TestRuntime
	{
		System: pezframe_system::{Pezpallet, Call, Config<T>, Storage, Event<T>},
		Grandpa1: pezpallet_bridge_grandpa::<Instance1>::{Pezpallet, Event<T>},
		Grandpa2: pezpallet_bridge_grandpa::<Instance2>::{Pezpallet, Event<T>},
		Teyrchains: pezpallet_bridge_teyrchains::{Call, Pezpallet, Event<T>},
	}
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for TestRuntime {
	type Block = Block;
}

parameter_types! {
	pub const HeadersToKeep: u32 = 5;
	pub const FreeHeadersInterval: u32 = 15;
}

impl pezpallet_bridge_grandpa::Config<pezpallet_bridge_grandpa::Instance1> for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type BridgedChain = TestBridgedChain;
	type MaxFreeHeadersPerBlock = ConstU32<2>;
	type FreeHeadersInterval = FreeHeadersInterval;
	type HeadersToKeep = HeadersToKeep;
	type WeightInfo = ();
}

impl pezpallet_bridge_grandpa::Config<pezpallet_bridge_grandpa::Instance2> for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type BridgedChain = TestBridgedChain;
	type MaxFreeHeadersPerBlock = ConstU32<2>;
	type FreeHeadersInterval = FreeHeadersInterval;
	type HeadersToKeep = HeadersToKeep;
	type WeightInfo = ();
}

parameter_types! {
	pub const HeadsToKeep: u32 = 4;
	pub const ParasPalletName: &'static str = PARAS_PALLET_NAME;
	pub GetTenFirstTeyrchains: Vec<ParaId> = (0..10).map(ParaId).collect();
}

impl pezpallet_bridge_teyrchains::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type BridgesGrandpaPalletInstance = pezpallet_bridge_grandpa::Instance1;
	type ParasPalletName = ParasPalletName;
	type ParaStoredHeaderDataBuilder = (Teyrchain1, Teyrchain2, Teyrchain3, BigTeyrchain);
	type HeadsToKeep = HeadsToKeep;
	type MaxParaHeadDataSize = ConstU32<MAXIMAL_TEYRCHAIN_HEAD_DATA_SIZE>;
	type OnNewHead = ();
}

#[cfg(feature = "runtime-benchmarks")]
impl pezpallet_bridge_teyrchains::benchmarking::Config<()> for TestRuntime {
	fn teyrchains() -> Vec<ParaId> {
		vec![
			ParaId(Teyrchain1::TEYRCHAIN_ID),
			ParaId(Teyrchain2::TEYRCHAIN_ID),
			ParaId(Teyrchain3::TEYRCHAIN_ID),
		]
	}

	fn prepare_teyrchain_heads_proof(
		teyrchains: &[ParaId],
		_teyrchain_head_size: u32,
		_proof_params: pezbp_runtime::UnverifiedStorageProofParams,
	) -> (
		crate::RelayBlockNumber,
		crate::RelayBlockHash,
		pezbp_pezkuwi_core::teyrchains::ParaHeadsProof,
		Vec<(ParaId, pezbp_pezkuwi_core::teyrchains::ParaHash)>,
	) {
		// in mock run we only care about benchmarks correctness, not the benchmark results
		// => ignore size related arguments
		let (state_root, proof, teyrchains) =
			pezbp_test_utils::prepare_teyrchain_heads_proof::<RegularTeyrchainHeader>(
				teyrchains.iter().map(|p| (p.0, crate::tests::head_data(p.0, 1))).collect(),
			);
		let relay_genesis_hash = crate::tests::initialize(state_root);
		(0, relay_genesis_hash, proof, teyrchains)
	}
}

#[derive(Debug)]
pub struct TestBridgedChain;

impl Chain for TestBridgedChain {
	const ID: ChainId = *b"tbch";

	type BlockNumber = crate::RelayBlockNumber;
	type Hash = crate::RelayBlockHash;
	type Hasher = crate::RelayBlockHasher;
	type Header = RelayBlockHeader;

	type AccountId = AccountId;
	type Balance = u32;
	type Nonce = u32;
	type Signature = pezsp_runtime::testing::TestSignature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		unreachable!()
	}

	fn max_extrinsic_weight() -> Weight {
		unreachable!()
	}
}

impl ChainWithGrandpa for TestBridgedChain {
	const WITH_CHAIN_GRANDPA_PALLET_NAME: &'static str = "";
	const MAX_AUTHORITIES_COUNT: u32 = 16;
	const REASONABLE_HEADERS_IN_JUSTIFICATION_ANCESTRY: u32 = 8;
	const MAX_MANDATORY_HEADER_SIZE: u32 = 256;
	const AVERAGE_HEADER_SIZE: u32 = 64;
}

/// Return test externalities to use in tests.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	pezsp_io::TestExternalities::new(Default::default())
}

/// Run pezpallet test.
pub fn run_test<T>(test: impl FnOnce() -> T) -> T {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		System::reset_events();
		test()
	})
}

/// Return test relay chain header with given number.
pub fn test_relay_header(
	num: crate::RelayBlockNumber,
	state_root: crate::RelayBlockHash,
) -> RelayBlockHeader {
	RelayBlockHeader::new(
		num,
		Default::default(),
		state_root,
		Default::default(),
		Default::default(),
	)
}
