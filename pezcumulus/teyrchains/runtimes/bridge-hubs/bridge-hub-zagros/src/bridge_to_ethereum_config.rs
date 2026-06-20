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

use crate::{
	bridge_common_config::BridgeReward,
	xcm_config,
	xcm_config::{RelayNetwork, RootLocation, TreasuryAccount, UniversalLocation, XcmConfig},
	Balances, BridgeRelayers, EthereumBeaconClient, EthereumInboundQueue, EthereumInboundQueueV2,
	EthereumOutboundQueue, EthereumOutboundQueueV2, EthereumSystem, EthereumSystemV2, MessageQueue,
	Runtime, RuntimeEvent, TransactionByteFee,
};
use hex_literal::hex;
use pezbp_asset_hub_zagros::CreateForeignAssetDeposit;
use pezbridge_hub_common::AggregateMessageOrigin;
use pezframe_support::{parameter_types, traits::Contains, weights::ConstantMultiplier};
use pezframe_system::EnsureRootWithSuccess;
use pezpallet_xcm::EnsureXcm;
use pezsnowbridge_beacon_primitives::{Fork, ForkVersions};
use pezsnowbridge_core::{gwei, meth, AllowSiblingsOnly, PricingParameters, Rewards};
use pezsnowbridge_inbound_queue_primitives::v2::CreateAssetCallInfo;
use pezsnowbridge_outbound_queue_primitives::{
	v1::{ConstantGasMeter, EthereumBlobExporter},
	v2::{ConstantGasMeter as ConstantGasMeterV2, EthereumBlobExporter as EthereumBlobExporterV2},
};
use pezsp_core::H160;
use pezsp_runtime::{
	traits::{ConstU32, ConstU8, Keccak256},
	FixedU128,
};
use testnet_teyrchains_constants::zagros::{
	currency::*,
	fee::WeightToFee,
	snowbridge::{
		AssetHubParaId, EthereumLocation, EthereumNetwork, FRONTEND_PALLET_INDEX,
		INBOUND_QUEUE_PALLET_INDEX_V1, INBOUND_QUEUE_PALLET_INDEX_V2,
	},
};
use teyrchains_common::{AccountId, Balance};
use xcm::prelude::{GlobalConsensus, InteriorLocation, Location, PalletInstance, Teyrchain};
use xcm_executor::XcmExecutor;
use zagros_runtime_constants::system_teyrchain::ASSET_HUB_ID;

pub const SLOTS_PER_EPOCH: u32 =
	pezsnowbridge_pezpallet_ethereum_client::config::SLOTS_PER_EPOCH as u32;

/// Exports message to the Ethereum Gateway contract.
pub type SnowbridgeExporter = EthereumBlobExporter<
	UniversalLocation,
	EthereumNetwork,
	pezsnowbridge_pezpallet_outbound_queue::Pezpallet<Runtime>,
	pezsnowbridge_core::AgentIdOf,
	EthereumSystem,
>;

pub type SnowbridgeExporterV2 = EthereumBlobExporterV2<
	UniversalLocation,
	EthereumNetwork,
	EthereumOutboundQueueV2,
	EthereumSystemV2,
	AssetHubParaId,
>;

// Ethereum Bridge
parameter_types! {
	pub storage EthereumGatewayAddress: H160 = H160(hex!("b1185ede04202fe62d38f5db72f71e38ff3e8305"));
}

parameter_types! {
	pub const CreateAssetCallIndex: [u8;2] = [53, 0];
	pub const SetReservesCallIndex: [u8;2] = [53, 33];
	pub Parameters: PricingParameters<u128> = PricingParameters {
		exchange_rate: FixedU128::from_rational(1, 400),
		fee_per_gas: gwei(20),
		rewards: Rewards { local: 1 * UNITS, remote: meth(1) },
		multiplier: FixedU128::from_rational(1, 1),
	};
	pub AssetHubFromEthereum: Location = Location::new(1, [GlobalConsensus(RelayNetwork::get()), Teyrchain(ASSET_HUB_ID)]);
	pub EthereumUniversalLocation: InteriorLocation = [GlobalConsensus(EthereumNetwork::get())].into();
	pub AssetHubUniversalLocation: InteriorLocation = [GlobalConsensus(RelayNetwork::get()), Teyrchain(ASSET_HUB_ID)].into();
	pub InboundQueueV2Location: InteriorLocation = [PalletInstance(INBOUND_QUEUE_PALLET_INDEX_V2)].into();
	pub const SnowbridgeReward: BridgeReward = BridgeReward::Snowbridge;
	pub CreateAssetCall: CreateAssetCallInfo = CreateAssetCallInfo {
		create_call: CreateAssetCallIndex::get(),
		deposit: CreateForeignAssetDeposit::get(),
		min_balance:1,
		set_reserves_call: SetReservesCallIndex::get(),
	};
	pub SnowbridgeFrontendLocation: Location = Location::new(1, [Teyrchain(ASSET_HUB_ID), PalletInstance(FRONTEND_PALLET_INDEX)]);
}

impl pezsnowbridge_pezpallet_inbound_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Verifier = pezsnowbridge_pezpallet_ethereum_client::Pezpallet<Runtime>;
	type Token = Balances;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmSender = crate::XcmRouter;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmSender = benchmark_helpers::DoNothingRouter;
	type ChannelLookup = EthereumSystem;
	type GatewayAddress = EthereumGatewayAddress;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = Runtime;
	type MessageConverter = pezsnowbridge_inbound_queue_primitives::v1::MessageToXcm<
		CreateAssetCallIndex,
		CreateForeignAssetDeposit,
		ConstU8<INBOUND_QUEUE_PALLET_INDEX_V1>,
		AccountId,
		Balance,
		EthereumSystem,
		EthereumUniversalLocation,
		AssetHubFromEthereum,
	>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type MaxMessageSize = ConstU32<2048>;
	type WeightInfo = crate::weights::pezsnowbridge_pezpallet_inbound_queue::WeightInfo<Runtime>;
	type PricingParameters = EthereumSystem;
	type AssetTransactor = <xcm_config::XcmConfig as xcm_executor::Config>::AssetTransactor;
}

impl pezsnowbridge_pezpallet_inbound_queue_v2::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Verifier = EthereumBeaconClient;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmSender = crate::XcmRouter;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmSender = benchmark_helpers::DoNothingRouter;
	type GatewayAddress = EthereumGatewayAddress;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = Runtime;
	type WeightInfo = crate::weights::pezsnowbridge_pezpallet_inbound_queue_v2::WeightInfo<Runtime>;
	type AssetHubParaId = AssetHubParaId;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type MessageConverter = pezsnowbridge_inbound_queue_primitives::v2::MessageToXcm<
		CreateAssetCall,
		EthereumNetwork,
		RelayNetwork,
		EthereumGatewayAddress,
		InboundQueueV2Location,
		AssetHubParaId,
		EthereumSystem,
		AccountId,
	>;
	type AccountToLocation = xcm_builder::AliasesIntoAccountId32<
		xcm_config::RelayNetwork,
		<Runtime as pezframe_system::Config>::AccountId,
	>;
	type RewardKind = BridgeReward;
	type DefaultRewardKind = SnowbridgeReward;
	type RewardPayment = BridgeRelayers;
}

impl pezsnowbridge_pezpallet_outbound_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Hashing = Keccak256;
	type MessageQueue = MessageQueue;
	type Decimals = ConstU8<12>;
	type MaxMessagePayloadSize = ConstU32<2048>;
	type MaxMessagesPerBlock = ConstU32<32>;
	type GasMeter = ConstantGasMeter;
	type Balance = Balance;
	type WeightToFee = WeightToFee;
	type WeightInfo = crate::weights::pezsnowbridge_pezpallet_outbound_queue::WeightInfo<Runtime>;
	type PricingParameters = EthereumSystem;
	type Channels = EthereumSystem;
}

impl pezsnowbridge_pezpallet_outbound_queue_v2::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Hashing = Keccak256;
	type MessageQueue = MessageQueue;
	// Maximum payload size for outbound messages.
	type MaxMessagePayloadSize = ConstU32<2048>;
	// Maximum number of outbound messages that can be committed per block.
	// It's benchmarked, including the entire process flow(initialize,submit,commit) in the
	// worst-case, Benchmark results in `../weights/pezsnowbridge_pezpallet_outbound_queue_v2.
	// rs` show that the `process` function consumes less than 1% of the block capacity, which is
	// safe enough.
	type MaxMessagesPerBlock = ConstU32<32>;
	type GasMeter = ConstantGasMeterV2;
	type Balance = Balance;
	type WeightToFee = WeightToFee;
	type Verifier = EthereumBeaconClient;
	type GatewayAddress = EthereumGatewayAddress;
	type WeightInfo =
		crate::weights::pezsnowbridge_pezpallet_outbound_queue_v2::WeightInfo<Runtime>;
	type EthereumNetwork = EthereumNetwork;
	type RewardKind = BridgeReward;
	type DefaultRewardKind = SnowbridgeReward;
	type RewardPayment = BridgeRelayers;
	type AggregateMessageOrigin = AggregateMessageOrigin;
	type OnNewCommitment = ();
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = Runtime;
}

#[cfg(not(any(feature = "std", feature = "fast-runtime", feature = "runtime-benchmarks", test)))]
parameter_types! {
	pub const ChainForkVersions: ForkVersions = ForkVersions {
		genesis: Fork {
			version: hex!("90000069"),
			epoch: 0,
		},
		altair: Fork {
			version: hex!("90000070"),
			epoch: 50,
		},
		bellatrix: Fork {
			version: hex!("90000071"),
			epoch: 100,
		},
		capella: Fork {
			version: hex!("90000072"),
			epoch: 56832,
		},
		deneb: Fork {
			version: hex!("90000073"),
			epoch: 132608,
		},
		electra: Fork {
			version: hex!("90000074"),
			epoch: 222464,
		},
		fulu: Fork {
			version: hex!("90000075"),
			epoch: 272640, // https://notes.ethereum.org/@bbusa/fusaka-bpo-timeline
		},
	};
}

#[cfg(any(feature = "std", feature = "fast-runtime", feature = "runtime-benchmarks", test))]
parameter_types! {
	pub const ChainForkVersions: ForkVersions = ForkVersions {
		genesis: Fork {
			version: hex!("00000000"),
			epoch: 0,
		},
		altair: Fork {
			version: hex!("01000000"),
			epoch: 0,
		},
		bellatrix: Fork {
			version: hex!("02000000"),
			epoch: 0,
		},
		capella: Fork {
			version: hex!("03000000"),
			epoch: 0,
		},
		deneb: Fork {
			version: hex!("04000000"),
			epoch: 0,
		},
		electra: Fork {
			version: hex!("05000000"),
			epoch: 0,
		},
		fulu: Fork {
			version: hex!("06000000"),
			epoch: 5000000,
		}
	};
}

impl pezsnowbridge_pezpallet_ethereum_client::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ForkVersions = ChainForkVersions;
	type FreeHeadersInterval = ConstU32<SLOTS_PER_EPOCH>;
	type WeightInfo = crate::weights::pezsnowbridge_pezpallet_ethereum_client::WeightInfo<Runtime>;
}

impl pezsnowbridge_pezpallet_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OutboundQueue = EthereumOutboundQueue;
	type SiblingOrigin = EnsureXcm<AllowSiblingsOnly>;
	type AgentIdOf = pezsnowbridge_core::AgentIdOf;
	type TreasuryAccount = TreasuryAccount;
	type Token = Balances;
	type WeightInfo = crate::weights::pezsnowbridge_pezpallet_system::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type DefaultPricingParameters = Parameters;
	type InboundDeliveryCost = EthereumInboundQueue;
	type UniversalLocation = UniversalLocation;
	type EthereumLocation = EthereumLocation;
}

pub struct AllowFromEthereumFrontend;
impl Contains<Location> for AllowFromEthereumFrontend {
	fn contains(location: &Location) -> bool {
		match location.unpack() {
			(1, [Teyrchain(para_id), PalletInstance(index)]) => {
				return *para_id == ASSET_HUB_ID && *index == FRONTEND_PALLET_INDEX
			},
			_ => false,
		}
	}
}

impl pezsnowbridge_pezpallet_system_v2::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OutboundQueue = EthereumOutboundQueueV2;
	type InboundQueue = EthereumInboundQueueV2;
	type FrontendOrigin = EnsureXcm<AllowFromEthereumFrontend>;
	type WeightInfo = crate::weights::pezsnowbridge_pezpallet_system_v2::WeightInfo<Runtime>;
	type GovernanceOrigin = EnsureRootWithSuccess<crate::AccountId, RootLocation>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_helpers {
	use crate::{
		bridge_to_ethereum_config::EthereumGatewayAddress, vec, EthereumBeaconClient, Runtime,
		RuntimeOrigin, System,
	};
	use codec::Encode;
	use hex_literal::hex;
	use pezframe_support::assert_ok;
	use pezsnowbridge_beacon_primitives::BeaconHeader;
	use pezsnowbridge_inbound_queue_primitives::EventFixture;
	use pezsnowbridge_pezpallet_inbound_queue::BenchmarkHelper;
	use pezsnowbridge_pezpallet_inbound_queue_fixtures::register_token::make_register_token_message;
	use pezsnowbridge_pezpallet_inbound_queue_v2::BenchmarkHelper as InboundQueueBenchmarkHelperV2;
	use pezsnowbridge_pezpallet_inbound_queue_v2_fixtures::register_token::make_register_token_message as make_register_token_message_v2;
	use pezsnowbridge_pezpallet_outbound_queue_v2::BenchmarkHelper as OutboundQueueBenchmarkHelperV2;
	use pezsp_core::H256;
	use xcm::latest::{Assets, Location, SendError, SendResult, SendXcm, Xcm, XcmHash};

	impl<T: pezsnowbridge_pezpallet_ethereum_client::Config> BenchmarkHelper<T> for Runtime {
		fn initialize_storage() -> EventFixture {
			let message = make_register_token_message();
			EthereumBeaconClient::store_finalized_header(
				message.finalized_header,
				message.block_roots_root,
			)
			.unwrap();
			System::set_storage(
				RuntimeOrigin::root(),
				vec![(
					EthereumGatewayAddress::key().to_vec(),
					hex!("EDa338E4dC46038493b885327842fD3E301CaB39").to_vec(),
				)],
			)
			.unwrap();
			message
		}
	}

	impl<T: pezsnowbridge_pezpallet_inbound_queue_v2::Config> InboundQueueBenchmarkHelperV2<T>
		for Runtime
	{
		fn initialize_storage() -> EventFixture {
			let message = make_register_token_message_v2();

			assert_ok!(EthereumBeaconClient::store_finalized_header(
				message.finalized_header,
				message.block_roots_root,
			));

			message
		}
	}

	impl<T: pezsnowbridge_pezpallet_outbound_queue_v2::Config> OutboundQueueBenchmarkHelperV2<T>
		for Runtime
	{
		fn initialize_storage(beacon_header: BeaconHeader, block_roots_root: H256) {
			EthereumBeaconClient::store_finalized_header(beacon_header, block_roots_root).unwrap();
		}
	}

	pub struct DoNothingRouter;
	impl SendXcm for DoNothingRouter {
		type Ticket = Xcm<()>;

		fn validate(
			_dest: &mut Option<Location>,
			xcm: &mut Option<Xcm<()>>,
		) -> SendResult<Self::Ticket> {
			Ok((xcm.clone().unwrap(), Assets::new()))
		}
		fn deliver(xcm: Xcm<()>) -> Result<XcmHash, SendError> {
			let hash = xcm.using_encoded(pezsp_io::hashing::blake2_256);
			Ok(hash)
		}
	}

	impl pezsnowbridge_pezpallet_system::BenchmarkHelper<RuntimeOrigin> for () {
		fn make_xcm_origin(location: Location) -> RuntimeOrigin {
			RuntimeOrigin::from(pezpallet_xcm::Origin::Xcm(location))
		}
	}

	impl pezsnowbridge_pezpallet_system_v2::BenchmarkHelper<RuntimeOrigin> for () {
		fn make_xcm_origin(location: Location) -> RuntimeOrigin {
			RuntimeOrigin::from(pezpallet_xcm::Origin::Xcm(location))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bridge_hub_inbound_queue_pallet_index_is_correct() {
		assert_eq!(
			INBOUND_QUEUE_PALLET_INDEX_V1,
			<EthereumInboundQueue as pezframe_support::traits::PalletInfoAccess>::index() as u8
		);
	}

	#[test]
	fn bridge_hub_inbound_v2_queue_pallet_index_is_correct() {
		assert_eq!(
			INBOUND_QUEUE_PALLET_INDEX_V2,
			<EthereumInboundQueueV2 as pezframe_support::traits::PalletInfoAccess>::index() as u8
		);
	}
}

pub(crate) mod migrations {
	use pezframe_support::pezpallet_prelude::*;
	use pezsnowbridge_core::TokenId;

	#[pezframe_support::storage_alias]
	pub type OldNativeToForeignId<T: pezsnowbridge_pezpallet_system::Config> = StorageMap<
		pezsnowbridge_pezpallet_system::Pezpallet<T>,
		Blake2_128Concat,
		xcm::v4::Location,
		TokenId,
		OptionQuery,
	>;

	/// One shot migration for NetworkId::Zagros to NetworkId::ByGenesis(ZAGROS_GENESIS_HASH)
	pub struct MigrationForXcmV5<T: pezsnowbridge_pezpallet_system::Config>(
		core::marker::PhantomData<T>,
	);
	impl<T: pezsnowbridge_pezpallet_system::Config> pezframe_support::traits::OnRuntimeUpgrade
		for MigrationForXcmV5<T>
	{
		fn on_runtime_upgrade() -> Weight {
			let mut weight = T::DbWeight::get().reads(1);

			let translate_zagros = |pre: xcm::v4::Location| -> Option<xcm::v5::Location> {
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				Some(xcm::v5::Location::try_from(pre).expect("valid location"))
			};
			pezsnowbridge_pezpallet_system::ForeignToNativeId::<T>::translate_values(
				translate_zagros,
			);

			weight
		}
	}
}
