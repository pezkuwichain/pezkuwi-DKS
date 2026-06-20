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

//! Bridge definitions used on BridgeHubPezkuwichain for bridging to Pezkuwichain Bulletin.
//!
//! Pezkuwichain Bulletin chain will be the 1:1 copy of the Pezkuwi Bulletin, so we
//! are reusing Pezkuwi Bulletin chain primitives everywhere here.

use crate::{
	bridge_common_config::RelayersForPermissionlessLanesInstance, weights,
	xcm_config::UniversalLocation, AccountId, Balance, Balances, BridgePezkuwichainBulletinGrandpa,
	BridgePezkuwichainBulletinMessages, Runtime, RuntimeEvent, RuntimeHoldReason,
	XcmOverPezkuwichainBulletin, XcmRouter,
};
use pezbp_messages::{
	source_chain::FromBridgedChainMessagesDeliveryProof,
	target_chain::FromBridgedChainMessagesProof, LegacyLaneId,
};

use pezframe_support::{
	parameter_types,
	traits::{Equals, PalletInfoAccess},
};
use pezframe_system::{EnsureNever, EnsureRoot};
use pezkuwi_teyrchain_primitives::primitives::Sibling;
use pezpallet_bridge_messages::LaneIdOf;
use pezpallet_bridge_relayers::extension::{
	BridgeRelayersTransactionExtension, WithMessagesExtensionConfig,
};
use pezpallet_xcm_bridge_hub::XcmAsPlainPayload;
use testnet_teyrchains_constants::pezkuwichain::currency::UNITS as TYR;
use xcm::{
	latest::prelude::*,
	prelude::{InteriorLocation, NetworkId},
	AlwaysV5,
};
use xcm_builder::{BridgeBlobDispatcher, ParentIsPreset, SiblingTeyrchainConvertsVia};

parameter_types! {
	/// Interior location (relative to this runtime) of the with-PezkuwichainBulletin messages pezpallet.
	pub BridgePezkuwichainToPezkuwichainBulletinMessagesPalletInstance: InteriorLocation = [
		PalletInstance(<BridgePezkuwichainBulletinMessages as PalletInfoAccess>::index() as u8)
	].into();
	/// Pezkuwichain Bulletin Network identifier.
	pub PezkuwichainBulletinGlobalConsensusNetwork: NetworkId = NetworkId::PezkuwiBulletin;
	/// Relative location of the Pezkuwichain Bulletin chain.
	pub PezkuwichainBulletinGlobalConsensusNetworkLocation: Location = Location::new(
		2,
		[GlobalConsensus(PezkuwichainBulletinGlobalConsensusNetwork::get())]
	);

	// see the `FEE_BOOST_PER_RELAY_HEADER` constant get the meaning of this value
	pub PriorityBoostPerRelayHeader: u64 = 58_014_163_614_163;

	/// Priority boost that the registered relayer receives for every additional message in the message
	/// delivery transaction.
	///
	/// It is determined semi-automatically - see `FEE_BOOST_PER_MESSAGE` constant to get the
	/// meaning of this value.
	pub PriorityBoostPerMessage: u64 = 364_088_888_888_888;

	/// PeoplePezkuwichain location
	pub PeoplePezkuwichainLocation: Location = Location::new(1, [Teyrchain(pezkuwichain_runtime_constants::system_teyrchain::PEOPLE_ID)]);

	pub storage BridgeDeposit: Balance = 5 * TYR;
}

/// Proof of messages, coming from Pezkuwichain Bulletin chain.
pub type FromPezkuwichainBulletinMessagesProof<MI> =
	FromBridgedChainMessagesProof<pezbp_pezkuwi_bulletin::Hash, LaneIdOf<Runtime, MI>>;
/// Messages delivery proof for Pezkuwichain Bridge Hub -> Pezkuwichain Bulletin messages.
pub type ToPezkuwichainBulletinMessagesDeliveryProof<MI> =
	FromBridgedChainMessagesDeliveryProof<pezbp_pezkuwi_bulletin::Hash, LaneIdOf<Runtime, MI>>;

/// Dispatches received XCM messages from other bridge.
type FromPezkuwichainBulletinMessageBlobDispatcher = BridgeBlobDispatcher<
	XcmRouter,
	UniversalLocation,
	BridgePezkuwichainToPezkuwichainBulletinMessagesPalletInstance,
>;

/// Transaction extension that refunds relayers that are delivering messages from the Pezkuwichain
/// Bulletin chain.
pub type OnBridgeHubPezkuwichainRefundPezkuwichainBulletinMessages =
	BridgeRelayersTransactionExtension<
		Runtime,
		WithMessagesExtensionConfig<
			StrOnBridgeHubPezkuwichainRefundPezkuwichainBulletinMessages,
			Runtime,
			WithPezkuwichainBulletinMessagesInstance,
			RelayersForPermissionlessLanesInstance,
			PriorityBoostPerMessage,
		>,
	>;
pezbp_runtime::generate_static_str_provider!(
	OnBridgeHubPezkuwichainRefundPezkuwichainBulletinMessages
);

/// Add XCM messages support for BridgeHubPezkuwichain to support Pezkuwichain->Pezkuwichain
/// Bulletin XCM messages.
pub type WithPezkuwichainBulletinMessagesInstance = pezpallet_bridge_messages::Instance4;
impl pezpallet_bridge_messages::Config<WithPezkuwichainBulletinMessagesInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo =
		weights::pezpallet_bridge_messages_pezkuwichain_to_pezkuwichain_bulletin::WeightInfo<
			Runtime,
		>;

	type ThisChain = pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichain;
	type BridgedChain = pezbp_pezkuwi_bulletin::PezkuwiBulletin;
	type BridgedHeaderChain = BridgePezkuwichainBulletinGrandpa;

	type OutboundPayload = XcmAsPlainPayload;
	type InboundPayload = XcmAsPlainPayload;
	type LaneId = LegacyLaneId;

	type DeliveryPayments = ();
	type DeliveryConfirmationPayments = ();

	type MessageDispatch = XcmOverPezkuwichainBulletin;
	type OnMessagesDelivered = XcmOverPezkuwichainBulletin;
}

/// Add support for the export and dispatch of XCM programs.
pub type XcmOverPezkuwiBulletinInstance = pezpallet_xcm_bridge_hub::Instance2;
impl pezpallet_xcm_bridge_hub::Config<XcmOverPezkuwiBulletinInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type UniversalLocation = UniversalLocation;
	type BridgedNetwork = PezkuwichainBulletinGlobalConsensusNetworkLocation;
	type BridgeMessagesPalletInstance = WithPezkuwichainBulletinMessagesInstance;

	type MessageExportPrice = ();
	type DestinationVersion = AlwaysV5;

	type ForceOrigin = EnsureRoot<AccountId>;
	// We don't want to allow creating bridges for this instance.
	type OpenBridgeOrigin = EnsureNever<Location>;
	// Converter aligned with `OpenBridgeOrigin`.
	type BridgeOriginAccountIdConverter =
		(ParentIsPreset<AccountId>, SiblingTeyrchainConvertsVia<Sibling, AccountId>);

	type BridgeDeposit = BridgeDeposit;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	// Do not require deposit from People teyrchains.
	type AllowWithoutBridgeDeposit = Equals<PeoplePezkuwichainLocation>;

	type LocalXcmChannelManager = ();
	type BlobDispatcher = FromPezkuwichainBulletinMessageBlobDispatcher;
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::bridge_common_config::BridgeGrandpaPezkuwichainBulletinInstance;
	use pezbridge_runtime_common::{
		assert_complete_bridge_types, integrity::check_message_lane_weights,
	};
	use testnet_teyrchains_constants::pezkuwichain;
	use teyrchains_common::Balance;

	/// Every additional message in the message delivery transaction boosts its priority.
	/// So the priority of transaction with `N+1` messages is larger than priority of
	/// transaction with `N` messages by the `PriorityBoostPerMessage`.
	///
	/// Economically, it is an equivalent of adding tip to the transaction with `N` messages.
	/// The `FEE_BOOST_PER_MESSAGE` constant is the value of this tip.
	///
	/// We want this tip to be large enough (delivery transactions with more messages = less
	/// operational costs and a faster bridge), so this value should be significant.
	const FEE_BOOST_PER_MESSAGE: Balance = 2 * pezkuwichain::currency::UNITS;

	// see `FEE_BOOST_PER_MESSAGE` comment
	const FEE_BOOST_PER_RELAY_HEADER: Balance = 2 * pezkuwichain::currency::UNITS;

	#[test]
	fn ensure_bridge_hub_pezkuwichain_message_lane_weights_are_correct() {
		check_message_lane_weights::<
			pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichain,
			Runtime,
			WithPezkuwichainBulletinMessagesInstance,
		>(
			pezbp_pezkuwi_bulletin::EXTRA_STORAGE_PROOF_SIZE,
			pezbp_bridge_hub_pezkuwichain::MAX_UNREWARDED_RELAYERS_IN_CONFIRMATION_TX,
			pezbp_bridge_hub_pezkuwichain::MAX_UNCONFIRMED_MESSAGES_IN_CONFIRMATION_TX,
			true,
		);
	}

	#[test]
	fn ensure_bridge_integrity() {
		assert_complete_bridge_types!(
			runtime: Runtime,
			with_bridged_chain_messages_instance: WithPezkuwichainBulletinMessagesInstance,
			this_chain: pezbp_bridge_hub_pezkuwichain::BridgeHubPezkuwichain,
			bridged_chain: pezbp_pezkuwi_bulletin::PezkuwiBulletin,
			expected_payload_type: XcmAsPlainPayload,
		);

		// we can't use `assert_complete_bridge_constants` here, because there's a trick with
		// Bulletin chain - it has the same (almost) runtime for Pezkuwi Bulletin and Pezkuwichain
		// Bulletin, so we have to adhere Pezkuwi names here

		pezpallet_bridge_relayers::extension::per_relay_header::ensure_priority_boost_is_sane::<
			Runtime,
			BridgeGrandpaPezkuwichainBulletinInstance,
			PriorityBoostPerRelayHeader,
		>(FEE_BOOST_PER_RELAY_HEADER);

		pezpallet_bridge_relayers::extension::per_message::ensure_priority_boost_is_sane::<
			Runtime,
			WithPezkuwichainBulletinMessagesInstance,
			PriorityBoostPerMessage,
		>(FEE_BOOST_PER_MESSAGE);

		let expected: InteriorLocation = PalletInstance(
			pezbp_bridge_hub_pezkuwichain::WITH_BRIDGE_PEZKUWICHAIN_TO_BULLETIN_MESSAGES_PALLET_INDEX,
		)
		.into();

		assert_eq!(BridgePezkuwichainToPezkuwichainBulletinMessagesPalletInstance::get(), expected,);
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
		[GlobalConsensus(PezkuwichainBulletinGlobalConsensusNetwork::get())].into();
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
