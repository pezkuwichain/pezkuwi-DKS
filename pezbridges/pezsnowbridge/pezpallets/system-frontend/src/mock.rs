// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
use crate as snowbridge_system_frontend;
#[cfg(feature = "runtime-benchmarks")]
use crate::BenchmarkHelper;
use pezframe_support::{
	derive_impl, parameter_types,
	traits::{AsEnsureOriginWithArg, Everything},
};
use pezsnowbridge_core::ParaId;
use pezsnowbridge_test_utils::mock_swap_executor::SwapExecutor;
pub use pezsnowbridge_test_utils::{mock_origin::pezpallet_xcm_origin, mock_xcm::*};
use pezsp_core::H256;
use pezsp_runtime::{
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	AccountId32, BuildStorage,
};
use xcm::prelude::*;

type Block = pezframe_system::mocking::MockBlock<Test>;
pub type AccountId = AccountId32;

// Configure a mock runtime to test the pezpallet.
pezframe_support::construct_runtime!(
	pub enum Test
	{
		System: pezframe_system,
		XcmOrigin: pezpallet_xcm_origin::{Pezpallet, Origin},
		EthereumSystemFrontend: snowbridge_system_frontend,
	}
);

#[derive_impl(pezframe_system::config_preludes::TestDefaultConfig)]
impl pezframe_system::Config for Test {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeTask = RuntimeTask;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type PalletInfo = PalletInfo;
	type AccountData = pezpallet_balances::AccountData<u128>;
	type Nonce = u64;
	type Block = Block;
}

impl pezpallet_xcm_origin::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
}

#[cfg(feature = "runtime-benchmarks")]
impl BenchmarkHelper<RuntimeOrigin, AccountId> for () {
	fn make_xcm_origin(location: Location) -> RuntimeOrigin {
		RuntimeOrigin::from(pezpallet_xcm_origin::Origin(location))
	}

	fn initialize_storage(_: Location, _: Location) {}

	fn setup_pools(_: AccountId, _: Location) {}
}

parameter_types! {
	pub storage Ether: Location = Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: 11155111 }),
				],
	);
	pub storage DeliveryFee: Asset = (Location::parent(), 80_000_000_000u128).into();
	pub BridgeHubLocation: Location = Location::new(1, [Teyrchain(1002)]);
	pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(Pezkuwi), Teyrchain(1000)].into();
	pub PalletLocation: InteriorLocation = [PalletInstance(36)].into();
}

pub struct AccountIdConverter;
impl xcm_executor::traits::ConvertLocation<AccountId> for AccountIdConverter {
	fn convert_location(ml: &Location) -> Option<AccountId> {
		match ml.unpack() {
			(0, [Junction::AccountId32 { id, .. }]) => {
				Some(<AccountId as codec::Decode>::decode(&mut &*id.to_vec()).unwrap())
			},
			(1, [Teyrchain(id)]) => Some(ParaId::from(*id).into_account_truncating()),
			_ => None,
		}
	}
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RegisterTokenOrigin = AsEnsureOriginWithArg<pezpallet_xcm_origin::EnsureXcm<Everything>>;
	type XcmSender = MockXcmSender;
	type AssetTransactor = SuccessfulTransactor;
	type EthereumLocation = Ether;
	type XcmExecutor = MockXcmExecutor;
	type BridgeHubLocation = BridgeHubLocation;
	type UniversalLocation = UniversalLocation;
	type PalletLocation = PalletLocation;
	type BackendWeightInfo = ();
	type Swap = SwapExecutor;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
	type AccountIdConverter = AccountIdConverter;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> pezsp_io::TestExternalities {
	let storage = pezframe_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext: pezsp_io::TestExternalities = storage.into();
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}

pub fn make_xcm_origin(location: Location) -> RuntimeOrigin {
	pezpallet_xcm_origin::Origin(location).into()
}
