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

//! The teyrchain inherent data provider
//!
//! Teyrchain backing and approval is an off-chain process, but the teyrchain needs to progress on
//! chain as well. To make it progress on chain a block producer needs to forward information about
//! the state of a teyrchain to the runtime. This information is forwarded through an inherent to
//! the runtime. Here we provide the [`TeyrchainsInherentDataProvider`] that requests the relevant
//! data from the provisioner subsystem and creates the the inherent data that the runtime will use
//! to create an inherent.

#![deny(unused_crate_dependencies, unused_results)]

use futures::{select, FutureExt};
use pezkuwi_node_subsystem::{
	errors::SubsystemError, messages::ProvisionerMessage, overseer::Handle,
};
use pezkuwi_primitives::{Block, Hash, InherentData as TeyrchainsInherentData};
use std::{sync::Arc, time};

pub(crate) const LOG_TARGET: &str = "teyrchain::teyrchains-inherent";

/// How long to wait for the provisioner, before giving up.
const PROVISIONER_TIMEOUT: time::Duration = core::time::Duration::from_millis(2500);

/// Provides the teyrchains inherent data.
pub struct TeyrchainsInherentDataProvider<C: pezsp_blockchain::HeaderBackend<Block>> {
	pub client: Arc<C>,
	pub overseer: pezkuwi_overseer::Handle,
	pub parent: Hash,
}

impl<C: pezsp_blockchain::HeaderBackend<Block>> TeyrchainsInherentDataProvider<C> {
	/// Create a new [`Self`].
	pub fn new(client: Arc<C>, overseer: pezkuwi_overseer::Handle, parent: Hash) -> Self {
		TeyrchainsInherentDataProvider { client, overseer, parent }
	}

	/// Create a new instance of the [`TeyrchainsInherentDataProvider`].
	pub async fn create(
		client: Arc<C>,
		mut overseer: Handle,
		parent: Hash,
	) -> Result<TeyrchainsInherentData, Error> {
		let pid = async {
			let (sender, receiver) = futures::channel::oneshot::channel();
			gum::trace!(
				target: LOG_TARGET,
				relay_parent = ?parent,
				"Inherent data requested by Babe"
			);
			overseer.wait_for_activation(parent, sender).await;
			receiver
				.await
				.map_err(|_| Error::ClosedChannelAwaitingActivation)?
				.map_err(|e| Error::Subsystem(e))?;

			let (sender, receiver) = futures::channel::oneshot::channel();
			gum::trace!(
				target: LOG_TARGET,
				relay_parent = ?parent,
				"Requesting inherent data (after having waited for activation)"
			);
			overseer
				.send_msg(
					ProvisionerMessage::RequestInherentData(parent, sender),
					std::any::type_name::<Self>(),
				)
				.await;

			receiver.await.map_err(|_| Error::ClosedChannelAwaitingInherentData)
		};

		let mut timeout = futures_timer::Delay::new(PROVISIONER_TIMEOUT).fuse();

		let parent_header = match client.header(parent) {
			Ok(Some(h)) => h,
			Ok(None) => return Err(Error::ParentHeaderNotFound(parent)),
			Err(err) => return Err(Error::Blockchain(err)),
		};

		let res = select! {
			pid = pid.fuse() => pid,
			_ = timeout => Err(Error::Timeout),
		};

		let inherent_data = match res {
			Ok(pd) => TeyrchainsInherentData {
				bitfields: pd.bitfields.into_iter().map(Into::into).collect(),
				backed_candidates: pd.backed_candidates,
				disputes: pd.disputes,
				parent_header,
			},
			Err(err) => {
				gum::debug!(
					target: LOG_TARGET,
					%err,
					"Could not get provisioner inherent data; injecting default data",
				);
				TeyrchainsInherentData {
					bitfields: Vec::new(),
					backed_candidates: Vec::new(),
					disputes: Vec::new(),
					parent_header,
				}
			},
		};

		Ok(inherent_data)
	}
}

#[async_trait::async_trait]
impl<C: pezsp_blockchain::HeaderBackend<Block>> pezsp_inherents::InherentDataProvider
	for TeyrchainsInherentDataProvider<C>
{
	async fn provide_inherent_data(
		&self,
		dst_inherent_data: &mut pezsp_inherents::InherentData,
	) -> Result<(), pezsp_inherents::Error> {
		let inherent_data = TeyrchainsInherentDataProvider::create(
			self.client.clone(),
			self.overseer.clone(),
			self.parent,
		)
		.await
		.map_err(|e| pezsp_inherents::Error::Application(Box::new(e)))?;

		dst_inherent_data
			.put_data(pezkuwi_primitives::TEYRCHAINS_INHERENT_IDENTIFIER, &inherent_data)
	}

	async fn try_handle_error(
		&self,
		_identifier: &pezsp_inherents::InherentIdentifier,
		_error: &[u8],
	) -> Option<Result<(), pezsp_inherents::Error>> {
		// Inherent isn't checked and can not return any error
		None
	}
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Blockchain error")]
	Blockchain(#[from] pezsp_blockchain::Error),
	#[error("Timeout: provisioner did not return inherent data after {:?}", PROVISIONER_TIMEOUT)]
	Timeout,
	#[error("Could not find the parent header in the blockchain: {:?}", _0)]
	ParentHeaderNotFound(Hash),
	#[error("Closed channel from overseer when awaiting activation")]
	ClosedChannelAwaitingActivation,
	#[error("Closed channel from provisioner when awaiting inherent data")]
	ClosedChannelAwaitingInherentData,
	#[error("Subsystem failed")]
	Subsystem(#[from] SubsystemError),
}
