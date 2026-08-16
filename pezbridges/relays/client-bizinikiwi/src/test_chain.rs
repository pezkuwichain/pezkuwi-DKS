// Copyright 2019-2021 Parity Technologies (UK) Ltd.
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

//! Pezpallet provides a set of guard functions that are running in background threads
//! and are aborting process if some condition fails.

//! Test chain implementation to use in tests.

#![cfg(any(feature = "test-helpers", test))]

use crate::{
	Chain, ChainWithBalances, ChainWithMessages, ChainWithRewards, ChainWithTransactions,
	Error as BizinikiwiError, SignParam, UnsignedTransaction,
};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use pezbp_messages::{ChainWithMessages as ChainWithMessagesBase, MessageNonce};
use pezbp_runtime::ChainId;
use pezframe_support::{pezsp_runtime::StateVersion, weights::Weight};
use scale_info::TypeInfo;
use std::time::Duration;

/// Chain that may be used in tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestChain;

impl pezbp_runtime::Chain for TestChain {
	const ID: ChainId = *b"test";

	type BlockNumber = u32;
	type Hash = pezsp_core::H256;
	type Hasher = pezsp_runtime::traits::BlakeTwo256;
	type Header = pezsp_runtime::generic::Header<u32, pezsp_runtime::traits::BlakeTwo256>;

	type AccountId = u32;
	type Balance = u32;
	type Nonce = u32;
	type Signature = pezsp_runtime::testing::TestSignature;

	const STATE_VERSION: StateVersion = StateVersion::V1;

	fn max_extrinsic_size() -> u32 {
		100000
	}

	fn max_extrinsic_weight() -> Weight {
		unreachable!()
	}
}

impl Chain for TestChain {
	const NAME: &'static str = "Test";
	const BEST_FINALIZED_HEADER_ID_METHOD: &'static str = "TestMethod";
	const FREE_HEADERS_INTERVAL_METHOD: &'static str = "TestMethod";
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_millis(0);

	type SignedBlock = pezsp_runtime::generic::SignedBlock<
		pezsp_runtime::generic::Block<Self::Header, pezsp_runtime::OpaqueExtrinsic>,
	>;
	type Call = TestRuntimeCall;
}

impl ChainWithBalances for TestChain {
	fn account_info_storage_key(_account_id: &u32) -> pezsp_core::storage::StorageKey {
		unreachable!()
	}
}

/// Reward type for the test chain.
#[derive(
	Clone,
	Copy,
	Debug,
	Decode,
	DecodeWithMemTracking,
	Encode,
	Eq,
	MaxEncodedLen,
	PartialEq,
	TypeInfo,
)]
pub enum ChainReward {
	/// Reward 1 type.
	Reward1,
}

impl ChainWithRewards for TestChain {
	const WITH_CHAIN_RELAYERS_PALLET_NAME: Option<&'static str> = None;
	type RewardBalance = u128;
	type Reward = ChainReward;

	fn account_reward_storage_key(
		_account_id: &Self::AccountId,
		_reward: impl Into<Self::Reward>,
	) -> pezsp_core::storage::StorageKey {
		unreachable!()
	}
}

impl ChainWithMessagesBase for TestChain {
	const WITH_CHAIN_MESSAGES_PALLET_NAME: &'static str = "Test";
	const MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX: MessageNonce = 0;
	const MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX: MessageNonce = 0;
}

impl ChainWithMessages for TestChain {
	const TO_CHAIN_MESSAGE_DETAILS_METHOD: &'static str = "TestMessagesDetailsMethod";
	const FROM_CHAIN_MESSAGE_DETAILS_METHOD: &'static str = "TestFromMessagesDetailsMethod";
}

impl ChainWithTransactions for TestChain {
	type AccountKeyPair = pezsp_core::sr25519::Pair;
	type SignedTransaction = pezbp_pezkuwi_core::UncheckedExtrinsic<
		TestRuntimeCall,
		pezbp_pezkuwi_core::SuffixedCommonTransactionExtension<(
			pezbp_runtime::extensions::BridgeRejectObsoleteHeadersAndMessages,
			pezbp_runtime::extensions::RefundBridgedTeyrchainMessagesSchema,
		)>,
	>;

	fn sign_transaction(
		_param: SignParam<Self>,
		_unsigned: UnsignedTransaction<Self>,
	) -> Result<Self::SignedTransaction, BizinikiwiError> {
		unreachable!()
	}
}

/// Dummy runtime call.
#[derive(Decode, Encode, Clone, Debug, PartialEq)]
pub enum TestRuntimeCall {
	/// Dummy call.
	#[codec(index = 0)]
	Dummy,
}

/// Primitives-level teyrchain that may be used in tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestTeyrchainBase;

impl pezbp_runtime::Chain for TestTeyrchainBase {
	const ID: ChainId = *b"tstp";

	type BlockNumber = u32;
	type Hash = pezsp_core::H256;
	type Hasher = pezsp_runtime::traits::BlakeTwo256;
	type Header = pezsp_runtime::generic::Header<u32, pezsp_runtime::traits::BlakeTwo256>;

	type AccountId = u32;
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

impl pezbp_runtime::Teyrchain for TestTeyrchainBase {
	const TEYRCHAIN_ID: u32 = 1000;
	const MAX_HEADER_SIZE: u32 = 1_024;
}

/// Teyrchain that may be used in tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestTeyrchain;

impl pezbp_runtime::UnderlyingChainProvider for TestTeyrchain {
	type Chain = TestTeyrchainBase;
}

impl Chain for TestTeyrchain {
	const NAME: &'static str = "TestTeyrchain";
	const BEST_FINALIZED_HEADER_ID_METHOD: &'static str = "TestTeyrchainMethod";
	const FREE_HEADERS_INTERVAL_METHOD: &'static str = "TestTeyrchainMethod";
	const AVERAGE_BLOCK_INTERVAL: Duration = Duration::from_millis(0);

	type SignedBlock = pezsp_runtime::generic::SignedBlock<
		pezsp_runtime::generic::Block<Self::Header, pezsp_runtime::OpaqueExtrinsic>,
	>;
	type Call = TestRuntimeCall;
}
