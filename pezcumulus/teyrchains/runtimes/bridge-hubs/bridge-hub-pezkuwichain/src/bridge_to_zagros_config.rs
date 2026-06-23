// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
// This file is part of Pezcumulus.
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

//! Bridge definitions used on BridgeHubPezkuwichain for bridging to BridgeHubZagros.

use crate::{
	bridge_common_config::{
		BridgeTeyrchainZagrosInstance, DeliveryRewardInBalance,
		RelayersForLegacyLaneIdsMessagesInstance,
	},
	weights,
	xcm_config::UniversalLocation,
	AccountId, Balance, Balances, BridgeZagrosMessages, PezkuwiXcm, Runtime, RuntimeEvent,
	RuntimeHoldReason, XcmOverBridgeHubZagros, XcmRouter, XcmpQueue,
};
use pezbp_messages::{
	source_chain::FromBridgedChainMessagesDeliveryProof,
	target_chain::FromBridgedChainMessagesProof, LegacyLaneId,
};
use pezbridge_hub_common::xcm_version::XcmVersionOfDestAndRemoteBridge;
use pezpallet_xcm_bridge_hub::{BridgeId, XcmAsPlainPayload};

use pezframe_support::{parameter_types, traits::PalletInfoAccess};
use pezframe_system::{EnsureNever, EnsureRoot};
use pezkuwi_teyrchain_primitives::primitives::Sibling;
use pezpallet_bridge_messages::LaneIdOf;
use pezpallet_bridge_relayers::extension::{
	BridgeRelayersTransactionExtension, WithMessagesExtensionConfig,
};
use testnet_teyrchains_constants::pezkuwichain::currency::UNITS as HEZ;
use teyrchains_common::xcm_config::{AllSiblingSystemTeyrchains, RelayOrOtherSystemTeyrchains};
use xcm::{
	latest::{prelude::*, ZAGROS_GENESIS_HASH},
	prelude::{InteriorLocation, NetworkId},
};
use xcm_builder::{BridgeBlobDispatcher, ParentIsPreset, SiblingTeyrchainConvertsVia};

parameter_types! {
	pub BridgePezkuwichainToZagrosMessagesPalletInstance: InteriorLocation = [PalletInstance(<BridgeZagrosMessages as PalletInfoAccess>::index() as u8)].into();
	pub ZagrosGlobalConsensusNetwork: NetworkId = NetworkId::ByGenesis(ZAGROS_GENESIS_HASH);
	pub ZagrosGlobalConsensusNetworkLocation: Location = Location::new(
		2,
		[GlobalConsensus(ZagrosGlobalConsensusNetwork::get())]
	);
	// see the `FEE_BOOST_PER_RELAY_HEADER` constant get the meaning of this value
	pub PriorityBoostPerRelayHeader: u64 = 32_007_814_407_814;
	// see the `FEE_BOOST_PER_TEYRCHAIN_HEADER` constant get the meaning of this value
	pub PriorityBoostPerTeyrchainHeader: u64 = 1_396_340_903_540_903;
	// see the `FEE_BOOST_PER_MESSAGE` constant to get the meaning of this value
	pub PriorityBoostPerMessage: u64 = 364_088_888_888_888;

	pub BridgeHubZagrosLocation: Location = Location::new(
		2,
		[
			GlobalConsensus(ZagrosGlobalConsensusNetwork::get()),
			Teyrchain(<pezbp_bridge_hub_zagros::BridgeHubZagros as pezbp_runtime::Teyrchain>::TEYRCHAIN_ID)
		]
	);

	pub storage BridgeDeposit: Balance = 5 * HEZ;
}

/// Proof of messages, coming from Zagros.
pub type FromZagrosBridgeHubMessagesProof<MI> =
	FromBridgedChainMessagesProof<pezbp_bridge_hub_zagros::Hash, LaneIdOf<Runtime, MI>>;
/// Messages delivery proof for Pezkuwichain Bridge Hub -> Zagros Bridge Hub messages.
pub type ToZagrosBridgeHubMessagesDeliveryProof<MI> =
	FromBridgedChainMessagesDeliveryProof<pezbp_bridge_hub_zagros::Hash, LaneIdOf<Runtime, MI>>;

/// Dispatches received XCM messages from other bridge
type FromZagrosMessageBlobDispatcher = BridgeBlobDispatcher<
	XcmRouter,
	UniversalLocation,
	BridgePezkuwichainToZagrosMessagesPalletInstance,
>;

/// Transaction extension that refunds relayers that are delivering messages from the Zagros
/// teyrchain.
pub type OnBridgeHubPezkuwichainRefundBridgeHubZagrosMessages = BridgeRelayersTransactionExtension<
	Runtime,
	WithMessagesExtensionConfig<
		StrOnBridgeHubPezkuwichainRefundBridgeHubZagrosMessages,
		Runtime,
		WithBridgeHubZagrosMessagesInstance,
		RelayersForLegacyLaneIdsMessagesInstance,
		PriorityBoostPerMessage,
	>,
>;
pezbp_runtime::generate_static_str_provider!(OnBridgeHubPezkuwichainRefundBridgeHubZagrosMessages);

/// Add XCM messages support for BridgeHubPezkuwichain to support Pezkuwichain->Zagros XCM messages
pub type WithBridgeHubZagrosMessagesInstance = pezpallet_bridge_messages::Instance3;
impl pezpallet_bridge_messages::Config<WithBridgeHubZagrosMessagesInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo =
		weights::pezpallet_bridge_messages_pezkuwichain_to_zagros::WeightInfo<Runtime>;

	type ThisChain = pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichain;
	type BridgedChain = pezbp_bridge_hub_zagros::BridgeHubZagros;
	type BridgedHeaderChain = pezpallet_bridge_teyrchains::TeyrchainHeaders<
		Runtime,
		BridgeTeyrchainZagrosInstance,
		pezbp_bridge_hub_zagros::BridgeHubZagros,
	>;

	type OutboundPayload = XcmAsPlainPayload;
	type InboundPayload = XcmAsPlainPayload;
	type LaneId = LegacyLaneId;

	type DeliveryPayments = ();
	type DeliveryConfirmationPayments =
		pezpallet_bridge_relayers::DeliveryConfirmationPaymentsAdapter<
			Runtime,
			WithBridgeHubZagrosMessagesInstance,
			RelayersForLegacyLaneIdsMessagesInstance,
			DeliveryRewardInBalance,
		>;

	type MessageDispatch = XcmOverBridgeHubZagros;
	type OnMessagesDelivered = XcmOverBridgeHubZagros;
}

/// Add support for the export and dispatch of XCM programs withing
/// `WithBridgeHubZagrosMessagesInstance`.
pub type XcmOverBridgeHubZagrosInstance = pezpallet_xcm_bridge_hub::Instance1;
impl pezpallet_xcm_bridge_hub::Config<XcmOverBridgeHubZagrosInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type UniversalLocation = UniversalLocation;
	type BridgedNetwork = ZagrosGlobalConsensusNetworkLocation;
	type BridgeMessagesPalletInstance = WithBridgeHubZagrosMessagesInstance;

	type MessageExportPrice = ();
	type DestinationVersion = XcmVersionOfDestAndRemoteBridge<PezkuwiXcm, BridgeHubZagrosLocation>;

	type ForceOrigin = EnsureRoot<AccountId>;
	// We don't want to allow creating bridges for this instance with `LegacyLaneId`.
	type OpenBridgeOrigin = EnsureNever<Location>;
	// Converter aligned with `OpenBridgeOrigin`.
	type BridgeOriginAccountIdConverter =
		(ParentIsPreset<AccountId>, SiblingTeyrchainConvertsVia<Sibling, AccountId>);

	type BridgeDeposit = BridgeDeposit;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	// Do not require deposit from system teyrchains or relay chain
	type AllowWithoutBridgeDeposit =
		RelayOrOtherSystemTeyrchains<AllSiblingSystemTeyrchains, Runtime>;

	type LocalXcmChannelManager = CongestionManager;
	type BlobDispatcher = FromZagrosMessageBlobDispatcher;
}

/// Implementation of `pezbp_xcm_bridge_hub::LocalXcmChannelManager` for congestion management.
pub struct CongestionManager;
impl pezpallet_xcm_bridge_hub::LocalXcmChannelManager for CongestionManager {
	type Error = SendError;

	fn is_congested(with: &Location) -> bool {
		// This is used to check the inbound bridge queue/messages to determine if they can be
		// dispatched and sent to the sibling teyrchain. Therefore, checking outbound `XcmpQueue`
		// is sufficient here.
		use pezbp_xcm_bridge_hub_router::XcmChannelStatusProvider;
		pezcumulus_pezpallet_xcmp_queue::bridging::OutXcmpChannelStatusProvider::<Runtime>::is_congested(
			with,
		)
	}

	fn suspend_bridge(local_origin: &Location, bridge: BridgeId) -> Result<(), Self::Error> {
		// This bridge is intended for AH<>AH communication with a hard-coded/static lane,
		// so `local_origin` is expected to represent only the local AH.
		send_xcm::<XcmpQueue>(
			local_origin.clone(),
			pezbp_asset_hub_pezkuwichain::build_congestion_message(bridge.inner(), true).into(),
		)
		.map(|_| ())
	}

	fn resume_bridge(local_origin: &Location, bridge: BridgeId) -> Result<(), Self::Error> {
		// This bridge is intended for AH<>AH communication with a hard-coded/static lane,
		// so `local_origin` is expected to represent only the local AH.
		send_xcm::<XcmpQueue>(
			local_origin.clone(),
			pezbp_asset_hub_pezkuwichain::build_congestion_message(bridge.inner(), false).into(),
		)
		.map(|_| ())
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub(crate) fn open_bridge_for_benchmarks<R, XBHI, C>(
	with: pezpallet_xcm_bridge_hub::LaneIdOf<R, XBHI>,
	sibling_para_id: u32,
) -> InteriorLocation
where
	R: pezpallet_xcm_bridge_hub::Config<XBHI>,
	XBHI: 'static,
	C: xcm_executor::traits::ConvertLocation<
		pezbp_runtime::AccountIdOf<pezpallet_xcm_bridge_hub::ThisChainOf<R, XBHI>>,
	>,
{
	use pezpallet_xcm_bridge_hub::{Bridge, BridgeId, BridgeState};
	use pezsp_runtime::traits::Zero;
	use xcm::{latest::PEZKUWICHAIN_GENESIS_HASH, VersionedInteriorLocation};

	// insert bridge metadata
	let lane_id = with;
	let sibling_teyrchain = Location::new(1, [Teyrchain(sibling_para_id)]);
	let universal_source =
		[GlobalConsensus(ByGenesis(PEZKUWICHAIN_GENESIS_HASH)), Teyrchain(sibling_para_id)].into();
	let universal_destination =
		[GlobalConsensus(ByGenesis(ZAGROS_GENESIS_HASH)), Teyrchain(2075)].into();
	let bridge_id = BridgeId::new(&universal_source, &universal_destination);

	// insert only bridge metadata, because the benchmarks create lanes
	pezpallet_xcm_bridge_hub::Bridges::<R, XBHI>::insert(
		bridge_id,
		Bridge {
			bridge_origin_relative_location: alloc::boxed::Box::new(
				sibling_teyrchain.clone().into(),
			),
			bridge_origin_universal_location: alloc::boxed::Box::new(
				VersionedInteriorLocation::from(universal_source.clone()),
			),
			bridge_destination_universal_location: alloc::boxed::Box::new(
				VersionedInteriorLocation::from(universal_destination),
			),
			state: BridgeState::Opened,
			bridge_owner_account: C::convert_location(&sibling_teyrchain).expect("valid AccountId"),
			deposit: Zero::zero(),
			lane_id,
		},
	);
	pezpallet_xcm_bridge_hub::LaneToBridge::<R, XBHI>::insert(lane_id, bridge_id);

	universal_source
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::bridge_common_config::BridgeGrandpaZagrosInstance;
	use pezbridge_runtime_common::{
		assert_complete_bridge_types,
		integrity::{
			assert_complete_with_teyrchain_bridge_constants, check_message_lane_weights,
			AssertChainConstants, AssertCompleteBridgeConstants,
		},
	};

	/// Every additional message in the message delivery transaction boosts its priority.
	/// So the priority of transaction with `N+1` messages is larger than priority of
	/// transaction with `N` messages by the `PriorityBoostPerMessage`.
	///
	/// Economically, it is an equivalent of adding tip to the transaction with `N` messages.
	/// The `FEE_BOOST_PER_MESSAGE` constant is the value of this tip.
	///
	/// We want this tip to be large enough (delivery transactions with more messages = less
	/// operational costs and a faster bridge), so this value should be significant.
	const FEE_BOOST_PER_MESSAGE: Balance = 2 * HEZ;

	// see `FEE_BOOST_PER_MESSAGE` comment
	const FEE_BOOST_PER_RELAY_HEADER: Balance = 2 * HEZ;
	// see `FEE_BOOST_PER_MESSAGE` comment
	const FEE_BOOST_PER_TEYRCHAIN_HEADER: Balance = 2 * HEZ;

	#[test]
	fn ensure_bridge_hub_pezkuwichain_message_lane_weights_are_correct() {
		check_message_lane_weights::<
			pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichain,
			Runtime,
			WithBridgeHubZagrosMessagesInstance,
		>(
			pezbp_bridge_hub_zagros::EXTRA_STORAGE_PROOF_SIZE,
			pezbp_bridge_hub_pezkuwichain::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
			pezbp_bridge_hub_pezkuwichain::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
			true,
		);
	}

	#[test]
	fn ensure_bridge_integrity() {
		assert_complete_bridge_types!(
			runtime: Runtime,
			with_bridged_chain_messages_instance: WithBridgeHubZagrosMessagesInstance,
			this_chain: pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichain,
			bridged_chain: pezbp_bridge_hub_zagros::BridgeHubZagros,
			expected_payload_type: XcmAsPlainPayload,
		);

		assert_complete_with_teyrchain_bridge_constants::<
			Runtime,
			BridgeGrandpaZagrosInstance,
			WithBridgeHubZagrosMessagesInstance,
		>(AssertCompleteBridgeConstants {
			this_chain_constants: AssertChainConstants {
				block_length: pezbp_bridge_hub_pezkuwichain::BlockLength::get(),
				block_weights: pezbp_bridge_hub_pezkuwichain::BlockWeightsForAsyncBacking::get(),
			},
		});

		pezpallet_bridge_relayers::extension::per_relay_header::ensure_priority_boost_is_sane::<
			Runtime,
			BridgeGrandpaZagrosInstance,
			PriorityBoostPerRelayHeader,
		>(FEE_BOOST_PER_RELAY_HEADER);

		pezpallet_bridge_relayers::extension::per_teyrchain_header::ensure_priority_boost_is_sane::<
			Runtime,
			WithBridgeHubZagrosMessagesInstance,
			pezbp_bridge_hub_zagros::BridgeHubZagros,
			PriorityBoostPerTeyrchainHeader,
		>(FEE_BOOST_PER_TEYRCHAIN_HEADER);

		pezpallet_bridge_relayers::extension::per_message::ensure_priority_boost_is_sane::<
			Runtime,
			WithBridgeHubZagrosMessagesInstance,
			PriorityBoostPerMessage,
		>(FEE_BOOST_PER_MESSAGE);

		let expected: InteriorLocation = [PalletInstance(
			pezbp_bridge_hub_pezkuwichain::WITH_BRIDGE_PEZKUWICHAIN_TO_ZAGROS_MESSAGES_PALLET_INDEX,
		)]
		.into();

		assert_eq!(BridgePezkuwichainToZagrosMessagesPalletInstance::get(), expected,);
	}
}

/// Contains the migration for the AssetHubPezkuwichain<>AssetHubZagros bridge.
pub mod migration {
	use super::*;
	use pezframe_support::traits::ConstBool;

	parameter_types! {
		pub AssetHubPezkuwichainToAssetHubZagrosMessagesLane: LegacyLaneId = LegacyLaneId([0, 0, 0, 2]);
		pub AssetHubPezkuwichainLocation: Location = Location::new(1, [Teyrchain(pezbp_asset_hub_pezkuwichain::ASSET_HUB_PEZKUWICHAIN_TEYRCHAIN_ID)]);
		pub AssetHubZagrosUniversalLocation: InteriorLocation = [GlobalConsensus(ZagrosGlobalConsensusNetwork::get()), Teyrchain(pezbp_asset_hub_zagros::ASSET_HUB_ZAGROS_TEYRCHAIN_ID)].into();
	}

	/// Ensure that the existing lanes for the AHR<>AHW bridge are correctly configured.
	pub type StaticToDynamicLanes = pezpallet_xcm_bridge_hub::migration::OpenBridgeForLane<
		Runtime,
		XcmOverBridgeHubZagrosInstance,
		AssetHubPezkuwichainToAssetHubZagrosMessagesLane,
		// the lanes are already created for AHR<>AHW, but we need to link them to the bridge
		// structs
		ConstBool<false>,
		AssetHubPezkuwichainLocation,
		AssetHubZagrosUniversalLocation,
	>;

	mod v1_wrong {
		use codec::{Decode, Encode};
		use pezbp_messages::{LaneState, MessageNonce, UnrewardedRelayer};
		use pezbp_runtime::AccountIdOf;
		use pezpallet_bridge_messages::BridgedChainOf;
		use pezsp_std::collections::vec_deque::VecDeque;

		#[derive(Encode, Decode, Clone, PartialEq, Eq)]
		pub(crate) struct StoredInboundLaneData<T: pezpallet_bridge_messages::Config<I>, I: 'static>(
			pub(crate) InboundLaneData<AccountIdOf<BridgedChainOf<T, I>>>,
		);
		#[derive(Encode, Decode, Clone, PartialEq, Eq)]
		pub(crate) struct InboundLaneData<RelayerId> {
			pub state: LaneState,
			pub(crate) relayers: VecDeque<UnrewardedRelayer<RelayerId>>,
			pub(crate) last_confirmed_nonce: MessageNonce,
		}
		#[derive(Encode, Decode, Clone, PartialEq, Eq)]
		pub(crate) struct OutboundLaneData {
			pub state: LaneState,
			pub(crate) oldest_unpruned_nonce: MessageNonce,
			pub(crate) latest_received_nonce: MessageNonce,
			pub(crate) latest_generated_nonce: MessageNonce,
		}
	}

	mod v1 {
		pub use pezbp_messages::{InboundLaneData, LaneState, OutboundLaneData};
		pub use pezpallet_bridge_messages::{InboundLanes, OutboundLanes, StoredInboundLaneData};
	}

	/// Fix for v1 migration - corrects data for OutboundLaneData/InboundLaneData (it is needed only
	/// for Pezkuwichain/Zagros).
	pub struct FixMessagesV1Migration<T, I>(pezsp_std::marker::PhantomData<(T, I)>);

	impl<T: pezpallet_bridge_messages::Config<I>, I: 'static>
		pezframe_support::traits::OnRuntimeUpgrade for FixMessagesV1Migration<T, I>
	{
		fn on_runtime_upgrade() -> Weight {
			use pezsp_core::Get;
			let mut weight = T::DbWeight::get().reads(1);

			// `InboundLanes` - add state to the old structs
			let translate_inbound =
				|pre: v1_wrong::StoredInboundLaneData<T, I>| -> Option<v1::StoredInboundLaneData<T, I>> {
					weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
					Some(v1::StoredInboundLaneData(v1::InboundLaneData {
						state: v1::LaneState::Opened,
						relayers: pre.0.relayers,
						last_confirmed_nonce: pre.0.last_confirmed_nonce,
					}))
				};
			v1::InboundLanes::<T, I>::translate_values(translate_inbound);

			// `OutboundLanes` - add state to the old structs
			let translate_outbound =
				|pre: v1_wrong::OutboundLaneData| -> Option<v1::OutboundLaneData> {
					weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
					Some(v1::OutboundLaneData {
						state: v1::LaneState::Opened,
						oldest_unpruned_nonce: pre.oldest_unpruned_nonce,
						latest_received_nonce: pre.latest_received_nonce,
						latest_generated_nonce: pre.latest_generated_nonce,
					})
				};
			v1::OutboundLanes::<T, I>::translate_values(translate_outbound);

			weight
		}
	}
}
