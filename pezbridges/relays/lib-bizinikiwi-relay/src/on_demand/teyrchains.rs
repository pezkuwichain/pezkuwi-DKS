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

//! On-demand Bizinikiwi -> Bizinikiwi teyrchain finality relay.

use crate::{
	messages::source::best_finalized_peer_header_at_self,
	on_demand::OnDemandRelay,
	teyrchains::{
		source::TeyrchainsSource, target::TeyrchainsTarget, BizinikiwiTeyrchainsPipeline,
		SubmitTeyrchainHeadsCallBuilder, TeyrchainsPipelineAdapter,
	},
	TransactionParams,
};

use async_std::{
	channel::{unbounded, Receiver, Sender},
	sync::{Arc, Mutex},
};
use async_trait::async_trait;
use futures::{select, FutureExt};
use num_traits::Zero;
use pezbp_pezkuwi_core::teyrchains::{ParaHash, ParaId};
use pezbp_runtime::HeaderIdProvider;
use pezbp_teyrchains::{RelayBlockHash, RelayBlockHasher, RelayBlockNumber};
use relay_bizinikiwi_client::{
	is_ancient_block, AccountIdOf, AccountKeyPairOf, BlockNumberOf, CallOf, Chain, Client,
	Error as BizinikiwiError, HashOf, HeaderIdOf, TeyrchainBase,
};
use relay_utils::{
	metrics::MetricsParams, relay_loop::Client as RelayClient, BlockNumberBase, FailedClient,
	HeaderId, UniqueSaturatedInto,
};
use std::fmt::Debug;
use teyrchains_relay::teyrchains_loop::{AvailableHeader, SourceClient, TargetClient};

/// On-demand Bizinikiwi <-> Bizinikiwi teyrchain finality relay.
///
/// This relay may be requested to sync more teyrchain headers, whenever some other relay
/// (e.g. messages relay) needs it to continue its regular work. When enough teyrchain headers
/// are relayed, on-demand stops syncing headers.
#[derive(Clone)]
pub struct OnDemandTeyrchainsRelay<P: BizinikiwiTeyrchainsPipeline, SourceRelayClnt, TargetClnt> {
	/// Relay task name.
	relay_task_name: String,
	/// Channel used to communicate with background task and ask for relay of teyrchain heads.
	required_header_number_sender: Sender<BlockNumberOf<P::SourceTeyrchain>>,
	/// Source relay chain client.
	source_relay_client: SourceRelayClnt,
	/// Target chain client.
	target_client: TargetClnt,
	/// On-demand relay chain relay.
	on_demand_source_relay_to_target_headers:
		Arc<dyn OnDemandRelay<P::SourceRelayChain, P::TargetChain>>,
}

impl<
		P: BizinikiwiTeyrchainsPipeline,
		SourceRelayClnt: Client<P::SourceRelayChain>,
		TargetClnt: Client<P::TargetChain>,
	> OnDemandTeyrchainsRelay<P, SourceRelayClnt, TargetClnt>
{
	/// Create new on-demand teyrchains relay.
	///
	/// Note that the argument is the source relay chain client, not the teyrchain client.
	/// That's because teyrchain finality is determined by the relay chain and we don't
	/// need to connect to the teyrchain itself here.
	pub fn new(
		source_relay_client: SourceRelayClnt,
		target_client: TargetClnt,
		target_transaction_params: TransactionParams<AccountKeyPairOf<P::TargetChain>>,
		on_demand_source_relay_to_target_headers: Arc<
			dyn OnDemandRelay<P::SourceRelayChain, P::TargetChain>,
		>,
	) -> Self
	where
		P::SourceTeyrchain: Chain<Hash = ParaHash>,
		P::SourceRelayChain:
			Chain<BlockNumber = RelayBlockNumber, Hash = RelayBlockHash, Hasher = RelayBlockHasher>,
		AccountIdOf<P::TargetChain>:
			From<<AccountKeyPairOf<P::TargetChain> as pezsp_core::Pair>::Public>,
	{
		let (required_header_number_sender, required_header_number_receiver) = unbounded();
		let this = OnDemandTeyrchainsRelay {
			relay_task_name: on_demand_teyrchains_relay_name::<P::SourceTeyrchain, P::TargetChain>(
			),
			required_header_number_sender,
			source_relay_client: source_relay_client.clone(),
			target_client: target_client.clone(),
			on_demand_source_relay_to_target_headers: on_demand_source_relay_to_target_headers
				.clone(),
		};
		async_std::task::spawn(async move {
			background_task::<P>(
				source_relay_client,
				target_client,
				target_transaction_params,
				on_demand_source_relay_to_target_headers,
				required_header_number_receiver,
			)
			.await;
		});

		this
	}
}

#[async_trait]
impl<P: BizinikiwiTeyrchainsPipeline, SourceRelayClnt, TargetClnt>
	OnDemandRelay<P::SourceTeyrchain, P::TargetChain>
	for OnDemandTeyrchainsRelay<P, SourceRelayClnt, TargetClnt>
where
	P::SourceTeyrchain: Chain<Hash = ParaHash>,
	SourceRelayClnt: Client<P::SourceRelayChain>,
	TargetClnt: Client<P::TargetChain>,
{
	async fn reconnect(&self) -> Result<(), BizinikiwiError> {
		// using clone is fine here (to avoid mut requirement), because clone on Client clones
		// internal references
		self.source_relay_client.clone().reconnect().await?;
		self.target_client.clone().reconnect().await?;
		// we'll probably need to reconnect relay chain relayer clients also
		self.on_demand_source_relay_to_target_headers.reconnect().await
	}

	async fn require_more_headers(&self, required_header: BlockNumberOf<P::SourceTeyrchain>) {
		if let Err(e) = self.required_header_number_sender.send(required_header).await {
			tracing::trace!(
				target: "bridge",
				error=?e,
				relay_task_name=%self.relay_task_name,
				source=%P::SourceTeyrchain::NAME,
				header=?required_header,
				"Failed to request"
			);
		}
	}

	/// Ask relay to prove source `required_header` to the `TargetChain`.
	async fn prove_header(
		&self,
		required_teyrchain_header: BlockNumberOf<P::SourceTeyrchain>,
	) -> Result<(HeaderIdOf<P::SourceTeyrchain>, Vec<CallOf<P::TargetChain>>), BizinikiwiError> {
		// select headers to prove
		let teyrchains_source = TeyrchainsSource::<P, _>::new(
			self.source_relay_client.clone(),
			Arc::new(Mutex::new(AvailableHeader::Missing)),
		);
		let env = (self, &teyrchains_source);
		let (need_to_prove_relay_block, selected_relay_block, selected_teyrchain_block) =
			select_headers_to_prove(env, required_teyrchain_header).await?;

		tracing::debug!(
			target: "bridge",
			relay_task_name=%self.relay_task_name,
			source=%P::SourceTeyrchain::NAME,
			?required_teyrchain_header,
			?selected_teyrchain_block,
			source_relay_chain=%P::SourceRelayChain::NAME,
			selected_relay_block=?if need_to_prove_relay_block {
				Some(selected_relay_block)
			} else {
				None
			},
			"Requested to prove head. Selected to prove head"
		);

		// now let's prove relay chain block (if needed)
		let mut calls = Vec::new();
		let mut proved_relay_block = selected_relay_block;
		if need_to_prove_relay_block {
			let (relay_block, relay_prove_call) = self
				.on_demand_source_relay_to_target_headers
				.prove_header(selected_relay_block.number())
				.await?;
			proved_relay_block = relay_block;
			calls.extend(relay_prove_call);
		}

		// despite what we've selected before (in `select_headers_to_prove` call), if headers relay
		// have chose the different header (e.g. because there's no GRANDPA jusstification for it),
		// we need to prove teyrchain head available at this header
		let para_id = ParaId(P::SourceTeyrchain::TEYRCHAIN_ID);
		let mut proved_teyrchain_block = selected_teyrchain_block;
		if proved_relay_block != selected_relay_block {
			proved_teyrchain_block = teyrchains_source
				.on_chain_para_head_id(proved_relay_block)
				.await?
				// this could happen e.g. if teyrchain has been offboarded?
				.ok_or_else(|| {
					BizinikiwiError::MissingRequiredTeyrchainHead(
						para_id,
						proved_relay_block.number().unique_saturated_into(),
					)
				})?;

			tracing::debug!(
				target: "bridge",
				relay_task_name=%self.relay_task_name,
				source=%P::SourceTeyrchain::NAME,
				?selected_teyrchain_block,
				source_relay_chain=%P::SourceRelayChain::NAME,
				?selected_relay_block,
				?proved_teyrchain_block,
				?proved_relay_block,
				"Selected to prove head. Instead proved head"
			);
		}

		// and finally - prove teyrchain head
		let (para_proof, para_hash) =
			teyrchains_source.prove_teyrchain_head(proved_relay_block).await?;
		calls.push(P::SubmitTeyrchainHeadsCallBuilder::build_submit_teyrchain_heads_call(
			proved_relay_block,
			vec![(para_id, para_hash)],
			para_proof,
			false,
		));

		Ok((proved_teyrchain_block, calls))
	}
}

/// Background task that is responsible for starting teyrchain headers relay.
async fn background_task<P: BizinikiwiTeyrchainsPipeline>(
	source_relay_client: impl Client<P::SourceRelayChain>,
	target_client: impl Client<P::TargetChain>,
	target_transaction_params: TransactionParams<AccountKeyPairOf<P::TargetChain>>,
	on_demand_source_relay_to_target_headers: Arc<
		dyn OnDemandRelay<P::SourceRelayChain, P::TargetChain>,
	>,
	required_teyrchain_header_number_receiver: Receiver<BlockNumberOf<P::SourceTeyrchain>>,
) where
	P::SourceTeyrchain: Chain<Hash = ParaHash>,
	P::SourceRelayChain:
		Chain<BlockNumber = RelayBlockNumber, Hash = RelayBlockHash, Hasher = RelayBlockHasher>,
	AccountIdOf<P::TargetChain>:
		From<<AccountKeyPairOf<P::TargetChain> as pezsp_core::Pair>::Public>,
{
	let relay_task_name = on_demand_teyrchains_relay_name::<P::SourceTeyrchain, P::TargetChain>();
	let target_transactions_mortality = target_transaction_params.mortality;

	let mut relay_state = RelayState::Idle;
	let mut required_teyrchain_header_number = Zero::zero();
	let required_para_header_ref = Arc::new(Mutex::new(AvailableHeader::Unavailable));

	let mut restart_relay = true;
	let teyrchains_relay_task = futures::future::Fuse::terminated();
	futures::pin_mut!(teyrchains_relay_task);

	let mut teyrchains_source = TeyrchainsSource::<P, _>::new(
		source_relay_client.clone(),
		required_para_header_ref.clone(),
	);
	let mut teyrchains_target = TeyrchainsTarget::<P, _, _>::new(
		source_relay_client.clone(),
		target_client.clone(),
		target_transaction_params.clone(),
	);

	loop {
		select! {
			new_required_teyrchain_header_number = required_teyrchain_header_number_receiver.recv().fuse() => {
				let new_required_teyrchain_header_number = match new_required_teyrchain_header_number {
					Ok(new_required_teyrchain_header_number) => new_required_teyrchain_header_number,
					Err(e) => {
						tracing::error!(
							target: "bridge",
							error=?e,
							%relay_task_name,
							"Background task has exited"
						);

						return;
					},
				};

				// keep in mind that we are not updating `required_para_header_ref` here, because
				// then we'll be submitting all previous headers as well (while required relay headers are
				// delivered) and we want to avoid that (to reduce cost)
				if new_required_teyrchain_header_number > required_teyrchain_header_number {
					tracing::trace!(
						target: "bridge",
						%relay_task_name,
						source=%P::SourceTeyrchain::NAME,
						%new_required_teyrchain_header_number,
						"More headers required. Going to sync up"
					);

					required_teyrchain_header_number = new_required_teyrchain_header_number;
				}
			},
			_ = async_std::task::sleep(P::TargetChain::AVERAGE_BLOCK_INTERVAL).fuse() => {},
			_ = teyrchains_relay_task => {
				// this should never happen in practice given the current code
				restart_relay = true;
			},
		}

		// the workflow of the on-demand teyrchains relay is:
		//
		// 1) message relay (or any other dependent relay) sees new message at teyrchain header
		// `PH`;
		//
		// 2) it sees that the target chain does not know `PH`;
		//
		// 3) it asks on-demand teyrchains relay to relay `PH` to the target chain;
		//
		// Phase#1: relaying relay chain header
		//
		// 4) on-demand teyrchains relay waits for GRANDPA-finalized block of the source relay chain
		//    `RH` that is storing `PH` or its descendant. Let it be `PH'`;
		// 5) it asks on-demand headers relay to relay `RH` to the target chain;
		// 6) it waits until `RH` (or its descendant) is relayed to the target chain;
		//
		// Phase#2: relaying teyrchain header
		//
		// 7) on-demand teyrchains relay sets `TeyrchainsSource::maximal_header_number` to the
		//    `PH'.number()`.
		// 8) teyrchains finality relay sees that the teyrchain head has been updated and relays
		//    `PH'` to    the target chain.

		// select headers to relay
		let relay_data = read_relay_data(
			&teyrchains_source,
			&teyrchains_target,
			required_teyrchain_header_number,
		)
		.await;
		match relay_data {
			Ok(relay_data) => {
				let prev_relay_state = relay_state;
				relay_state = select_headers_to_relay(&relay_data, relay_state);
				tracing::trace!(
					target: "bridge",
					%relay_task_name,
					?relay_state,
					?prev_relay_state,
					?relay_data,
					"Selected new relay state"
				);
			},
			Err(failed_client) => {
				relay_utils::relay_loop::reconnect_failed_client(
					failed_client,
					relay_utils::relay_loop::RECONNECT_DELAY,
					&mut teyrchains_source,
					&mut teyrchains_target,
				)
				.await;
				continue;
			},
		}

		// we have selected our new 'state' => let's notify our source clients about our new
		// requirements
		match relay_state {
			RelayState::Idle => (),
			RelayState::RelayingRelayHeader(required_relay_header) => {
				on_demand_source_relay_to_target_headers
					.require_more_headers(required_relay_header)
					.await;
			},
			RelayState::RelayingParaHeader(required_para_header) => {
				*required_para_header_ref.lock().await =
					AvailableHeader::Available(required_para_header);
			},
		}

		// start/restart relay
		if restart_relay {
			let stall_timeout = relay_bizinikiwi_client::transaction_stall_timeout(
				target_transactions_mortality,
				P::TargetChain::AVERAGE_BLOCK_INTERVAL,
				relay_utils::STALL_TIMEOUT,
			);

			tracing::info!(
				target: "bridge",
				relay_task_name,
				target_transactions_mortality,
				stall_timeout_as_mins=%stall_timeout.as_secs_f64() / 60.0f64,
				?stall_timeout,
				"Starting on-demand-teyrchains relay task"
			);

			teyrchains_relay_task.set(
				teyrchains_relay::teyrchains_loop::run(
					teyrchains_source.clone(),
					teyrchains_target.clone(),
					MetricsParams::disabled(),
					// we do not support free teyrchain headers relay in on-demand relays
					false,
					futures::future::pending(),
				)
				.fuse(),
			);

			restart_relay = false;
		}
	}
}

/// On-demand teyrchains relay task name.
fn on_demand_teyrchains_relay_name<SourceChain: Chain, TargetChain: Chain>() -> String {
	format!("{}-to-{}-on-demand-teyrchain", SourceChain::NAME, TargetChain::NAME)
}

/// On-demand relay state.
#[derive(Clone, Copy, Debug, PartialEq)]
enum RelayState<ParaHash, ParaNumber, RelayNumber> {
	/// On-demand relay is not doing anything.
	Idle,
	/// Relaying given relay header to relay given teyrchain header later.
	RelayingRelayHeader(RelayNumber),
	/// Relaying given teyrchain header.
	RelayingParaHeader(HeaderId<ParaHash, ParaNumber>),
}

/// Data gathered from source and target clients, used by on-demand relay.
#[derive(Debug)]
struct RelayData<ParaHash, ParaNumber, RelayNumber> {
	/// Teyrchain header number that is required at the target chain.
	pub required_para_header: ParaNumber,
	/// Teyrchain header number, known to the target chain.
	pub para_header_at_target: Option<ParaNumber>,
	/// Teyrchain header id, known to the source (relay) chain.
	pub para_header_at_source: Option<HeaderId<ParaHash, ParaNumber>>,
	/// Teyrchain header, that is available at the source relay chain at `relay_header_at_target`
	/// block.
	///
	/// May be `None` if there's no `relay_header_at_target` yet, or if the
	/// `relay_header_at_target` is too old and we think its state has been pruned.
	pub para_header_at_relay_header_at_target: Option<HeaderId<ParaHash, ParaNumber>>,
	/// Relay header number at the source chain.
	pub relay_header_at_source: RelayNumber,
	/// Relay header number at the target chain.
	pub relay_header_at_target: Option<RelayNumber>,
}

/// Read required data from source and target clients.
async fn read_relay_data<P: BizinikiwiTeyrchainsPipeline, SourceRelayClnt, TargetClnt>(
	source: &TeyrchainsSource<P, SourceRelayClnt>,
	target: &TeyrchainsTarget<P, SourceRelayClnt, TargetClnt>,
	required_header_number: BlockNumberOf<P::SourceTeyrchain>,
) -> Result<
	RelayData<
		HashOf<P::SourceTeyrchain>,
		BlockNumberOf<P::SourceTeyrchain>,
		BlockNumberOf<P::SourceRelayChain>,
	>,
	FailedClient,
>
where
	SourceRelayClnt: Client<P::SourceRelayChain>,
	TargetClnt: Client<P::TargetChain>,
	TeyrchainsTarget<P, SourceRelayClnt, TargetClnt>:
		TargetClient<TeyrchainsPipelineAdapter<P>> + RelayClient<Error = BizinikiwiError>,
{
	let map_target_err = |e| {
		tracing::error!(
			target: "bridge",
			error=?e,
			relay_name=%on_demand_teyrchains_relay_name::<P::SourceTeyrchain, P::TargetChain>(),
			target=%P::TargetChain::NAME,
			"Failed to read relay data from client"
		);
		FailedClient::Target
	};
	let map_source_err = |e| {
		tracing::error!(
			target: "bridge",
			error=?e,
			relay_name=%on_demand_teyrchains_relay_name::<P::SourceTeyrchain, P::TargetChain>(),
			source_relay_chain=%P::SourceRelayChain::NAME,
			"Failed to read relay data from client"
		);
		FailedClient::Source
	};

	let best_target_block_hash = target.best_block().await.map_err(map_target_err)?.1;
	let para_header_at_target = best_finalized_peer_header_at_self::<
		P::TargetChain,
		P::SourceTeyrchain,
	>(target.target_client(), best_target_block_hash)
	.await;
	// if there are no teyrchain heads at the target (`NoTeyrchainHeadAtTarget`), we'll need to
	// submit at least one. Otherwise the pezpallet will be treated as uninitialized and messages
	// sync will stall.
	let para_header_at_target = match para_header_at_target {
		Ok(Some(para_header_at_target)) => Some(para_header_at_target.0),
		Ok(None) => None,
		Err(e) => return Err(map_target_err(e)),
	};

	let best_finalized_relay_header =
		source.client().best_finalized_header().await.map_err(map_source_err)?;
	let best_finalized_relay_block_id = best_finalized_relay_header.id();
	let para_header_at_source = source
		.on_chain_para_head_id(best_finalized_relay_block_id)
		.await
		.map_err(map_source_err)?;

	let relay_header_at_source = best_finalized_relay_block_id.0;
	let relay_header_at_target = best_finalized_peer_header_at_self::<
		P::TargetChain,
		P::SourceRelayChain,
	>(target.target_client(), best_target_block_hash)
	.await
	.map_err(map_target_err)?;

	// if relay header at target is too old then its state may already be discarded at the source
	// => just use `None` in this case
	//
	// the same is for case when there's no relay header at target at all
	let available_relay_header_at_target =
		relay_header_at_target.filter(|relay_header_at_target| {
			!is_ancient_block(relay_header_at_target.number(), relay_header_at_source)
		});
	let para_header_at_relay_header_at_target =
		if let Some(available_relay_header_at_target) = available_relay_header_at_target {
			source
				.on_chain_para_head_id(available_relay_header_at_target)
				.await
				.map_err(map_source_err)?
		} else {
			None
		};

	Ok(RelayData {
		required_para_header: required_header_number,
		para_header_at_target,
		para_header_at_source,
		relay_header_at_source,
		relay_header_at_target: relay_header_at_target
			.map(|relay_header_at_target| relay_header_at_target.0),
		para_header_at_relay_header_at_target,
	})
}

/// Select relay and teyrchain headers that need to be relayed.
fn select_headers_to_relay<ParaHash, ParaNumber, RelayNumber>(
	data: &RelayData<ParaHash, ParaNumber, RelayNumber>,
	state: RelayState<ParaHash, ParaNumber, RelayNumber>,
) -> RelayState<ParaHash, ParaNumber, RelayNumber>
where
	ParaHash: Clone,
	ParaNumber: Copy + PartialOrd + Zero,
	RelayNumber: Copy + Debug + Ord,
{
	// we can't do anything until **relay chain** bridge GRANDPA pezpallet is not initialized at the
	// target chain
	let relay_header_at_target = match data.relay_header_at_target {
		Some(relay_header_at_target) => relay_header_at_target,
		None => return RelayState::Idle,
	};

	// Process the `RelayingRelayHeader` state.
	if let &RelayState::RelayingRelayHeader(relay_header_number) = &state {
		if relay_header_at_target < relay_header_number {
			// The required relay header hasn't yet been relayed. Ask / wait for it.
			return state;
		}

		// We may switch to `RelayingParaHeader` if teyrchain head is available.
		if let Some(para_header_at_relay_header_at_target) =
			data.para_header_at_relay_header_at_target.as_ref()
		{
			return RelayState::RelayingParaHeader(para_header_at_relay_header_at_target.clone());
		}

		// else use the regular process - e.g. we may require to deliver new relay header first
	}

	// Process the `RelayingParaHeader` state.
	if let RelayState::RelayingParaHeader(para_header_id) = &state {
		let para_header_at_target_or_zero = data.para_header_at_target.unwrap_or_else(Zero::zero);
		if para_header_at_target_or_zero < para_header_id.0 {
			// The required teyrchain header hasn't yet been relayed. Ask / wait for it.
			return state;
		}
	}

	// if we haven't read para head from the source, we can't yet do anything
	let para_header_at_source = match data.para_header_at_source {
		Some(ref para_header_at_source) => para_header_at_source.clone(),
		None => return RelayState::Idle,
	};

	// if we have teyrchain head at the source, but no teyrchain heads at the target, we'll need
	// to deliver at least one teyrchain head
	let (required_para_header, para_header_at_target) = match data.para_header_at_target {
		Some(para_header_at_target) => (data.required_para_header, para_header_at_target),
		None => (para_header_at_source.0, Zero::zero()),
	};

	// if we have already satisfied our "customer", do nothing
	if required_para_header <= para_header_at_target {
		return RelayState::Idle;
	}

	// if required header is not available even at the source chain, let's wait
	if required_para_header > para_header_at_source.0 {
		return RelayState::Idle;
	}

	// we will always try to sync latest teyrchain/relay header, even if we've been asked for some
	// its ancestor

	// we need relay chain header first
	if relay_header_at_target < data.relay_header_at_source {
		return RelayState::RelayingRelayHeader(data.relay_header_at_source);
	}

	// if all relay headers synced, we may start directly with teyrchain header
	RelayState::RelayingParaHeader(para_header_at_source)
}

/// Environment for the `select_headers_to_prove` call.
#[async_trait]
trait SelectHeadersToProveEnvironment<RBN, RBH, PBN, PBH> {
	/// Returns associated teyrchain id.
	fn teyrchain_id(&self) -> ParaId;
	/// Returns best finalized relay block.
	async fn best_finalized_relay_block_at_source(
		&self,
	) -> Result<HeaderId<RBH, RBN>, BizinikiwiError>;
	/// Returns best finalized relay block that is known at `P::TargetChain`.
	async fn best_finalized_relay_block_at_target(
		&self,
	) -> Result<HeaderId<RBH, RBN>, BizinikiwiError>;
	/// Returns best finalized teyrchain block at given source relay chain block.
	async fn best_finalized_para_block_at_source(
		&self,
		at_relay_block: HeaderId<RBH, RBN>,
	) -> Result<Option<HeaderId<PBH, PBN>>, BizinikiwiError>;
}

#[async_trait]
impl<'a, P: BizinikiwiTeyrchainsPipeline, SourceRelayClnt, TargetClnt>
	SelectHeadersToProveEnvironment<
		BlockNumberOf<P::SourceRelayChain>,
		HashOf<P::SourceRelayChain>,
		BlockNumberOf<P::SourceTeyrchain>,
		HashOf<P::SourceTeyrchain>,
	>
	for (
		&'a OnDemandTeyrchainsRelay<P, SourceRelayClnt, TargetClnt>,
		&'a TeyrchainsSource<P, SourceRelayClnt>,
	)
where
	SourceRelayClnt: Client<P::SourceRelayChain>,
	TargetClnt: Client<P::TargetChain>,
{
	fn teyrchain_id(&self) -> ParaId {
		ParaId(P::SourceTeyrchain::TEYRCHAIN_ID)
	}

	async fn best_finalized_relay_block_at_source(
		&self,
	) -> Result<HeaderIdOf<P::SourceRelayChain>, BizinikiwiError> {
		Ok(self.0.source_relay_client.best_finalized_header().await?.id())
	}

	async fn best_finalized_relay_block_at_target(
		&self,
	) -> Result<HeaderIdOf<P::SourceRelayChain>, BizinikiwiError> {
		Ok(crate::messages::source::read_client_state::<P::TargetChain, P::SourceRelayChain>(
			&self.0.target_client,
		)
		.await?
		.best_finalized_peer_at_best_self
		.ok_or(BizinikiwiError::BridgePalletIsNotInitialized)?)
	}

	async fn best_finalized_para_block_at_source(
		&self,
		at_relay_block: HeaderIdOf<P::SourceRelayChain>,
	) -> Result<Option<HeaderIdOf<P::SourceTeyrchain>>, BizinikiwiError> {
		self.1.on_chain_para_head_id(at_relay_block).await
	}
}

/// Given request to prove `required_teyrchain_header`, select actual headers that need to be
/// proved.
async fn select_headers_to_prove<RBN, RBH, PBN, PBH>(
	env: impl SelectHeadersToProveEnvironment<RBN, RBH, PBN, PBH>,
	required_teyrchain_header: PBN,
) -> Result<(bool, HeaderId<RBH, RBN>, HeaderId<PBH, PBN>), BizinikiwiError>
where
	RBH: Copy,
	RBN: BlockNumberBase,
	PBH: Copy,
	PBN: BlockNumberBase,
{
	// teyrchains proof also requires relay header proof. Let's first select relay block
	// number that we'll be dealing with
	let best_finalized_relay_block_at_source = env.best_finalized_relay_block_at_source().await?;
	let best_finalized_relay_block_at_target = env.best_finalized_relay_block_at_target().await?;

	// if we can't prove `required_header` even using `best_finalized_relay_block_at_source`, we
	// can't do anything here
	// (this shall not actually happen, given current code, because we only require finalized
	// headers)
	let best_possible_teyrchain_block = env
		.best_finalized_para_block_at_source(best_finalized_relay_block_at_source)
		.await?
		.filter(|best_possible_teyrchain_block| {
			best_possible_teyrchain_block.number() >= required_teyrchain_header
		})
		.ok_or(BizinikiwiError::MissingRequiredTeyrchainHead(
			env.teyrchain_id(),
			required_teyrchain_header.unique_saturated_into(),
		))?;

	// we don't require source node to be archive, so we can't craft storage proofs using
	// ancient headers. So if the `best_finalized_relay_block_at_target` is too ancient, we
	// can't craft storage proofs using it
	let may_use_state_at_best_finalized_relay_block_at_target = !is_ancient_block(
		best_finalized_relay_block_at_target.number(),
		best_finalized_relay_block_at_source.number(),
	);

	// now let's check if `required_header` may be proved using
	// `best_finalized_relay_block_at_target`
	let selection = if may_use_state_at_best_finalized_relay_block_at_target {
		env.best_finalized_para_block_at_source(best_finalized_relay_block_at_target)
			.await?
			.filter(|best_finalized_para_block_at_target| {
				best_finalized_para_block_at_target.number() >= required_teyrchain_header
			})
			.map(|best_finalized_para_block_at_target| {
				(false, best_finalized_relay_block_at_target, best_finalized_para_block_at_target)
			})
	} else {
		None
	};

	Ok(selection.unwrap_or((
		true,
		best_finalized_relay_block_at_source,
		best_possible_teyrchain_block,
	)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn relay_waits_for_relay_header_to_be_delivered() {
		assert_eq!(
			select_headers_to_relay(
				&RelayData {
					required_para_header: 90,
					para_header_at_target: Some(50),
					para_header_at_source: Some(HeaderId(110, 110)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(700),
					para_header_at_relay_header_at_target: Some(HeaderId(100, 100)),
				},
				RelayState::RelayingRelayHeader(750),
			),
			RelayState::RelayingRelayHeader(750),
		);
	}

	#[test]
	fn relay_starts_relaying_requested_para_header_after_relay_header_is_delivered() {
		assert_eq!(
			select_headers_to_relay(
				&RelayData {
					required_para_header: 90,
					para_header_at_target: Some(50),
					para_header_at_source: Some(HeaderId(110, 110)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(750),
					para_header_at_relay_header_at_target: Some(HeaderId(100, 100)),
				},
				RelayState::RelayingRelayHeader(750),
			),
			RelayState::RelayingParaHeader(HeaderId(100, 100)),
		);
	}

	#[test]
	fn relay_selects_better_para_header_after_better_relay_header_is_delivered() {
		assert_eq!(
			select_headers_to_relay(
				&RelayData {
					required_para_header: 90,
					para_header_at_target: Some(50),
					para_header_at_source: Some(HeaderId(110, 110)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(780),
					para_header_at_relay_header_at_target: Some(HeaderId(105, 105)),
				},
				RelayState::RelayingRelayHeader(750),
			),
			RelayState::RelayingParaHeader(HeaderId(105, 105)),
		);
	}
	#[test]
	fn relay_waits_for_para_header_to_be_delivered() {
		assert_eq!(
			select_headers_to_relay(
				&RelayData {
					required_para_header: 90,
					para_header_at_target: Some(50),
					para_header_at_source: Some(HeaderId(110, 110)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(780),
					para_header_at_relay_header_at_target: Some(HeaderId(105, 105)),
				},
				RelayState::RelayingParaHeader(HeaderId(105, 105)),
			),
			RelayState::RelayingParaHeader(HeaderId(105, 105)),
		);
	}

	#[test]
	fn relay_stays_idle_if_required_para_header_is_already_delivered() {
		assert_eq!(
			select_headers_to_relay(
				&RelayData {
					required_para_header: 90,
					para_header_at_target: Some(105),
					para_header_at_source: Some(HeaderId(110, 110)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(780),
					para_header_at_relay_header_at_target: Some(HeaderId(105, 105)),
				},
				RelayState::Idle,
			),
			RelayState::Idle,
		);
	}

	#[test]
	fn relay_waits_for_required_para_header_to_appear_at_source_1() {
		assert_eq!(
			select_headers_to_relay(
				&RelayData {
					required_para_header: 120,
					para_header_at_target: Some(105),
					para_header_at_source: None,
					relay_header_at_source: 800,
					relay_header_at_target: Some(780),
					para_header_at_relay_header_at_target: Some(HeaderId(105, 105)),
				},
				RelayState::Idle,
			),
			RelayState::Idle,
		);
	}

	#[test]
	fn relay_waits_for_required_para_header_to_appear_at_source_2() {
		assert_eq!(
			select_headers_to_relay(
				&RelayData {
					required_para_header: 120,
					para_header_at_target: Some(105),
					para_header_at_source: Some(HeaderId(110, 110)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(780),
					para_header_at_relay_header_at_target: Some(HeaderId(105, 105)),
				},
				RelayState::Idle,
			),
			RelayState::Idle,
		);
	}

	#[test]
	fn relay_starts_relaying_relay_header_when_new_para_header_is_requested() {
		assert_eq!(
			select_headers_to_relay(
				&RelayData {
					required_para_header: 120,
					para_header_at_target: Some(105),
					para_header_at_source: Some(HeaderId(125, 125)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(780),
					para_header_at_relay_header_at_target: Some(HeaderId(105, 105)),
				},
				RelayState::Idle,
			),
			RelayState::RelayingRelayHeader(800),
		);
	}

	#[test]
	fn relay_starts_relaying_para_header_when_new_para_header_is_requested() {
		assert_eq!(
			select_headers_to_relay(
				&RelayData {
					required_para_header: 120,
					para_header_at_target: Some(105),
					para_header_at_source: Some(HeaderId(125, 125)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(800),
					para_header_at_relay_header_at_target: Some(HeaderId(125, 125)),
				},
				RelayState::Idle,
			),
			RelayState::RelayingParaHeader(HeaderId(125, 125)),
		);
	}

	#[test]
	fn relay_goes_idle_when_teyrchain_is_deregistered() {
		assert_eq!(
			select_headers_to_relay::<i32, _, _>(
				&RelayData {
					required_para_header: 120,
					para_header_at_target: Some(105),
					para_header_at_source: None,
					relay_header_at_source: 800,
					relay_header_at_target: Some(800),
					para_header_at_relay_header_at_target: None,
				},
				RelayState::RelayingRelayHeader(800),
			),
			RelayState::Idle,
		);
	}

	#[test]
	fn relay_starts_relaying_first_teyrchain_header() {
		assert_eq!(
			select_headers_to_relay::<i32, _, _>(
				&RelayData {
					required_para_header: 0,
					para_header_at_target: None,
					para_header_at_source: Some(HeaderId(125, 125)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(800),
					para_header_at_relay_header_at_target: Some(HeaderId(125, 125)),
				},
				RelayState::Idle,
			),
			RelayState::RelayingParaHeader(HeaderId(125, 125)),
		);
	}

	#[test]
	fn relay_starts_relaying_relay_header_to_relay_first_teyrchain_header() {
		assert_eq!(
			select_headers_to_relay::<i32, _, _>(
				&RelayData {
					required_para_header: 0,
					para_header_at_target: None,
					para_header_at_source: Some(HeaderId(125, 125)),
					relay_header_at_source: 800,
					relay_header_at_target: Some(700),
					para_header_at_relay_header_at_target: Some(HeaderId(125, 125)),
				},
				RelayState::Idle,
			),
			RelayState::RelayingRelayHeader(800),
		);
	}

	// tuple is:
	//
	// - best_finalized_relay_block_at_source
	// - best_finalized_relay_block_at_target
	// - best_finalized_para_block_at_source at best_finalized_relay_block_at_source
	// - best_finalized_para_block_at_source at best_finalized_relay_block_at_target
	#[async_trait]
	impl SelectHeadersToProveEnvironment<u32, u32, u32, u32> for (u32, u32, u32, u32) {
		fn teyrchain_id(&self) -> ParaId {
			ParaId(0)
		}

		async fn best_finalized_relay_block_at_source(
			&self,
		) -> Result<HeaderId<u32, u32>, BizinikiwiError> {
			Ok(HeaderId(self.0, self.0))
		}

		async fn best_finalized_relay_block_at_target(
			&self,
		) -> Result<HeaderId<u32, u32>, BizinikiwiError> {
			Ok(HeaderId(self.1, self.1))
		}

		async fn best_finalized_para_block_at_source(
			&self,
			at_relay_block: HeaderId<u32, u32>,
		) -> Result<Option<HeaderId<u32, u32>>, BizinikiwiError> {
			if at_relay_block.0 == self.0 {
				Ok(Some(HeaderId(self.2, self.2)))
			} else if at_relay_block.0 == self.1 {
				Ok(Some(HeaderId(self.3, self.3)))
			} else {
				Ok(None)
			}
		}
	}

	#[async_std::test]
	async fn select_headers_to_prove_returns_err_if_required_para_block_is_missing_at_source() {
		assert!(matches!(
			select_headers_to_prove((20_u32, 10_u32, 200_u32, 100_u32), 300_u32,).await,
			Err(BizinikiwiError::MissingRequiredTeyrchainHead(ParaId(0), 300_u64)),
		));
	}

	#[async_std::test]
	async fn select_headers_to_prove_fails_to_use_existing_ancient_relay_block() {
		assert_eq!(
			select_headers_to_prove((220_u32, 10_u32, 200_u32, 100_u32), 100_u32,)
				.await
				.map_err(drop),
			Ok((true, HeaderId(220, 220), HeaderId(200, 200))),
		);
	}

	#[async_std::test]
	async fn select_headers_to_prove_is_able_to_use_existing_recent_relay_block() {
		assert_eq!(
			select_headers_to_prove((40_u32, 10_u32, 200_u32, 100_u32), 100_u32,)
				.await
				.map_err(drop),
			Ok((false, HeaderId(10, 10), HeaderId(100, 100))),
		);
	}

	#[async_std::test]
	async fn select_headers_to_prove_uses_new_relay_block() {
		assert_eq!(
			select_headers_to_prove((20_u32, 10_u32, 200_u32, 100_u32), 200_u32,)
				.await
				.map_err(drop),
			Ok((true, HeaderId(20, 20), HeaderId(200, 200))),
		);
	}
}
