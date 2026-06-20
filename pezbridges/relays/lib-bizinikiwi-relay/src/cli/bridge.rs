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

//! Basic traits for exposing bridges in the CLI.

use crate::{
	equivocation::BizinikiwiEquivocationDetectionPipeline,
	finality::BizinikiwiFinalitySyncPipeline,
	messages::{BizinikiwiMessageLane, MessagesRelayLimits},
	teyrchains::BizinikiwiTeyrchainsPipeline,
};
use pezbp_teyrchains::{RelayBlockHash, RelayBlockHasher, RelayBlockNumber};
use relay_bizinikiwi_client::{
	Chain, ChainWithRuntimeVersion, ChainWithTransactions, RelayChain, Teyrchain,
};

/// Minimal bridge representation that can be used from the CLI.
/// It connects a source chain to a target chain.
pub trait CliBridgeBase: Sized {
	/// The source chain.
	type Source: Chain + ChainWithRuntimeVersion;
	/// The target chain.
	type Target: ChainWithTransactions + ChainWithRuntimeVersion;
}

/// Bridge representation that can be used from the CLI for relaying headers
/// from a relay chain to a relay chain.
pub trait RelayToRelayHeadersCliBridge: CliBridgeBase {
	/// Finality proofs synchronization pipeline.
	type Finality: BizinikiwiFinalitySyncPipeline<
		SourceChain = Self::Source,
		TargetChain = Self::Target,
	>;
}

/// Convenience trait that adds bounds to `CliBridgeBase`.
pub trait RelayToRelayEquivocationDetectionCliBridgeBase: CliBridgeBase {
	/// The source chain with extra bounds.
	type BoundedSource: ChainWithTransactions;
}

impl<T> RelayToRelayEquivocationDetectionCliBridgeBase for T
where
	T: CliBridgeBase,
	T::Source: ChainWithTransactions,
{
	type BoundedSource = T::Source;
}

/// Bridge representation that can be used from the CLI for detecting equivocations
/// in the headers synchronized from a relay chain to a relay chain.
pub trait RelayToRelayEquivocationDetectionCliBridge:
	RelayToRelayEquivocationDetectionCliBridgeBase
{
	/// Equivocation detection pipeline.
	type Equivocation: BizinikiwiEquivocationDetectionPipeline<
		SourceChain = Self::Source,
		TargetChain = Self::Target,
	>;
}

/// Bridge representation that can be used from the CLI for relaying headers
/// from a teyrchain to a relay chain.
pub trait TeyrchainToRelayHeadersCliBridge: CliBridgeBase
where
	Self::Source: Teyrchain,
{
	/// The `CliBridgeBase` type represents the teyrchain in this situation.
	/// We need to add an extra type for the relay chain.
	type SourceRelay: Chain<BlockNumber = RelayBlockNumber, Hash = RelayBlockHash, Hasher = RelayBlockHasher>
		+ ChainWithRuntimeVersion
		+ RelayChain;
	/// Finality proofs synchronization pipeline (source teyrchain -> target).
	type TeyrchainFinality: BizinikiwiTeyrchainsPipeline<
		SourceRelayChain = Self::SourceRelay,
		SourceTeyrchain = Self::Source,
		TargetChain = Self::Target,
	>;
	/// Finality proofs synchronization pipeline (source relay chain -> target).
	type RelayFinality: BizinikiwiFinalitySyncPipeline<
		SourceChain = Self::SourceRelay,
		TargetChain = Self::Target,
	>;
}

/// Bridge representation that can be used from the CLI for relaying messages.
pub trait MessagesCliBridge: CliBridgeBase {
	/// The Source -> Destination messages synchronization pipeline.
	type MessagesLane: BizinikiwiMessageLane<SourceChain = Self::Source, TargetChain = Self::Target>;

	/// Optional messages delivery transaction limits that the messages relay is going
	/// to use. If it returns `None`, limits are estimated using `TransactionPayment` API
	/// at the target chain.
	fn maybe_messages_limits() -> Option<MessagesRelayLimits> {
		None
	}
}

/// An alias for lane identifier type.
pub type MessagesLaneIdOf<B> =
	<<B as MessagesCliBridge>::MessagesLane as BizinikiwiMessageLane>::LaneId;
