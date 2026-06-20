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

//! A generic prospective teyrchains subsystem mockup suitable to be used in benchmarks.

use futures::FutureExt;
use pezkuwi_node_subsystem::{
	messages::ProspectiveTeyrchainsMessage, overseer, SpawnedSubsystem, SubsystemError,
};
use pezkuwi_node_subsystem_types::OverseerSignal;
use pezkuwi_primitives::Hash;

pub struct MockProspectiveTeyrchains {}

impl MockProspectiveTeyrchains {
	pub fn new() -> Self {
		Self {}
	}
}

#[overseer::subsystem(ProspectiveTeyrchains, error=SubsystemError, prefix=self::overseer)]
impl<Context> MockProspectiveTeyrchains {
	fn start(self, ctx: Context) -> SpawnedSubsystem {
		let future = self.run(ctx).map(|_| Ok(())).boxed();

		SpawnedSubsystem { name: "test-environment", future }
	}
}

#[overseer::contextbounds(ProspectiveTeyrchains, prefix = self::overseer)]
impl MockProspectiveTeyrchains {
	async fn run<Context>(self, mut ctx: Context) {
		loop {
			let msg = ctx.recv().await.expect("Overseer never fails us");
			match msg {
				orchestra::FromOrchestra::Signal(signal) => {
					if signal == OverseerSignal::Conclude {
						return;
					}
				},
				orchestra::FromOrchestra::Communication { msg } => match msg {
					ProspectiveTeyrchainsMessage::GetMinimumRelayParents(_relay_parent, tx) => {
						tx.send(vec![]).unwrap();
					},
					ProspectiveTeyrchainsMessage::GetHypotheticalMembership(req, tx) => {
						tx.send(
							req.candidates
								.iter()
								.cloned()
								.map(|candidate| (candidate, vec![Hash::repeat_byte(0)]))
								.collect(),
						)
						.unwrap();
					},
					_ => {
						unimplemented!("Unexpected chain-api message")
					},
				},
			}
		}
	}
}
