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

//! Module contains tests code, that is shared by all types of bridges

use crate::test_cases::{bridges_prelude::*, run_test, RuntimeHelper};

use asset_test_pezutils::BasicTeyrchainRuntime;
use codec::Decode;
use core::marker::PhantomData;
use pezbp_messages::MessageNonce;
use pezbp_pezkuwi_core::teyrchains::{ParaHash, ParaId};
use pezbp_runtime::Chain;
use pezframe_support::{
	assert_ok,
	dispatch::GetDispatchInfo,
	traits::{fungible::Mutate, Contains, OnFinalize, OnInitialize, PalletInfoAccess},
};
use pezframe_system::pezpallet_prelude::BlockNumberFor;
use pezpallet_bridge_grandpa::{BridgedBlockHash, BridgedHeader};
use pezpallet_bridge_messages::{BridgedChainOf, LaneIdOf};
use pezsp_core::Get;
use pezsp_keyring::Sr25519Keyring::*;
use pezsp_runtime::{traits::TrailingZeroInput, AccountId32};
use teyrchains_common::AccountId;
use teyrchains_runtimes_test_utils::{
	mock_open_hrmp_channel, AccountIdOf, CollatorSessionKeys, RuntimeCallOf, SlotDurations,
};
use xcm::latest::prelude::*;
use xcm_executor::traits::ConvertLocation;

/// Verify that the transaction has succeeded.
#[impl_trait_for_tuples::impl_for_tuples(30)]
pub trait VerifyTransactionOutcome {
	fn verify_outcome(&self);
}

impl VerifyTransactionOutcome for Box<dyn VerifyTransactionOutcome> {
	fn verify_outcome(&self) {
		VerifyTransactionOutcome::verify_outcome(&**self)
	}
}

/// Checks that the best finalized header hash in the bridge GRANDPA pezpallet equals to given one.
pub struct VerifySubmitGrandpaFinalityProofOutcome<Runtime, GPI>
where
	Runtime: BridgeGrandpaConfig<GPI>,
	GPI: 'static,
{
	expected_best_hash: BridgedBlockHash<Runtime, GPI>,
}

impl<Runtime, GPI> VerifySubmitGrandpaFinalityProofOutcome<Runtime, GPI>
where
	Runtime: BridgeGrandpaConfig<GPI>,
	GPI: 'static,
{
	/// Expect given header hash to be the best after transaction.
	pub fn expect_best_header_hash(
		expected_best_hash: BridgedBlockHash<Runtime, GPI>,
	) -> Box<dyn VerifyTransactionOutcome> {
		Box::new(Self { expected_best_hash })
	}
}

impl<Runtime, GPI> VerifyTransactionOutcome
	for VerifySubmitGrandpaFinalityProofOutcome<Runtime, GPI>
where
	Runtime: BridgeGrandpaConfig<GPI>,
	GPI: 'static,
{
	fn verify_outcome(&self) {
		assert_eq!(
			pezpallet_bridge_grandpa::BestFinalized::<Runtime, GPI>::get().unwrap().1,
			self.expected_best_hash
		);
		assert!(pezpallet_bridge_grandpa::ImportedHeaders::<Runtime, GPI>::contains_key(
			self.expected_best_hash
		));
	}
}

/// Checks that the best teyrchain header hash in the bridge teyrchains pezpallet equals to given
/// one.
pub struct VerifySubmitTeyrchainHeaderProofOutcome<Runtime, PPI> {
	bridged_para_id: u32,
	expected_best_hash: ParaHash,
	_marker: PhantomData<(Runtime, PPI)>,
}

impl<Runtime, PPI> VerifySubmitTeyrchainHeaderProofOutcome<Runtime, PPI>
where
	Runtime: BridgeTeyrchainsConfig<PPI>,
	PPI: 'static,
{
	/// Expect given header hash to be the best after transaction.
	pub fn expect_best_header_hash(
		bridged_para_id: u32,
		expected_best_hash: ParaHash,
	) -> Box<dyn VerifyTransactionOutcome> {
		Box::new(Self { bridged_para_id, expected_best_hash, _marker: PhantomData })
	}
}

impl<Runtime, PPI> VerifyTransactionOutcome
	for VerifySubmitTeyrchainHeaderProofOutcome<Runtime, PPI>
where
	Runtime: BridgeTeyrchainsConfig<PPI>,
	PPI: 'static,
{
	fn verify_outcome(&self) {
		assert_eq!(
			pezpallet_bridge_teyrchains::ParasInfo::<Runtime, PPI>::get(ParaId(
				self.bridged_para_id
			))
			.map(|info| info.best_head_hash.head_hash),
			Some(self.expected_best_hash),
		);
	}
}

/// Checks that the latest delivered nonce in the bridge messages pezpallet equals to given one.
pub struct VerifySubmitMessagesProofOutcome<Runtime: BridgeMessagesConfig<MPI>, MPI: 'static> {
	lane: LaneIdOf<Runtime, MPI>,
	expected_nonce: MessageNonce,
	_marker: PhantomData<(Runtime, MPI)>,
}

impl<Runtime, MPI> VerifySubmitMessagesProofOutcome<Runtime, MPI>
where
	Runtime: BridgeMessagesConfig<MPI>,
	MPI: 'static,
{
	/// Expect given delivered nonce to be the latest after transaction.
	pub fn expect_last_delivered_nonce(
		lane: LaneIdOf<Runtime, MPI>,
		expected_nonce: MessageNonce,
	) -> Box<dyn VerifyTransactionOutcome> {
		Box::new(Self { lane, expected_nonce, _marker: PhantomData })
	}
}

impl<Runtime, MPI> VerifyTransactionOutcome for VerifySubmitMessagesProofOutcome<Runtime, MPI>
where
	Runtime: BridgeMessagesConfig<MPI>,
	MPI: 'static,
{
	fn verify_outcome(&self) {
		assert_eq!(
			pezpallet_bridge_messages::InboundLanes::<Runtime, MPI>::get(self.lane)
				.map(|d| d.last_delivered_nonce()),
			Some(self.expected_nonce),
		);
	}
}

/// Verifies that relayer is rewarded at this chain.
pub struct VerifyRelayerRewarded<Runtime: pezpallet_bridge_relayers::Config<RPI>, RPI: 'static> {
	relayer: Runtime::AccountId,
	reward_params: Runtime::Reward,
}

impl<Runtime, RPI> VerifyRelayerRewarded<Runtime, RPI>
where
	Runtime: pezpallet_bridge_relayers::Config<RPI>,
	RPI: 'static,
{
	/// Expect given delivered nonce to be the latest after transaction.
	pub fn expect_relayer_reward(
		relayer: Runtime::AccountId,
		reward_params: impl Into<Runtime::Reward>,
	) -> Box<dyn VerifyTransactionOutcome> {
		Box::new(Self { relayer, reward_params: reward_params.into() })
	}
}

impl<Runtime, RPI> VerifyTransactionOutcome for VerifyRelayerRewarded<Runtime, RPI>
where
	Runtime: pezpallet_bridge_relayers::Config<RPI>,
	RPI: 'static,
{
	fn verify_outcome(&self) {
		assert!(pezpallet_bridge_relayers::RelayerRewards::<Runtime, RPI>::get(
			&self.relayer,
			&self.reward_params,
		)
		.is_some());
	}
}

/// Verifies that relayer balance is equal to given value.
pub struct VerifyRelayerBalance<Runtime: pezpallet_balances::Config> {
	relayer: Runtime::AccountId,
	balance: Runtime::Balance,
}

impl<Runtime> VerifyRelayerBalance<Runtime>
where
	Runtime: pezpallet_balances::Config,
{
	/// Expect given relayer balance after transaction.
	pub fn expect_relayer_balance(
		relayer: Runtime::AccountId,
		balance: Runtime::Balance,
	) -> Box<dyn VerifyTransactionOutcome> {
		Box::new(Self { relayer, balance })
	}
}

impl<Runtime> VerifyTransactionOutcome for VerifyRelayerBalance<Runtime>
where
	Runtime: pezpallet_balances::Config,
{
	fn verify_outcome(&self) {
		assert_eq!(
			pezpallet_balances::Pezpallet::<Runtime>::free_balance(&self.relayer),
			self.balance,
		);
	}
}

/// Initialize bridge GRANDPA pezpallet.
pub(crate) fn initialize_bridge_grandpa_pallet<Runtime, GPI>(
	init_data: pezbp_header_pez_chain::InitializationData<BridgedHeader<Runtime, GPI>>,
) where
	Runtime: BridgeGrandpaConfig<GPI>
		+ pezcumulus_pezpallet_teyrchain_system::Config
		+ pezpallet_timestamp::Config,
{
	pezpallet_bridge_grandpa::Pezpallet::<Runtime, GPI>::initialize(
		RuntimeHelper::<Runtime>::root_origin(),
		init_data,
	)
	.unwrap();
}

/// Runtime calls and their verifiers.
pub type CallsAndVerifiers<Runtime> =
	Vec<(RuntimeCallOf<Runtime>, Box<dyn VerifyTransactionOutcome>)>;

pub type InboundRelayerId<Runtime, MPI> = pezbp_runtime::AccountIdOf<BridgedChainOf<Runtime, MPI>>;

/// Returns relayer id at the bridged chain.
pub fn relayer_id_at_bridged_chain<Runtime: pezpallet_bridge_messages::Config<MPI>, MPI>(
) -> InboundRelayerId<Runtime, MPI> {
	Decode::decode(&mut TrailingZeroInput::zeroes()).unwrap()
}

/// Test-case makes sure that Runtime can dispatch XCM messages submitted by relayer,
/// with proofs (finality, message) independently submitted.
pub fn relayed_incoming_message_works<Runtime, AllPalletsWithoutSystem, MPI>(
	collator_session_key: CollatorSessionKeys<Runtime>,
	slot_durations: SlotDurations,
	runtime_para_id: u32,
	sibling_teyrchain_id: u32,
	local_relay_chain_id: NetworkId,
	construct_and_apply_extrinsic: fn(
		pezsp_keyring::Sr25519Keyring,
		RuntimeCallOf<Runtime>,
	) -> pezsp_runtime::DispatchOutcome,
	prepare_message_proof_import: impl FnOnce(
		Runtime::AccountId,
		InboundRelayerId<Runtime, MPI>,
		InteriorLocation,
		MessageNonce,
		Xcm<()>,
		pezbp_runtime::ChainId,
	) -> CallsAndVerifiers<Runtime>,
) where
	Runtime:
		BasicTeyrchainRuntime + pezcumulus_pezpallet_xcmp_queue::Config + BridgeMessagesConfig<MPI>,
	AllPalletsWithoutSystem:
		OnInitialize<BlockNumberFor<Runtime>> + OnFinalize<BlockNumberFor<Runtime>>,
	MPI: 'static,
	AccountIdOf<Runtime>: From<AccountId32>,
{
	let relayer_at_target = Bob;
	let relayer_id_on_target: AccountId32 = relayer_at_target.public().into();
	let relayer_id_on_source = relayer_id_at_bridged_chain::<Runtime, MPI>();
	let bridged_chain_id = Runtime::BridgedChain::ID;

	assert_ne!(runtime_para_id, sibling_teyrchain_id);

	run_test::<Runtime, _>(
		collator_session_key,
		runtime_para_id,
		vec![(
			relayer_id_on_target.clone().into(),
			// this value should be enough to cover all transaction costs, but computing the actual
			// value here is tricky - there are several transaction payment pallets and we don't
			// want to introduce additional bounds and traits here just for that, so let's just
			// select some presumably large value
			core::cmp::max::<Runtime::Balance>(Runtime::ExistentialDeposit::get(), 1u32.into())
				* 100_000_000u32.into(),
		)],
		|| {
			let mut alice = [0u8; 32];
			alice[0] = 1;

			let included_head = RuntimeHelper::<Runtime, AllPalletsWithoutSystem>::run_to_block(
				2,
				AccountId::from(alice).into(),
			);
			mock_open_hrmp_channel::<
				Runtime,
				pezcumulus_pezpallet_teyrchain_system::Pezpallet<Runtime>,
			>(
				runtime_para_id.into(),
				sibling_teyrchain_id.into(),
				included_head,
				&alice,
				&slot_durations,
			);

			// set up relayer details and proofs

			let message_destination: InteriorLocation =
				[GlobalConsensus(local_relay_chain_id), Teyrchain(sibling_teyrchain_id)].into();
			// some random numbers (checked by test)
			let message_nonce = 1;

			let xcm = vec![Instruction::<()>::ClearOrigin; 42];
			let expected_dispatch = xcm::latest::Xcm::<()>({
				let mut expected_instructions = xcm.clone();
				// dispatch prepends bridge pezpallet instance
				expected_instructions.insert(
					0,
					DescendOrigin([PalletInstance(
						<pezpallet_bridge_messages::Pezpallet<Runtime, MPI> as PalletInfoAccess>::index()
							as u8,
					)].into()),
				);
				expected_instructions
			});

			execute_and_verify_calls::<Runtime>(
				relayer_at_target,
				construct_and_apply_extrinsic,
				prepare_message_proof_import(
					relayer_id_on_target.clone().into(),
					relayer_id_on_source.clone().into(),
					message_destination,
					message_nonce,
					xcm.clone().into(),
					bridged_chain_id,
				),
			);

			// verify that imported XCM contains original message
			let imported_xcm =
				RuntimeHelper::<pezcumulus_pezpallet_xcmp_queue::Pezpallet<Runtime>>::take_xcm(
					sibling_teyrchain_id.into(),
				)
				.unwrap();
			let dispatched = xcm::latest::Xcm::<()>::try_from(imported_xcm).unwrap();
			let mut dispatched_clone = dispatched.clone();
			for (idx, expected_instr) in expected_dispatch.0.iter().enumerate() {
				assert_eq!(expected_instr, &dispatched.0[idx]);
				assert_eq!(expected_instr, &dispatched_clone.0.remove(0));
			}
			match dispatched_clone.0.len() {
				0 => (),
				1 => assert!(matches!(dispatched_clone.0[0], SetTopic(_))),
				count => assert!(false, "Unexpected messages count: {:?}", count),
			}
		},
	)
}

/// Execute every call and verify its outcome.
fn execute_and_verify_calls<Runtime: pezframe_system::Config>(
	submitter: pezsp_keyring::Sr25519Keyring,
	construct_and_apply_extrinsic: fn(
		pezsp_keyring::Sr25519Keyring,
		RuntimeCallOf<Runtime>,
	) -> pezsp_runtime::DispatchOutcome,
	calls_and_verifiers: CallsAndVerifiers<Runtime>,
) {
	for (call, verifier) in calls_and_verifiers {
		let dispatch_outcome = construct_and_apply_extrinsic(submitter, call);
		assert_ok!(dispatch_outcome);
		verifier.verify_outcome();
	}
}

pub(crate) mod for_pallet_xcm_bridge_hub {
	use super::{super::for_pallet_xcm_bridge_hub::*, *};

	/// Helper function to open the bridge/lane for `source` and `destination` while ensuring all
	/// required balances are placed into the SA of the source.
	pub fn ensure_opened_bridge<
		Runtime,
		XcmOverBridgePalletInstance,
		LocationToAccountId,
		TokenLocation>
	(source: Location, destination: InteriorLocation, is_paid_xcm_execution: bool, bridge_opener: impl Fn(pezpallet_xcm_bridge_hub::BridgeLocations, Option<Asset>)) -> (pezpallet_xcm_bridge_hub::BridgeLocations, pezpallet_xcm_bridge_hub::LaneIdOf<Runtime, XcmOverBridgePalletInstance>)
	where
		Runtime: BasicTeyrchainRuntime + BridgeXcmOverBridgeConfig<XcmOverBridgePalletInstance>,
		XcmOverBridgePalletInstance: 'static,
		<Runtime as pezframe_system::Config>::RuntimeCall: GetDispatchInfo + From<BridgeXcmOverBridgeCall<Runtime, XcmOverBridgePalletInstance>>,
		<Runtime as pezpallet_balances::Config>::Balance: From<<<Runtime as pezpallet_bridge_messages::Config<<Runtime as pezpallet_xcm_bridge_hub::Config<XcmOverBridgePalletInstance>>::BridgeMessagesPalletInstance>>::ThisChain as pezbp_runtime::Chain>::Balance>,
		<Runtime as pezpallet_balances::Config>::Balance: From<u128>,
		LocationToAccountId: ConvertLocation<AccountIdOf<Runtime>>,
		TokenLocation: Get<Location>
	{
		// construct expected bridge configuration
		let locations =
			pezpallet_xcm_bridge_hub::Pezpallet::<Runtime, XcmOverBridgePalletInstance>::bridge_locations(
				source.clone().into(),
				destination.clone().into(),
			)
				.expect("valid bridge locations");
		assert!(pezpallet_xcm_bridge_hub::Bridges::<Runtime, XcmOverBridgePalletInstance>::get(
			locations.bridge_id()
		)
		.is_none());

		// SA of source location needs to have some required balance
		if !<Runtime as pezpallet_xcm_bridge_hub::Config<XcmOverBridgePalletInstance>>::AllowWithoutBridgeDeposit::contains(&source) {
			// required balance: ED + fee + BridgeDeposit
			let bridge_deposit =
				<Runtime as pezpallet_xcm_bridge_hub::Config<XcmOverBridgePalletInstance>>::BridgeDeposit::get();
			let balance_needed = <Runtime as pezpallet_balances::Config>::ExistentialDeposit::get() + bridge_deposit.into();

			let source_account_id = LocationToAccountId::convert_location(&source).expect("valid location");
			let _ = <pezpallet_balances::Pezpallet<Runtime>>::mint_into(&source_account_id, balance_needed)
				.expect("mint_into passes");
		};

		let maybe_paid_execution = if is_paid_xcm_execution {
			// random high enough value for `BuyExecution` fees
			let buy_execution_fee_amount = 5_000_000_000_000_u128;
			let buy_execution_fee = (TokenLocation::get(), buy_execution_fee_amount).into();

			let balance_needed = <Runtime as pezpallet_balances::Config>::ExistentialDeposit::get()
				+ buy_execution_fee_amount.into();
			let source_account_id =
				LocationToAccountId::convert_location(&source).expect("valid location");
			let _ = <pezpallet_balances::Pezpallet<Runtime>>::mint_into(
				&source_account_id,
				balance_needed,
			)
			.expect("mint_into passes");
			Some(buy_execution_fee)
		} else {
			None
		};

		// call the bridge opener
		bridge_opener(*locations.clone(), maybe_paid_execution);

		// check opened bridge
		let bridge =
			pezpallet_xcm_bridge_hub::Bridges::<Runtime, XcmOverBridgePalletInstance>::get(
				locations.bridge_id(),
			)
			.expect("opened bridge");

		// check state
		assert_ok!(
			pezpallet_xcm_bridge_hub::Pezpallet::<Runtime, XcmOverBridgePalletInstance>::do_try_state()
		);

		// return locations
		(*locations, bridge.lane_id)
	}

	/// Utility for opening bridge with dedicated `pezpallet_xcm_bridge_hub`'s extrinsic.
	pub fn open_bridge_with_extrinsic<Runtime, XcmOverBridgePalletInstance>(
		(origin, origin_kind): (Location, OriginKind),
		bridge_destination_universal_location: InteriorLocation,
		maybe_paid_execution: Option<Asset>,
	) where
		Runtime: pezframe_system::Config
			+ pezpallet_xcm_bridge_hub::Config<XcmOverBridgePalletInstance>
			+ pezcumulus_pezpallet_teyrchain_system::Config
			+ pezpallet_xcm::Config,
		XcmOverBridgePalletInstance: 'static,
		<Runtime as pezframe_system::Config>::RuntimeCall:
			GetDispatchInfo + From<BridgeXcmOverBridgeCall<Runtime, XcmOverBridgePalletInstance>>,
	{
		// open bridge with `Transact` call
		let open_bridge_call = RuntimeCallOf::<Runtime>::from(BridgeXcmOverBridgeCall::<
			Runtime,
			XcmOverBridgePalletInstance,
		>::open_bridge {
			bridge_destination_universal_location: Box::new(
				bridge_destination_universal_location.clone().into(),
			),
		});

		// execute XCM as source origin would do with `Transact -> Origin::Xcm`
		assert_ok!(RuntimeHelper::<Runtime>::execute_as_origin(
			(origin, origin_kind),
			open_bridge_call,
			maybe_paid_execution
		)
		.ensure_complete());
	}

	/// Utility for opening bridge directly inserting data to the `pezpallet_xcm_bridge_hub`'s
	/// storage (used only for legacy purposes).
	pub fn open_bridge_with_storage<Runtime, XcmOverBridgePalletInstance>(
		locations: pezpallet_xcm_bridge_hub::BridgeLocations,
		lane_id: pezpallet_xcm_bridge_hub::LaneIdOf<Runtime, XcmOverBridgePalletInstance>,
	) where
		Runtime: pezpallet_xcm_bridge_hub::Config<XcmOverBridgePalletInstance>,
		XcmOverBridgePalletInstance: 'static,
	{
		// insert bridge data directly to the storage
		assert_ok!(
			pezpallet_xcm_bridge_hub::Pezpallet::<Runtime, XcmOverBridgePalletInstance>::do_open_bridge(
				Box::new(locations),
				lane_id,
				true
			)
		);
	}

	/// Helper function to close the bridge/lane for `source` and `destination`.
	pub fn close_bridge<Runtime, XcmOverBridgePalletInstance, LocationToAccountId, TokenLocation>(
		expected_source: Location,
		bridge_destination_universal_location: InteriorLocation,
		(origin, origin_kind): (Location, OriginKind),
		is_paid_xcm_execution: bool
	) where
		Runtime: BasicTeyrchainRuntime + BridgeXcmOverBridgeConfig<XcmOverBridgePalletInstance>,
		XcmOverBridgePalletInstance: 'static,
		<Runtime as pezframe_system::Config>::RuntimeCall: GetDispatchInfo + From<BridgeXcmOverBridgeCall<Runtime, XcmOverBridgePalletInstance>>,
		<Runtime as pezpallet_balances::Config>::Balance: From<<<Runtime as pezpallet_bridge_messages::Config<<Runtime as pezpallet_xcm_bridge_hub::Config<XcmOverBridgePalletInstance>>::BridgeMessagesPalletInstance>>::ThisChain as pezbp_runtime::Chain>::Balance>,
		<Runtime as pezpallet_balances::Config>::Balance: From<u128>,
		LocationToAccountId: ConvertLocation<AccountIdOf<Runtime>>,
		TokenLocation: Get<Location>
	{
		// construct expected bridge configuration
		let locations =
			pezpallet_xcm_bridge_hub::Pezpallet::<Runtime, XcmOverBridgePalletInstance>::bridge_locations(
				expected_source.clone().into(),
				bridge_destination_universal_location.clone().into(),
			)
				.expect("valid bridge locations");
		assert!(pezpallet_xcm_bridge_hub::Bridges::<Runtime, XcmOverBridgePalletInstance>::get(
			locations.bridge_id()
		)
		.is_some());

		// required balance: ED + fee + BridgeDeposit
		let maybe_paid_execution = if is_paid_xcm_execution {
			// random high enough value for `BuyExecution` fees
			let buy_execution_fee_amount = 2_500_000_000_000_u128;
			let buy_execution_fee = (TokenLocation::get(), buy_execution_fee_amount).into();

			let balance_needed = <Runtime as pezpallet_balances::Config>::ExistentialDeposit::get()
				+ buy_execution_fee_amount.into();
			let source_account_id =
				LocationToAccountId::convert_location(&expected_source).expect("valid location");
			let _ = <pezpallet_balances::Pezpallet<Runtime>>::mint_into(
				&source_account_id,
				balance_needed,
			)
			.expect("mint_into passes");
			Some(buy_execution_fee)
		} else {
			None
		};

		// close bridge with `Transact` call
		let close_bridge_call = RuntimeCallOf::<Runtime>::from(BridgeXcmOverBridgeCall::<
			Runtime,
			XcmOverBridgePalletInstance,
		>::close_bridge {
			bridge_destination_universal_location: Box::new(
				bridge_destination_universal_location.into(),
			),
			may_prune_messages: 16,
		});

		// execute XCM as source origin would do with `Transact -> Origin::Xcm`
		assert_ok!(RuntimeHelper::<Runtime>::execute_as_origin(
			(origin, origin_kind),
			close_bridge_call,
			maybe_paid_execution
		)
		.ensure_complete());

		// bridge is closed
		assert!(pezpallet_xcm_bridge_hub::Bridges::<Runtime, XcmOverBridgePalletInstance>::get(
			locations.bridge_id()
		)
		.is_none());

		// check state
		assert_ok!(
			pezpallet_xcm_bridge_hub::Pezpallet::<Runtime, XcmOverBridgePalletInstance>::do_try_state()
		);
	}
}
