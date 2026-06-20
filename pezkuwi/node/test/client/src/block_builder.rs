// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezkuwi.

// Pezkuwi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Pezkuwi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Pezkuwi.  If not, see <http://www.gnu.org/licenses/>.

use crate::Client;
use codec::{Decode, Encode};
use pezkuwi_primitives::{Block, InherentData as TeyrchainsInherentData};
use pezkuwi_test_runtime::UncheckedExtrinsic;
use pezkuwi_test_service::GetLastTimestamp;
use pezsc_block_builder::{BlockBuilder, BlockBuilderBuilder};
use pezsp_api::ProvideRuntimeApi;
use pezsp_consensus_babe::{
	digests::{PreDigest, SecondaryPlainPreDigest},
	BABE_ENGINE_ID,
};
use pezsp_runtime::{traits::Block as BlockT, Digest, DigestItem};
use pezsp_state_machine::BasicExternalities;

/// An extension for the test client to initialize a Pezkuwi specific block builder.
pub trait InitPezkuwiBlockBuilder {
	/// Init a Pezkuwi specific block builder that works for the test runtime.
	///
	/// This will automatically create and push the inherents for you to make the block valid for
	/// the test runtime.
	fn init_pezkuwi_block_builder(&self) -> pezsc_block_builder::BlockBuilder<'_, Block, Client>;

	/// Init a Pezkuwi specific block builder at a specific block that works for the test runtime.
	///
	/// Same as [`InitPezkuwiBlockBuilder::init_pezkuwi_block_builder`] besides that it takes a
	/// `Hash` to say which should be the parent block of the block that is being build.
	fn init_pezkuwi_block_builder_at(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> pezsc_block_builder::BlockBuilder<'_, Block, Client>;
}

impl InitPezkuwiBlockBuilder for Client {
	fn init_pezkuwi_block_builder(&self) -> BlockBuilder<'_, Block, Client> {
		let chain_info = self.chain_info();
		self.init_pezkuwi_block_builder_at(chain_info.best_hash)
	}

	fn init_pezkuwi_block_builder_at(
		&self,
		hash: <Block as BlockT>::Hash,
	) -> BlockBuilder<'_, Block, Client> {
		let last_timestamp =
			self.runtime_api().get_last_timestamp(hash).expect("Get last timestamp");

		// `MinimumPeriod` is a storage parameter type that requires externalities to access the
		// value.
		let minimum_period = BasicExternalities::new_empty()
			.execute_with(|| pezkuwi_test_runtime::MinimumPeriod::get());

		let timestamp = if last_timestamp == 0 {
			std::time::SystemTime::now()
				.duration_since(std::time::SystemTime::UNIX_EPOCH)
				.expect("Time is always after UNIX_EPOCH; qed")
				.as_millis() as u64
		} else {
			last_timestamp + minimum_period
		};

		// `SlotDuration` is a storage parameter type that requires externalities to access the
		// value.
		let slot_duration = BasicExternalities::new_empty()
			.execute_with(|| pezkuwi_test_runtime::SlotDuration::get());

		let slot = (timestamp / slot_duration).into();

		let digest = Digest {
			logs: vec![DigestItem::PreRuntime(
				BABE_ENGINE_ID,
				PreDigest::SecondaryPlain(SecondaryPlainPreDigest { slot, authority_index: 42 })
					.encode(),
			)],
		};

		let mut block_builder = BlockBuilderBuilder::new(self)
			.on_parent_block(hash)
			.fetch_parent_block_number(&self)
			.expect("Fetches parent block number")
			.with_inherent_digests(digest)
			.build()
			.expect("Creates new block builder for test runtime");

		let mut inherent_data = pezsp_inherents::InherentData::new();

		inherent_data
			.put_data(pezsp_timestamp::INHERENT_IDENTIFIER, &timestamp)
			.expect("Put timestamp inherent data");

		let parent_header = self
			.header(hash)
			.expect("Get the parent block header")
			.expect("The target block header must exist");

		let teyrchains_inherent_data = TeyrchainsInherentData {
			bitfields: Vec::new(),
			backed_candidates: Vec::new(),
			disputes: Vec::new(),
			parent_header,
		};

		inherent_data
			.put_data(pezkuwi_primitives::TEYRCHAINS_INHERENT_IDENTIFIER, &teyrchains_inherent_data)
			.expect("Put teyrchains inherent data");

		let inherents = block_builder.create_inherents(inherent_data).expect("Creates inherents");

		inherents
			.into_iter()
			.for_each(|ext| block_builder.push(ext).expect("Pushes inherent"));

		block_builder
	}
}

/// Pezkuwi specific extensions for the [`BlockBuilder`].
pub trait BlockBuilderExt {
	/// Push a Pezkuwi test runtime specific extrinsic to the block.
	///
	/// This will internally use the [`BlockBuilder::push`] method, but this method expects a opaque
	/// extrinsic. So, we provide this wrapper which converts a test runtime specific extrinsic to a
	/// opaque extrinsic and pushes it to the block.
	///
	/// Returns the result of the application of the extrinsic.
	fn push_pezkuwi_extrinsic(
		&mut self,
		ext: UncheckedExtrinsic,
	) -> Result<(), pezsp_blockchain::Error>;
}

impl BlockBuilderExt for BlockBuilder<'_, Block, Client> {
	fn push_pezkuwi_extrinsic(
		&mut self,
		ext: UncheckedExtrinsic,
	) -> Result<(), pezsp_blockchain::Error> {
		let encoded = ext.encode();
		self.push(
			Decode::decode(&mut &encoded[..]).expect(
				"The runtime specific extrinsic always decodes to an opaque extrinsic; qed",
			),
		)
	}
}
