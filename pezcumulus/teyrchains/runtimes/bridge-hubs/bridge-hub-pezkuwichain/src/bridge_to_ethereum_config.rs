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

#[cfg(not(feature = "runtime-benchmarks"))]
use crate::XcmRouter;
use crate::{
	xcm_config, xcm_config::UniversalLocation, Balances, EthereumInboundQueue,
	EthereumOutboundQueue, EthereumSystem, MessageQueue, Runtime, RuntimeEvent, TransactionByteFee,
	TreasuryAccount,
};
use pezsnowbridge_beacon_primitives::{Fork, ForkVersions};
use pezsnowbridge_core::{gwei, meth, AllowSiblingsOnly, PricingParameters, Rewards};
use pezsnowbridge_inbound_queue_primitives::v1::MessageToXcm;
use pezsnowbridge_outbound_queue_primitives::v1::EthereumBlobExporter;
use teyrchains_common::{AccountId, Balance};

use pezsp_core::H160;
use testnet_teyrchains_constants::pezkuwichain::{
	currency::*,
	fee::WeightToFee,
	snowbridge::{EthereumLocation, EthereumNetwork, INBOUND_QUEUE_PALLET_INDEX},
};

use crate::xcm_config::RelayNetwork;
#[cfg(feature = "runtime-benchmarks")]
use benchmark_helpers::DoNothingRouter;
use hex_literal::hex;
use pezbp_asset_hub_pezkuwichain::CreateForeignAssetDeposit;
use pezframe_support::{parameter_types, weights::ConstantMultiplier};
use pezpallet_xcm::EnsureXcm;
use pezsp_runtime::{
	traits::{ConstU32, ConstU8, Keccak256},
	FixedU128,
};
use xcm::prelude::{GlobalConsensus, InteriorLocation, Location, Teyrchain};

/// Exports message to the Ethereum Gateway contract.
pub type SnowbridgeExporter = EthereumBlobExporter<
	UniversalLocation,
	EthereumNetwork,
	pezsnowbridge_pezpallet_outbound_queue::Pezpallet<Runtime>,
	pezsnowbridge_core::AgentIdOf,
	EthereumSystem,
>;

// Ethereum Bridge
parameter_types! {
	pub storage EthereumGatewayAddress: H160 = H160(hex!("EDa338E4dC46038493b885327842fD3E301CaB39"));
}

parameter_types! {
	pub const CreateAssetCall: [u8;2] = [53, 0];
	pub Parameters: PricingParameters<u128> = PricingParameters {
		exchange_rate: FixedU128::from_rational(1, 400),
		fee_per_gas: gwei(20),
		rewards: Rewards { local: 1 * UNITS, remote: meth(1) },
		multiplier: FixedU128::from_rational(1, 1),
	};
	pub AssetHubFromEthereum: Location = Location::new(1,[GlobalConsensus(RelayNetwork::get()),Teyrchain(pezkuwichain_runtime_constants::system_teyrchain::ASSET_HUB_ID)]);
	pub EthereumUniversalLocation: InteriorLocation = [GlobalConsensus(EthereumNetwork::get())].into();
}

impl pezsnowbridge_pezpallet_inbound_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Verifier = pezsnowbridge_pezpallet_ethereum_client::Pezpallet<Runtime>;
	type Token = Balances;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmSender = XcmRouter;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmSender = DoNothingRouter;
	type ChannelLookup = EthereumSystem;
	type GatewayAddress = EthereumGatewayAddress;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = Runtime;
	type MessageConverter = MessageToXcm<
		CreateAssetCall,
		CreateForeignAssetDeposit,
		ConstU8<INBOUND_QUEUE_PALLET_INDEX>,
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

impl pezsnowbridge_pezpallet_outbound_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Hashing = Keccak256;
	type MessageQueue = MessageQueue;
	type Decimals = ConstU8<12>;
	type MaxMessagePayloadSize = ConstU32<2048>;
	type MaxMessagesPerBlock = ConstU32<32>;
	type GasMeter = crate::ConstantGasMeter;
	type Balance = Balance;
	type WeightToFee = WeightToFee;
	type WeightInfo = crate::weights::pezsnowbridge_pezpallet_outbound_queue::WeightInfo<Runtime>;
	type PricingParameters = EthereumSystem;
	type Channels = EthereumSystem;
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

pub const SLOTS_PER_EPOCH: u32 =
	pezsnowbridge_pezpallet_ethereum_client::config::SLOTS_PER_EPOCH as u32;

impl pezsnowbridge_pezpallet_ethereum_client::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ForkVersions = ChainForkVersions;
	// Free consensus update every epoch. Works out to be 225 updates per day.
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

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_helpers {
	use crate::{EthereumBeaconClient, Runtime, RuntimeOrigin};
	use codec::Encode;
	use pezsnowbridge_inbound_queue_primitives::EventFixture;
	use pezsnowbridge_pezpallet_inbound_queue::BenchmarkHelper;
	use pezsnowbridge_pezpallet_inbound_queue_fixtures::register_token::make_register_token_message;
	use xcm::latest::{Assets, Location, SendError, SendResult, SendXcm, Xcm, XcmHash};

	impl<T: pezsnowbridge_pezpallet_ethereum_client::Config> BenchmarkHelper<T> for Runtime {
		fn initialize_storage() -> EventFixture {
			let message = make_register_token_message();
			EthereumBeaconClient::store_finalized_header(
				message.finalized_header,
				message.block_roots_root,
			)
			.unwrap();
			message
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
}
