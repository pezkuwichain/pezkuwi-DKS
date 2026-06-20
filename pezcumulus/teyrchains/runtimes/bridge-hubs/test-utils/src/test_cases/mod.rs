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

//! Module contains predefined test-case scenarios for `Runtime` with bridging capabilities.
//!
//! This file contains tests, suitable for all bridge runtimes. See `from_teyrchain` and
//! `from_grandpa_chain` submodules for tests, that are specific to the bridged chain type.

pub mod from_grandpa_chain;
pub mod from_teyrchain;

pub(crate) mod helpers;

use crate::{test_cases::bridges_prelude::*, test_data};

use asset_test_pezutils::BasicTeyrchainRuntime;
use codec::Encode;
use pezbp_messages::{
	target_chain::{DispatchMessage, DispatchMessageData, MessageDispatch},
	LaneState, MessageKey, MessagesOperatingMode, OutboundLaneData,
};
use pezbp_runtime::BasicOperatingMode;
use pezframe_support::{
	assert_ok,
	dispatch::GetDispatchInfo,
	traits::{Contains, Get, OnFinalize, OnInitialize, OriginTrait},
};
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezsp_runtime::{traits::Zero, AccountId32};
use teyrchains_common::AccountId;
use teyrchains_runtimes_test_utils::{
	mock_open_hrmp_channel, AccountIdOf, BalanceOf, CollatorSessionKeys, ExtBuilder,
	GovernanceOrigin, RuntimeCallOf, RuntimeOriginOf, SlotDurations, XcmReceivedFrom,
};
use xcm::{latest::prelude::*, AlwaysLatest};
use xcm_builder::DispatchBlobError;
use xcm_executor::{
	traits::{ConvertLocation, TransactAsset, WeightBounds},
	XcmExecutor,
};

/// Common bridges exports.
pub(crate) mod bridges_prelude {
	pub use pezbp_teyrchains::{RelayBlockHash, RelayBlockNumber};
	pub use pezpallet_bridge_grandpa::{Call as BridgeGrandpaCall, Config as BridgeGrandpaConfig};
	pub use pezpallet_bridge_messages::{
		Call as BridgeMessagesCall, Config as BridgeMessagesConfig, LanesManagerError,
	};
	pub use pezpallet_bridge_teyrchains::{
		Call as BridgeTeyrchainsCall, Config as BridgeTeyrchainsConfig,
	};
}

// Re-export test-case
pub use for_pallet_xcm_bridge_hub::open_and_close_bridge_works;

// Re-export test_case from assets
pub use asset_test_pezutils::include_teleports_for_native_asset_works;
use pezpallet_bridge_messages::LaneIdOf;

pub type RuntimeHelper<Runtime, AllPalletsWithoutSystem = ()> =
	teyrchains_runtimes_test_utils::RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

// Re-export test_case from `teyrchains-runtimes-test-utils`
pub use teyrchains_runtimes_test_utils::test_cases::{
	change_storage_constant_by_governance_works, set_storage_keys_by_governance_works,
};

/// Prepare default runtime storage and run test within this context.
pub fn run_test<Runtime, T>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	balances: Vec<(Runtime::AccountId, Runtime::Balance)>,
	test: impl FnOnce() -> T,
) -> T
where
	Runtime: BasicTeyrchainRuntime,
{
	ExtBuilder::<Runtime>::default()
		.with_collators(collator_session_key.collators())
		.with_session_keys(collator_session_key.session_keys())
		.with_safe_xcm_version(XCM_VERSION)
		.with_para_id(runtime_para_id.into())
		.with_balances(balances)
		.with_tracing()
		.build()
		.execute_with(|| test())
}

/// Test-case makes sure that `Runtime` can process bridging initialize via governance-like call
pub fn initialize_bridge_by_governance_works<Runtime, GrandpaPalletInstance>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	governance_origin: GovernanceOrigin<RuntimeOriginOf<Runtime>>,
) where
	Runtime: BasicTeyrchainRuntime + BridgeGrandpaConfig<GrandpaPalletInstance>,
	GrandpaPalletInstance: 'static,
	RuntimeCallOf<Runtime>:
		GetDispatchInfo + From<BridgeGrandpaCall<Runtime, GrandpaPalletInstance>>,
{
	run_test::<Runtime, _>(collator_session_key, runtime_para_id, vec![], || {
		// check mode before
		assert_eq!(
			pezpallet_bridge_grandpa::PalletOperatingMode::<Runtime, GrandpaPalletInstance>::try_get(),
			Err(())
		);

		// prepare the `initialize` call
		let initialize_call = RuntimeCallOf::<Runtime>::from(BridgeGrandpaCall::<
			Runtime,
			GrandpaPalletInstance,
		>::initialize {
			init_data: test_data::initialization_data::<Runtime, GrandpaPalletInstance>(12345),
		});

		// execute XCM with Transacts to `initialize bridge` as governance does
		assert_ok!(RuntimeHelper::<Runtime>::execute_as_governance_call(
			initialize_call,
			governance_origin
		));

		// check mode after
		assert_eq!(
			pezpallet_bridge_grandpa::PalletOperatingMode::<Runtime, GrandpaPalletInstance>::try_get(),
			Ok(BasicOperatingMode::Normal)
		);
	})
}

/// Test-case makes sure that `Runtime` can change bridge GRANDPA pezpallet operating mode via
/// governance-like call.
pub fn change_bridge_grandpa_pallet_mode_by_governance_works<Runtime, GrandpaPalletInstance>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	governance_origin: GovernanceOrigin<RuntimeOriginOf<Runtime>>,
) where
	Runtime: BasicTeyrchainRuntime + BridgeGrandpaConfig<GrandpaPalletInstance>,
	GrandpaPalletInstance: 'static,
	RuntimeCallOf<Runtime>:
		GetDispatchInfo + From<BridgeGrandpaCall<Runtime, GrandpaPalletInstance>>,
{
	run_test::<Runtime, _>(collator_session_key, runtime_para_id, vec![], || {
		let dispatch_set_operating_mode_call = |old_mode, new_mode| {
			// check old mode
			assert_eq!(
				pezpallet_bridge_grandpa::PalletOperatingMode::<Runtime, GrandpaPalletInstance>::get(),
				old_mode,
			);

			// prepare the `set_operating_mode` call
			let set_operating_mode_call = <Runtime as pezframe_system::Config>::RuntimeCall::from(
				pezpallet_bridge_grandpa::Call::<Runtime, GrandpaPalletInstance>::set_operating_mode {
					operating_mode: new_mode,
				},
			);

			// execute XCM with Transacts to `initialize bridge` as governance does
			assert_ok!(RuntimeHelper::<Runtime>::execute_as_governance_call(
				set_operating_mode_call,
				governance_origin.clone()
			));

			// check mode after
			assert_eq!(
				pezpallet_bridge_grandpa::PalletOperatingMode::<Runtime, GrandpaPalletInstance>::try_get(),
				Ok(new_mode)
			);
		};

		// check mode before
		assert_eq!(
			pezpallet_bridge_grandpa::PalletOperatingMode::<Runtime, GrandpaPalletInstance>::try_get(),
			Err(())
		);

		dispatch_set_operating_mode_call(BasicOperatingMode::Normal, BasicOperatingMode::Halted);
		dispatch_set_operating_mode_call(BasicOperatingMode::Halted, BasicOperatingMode::Normal);
	});
}

/// Test-case makes sure that `Runtime` can change bridge teyrchains pezpallet operating mode via
/// governance-like call.
pub fn change_bridge_teyrchains_pallet_mode_by_governance_works<Runtime, TeyrchainsPalletInstance>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	governance_origin: GovernanceOrigin<RuntimeOriginOf<Runtime>>,
) where
	Runtime: BasicTeyrchainRuntime + BridgeTeyrchainsConfig<TeyrchainsPalletInstance>,
	TeyrchainsPalletInstance: 'static,
	RuntimeCallOf<Runtime>:
		GetDispatchInfo + From<BridgeTeyrchainsCall<Runtime, TeyrchainsPalletInstance>>,
{
	run_test::<Runtime, _>(collator_session_key, runtime_para_id, vec![], || {
		let dispatch_set_operating_mode_call = |old_mode, new_mode| {
			// check old mode
			assert_eq!(
				pezpallet_bridge_teyrchains::PalletOperatingMode::<Runtime, TeyrchainsPalletInstance>::get(),
				old_mode,
			);

			// prepare the `set_operating_mode` call
			let set_operating_mode_call =
				RuntimeCallOf::<Runtime>::from(pezpallet_bridge_teyrchains::Call::<
					Runtime,
					TeyrchainsPalletInstance,
				>::set_operating_mode {
					operating_mode: new_mode,
				});

			// execute XCM with Transacts to `initialize bridge` as governance does
			assert_ok!(RuntimeHelper::<Runtime>::execute_as_governance_call(
				set_operating_mode_call,
				governance_origin.clone()
			));

			// check mode after
			assert_eq!(
				pezpallet_bridge_teyrchains::PalletOperatingMode::<Runtime, TeyrchainsPalletInstance>::try_get(),
				Ok(new_mode)
			);
		};

		// check mode before
		assert_eq!(
			pezpallet_bridge_teyrchains::PalletOperatingMode::<Runtime, TeyrchainsPalletInstance>::try_get(),
			Err(())
		);

		dispatch_set_operating_mode_call(BasicOperatingMode::Normal, BasicOperatingMode::Halted);
		dispatch_set_operating_mode_call(BasicOperatingMode::Halted, BasicOperatingMode::Normal);
	});
}

/// Test-case makes sure that `Runtime` can change bridge messaging pezpallet operating mode via
/// governance-like call.
pub fn change_bridge_messages_pallet_mode_by_governance_works<Runtime, MessagesPalletInstance>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	governance_origin: GovernanceOrigin<RuntimeOriginOf<Runtime>>,
) where
	Runtime: BasicTeyrchainRuntime + BridgeMessagesConfig<MessagesPalletInstance>,
	MessagesPalletInstance: 'static,
	RuntimeCallOf<Runtime>:
		GetDispatchInfo + From<BridgeMessagesCall<Runtime, MessagesPalletInstance>>,
{
	run_test::<Runtime, _>(collator_session_key, runtime_para_id, vec![], || {
		let dispatch_set_operating_mode_call = |old_mode, new_mode| {
			// check old mode
			assert_eq!(
				pezpallet_bridge_messages::PalletOperatingMode::<Runtime, MessagesPalletInstance>::get(
				),
				old_mode,
			);

			// encode `set_operating_mode` call
			let set_operating_mode_call = RuntimeCallOf::<Runtime>::from(BridgeMessagesCall::<
				Runtime,
				MessagesPalletInstance,
			>::set_operating_mode {
				operating_mode: new_mode,
			});

			// execute XCM with Transacts to `initialize bridge` as governance does
			assert_ok!(RuntimeHelper::<Runtime>::execute_as_governance_call(
				set_operating_mode_call,
				governance_origin.clone()
			));

			// check mode after
			assert_eq!(
				pezpallet_bridge_messages::PalletOperatingMode::<Runtime, MessagesPalletInstance>::try_get(),
				Ok(new_mode)
			);
		};

		// check mode before
		assert_eq!(
			pezpallet_bridge_messages::PalletOperatingMode::<Runtime, MessagesPalletInstance>::try_get(
			),
			Err(())
		);

		dispatch_set_operating_mode_call(
			MessagesOperatingMode::Basic(BasicOperatingMode::Normal),
			MessagesOperatingMode::RejectingOutboundMessages,
		);
		dispatch_set_operating_mode_call(
			MessagesOperatingMode::RejectingOutboundMessages,
			MessagesOperatingMode::Basic(BasicOperatingMode::Halted),
		);
		dispatch_set_operating_mode_call(
			MessagesOperatingMode::Basic(BasicOperatingMode::Halted),
			MessagesOperatingMode::Basic(BasicOperatingMode::Normal),
		);
	});
}

/// Test-case makes sure that `Runtime` can handle xcm `ExportMessage`:
/// Checks if received XCM messages is correctly added to the message outbound queue for delivery.
/// For SystemTeyrchains we expect unpaid execution.
pub fn handle_export_message_from_system_teyrchain_to_outbound_queue_works<
	Runtime,
	XcmConfig,
	MessagesPalletInstance,
>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	runtime_para_id: u32,
	sibling_teyrchain_id: u32,
	unwrap_pallet_bridge_messages_event: Box<
		dyn Fn(
			Vec<u8>,
		) -> Option<pezpallet_bridge_messages::Event<Runtime, MessagesPalletInstance>>,
	>,
	export_message_instruction: fn() -> Instruction<XcmConfig::RuntimeCall>,
	existential_deposit: Option<Asset>,
	maybe_paid_export_message: Option<Asset>,
	prepare_configuration: impl Fn() -> LaneIdOf<Runtime, MessagesPalletInstance>,
) where
	Runtime: BasicTeyrchainRuntime + BridgeMessagesConfig<MessagesPalletInstance>,
	XcmConfig: xcm_executor::Config,
	MessagesPalletInstance: 'static,
{
	assert_ne!(runtime_para_id, sibling_teyrchain_id);
	let sibling_teyrchain_location = Location::new(1, [Teyrchain(sibling_teyrchain_id)]);

	run_test::<Runtime, _>(collator_session_key, runtime_para_id, vec![], || {
		let expected_lane_id = prepare_configuration();

		// check queue before
		assert_eq!(
			pezpallet_bridge_messages::OutboundLanes::<Runtime, MessagesPalletInstance>::try_get(
				expected_lane_id
			),
			Ok(OutboundLaneData {
				state: LaneState::Opened,
				oldest_unpruned_nonce: 1,
				latest_received_nonce: 0,
				latest_generated_nonce: 0
			})
		);

		// prepare `ExportMessage`
		let xcm = if let Some(fee) = maybe_paid_export_message {
			// deposit ED to origin (if needed)
			if let Some(ed) = existential_deposit {
				XcmConfig::AssetTransactor::deposit_asset(
					&ed,
					&sibling_teyrchain_location,
					Some(&XcmContext::with_message_id([0; 32])),
				)
				.expect("deposited ed");
			}
			// deposit fee to origin
			XcmConfig::AssetTransactor::deposit_asset(
				&fee,
				&sibling_teyrchain_location,
				Some(&XcmContext::with_message_id([0; 32])),
			)
			.expect("deposited fee");

			Xcm(vec![
				WithdrawAsset(Assets::from(vec![fee.clone()])),
				BuyExecution { fees: fee, weight_limit: Unlimited },
				export_message_instruction(),
			])
		} else {
			Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				export_message_instruction(),
			])
		};

		// execute XCM
		let mut hash = xcm.using_encoded(pezsp_io::hashing::blake2_256);
		assert_ok!(XcmExecutor::<XcmConfig>::prepare_and_execute(
			sibling_teyrchain_location,
			xcm,
			&mut hash,
			RuntimeHelper::<Runtime>::xcm_max_weight(XcmReceivedFrom::Sibling),
			Weight::zero(),
		)
		.ensure_complete());

		// check queue after
		assert_eq!(
			pezpallet_bridge_messages::OutboundLanes::<Runtime, MessagesPalletInstance>::try_get(
				expected_lane_id
			),
			Ok(OutboundLaneData {
				state: LaneState::Opened,
				oldest_unpruned_nonce: 1,
				latest_received_nonce: 0,
				latest_generated_nonce: 1,
			})
		);

		// check events
		let mut events = <pezframe_system::Pezpallet<Runtime>>::events()
			.into_iter()
			.filter_map(|e| unwrap_pallet_bridge_messages_event(e.event.encode()));
		assert!(
			events.any(|e| matches!(e, pezpallet_bridge_messages::Event::MessageAccepted { .. }))
		);
	})
}

/// Test-case makes sure that Runtime can route XCM messages received in inbound queue,
/// We just test here `MessageDispatch` configuration.
/// We expect that runtime can route messages:
///     1. to Parent (relay chain)
///     2. to Sibling teyrchain
pub fn message_dispatch_routing_works<
	Runtime,
	AllPalletsWithoutSystem,
	XcmConfig,
	HrmpChannelOpener,
	MessagesPalletInstance,
	RuntimeNetwork,
	BridgedNetwork,
	NetworkDistanceAsParentCount,
>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	slot_durations: SlotDurations,
	runtime_para_id: u32,
	sibling_teyrchain_id: u32,
	unwrap_pezcumulus_pezpallet_teyrchain_system_event: Box<
		dyn Fn(Vec<u8>) -> Option<pezcumulus_pezpallet_teyrchain_system::Event<Runtime>>,
	>,
	unwrap_pezcumulus_pezpallet_xcmp_queue_event: Box<
		dyn Fn(Vec<u8>) -> Option<pezcumulus_pezpallet_xcmp_queue::Event<Runtime>>,
	>,
	prepare_configuration: impl Fn(),
) where
	Runtime: BasicTeyrchainRuntime
		+ pezcumulus_pezpallet_xcmp_queue::Config
		+ BridgeMessagesConfig<MessagesPalletInstance, InboundPayload = test_data::XcmAsPlainPayload>,
	AllPalletsWithoutSystem:
		OnInitialize<BlockNumberFor<Runtime>> + OnFinalize<BlockNumberFor<Runtime>>,
	AccountIdOf<Runtime>: From<AccountId32>
		+ Into<<<Runtime as pezframe_system::Config>::RuntimeOrigin as OriginTrait>::AccountId>,
	XcmConfig: xcm_executor::Config,
	MessagesPalletInstance: 'static,
	HrmpChannelOpener: pezframe_support::inherent::ProvideInherent<
		Call = pezcumulus_pezpallet_teyrchain_system::Call<Runtime>,
	>,
	RuntimeNetwork: Get<NetworkId>,
	BridgedNetwork: Get<NetworkId>,
	NetworkDistanceAsParentCount: Get<u8>,
{
	struct NetworkWithParentCount<N, C>(core::marker::PhantomData<(N, C)>);
	impl<N: Get<NetworkId>, C: Get<u8>> Get<Location> for NetworkWithParentCount<N, C> {
		fn get() -> Location {
			Location::new(C::get(), [GlobalConsensus(N::get())])
		}
	}
	assert_ne!(runtime_para_id, sibling_teyrchain_id);

	#[derive(Debug)]
	enum XcmBlobMessageDispatchResult {
		Dispatched,
		#[allow(dead_code)]
		NotDispatched(Option<DispatchBlobError>),
	}

	run_test::<Runtime, _>(collator_session_key, runtime_para_id, vec![], || {
		prepare_configuration();

		let dummy_lane_id = LaneIdOf::<Runtime, MessagesPalletInstance>::default();
		let mut alice = [0u8; 32];
		alice[0] = 1;

		let included_head = RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::run_to_block(
			2,
			AccountId::from(alice).into(),
		);
		// 1. this message is sent from other global consensus with destination of this Runtime
		//    relay chain (UMP)
		let bridging_message = test_data::simulate_message_exporter_on_bridged_chain::<
			BridgedNetwork,
			NetworkWithParentCount<RuntimeNetwork, NetworkDistanceAsParentCount>,
			AlwaysLatest,
		>((RuntimeNetwork::get(), Here));
		let result =
			<<Runtime as BridgeMessagesConfig<MessagesPalletInstance>>::MessageDispatch>::dispatch(
				test_data::dispatch_message(dummy_lane_id, 1, bridging_message),
			);
		assert_eq!(
			format!("{:?}", result.dispatch_level_result),
			format!("{:?}", XcmBlobMessageDispatchResult::Dispatched)
		);

		// check events - UpwardMessageSent
		let mut events = <pezframe_system::Pezpallet<Runtime>>::events()
			.into_iter()
			.filter_map(|e| unwrap_pezcumulus_pezpallet_teyrchain_system_event(e.event.encode()));
		assert!(events.any(|e| matches!(
			e,
			pezcumulus_pezpallet_teyrchain_system::Event::UpwardMessageSent { .. }
		)));

		// 2. this message is sent from other global consensus with destination of this Runtime
		//    sibling teyrchain (HRMP)
		let bridging_message =
			test_data::simulate_message_exporter_on_bridged_chain::<
				BridgedNetwork,
				NetworkWithParentCount<RuntimeNetwork, NetworkDistanceAsParentCount>,
				AlwaysLatest,
			>((RuntimeNetwork::get(), [Teyrchain(sibling_teyrchain_id)].into()));

		// 2.1. WITHOUT opened hrmp channel -> RoutingError
		let result =
			<<Runtime as BridgeMessagesConfig<MessagesPalletInstance>>::MessageDispatch>::dispatch(
				DispatchMessage {
					key: MessageKey { lane_id: dummy_lane_id, nonce: 1 },
					data: DispatchMessageData { payload: Ok(bridging_message.clone()) },
				},
			);
		assert_eq!(
			format!("{:?}", result.dispatch_level_result),
			format!(
				"{:?}",
				XcmBlobMessageDispatchResult::NotDispatched(Some(DispatchBlobError::RoutingError))
			)
		);

		// check events - no XcmpMessageSent
		assert_eq!(
			<pezframe_system::Pezpallet<Runtime>>::events()
				.into_iter()
				.filter_map(|e| unwrap_pezcumulus_pezpallet_xcmp_queue_event(e.event.encode()))
				.count(),
			0
		);

		// 2.1. WITH hrmp channel -> Ok
		mock_open_hrmp_channel::<Runtime, HrmpChannelOpener>(
			runtime_para_id.into(),
			sibling_teyrchain_id.into(),
			included_head,
			&alice,
			&slot_durations,
		);
		let result =
			<<Runtime as BridgeMessagesConfig<MessagesPalletInstance>>::MessageDispatch>::dispatch(
				DispatchMessage {
					key: MessageKey { lane_id: dummy_lane_id, nonce: 1 },
					data: DispatchMessageData { payload: Ok(bridging_message) },
				},
			);
		assert_eq!(
			format!("{:?}", result.dispatch_level_result),
			format!("{:?}", XcmBlobMessageDispatchResult::Dispatched)
		);

		// check events - XcmpMessageSent
		let mut events = <pezframe_system::Pezpallet<Runtime>>::events()
			.into_iter()
			.filter_map(|e| unwrap_pezcumulus_pezpallet_xcmp_queue_event(e.event.encode()));
		assert!(events
			.any(|e| matches!(e, pezcumulus_pezpallet_xcmp_queue::Event::XcmpMessageSent { .. })));
	})
}

/// Estimates XCM execution fee for paid `ExportMessage` processing.
pub fn can_calculate_weight_for_paid_export_message_with_reserve_transfer<
	Runtime,
	XcmConfig,
	WeightToFee,
>() -> u128
where
	Runtime: pezframe_system::Config + pezpallet_balances::Config,
	XcmConfig: xcm_executor::Config,
	WeightToFee: pezframe_support::weights::WeightToFee<Balance = BalanceOf<Runtime>>,
	<WeightToFee as pezframe_support::weights::WeightToFee>::Balance: From<u128> + Into<u128>,
{
	// data here are not relevant for weighing
	let mut xcm = Xcm(vec![
		WithdrawAsset(Assets::from(vec![Asset {
			id: AssetId(Location::new(1, [])),
			fun: Fungible(34333299),
		}])),
		BuyExecution {
			fees: Asset { id: AssetId(Location::new(1, [])), fun: Fungible(34333299) },
			weight_limit: Unlimited,
		},
		SetAppendix(Xcm(vec![DepositAsset {
			assets: Wild(AllCounted(1)),
			beneficiary: Location::new(1, [Teyrchain(1000)]),
		}])),
		ExportMessage {
			network: Pezkuwi,
			destination: [Teyrchain(1000)].into(),
			xcm: Xcm(vec![
				ReserveAssetDeposited(Assets::from(vec![Asset {
					id: AssetId(Location::new(2, [GlobalConsensus(Dicle)])),
					fun: Fungible(1000000000000),
				}])),
				ClearOrigin,
				BuyExecution {
					fees: Asset {
						id: AssetId(Location::new(2, [GlobalConsensus(Dicle)])),
						fun: Fungible(1000000000000),
					},
					weight_limit: Unlimited,
				},
				DepositAsset {
					assets: Wild(AllCounted(1)),
					beneficiary: Location::new(
						0,
						[xcm::latest::prelude::AccountId32 {
							network: None,
							id: [
								212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159,
								214, 130, 44, 133, 88, 133, 76, 205, 227, 154, 86, 132, 231, 165,
								109, 162, 125,
							],
						}],
					),
				},
				SetTopic([
					116, 82, 194, 132, 171, 114, 217, 165, 23, 37, 161, 177, 165, 179, 247, 114,
					137, 101, 147, 70, 28, 157, 168, 32, 154, 63, 74, 228, 152, 180, 5, 63,
				]),
			]),
		},
		SetTopic([
			36, 224, 250, 165, 82, 195, 67, 110, 160, 170, 140, 87, 217, 62, 201, 164, 42, 98, 219,
			157, 124, 105, 248, 25, 131, 218, 199, 36, 109, 173, 100, 122,
		]),
	]);

	// get weight
	let weight = XcmConfig::Weigher::weight(&mut xcm, Weight::MAX);
	assert_ok!(weight);
	let weight = weight.unwrap();
	// check if sane
	let max_expected = Runtime::BlockWeights::get().max_block / 10;
	assert!(
		weight.all_lte(max_expected),
		"calculated weight: {:?}, max_expected: {:?}",
		weight,
		max_expected
	);

	// check fee, should not be 0
	let estimated_fee = WeightToFee::weight_to_fee(&weight);
	assert!(estimated_fee > BalanceOf::<Runtime>::zero());

	estimated_fee.into()
}

pub(crate) mod for_pallet_xcm_bridge_hub {
	use super::*;
	use crate::test_cases::helpers::for_pallet_xcm_bridge_hub::{
		close_bridge, ensure_opened_bridge, open_bridge_with_extrinsic,
	};
	pub(crate) use pezpallet_xcm_bridge_hub::{
		Bridge, BridgeState, Call as BridgeXcmOverBridgeCall, Config as BridgeXcmOverBridgeConfig,
		LanesManagerOf,
	};

	/// Test-case makes sure that `Runtime` can open/close bridges.
	pub fn open_and_close_bridge_works<Runtime, XcmOverBridgePalletInstance, LocationToAccountId, TokenLocation>(
		collator_session_key: CollatorSessionKeys<Runtime>,
		runtime_para_id: u32,
		expected_source: Location,
		destination: InteriorLocation,
		origin_with_origin_kind: (Location, OriginKind),
		is_paid_xcm_execution: bool,
	) where
		Runtime: BasicTeyrchainRuntime + BridgeXcmOverBridgeConfig<XcmOverBridgePalletInstance>,
		XcmOverBridgePalletInstance: 'static,
		<Runtime as pezframe_system::Config>::RuntimeCall: GetDispatchInfo + From<BridgeXcmOverBridgeCall<Runtime, XcmOverBridgePalletInstance>>,
		<Runtime as pezpallet_balances::Config>::Balance: From<<<Runtime as pezpallet_bridge_messages::Config<<Runtime as pezpallet_xcm_bridge_hub::Config<XcmOverBridgePalletInstance>>::BridgeMessagesPalletInstance>>::ThisChain as pezbp_runtime::Chain>::Balance>,
		<Runtime as pezpallet_balances::Config>::Balance: From<u128>,
		<<Runtime as pezpallet_bridge_messages::Config<<Runtime as pezpallet_xcm_bridge_hub::Config<XcmOverBridgePalletInstance>>::BridgeMessagesPalletInstance>>::ThisChain as pezbp_runtime::Chain>::AccountId: From<<Runtime as pezframe_system::Config>::AccountId>,
		LocationToAccountId: ConvertLocation<AccountIdOf<Runtime>>,
		TokenLocation: Get<Location>,
	{
		run_test::<Runtime, _>(collator_session_key, runtime_para_id, vec![], || {
			// construct expected bridge configuration
			let locations = pezpallet_xcm_bridge_hub::Pezpallet::<
				Runtime,
				XcmOverBridgePalletInstance,
			>::bridge_locations(
				expected_source.clone().into(), destination.clone().into()
			)
			.expect("valid bridge locations");
			let expected_lane_id =
				locations.calculate_lane_id(xcm::latest::VERSION).expect("valid laneId");
			let lanes_manager = LanesManagerOf::<Runtime, XcmOverBridgePalletInstance>::new();

			let expected_deposit = if <Runtime as pezpallet_xcm_bridge_hub::Config<
				XcmOverBridgePalletInstance,
			>>::AllowWithoutBridgeDeposit::contains(
				locations.bridge_origin_relative_location()
			) {
				Zero::zero()
			} else {
				<Runtime as pezpallet_xcm_bridge_hub::Config<
					XcmOverBridgePalletInstance,
				>>::BridgeDeposit::get()
			};

			// check bridge/lane DOES not exist
			assert_eq!(
				pezpallet_xcm_bridge_hub::Bridges::<Runtime, XcmOverBridgePalletInstance>::get(
					locations.bridge_id()
				),
				None
			);
			assert_eq!(
				lanes_manager.active_inbound_lane(expected_lane_id).map(drop),
				Err(LanesManagerError::UnknownInboundLane)
			);
			assert_eq!(
				lanes_manager.active_outbound_lane(expected_lane_id).map(drop),
				Err(LanesManagerError::UnknownOutboundLane)
			);

			// open bridge with Transact call
			assert_eq!(
				ensure_opened_bridge::<
					Runtime,
					XcmOverBridgePalletInstance,
					LocationToAccountId,
					TokenLocation,
				>(
					expected_source.clone(),
					destination.clone(),
					is_paid_xcm_execution,
					|locations, maybe_paid_execution| open_bridge_with_extrinsic::<
						Runtime,
						XcmOverBridgePalletInstance,
					>(
						origin_with_origin_kind.clone(),
						locations.bridge_destination_universal_location().clone(),
						maybe_paid_execution
					)
				)
				.0
				.bridge_id(),
				locations.bridge_id()
			);

			// check bridge/lane DOES exist
			assert_eq!(
				pezpallet_xcm_bridge_hub::Bridges::<Runtime, XcmOverBridgePalletInstance>::get(
					locations.bridge_id()
				),
				Some(Bridge {
					bridge_origin_relative_location: Box::new(expected_source.clone().into()),
					bridge_origin_universal_location: Box::new(
						locations.bridge_origin_universal_location().clone().into()
					),
					bridge_destination_universal_location: Box::new(
						locations.bridge_destination_universal_location().clone().into()
					),
					state: BridgeState::Opened,
					bridge_owner_account: LocationToAccountId::convert_location(&expected_source)
						.expect("valid location")
						.into(),
					deposit: expected_deposit,
					lane_id: expected_lane_id,
				})
			);
			assert_eq!(
				lanes_manager.active_inbound_lane(expected_lane_id).map(|lane| lane.state()),
				Ok(LaneState::Opened)
			);
			assert_eq!(
				lanes_manager.active_outbound_lane(expected_lane_id).map(|lane| lane.state()),
				Ok(LaneState::Opened)
			);

			// close bridge with Transact call
			close_bridge::<Runtime, XcmOverBridgePalletInstance, LocationToAccountId, TokenLocation>(
				expected_source,
				destination,
				origin_with_origin_kind,
				is_paid_xcm_execution,
			);

			// check bridge/lane DOES not exist
			assert_eq!(
				pezpallet_xcm_bridge_hub::Bridges::<Runtime, XcmOverBridgePalletInstance>::get(
					locations.bridge_id()
				),
				None
			);
			assert_eq!(
				lanes_manager.active_inbound_lane(expected_lane_id).map(drop),
				Err(LanesManagerError::UnknownInboundLane)
			);
			assert_eq!(
				lanes_manager.active_outbound_lane(expected_lane_id).map(drop),
				Err(LanesManagerError::UnknownOutboundLane)
			);
		});
	}
}
