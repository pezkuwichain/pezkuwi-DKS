// This file is part of Bizinikiwi.

// Copyright (C) Parity Technologies (UK) Ltd. and Dijital Kurdistan Tech Institute
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

//! New governance configurations for the Dicle runtime.

use super::*;
use crate::xcm_config::Collectives;
use pezframe_support::{
	parameter_types,
	traits::{
		fungible::HoldConsideration, tokens::UnityOrOuterConversion, EitherOf, EitherOfDiverse,
		FromContains, LinearStoragePrice,
	},
};
use pezframe_system::EnsureRootWithSuccess;
use pezkuwi_runtime_common::impls::{
	ContainsParts, LocatableAssetConverter, VersionedLocatableAsset, VersionedLocationConverter,
};
use pezpallet_xcm::{EnsureXcm, IsVoiceOfBody};
use pezsp_runtime::{traits::IdentityLookup, Percent};
use xcm::latest::{
	prelude::{InteriorLocation, PalletInstance},
	BodyId,
};

mod origins;
pub use origins::{
	pezpallet_custom_origins, AuctionAdmin, FellowshipAdmin, GeneralAdmin, LeaseAdmin,
	ReferendumCanceller, ReferendumKiller, Spender, StakingAdmin, Treasurer, WhitelistedCaller,
};
mod tracks;
pub use tracks::TracksInfo;
use xcm_builder::PayOverXcm;

parameter_types! {
	pub const VoteLockingPeriod: BlockNumber = 7 * DAYS;
}

impl pezpallet_conviction_voting::Config for Runtime {
	type WeightInfo = weights::pezpallet_conviction_voting::WeightInfo<Self>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type VoteLockingPeriod = VoteLockingPeriod;
	type MaxVotes = ConstU32<512>;
	type MaxTurnout =
		pezframe_support::traits::tokens::currency::ActiveIssuanceOf<Balances, Self::AccountId>;
	type Polls = Referenda;
	type BlockNumberProvider = RelayChainBlockNumberProvider;
	type VotingHooks = ();
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 1 * 3 * CENTS;
	pub const UndecidingTimeout: BlockNumber = 14 * DAYS;
}

impl origins::pezpallet_custom_origins::Config for Runtime {}

parameter_types! {
	// Fellows pluralistic body.
	pub const FellowsBodyId: BodyId = BodyId::Technical;
}

impl pezpallet_whitelist::Config for Runtime {
	type WeightInfo = weights::pezpallet_whitelist::WeightInfo<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WhitelistOrigin = EitherOfDiverse<
		EnsureRoot<Self::AccountId>,
		EnsureXcm<IsVoiceOfBody<Collectives, FellowsBodyId>>,
	>;
	type DispatchWhitelistedOrigin = EitherOf<EnsureRoot<Self::AccountId>, WhitelistedCaller>;
	type Preimages = Preimage;
}

impl pezpallet_referenda::Config for Runtime {
	type WeightInfo = weights::pezpallet_referenda::WeightInfo<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin = pezframe_system::EnsureSigned<AccountId>;
	type CancelOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumCanceller>;
	type KillOrigin = EitherOf<EnsureRoot<AccountId>, ReferendumKiller>;
	type Slash = Treasury;
	type Votes = pezpallet_conviction_voting::VotesOf<Runtime>;
	type Tally = pezpallet_conviction_voting::TallyOf<Runtime>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = RelayChainBlockNumberProvider;
}

parameter_types! {
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(2);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	// The asset's interior location for the paying account. This is the Treasury
	// pezpallet instance (which sits at index 37).
	pub TreasuryInteriorLocation: InteriorLocation = PalletInstance(37).into();

	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 100 * CENTS;
	pub const DataDepositPerByte: Balance = 1 * CENTS;
	pub const MaxApprovals: u32 = 100;
	pub const MaxAuthorities: u32 = 100_000;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxBalance: Balance = Balance::max_value();
}

pub type TreasurySpender = EitherOf<EnsureRootWithSuccess<AccountId, MaxBalance>, Spender>;

impl pezpallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type MaxApprovals = MaxApprovals;
	type WeightInfo = weights::pezpallet_treasury::WeightInfo<Runtime>;
	type SpendFunds = ();
	type SpendOrigin = TreasurySpender;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayOverXcm<
		TreasuryInteriorLocation,
		crate::xcm_config::XcmRouter,
		crate::PezkuwiXcm,
		ConstU32<{ 6 * HOURS }>,
		Self::Beneficiary,
		Self::AssetKind,
		LocatableAssetConverter,
		VersionedLocationConverter,
	>;
	type BalanceConverter = UnityOrOuterConversion<
		ContainsParts<
			FromContains<
				xcm_builder::IsChildSystemTeyrchain<ParaId>,
				xcm_builder::IsParentsOnly<ConstU8<1>>,
			>,
		>,
		AssetRate,
	>;
	type PayoutPeriod = PayoutSpendPeriod;
	type BlockNumberProvider = RelayChainBlockNumberProvider;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pezkuwi_runtime_common::impls::benchmarks::TreasuryArguments;
}
impl pezpallet_asset_rate::Config for Runtime {
	type WeightInfo = weights::pezpallet_asset_rate::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type CreateOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type Currency = Balances;
	type AssetKind = <Runtime as pezpallet_treasury::Config>::AssetKind;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pezkuwi_runtime_common::impls::benchmarks::AssetRateArguments;
}

parameter_types! {
	pub MaximumSchedulerWeight: pezframe_support::weights::Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pezpallet_scheduler::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = weights::pezpallet_scheduler::WeightInfo<Runtime>;
	type OriginPrivilegeCmp = pezframe_support::traits::EqualPrivilegeOnly;
	type Preimages = Preimage;
	type BlockNumberProvider = RelayChainBlockNumberProvider;
}

parameter_types! {
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pezpallet_preimage::HoldReason::Preimage);
}

impl pezpallet_preimage::Config for Runtime {
	type WeightInfo = weights::pezpallet_preimage::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
}
