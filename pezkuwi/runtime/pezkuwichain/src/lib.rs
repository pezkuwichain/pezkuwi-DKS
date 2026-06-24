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

//! The Pezkuwichain runtime for v1 teyrchains.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit.
#![recursion_limit = "512"]

#[cfg(all(any(target_arch = "riscv32", target_arch = "riscv64"), target_feature = "e"))]
// Allocate 2 MiB stack.
//
// TODO: A workaround. Invoke polkavm_derive::min_stack_size!() instead
// later on.
::core::arch::global_asm!(
	".pushsection .polkavm_min_stack_size,\"R\",@note\n",
	".4byte 2097152",
	".popsection\n",
);

extern crate alloc;

use alloc::{
	collections::{btree_map::BTreeMap, vec_deque::VecDeque},
	vec,
	vec::Vec,
};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::cmp::Ordering;
use pezframe_support::dynamic_params::{dynamic_pallet_params, dynamic_params};
use pezkuwi_primitives::{
	async_backing::Constraints, slashing, AccountId, AccountIndex, ApprovalVotingParams, Balance,
	BlockNumber, CandidateEvent, CandidateHash,
	CommittedCandidateReceiptV2 as CommittedCandidateReceipt, CoreIndex, CoreState, DisputeState,
	ExecutorParams, GroupRotationInfo, Hash, Id as ParaId, InboundDownwardMessage,
	InboundHrmpMessage, Moment, NodeFeatures, Nonce, OccupiedCoreAssumption,
	PersistedValidationData, ScrapedOnChainVotes, SessionInfo, Signature, ValidationCode,
	ValidationCodeHash, ValidatorId, ValidatorIndex, TEYRCHAIN_KEY_TYPE_ID,
};
use pezkuwi_runtime_common::{
	assigned_slots, auctions, claims, crowdloan, impl_runtime_weights,
	impls::{
		LocatableAssetConverter, ToAuthor, VersionedLocatableAsset, VersionedLocationConverter,
	},
	paras_registrar, paras_sudo_wrapper, prod_or_fast, slots,
	traits::{Leaser, OnSwap},
	BlockHashCount, BlockLength, SlowAdjustingFeeUpdate,
};
use pezkuwi_runtime_teyrchains::{
	assigner_coretime as teyrchains_assigner_coretime, configuration as teyrchains_configuration,
	configuration::ActiveConfigHrmpChannelSizeAndCapacityRatio,
	coretime, disputes as teyrchains_disputes,
	disputes::slashing as teyrchains_slashing,
	dmp as teyrchains_dmp, hrmp as teyrchains_hrmp, inclusion as teyrchains_inclusion,
	inclusion::{AggregateMessageOrigin, UmpQueueId},
	initializer as teyrchains_initializer, on_demand as teyrchains_on_demand,
	origin as teyrchains_origin, paras as teyrchains_paras,
	paras_inherent as teyrchains_paras_inherent, reward_points as teyrchains_reward_points,
	runtime_api_impl::{
		v13 as teyrchains_runtime_api_impl, vstaging as teyrchains_staging_runtime_api_impl,
	},
	scheduler as teyrchains_scheduler, session_info as teyrchains_session_info,
	shared as teyrchains_shared,
};
use pezkuwichain_runtime_constants::system_teyrchain::{
	coretime::TIMESLICE_PERIOD, ASSET_HUB_ID, BROKER_ID,
};
use pezpallet_balances::WeightInfo;
use pezsp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use pezsp_consensus_beefy::{
	ecdsa_crypto::{AuthorityId as BeefyId, Signature as BeefySignature},
	mmr::{BeefyDataProvider, MmrLeafVersion},
};
use pezsp_genesis_builder::PresetId;
use scale_info::TypeInfo;

use pezframe_support::{
	construct_runtime, derive_impl,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{
		fungible::HoldConsideration, EitherOf, EitherOfDiverse, EnsureOriginWithArg,
		InstanceFilter, KeyOwnerProofSystem, LinearStoragePrice, PrivilegeCmp, ProcessMessage,
		ProcessMessageError, WithdrawReasons,
	},
	weights::{ConstantMultiplier, WeightMeter},
	PalletId,
};
use pezframe_system::EnsureRoot;
use pezpallet_grandpa::{fg_primitives, AuthorityId as GrandpaId};
use pezpallet_session::historical as session_historical;
use pezpallet_staking_async_ah_client as ah_client;
use pezpallet_staking_async_rc_client as rc_client;
use pezpallet_transaction_payment::{FeeDetails, FungibleAdapter, RuntimeDispatchInfo};
use pezsp_core::{ConstUint, Get, OpaqueMetadata, H256};
use pezsp_runtime::{
	generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, BlakeTwo256, Block as BlockT, ConstU32, ConvertInto, IdentityLookup,
		Keccak256, OpaqueKeys, SaturatedConversion, Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU128, KeyTypeId, Perbill, Percent, Permill, RuntimeDebug,
};
use pezsp_staking::SessionIndex;
#[cfg(any(feature = "std", test))]
use pezsp_version::NativeVersion;
use pezsp_version::RuntimeVersion;
use xcm::{
	latest::prelude::*, Version as XcmVersion, VersionedAsset, VersionedAssetId, VersionedAssets,
	VersionedLocation, VersionedXcm,
};
use xcm_builder::PayOverXcm;

pub use pezframe_system::Call as SystemCall;
pub use pezpallet_balances::Call as BalancesCall;

/// Constant values used within the runtime.
use pezkuwichain_runtime_constants::{currency::*, fee::*, time::*};

// Weights used in the runtime.
mod weights;

// XCM configurations.
pub mod xcm_config;

// Governance and configurations.
pub mod governance;
use governance::{
	pezpallet_custom_origins, AuctionAdmin, Fellows, LeaseAdmin, Treasurer, TreasurySpender,
};
use xcm_config::XcmConfig;
use xcm_runtime_pezapis::{
	dry_run::{CallDryRunEffects, Error as XcmDryRunApiError, XcmDryRunEffects},
	fees::Error as XcmPaymentApiError,
};

#[cfg(test)]
mod tests;

mod genesis_config_presets;
mod validator_manager;

impl_runtime_weights!(pezkuwichain_runtime_constants);

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Provides the `WASM_BINARY` build with `fast-runtime` feature enabled.
///
/// This is for example useful for local test chains.
#[cfg(feature = "std")]
pub mod fast_runtime_binary {
	include!(concat!(env!("OUT_DIR"), "/fast_runtime_binary.rs"));
}

/// Runtime version (Pezkuwichain).
#[pezsp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("pezkuwichain"),
	impl_name: alloc::borrow::Cow::Borrowed("pezkuwichain"),
	authoring_version: 0,
	spec_version: 1_020_007,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 26,
	system_version: 1,
};

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: pezsp_consensus_babe::BabeEpochConfiguration =
	pezsp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: pezsp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(pezframe_system::config_preludes::RelayChainDefaultConfig)]
impl pezframe_system::Config for Runtime {
	type BaseCallFilter = pezframe_support::traits::Everything;
	type BlockWeights = BlockWeights;
	type BlockLength = BlockLength;
	type DbWeight = RocksDbWeight;
	type Nonce = Nonce;
	type Hash = Hash;
	type AccountId = AccountId;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type AccountData = pezpallet_balances::AccountData<Balance>;
	type SystemWeightInfo = weights::pezframe_system::WeightInfo<Runtime>;
	type ExtensionsWeightInfo = weights::pezframe_system_extensions::WeightInfo<Runtime>;
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = pezframe_support::traits::ConstU32<16>;
	type MultiBlockMigrator = MultiBlockMigrations;
	type SingleBlockMigrations = Migrations;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

/// Used the compare the privilege of an origin inside the scheduler.
pub struct OriginPrivilegeCmp;

impl PrivilegeCmp<OriginCaller> for OriginPrivilegeCmp {
	fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<Ordering> {
		if left == right {
			return Some(Ordering::Equal);
		}

		match (left, right) {
			// Root is greater than anything.
			(OriginCaller::system(pezframe_system::RawOrigin::Root), _) => Some(Ordering::Greater),
			// For every other origin we don't care, as they are not used for `ScheduleOrigin`.
			_ => None,
		}
	}
}

/// Dynamic params that can be adjusted at runtime.
#[dynamic_params(RuntimeParameters, pezpallet_parameters::Parameters::<Runtime>)]
pub mod dynamic_params {
	use super::*;

	#[dynamic_pallet_params]
	#[codec(index = 0)]
	pub mod preimage {
		use super::*;

		#[codec(index = 0)]
		pub static BaseDeposit: Balance = deposit(2, 64);

		#[codec(index = 1)]
		pub static ByteDeposit: Balance = deposit(0, 1);
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
	fn default() -> Self {
		RuntimeParameters::Preimage(dynamic_params::preimage::Parameters::BaseDeposit(
			dynamic_params::preimage::BaseDeposit,
			Some(1u32.into()),
		))
	}
}

/// Defines what origin can modify which dynamic parameters.
pub struct DynamicParameterOrigin;
impl EnsureOriginWithArg<RuntimeOrigin, RuntimeParametersKey> for DynamicParameterOrigin {
	type Success = ();

	fn try_origin(
		origin: RuntimeOrigin,
		key: &RuntimeParametersKey,
	) -> Result<Self::Success, RuntimeOrigin> {
		use crate::RuntimeParametersKey::*;

		match key {
			Preimage(_) => pezframe_system::ensure_root(origin.clone()),
		}
		.map_err(|_| origin)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_key: &RuntimeParametersKey) -> Result<RuntimeOrigin, ()> {
		// Provide the origin for the parameter returned by `Default`:
		Ok(RuntimeOrigin::root())
	}
}

impl pezpallet_scheduler::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	// The goal of having ScheduleOrigin include AuctionAdmin is to allow the auctions track of
	// OpenGov to schedule periodic auctions.
	type ScheduleOrigin = EitherOf<EnsureRoot<AccountId>, AuctionAdmin>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = weights::pezpallet_scheduler::WeightInfo<Runtime>;
	type OriginPrivilegeCmp = OriginPrivilegeCmp;
	type Preimages = Preimage;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

parameter_types! {
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
		LinearStoragePrice<
			dynamic_params::preimage::BaseDeposit,
			dynamic_params::preimage::ByteDeposit,
			Balance,
		>,
	>;
}

parameter_types! {
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	pub ReportLongevity: u64 = EpochDurationInBlocks::get() as u64 * 10;
}

impl pezpallet_babe::Config for Runtime {
	type EpochDuration = EpochDurationInBlocks;
	type ExpectedBlockTime = ExpectedBlockTime;
	// session module is the trigger
	type EpochChangeTrigger = pezpallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = ConstU32<0>;
	type KeyOwnerProof = pezsp_session::MembershipProof;
	type EquivocationReportSystem =
		pezpallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	pub const IndexDeposit: Balance = 100 * CENTS;
}

impl pezpallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezpallet_indices::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pezpallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = weights::pezpallet_balances_balances::WeightInfo<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	/// This value increases the priority of `Operational` transactions by adding
	/// a "virtual tip" that's equal to the `OperationalFeeMultiplier * final_fee`.
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pezpallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ToAuthor<Runtime>>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type WeightInfo = weights::pezpallet_transaction_payment::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}
impl pezpallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pezpallet_timestamp::WeightInfo<Runtime>;
}

impl pezpallet_authorship::Config for Runtime {
	type FindAuthor = pezpallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = StakingAhClient;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub para_validator: Initializer,
		pub para_assignment: ParaSessionInfo,
		pub authority_discovery: AuthorityDiscovery,
		pub beefy: Beefy,
	}
}

/// Special `ValidatorIdOf` implementation that is just returning the input as result.
pub struct ValidatorIdOf;
impl pezsp_runtime::traits::Convert<AccountId, Option<AccountId>> for ValidatorIdOf {
	fn convert(a: AccountId) -> Option<AccountId> {
		Some(a)
	}
}

impl pezpallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ValidatorIdOf;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pezpallet_session::historical::NoteHistoricalRoot<Self, StakingAhClient>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisablingStrategy = ();
	type WeightInfo = weights::pezpallet_session::WeightInfo<Runtime>;
	type Currency = Balances;
	type KeyDeposit = ();
}

/// Returns a default empty exposure for historical session tracking.
/// Exposure data is now on Asset Hub — RC returns default to keep validators in authority set.
pub struct ExposureOfOrDefault;
impl pezsp_runtime::traits::Convert<AccountId, Option<pezsp_staking::Exposure<AccountId, Balance>>>
	for ExposureOfOrDefault
{
	fn convert(_validator: AccountId) -> Option<pezsp_staking::Exposure<AccountId, Balance>> {
		Some(Default::default())
	}
}

/// No-op fallback for StakingAhClient. Required at compile time but never called
/// since Mode is Active — all session/offence/reward logic goes through StakingAhClient.
pub struct NoopFallback;

impl pezpallet_session::SessionManager<AccountId> for NoopFallback {
	fn new_session(_: SessionIndex) -> Option<Vec<AccountId>> {
		None
	}
	fn start_session(_: SessionIndex) {}
	fn end_session(_: SessionIndex) {}
}

impl
	pezsp_staking::offence::OnOffenceHandler<
		AccountId,
		(AccountId, pezsp_staking::Exposure<AccountId, Balance>),
		Weight,
	> for NoopFallback
{
	fn on_offence(
		_offenders: &[pezsp_staking::offence::OffenceDetails<
			AccountId,
			(AccountId, pezsp_staking::Exposure<AccountId, Balance>),
		>],
		_slash_fraction: &[Perbill],
		_session: SessionIndex,
	) -> Weight {
		Weight::zero()
	}
}

impl pezframe_support::traits::RewardsReporter<AccountId> for NoopFallback {
	fn reward_by_ids(_: impl IntoIterator<Item = (AccountId, u32)>) {}
}

impl pezpallet_authorship::EventHandler<AccountId, BlockNumber> for NoopFallback {
	fn note_author(_: AccountId) {}
}

impl pezpallet_session::historical::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type FullIdentification = pezsp_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = ExposureOfOrDefault;
}

// =====================================================
// STAKING CONFIGURATION (async — managed by StakingAhClient)
// =====================================================

parameter_types! {
	pub const MaxActiveValidators: u32 = 1000;
}

// =====================================================
// STAKING AH CLIENT CONFIGURATION (XCM Session Reports)
// =====================================================

#[derive(Encode, Decode)]
enum AssetHubRuntimePallets<AccountId> {
	// Audit: `StakingRcClient` in asset-hub-pezkuwichain (pallet index 89)
	#[codec(index = 89)]
	RcClient(RcClientCalls<AccountId>),
}

#[derive(Encode, Decode)]
enum RcClientCalls<AccountId> {
	#[codec(index = 0)]
	RelaySessionReport(rc_client::SessionReport<AccountId>),
	#[codec(index = 1)]
	RelayNewOffencePaged(Vec<(SessionIndex, rc_client::Offence<AccountId>)>),
}

pub struct AssetHubLocation;
impl Get<Location> for AssetHubLocation {
	fn get() -> Location {
		Location::new(0, [Junction::Teyrchain(ASSET_HUB_ID)])
	}
}

pub struct EnsureAssetHub;
impl pezframe_support::traits::EnsureOrigin<RuntimeOrigin> for EnsureAssetHub {
	type Success = ();
	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		match <RuntimeOrigin as Into<Result<teyrchains_origin::Origin, RuntimeOrigin>>>::into(
			o.clone(),
		) {
			Ok(teyrchains_origin::Origin::Teyrchain(id)) if id == ASSET_HUB_ID.into() => Ok(()),
			_ => Err(o),
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

pub struct SessionReportToXcm;
impl pezsp_runtime::traits::Convert<rc_client::SessionReport<AccountId>, Xcm<()>>
	for SessionReportToXcm
{
	fn convert(a: rc_client::SessionReport<AccountId>) -> Xcm<()> {
		Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			Instruction::Transact {
				origin_kind: OriginKind::Superuser,
				fallback_max_weight: None,
				call: AssetHubRuntimePallets::RcClient(RcClientCalls::RelaySessionReport(a))
					.encode()
					.into(),
			},
		])
	}
}

pub struct QueuedOffenceToXcm;
impl pezsp_runtime::traits::Convert<Vec<ah_client::QueuedOffenceOf<Runtime>>, Xcm<()>>
	for QueuedOffenceToXcm
{
	fn convert(offences: Vec<ah_client::QueuedOffenceOf<Runtime>>) -> Xcm<()> {
		Xcm(vec![
			Instruction::UnpaidExecution {
				weight_limit: WeightLimit::Unlimited,
				check_origin: None,
			},
			Instruction::Transact {
				origin_kind: OriginKind::Superuser,
				fallback_max_weight: None,
				call: AssetHubRuntimePallets::RcClient(RcClientCalls::RelayNewOffencePaged(
					offences,
				))
				.encode()
				.into(),
			},
		])
	}
}

pub struct StakingXcmToAssetHub;
impl ah_client::SendToAssetHub for StakingXcmToAssetHub {
	type AccountId = AccountId;

	fn relay_session_report(
		session_report: rc_client::SessionReport<Self::AccountId>,
	) -> Result<(), ()> {
		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			AssetHubLocation,
			rc_client::SessionReport<AccountId>,
			SessionReportToXcm,
		>::send(session_report)
	}

	fn relay_new_offence_paged(
		offences: Vec<ah_client::QueuedOffenceOf<Runtime>>,
	) -> Result<(), ()> {
		rc_client::XCMSender::<
			xcm_config::XcmRouter,
			AssetHubLocation,
			Vec<ah_client::QueuedOffenceOf<Runtime>>,
			QueuedOffenceToXcm,
		>::send(offences)
	}
}

impl ah_client::Config for Runtime {
	type CurrencyBalance = Balance;
	type AssetHubOrigin =
		pezframe_support::traits::EitherOfDiverse<EnsureRoot<AccountId>, EnsureAssetHub>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type SessionInterface = Self;
	type SendToAssetHub = StakingXcmToAssetHub;
	type MinimumValidatorSetSize = ConstU32<1>;
	type UnixTime = Timestamp;
	type PointsPerBlock = ConstU32<20>;
	type MaxOffenceBatchSize = ConstU32<50>;
	type Fallback = NoopFallback;
	type MaximumValidatorsWithPoints = ConstU32<{ MaxActiveValidators::get() * 4 }>;
	type MaxSessionReportRetries = ConstU32<64>;
}

// =====================================================
// VALIDATOR POOL CONFIGURATION (TNPoS Shadow Mode)
// =====================================================

/// Stub Trust Score Provider - returns default trust for shadow mode
/// Will be replaced with XCM cache from People Parachain in Phase 5
pub struct StubTrustProvider;
impl pezpallet_validator_pool::TrustScoreProvider<AccountId> for StubTrustProvider {
	fn trust_score_of(_who: &AccountId) -> u128 {
		1000 // Default trust score for shadow mode
	}
}

/// Stub Tiki Score Provider - returns default tiki for shadow mode
/// Will be replaced with XCM cache from People Parachain in Phase 5
pub struct StubTikiProvider;
impl pezpallet_validator_pool::TikiScoreProvider<AccountId> for StubTikiProvider {
	fn get_tiki_score(_who: &AccountId) -> u32 {
		0 // No tiki in shadow mode
	}
}

/// Stub Referral Provider - returns default referral count for shadow mode
/// Will be replaced with XCM cache from People Parachain in Phase 5
pub struct StubReferralProvider;
impl pezpallet_validator_pool::types::ReferralProvider<AccountId> for StubReferralProvider {
	fn get_referral_count(_who: &AccountId) -> u32 {
		0 // No referrals in shadow mode
	}
}

/// Stub Perwerde Provider - returns default perwerde score for shadow mode
/// Will be replaced with XCM cache from People Parachain in Phase 5
pub struct StubPerwerdeProvider;
impl pezpallet_validator_pool::types::PerwerdeProvider<AccountId> for StubPerwerdeProvider {
	fn get_perwerde_score(_who: &AccountId) -> u32 {
		0 // No perwerde in shadow mode
	}
}

parameter_types! {
	pub const ValidatorPoolMaxValidators: u32 = 21; // Target: 10 stake + 6 parliamentary + 5 merit
	pub const ValidatorPoolMaxPoolSize: u32 = 1000;
	pub const ValidatorPoolMinStakeAmount: u128 = 100 * UNITS;
}

impl pezpallet_validator_pool::Config for Runtime {
	type WeightInfo = pezpallet_validator_pool::weights::BizinikiwiWeight<Runtime>;
	type Randomness = pezpallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type TrustSource = StubTrustProvider;
	type TikiSource = StubTikiProvider;
	type ReferralSource = StubReferralProvider;
	type PerwerdeSource = StubPerwerdeProvider;
	type PoolManagerOrigin = EnsureRoot<AccountId>;
	type MaxValidators = ValidatorPoolMaxValidators;
	type MaxPoolSize = ValidatorPoolMaxPoolSize;
	type MinStakeAmount = ValidatorPoolMinStakeAmount;
}

// =====================================================
// COUNCIL CONFIGURATION
// =====================================================

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
	pub MaxCollectivesProposalWeight: pezframe_support::weights::Weight = Perbill::from_percent(50) * BlockWeights::get().max_block;
}

pub type CouncilCollective = pezpallet_collective::Instance1;
impl pezpallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pezpallet_collective::PrimeDefaultVote;
	type WeightInfo = pezpallet_collective::weights::BizinikiwiWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
	type DisapproveOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Consideration = ();
}

parameter_types! {
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(2);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	// The asset's interior location for the paying account. This is the Treasury
	// pezpallet instance (which sits at index 18).
	pub TreasuryInteriorLocation: InteriorLocation = PalletInstance(18).into();

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
		crate::XcmPallet,
		ConstU32<{ 6 * HOURS }>,
		Self::Beneficiary,
		Self::AssetKind,
		LocatableAssetConverter,
		VersionedLocationConverter,
	>;
	type BalanceConverter = pezframe_support::traits::tokens::UnityAssetBalanceConversion;
	type PayoutPeriod = PayoutSpendPeriod;
	type BlockNumberProvider = System;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = pezkuwi_runtime_common::impls::benchmarks::TreasuryArguments;
}

impl pezpallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pezpallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = ();
}

impl pezpallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	// BondingDuration(2) * SessionsPerEra(6) — matches AH staking config
	pub const MaxSetIdSessionEntries: u32 = 2 * 6;
}

impl pezpallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type KeyOwnerProof = pezsp_session::MembershipProof;
	type EquivocationReportSystem =
		pezpallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl pezframe_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> pezframe_system::offchain::CreateTransactionBase<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type Extrinsic = UncheckedExtrinsic;
	type RuntimeCall = RuntimeCall;
}

/// Submits a transaction with the node's public and signature type. Adheres to the signed
/// extension format of the chain.
impl<LocalCall> pezframe_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_signed_transaction<
		C: pezframe_system::offchain::AppCrypto<Self::Public, Self::Signature>,
	>(
		call: RuntimeCall,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: <Runtime as pezframe_system::Config>::Nonce,
	) -> Option<UncheckedExtrinsic> {
		use pezsp_runtime::traits::StaticLookup;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;

		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let tip = 0;
		let tx_ext: TxExtension = (
			pezframe_system::AuthorizeCall::<Runtime>::new(),
			pezframe_system::CheckNonZeroSender::<Runtime>::new(),
			pezframe_system::CheckSpecVersion::<Runtime>::new(),
			pezframe_system::CheckTxVersion::<Runtime>::new(),
			pezframe_system::CheckGenesis::<Runtime>::new(),
			pezframe_system::CheckMortality::<Runtime>::from(generic::Era::mortal(
				period,
				current_block,
			)),
			pezframe_system::CheckNonce::<Runtime>::from(nonce),
			pezframe_system::CheckWeight::<Runtime>::new(),
			pezpallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			pezframe_metadata_hash_extension::CheckMetadataHash::new(true),
			pezframe_system::WeightReclaim::<Runtime>::new(),
		)
			.into();
		let raw_payload = SignedPayload::new(call, tx_ext)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, tx_ext, _) = raw_payload.deconstruct();
		let address = <Runtime as pezframe_system::Config>::Lookup::unlookup(account);
		let transaction = UncheckedExtrinsic::new_signed(call, address, signature, tx_ext);
		Some(transaction)
	}
}

impl<LocalCall> pezframe_system::offchain::CreateTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type Extension = TxExtension;

	fn create_transaction(call: RuntimeCall, tx_ext: Self::Extension) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_transaction(call, tx_ext)
	}
}

impl<LocalCall> pezframe_system::offchain::CreateBare<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_bare(call)
	}
}

impl<LocalCall> pezframe_system::offchain::CreateAuthorizedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_extension() -> Self::Extension {
		(
			pezframe_system::AuthorizeCall::<Runtime>::new(),
			pezframe_system::CheckNonZeroSender::<Runtime>::new(),
			pezframe_system::CheckSpecVersion::<Runtime>::new(),
			pezframe_system::CheckTxVersion::<Runtime>::new(),
			pezframe_system::CheckGenesis::<Runtime>::new(),
			pezframe_system::CheckMortality::<Runtime>::from(generic::Era::Immortal),
			pezframe_system::CheckNonce::<Runtime>::from(0),
			pezframe_system::CheckWeight::<Runtime>::new(),
			pezpallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
			pezframe_metadata_hash_extension::CheckMetadataHash::new(false),
			pezframe_system::WeightReclaim::<Runtime>::new(),
		)
	}
}

parameter_types! {
	pub Prefix: &'static [u8] = b"Pay HEZ to the Pezkuwichain account:";
}

impl claims::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type Prefix = Prefix;
	type MoveClaimOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pezkuwi_runtime_common_claims::WeightInfo<Runtime>;
}

impl pezpallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pezpallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u32 = 100;
}

impl pezpallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = weights::pezpallet_multisig::WeightInfo<Runtime>;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * CENTS;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pezpallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = weights::pezpallet_vesting::WeightInfo<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	MaxEncodedLen,
	TypeInfo,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
	CancelProxy,
	Auction,
	OnDemandOrdering,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => matches!(
				c,
				RuntimeCall::System(..) |
				RuntimeCall::Babe(..) |
				RuntimeCall::Timestamp(..) |
				RuntimeCall::Indices(pezpallet_indices::Call::claim {..}) |
				RuntimeCall::Indices(pezpallet_indices::Call::free {..}) |
				RuntimeCall::Indices(pezpallet_indices::Call::freeze {..}) |
				// Specifically omitting Indices `transfer`, `force_transfer`
				// Specifically omitting the entire Balances pezpallet
				RuntimeCall::Session(..) |
				RuntimeCall::Grandpa(..) |
				RuntimeCall::Treasury(..) |
				RuntimeCall::ConvictionVoting(..) |
				RuntimeCall::Referenda(..) |
				RuntimeCall::Whitelist(..) |
				RuntimeCall::Claims(..) |
				RuntimeCall::Utility(..) |
				RuntimeCall::Vesting(pezpallet_vesting::Call::vest {..}) |
				RuntimeCall::Vesting(pezpallet_vesting::Call::vest_other {..}) |
				// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
				RuntimeCall::Scheduler(..) |
				RuntimeCall::Proxy(..) |
				RuntimeCall::Multisig(..) |
				RuntimeCall::Registrar(paras_registrar::Call::register {..}) |
				RuntimeCall::Registrar(paras_registrar::Call::deregister {..}) |
				// Specifically omitting Registrar `swap`
				RuntimeCall::Registrar(paras_registrar::Call::reserve {..}) |
				RuntimeCall::Crowdloan(..) |
				RuntimeCall::Slots(..) |
				RuntimeCall::Auctions(..) // Specifically omitting the entire XCM Pezpallet
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Utility(..) |
					// OpenGov calls
					RuntimeCall::ConvictionVoting(..) |
					RuntimeCall::Referenda(..) |
					RuntimeCall::Whitelist(..)
			),
			ProxyType::CancelProxy => {
				matches!(c, RuntimeCall::Proxy(pezpallet_proxy::Call::reject_announcement { .. }))
			},
			ProxyType::Auction => matches!(
				c,
				RuntimeCall::Auctions { .. }
					| RuntimeCall::Crowdloan { .. }
					| RuntimeCall::Registrar { .. }
					| RuntimeCall::Multisig(..)
					| RuntimeCall::Slots { .. }
			),
			ProxyType::OnDemandOrdering => matches!(c, RuntimeCall::OnDemandAssignmentProvider(..)),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

impl pezpallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = MaxProxies;
	type WeightInfo = weights::pezpallet_proxy::WeightInfo<Runtime>;
	type MaxPending = MaxPending;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type BlockNumberProvider = pezframe_system::Pezpallet<Runtime>;
}

impl teyrchains_origin::Config for Runtime {}

impl teyrchains_configuration::Config for Runtime {
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_configuration::WeightInfo<Runtime>;
}

impl teyrchains_shared::Config for Runtime {
	type DisabledValidators = Session;
}

impl teyrchains_session_info::Config for Runtime {
	type ValidatorSet = Historical;
}

impl teyrchains_inclusion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DisputesHandler = ParasDisputes;
	type RewardValidators =
		teyrchains_reward_points::RewardValidatorsWithEraPoints<Runtime, StakingAhClient>;
	type MessageQueue = MessageQueue;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_inclusion::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ParasUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl teyrchains_paras::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_paras::WeightInfo<Runtime>;
	type UnsignedPriority = ParasUnsignedPriority;
	type QueueFootprinter = ParaInclusion;
	type NextSessionRotation = Babe;
	type OnNewHead = Registrar;
	type AssignCoretime = CoretimeAssignmentProvider;
	type Fungible = Balances;
	// Per day the cooldown is removed earlier, it should cost 1000.
	type CooldownRemovalMultiplier = ConstUint<{ 1000 * UNITS / DAYS as u128 }>;
	type AuthorizeCurrentCodeOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	/// Amount of weight that can be spent per block to service messages.
	///
	/// # WARNING
	///
	/// This is not a good value for para-chains since the `Scheduler` already uses up to 80% block weight.
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(20) * BlockWeights::get().max_block;
	pub const MessageQueueHeapSize: u32 = 32 * 1024;
	pub const MessageQueueMaxStale: u32 = 96;
}

/// Message processor to handle any messages that were enqueued into the `MessageQueue` pezpallet.
pub struct MessageProcessor;
impl ProcessMessage for MessageProcessor {
	type Origin = AggregateMessageOrigin;

	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		meter: &mut WeightMeter,
		id: &mut [u8; 32],
	) -> Result<bool, ProcessMessageError> {
		let para = match origin {
			AggregateMessageOrigin::Ump(UmpQueueId::Para(para)) => para,
		};
		xcm_builder::ProcessXcmMessage::<
			Junction,
			xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
			RuntimeCall,
		>::process_message(message, Junction::Teyrchain(para.into()), meter, id)
	}
}

impl pezpallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Size = u32;
	type HeapSize = MessageQueueHeapSize;
	type MaxStale = MessageQueueMaxStale;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = MessageQueueServiceWeight;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = MessageProcessor;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor =
		pezpallet_message_queue::mock_helpers::NoopMessageProcessor<AggregateMessageOrigin>;
	type QueueChangeHandler = ParaInclusion;
	type QueuePausedQuery = ();
	type WeightInfo = weights::pezpallet_message_queue::WeightInfo<Runtime>;
}

impl teyrchains_dmp::Config for Runtime {}

parameter_types! {
	pub const HrmpChannelSizeAndCapacityWithSystemRatio: Percent = Percent::from_percent(100);
}

impl teyrchains_hrmp::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type ChannelManager = EnsureRoot<AccountId>;
	type Currency = Balances;
	type DefaultChannelSizeAndCapacityWithSystem = ActiveConfigHrmpChannelSizeAndCapacityRatio<
		Runtime,
		HrmpChannelSizeAndCapacityWithSystemRatio,
	>;
	type VersionWrapper = crate::XcmPallet;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_hrmp::WeightInfo<Runtime>;
}

impl teyrchains_paras_inherent::Config for Runtime {
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_paras_inherent::WeightInfo<Runtime>;
}

impl teyrchains_scheduler::Config for Runtime {
	// If you change this, make sure the `Assignment` type of the new provider is binary compatible,
	// otherwise provide a migration.
	type AssignmentProvider = CoretimeAssignmentProvider;
}

parameter_types! {
	pub const BrokerId: u32 = BROKER_ID;
	pub const BrokerPalletId: PalletId = PalletId(*b"py/broke");
	pub MaxXcmTransactWeight: Weight = Weight::from_parts(200_000_000, 20_000);
}

pub struct BrokerPot;
impl Get<InteriorLocation> for BrokerPot {
	fn get() -> InteriorLocation {
		Junction::AccountId32 { network: None, id: BrokerPalletId::get().into_account_truncating() }
			.into()
	}
}

impl coretime::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type BrokerId = BrokerId;
	type BrokerPotLocation = BrokerPot;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_coretime::WeightInfo<Runtime>;
	type SendXcm = crate::xcm_config::XcmRouter;
	type AssetTransactor = crate::xcm_config::LocalAssetTransactor;
	type AccountToLocation = xcm_builder::AliasesIntoAccountId32<
		xcm_config::ThisNetwork,
		<Runtime as pezframe_system::Config>::AccountId,
	>;
	type MaxXcmTransactWeight = MaxXcmTransactWeight;
}

parameter_types! {
	pub const OnDemandTrafficDefaultValue: FixedU128 = FixedU128::from_u32(1);
	// Keep 2 timeslices worth of revenue information.
	pub const MaxHistoricalRevenue: BlockNumber = 2 * TIMESLICE_PERIOD;
	pub const OnDemandPalletId: PalletId = PalletId(*b"py/ondmd");
}

impl teyrchains_on_demand::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type TrafficDefaultValue = OnDemandTrafficDefaultValue;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_on_demand::WeightInfo<Runtime>;
	type MaxHistoricalRevenue = MaxHistoricalRevenue;
	type PalletId = OnDemandPalletId;
}

impl teyrchains_assigner_coretime::Config for Runtime {}

impl teyrchains_initializer::Config for Runtime {
	type Randomness = pezpallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_initializer::WeightInfo<Runtime>;
	type CoretimeOnNewSession = Coretime;
}

impl teyrchains_disputes::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RewardValidators =
		teyrchains_reward_points::RewardValidatorsWithEraPoints<Runtime, StakingAhClient>;
	type SlashingHandler = teyrchains_slashing::SlashValidatorsForDisputes<ParasSlashing>;
	type WeightInfo = weights::pezkuwi_runtime_teyrchains_disputes::WeightInfo<Runtime>;
}

impl teyrchains_slashing::Config for Runtime {
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, ValidatorId)>>::Proof;
	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		ValidatorId,
	)>>::IdentificationTuple;
	type HandleReports = teyrchains_slashing::SlashingReportHandler<
		Self::KeyOwnerIdentification,
		Offences,
		ReportLongevity,
	>;
	type WeightInfo = teyrchains_slashing::TestWeightInfo;
	type BenchmarkingConfig = teyrchains_slashing::BenchConfig<200>;
}

parameter_types! {
	pub const ParaDeposit: Balance = 40 * UNITS;
}

impl paras_registrar::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type OnSwap = (Crowdloan, Slots, SwapLeases);
	type ParaDeposit = ParaDeposit;
	type DataDepositPerByte = DataDepositPerByte;
	type WeightInfo = weights::pezkuwi_runtime_common_paras_registrar::WeightInfo<Runtime>;
}

parameter_types! {
	pub LeasePeriod: BlockNumber = prod_or_fast!(1 * DAYS, 1 * DAYS, "TYR_LEASE_PERIOD");
}

impl slots::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Registrar = Registrar;
	type LeasePeriod = LeasePeriod;
	type LeaseOffset = ();
	type ForceOrigin = EitherOf<EnsureRoot<Self::AccountId>, LeaseAdmin>;
	type WeightInfo = weights::pezkuwi_runtime_common_slots::WeightInfo<Runtime>;
}

parameter_types! {
	pub const CrowdloanId: PalletId = PalletId(*b"py/cfund");
	pub const SubmissionDeposit: Balance = 3 * GRAND;
	pub const MinContribution: Balance = 3_000 * CENTS;
	pub const RemoveKeysLimit: u32 = 1000;
	// Allow 32 bytes for an additional memo to a crowdloan.
	pub const MaxMemoLength: u8 = 32;
}

impl crowdloan::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = CrowdloanId;
	type SubmissionDeposit = SubmissionDeposit;
	type MinContribution = MinContribution;
	type RemoveKeysLimit = RemoveKeysLimit;
	type Registrar = Registrar;
	type Auctioneer = Auctions;
	type MaxMemoLength = MaxMemoLength;
	type WeightInfo = weights::pezkuwi_runtime_common_crowdloan::WeightInfo<Runtime>;
}

parameter_types! {
	// The average auction is 7 days long, so this will be 70% for ending period.
	// 5 Days = 72000 Blocks @ 6 sec per block
	pub const EndingPeriod: BlockNumber = 5 * DAYS;
	// ~ 1000 samples per day -> ~ 20 blocks per sample -> 2 minute samples
	pub const SampleLength: BlockNumber = 2 * MINUTES;
}

impl auctions::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Leaser = Slots;
	type Registrar = Registrar;
	type EndingPeriod = EndingPeriod;
	type SampleLength = SampleLength;
	type Randomness = pezpallet_babe::RandomnessFromOneEpochAgo<Runtime>;
	type InitiateOrigin = EitherOf<EnsureRoot<Self::AccountId>, AuctionAdmin>;
	type WeightInfo = weights::pezkuwi_runtime_common_auctions::WeightInfo<Runtime>;
}

impl pezpallet_parameters::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeParameters = RuntimeParameters;
	type AdminOrigin = DynamicParameterOrigin;
	type WeightInfo = weights::pezpallet_parameters::WeightInfo<Runtime>;
}

parameter_types! {
	// BondingDuration(2) * SessionsPerEra(6) — matches AH staking config
	pub BeefySetIdSessionEntries: u32 = 2 * 6;
}

impl pezpallet_beefy::Config for Runtime {
	type BeefyId = BeefyId;
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = BeefySetIdSessionEntries;
	type OnNewValidatorSet = MmrLeaf;
	type AncestryHelper = MmrLeaf;
	type WeightInfo = ();
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, BeefyId)>>::Proof;
	type EquivocationReportSystem =
		pezpallet_beefy::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

/// MMR helper types.
mod mmr {
	use super::Runtime;
	pub use pezpallet_mmr::primitives::*;

	pub type Leaf = <<Runtime as pezpallet_mmr::Config>::LeafData as LeafDataProvider>::LeafData;
	pub type Hashing = <Runtime as pezpallet_mmr::Config>::Hashing;
	pub type Hash = <Hashing as pezsp_runtime::traits::Hash>::Output;
}

impl pezpallet_mmr::Config for Runtime {
	const INDEXING_PREFIX: &'static [u8] = mmr::INDEXING_PREFIX;
	type Hashing = Keccak256;
	type OnNewRoot = pezpallet_beefy_mmr::DepositBeefyDigest<Runtime>;
	type LeafData = pezpallet_beefy_mmr::Pezpallet<Runtime>;
	type BlockHashProvider = pezpallet_mmr::DefaultBlockHashProvider<Runtime>;
	type WeightInfo = weights::pezpallet_mmr::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = teyrchains_paras::benchmarking::mmr_setup::MmrSetup<Runtime>;
}

parameter_types! {
	pub LeafVersion: MmrLeafVersion = MmrLeafVersion::new(0, 0);
}

pub struct ParaHeadsRootProvider;
impl BeefyDataProvider<H256> for ParaHeadsRootProvider {
	fn extra_data() -> H256 {
		let para_heads: Vec<(u32, Vec<u8>)> =
			teyrchains_paras::Pezpallet::<Runtime>::sorted_para_heads();
		pez_binary_merkle_tree::merkle_root::<mmr::Hashing, _>(
			para_heads.into_iter().map(|pair| pair.encode()),
		)
		.into()
	}
}

impl pezpallet_beefy_mmr::Config for Runtime {
	type LeafVersion = LeafVersion;
	type BeefyAuthorityToMerkleLeaf = pezpallet_beefy_mmr::BeefyEcdsaToEthereum;
	type LeafExtra = H256;
	type BeefyDataProvider = ParaHeadsRootProvider;
	type WeightInfo = weights::pezpallet_beefy_mmr::WeightInfo<Runtime>;
}

impl paras_sudo_wrapper::Config for Runtime {}

parameter_types! {
	pub const PermanentSlotLeasePeriodLength: u32 = 365;
	pub const TemporarySlotLeasePeriodLength: u32 = 5;
	pub const MaxTemporarySlotPerLeasePeriod: u32 = 5;
}

impl assigned_slots::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssignSlotOrigin = EnsureRoot<AccountId>;
	type Leaser = Slots;
	type PermanentSlotLeasePeriodLength = PermanentSlotLeasePeriodLength;
	type TemporarySlotLeasePeriodLength = TemporarySlotLeasePeriodLength;
	type MaxTemporarySlotPerLeasePeriod = MaxTemporarySlotPerLeasePeriod;
	type WeightInfo = weights::pezkuwi_runtime_common_assigned_slots::WeightInfo<Runtime>;
}

impl validator_manager::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PrivilegedOrigin = EnsureRoot<AccountId>;
	type Staking = StakingAhClient;
}

parameter_types! {
	pub MbmServiceWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
}

impl pezpallet_migrations::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Migrations = ();
	// Benchmarks need mocked migrations to guarantee that they succeed.
	#[cfg(feature = "runtime-benchmarks")]
	type Migrations = pezpallet_migrations::mock_helpers::MockedMigrations;
	type CursorMaxLen = ConstU32<65_536>;
	type IdentifierMaxLen = ConstU32<256>;
	type MigrationStatusHandler = ();
	type FailedMigrationHandler = pezframe_support::migrations::FreezeChainOnFailedMigration;
	type MaxServiceWeight = MbmServiceWeight;
	type WeightInfo = weights::pezpallet_migrations::WeightInfo<Runtime>;
}

impl pezpallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = weights::pezpallet_sudo::WeightInfo<Runtime>;
}

impl pezpallet_root_testing::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

// Notify `coretime` pezpallet when a lease swap occurs
pub struct SwapLeases;
impl OnSwap for SwapLeases {
	fn on_swap(one: ParaId, other: ParaId) {
		coretime::Pezpallet::<Runtime>::on_legacy_lease_swap(one, other);
	}
}

construct_runtime! {
	pub enum Runtime
	{
		// Basic stuff; balances is uncallable initially.
		System: pezframe_system = 0,

		// Babe must be before session.
		Babe: pezpallet_babe = 1,

		Timestamp: pezpallet_timestamp = 2,
		Indices: pezpallet_indices = 3,
		Balances: pezpallet_balances = 4,
		Parameters: pezpallet_parameters = 6,
		TransactionPayment: pezpallet_transaction_payment = 33,

		// Consensus support.
		// Authorship must be before session in order to note author in the correct session and era.
		Authorship: pezpallet_authorship = 5,
		Offences: pezpallet_offences = 7,
		Historical: session_historical = 34,

		Session: pezpallet_session = 8,
		Grandpa: pezpallet_grandpa = 10,
		AuthorityDiscovery: pezpallet_authority_discovery = 12,

		// Governance stuff; uncallable initially.
		Council: pezpallet_collective::<Instance1> = 17,
		Treasury: pezpallet_treasury = 18,
		ConvictionVoting: pezpallet_conviction_voting = 20,
		Referenda: pezpallet_referenda = 21,
		Origins: pezpallet_custom_origins = 43,
		Whitelist: pezpallet_whitelist = 44,
		// Claims. Usable initially.
		Claims: claims = 19,

		// Utility module.
		Utility: pezpallet_utility = 24,

		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: pezpallet_vesting = 28,

		// System scheduler.
		Scheduler: pezpallet_scheduler = 29,

		// Proxy module. Late addition.
		Proxy: pezpallet_proxy = 30,

		// Multisig module. Late addition.
		Multisig: pezpallet_multisig = 31,

		// Preimage registrar.
		Preimage: pezpallet_preimage = 32,

		// Teyrchains pallets. Start indices at 50 to leave room.
		TeyrchainsOrigin: teyrchains_origin = 50,
		Configuration: teyrchains_configuration = 51,
		ParasShared: teyrchains_shared = 52,
		ParaInclusion: teyrchains_inclusion = 53,
		ParaInherent: teyrchains_paras_inherent = 54,
		ParaScheduler: teyrchains_scheduler = 55,
		Paras: teyrchains_paras = 56,
		Initializer: teyrchains_initializer = 57,
		Dmp: teyrchains_dmp = 58,
		Hrmp: teyrchains_hrmp = 60,
		ParaSessionInfo: teyrchains_session_info = 61,
		ParasDisputes: teyrchains_disputes = 62,
		ParasSlashing: teyrchains_slashing = 63,
		MessageQueue: pezpallet_message_queue = 64,
		OnDemandAssignmentProvider: teyrchains_on_demand = 66,
		StakingAhClient: pezpallet_staking_async_ah_client = 67,
		CoretimeAssignmentProvider: teyrchains_assigner_coretime = 68,

		// Teyrchain Onboarding Pallets. Start indices at 70 to leave room.
		Registrar: paras_registrar = 70,
		Slots: slots = 71,
		Auctions: auctions = 72,
		Crowdloan: crowdloan = 73,
		Coretime: coretime = 74,

		// Migrations pezpallet
		MultiBlockMigrations: pezpallet_migrations = 98,

		// Pezpallet for sending XCM.
		XcmPallet: pezpallet_xcm = 99,

		// BEEFY Bridges support.
		Beefy: pezpallet_beefy = 240,
		// MMR leaf construction must be after session in order to have a leaf's next_auth_set
		// refer to block<N>. See issue pezkuwi-fellows/runtimes#160 for details.
		Mmr: pezpallet_mmr = 241,
		MmrLeaf: pezpallet_beefy_mmr = 242,

		ParasSudoWrapper: paras_sudo_wrapper = 250,
		AssignedSlots: assigned_slots = 251,

		// Validator Manager pezpallet.
		ValidatorManager: validator_manager = 252,

		// State trie migration pezpallet, only temporary.
		StateTrieMigration: pezpallet_state_trie_migration = 254,

		// === CUSTOM PEZKUWI PALLETS ===
		// TNPoS Validator Pool - Shadow Mode (runs parallel to NPoS)
		ValidatorPool: pezpallet_validator_pool = 91,

		// Root testing pezpallet.
		RootTesting: pezpallet_root_testing = 249,

		// Sudo.
		Sudo: pezpallet_sudo = 255,
	}
}

/// The address format for describing accounts.
pub type Address = pezsp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// `BlockId` type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The extension to the basic transaction logic.
pub type TxExtension = (
	pezframe_system::AuthorizeCall<Runtime>,
	pezframe_system::CheckNonZeroSender<Runtime>,
	pezframe_system::CheckSpecVersion<Runtime>,
	pezframe_system::CheckTxVersion<Runtime>,
	pezframe_system::CheckGenesis<Runtime>,
	pezframe_system::CheckMortality<Runtime>,
	pezframe_system::CheckNonce<Runtime>,
	pezframe_system::CheckWeight<Runtime>,
	pezpallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	pezframe_metadata_hash_extension::CheckMetadataHash<Runtime>,
	pezframe_system::WeightReclaim<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;
/// Unchecked signature payload type as expected by this runtime.
pub type UncheckedSignaturePayload =
	generic::UncheckedSignaturePayload<Address, Signature, TxExtension>;

/// All migrations that will run on the next runtime upgrade.
///
/// This contains the combined migrations of the last 10 releases. It allows to skip runtime
/// upgrades in case governance decides to do so. THE ORDER IS IMPORTANT.
pub type Migrations = migrations::Unreleased;

/// The runtime migrations per release.
#[allow(deprecated, missing_docs)]
pub mod migrations {
	use super::*;

	use pezframe_support::traits::LockIdentifier;

	pub struct GetLegacyLeaseImpl;
	impl coretime::migration::GetLegacyLease<BlockNumber> for GetLegacyLeaseImpl {
		fn get_teyrchain_lease_in_blocks(para: ParaId) -> Option<BlockNumber> {
			let now = pezframe_system::Pezpallet::<Runtime>::block_number();
			let lease = slots::Leases::<Runtime>::get(para);
			if lease.is_empty() {
				return None;
			}
			// Lease not yet started, ignore:
			if lease.iter().any(Option::is_none) {
				return None;
			}
			let (index, _) =
				<slots::Pezpallet<Runtime> as Leaser<BlockNumber>>::lease_period_index(now)?;
			Some(index.saturating_add(lease.len() as u32).saturating_mul(LeasePeriod::get()))
		}

		fn get_all_teyrchains_with_leases() -> Vec<ParaId> {
			slots::Leases::<Runtime>::iter()
				.filter(|(_, lease)| !lease.is_empty())
				.map(|(para, _)| para)
				.collect::<Vec<_>>()
		}
	}

	parameter_types! {
		pub const DemocracyPalletName: &'static str = "Democracy";
		pub const CouncilPalletName: &'static str = "Council";
		pub const TechnicalCommitteePalletName: &'static str = "TechnicalCommittee";
		pub const PhragmenElectionPalletName: &'static str = "PhragmenElection";
		pub const TechnicalMembershipPalletName: &'static str = "TechnicalMembership";
		pub const TipsPalletName: &'static str = "Tips";
		pub const PhragmenElectionPalletId: LockIdentifier = *b"phrelect";
		// Old staking ecosystem pallets (replaced by StakingAhClient + AH staking)
		pub const StakingPalletName: &'static str = "Staking";
		pub const FastUnstakePalletName: &'static str = "FastUnstake";
		pub const VoterBagsListPalletName: &'static str = "VoterBagsList";
		/// Weight for balance unreservations
		pub BalanceUnreserveWeight: Weight = weights::pezpallet_balances_balances::WeightInfo::<Runtime>::force_unreserve();
		pub BalanceTransferAllowDeath: Weight = weights::pezpallet_balances_balances::WeightInfo::<Runtime>::transfer_allow_death();
	}

	// Special Config for Gov V1 pallets, allowing us to run migrations for them without
	// implementing their configs on [`Runtime`].
	// NOTE: Gov1 migration configs removed - pezpallet-democracy, pezpallet-elections-phragmen,
	// and pezpallet-tips are no longer part of this runtime (using pezpallet-welati for governance)

	/// Unreleased migrations. Add new ones here:
	pub type Unreleased = (
		teyrchains_configuration::migration::v7::MigrateToV7<Runtime>,
		assigned_slots::migration::v1::MigrateToV1<Runtime>,
		teyrchains_scheduler::migration::MigrateV1ToV2<Runtime>,
		teyrchains_configuration::migration::v8::MigrateToV8<Runtime>,
		teyrchains_configuration::migration::v9::MigrateToV9<Runtime>,
		paras_registrar::migration::MigrateToV1<Runtime, ()>,
		pezpallet_referenda::migration::v1::MigrateV0ToV1<Runtime, ()>,
		// NOTE: Gov1 migration steps removed - pallets no longer in runtime
		// Treasury cleanup still included as it may have existing proposals
		pezpallet_treasury::migration::cleanup_proposals::Migration<
			Runtime,
			(),
			BalanceUnreserveWeight,
		>,
		// Delete all Gov v1 pezpallet storage key/values (still needed to clean up any leftover
		// storage)
		pezframe_support::migrations::RemovePallet<
			DemocracyPalletName,
			<Runtime as pezframe_system::Config>::DbWeight,
		>,
		pezframe_support::migrations::RemovePallet<
			CouncilPalletName,
			<Runtime as pezframe_system::Config>::DbWeight,
		>,
		pezframe_support::migrations::RemovePallet<
			TechnicalCommitteePalletName,
			<Runtime as pezframe_system::Config>::DbWeight,
		>,
		pezframe_support::migrations::RemovePallet<
			PhragmenElectionPalletName,
			<Runtime as pezframe_system::Config>::DbWeight,
		>,
		pezframe_support::migrations::RemovePallet<
			TechnicalMembershipPalletName,
			<Runtime as pezframe_system::Config>::DbWeight,
		>,
		pezframe_support::migrations::RemovePallet<
			TipsPalletName,
			<Runtime as pezframe_system::Config>::DbWeight,
		>,
		pezpallet_grandpa::migrations::MigrateV4ToV5<Runtime>,
		teyrchains_configuration::migration::v10::MigrateToV10<Runtime>,
		teyrchains_configuration::migration::v11::MigrateToV11<Runtime>,
		// This needs to come after the `teyrchains_configuration` above as we are reading the
		// configuration.
		coretime::migration::MigrateToCoretime<
			Runtime,
			crate::xcm_config::XcmRouter,
			GetLegacyLeaseImpl,
			TIMESLICE_PERIOD,
		>,
		teyrchains_configuration::migration::v12::MigrateToV12<Runtime>,
		teyrchains_on_demand::migration::MigrateV0ToV1<Runtime>,
		// migrates session storage item
		pezpallet_session::migrations::v1::MigrateV0ToV1<
			Runtime,
			pezpallet_session::migrations::v1::InitOffenceSeverity<Runtime>,
		>,
		// permanent
		pezpallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
		teyrchains_inclusion::migration::MigrateToV1<Runtime>,
		teyrchains_shared::migration::MigrateToV1<Runtime>,
		teyrchains_scheduler::migration::MigrateV2ToV3<Runtime>,
		// Remove old staking ecosystem pallets (replaced by StakingAhClient + AH staking)
		pezframe_support::migrations::RemovePallet<
			StakingPalletName,
			<Runtime as pezframe_system::Config>::DbWeight,
		>,
		pezframe_support::migrations::RemovePallet<
			FastUnstakePalletName,
			<Runtime as pezframe_system::Config>::DbWeight,
		>,
		pezframe_support::migrations::RemovePallet<
			VoterBagsListPalletName,
			<Runtime as pezframe_system::Config>::DbWeight,
		>,
	);
}

/// Executive: handles dispatch to the various modules.
pub type Executive = pezframe_executive::Executive<
	Runtime,
	Block,
	pezframe_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;

parameter_types! {
	// The deposit configuration for the singed migration. Specially if you want to allow any signed account to do the migration (see `SignedFilter`, these deposits should be high)
	pub const MigrationSignedDepositPerItem: Balance = 1 * CENTS;
	pub const MigrationSignedDepositBase: Balance = 20 * CENTS * 100;
	pub const MigrationMaxKeyLen: u32 = 512;
}

impl pezpallet_state_trie_migration::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type SignedDepositPerItem = MigrationSignedDepositPerItem;
	type SignedDepositBase = MigrationSignedDepositBase;
	type ControlOrigin = EnsureRoot<AccountId>;
	// specific account for the migration, can trigger the signed migrations.
	type SignedFilter = pezframe_system::EnsureSignedBy<MigController, AccountId>;

	// Use same weights as bizinikiwi ones.
	type WeightInfo = pezpallet_state_trie_migration::weights::BizinikiwiWeight<Runtime>;
	type MaxKeyLen = MigrationMaxKeyLen;
}

pezframe_support::ord_parameter_types! {
	pub const MigController: AccountId = AccountId::from(hex_literal::hex!("52bc71c1eca5353749542dfdf0af97bf764f9c2f44e860cd485f1cd86400f649"));
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	pezframe_benchmarking::define_benchmarks!(
		// Pezkuwi
		// NOTE: Make sure to prefix these with `runtime_common::` so
		// the that path resolves correctly in the generated file.
		[pezkuwi_runtime_common::assigned_slots, AssignedSlots]
		[pezkuwi_runtime_common::auctions, Auctions]
		[pezkuwi_runtime_common::crowdloan, Crowdloan]
		[pezkuwi_runtime_common::claims, Claims]
		[pezkuwi_runtime_common::slots, Slots]
		[pezkuwi_runtime_common::paras_registrar, Registrar]
		[pezkuwi_runtime_teyrchains::configuration, Configuration]
		[pezkuwi_runtime_teyrchains::coretime, Coretime]
		[pezkuwi_runtime_teyrchains::hrmp, Hrmp]
		[pezkuwi_runtime_teyrchains::disputes, ParasDisputes]
		[pezkuwi_runtime_teyrchains::inclusion, ParaInclusion]
		[pezkuwi_runtime_teyrchains::initializer, Initializer]
		[pezkuwi_runtime_teyrchains::paras_inherent, ParaInherent]
		[pezkuwi_runtime_teyrchains::paras, Paras]
		[pezkuwi_runtime_teyrchains::on_demand, OnDemandAssignmentProvider]
		// Bizinikiwi
		[pezpallet_balances, Balances]
		[pezpallet_beefy_mmr, MmrLeaf]
		[pezframe_benchmarking::baseline, Baseline::<Runtime>]
		[pezpallet_conviction_voting, ConvictionVoting]
		[pezpallet_indices, Indices]
		[pezpallet_message_queue, MessageQueue]
		[pezpallet_migrations, MultiBlockMigrations]
		[pezpallet_mmr, Mmr]
		[pezpallet_multisig, Multisig]
		[pezpallet_parameters, Parameters]
		[pezpallet_preimage, Preimage]
		[pezpallet_proxy, Proxy]
		[pezpallet_referenda, Referenda]
		[pezpallet_scheduler, Scheduler]
		[pezpallet_sudo, Sudo]
		[pezframe_system, SystemBench::<Runtime>]
		[pezframe_system_extensions, SystemExtensionsBench::<Runtime>]
		[pezpallet_timestamp, Timestamp]
		[pezpallet_transaction_payment, TransactionPayment]
		[pezpallet_treasury, Treasury]
		[pezpallet_utility, Utility]
		[pezpallet_vesting, Vesting]
		[pezpallet_whitelist, Whitelist]
		// Pezkuwichain Custom Pallets
		[pezpallet_validator_pool, ValidatorPool]
		// XCM
		[pezpallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
		[pezpallet_xcm_benchmarks::fungible, pezpallet_xcm_benchmarks::fungible::Pezpallet::<Runtime>]
		[pezpallet_xcm_benchmarks::generic, pezpallet_xcm_benchmarks::generic::Pezpallet::<Runtime>]
	);
}

pezsp_api::impl_runtime_apis! {
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

	impl xcm_runtime_pezapis::fees::XcmPaymentApi<Block> for Runtime {
		fn query_acceptable_payment_assets(xcm_version: xcm::Version) -> Result<Vec<VersionedAssetId>, XcmPaymentApiError> {
			let acceptable_assets = vec![AssetId(xcm_config::TokenLocation::get())];
			XcmPallet::query_acceptable_payment_assets(xcm_version, acceptable_assets)
		}

		fn query_weight_to_asset_fee(weight: Weight, asset: VersionedAssetId) -> Result<u128, XcmPaymentApiError> {
			type Trader = <XcmConfig as xcm_executor::Config>::Trader;
			XcmPallet::query_weight_to_asset_fee::<Trader>(weight, asset)
		}

		fn query_xcm_weight(message: VersionedXcm<()>) -> Result<Weight, XcmPaymentApiError> {
			XcmPallet::query_xcm_weight(message)
		}

		fn query_delivery_fees(destination: VersionedLocation, message: VersionedXcm<()>, asset_id: VersionedAssetId) -> Result<VersionedAssets, XcmPaymentApiError> {
			type AssetExchanger = <XcmConfig as xcm_executor::Config>::AssetExchanger;
			XcmPallet::query_delivery_fees::<AssetExchanger>(destination, message, asset_id)
		}
	}

	impl xcm_runtime_pezapis::dry_run::DryRunApi<Block, RuntimeCall, RuntimeEvent, OriginCaller> for Runtime {
		fn dry_run_call(origin: OriginCaller, call: RuntimeCall, result_xcms_version: XcmVersion) -> Result<CallDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			XcmPallet::dry_run_call::<Runtime, xcm_config::XcmRouter, OriginCaller, RuntimeCall>(origin, call, result_xcms_version)
		}

		fn dry_run_xcm(origin_location: VersionedLocation, xcm: VersionedXcm<RuntimeCall>) -> Result<XcmDryRunEffects<RuntimeEvent>, XcmDryRunApiError> {
			XcmPallet::dry_run_xcm::<xcm_config::XcmRouter>(origin_location, xcm)
		}
	}

	impl xcm_runtime_pezapis::conversions::LocationToAccountApi<Block, AccountId> for Runtime {
		fn convert_location(location: VersionedLocation) -> Result<
			AccountId,
			xcm_runtime_pezapis::conversions::Error
		> {
			xcm_runtime_pezapis::conversions::LocationToAccountHelper::<
				AccountId,
				xcm_config::LocationConverter,
			>::convert_location(location)
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
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: pezsp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: <Block as BlockT>::LazyBlock,
			data: pezsp_inherents::InherentData,
		) -> pezsp_inherents::CheckInherentsResult {
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

	#[api_version(15)]
	impl pezkuwi_primitives::runtime_api::TeyrchainHost<Block> for Runtime {
		fn validators() -> Vec<ValidatorId> {
			teyrchains_runtime_api_impl::validators::<Runtime>()
		}

		fn validator_groups() -> (Vec<Vec<ValidatorIndex>>, GroupRotationInfo<BlockNumber>) {
			teyrchains_runtime_api_impl::validator_groups::<Runtime>()
		}

		fn availability_cores() -> Vec<CoreState<Hash, BlockNumber>> {
			teyrchains_runtime_api_impl::availability_cores::<Runtime>()
		}

		fn persisted_validation_data(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<PersistedValidationData<Hash, BlockNumber>> {
			teyrchains_runtime_api_impl::persisted_validation_data::<Runtime>(para_id, assumption)
		}

		fn assumed_validation_data(
			para_id: ParaId,
			expected_persisted_validation_data_hash: Hash,
		) -> Option<(PersistedValidationData<Hash, BlockNumber>, ValidationCodeHash)> {
			teyrchains_runtime_api_impl::assumed_validation_data::<Runtime>(
				para_id,
				expected_persisted_validation_data_hash,
			)
		}

		fn check_validation_outputs(
			para_id: ParaId,
			outputs: pezkuwi_primitives::CandidateCommitments,
		) -> bool {
			teyrchains_runtime_api_impl::check_validation_outputs::<Runtime>(para_id, outputs)
		}

		fn session_index_for_child() -> SessionIndex {
			teyrchains_runtime_api_impl::session_index_for_child::<Runtime>()
		}

		fn validation_code(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<ValidationCode> {
			teyrchains_runtime_api_impl::validation_code::<Runtime>(para_id, assumption)
		}

		fn candidate_pending_availability(para_id: ParaId) -> Option<CommittedCandidateReceipt<Hash>> {
			#[allow(deprecated)]
			teyrchains_runtime_api_impl::candidate_pending_availability::<Runtime>(para_id)
		}

		fn candidate_events() -> Vec<CandidateEvent<Hash>> {
			teyrchains_runtime_api_impl::candidate_events::<Runtime, _>(|ev| {
				match ev {
					RuntimeEvent::ParaInclusion(ev) => {
						Some(ev)
					}
					_ => None,
				}
			})
		}

		fn session_info(index: SessionIndex) -> Option<SessionInfo> {
			teyrchains_runtime_api_impl::session_info::<Runtime>(index)
		}

		fn session_executor_params(session_index: SessionIndex) -> Option<ExecutorParams> {
			teyrchains_runtime_api_impl::session_executor_params::<Runtime>(session_index)
		}

		fn dmq_contents(recipient: ParaId) -> Vec<InboundDownwardMessage<BlockNumber>> {
			teyrchains_runtime_api_impl::dmq_contents::<Runtime>(recipient)
		}

		fn inbound_hrmp_channels_contents(
			recipient: ParaId
		) -> BTreeMap<ParaId, Vec<InboundHrmpMessage<BlockNumber>>> {
			teyrchains_runtime_api_impl::inbound_hrmp_channels_contents::<Runtime>(recipient)
		}

		fn validation_code_by_hash(hash: ValidationCodeHash) -> Option<ValidationCode> {
			teyrchains_runtime_api_impl::validation_code_by_hash::<Runtime>(hash)
		}

		fn on_chain_votes() -> Option<ScrapedOnChainVotes<Hash>> {
			teyrchains_runtime_api_impl::on_chain_votes::<Runtime>()
		}

		fn submit_pvf_check_statement(
			stmt: pezkuwi_primitives::PvfCheckStatement,
			signature: pezkuwi_primitives::ValidatorSignature
		) {
			teyrchains_runtime_api_impl::submit_pvf_check_statement::<Runtime>(stmt, signature)
		}

		fn pvfs_require_precheck() -> Vec<ValidationCodeHash> {
			teyrchains_runtime_api_impl::pvfs_require_precheck::<Runtime>()
		}

		fn validation_code_hash(para_id: ParaId, assumption: OccupiedCoreAssumption)
			-> Option<ValidationCodeHash>
		{
			teyrchains_runtime_api_impl::validation_code_hash::<Runtime>(para_id, assumption)
		}

		fn disputes() -> Vec<(SessionIndex, CandidateHash, DisputeState<BlockNumber>)> {
			teyrchains_runtime_api_impl::get_session_disputes::<Runtime>()
		}

		fn unapplied_slashes(
		) -> Vec<(SessionIndex, CandidateHash, slashing::LegacyPendingSlashes)> {
			teyrchains_runtime_api_impl::unapplied_slashes::<Runtime>()
		}

		fn unapplied_slashes_v2(
		) -> Vec<(SessionIndex, CandidateHash, slashing::PendingSlashes)> {
			teyrchains_runtime_api_impl::unapplied_slashes_v2::<Runtime>()
		}

		fn key_ownership_proof(
			validator_id: ValidatorId,
		) -> Option<slashing::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((TEYRCHAIN_KEY_TYPE_ID, validator_id))
				.map(|p| p.encode())
				.map(slashing::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_dispute_lost(
			dispute_proof: slashing::DisputeProof,
			key_ownership_proof: slashing::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			teyrchains_runtime_api_impl::submit_unsigned_slashing_report::<Runtime>(
				dispute_proof,
				key_ownership_proof,
			)
		}

		fn minimum_backing_votes() -> u32 {
			teyrchains_runtime_api_impl::minimum_backing_votes::<Runtime>()
		}

		fn para_backing_state(para_id: ParaId) -> Option<pezkuwi_primitives::async_backing::BackingState> {
			#[allow(deprecated)]
			teyrchains_runtime_api_impl::backing_state::<Runtime>(para_id)
		}

		fn async_backing_params() -> pezkuwi_primitives::AsyncBackingParams {
			#[allow(deprecated)]
			teyrchains_runtime_api_impl::async_backing_params::<Runtime>()
		}

		fn approval_voting_params() -> ApprovalVotingParams {
			teyrchains_runtime_api_impl::approval_voting_params::<Runtime>()
		}

		fn disabled_validators() -> Vec<ValidatorIndex> {
			teyrchains_runtime_api_impl::disabled_validators::<Runtime>()
		}

		fn node_features() -> NodeFeatures {
			teyrchains_runtime_api_impl::node_features::<Runtime>()
		}

		fn claim_queue() -> BTreeMap<CoreIndex, VecDeque<ParaId>> {
			teyrchains_runtime_api_impl::claim_queue::<Runtime>()
		}

		fn candidates_pending_availability(para_id: ParaId) -> Vec<CommittedCandidateReceipt<Hash>> {
			teyrchains_runtime_api_impl::candidates_pending_availability::<Runtime>(para_id)
		}

		fn backing_constraints(para_id: ParaId) -> Option<Constraints> {
			teyrchains_runtime_api_impl::backing_constraints::<Runtime>(para_id)
		}

		fn scheduling_lookahead() -> u32 {
			teyrchains_runtime_api_impl::scheduling_lookahead::<Runtime>()
		}

		fn validation_code_bomb_limit() -> u32 {
			teyrchains_runtime_api_impl::validation_code_bomb_limit::<Runtime>()
		}

		fn para_ids() -> Vec<ParaId> {
			teyrchains_staging_runtime_api_impl::para_ids::<Runtime>()
		}
	}

	#[api_version(6)]
	impl pezsp_consensus_beefy::BeefyApi<Block, BeefyId> for Runtime {
		fn beefy_genesis() -> Option<BlockNumber> {
			pezpallet_beefy::GenesisBlock::<Runtime>::get()
		}

		fn validator_set() -> Option<pezsp_consensus_beefy::ValidatorSet<BeefyId>> {
			Beefy::validator_set()
		}

		fn submit_report_double_voting_unsigned_extrinsic(
			equivocation_proof: pezsp_consensus_beefy::DoubleVotingProof<
				BlockNumber,
				BeefyId,
				BeefySignature,
			>,
			key_owner_proof: pezsp_consensus_beefy::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Beefy::submit_unsigned_double_voting_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn submit_report_fork_voting_unsigned_extrinsic(
			equivocation_proof:
				pezsp_consensus_beefy::ForkVotingProof<
					<Block as BlockT>::Header,
					BeefyId,
					pezsp_runtime::OpaqueValue
				>,
			key_owner_proof: pezsp_consensus_beefy::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			Beefy::submit_unsigned_fork_voting_report(
				equivocation_proof.try_into()?,
				key_owner_proof.decode()?,
			)
		}

		fn submit_report_future_block_voting_unsigned_extrinsic(
			equivocation_proof: pezsp_consensus_beefy::FutureBlockVotingProof<BlockNumber, BeefyId>,
			key_owner_proof: pezsp_consensus_beefy::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			Beefy::submit_unsigned_future_block_voting_report(
				equivocation_proof,
				key_owner_proof.decode()?,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: pezsp_consensus_beefy::ValidatorSetId,
			authority_id: BeefyId,
		) -> Option<pezsp_consensus_beefy::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((pezsp_consensus_beefy::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(pezsp_consensus_beefy::OpaqueKeyOwnershipProof::new)
		}
	}

	#[api_version(3)]
	impl mmr::MmrApi<Block, mmr::Hash, BlockNumber> for Runtime {
		fn mmr_root() -> Result<mmr::Hash, mmr::Error> {
			Ok(pezpallet_mmr::RootHash::<Runtime>::get())
		}

		fn mmr_leaf_count() -> Result<mmr::LeafIndex, mmr::Error> {
			Ok(pezpallet_mmr::NumberOfLeaves::<Runtime>::get())
		}

		fn generate_proof(
			block_numbers: Vec<BlockNumber>,
			best_known_block_number: Option<BlockNumber>,
		) -> Result<(Vec<mmr::EncodableOpaqueLeaf>, mmr::LeafProof<mmr::Hash>), mmr::Error> {
			Mmr::generate_proof(block_numbers, best_known_block_number).map(
				|(leaves, proof)| {
					(
						leaves
							.into_iter()
							.map(|leaf| mmr::EncodableOpaqueLeaf::from_leaf(&leaf))
							.collect(),
						proof,
					)
				},
			)
		}

		fn generate_ancestry_proof(
			prev_block_number: BlockNumber,
			best_known_block_number: Option<BlockNumber>,
		) -> Result<mmr::AncestryProof<mmr::Hash>, mmr::Error> {
			Mmr::generate_ancestry_proof(prev_block_number, best_known_block_number)
		}

		fn verify_proof(leaves: Vec<mmr::EncodableOpaqueLeaf>, proof: mmr::LeafProof<mmr::Hash>)
			-> Result<(), mmr::Error>
		{
			let leaves = leaves.into_iter().map(|leaf|
				leaf.into_opaque_leaf()
				.try_decode()
				.ok_or(mmr::Error::Verify)).collect::<Result<Vec<mmr::Leaf>, mmr::Error>>()?;
			Mmr::verify_leaves(leaves, proof)
		}

		fn verify_proof_stateless(
			root: mmr::Hash,
			leaves: Vec<mmr::EncodableOpaqueLeaf>,
			proof: mmr::LeafProof<mmr::Hash>
		) -> Result<(), mmr::Error> {
			let nodes = leaves.into_iter().map(|leaf|mmr::DataOrHash::Data(leaf.into_opaque_leaf())).collect();
			pezpallet_mmr::verify_leaves_proof::<mmr::Hashing, _>(root, nodes, proof)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> Vec<(GrandpaId, u64)> {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			pezpallet_grandpa::CurrentSetId::<Runtime>::get()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				pezsp_runtime::traits::NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: fg_primitives::AuthorityId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
		}
	}

	impl pezsp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> pezsp_consensus_babe::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
			pezsp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDurationInBlocks::get().into(),
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> pezsp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> pezsp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> pezsp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: pezsp_consensus_babe::Slot,
			authority_id: pezsp_consensus_babe::AuthorityId,
		) -> Option<pezsp_consensus_babe::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((pezsp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(pezsp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: pezsp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: pezsp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl pezsp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			teyrchains_runtime_api_impl::relevant_authority_ids::<Runtime>()
		}
	}

	impl pezsp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, pezsp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl pezframe_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pezpallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pezpallet_beefy_mmr::BeefyMmrApi<Block, Hash> for RuntimeApi {
		fn authority_set_proof() -> pezsp_consensus_beefy::mmr::BeefyAuthoritySet<Hash> {
			MmrLeaf::authority_set_proof()
		}

		fn next_authority_set_proof() -> pezsp_consensus_beefy::mmr::BeefyNextAuthoritySet<Hash> {
			MmrLeaf::next_authority_set_proof()
		}
	}

	#[cfg(feature = "try-runtime")]
	impl pezframe_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: pezframe_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade pezkuwichain.");
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: <Block as BlockT>::LazyBlock,
			state_root_check: bool,
			signature_check: bool,
			select: pezframe_try_runtime::TryStateSelect,
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl pezframe_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<pezframe_benchmarking::BenchmarkList>,
			Vec<pezframe_support::traits::StorageInfo>,
		) {
			use pezframe_benchmarking::BenchmarkList;
			use pezframe_support::traits::StorageInfoTrait;

			use pezframe_system_benchmarking::Pezpallet as SystemBench;
			use pezframe_system_benchmarking::extensions::Pezpallet as SystemExtensionsBench;
			use pezframe_benchmarking::baseline::Pezpallet as Baseline;

			use pezpallet_xcm::benchmarking::Pezpallet as PalletXcmExtrinsicsBenchmark;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			return (list, storage_info)
		}

		#[allow(non_local_definitions)]
		fn dispatch_benchmark(
			config: pezframe_benchmarking::BenchmarkConfig,
		) -> Result<
			Vec<pezframe_benchmarking::BenchmarkBatch>,
			alloc::string::String,
		> {
			use pezframe_support::traits::WhitelistedStorageKeys;
			use pezframe_benchmarking::{BenchmarkBatch, BenchmarkError};
			use pezframe_system_benchmarking::Pezpallet as SystemBench;
			use pezframe_system_benchmarking::extensions::Pezpallet as SystemExtensionsBench;
			use pezframe_benchmarking::baseline::Pezpallet as Baseline;
			use pezpallet_xcm::benchmarking::Pezpallet as PalletXcmExtrinsicsBenchmark;
			use pezsp_storage::TrackedStorageKey;
			use xcm::latest::prelude::*;
			use xcm_config::{
				AssetHub, LocationConverter, TeleportTracking, TokenLocation, XcmConfig,
			};

			parameter_types! {
				pub ExistentialDepositAsset: Option<Asset> = Some((
					TokenLocation::get(),
					ExistentialDeposit::get()
				).into());
				pub AssetHubParaId: ParaId = pezkuwichain_runtime_constants::system_teyrchain::ASSET_HUB_ID.into();
				pub const RandomParaId: ParaId = ParaId::new(43211234);
			}

			impl pezframe_system_benchmarking::Config for Runtime {}
			impl pezframe_benchmarking::baseline::Config for Runtime {}
			impl pezpallet_xcm::benchmarking::Config for Runtime {
				type DeliveryHelper = (
					pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
						XcmConfig,
						ExistentialDepositAsset,
						xcm_config::PriceForChildTeyrchainDelivery,
						AssetHubParaId,
						Dmp,
					>,
					pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
						XcmConfig,
						ExistentialDepositAsset,
						xcm_config::PriceForChildTeyrchainDelivery,
						RandomParaId,
						Dmp,
					>
				);

				fn reachable_dest() -> Option<Location> {
					Some(crate::xcm_config::AssetHub::get())
				}

				fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
					// Relay/native token can be teleported to/from AH.
					Some((
						Asset {
							fun: Fungible(ExistentialDeposit::get()),
							id: AssetId(Here.into())
						},
						crate::xcm_config::AssetHub::get(),
					))
				}

				fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
					None
				}

				fn set_up_complex_asset_transfer(
				) -> Option<(Assets, AssetId, Location, alloc::boxed::Box<dyn FnOnce()>)> {
					// Relay supports only native token, either reserve transfer it to non-system teyrchains,
					// or teleport it to system teyrchain. Use the teleport case for benchmarking as it's
					// slightly heavier.
					// Relay/native token can be teleported to/from AH.
					let native_location = Here.into();
					let dest = crate::xcm_config::AssetHub::get();
					pezpallet_xcm::benchmarking::helpers::native_teleport_as_asset_transfer::<Runtime>(
						native_location,
						dest
					)
				}

				fn get_asset() -> Asset {
					Asset {
						id: AssetId(Location::here()),
						fun: Fungible(ExistentialDeposit::get()),
					}
				}
			}
			impl pezpallet_xcm_benchmarks::Config for Runtime {
				type XcmConfig = XcmConfig;
				type AccountIdConverter = LocationConverter;
				type DeliveryHelper = pezkuwi_runtime_common::xcm_sender::ToTeyrchainDeliveryHelper<
					XcmConfig,
					ExistentialDepositAsset,
					xcm_config::PriceForChildTeyrchainDelivery,
					AssetHubParaId,
					Dmp,
				>;
				fn valid_destination() -> Result<Location, BenchmarkError> {
					Ok(AssetHub::get())
				}
				fn worst_case_holding(_depositable_count: u32) -> Assets {
					// Pezkuwichain only knows about HEZ
					vec![Asset{
						id: AssetId(TokenLocation::get()),
						fun: Fungible(1_000_000 * UNITS),
					}].into()
				}
			}

			parameter_types! {
				pub TrustedTeleporter: Option<(Location, Asset)> = Some((
					AssetHub::get(),
					Asset { fun: Fungible(1 * UNITS), id: AssetId(TokenLocation::get()) },
				));
				pub TrustedReserve: Option<(Location, Asset)> = None;
			}

			impl pezpallet_xcm_benchmarks::fungible::Config for Runtime {
				type TransactAsset = Balances;

				type CheckedAccount = TeleportTracking;
				type TrustedTeleporter = TrustedTeleporter;
				type TrustedReserve = TrustedReserve;

				fn get_asset() -> Asset {
					Asset {
						id: AssetId(TokenLocation::get()),
						fun: Fungible(1 * UNITS),
					}
				}
			}

			impl pezpallet_xcm_benchmarks::generic::Config for Runtime {
				type TransactAsset = Balances;
				type RuntimeCall = RuntimeCall;

				fn worst_case_response() -> (u64, Response) {
					(0u64, Response::Version(Default::default()))
				}

				fn worst_case_asset_exchange() -> Result<(Assets, Assets), BenchmarkError> {
					// Pezkuwichain doesn't support asset exchanges
					Err(BenchmarkError::Skip)
				}

				fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
					// The XCM executor of Pezkuwichain doesn't have a configured `UniversalAliases`
					Err(BenchmarkError::Skip)
				}

				fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
					Ok((AssetHub::get(), pezframe_system::Call::remark_with_event { remark: vec![] }.into()))
				}

				fn subscribe_origin() -> Result<Location, BenchmarkError> {
					Ok(AssetHub::get())
				}

				fn claimable_asset() -> Result<(Location, Location, Assets), BenchmarkError> {
					let origin = AssetHub::get();
					let assets: Assets = (AssetId(TokenLocation::get()), 1_000 * UNITS).into();
					let ticket = Location { parents: 0, interior: Here };
					Ok((origin, ticket, assets))
				}

				fn worst_case_for_trader() -> Result<(Asset, WeightLimit), BenchmarkError> {
					Ok((Asset {
						id: AssetId(TokenLocation::get()),
						fun: Fungible(1_000_000 * UNITS),
					}, WeightLimit::Limited(Weight::from_parts(5000, 5000))))
				}

				fn unlockable_asset() -> Result<(Location, Location, Asset), BenchmarkError> {
					// Pezkuwichain doesn't support asset locking
					Err(BenchmarkError::Skip)
				}

				fn export_message_origin_and_destination(
				) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
					// Pezkuwichain doesn't support exporting messages
					Err(BenchmarkError::Skip)
				}

				fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
					// The XCM executor of Pezkuwichain doesn't have a configured `Aliasers`
					Err(BenchmarkError::Skip)
				}
			}

			let mut whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();
			let treasury_key = pezframe_system::Account::<Runtime>::hashed_key_for(Treasury::account_id());
			whitelist.push(treasury_key.to_vec().into());

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	impl pezsp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> pezsp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, &genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<PresetId> {
			genesis_config_presets::preset_names()
		}
	}

	impl xcm_runtime_pezapis::trusted_query::TrustedQueryApi<Block> for Runtime {
		fn is_trusted_reserve(asset: VersionedAsset, location: VersionedLocation) -> Result<bool, xcm_runtime_pezapis::trusted_query::Error> {
			XcmPallet::is_trusted_reserve(asset, location)
		}
		fn is_trusted_teleporter(asset: VersionedAsset, location: VersionedLocation) -> Result<bool, xcm_runtime_pezapis::trusted_query::Error> {
			XcmPallet::is_trusted_teleporter(asset, location)
		}
	}
}

#[cfg(all(test, feature = "try-runtime"))]
mod remote_tests {
	use super::*;
	use pezframe_try_runtime::{runtime_decl_for_try_runtime::TryRuntime, UpgradeCheckSelect};
	use remote_externalities::{
		Builder, Mode, OfflineConfig, OnlineConfig, SnapshotConfig, Transport,
	};
	use std::env::var;

	#[tokio::test]
	async fn run_migrations() {
		if var("RUN_MIGRATION_TESTS").is_err() {
			return;
		}

		pezsp_tracing::try_init_simple();
		let transport: Transport = var("WS")
			.unwrap_or("wss://pezkuwichain-rpc.pezkuwichain.io:443".to_string())
			.into();
		let maybe_state_snapshot: Option<SnapshotConfig> = var("SNAP").map(|s| s.into()).ok();
		let mut ext = Builder::<Block>::default()
			.mode(if let Some(state_snapshot) = maybe_state_snapshot {
				Mode::OfflineOrElseOnline(
					OfflineConfig { state_snapshot: state_snapshot.clone() },
					OnlineConfig {
						transport,
						state_snapshot: Some(state_snapshot),
						..Default::default()
					},
				)
			} else {
				Mode::Online(OnlineConfig { transport, ..Default::default() })
			})
			.build()
			.await
			.unwrap();
		ext.execute_with(|| Runtime::on_runtime_upgrade(UpgradeCheckSelect::PreAndPost));
	}
}
