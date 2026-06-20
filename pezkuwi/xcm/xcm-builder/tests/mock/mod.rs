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

use core::cell::RefCell;
use pezframe_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{Disabled, Everything, Nothing},
	weights::Weight,
};
use pezframe_system::EnsureRoot;
use pezsp_runtime::{traits::IdentityLookup, AccountId32, BuildStorage};
use primitive_types::H256;

use pezkuwi_runtime_teyrchains::{configuration, origin, shared};
use pezkuwi_teyrchain_primitives::primitives::Id as ParaId;
use xcm::latest::{opaque, prelude::*};
use xcm_executor::XcmExecutor;

use pezstaging_xcm_builder as xcm_builder;

use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom,
	ChildSystemTeyrchainAsSuperuser, ChildTeyrchainAsNative, ChildTeyrchainConvertsVia,
	EnsureDecodableXcm, FixedRateOfFungible, FixedWeightBounds, FungibleAdapter,
	IsChildSystemTeyrchain, IsConcrete, MintLocation, RespectSuspension, SignedAccountId32AsNative,
	SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
};
use xcm_pez_simulator::helpers::derive_topic_id;

pub type AccountId = AccountId32;
pub type Balance = u128;

thread_local! {
	pub static SENT_XCM: RefCell<Vec<(Location, opaque::Xcm, XcmHash)>> = RefCell::new(Vec::new());
}
pub fn sent_xcm() -> Vec<(Location, opaque::Xcm, XcmHash)> {
	SENT_XCM.with(|q| (*q.borrow()).clone())
}
pub struct TestSendXcm;
impl SendXcm for TestSendXcm {
	type Ticket = (Location, Xcm<()>, XcmHash);
	fn validate(
		dest: &mut Option<Location>,
		msg: &mut Option<Xcm<()>>,
	) -> SendResult<(Location, Xcm<()>, XcmHash)> {
		let msg = msg.take().unwrap();
		let hash = derive_topic_id(&msg);
		let triplet = (dest.take().unwrap(), msg, hash);
		Ok((triplet, Assets::new()))
	}
	fn deliver(triplet: (Location, Xcm<()>, XcmHash)) -> Result<XcmHash, SendError> {
		let hash = triplet.2;
		SENT_XCM.with(|q| q.borrow_mut().push(triplet));
		Ok(hash)
	}
}

pub type TestXcmRouter = EnsureDecodableXcm<TestSendXcm>;

// copied from dicle constants
pub const UNITS: Balance = 1_000_000_000_000;
pub const CENTS: Balance = UNITS / 30_000;

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = ::pezsp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
}

parameter_types! {
	pub ExistentialDeposit: Balance = 1 * CENTS;
}

#[derive_impl(pezpallet_balances::config_preludes::TestDefaultConfig)]
impl pezpallet_balances::Config for Runtime {
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
}

impl shared::Config for Runtime {
	type DisabledValidators = ();
}

impl configuration::Config for Runtime {
	type WeightInfo = configuration::TestWeightInfo;
}

// aims to closely emulate the Dicle XcmConfig
parameter_types! {
	pub const KsmLocation: Location = Location::here();
	pub const DicleNetwork: NetworkId = NetworkId::Dicle;
	pub UniversalLocation: InteriorLocation = DicleNetwork::get().into();
	pub CheckAccount: (AccountId, MintLocation) = (XcmPallet::check_account(), MintLocation::Local);
}

pub type SovereignAccountOf =
	(ChildTeyrchainConvertsVia<ParaId, AccountId>, AccountId32Aliases<DicleNetwork, AccountId>);

pub type LocalCurrencyAdapter =
	FungibleAdapter<Balances, IsConcrete<KsmLocation>, SovereignAccountOf, AccountId, CheckAccount>;

pub type LocalAssetTransactor = (LocalCurrencyAdapter,);

type LocalOriginConverter = (
	SovereignSignedViaLocation<SovereignAccountOf, RuntimeOrigin>,
	ChildTeyrchainAsNative<origin::Origin, RuntimeOrigin>,
	SignedAccountId32AsNative<DicleNetwork, RuntimeOrigin>,
	ChildSystemTeyrchainAsSuperuser<ParaId, RuntimeOrigin>,
);

parameter_types! {
	pub const BaseXcmWeight: Weight = Weight::from_parts(1_000_000_000, 1024);
	pub KsmPerSecondPerByte: (AssetId, u128, u128) = (KsmLocation::get().into(), 1, 1);
}

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	// Unused/Untested
	AllowUnpaidExecutionFrom<IsChildSystemTeyrchain<ParaId>>,
);

parameter_types! {
	pub DicleForAssetHub: (AssetFilter, Location) =
		(Wild(AllOf { id: AssetId(Here.into()), fun: WildFungible }), Teyrchain(1000).into());
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 4;
}

pub type TrustedTeleporters = (xcm_builder::Case<DicleForAssetHub>,);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = TestXcmRouter;
	type XcmEventEmitter = XcmPallet;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = ();
	type IsTeleporter = TrustedTeleporters;
	type UniversalLocation = UniversalLocation;
	type Barrier = RespectSuspension<Barrier, XcmPallet>;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type Trader = FixedRateOfFungible<KsmPerSecondPerByte, ()>;
	type ResponseHandler = XcmPallet;
	type AssetTrap = XcmPallet;
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = XcmPallet;
	type SubscriptionService = XcmPallet;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = ();
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = XcmPallet;
}

/// Converts a local signed origin into an XCM location. Forms the basis for local origins
/// sending/executing XCMs.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, DicleNetwork>;

impl pezpallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type UniversalLocation = UniversalLocation;
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = TestXcmRouter;
	// Anyone can execute XCM messages locally...
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type RuntimeCall = RuntimeCall;
	type RuntimeOrigin = RuntimeOrigin;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pezpallet_xcm::CurrentXcmVersion;
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type Currency = Balances;
	type CurrencyMatcher = IsConcrete<KsmLocation>;
	type MaxLockers = pezframe_support::traits::ConstU32<8>;
	type MaxRemoteLockConsumers = pezframe_support::traits::ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type WeightInfo = pezpallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	// Aliasing is disabled: xcm_executor::Config::Aliasers is set to `Nothing`.
	type AuthorizedAliasConsideration = Disabled;
}

impl origin::Config for Runtime {}

type Block = pezframe_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime
	{
		System: pezframe_system,
		Balances: pezpallet_balances,
		ParasOrigin: origin,
		XcmPallet: pezpallet_xcm,
	}
);

pub fn dicle_like_with_balances(
	balances: Vec<(AccountId, Balance)>,
) -> pezsp_io::TestExternalities {
	let mut t = pezframe_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

	pezpallet_balances::GenesisConfig::<Runtime> { balances, ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = pezsp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
