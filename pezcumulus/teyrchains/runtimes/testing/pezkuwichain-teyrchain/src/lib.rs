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

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod genesis_config_presets;

extern crate alloc;

use alloc::vec::Vec;
use pezcumulus_pezpallet_teyrchain_system::RelayNumberMonotonicallyIncreases;
use pezkuwi_runtime_common::xcm_sender::NoPriceForMessageDelivery;
use pezsp_api::impl_runtime_apis;
use pezsp_core::OpaqueMetadata;
use pezsp_runtime::{
	generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, Hash as HashT},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};
#[cfg(feature = "std")]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use pezframe_support::{
	construct_runtime, derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstBool, ConstU32, ConstU64, ConstU8, Contains, EitherOfDiverse,
		Everything, IsInVec, Nothing, Randomness,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		ConstantMultiplier, IdentityFee, Weight,
	},
	StorageValue,
};
use pezframe_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureSigned,
};
pub use pezpallet_balances::Call as BalancesCall;
pub use pezpallet_timestamp::Call as TimestampCall;
pub use pezsp_consensus_aura::sr25519::AuthorityId as AuraId;
#[cfg(any(feature = "std", test))]
pub use pezsp_runtime::BuildStorage;
pub use pezsp_runtime::{Perbill, Permill};

use pezcumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use pezframe_support::traits::{Disabled, TransformOrigin};
use teyrchains_common::{
	impls::{AssetsFrom, NonZeroIssuance},
	message_queue::{NarrowOriginToSibling, ParaIdToSibling},
	AccountId, AssetIdForTrustBackedAssets, Signature,
};
use xcm_builder::{
	AllowHrmpNotificationsFromRelayChain, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AsPrefixedGeneralIndex, ConvertedConcreteId, FrameTransactionalProcessor, FungiblesAdapter,
	LocalMint, TrailingSetTopicAsId, WithUniqueTopic,
};
use xcm_executor::traits::JustTry;

// XCM imports
use pezkuwi_teyrchain_primitives::primitives::Sibling;
use pezpallet_xcm::{EnsureXcm, IsMajorityOfBody, XcmPassthrough};
use xcm::latest::{prelude::*, PEZKUWICHAIN_GENESIS_HASH};
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowTopLevelPaidExecutionFrom,
	EnsureXcmOrigin, FixedWeightBounds, FungibleAdapter, IsConcrete, NativeAsset,
	ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SiblingTeyrchainAsNative,
	SiblingTeyrchainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::XcmExecutor;

pub type SessionHandlers = ();

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

/// This runtime version.
#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("test-teyrchain"),
	impl_name: alloc::borrow::Cow::Borrowed("test-teyrchain"),
	authoring_version: 1,
	spec_version: 1_020_001,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 6,
	system_version: 0,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

pub const EPOCH_DURATION_IN_BLOCKS: u32 = 10 * MINUTES;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const HEZ: Balance = 1_000_000_000_000;
pub const MILLIROC: Balance = 1_000_000_000;
pub const MICROROC: Balance = 1_000_000;

// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
	pezcumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
/// into the relay chain.
const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;
/// How many teyrchain blocks are processed by the relay chain per parent. Limits the
/// number of blocks authored per slot.
const BLOCK_PROCESSING_VELOCITY: u32 = 2;
/// Relay chain slot duration, in milliseconds.
const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The block type.
	type Block = Block;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = RocksDbWeight;
	type BaseCallFilter = pezframe_support::traits::Everything;
	type SystemWeightInfo = ();
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = pezcumulus_pezpallet_teyrchain_system::TeyrchainSetCode<Self>;
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
	type SingleBlockMigrations = RemoveCollectiveFlip;
}

impl pezcumulus_pezpallet_weight_reclaim::Config for Runtime {
	type WeightInfo = ();
}

impl pezpallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<0>;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = MILLIROC;
	pub const TransferFee: u128 = MILLIROC;
	pub const CreationFee: u128 = MILLIROC;
	pub const TransactionByteFee: u128 = MICROROC;
}

impl pezpallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type DoneSlashHandler = ();
}

impl pezpallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pezpallet_transaction_payment::FungibleAdapter<Balances, ()>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = ();
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightInfo = ();
}

impl pezpallet_sudo::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pezpallet_sudo::weights::BizinikiwiWeight<Runtime>;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

type ConsensusHook = pezcumulus_pezpallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,
	RELAY_CHAIN_SLOT_DURATION_MILLIS,
	BLOCK_PROCESSING_VELOCITY,
	UNINCLUDED_SEGMENT_CAPACITY,
>;

impl pezcumulus_pezpallet_teyrchain_system::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = teyrchain_info::Pezpallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpQueue = pezframe_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
	type RelayParentOffset = ConstU32<0>;
}

impl teyrchain_info::Config for Runtime {}

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pezpallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	// The XCMP queue pezpallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = pezsp_core::ConstU32<{ 103 * 1024 }>;
	type MaxStale = pezsp_core::ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = ();
}

impl pezcumulus_pezpallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const RocLocation: Location = Location::parent();
	pub const PezkuwichainNetwork: NetworkId = NetworkId::ByGenesis(PEZKUWICHAIN_GENESIS_HASH);
	pub RelayChainOrigin: RuntimeOrigin = pezcumulus_pezpallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation = [GlobalConsensus(PezkuwichainNetwork::get()), Teyrchain(TeyrchainInfo::teyrchain_id().into())].into();
	pub CheckingAccount: AccountId = PezkuwiXcm::check_account();
}

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling teyrchain origins convert to AccountId via the `ParaId::into`.
	SiblingTeyrchainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<PezkuwichainNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type FungibleTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RocLocation>,
	// Do a simple punn to convert an AccountId32 Location into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	ConvertedConcreteId<
		AssetIdForTrustBackedAssets,
		u64,
		AsPrefixedGeneralIndex<
			SystemAssetHubAssetsPalletLocation,
			AssetIdForTrustBackedAssets,
			JustTry,
		>,
		JustTry,
	>,
	// Convert an XCM Location into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	LocalMint<NonZeroIssuance<AccountId, Assets>>,
	// The account to use for tracking teleports.
	CheckingAccount,
>;
/// Means for transacting assets on this chain.
pub type AssetTransactors = (FungibleTransactor, FungiblesTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
	// recognised.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Teyrchains; will convert to a `SiblingPara` origin when
	// recognised.
	SiblingTeyrchainAsNative<pezcumulus_pezpallet_xcm::Origin, RuntimeOrigin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<PezkuwichainNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pezpallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
	// One HEZ buys 1 second of weight.
	pub const WeightPrice: (Location, u128) = (Location::parent(), HEZ);
	pub const MaxInstructions: u32 = 100;
}

pub struct ParentOrParentsUnitPlurality;
impl Contains<Location> for ParentOrParentsUnitPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, []) | (1, [Plurality { id: BodyId::Unit, .. }]))
	}
}

pub struct AssetHub;
impl Contains<Location> for AssetHub {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, [Teyrchain(1000)]))
	}
}

pub type Barrier = TrailingSetTopicAsId<(
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	// Parent & its unit plurality gets free execution.
	AllowExplicitUnpaidExecutionFrom<ParentOrParentsUnitPlurality>,
	// The network's Asset Hub gets free execution.
	AllowExplicitUnpaidExecutionFrom<AssetHub>,
	// Expected responses are OK.
	AllowKnownQueryResponses<PezkuwiXcm>,
	// Subscriptions for version tracking are OK.
	AllowSubscriptionsFrom<Everything>,
	// HRMP notifications from the relay chain are OK.
	AllowHrmpNotificationsFromRelayChain,
)>;

parameter_types! {
	pub MaxAssetsIntoHolding: u32 = 64;
	pub SystemAssetHubLocation: Location = Location::new(1, [Teyrchain(1000)]);
	// ALWAYS ensure that the index in PalletInstance stays up-to-date with
	// the Relay Chain's Asset Hub's Assets pezpallet index
	pub SystemAssetHubAssetsPalletLocation: Location =
		Location::new(1, [Teyrchain(1000), PalletInstance(50)]);
}

pub type Reserves = (NativeAsset, AssetsFrom<SystemAssetHubLocation>);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type XcmEventEmitter = PezkuwiXcm;
	// How to withdraw and deposit an asset.
	type AssetTransactor = AssetTransactors;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = Reserves;
	type IsTeleporter = NativeAsset; // <- should be enough to allow teleportation of HEZ
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = UsingComponents<IdentityFee<Balance>, RocLocation, AccountId, Balances, ()>;
	type ResponseHandler = PezkuwiXcm;
	type AssetTrap = PezkuwiXcm;
	type AssetClaims = PezkuwiXcm;
	type SubscriptionService = PezkuwiXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = PezkuwiXcm;
}

/// Converts a local signed origin into an XCM location. Forms the basis for local origins
/// sending/executing XCMs.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, PezkuwichainNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
	// Two routers - use UMP to communicate with the relay chain:
	pezcumulus_primitives_utility::ParentAsUmp<TeyrchainSystem, (), ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
)>;

impl pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Nothing;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pezpallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pezpallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	// Aliasing is disabled: xcm_executor::Config::Aliasers is set to `Nothing`.
	type AuthorizedAliasConsideration = Disabled;
}

impl pezcumulus_pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl pezcumulus_pezpallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = TeyrchainSystem;
	type VersionWrapper = ();
	// Enqueue XCMP messages from siblings for later processing.
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = ConstU32<1_000>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	// Most on-chain HRMP channels are configured to use 102400 bytes of max message size, so we
	// need to set the page size larger than that until we reduce the channel size on-chain.
	type MaxPageSize = ConstU32<{ 103 * 1024 }>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = pezcumulus_pezpallet_xcmp_queue::weights::BizinikiwiWeight<Runtime>;
	type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
}

impl pezcumulus_ping::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type WeightInfo = ();
}

parameter_types! {
	pub const AssetDeposit: Balance = HEZ;
	pub const AssetAccountDeposit: Balance = HEZ;
	pub const ApprovalDeposit: Balance = 100 * MILLIROC;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = HEZ;
	pub const MetadataDepositPerByte: Balance = 10 * MILLIROC;
	pub const UnitBody: BodyId = BodyId::Unit;
}

/// A majority of the Unit body from Pezkuwichain over XCM is our required administration origin.
pub type AdminOrigin =
	EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsMajorityOfBody<RocLocation, UnitBody>>>;

impl pezpallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u64;
	type AssetId = AssetIdForTrustBackedAssets;
	type AssetIdParameter = codec::Compact<AssetIdForTrustBackedAssets>;
	type ReserveData = ();
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = AdminOrigin;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pezpallet_assets::weights::BizinikiwiWeight<Runtime>;
	type CallbackHandle = ();
	type AssetAccountDeposit = AssetAccountDeposit;
	type RemoveItemsLimit = pezframe_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pezpallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<true>;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

construct_runtime! {
	pub enum Runtime
	{
		System: pezframe_system,
		Timestamp: pezpallet_timestamp,
		Sudo: pezpallet_sudo,
		TransactionPayment: pezpallet_transaction_payment,
		WeightReclaim: pezcumulus_pezpallet_weight_reclaim,

		TeyrchainSystem: pezcumulus_pezpallet_teyrchain_system = 20,
		TeyrchainInfo: teyrchain_info = 21,

		Balances: pezpallet_balances = 30,
		Assets: pezpallet_assets = 31,

		Aura: pezpallet_aura,
		AuraExt: pezcumulus_pezpallet_aura_ext,

		// XCM helpers.
		XcmpQueue: pezcumulus_pezpallet_xcmp_queue = 50,
		PezkuwiXcm: pezpallet_xcm = 51,
		CumulusXcm: pezcumulus_pezpallet_xcm = 52,
		// RIP DmpQueue 53
		MessageQueue: pezpallet_message_queue = 54,

		Spambot: pezcumulus_ping = 99,
	}
}

/// Balance of an account.
pub type Balance = u128;
/// Index of a transaction in the chain.
pub type Nonce = u32;
/// A hash of some data used by the chain.
pub type Hash = <BlakeTwo256 as HashT>::Output;
/// An index to a block.
pub type BlockNumber = u32;
/// The address format for describing accounts.
pub type Address = pezsp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The extension to the basic transaction logic.
pub type TxExtension = pezcumulus_pezpallet_weight_reclaim::StorageWeightReclaim<
	Runtime,
	(
		pezframe_system::AuthorizeCall<Runtime>,
		pezframe_system::CheckNonZeroSender<Runtime>,
		pezframe_system::CheckSpecVersion<Runtime>,
		pezframe_system::CheckTxVersion<Runtime>,
		pezframe_system::CheckGenesis<Runtime>,
		pezframe_system::CheckEra<Runtime>,
		pezframe_system::CheckNonce<Runtime>,
		pezframe_system::CheckWeight<Runtime>,
		pezpallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	),
>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
/// Executive: handles dispatch to the various modules.
pub type Executive = pezframe_executive::Executive<
	Runtime,
	Block,
	pezframe_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

pub struct RemoveCollectiveFlip;
impl pezframe_support::traits::OnRuntimeUpgrade for RemoveCollectiveFlip {
	fn on_runtime_upgrade() -> Weight {
		use pezframe_support::storage::migration;
		// Remove the storage value `RandomMaterial` from removed pezpallet
		// `RandomnessCollectiveFlip`
		#[allow(deprecated)]
		migration::remove_storage_prefix(b"RandomnessCollectiveFlip", b"RandomMaterial", b"");
		<Runtime as pezframe_system::Config>::DbWeight::get().writes(1)
	}
}

impl_runtime_apis! {
	impl pezsp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: <Block as BlockT>::LazyBlock) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> pezsp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl pezsp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> alloc::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl pezsp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(
			extrinsic: <Block as BlockT>::Extrinsic,
		) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: pezsp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: <Block as BlockT>::LazyBlock, data: pezsp_inherents::InherentData) -> pezsp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl pezsp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl pezsp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl pezsp_session::SessionKeys<Block> for Runtime {
		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, pezsp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}

		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}
	}

	impl pezsp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> pezsp_consensus_aura::SlotDuration {
			pezsp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			pezpallet_aura::Authorities::<Runtime>::get().into_inner()
		}
	}

	impl pezframe_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pezpallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pezpallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pezpallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pezpallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pezcumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> pezcumulus_primitives_core::CollationInfo {
			TeyrchainSystem::collect_collation_info(header)
		}
	}

	impl pezsp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> pezsp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<pezsp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, &genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<pezsp_genesis_builder::PresetId> {
			genesis_config_presets::preset_names()
		}
	}

	impl pezcumulus_primitives_core::RelayParentOffsetApi<Block> for Runtime {
		fn relay_parent_offset() -> u32 {
			0
		}
	}

	impl pezcumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
		fn can_build_upon(
			included_hash: <Block as BlockT>::Hash,
			slot: pezcumulus_primitives_aura::Slot,
		) -> bool {
			ConsensusHook::can_build_upon(included_hash, slot)
		}
	}

	impl pezcumulus_primitives_core::GetTeyrchainInfo<Block> for Runtime {
		fn teyrchain_id() -> ParaId {
			TeyrchainInfo::teyrchain_id()
		}
	}
}

pezcumulus_pezpallet_teyrchain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = pezcumulus_pezpallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
